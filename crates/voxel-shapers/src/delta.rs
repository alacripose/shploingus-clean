//! Deltas — where a river's load comes to rest.
//!
//! **Sources.**
//! * Shields, A. *Anwendung der Aehnlichkeitsmechanik und der
//!   Turbulenzforschung auf die Geschiebebewegung.* 1936 — the **Shields
//!   threshold**: below a critical shear stress, a grain does not move at all.
//! * Hjulström, F. *Studies of the morphological activity of rivers as
//!   illustrated by the River Fyris.* 1935 — the velocity at which a bed *starts*
//!   moving is **higher** than the velocity at which it settles, the hysteresis
//!   that makes a deposited grain harder to pick up than a moving one.
//! * Gilbert, G.K. *The Topographic Features of Lake Shores.* USGS 1885 — the
//!   **Gilbert delta**: topset, foreset and bottomset beds where a channel meets
//!   standing water.
//! * Slingerland, R. & Smith, N. *River avulsions and their deposits.* 2004 — a
//!   channel **abandons** its course once its own deposits raise it above its
//!   surroundings.
//!
//! # The refusal that matters here
//!
//! Below the Shields threshold, [`transport_capacity`] returns **exactly zero**.
//! Not a small number: a river that is not moving its bed is not moving its bed,
//! and a "negligible" 0.001 silts every still pool in the world over enough
//! ticks.

use crate::spec::WorldSpec;
use voxel_noise::mix;

/// The dimensionless Shields parameter above which a bed moves.
pub const SHIELDS_THRESHOLD: f64 = 0.045;
/// The velocity, in m/s, below which a bed is not entrained (Hjulström 1935).
pub const ENTRAINMENT_VELOCITY_M_S: f64 = 0.30;
/// The velocity below which suspended load settles.
pub const SETTLING_VELOCITY_M_S: f64 = 0.08;

/// Sediment transport capacity, in arbitrary mass units per second.
///
/// Zero below the entrainment velocity — the Hjulström/Shields refusal, and it
/// is exact.
pub fn transport_capacity(velocity_m_s: f64) -> f64 {
    if velocity_m_s <= ENTRAINMENT_VELOCITY_M_S {
        return 0.0;
    }
    (velocity_m_s - ENTRAINMENT_VELOCITY_M_S).powi(2)
}

/// Velocity implied by a slope and a depth, for the shallow-water régime this
/// world uses. Declared as a linearisation rather than claimed as Manning's.
pub fn velocity_m_s(slope: f64, depth_m: f64) -> f64 {
    (slope.max(0.0) * depth_m.max(0.05)).sqrt() * 6.0
}

/// How thick a delta's deposition is at a point, in metres.
///
/// Deposition needs **standing water**: a channel in the open air does not build
/// a delta, it builds a bank. So this returns `0` unless there is water above the
/// bed, which is the Gilbert delta's precondition.
pub fn deposition_m(capacity: f64, standing_water_depth_m: f64, slope: f64) -> f64 {
    if standing_water_depth_m <= 0.0 || capacity <= 0.0 {
        return 0.0;
    }
    // 0.5 is a **declared** packing factor: half the transported load is dropped
    // at the mouth and the other half is carried further. Stated so it can be
    // tuned, rather than being an unexplained fraction.
    const DEPOSITION_FRACTION: f64 = 0.5;
    // A steeper approach drops less: the flow keeps its energy.
    let retention = (1.0 - slope.clamp(0.0, 1.0)).max(0.0);
    capacity * DEPOSITION_FRACTION * retention * 1e-4
}

/// A delta's own state: how much it has built up, and where its lobe points.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DeltaState {
    /// Total thickness accumulated at the mouth, in metres.
    pub accumulated_m: f64,
    /// The lobe's bearing, in radians.
    pub bearing_rad: f64,
    /// How many times this delta has avulsed.
    pub avulsions: u32,
}

impl Default for DeltaState {
    fn default() -> Self {
        DeltaState { accumulated_m: 0.0, bearing_rad: 0.0, avulsions: 0 }
    }
}

/// The thickness at which a delta must abandon its course (Slingerland & Smith
/// 2004, as a declared threshold).
pub const AVULSION_THICKNESS_M: f64 = 3.0;

