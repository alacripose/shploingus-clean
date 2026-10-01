//! Substrate — how deep the soil is.
//!
//! **Source.** Heimsath, A., Dietrich, W., Nishiizumi, K. & Finkel, R. *The soil
//! production function and landscape equilibrium.* Nature 1997. Their result is
//! that soil is produced from bedrock at a rate that **falls exponentially with
//! the soil's own thickness** — thin soil produces fast, thick soil insulates its
//! own bedrock — and that a hillslope's soil depth is where production meets
//! removal.
//!
//! ```text
//!   production(h) = P₀ · e^(−h/h₀)          (Heimsath et al. 1997)
//!   removal(s)    = E · s                    (slope-proportional transport)
//!   steady state: h = −h₀ · ln(removal / P₀)
//! ```
//!
//! # Why the closed form and not a simulation
//!
//! Because it is **exact, deterministic and one line**. Simulating production and
//! removal over time would give the same steady state and would make the soil
//! depth depend on how many ticks the world had been running.
//!
//! # The rule this module obeys, and the one it does not
//!
//! **The baseline does not depend on the tick.** Soil depth here is a closed
//! form of lithology, slope and moisture and nothing else, so two worlds with the
//! same seed hold the same soil at the same coordinates no matter how many ticks
//! either has run — and an old world's soil is not a different soil from a new
//! world's at the same place. That is the replay guarantee F1 exists to protect,
//! and a world whose ground quietly depends on its own uptime cannot be replayed.
//!
//! **Transport does depend on the tick, and it lives in another stage.** Soil that
//! *moves* — down a slope, off a cut face, into a river — is not this module's
//! business. That is [`crate::stages::Stage::SoilTransport`], a sibling stage with
//! its own declared cadence on H-8.3's ladder, and it emits **deltas against this
//! baseline rather than editing it**. So the soil at an address is
//! `baseline + Σ deltas`, which is still a pure function of `(tick, address)`.
//!
//! **The distinction is the reference design's own, not a convenience.** ala-cities
//! keeps a derived **baseline** and a **transport** ledger apart on purpose: the
//! baseline is immutable and re-derived, while cut/fill/borrow/spoil carry
//! material lineage and can affect later dynamics. Splitting one subject into a
//! tick-free baseline and a tick-driven delta is what lets both be honest at once.
//! (`docs/HYPERGRILLING.md` H-16.3.)
//!
//! # The refusal this function contains
//!
//! Where `removal ≥ P₀` the logarithm has no solution and the answer is **bare
//! rock, depth zero** — not a small positive number. A slope too steep to hold
//! soil does not hold a centimetre of it, and returning `0.001` would put a
//! ghost of a soil layer on every cliff in the world.

use crate::geology::Lithology;
use crate::spec::WorldSpec;

/// The depth scale `h₀` at which production falls by `1/e`, in metres.
pub const SOIL_PRODUCTION_H0_M: f64 = 0.5;
/// Production at zero soil depth, in metres of soil per thousand years.
pub const SOIL_PRODUCTION_P0_M_PER_KYR: f64 = 0.35;
/// The transport coefficient `E`: metres of soil removed per thousand years per
/// unit of slope. Declared, and it is what sets where the soil line falls.
pub const TRANSPORT_E_M_PER_KYR: f64 = 0.12;

/// The deepest soil the model will report, in metres.
///
/// **A declared boundary, and it exists because the closed form diverges.** As
/// removal → 0 the equilibrium depth → ∞: flat ground with no transport never
/// reaches a steady state, because production never stops. A world cannot hold
/// infinite soil, so the model says where it stops instead of returning `inf`.
/// (An earlier revision did return `inf`, and the test that asserted finiteness
/// is the reason this constant exists.)
pub const MAX_SOIL_DEPTH_M: f64 = 3.0;
/// How much wetter ground transports faster, as a multiplier at full moisture.
///
/// Soil creep is faster on wet ground — the water is a lubricant and a transport
/// agent — and this is the coupling, stated, rather than a second depth field.
const WET_TRANSPORT_BOOST: f64 = 1.6;

