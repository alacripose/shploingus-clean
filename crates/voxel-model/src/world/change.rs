//! The governed change vocabulary: **what a change is made of**, before it is applied.
//!
//! Transcribed from `ala-cities/src/world/delta.rs` (lines 15–520: `ComponentId`, `Account`,
//! `LineageId`, `FluidAddress`, `Occupancy`, `OccupancyProvenance`, `Delta`, `Delta::validate`,
//! `canonical_deltas`) and `ala-cities/src/world/earthworks.rs` (124 lines: `EarthworksEffect`
//! and `EarthworksError`) — **seams only**, and the seams are three:
//!
//! | the source | here |
//! |---|---|
//! | its hand-rolled `Digest` | this tree's [`crate::identity::Digest`] (`sha2`, named by `voxel-model` and no other crate) |
//! | its `Coord` | this tree's [`super::region::Coord`] |
//! | `crate::matter::Rational` | this tree's [`crate::matter::Rational`] — the port's own signed number, which is why `matter` is at the crate root |
//!
//! # Three variants are admitted; five stay the generator's
//!
//! The source's `Delta` carries **eight** variants. **Three are here** — `MatterTransfer`,
//! `OccupancyChange`, `EvidenceOutput` — and the measurement is why: the machine constructs
//! exactly these (`agency/action.rs:1096` and `:1126` build `OccupancyChange`; `:1056–1140`
//! builds `MatterTransfer` and `EvidenceOutput`), and its single commit point reads back only
//! what moves mass and what occupies a coordinate.
//!
//! The other five — `FluidTransfer`, `FieldChange`, `FieldOutput`, `EcologyChange`, `Stratum` —
//! carry `FieldId`, `FieldValue`, `SpeciesId` and `Population`: **the world generation**, which
//! this tree owns as `crates/voxel-shapers` (`docs/DESIGN-SPEC.md:79` gives `delta` and
//! `earthworks` to it; `docs/SHAPERS.md` is its contract). Their two bridges,
//! `Delta::from_process_output` and `from_process_output_with_context`, read the generator's
//! `ProcessOutput` and are the generator's for the same reason. This is the ruling already
//! recorded in `docs/CIVILIAN-PORT.md` and in this module's parent: the world half is **read and
//! mapped, never copied** (`event-0133`; the vocabulary's own landing is `event-0138`).
//!
//! So `EarthworksEffect::fluid_transfer` is not admitted either — it builds `FluidTransfer` —
//! and the governed `excavation`/`deposition` family is what remains.
//!
//! # What is kept, and why it is kept exactly
//!
//! * every id **trims and refuses empty** (`ComponentId`, `Account`, a `LineageId` part, a
//!   `FluidAddress` name) — `super::transition::named_id` is the one statement of that rule;
//! * `LineageId` keeps its **parts** *and* its `":"`-joined text, because the parts are the
//!   identity (`docs/LEDGER.md:1994`'s boundary is why this tree length-prefixes rather than
//!   concatenates, and the join is only for reading);
//! * `Occupancy` **cannot be constructed negative** and has two declared values — `empty` and
//!   `full`; a third value is a `Rational`, so partial occupancy is expressible and never a
//!   float;
//! * `Delta::validate` refuses a negative transfer, an occupancy change with no provenance, and
//!   an evidence output with no message — **before** anything is applied, which is the same
//!   order the source checks in;
//! * `canonical_deltas` sorts by the source's own order key, **keeping its discriminants**
//!   (matter `0`, occupancy `2`, evidence `7`) so the relative order is the source's order and
//!   not a new one.

use crate::identity::Digest;
use crate::matter::{FluidSpecies, Phase, Rational};

use super::region::Coord;
use super::transition::named_id;
use super::WorldError;

/// The zero digest, for the one check the source spells `is_zero()`.
///
/// This tree's [`Digest`] deliberately has no `is_zero` — the source's check is a comparison,
/// and a comparison is what this is, rather than new surface on a type this module does not own.
const ZERO_DIGEST: Digest = Digest::from_bytes([0; 32]);

