//! **What a material is made of, by element** — the law a broken cell's inventory credit is
//! stated in.
//!
//! # The law, and where it comes from
//!
//! The world's materials are declared by class rather than by assay, so the composition is a
//! **class composition**, stated once and cited:
//!
//! * **Rock and soil** — the major-element mass fractions of the continental crust, from the
//!   CRC Handbook of Chemistry and Physics table *Abundance of Elements in the Earth's Crust
//!   and in the Sea* (the table reproduced at
//!   <https://hyperphysics.phy-astr.gsu.edu/hbase/Tables/elabund.html>): O 46.6 %, Si 27.7 %,
//!   Al 8.1 %, Fe 5.0 %, Ca 3.6 %, Na 2.8 %, K 2.6 %, Mg 2.1 %, Ti 0.66 %, H 0.14 %, P 0.10 %
//!   by mass. The published rows sum to 99.4 %; the missing 0.6 % is the table's unlisted
//!   trace constituents, so the fractions here are **normalized to one** over the listed
//!   elements. That normalization is this project's arithmetic on the published numbers, not
//!   a second measurement.
//! * **Sand** — quartz, `SiO2`, with the IUPAC standard atomic weights (Si 28.085, O 15.999;
//!   CIAAW, <https://www.ciaaw.org/atomic-weights.htm>).
//! * **Water and ice** — `H2O`, same atomic weights (H 1.008).
//! * **Wood, leaves and grass** — cellulose, `(C6H10O5)n`, the structural polysaccharide of
//!   plant cell walls. The formula and its role are from Klemm, Heublein, Fink & Bohn,
//!   *Cellulose: fascinating biopolymer and sustainable raw material*, Angewandte Chemie
//!   International Edition 44 (2005) 3358–3393, <https://doi.org/10.1002/anie.200460587>.
//!
//! # What this is not
//!
//! It is not a claim that a voxel of `SAND` is pure quartz or that a grass voxel is dry
//! cellulose; it is the **declared class composition** the inventory's books are kept in, and
//! the module's own strings say so. A material with no row falls to the crust table, which is
//! the class the rock materials belong to. Air is not a material a break removes, so it has no
//! row.

use crate::materials;
use voxel_model::{MaterialId, Ratio};

/// An element the inventory keeps books in. The vocabulary is closed at the elements the
/// four composition tables name; a table that named a fifth would fail to compile.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Element {
    Oxygen,
    Silicon,
    Aluminium,
    Iron,
    Calcium,
    Sodium,
    Potassium,
    Magnesium,
    Titanium,
    Hydrogen,
    Carbon,
    Phosphorus,
}

impl Element {
    /// Every element, in the order the inventory prints them.
    pub const ALL: [Element; 12] = [
        Element::Oxygen,
        Element::Silicon,
        Element::Aluminium,
        Element::Iron,
        Element::Calcium,
        Element::Sodium,
        Element::Potassium,
        Element::Magnesium,
        Element::Titanium,
        Element::Hydrogen,
        Element::Carbon,
        Element::Phosphorus,
    ];

    /// The chemical symbol a person reads.
    pub const fn symbol(self) -> &'static str {
        match self {
            Element::Oxygen => "O",
            Element::Silicon => "Si",
            Element::Aluminium => "Al",
            Element::Iron => "Fe",
            Element::Calcium => "Ca",
            Element::Sodium => "Na",
            Element::Potassium => "K",
            Element::Magnesium => "Mg",
            Element::Titanium => "Ti",
            Element::Hydrogen => "H",
            Element::Carbon => "C",
            Element::Phosphorus => "P",
        }
    }
}

/// The crust table's listed rows, in tenths of a per cent, over the listed total (994).
const CRUST: &[(Element, Ratio)] = &[
    (Element::Oxygen, Ratio::from_parts(4660, 9940)),
    (Element::Silicon, Ratio::from_parts(2770, 9940)),
    (Element::Aluminium, Ratio::from_parts(810, 9940)),
    (Element::Iron, Ratio::from_parts(500, 9940)),
    (Element::Calcium, Ratio::from_parts(360, 9940)),
    (Element::Sodium, Ratio::from_parts(280, 9940)),
    (Element::Potassium, Ratio::from_parts(260, 9940)),
    (Element::Magnesium, Ratio::from_parts(210, 9940)),
    (Element::Titanium, Ratio::from_parts(66, 9940)),
    (Element::Hydrogen, Ratio::from_parts(14, 9940)),
    (Element::Phosphorus, Ratio::from_parts(10, 9940)),
];

/// `SiO2`'s mass fractions, from the atomic weights (28.085 + 2 × 15.999 = 60.083).
const QUARTZ: &[(Element, Ratio)] = &[
    (Element::Silicon, Ratio::from_parts(28085, 60083)),
    (Element::Oxygen, Ratio::from_parts(31998, 60083)),
];

/// `H2O`'s mass fractions (2 × 1.008 + 15.999 = 18.015).
const WATER: &[(Element, Ratio)] = &[
    (Element::Hydrogen, Ratio::from_parts(2016, 18015)),
    (Element::Oxygen, Ratio::from_parts(15999, 18015)),
];

/// Cellulose `C6H10O5` mass fractions (6 × 12.011 + 10 × 1.008 + 5 × 15.999 = 162.141).
const CELLULOSE: &[(Element, Ratio)] = &[
    (Element::Carbon, Ratio::from_parts(72066, 162141)),
    (Element::Hydrogen, Ratio::from_parts(10080, 162141)),
    (Element::Oxygen, Ratio::from_parts(79995, 162141)),
];

/// **The mass fractions of a material's element composition**, summing to exactly one.
///
/// The row's own class decides the table: water and ice are `H2O`, sand is quartz, the four
/// plant materials are cellulose, and every other admitted material is continental crust.
pub fn of(material: MaterialId) -> &'static [(Element, Ratio)] {
    match material {
        materials::WATER | materials::ICE => WATER,
        materials::SAND => QUARTZ,
        materials::GRASS | materials::DRY_GRASS | materials::LEAF | materials::CONIFER
        | materials::WOOD => CELLULOSE,
        _ => CRUST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(table: &[(Element, Ratio)]) -> Ratio {
        table.iter().fold(Ratio::ZERO, |sum, (_, ratio)| sum + *ratio)
    }

    #[test]
    fn every_composition_sums_to_exactly_one() {
        for (name, table) in [
            ("crust", CRUST),
            ("quartz", QUARTZ),
            ("water", WATER),
            ("cellulose", CELLULOSE),
        ] {
            assert_eq!(total(table), Ratio::ONE, "{name} does not close its books");
        }
    }

    #[test]
    fn the_class_tables_are_the_ones_the_materials_are_declared_in() {
        assert_eq!(of(materials::SAND), QUARTZ, "sand is not quartz");
        assert_eq!(of(materials::ICE), WATER, "ice is not water");
        assert_eq!(of(materials::WOOD), CELLULOSE, "wood is not cellulose");
        assert_eq!(of(materials::IGNEOUS), CRUST, "rock is not crust");
        assert_eq!(of(materials::REGOLITH), CRUST, "soil is not crust");
    }

    #[test]
    fn each_table_names_only_elements_it_accounts_for() {
        // A row whose element had no phrase in the display would be a book with no column.
        for table in [CRUST, QUARTZ, WATER, CELLULOSE] {
            for (element, _) in table {
                assert!(!element.symbol().is_empty());
            }
        }
    }
}
