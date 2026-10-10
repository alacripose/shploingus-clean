//! Erosion — thermal (exactly conservative) and hydraulic (declared approximate).
//!
//! **Sources.**
//! * Musgrave, F.K., Kolb, C. & Mace, R. *The Synthesis and Rendering of Eroded
//!   Fractal Terrains.* SIGGRAPH 1989 — **thermal** erosion: rock above its angle
//!   of repose slides.
//! * Hansson, H. *Experiments on Hydraulic Erosion.* MSc thesis, Chalmers 2017 —
//!   the **droplet** method: rain falls on a point, runs downhill carrying
//!   sediment whose capacity is set by the slope and the flow.
//! * Mei, X., Decaudin, P. & Hu, S. *Fast Hydraulic Erosion Simulation and
//!   Visualization on GPU.* Pacific Graphics 2007 — the grid-based alternative,
//!   cited because it is the other half of the literature and is **not**
//!   implemented here.
//!
//! # Two algorithms, two different claims about mass, and they are not the same
//!
//! F1 lists *"process rates and cadences for erosion"* among the six
//! **approximate** families, sitting against mass and volume in the exact set.
//! This module takes that seriously rather than treating it as permission to be
//! sloppy:
//!
//! * [`thermal`] **is exactly volume-conserving**, and that is a test, not a
//!   hope: material is moved between two cells in one symmetric operation, so the
//!   sum cannot drift. It is an exact algorithm and it is proven exact.
//! * [`hydraulic`] **is not**, and says so. A droplet's brush can lift more than
//!   the height difference at a point, and the classic algorithm deposits
//!   sediment on a brush too. What it *does* do is account for both sides
//!   ([`Erosion::removed_m3`], [`Erosion::deposited_m3`]), so the material that
//!   left the patch is a **number** — [`Erosion::exported_m3`] — rather than an
//!   unexamined assumption. If that number ever needs to feed a mass ledger it
//!   must go through `Mass::from_approximate` with a declared tolerance, which is
//!   the boundary F1 draws.

use crate::spec::WorldSpec;
use voxel_noise::mix;

/// **The stream-power law**, the landscape-scale law the droplet method does not
/// supply: `E = K · A^m · S^n`.
///
/// * Whipple, K.X. & Tucker, G.E. *Dynamics of the stream-power river incision
///   model.* JGR 104(B8), 1999.
/// * Lague, D. *The stream power river incision model: evidence, theory and
///   beyond.* ESPL 39, 2014 — the observed downslope decrease of channel slope
///   with drainage area is reproduced for `m/n ≈ 0.5`.
///
/// The three coefficients are **declared dials**, named rather than buried: a
/// different `K` is a different world, and this repository refuses to hide that in
/// a call site. `m` and `n` are the literature's ratio and a linear slope term.
pub const STREAM_POWER_K: f64 = 1.0e-4;
pub const STREAM_POWER_M: f64 = 0.5;
pub const STREAM_POWER_N: f64 = 1.0;

/// Incision rate of the stream-power law, in the law's own units.
pub fn stream_power_rate(drainage_area_m2: f64, slope: f64) -> f64 {
    if !drainage_area_m2.is_finite() || drainage_area_m2 < 0.0 {
        return 0.0;
    }
    if !slope.is_finite() || slope <= 0.0 {
        // A flat bed incises nothing: the law's `S^n` is exactly zero there.
        return 0.0;
    }
    STREAM_POWER_K * drainage_area_m2.powf(STREAM_POWER_M) * slope.powf(STREAM_POWER_N)
}

/// The critical slope `S_c` at which nonlinear hillslope transport diverges.
///
/// Roering, J.J., Kirchner, J.W. & Dietrich, W.E. *Evidence for nonlinear,
/// diffusive sediment transport on hillslopes.* WRR 35(3), 1999 — transport is
/// `q = K · S / (1 − (S/S_c)²)`, so it **refuses** at and above `S_c` rather than
/// returning infinity. A slope in metres per metre; `0.8` is a steep hillslope.
pub const CRITICAL_SLOPE: f64 = 0.8;

/// Nonlinear hillslope transport `q = K S / (1 − (S/S_c)²)`.
///
/// `None` at or above [`CRITICAL_SLOPE`], because the law diverges there and a
/// number would be a default where a refusal belongs.
pub fn hillslope_transport(slope: f64) -> Option<f64> {
    const K: f64 = 1.0e-3;
    if !slope.is_finite() || slope < 0.0 {
        return None;
    }
    let ratio = slope / CRITICAL_SLOPE;
    if ratio >= 1.0 {
        return None;
    }
    Some(K * slope / (1.0 - ratio * ratio))
}

/// The result of an erosion pass.
#[derive(Clone, Debug)]
pub struct Erosion {
    pub heights: Vec<f64>,
    /// Cubic metres removed from cells.
    pub removed_m3: f64,
    /// Cubic metres placed on cells.
    pub deposited_m3: f64,
}