/// The component a change moves — a material, by name.
///
/// The machine's `crate::world::ComponentId`. **Not** [`crate::material::ComponentId`], which is
/// this tree's own `u16` registry handle for a different subject: the source's is a `String`
/// newtype over a *named* component (`world/delta.rs:15`), and the governed book keys accounts
/// by it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(String);

impl ComponentId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("component", value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An account a change moves mass from or to: a named ledger, the substrate of a coordinate,
/// `carried:<actor>`, `spoil:<actor>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account(String);

impl Account {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("account", value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The chain of custody a change came through. **Parts are the identity**; the joined form is
/// only how it reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineageId {
    parts: Vec<String>,
    joined: String,
}

impl LineageId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Self::from_parts([value.into()])
    }

    pub fn from_parts<I, S>(parts: I) -> Result<Self, WorldError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let parts = parts
            .into_iter()
            .map(|part| {
                let part = part.into();
                let part = part.trim();
                if part.is_empty() {
                    Err(WorldError::InvalidTransition {
                        reason: "lineage parts must not be empty".to_owned(),
                    })
                } else {
                    Ok(part.to_owned())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parts.is_empty() {
            return Err(WorldError::InvalidTransition {
                reason: "lineage must not be empty".to_owned(),
            });
        }
        let joined = parts.join(":");
        Ok(Self { parts, joined })
    }

    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    pub fn as_str(&self) -> &str {
        &self.joined
    }
}

/// A fluid, addressed by name, species, phase and — when it is *at* a coordinate — where.
///
/// The vocabulary a governed change is spelled in for fluids. The **hydrology** that computes
/// them is the generator's (`crates/voxel-shapers`, `docs/SHAPERS.md`) and is never copied.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluidAddress {
    name: String,
    species: FluidSpecies,
    phase: Phase,
    coord: Option<Coord>,
}

impl FluidAddress {
    pub fn new(
        name: impl Into<String>,
        species: FluidSpecies,
        phase: Phase,
    ) -> Result<Self, WorldError> {
        Self::at(name, species, phase, None)
    }

    pub fn at(
        name: impl Into<String>,
        species: FluidSpecies,
        phase: Phase,
        coord: Option<Coord>,
    ) -> Result<Self, WorldError> {
        let name = named_id("fluid address", name)?;
        if !valid_phase(species, phase) {
            return Err(WorldError::InvalidTransition {
                reason: format!("invalid phase for {species:?}"),
            });
        }
        Ok(Self {
            name,
            species,
            phase,
            coord,
        })
    }

    pub fn species(&self) -> FluidSpecies {
        self.species
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn coord(&self) -> Option<Coord> {
        self.coord
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}

/// A species and phase that can co-exist. Copied whole from the source, including the refusals.
fn valid_phase(species: FluidSpecies, phase: Phase) -> bool {
    matches!(
        (species, phase),
        (
            FluidSpecies::Water
                | FluidSpecies::Groundwater
                | FluidSpecies::Rain
                | FluidSpecies::Oil
                | FluidSpecies::Lava,
            Phase::Liquid
        ) | (FluidSpecies::Ice, Phase::Solid)
            | (FluidSpecies::Steam | FluidSpecies::Gas, Phase::Gas)
    )
}

/// How much of a coordinate is occupied, as an **exact** fraction.
///
/// The field is public because the source's is: a caller that has decided a coordinate is full
/// or empty does not have to go through a constructor, and `Delta::validate` re-checks what
/// arrives anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occupancy(pub Rational);

impl Occupancy {
    pub fn new(value: Rational) -> Result<Self, WorldError> {
        if value.is_negative() {
            return Err(WorldError::InvalidTransition {
                reason: "occupancy cannot be negative".to_owned(),
            });
        }
        Ok(Self(value))
    }

    pub fn empty() -> Self {
        Self(Rational::ZERO)
    }

    pub fn full() -> Self {
        Self(Rational::from_integer(1))
    }

    pub fn value(self) -> Rational {
        self.0
    }
}

/// Which transition made a coordinate occupied, and out of what account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccupancyProvenance {
    pub transition_id: Digest,
    pub account: Account,
    pub lineage: LineageId,
    pub fluid_address: Option<FluidAddress>,
}

/// A governed change of the world, **before** it is applied.
///
/// Three of the source's eight variants — see the module documentation for what the other five
/// are and where they live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delta {
    MatterTransfer {
        component: ComponentId,
        from: Account,
        to: Account,
        amount: Rational,
        lineage: LineageId,
    },
    OccupancyChange {
        coord: Coord,
        before: Occupancy,
        after: Occupancy,
        provenance: OccupancyProvenance,
    },
    EvidenceOutput {
        message: String,
        lineage: LineageId,
    },
}

impl Delta {
    /// Refuse a change that could not be applied, **before** anything is.
    ///
    /// The source's checks, variant for variant and message for message, minus the five variants
    /// that are not admitted.
    pub fn validate(&self) -> Result<(), WorldError> {
        match self {
            Self::MatterTransfer {
                amount, lineage, ..
            } => {
                if amount.is_negative() {
                    return Err(WorldError::InvalidTransition {
                        reason: "matter transfer cannot be negative".to_owned(),
                    });
                }
                if lineage.parts().is_empty() {
                    return Err(WorldError::InvalidTransition {
                        reason: "matter transfer requires lineage".to_owned(),
                    });
                }
            }
            Self::OccupancyChange {
                before,
                after,
                provenance,
                ..
            } => {
                if before.value().is_negative()
                    || after.value().is_negative()
                    || provenance.transition_id == ZERO_DIGEST
                    || provenance.lineage.parts().is_empty()
                {
                    return Err(WorldError::InvalidTransition {
                        reason: "occupancy requires non-negative values and provenance".to_owned(),
                    });
                }
            }
            Self::EvidenceOutput { message, lineage } => {
                if message.trim().is_empty() || lineage.parts().is_empty() {
                    return Err(WorldError::InvalidTransition {
                        reason: "evidence output requires lineage".to_owned(),
                    });
                }
            }
        }
        Ok(())
    }

    /// The source's own ordering key, kept so a batch canonicalises to the source's order.
    ///
    /// The discriminants are the source's (`0`, `2`, `7`), not `0..3`, so every relative order
    /// the source states still holds.
    #[allow(dead_code)] // called by canonical_deltas; both land with the store, the next landing
    fn order_key(&self) -> (u8, String, String, String) {
        match self {
            Self::MatterTransfer {
                component,
                from,
                to,
                amount,
                lineage,
            } => (
                0,
                component.as_str().to_owned(),
                format!(
                    "{}>{}:{}:{}:{}",
                    from.as_str(),
                    to.as_str(),
                    amount.numerator(),
                    amount.denominator(),
                    lineage.parts().join(">")
                ),
                String::new(),
            ),
            Self::OccupancyChange {
                coord,
                before,
                after,
                provenance,
            } => (
                2,
                format!("{},{},{}", coord.x, coord.y, coord.z),
                format!(
                    "{}:{}:{}:{}",
                    before.value().numerator(),
                    before.value().denominator(),
                    after.value().numerator(),
                    after.value().denominator()
                ),
                format!(
                    "{:?}:{}:{}:{:?}",
                    provenance.transition_id,
                    provenance.account.as_str(),
                    provenance.lineage.parts().join(">"),
                    provenance.fluid_address
                ),
            ),
            Self::EvidenceOutput { message, lineage } => {
                (7, message.clone(), lineage.parts().join(">"), String::new())
            }
        }
    }
}

/// Put a batch in the source's canonical order. Two runs of one batch apply identically.
#[allow(dead_code)] // the governed store is the next landing; this is its first reader
pub(crate) fn canonical_deltas(deltas: &[Delta]) -> Vec<Delta> {
    let mut sorted = deltas.to_vec();
    sorted.sort_by_key(Delta::order_key);
    sorted
}

/// What a physical act does, as the deltas it makes.
///
/// This is the source's `EarthworksEffect` (`world/earthworks.rs`, 124 lines) minus
/// `fluid_transfer`, which builds the generator's `FluidTransfer`. The name is a near-miss with
/// this tree's `voxel_shapers::earthworks` on purpose and is named here rather than left to
/// collide silently: **that** is the record a change is written into; **this** is the governed
/// delta a change is made of, and the record's entry carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EarthworksEffect {
    pub deltas: Vec<Delta>,
}