/// How erodible a rock's own soil is. A rock that weathers to clay holds more of
/// it; a rock that weathers to sand holds less.
fn erodibility(lith: Lithology) -> f64 {
    match lith {
        Lithology::Regolith => 1.0,
        Lithology::Sedimentary => 0.9,
        Lithology::Metamorphic => 0.6,
        Lithology::Igneous => 0.5,
        // Bedrock is the thing soil is made *from*; it does not sit under more of
        // itself.
        Lithology::Bedrock => 0.2,
    }
}

/// Steady-state soil depth at a point, in metres.
///
/// `slope` is metres of drop per metre of run; `moisture` is in `[0, 1]`.
pub fn soil_depth_m(spec: &WorldSpec, lith: Lithology, slope: f64, moisture: f64) -> f64 {
    let _ = spec; // reserved: a world may declare a different production rate later
    let production = SOIL_PRODUCTION_P0_M_PER_KYR * erodibility(lith);
    if production <= 0.0 {
        return 0.0;
    }

    let m = moisture.clamp(0.0, 1.0);
    let removal = TRANSPORT_E_M_PER_KYR * slope.max(0.0) * (1.0 + (WET_TRANSPORT_BOOST - 1.0) * m);

    // Too steep to hold soil: bare rock, and the zero is deliberate.
    if removal >= production {
        return 0.0;
    }

    let depth = -SOIL_PRODUCTION_H0_M * (removal / production).ln();
    depth.clamp(0.0, MAX_SOIL_DEPTH_M)
}

/// Whether a point is bare rock rather than soil-covered.
pub fn is_bare_rock(spec: &WorldSpec, lith: Lithology, slope: f64, moisture: f64) -> bool {
    soil_depth_m(spec, lith, slope, moisture) <= 0.0
}