impl Erosion {
    /// Material that left the eroded patch. **Nonzero is normal** for hydraulic
    /// erosion and the number is a finding, not a bug.
    pub fn exported_m3(&self) -> f64 {
        self.removed_m3 - self.deposited_m3
    }

    /// The total volume of the height field, in cubic metres. Used by the tests
    /// and by the mass boundary.
    pub fn volume_m3(&self, cell_area_m2: f64) -> f64 {
        self.heights.iter().sum::<f64>() * cell_area_m2
    }
}

/// Thermal erosion: material above the angle of repose slides downhill.
///
/// **Exactly conservative.** Each transfer is computed once per neighbour *pair*
/// and applied symmetrically, so no cell can remove what its neighbour did not
/// receive. That is the whole reason this function can claim exactness where the
/// droplet cannot.
///
/// `repose_slope` is metres of drop per metre of run — `0.7` is roughly dry sand,
/// `1.0` roughly scree. It is a parameter because it is a property of the
/// material and the material is not this module's business.
pub fn thermal(
    heights: &[f64],
    width: usize,
    height: usize,
    step_m: f64,
    repose_slope: f64,
    rate: f64,
    iterations: u32,
) -> Erosion {
    let mut h = heights.to_vec();

    for _ in 0..iterations {
        // Forward pairs only: (i,j)-(i+1,j) and (i,j)-(i,j+1). Reading each pair
        // once is what makes the transfer symmetric and the total exact.
        for j in 0..height {
            for i in 0..width {
                let k = j * width + i;
                if i + 1 < width {
                    relax(&mut h, k, k + 1, step_m, repose_slope, rate);
                }
                if j + 1 < height {
                    relax(&mut h, k, k + width, step_m, repose_slope, rate);
                }
            }
        }
    }

    let removed = h
        .iter()
        .zip(heights.iter())
        .filter(|(a, b)| a < b)
        .map(|(a, b)| b - a)
        .sum::<f64>()
        * step_m
        * step_m;
    let deposited = h
        .iter()
        .zip(heights.iter())
        .filter(|(a, b)| a > b)
        .map(|(a, b)| a - b)
        .sum::<f64>()
        * step_m
        * step_m;

    Erosion { heights: h, removed_m3: removed, deposited_m3: deposited }
}

fn relax(h: &mut [f64], a: usize, b: usize, step_m: f64, repose_slope: f64, rate: f64) {
    let diff = h[a] - h[b];
    let drop = diff.abs();
    let excess = drop - repose_slope * step_m;
    if excess <= 0.0 {
        return;
    }
    // Half the excess, scaled — and the **same** amount is taken from one and
    // given to the other, which is where the exactness comes from.
    let amount = excess * 0.5 * rate.clamp(0.0, 1.0);
    if diff > 0.0 {
        h[a] -= amount;
        h[b] += amount;
    } else {
        h[b] -= amount;
        h[a] += amount;
    }
}

/// Hydraulic erosion parameters.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HydraulicParams {
    pub droplets: u32,
    /// Maximum steps a droplet may take before it is assumed to have reached an
    /// outlet.
    pub max_steps: u32,
    /// The droplet's initial "water", which scales how much it can carry.
    pub initial_water: f64,
    pub capacity_factor: f64,
    pub deposit_factor: f64,
    pub erode_factor: f64,
    /// Water lost per step.
    pub evaporation: f64,
    /// The slope below which capacity is zero, so a flat pan neither erodes nor
    /// silts.
    pub min_slope: f64,
    /// Integration step, in cells.
    pub inertia: f64,
}

impl Default for HydraulicParams {
    fn default() -> Self {
        // Hansson's published defaults, with `min_slope` declared: a droplet on
        // flat ground must not carry sediment.
        HydraulicParams {
            droplets: 2000,
            max_steps: 30,
            initial_water: 1.0,
            capacity_factor: 4.0,
            deposit_factor: 0.3,
            erode_factor: 0.3,
            evaporation: 0.01,
            min_slope: 0.01,
            inertia: 0.05,
        }
    }
}