/// Why an earthwork could not be formed. Copied whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EarthworksError {
    MissingAccount,
    InvalidQuantity,
}

impl core::fmt::Display for EarthworksError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::MissingAccount => "earthwork account is missing",
            Self::InvalidQuantity => "earthwork quantity is invalid",
        })
    }
}

impl core::error::Error for EarthworksError {}

impl EarthworksEffect {
    /// Break, gather or dig: matter leaves an account for `spoil`/`carried`.
    ///
    /// The coordinate is the source's own unused parameter — the occupancy provenance carries
    /// the coordinate, so this does not need it, and dropping it would have edited a call site
    /// the machine still makes.
    pub fn excavation(
        _coord: Coord,
        source: Account,
        spoil: Account,
        component: ComponentId,
        amount: Rational,
        lineage: LineageId,
    ) -> Result<Self, WorldError> {
        Ok(Self {
            deltas: vec![matter_delta(source, spoil, component, amount, lineage)?],
        })
    }

    pub fn deposition(
        source: Account,
        destination: Account,
        component: ComponentId,
        amount: Rational,
        lineage: LineageId,
    ) -> Result<Self, EarthworksError> {
        Ok(Self {
            deltas: vec![matter_delta(source, destination, component, amount, lineage)
                .map_err(map_world_error)?],
        })
    }

