//! **The player's books**: the elements a broken cell's material added to them.
//!
//! A break is a step down the profile32 cell (`VoxelShape::break_top_layer`), and the matter
//! that left is credited here by element. The books are **exact** ([`Ratio`] grams, no floats)
//! and are a consequence of two declared things rather than of numbers chosen here:
//!
//! * the removed volume — one subvoxel is `(1/4 m)³ = 1/64 m³`, so a layer is what was
//!   actually taken, not a nominal block;
//! * the material's own row — its bulk density from the registry, and its element mass
//!   fractions from [`voxel_shapers::composition`], which carries its sources.
//!
//! So the credit for a break is `density × removed-subvoxels/64 × fraction`, and the same break
//! twice adds the same mass twice. Nothing is cached and nothing is rounded: [`Inventory::summary`]
//! is the one place a number becomes a string, and it is a display value derived from the books.

use std::collections::BTreeMap;

use voxel_model::{MaterialId, Ratio, Refused};
use voxel_shapers::composition::{self, Element};

/// **What one break credited**, in grams: the total, and the elements in `Element::ALL` order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Credit {
    /// The mass this break took out of the world.
    pub grams: Ratio,
    /// The elements that mass was made of, largest first only if the caller sorts it; this
    /// vector is in the composition table's order and includes no zero entries.
    pub elements: Vec<(Element, Ratio)>,
}

/// The player's element totals, in grams.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    grams: BTreeMap<Element, Ratio>,
}

impl Inventory {
    pub fn new() -> Self {
        Inventory::default()
    }

    /// **Credit the matter a break removed.** `removed` is the subvoxel count per material that
    /// `VoxelShape::break_top_layer` returned; a material with no admitted row refuses rather
    /// than crediting a guessed mass.
    pub fn credit(&mut self, removed: &[(MaterialId, u32)]) -> Refused<Credit> {
        let registry = voxel_shapers::materials::registry()?;
        let mut gained: BTreeMap<Element, Ratio> = BTreeMap::new();
        let mut total = Ratio::ZERO;
        for (material, count) in removed {
            if *count == 0 {
                continue;
            }
            let row = registry.material(*material)?;
            // One subvoxel is (1/4 m)^3 = 1/64 m^3; the row's bulk density is kg/m^3.
            let kg = row.bulk_density_kg_m3() * Ratio::from_parts(u64::from(*count), 64);
            let grams = kg * Ratio::from_u64(1000);
            for (element, fraction) in composition::of(*material) {
                let part = grams * *fraction;
                *gained.entry(*element).or_insert(Ratio::ZERO) += part;
                total += part;
            }
        }
        for (element, part) in &gained {
            *self.grams.entry(*element).or_insert(Ratio::ZERO) += *part;
        }
        Ok(Credit {
            grams: total,
            elements: gained.into_iter().collect(),
        })
    }

    /// The mass of one element the player holds, in grams.
    pub fn grams_of(&self, element: Element) -> Ratio {
        self.grams.get(&element).copied().unwrap_or(Ratio::ZERO)
    }

    /// Everything the player holds, in grams.
    pub fn total_grams(&self) -> Ratio {
        self.grams.values().fold(Ratio::ZERO, |sum, grams| sum + *grams)
    }

    /// **The line a person reads.** Kilograms to two places for the total, then each element
    /// the player holds, in the declared element order — the order is a property of the
    /// vocabulary, not of the last break, so the line does not reshuffle as the books grow.
    pub fn summary(&self) -> String {
        let total_kg = self.total_grams().to_f64() / 1000.0;
        let parts: Vec<String> = Element::ALL
            .iter()
            .filter_map(|element| {
                let kg = self.grams_of(*element).to_f64() / 1000.0;
                (kg > 0.0).then(|| format!("{} {kg:.2}", element.symbol()))
            })
            .collect();
        if parts.is_empty() {
            format!("{total_kg:.2} kg")
        } else {
            format!("{total_kg:.2} kg ({})", parts.join(", "))
        }
    }

    /// **The gain one break writes**, for the console record: what this break added.
    ///
    /// The totals are [`Self::summary`]'s line and the caller puts them first, because the
    /// surface a break speaks on — the toast — **truncates**, and a record that led with the
    /// gain would cut off the number the player actually wants. So this line is the delta and
    /// the pack is the heading; neither repeats the other.
    pub fn credit_line(&self, credit: &Credit) -> String {
        let added: Vec<String> = credit
            .elements
            .iter()
            .map(|(element, grams)| format!("{} {:.2}", element.symbol(), grams.to_f64() / 1000.0))
            .collect();
        format!("+{} kg ({})", format_kg(credit.grams), added.join(", "))
    }
}

fn format_kg(grams: Ratio) -> String {
    format!("{:.2}", grams.to_f64() / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::materials;

    fn row_density(material: MaterialId) -> Ratio {
        voxel_shapers::materials::registry()
            .expect("the registry builds")
            .material(material)
            .expect("the material is admitted")
            .bulk_density_kg_m3()
    }

    #[test]
    fn one_layer_credits_exactly_its_volume_times_the_rows_density() {
        let mut inventory = Inventory::new();
        let credit = inventory
            .credit(&[(materials::REGOLITH, 16)])
            .expect("the registry answers");
        let expected = row_density(materials::REGOLITH) * Ratio::from_parts(16, 64) * Ratio::from_u64(1000);
        assert_eq!(credit.grams, expected, "the credit is not the removed mass");
        assert_eq!(inventory.total_grams(), expected);
        // And the elements sum to the credit: no mass is lost in the split.
        let elements = credit
            .elements
            .iter()
            .fold(Ratio::ZERO, |sum, (_, grams)| sum + *grams);
        assert_eq!(elements, credit.grams);
    }

    #[test]
    fn a_second_break_adds_the_same_mass_to_the_totals() {
        let mut inventory = Inventory::new();
        let first = inventory.credit(&[(materials::SAND, 16)]).expect("the first");
        let after_first = inventory.total_grams();
        let second = inventory.credit(&[(materials::SAND, 16)]).expect("the second");
        assert_eq!(first.grams, second.grams, "the same layer weighed differently");
        assert_eq!(inventory.total_grams(), after_first + second.grams);
    }

    #[test]
    fn the_classes_credit_the_elements_their_law_names() {
        let mut sand = Inventory::new();
        sand.credit(&[(materials::SAND, 64)]).expect("sand credits");
        assert!(sand.grams_of(Element::Silicon) > Ratio::ZERO, "sand has no silicon");
        assert_eq!(sand.grams_of(Element::Carbon), Ratio::ZERO, "sand has carbon");

        let mut wood = Inventory::new();
        wood.credit(&[(materials::WOOD, 64)]).expect("wood credits");
        assert!(wood.grams_of(Element::Carbon) > Ratio::ZERO, "wood has no carbon");
        assert_eq!(wood.grams_of(Element::Silicon), Ratio::ZERO, "wood has silicon");
    }

    #[test]
    fn an_empty_removal_credits_nothing_and_the_line_says_so() {
        let mut inventory = Inventory::new();
        let credit = inventory.credit(&[]).expect("nothing is not a refusal");
        assert_eq!(credit.grams, Ratio::ZERO);
        assert!(credit.elements.is_empty());
        assert_eq!(inventory.total_grams(), Ratio::ZERO);
        assert_eq!(inventory.summary(), "0.00 kg");
    }
}