impl DeltaState {
    /// Add a deposition and return whether the delta avulsed as a result.
    pub fn deposit(&mut self, thickness_m: f64) -> bool {
        if thickness_m <= 0.0 {
            return false;
        }
        self.accumulated_m += thickness_m;
        self.accumulated_m >= AVULSION_THICKNESS_M
    }

    /// Abandon the current course: a new lobe bearing, and the accumulation
    /// reset.
    ///
    /// The new bearing comes from a **hash of the avulsion count and the world
    /// seed**, not an RNG, so a replay produces the same avulsion at the same
    /// tick (F1).
    pub fn avulse(&mut self, spec: &WorldSpec) {
        // **Stationary**: an avulsion's bearing is a function of how many have happened,
        // not of when. With the moving seed the same avulsion number would draw a
        // different bearing every tick, so a delta could not be replayed.
        let h = mix(spec.stationary_stage_seed("delta") ^ self.avulsions as u64);
        self.bearing_rad = (h >> 11) as f64 / (1u64 << 53) as f64 * std::f64::consts::TAU;
        self.accumulated_m = 0.0;
        self.avulsions = self.avulsions.saturating_add(1);
    }

    /// Whether the delta is at its avulsion threshold.
    pub fn should_avulse(&self) -> bool {
        self.accumulated_m >= AVULSION_THICKNESS_M
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(12, Config::default())
    }

    #[test]
    fn a_slow_river_transports_exactly_nothing() {
        // The Shields/Hjulström refusal, and it is exact: not "very small".
        assert_eq!(transport_capacity(0.0), 0.0);
        assert_eq!(transport_capacity(0.1), 0.0);
        assert_eq!(transport_capacity(ENTRAINMENT_VELOCITY_M_S), 0.0);
        assert!(transport_capacity(ENTRAINMENT_VELOCITY_M_S + 0.01) > 0.0);
    }

    #[test]
    fn capacity_grows_with_velocity() {
        let mut last = 0.0;
        for v in [0.31, 0.5, 1.0, 2.0, 4.0] {
            let c = transport_capacity(v);
            assert!(c > last, "capacity did not grow at v = {v}");
            last = c;
        }
    }

    #[test]
    fn no_standing_water_means_no_delta() {
        let capacity = transport_capacity(1.5);
        assert!(capacity > 0.0);
        assert_eq!(deposition_m(capacity, 0.0, 0.01), 0.0, "a dry channel built a delta");
        assert!(deposition_m(capacity, 2.0, 0.01) > 0.0);
    }

    #[test]
    fn a_delta_avulses_only_after_it_has_built_up() {
        let s = spec();
        let mut d = DeltaState::default();
        assert!(!d.should_avulse());
        let mut avulsed = false;
        for _ in 0..1000 {
            if d.deposit(0.01) {
                avulsed = true;
                break;
            }
        }
        assert!(avulsed, "the delta never reached its avulsion threshold");
        let before = d.bearing_rad;
        d.avulse(&s);
        assert_ne!(d.bearing_rad, before, "avulsion did not move the lobe");
        assert_eq!(d.avulsions, 1);
        assert_eq!(d.accumulated_m, 0.0, "avulsion did not reset the accumulation");
    }

    #[test]
    fn avulsion_is_deterministic() {
        let s = spec();
        let mut a = DeltaState::default();
        let mut b = DeltaState::default();
        for _ in 0..10 {
            a.avulse(&s);
            b.avulse(&s);
        }
        assert_eq!(a, b);
        // And a different world avulses differently.
        let mut c = DeltaState::default();
        c.avulse(&WorldSpec::new(13, Config::default()));
        assert_ne!(a.bearing_rad, c.bearing_rad);
    }

    #[test]
    fn deposition_on_a_steep_approach_is_smaller_than_on_a_flat_one() {
        let capacity = transport_capacity(2.0);
        let flat = deposition_m(capacity, 2.0, 0.0);
        let steep = deposition_m(capacity, 2.0, 0.6);
        assert!(flat > steep, "steep {steep} was not less than flat {flat}");
    }

    #[test]
    fn velocity_is_zero_on_a_flat_bed() {
        assert_eq!(velocity_m_s(0.0, 1.0), 0.0);
        assert!(velocity_m_s(0.01, 1.0) > 0.0);
    }
}