    pub fn fill(
        source: Account,
        destination: Account,
        component: ComponentId,
        amount: Rational,
        lineage: LineageId,
    ) -> Result<Self, EarthworksError> {
        Self::deposition(source, destination, component, amount, lineage)
    }

    pub fn erosion_repair(
        source: Account,
        destination: Account,
        component: ComponentId,
        amount: Rational,
        lineage: LineageId,
    ) -> Result<Self, EarthworksError> {
        Self::deposition(source, destination, component, amount, lineage)
    }

    pub fn placement(
        source: Account,
        destination: Account,
        component: ComponentId,
        amount: Rational,
        lineage: LineageId,
    ) -> Result<Self, EarthworksError> {
        Self::deposition(source, destination, component, amount, lineage)
    }
}

/// A matter transfer that may not be negative. The source's `matter_delta`, whole.
fn matter_delta(
    source: Account,
    destination: Account,
    component: ComponentId,
    amount: Rational,
    lineage: LineageId,
) -> Result<Delta, WorldError> {
    if amount.is_negative() {
        return Err(WorldError::InvalidTransition {
            reason: "earthwork quantity cannot be negative".to_owned(),
        });
    }
    Ok(Delta::MatterTransfer {
        component,
        from: source,
        to: destination,
        amount,
        lineage,
    })
}