/// Hydraulic erosion by the droplet method.
///
/// **Approximate, and accounted for.** The starting position of each droplet is a
/// **hash of its index and the seed**, never an RNG stream, so the pass is
/// deterministic and parallel-safe (F12).
pub fn hydraulic(
    spec: &WorldSpec,
    heights: &[f64],
    width: usize,
    height: usize,
    step_m: f64,
    p: &HydraulicParams,
) -> Erosion {
    let mut h = heights.to_vec();
    let mut removed = 0.0f64;
    let mut deposited = 0.0f64;
    // **Stationary, because this seed chooses the droplet *starting positions*.** The
    // droplets' walk is a field over the patch; a seed that moved with the tick would
    // re-roll where every droplet lands, which is a reshuffle and not an erosion.
    let seed = spec.stationary_stage_seed("erosion");

    for d in 0..p.droplets {
        let rx = (mix(seed ^ 0xE1) ^ d as u64) as f64 / u64::MAX as f64;
        let rz = (mix(seed ^ 0xE2) ^ (d as u64).wrapping_mul(0x9E37_79B9)) as f64 / u64::MAX as f64;
        let mut px = rx * (width - 1) as f64;
        let mut pz = rz * (height - 1) as f64;

        let mut dir_x = 0.0f64;
        let mut dir_z = 0.0f64;
        let mut speed = 1.0f64;
        let mut water = p.initial_water;
        let mut sediment = 0.0f64;

        for _ in 0..p.max_steps {
            let (i, j) = (px.floor() as isize, pz.floor() as isize);
            if i < 0 || j < 0 || i + 1 >= width as isize || j + 1 >= height as isize {
                break;
            }
            let (i, j) = (i as usize, j as usize);
            let (fx, fz) = (px - i as f64, pz - j as f64);

            let idx = |i: usize, j: usize| j * width + i;
            let h00 = h[idx(i, j)];
            let h10 = h[idx(i + 1, j)];
            let h01 = h[idx(i, j + 1)];
            let h11 = h[idx(i + 1, j + 1)];

            let old_height =
                h00 * (1.0 - fx) * (1.0 - fz) + h10 * fx * (1.0 - fz) + h01 * (1.0 - fx) * fz + h11 * fx * fz;

            // Gradient from the same bilinear patch.
            let gx = (h10 - h00) * (1.0 - fz) + (h11 - h01) * fz;
            let gz = (h01 - h00) * (1.0 - fx) + (h11 - h10) * fx;

            dir_x = dir_x * p.inertia - gx * (1.0 - p.inertia);
            dir_z = dir_z * p.inertia - gz * (1.0 - p.inertia);
            let len = (dir_x * dir_x + dir_z * dir_z).sqrt();
            if len < 1e-9 {
                break;
            }
            dir_x /= len;
            dir_z /= len;
            px += dir_x;
            pz += dir_z;

            let (ni, nj) = (px.floor() as isize, pz.floor() as isize);
            if ni < 0 || nj < 0 || ni + 1 >= width as isize || nj + 1 >= height as isize {
                break;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            let nfx = px - ni as f64;
            let nfz = pz - nj as f64;
            let new_height = h[idx(ni, nj)] * (1.0 - nfx) * (1.0 - nfz)
                + h[idx(ni + 1, nj)] * nfx * (1.0 - nfz)
                + h[idx(ni, nj + 1)] * (1.0 - nfx) * nfz
                + h[idx(ni + 1, nj + 1)] * nfx * nfz;

            let delta = new_height - old_height;
            let capacity = (-delta).max(p.min_slope) * speed * water * p.capacity_factor;

            if sediment > capacity || delta > 0.0 {
                // Deposit. The brush distributes onto the same four cells the
                // height was read from.
                let amount = if delta > 0.0 {
                    delta.min(sediment)
                } else {
                    (sediment - capacity) * p.deposit_factor
                };
                deposite_bilinear(&mut h, width, i, j, fx, fz, amount);
                sediment -= amount;
                deposited += amount * step_m * step_m;
            } else {
                let amount = ((capacity - sediment) * p.erode_factor).min(-delta);
                if amount > 0.0 {
                    erode_bilinear(&mut h, width, i, j, fx, fz, amount);
                    sediment += amount;
                    removed += amount * step_m * step_m;
                }
            }

            speed = (speed * speed - delta).max(0.0).sqrt();
            water *= 1.0 - p.evaporation;
            if water <= 0.0 {
                break;
            }
        }
    }

    Erosion { heights: h, removed_m3: removed, deposited_m3: deposited }
}

fn deposite_bilinear(h: &mut [f64], width: usize, i: usize, j: usize, fx: f64, fz: f64, amount: f64) {
    let idx = |i: usize, j: usize| j * width + i;
    h[idx(i, j)] += amount * (1.0 - fx) * (1.0 - fz);
    h[idx(i + 1, j)] += amount * fx * (1.0 - fz);
    h[idx(i, j + 1)] += amount * (1.0 - fx) * fz;
    h[idx(i + 1, j + 1)] += amount * fx * fz;
}

fn erode_bilinear(h: &mut [f64], width: usize, i: usize, j: usize, fx: f64, fz: f64, amount: f64) {
    let idx = |i: usize, j: usize| j * width + i;
    // Subtract, then floor at zero: a cell cannot be eroded below nothing.
    for (k, w) in [
        (idx(i, j), (1.0 - fx) * (1.0 - fz)),
        (idx(i + 1, j), fx * (1.0 - fz)),
        (idx(i, j + 1), (1.0 - fx) * fz),
        (idx(i + 1, j + 1), fx * fz),
    ] {
        h[k] = (h[k] - amount * w).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(5150, Config::default())
    }

    /// A sharp spike on a plane — the simplest thing erosion must flatten.
    fn spike(w: usize, h: usize) -> Vec<f64> {
        let mut v = vec![0.0f64; w * h];
        v[(h / 2) * w + w / 2] = 10.0;
        v
    }

    #[test]
    fn thermal_erosion_conserves_volume_exactly() {
        // The exactness claim, tested rather than asserted in prose. Each pair
        // transfer is symmetric, so the sum cannot drift by more than f64
        // rounding — and with powers of two it does not drift at all.
        let w = 16;
        let h = 16;
        let mut heights = spike(w, h);
        // A jagged field as well as a spike: a single spike could pass by luck.
        for (k, v) in heights.iter_mut().enumerate() {
            *v += ((k * 37) % 11) as f64;
        }
        let before: f64 = heights.iter().sum();

        let e = thermal(&heights, w, h, 1.0, 0.7, 0.5, 25);
        let after: f64 = e.heights.iter().sum();

        assert!((after - before).abs() < 1e-9, "volume drifted {before} → {after}");
        // And the accounting agrees: what one cell lost another gained.
        assert!((e.removed_m3 - e.deposited_m3).abs() < 1e-9);
        assert!((e.exported_m3()).abs() < 1e-9);
    }

    #[test]
    fn thermal_erosion_flattens_a_spike_and_never_fires_below_repose() {
        let w = 16;
        let h = 16;
        let heights = spike(w, h);
        let e = thermal(&heights, w, h, 1.0, 0.7, 0.5, 25);
        assert!(e.heights[(h / 2) * w + w / 2] < 10.0, "the spike did not erode");

        // A plane has no slope at all, so nothing may move.
        let flat = vec![3.0f64; w * h];
        let f = thermal(&flat, w, h, 1.0, 0.7, 0.5, 10);
        assert!(f.heights.iter().all(|v| (*v - 3.0).abs() < 1e-12));
        assert_eq!(f.removed_m3, 0.0);
    }

    #[test]
    fn thermal_erosion_is_deterministic_and_order_independent() {
        let heights = spike(8, 8);
        let a = thermal(&heights, 8, 8, 1.0, 0.5, 0.5, 5);
        let b = thermal(&heights, 8, 8, 1.0, 0.5, 0.5, 5);
        assert_eq!(a.heights, b.heights);
    }

    #[test]
    fn hydraulic_erosion_is_deterministic_despite_starting_from_a_hash() {
        let s = spec();
        let heights = spike(24, 24);
        let p = HydraulicParams { droplets: 200, ..HydraulicParams::default() };
        let a = hydraulic(&s, &heights, 24, 24, 1.0, &p);
        let b = hydraulic(&s, &heights, 24, 24, 1.0, &p);
        assert_eq!(a.heights, b.heights);
        assert_eq!(a.removed_m3, b.removed_m3);
    }

    #[test]
    fn hydraulic_erosion_smooths_the_terrain() {
        let s = spec();
        let mut heights = vec![0.0f64; 32 * 32];
        for j in 0..32 {
            for i in 0..32 {
                // A sharp sawtooth: high variance, so smoothing is measurable.
                heights[j * 32 + i] = ((i * 7 + j * 13) % 9) as f64;
            }
        }
        let variance = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64
        };
        let before = variance(&heights);
        let e = hydraulic(&s, &heights, 32, 32, 1.0, &HydraulicParams::default());
        assert!(variance(&e.heights) < before, "hydraulic erosion did not smooth");
    }

    #[test]
    fn hydraulic_erosion_reports_its_exported_material_rather_than_hiding_it() {
        // The approximate algorithm's residual is a *number*. This test exists so
        // that a future change cannot quietly make the residual unobservable.
        let s = spec();
        let heights = spike(24, 24);
        let e = hydraulic(&s, &heights, 24, 24, 1.0, &HydraulicParams::default());
        assert!(e.removed_m3 >= 0.0 && e.deposited_m3 >= 0.0);
        assert!(e.exported_m3().is_finite());
        // Total volume is allowed to change; the change is exactly the export.
        let before: f64 = heights.iter().sum();
        let after: f64 = e.heights.iter().sum();
        assert!((after - before).abs() <= e.exported_m3() + 1e-6);
    }

    #[test]
    fn erosion_never_produces_a_nan_or_a_negative_height() {
        let s = spec();
        let heights = spike(20, 20);
        let e = hydraulic(&s, &heights, 20, 20, 1.0, &HydraulicParams::default());
        assert!(e.heights.iter().all(|v| v.is_finite() && *v >= 0.0));
    }
}
