//! **`matter` — the number and the phase vocabulary the civilian machine is written in.**
//!
//! # Why this is a module of the port and not a rename of `crate::material`
//!
//! The machine in `agency/`, `gov/` and `sim/` is written in `Rational` everywhere — a
//! `Needs { nourishment, rest, safety }`, a consequence's compensation, a skill threshold, a
//! selection risk, a delta's amount — and it is written in it *signed*, because a deficit is
//! expressed as a negative quantity. This tree's [`crate::material::Ratio`] is unsigned
//! (`u64/u64`, no `Sub`) and its pair is [`crate::material::ComponentId`], a `u16` registry
//! handle. Two different numbers, and one name for each: the tree keeps its own, and this is
//! the port's, side by side and declared.
//!
//! # What is copied, and from where
//!
//! | here | `ala-cities/src/` | how |
//! |---|---|---|
//! | [`Rational`] | `matter/properties.rs` | **copied whole** — see [`rational`]; only its refusals are mapped onto this tree's [`Refusal`](crate::refusal::Refusal) |
//! | [`Lifecycle`] | `matter/generated.rs:5` | **copied verbatim** — the six review states a declared value carries; the field plane's contracts and the generated manifest spell the same word |
//! | [`FluidSpecies`], [`Phase`], [`ReservoirAccount`] | `matter/fluids.rs` lines 12–72 | **copied verbatim** — the vocabulary a [`FluidAddress`](crate::world::FluidAddress) is spelled in |
//!
//! `matter/fluids.rs` is 900 lines and the rest of it — `ContainerId`, `Reservoir`, pressure and
//! temperature scales, `FlowExchange` — is the **hydrology**, which is this tree's world
//! generator (`crates/voxel-shapers`, `docs/SHAPERS.md`) and is never copied (`event-0133`).
//! The three items above are not hydrology: they are the names a governed change uses to say
//! *which* fluid it moved, and the machine names them (`agency/selection.rs` reads
//! `provenance.fluid_address`, formats its species and phase, and looks its mass up in the
//! snapshot).

pub mod rational;

pub use rational::Rational;

use core::fmt;

/// The review lifecycle a declared value carries. Copied verbatim from the
/// reference's `matter/generated.rs:5` (`schema::Lifecycle`), because there a
/// field contract's `review_state` and a generated row's `lifecycle` are the
/// **same** six states, and a second vocabulary for one question is how a
/// reviewed value comes to look unreviewed or the reverse.
///
/// `Admitted` is the only state an authoritative value may be constructed
/// from. The others are kept because they are different facts: *reference*
/// material is not *provisional* material, and *derived* is not *retired*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lifecycle {
    /// The source the shape was measured from — reference, never rules.
    Reference,
    /// Declared from provisional evidence; human review still required.
    Provisional,
    /// Reviewed and admitted; the only state authority may be built from.
    Admitted,
    /// Produced by a declared derivation of admitted inputs.
    Derived,
    /// Known and deliberately not built yet.
    Deferred,
    /// Superseded; kept for the history it is.
    Retired,
}

impl Lifecycle {
    /// Every state, in a stable order. A state added without a [`Lifecycle::name`]
    /// arm does not compile; a state added without being in this list is caught
    /// by [`tests::every_lifecycle_state_is_named`].
    pub const ALL: [Lifecycle; 6] = [
        Lifecycle::Reference,
        Lifecycle::Provisional,
        Lifecycle::Admitted,
        Lifecycle::Derived,
        Lifecycle::Deferred,
        Lifecycle::Retired,
    ];

    /// The stable, machine-readable name.
    pub const fn name(self) -> &'static str {
        match self {
            Lifecycle::Reference => "reference",
            Lifecycle::Provisional => "provisional",
            Lifecycle::Admitted => "admitted",
            Lifecycle::Derived => "derived",
            Lifecycle::Deferred => "deferred",
            Lifecycle::Retired => "retired",
        }
    }
}

/// A fluid, by species. Copied verbatim from `ala-cities/src/matter/fluids.rs:12`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FluidSpecies {
    Water,
    Groundwater,
    Rain,
    Ice,
    Steam,
    Oil,
    Lava,
    Gas,
}

impl FluidSpecies {
    pub const ALL: [Self; 8] = [
        Self::Water,
        Self::Groundwater,
        Self::Rain,
        Self::Ice,
        Self::Steam,
        Self::Oil,
        Self::Lava,
        Self::Gas,
    ];

    pub const fn reservoir_account(self) -> ReservoirAccount {
        match self {
            Self::Water | Self::Groundwater | Self::Rain | Self::Ice | Self::Steam => {
                ReservoirAccount::WaterCycle
            }
            Self::Oil => ReservoirAccount::Oil,
            Self::Lava => ReservoirAccount::Lava,
            Self::Gas => ReservoirAccount::Gas,
        }
    }

    /// Whether this species may be in this phase. Copied verbatim; `world::delta`
    /// carries the same rule for the address it validates.
    pub const fn valid_phase(self, phase: Phase) -> bool {
        matches!(
            (self, phase),
            (
                Self::Water | Self::Groundwater | Self::Rain | Self::Oil | Self::Lava,
                Phase::Liquid
            ) | (Self::Ice, Phase::Solid)
                | (Self::Steam | Self::Gas, Phase::Gas)
        )
    }
}

/// The phase a fluid is in. Copied verbatim from `ala-cities/src/matter/fluids.rs:59`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    Liquid,
    Solid,
    Gas,
}

impl fmt::Display for Phase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Liquid => "liquid",
            Self::Solid => "solid",
            Self::Gas => "gas",
        })
    }
}

/// The reservoir a species belongs to. Copied verbatim from
/// `ala-cities/src/matter/fluids.rs:65`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReservoirAccount {
    WaterCycle,
    Oil,
    Lava,
    Gas,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_rule_is_the_sources() {
        assert!(FluidSpecies::Water.valid_phase(Phase::Liquid));
        assert!(FluidSpecies::Ice.valid_phase(Phase::Solid));
        assert!(FluidSpecies::Steam.valid_phase(Phase::Gas));
        assert!(!FluidSpecies::Water.valid_phase(Phase::Solid));
        assert!(!FluidSpecies::Gas.valid_phase(Phase::Liquid));
    }

    #[test]
    fn every_lifecycle_state_is_named() {
        // The residual of the compile-time guard: the `name` match is exhaustive,
        // so a new state cannot compile without a word; this catches a state
        // added to the enum but forgotten in `ALL`, which nothing else can.
        assert_eq!(Lifecycle::ALL.len(), 6);
        let mut names: Vec<&str> = Lifecycle::ALL.iter().map(|s| s.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "two lifecycle states share a name");
        assert!(
            Lifecycle::ALL.contains(&Lifecycle::Admitted),
            "the only state authority may be built from is not in ALL"
        );
    }

    #[test]
    fn every_species_has_a_reservoir() {
        for species in FluidSpecies::ALL {
            let _ = species.reservoir_account();
        }
        assert_eq!(
            FluidSpecies::Rain.reservoir_account(),
            ReservoirAccount::WaterCycle
        );
    }
}