fn map_world_error(error: WorldError) -> EarthworksError {
    match error {
        WorldError::InvalidTransition { .. } => EarthworksError::InvalidQuantity,
        _ => EarthworksError::MissingAccount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(name: &str) -> Account {
        Account::new(name).unwrap()
    }

    fn component() -> ComponentId {
        ComponentId::new("rock").unwrap()
    }

    fn lineage() -> LineageId {
        LineageId::from_parts(["test", "1"]).unwrap()
    }

    #[test]
    fn every_id_trims_and_refuses_empty() {
        assert!(ComponentId::new("").is_err());
        assert!(ComponentId::new("   ").is_err());
        assert!(Account::new("").is_err());
        assert_eq!(Account::new("  carried:ala  ").unwrap().as_str(), "carried:ala");
        assert!(FluidAddress::new("", FluidSpecies::Water, Phase::Liquid).is_err());
    }

    #[test]
    fn a_lineage_is_its_parts_and_refuses_an_empty_one() {
        let lineage = LineageId::from_parts(["a", "b"]).unwrap();
        assert_eq!(lineage.parts(), ["a", "b"]);
        assert_eq!(lineage.as_str(), "a:b");
        assert!(LineageId::from_parts(["a", ""]).is_err());
        assert!(LineageId::from_parts(Vec::<String>::new()).is_err());
    }

    #[test]
    fn occupancy_cannot_be_negative_and_has_two_declared_values() {
        assert!(Occupancy::new(Rational::from_integer(-1)).is_err());
        assert_eq!(Occupancy::empty().value(), Rational::ZERO);
        assert_eq!(Occupancy::full().value(), Rational::from_integer(1));
    }

    #[test]
    fn a_fluid_address_refuses_a_phase_its_species_cannot_have() {
        assert!(FluidAddress::new("spring", FluidSpecies::Water, Phase::Solid).is_err());
        assert!(FluidAddress::new("glacier", FluidSpecies::Ice, Phase::Solid).is_ok());
    }

    #[test]
    fn a_matter_transfer_may_not_be_negative_and_keeps_its_lineage() {
        let negative = Delta::MatterTransfer {
            component: component(),
            from: account("substrate"),
            to: account("carried:ala"),
            amount: Rational::from_integer(-1),
            lineage: lineage(),
        };
        assert!(negative.validate().is_err());

        let positive = Delta::MatterTransfer {
            component: component(),
            from: account("substrate"),
            to: account("carried:ala"),
            amount: Rational::from_integer(1),
            lineage: lineage(),
        };
        assert!(positive.validate().is_ok());
    }

    #[test]
    fn an_occupancy_change_requires_provenance() {
        let without_provenance = Delta::OccupancyChange {
            coord: Coord::new(0, 0, 0),
            before: Occupancy::full(),
            after: Occupancy::empty(),
            provenance: OccupancyProvenance {
                transition_id: ZERO_DIGEST,
                account: account("substrate"),
                lineage: lineage(),
                fluid_address: None,
            },
        };
        assert!(without_provenance.validate().is_err());
    }

    #[test]
    fn an_evidence_output_requires_a_message() {
        let empty = Delta::EvidenceOutput {
            message: "   ".to_owned(),
            lineage: lineage(),
        };
        assert!(empty.validate().is_err());
    }

    #[test]
    fn canonical_order_is_the_sources_order_and_not_a_new_one() {
        let evidence = Delta::EvidenceOutput {
            message: "process.evidence".to_owned(),
            lineage: lineage(),
        };
        let occupancy = Delta::OccupancyChange {
            coord: Coord::new(1, 2, 3),
            before: Occupancy::empty(),
            after: Occupancy::full(),
            provenance: OccupancyProvenance {
                transition_id: Digest::of(b"occupancy"),
                account: account("substrate"),
                lineage: lineage(),
                fluid_address: None,
            },
        };
        let matter = Delta::MatterTransfer {
            component: component(),
            from: account("carried:ala"),
            to: account("substrate"),
            amount: Rational::from_integer(2),
            lineage: lineage(),
        };
        let ordered = canonical_deltas(&[evidence, occupancy, matter]);
        assert!(matches!(ordered[0], Delta::MatterTransfer { .. }));
        assert!(matches!(ordered[1], Delta::OccupancyChange { .. }));
        assert!(matches!(ordered[2], Delta::EvidenceOutput { .. }));
    }

    #[test]
    fn an_excavation_moves_matter_and_refuses_a_negative_amount() {
        let effect = EarthworksEffect::excavation(
            Coord::new(0, 0, 0),
            account("substrate"),
            account("spoil:ala"),
            component(),
            Rational::from_integer(3),
            lineage(),
        )
        .unwrap();
        match &effect.deltas[0] {
            Delta::MatterTransfer { from, to, amount, .. } => {
                assert_eq!(from.as_str(), "substrate");
                assert_eq!(to.as_str(), "spoil:ala");
                assert_eq!(*amount, Rational::from_integer(3));
            }
            other => panic!("excavation must build a matter transfer, not {other:?}"),
        }

        let refused = EarthworksEffect::excavation(
            Coord::new(0, 0, 0),
            account("substrate"),
            account("spoil:ala"),
            component(),
            Rational::from_integer(-3),
            lineage(),
        );
        assert!(refused.is_err());
    }
}