/// The soil depth that a slope must fall below for soil to start accumulating,
/// in metres of drop per metre of run. Returned so a caller can *state* the soil
/// line rather than rediscover it.
pub fn soil_line_slope(spec: &WorldSpec, lith: Lithology, moisture: f64) -> f64 {
    let _ = spec;
    let production = SOIL_PRODUCTION_P0_M_PER_KYR * erodibility(lith);
    let m = moisture.clamp(0.0, 1.0);
    let factor = TRANSPORT_E_M_PER_KYR * (1.0 + (WET_TRANSPORT_BOOST - 1.0) * m);
    if factor <= 0.0 {
        return f64::INFINITY;
    }
    production / factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(1, Config::default())
    }

    #[test]
    fn soil_is_deep_on_flat_ground_and_absent_on_cliffs() {
        let s = spec();
        let flat = soil_depth_m(&s, Lithology::Sedimentary, 0.0, 0.5);
        let gentle = soil_depth_m(&s, Lithology::Sedimentary, 0.2, 0.5);
        let cliff = soil_depth_m(&s, Lithology::Sedimentary, 5.0, 0.5);

        assert!(flat > gentle, "flat {flat} was not deeper than gentle {gentle}");
        assert!(gentle > cliff);
        assert_eq!(cliff, 0.0, "a cliff held {} m of soil", cliff);
        assert!(is_bare_rock(&s, Lithology::Sedimentary, 5.0, 0.5));
    }

    #[test]
    fn depth_is_never_negative_or_infinite_for_finite_input() {
        let s = spec();
        for lith in Lithology::ALL {
            for slope in [0.0, 0.01, 0.1, 0.5, 1.0, 10.0, 1e6] {
                for m in [0.0, 0.5, 1.0] {
                    let d = soil_depth_m(&s, lith, slope, m);
                    assert!(d >= 0.0 && d.is_finite(), "{lith:?} slope {slope} m {m} → {d}");
                    assert!(d <= MAX_SOIL_DEPTH_M, "{lith:?} slope {slope} → {d} exceeded the cap");
                }
            }
        }
    }

    #[test]
    fn the_soil_line_is_the_slope_where_depth_reaches_zero() {
        // The boundary the function claims, verified from both sides.
        let s = spec();
        let line = soil_line_slope(&s, Lithology::Sedimentary, 0.5);
        assert!(soil_depth_m(&s, Lithology::Sedimentary, line * 0.99, 0.5) > 0.0);
        assert_eq!(soil_depth_m(&s, Lithology::Sedimentary, line * 1.01, 0.5), 0.0);
    }

    #[test]
    fn wet_ground_holds_less_soil_than_dry_ground_at_the_same_slope() {
        let s = spec();
        let dry = soil_depth_m(&s, Lithology::Igneous, 0.3, 0.0);
        let wet = soil_depth_m(&s, Lithology::Igneous, 0.3, 1.0);
        assert!(wet < dry, "wet {wet} was not shallower than dry {dry}");
    }

    #[test]
    fn a_more_weatherable_rock_holds_deeper_soil() {
        let s = spec();
        let sed = soil_depth_m(&s, Lithology::Sedimentary, 0.25, 0.5);
        let ig = soil_depth_m(&s, Lithology::Igneous, 0.25, 0.5);
        assert!(sed > ig, "sedimentary {sed} was not deeper than igneous {ig}");
    }

    #[test]
    fn the_baseline_soil_does_not_depend_on_how_many_ticks_the_world_ran() {
        // **The rule this module's closed form exists to protect.** An old
        // world's ground must be the same ground a new world's holds at the same
        // coordinates, or the world cannot be replayed into its past (F1). The
        // transport half of this subject lives in `Stage::SoilTransport` and
        // deltas against this baseline; it does not move this number.
        let a = WorldSpec::new(1, Config::default());
        let old = a.at_tick(0);
        let far = a.at_tick(crate::climate::YEAR_TICKS * 3 + 7);
        for lith in Lithology::ALL {
            for slope in [0.0, 0.05, 0.3, 1.0, 4.0] {
                for m in [0.0, 0.5, 1.0] {
                    let first = soil_depth_m(&old, lith, slope, m);
                    let later = soil_depth_m(&far, lith, slope, m);
                    assert_eq!(
                        first, later,
                        "{lith:?} slope {slope} moisture {m}: the baseline moved from \
                         tick 0 to {} because the world got older",
                        far.tick
                    );
                }
            }
        }
    }

    #[test]
    fn the_transport_that_moves_soil_is_a_separate_stage_with_its_own_period() {
        // The other half of the rule: the tick-dependent half exists, and it is
        // somewhere else. If this stage vanished, the baseline would quietly
        // start depending on the tick — which is the failure the split prevents.
        use crate::stages::{Cadence, Stage};
        let transport = Stage::SoilTransport;
        assert_eq!(transport.name(), "soil_transport");
        assert_ne!(
            transport,
            Stage::Substrate,
            "transport and baseline cannot be the same stage"
        );
        // It is on the ladder at ×20 — erosion and deposition — not at ×1.
        assert_eq!(transport.cadence(), Cadence::Sediment);
        assert!(transport.is_due_at(20) && !transport.is_due_at(19));
    }

    #[test]
    fn depth_follows_the_published_production_curve() {
        // At h = h0, production has fallen to 1/e of P0, so the removal that
        // balances it is P0/e. Check the closed form against that definition.
        let s = spec();
        let h0 = SOIL_PRODUCTION_H0_M;
        // Solve for the slope that yields depth exactly h0, then check it.
        // production_at_h0 = P0 * erodibility / e, and removal = E * slope.
        let erodibility = 0.9; // sedimentary
        let p0 = SOIL_PRODUCTION_P0_M_PER_KYR * erodibility;
        let removal = p0 / std::f64::consts::E;
        let slope = removal / TRANSPORT_E_M_PER_KYR; // dry, so no boost
        let d = soil_depth_m(&s, Lithology::Sedimentary, slope, 0.0);
        assert!((d - h0).abs() < 1e-9, "depth at the h0 slope was {d}, not {h0}");
    }
}
