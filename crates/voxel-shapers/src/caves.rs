//! Caves — karst dissolution.
//!
//! **Source.** Ford, D. & Williams, P. *Karst Hydrogeology and Geomorphology.*
//! Wiley 2007. The model is their dissolution picture reduced to what a
//! generator can use: water enters along **fractures**, dissolves preferentially
//! where the rock is **permeable**, and the dissolving front stops at the **water
//! table**, below which the water is already saturated and the voids fill.
//!
//! # Why caves are not just 3D noise
//!
//! A void field carved by `noise > 0.6` makes *sponges*: a world riddled with
//! disconnected pockets that no player can navigate and no hydrology connects. A
//! karst is a **network**: voids concentrate on fractures (the Worley cell edges
//! below) and they connect, because fractures connect. That is the difference the
//! shaper buys, and it is why the fracture term is a cellular *boundary* rather
//! than a third blob field.
//!
//! The three couplings — permeability, water table and fracture network — are all
//! **declared inputs**, not lookups into a table of constants, because they come
//! from other stages (WG-Q5).

use crate::geology::Lithology;
use crate::spec::WorldSpec;
use voxel_noise::{Fractal, Kind, Vec3};

/// Cave carving parameters, per world.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveParams {
    /// How dissolved a cell must be to be a void.
    pub threshold: f64,
    /// The water table's height above the sea, in metres. Voids below it are
    /// declared **flooded** rather than absent — see [`Void::flooded`].
    pub water_table_above_sea_m: f64,
    /// The scale of the fracture network, in metres.
    pub fracture_scale_m: f64,
    /// How far the fracture term reaches, as a fraction of a cell.
    pub fracture_width: f64,
}

impl Default for CaveParams {
    fn default() -> Self {
        CaveParams {
            threshold: 0.40,
            water_table_above_sea_m: 2.0,
            fracture_scale_m: 42.0,
            fracture_width: 0.30,
        }
    }
}

impl CaveParams {
    /// The cave parameters a world's own configuration asks for.
    ///
    /// Derived from the spec rather than defaulted so they **travel with the world**:
    /// two worlds with different dials carve different networks, and a caller reading
    /// a voxel cannot accidentally use the library default and disagree with the
    /// chunk build about where a void is.
    pub fn for_spec(spec: &crate::spec::WorldSpec) -> Self {
        CaveParams { threshold: spec.config.cave_threshold, ..CaveParams::default() }
    }
}

/// What is at a cell underground.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Void {
    /// Solid rock.
    Rock,
    /// An open void above the water table.
    Open,
    /// A void **below** the water table. There is no air here; there is water.
    ///
    /// Distinct from `Rock` on purpose: collapsing the two is how a cave
    /// network ends up visually identical to a sponge, and how a player drowns in
    /// what the display called stone.
    Flooded,
}

impl Void {
    pub const fn is_solid(self) -> bool {
        matches!(self, Void::Rock)
    }

    pub const fn is_air(self) -> bool {
        matches!(self, Void::Open)
    }
}

/// How dissolved a cell is, in `[0, 1]`.
///
/// A fracture is a **crease**, and Musgrave's `1 − |n|` fold is the primitive that
/// produces creases: its maxima lie along curves, which is exactly the shape a
/// fracture network has.
///
/// **A Worley field was tried first and carved nothing.** Normalised `F1`
/// distance is almost never small, so `1 − d/width` was zero at essentially every
/// sample and the world had no caves at all. The test that counts open cells
/// caught it; the choice is recorded here because the natural-looking primitive
/// was the wrong one.
pub fn dissolution(spec: &WorldSpec, lith: Lithology, x: f64, y: f64, z: f64, p: &CaveParams) -> f64 {
    let fracture_scale = 1.0 / p.fracture_scale_m.max(1.0);

    // **Stationary**: the fracture network is a field over position. A seed that moved
    // with the tick would open and close every cave when the clock advanced.
    let ridged = Fractal::new(4, 2.0, 0.5, Kind::Perlin).ridged(
        spec.stationary_stage_seed("caves"),
        Vec3::new(x, y, z) * fracture_scale,
    );
    // `fracture_width` is the ridge height above the 0.5 baseline at which a
    // crease becomes a full fracture.
    let fracture = ((ridged - 0.5) / p.fracture_width.max(0.05)).clamp(0.0, 1.0);

    // A slow field so the network is not uniform: some regions are more soluble.
    let f = Fractal::new(3, 2.0, 0.5, Kind::Perlin);
    let region01 = f.fbm(
        spec.stationary_stage_seed("caves_region"),
        Vec3::new(x * fracture_scale * 0.12, y * fracture_scale * 0.12, z * fracture_scale * 0.12),
    ) * 0.5
        + 0.5;
    let region = 0.55 + 0.45 * region01;

    // Permeability is the rock's own property, declared on the lithology rather
    // than looked up by id: a new rock class cannot forget to say how porous it
    // is. The 0.35 floor keeps a *little* dissolution possible in tight rock
    // rather than making it exactly zero, which would make an igneous world
    // perfectly solid and hide the difference between "resists" and "cannot".
    let permeability = lith.permeability();
    let pore = 0.35 + 0.65 * permeability;

    (fracture * region * pore).clamp(0.0, 1.0)
}

/// What is at a point underground.
pub fn void_at(
    spec: &WorldSpec,
    lith: Lithology,
    x: f64,
    y: f64,
    z: f64,
    surface_m: f64,
    p: &CaveParams,
) -> Void {
    // The surface itself and the soil above it are not carved: an entrance has to
    // exist somewhere, but a countryside perforated at the surface is not a karst,
    // it is a sieve.
    let depth = surface_m - y;
    if depth < 1.0 {
        return Void::Rock;
    }

    if dissolution(spec, lith, x, y, z, p) <= p.threshold {
        return Void::Rock;
    }

    let water_table = spec.config.sea_level_m + p.water_table_above_sea_m;
    if y < water_table {
        Void::Flooded
    } else {
        Void::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(8080, Config::default())
    }

    #[test]
    fn dissolution_is_in_range_and_respects_permeability() {
        let s = spec();
        let p = CaveParams::default();
        let mut impermeable_votes = 0.0;
        let mut permeable_votes = 0.0;
        let mut z = 0.0;
        while z < 300.0 {
            let mut y = 0.0;
            while y < 200.0 {
                let d_ig = dissolution(&s, Lithology::Igneous, 10.0, y, z, &p);
                let d_re = dissolution(&s, Lithology::Regolith, 10.0, y, z, &p);
                assert!((0.0..=1.0).contains(&d_ig) && (0.0..=1.0).contains(&d_re));
                assert!(d_re >= d_ig, "regolith was less soluble than igneous rock");
                impermeable_votes += d_ig;
                permeable_votes += d_re;
                y += 7.0;
            }
            z += 7.0;
        }
        assert!(permeable_votes > impermeable_votes * 2.0, "permeability barely mattered");
    }

    #[test]
    fn voids_are_open_above_the_water_table_and_flooded_below_it() {
        let s = spec();
        let p = CaveParams::default();
        let surface = 40.0;
        let mut saw_open = false;
        let mut saw_flooded = false;
        let mut z = 0.0;
        while z < 400.0 && !(saw_open && saw_flooded) {
            let mut y = 0.0;
            while y < surface {
                match void_at(&s, Lithology::Sedimentary, 10.0, y, z, surface, &p) {
                    Void::Open => saw_open = true,
                    Void::Flooded => saw_flooded = true,
                    Void::Rock => {}
                }
                y += 1.0;
            }
            z += 1.5;
        }
        assert!(saw_open, "no open void was ever carved");
        assert!(saw_flooded, "no flooded void was ever carved");
    }

    #[test]
    fn rock_is_rock_at_the_surface_and_one_metre_down() {
        let s = spec();
        let p = CaveParams::default();
        let surface = 30.0;
        for z in 0..50 {
            assert_eq!(
                void_at(&s, Lithology::Sedimentary, 0.0, surface, z as f64, surface, &p),
                Void::Rock,
                "the surface was carved at z = {z}"
            );
            assert_eq!(
                void_at(&s, Lithology::Sedimentary, 0.0, surface - 0.5, z as f64, surface, &p),
                Void::Rock
            );
        }
    }

    #[test]
    fn cavities_form_a_connected_network_rather_than_a_sponge() {
        // The property the fracture term exists for: voids must be adjacent to
        // one another along the fracture, not sprinkled independently. Measure
        // the fraction of open cells that have an open neighbour.
        let s = spec();
        let p = CaveParams { threshold: 0.45, ..CaveParams::default() };
        let n = 48usize;
        let surface = 40.0;
        let mut open = vec![false; n * n * n];
        for k in 0..n {
            for j in 0..n {
                for i in 0..n {
                    let v = void_at(
                        &s,
                        Lithology::Sedimentary,
                        i as f64 * 3.0,
                        (j as f64 * 3.0) + 1.0,
                        k as f64 * 3.0,
                        surface,
                        &p,
                    );
                    open[(k * n + j) * n + i] = v.is_air();
                }
            }
        }
        let idx = |i: usize, j: usize, k: usize| (k * n + j) * n + i;
        let mut open_count = 0;
        let mut connected = 0;
        for k in 1..n - 1 {
            for j in 1..n - 1 {
                for i in 1..n - 1 {
                    if !open[idx(i, j, k)] {
                        continue;
                    }
                    open_count += 1;
                    let neighbours = [
                        open[idx(i + 1, j, k)],
                        open[idx(i - 1, j, k)],
                        open[idx(i, j + 1, k)],
                        open[idx(i, j - 1, k)],
                        open[idx(i, j, k + 1)],
                        open[idx(i, j, k - 1)],
                    ];
                    if neighbours.iter().any(|b| *b) {
                        connected += 1;
                    }
                }
            }
        }
        assert!(open_count > 20, "only {open_count} open cells to judge");
        let frac = connected as f64 / open_count as f64;
        assert!(
            frac > 0.6,
            "only {frac:.2} of open cells touch another: this is a sponge, not a cave"
        );
    }

    #[test]
    fn carving_is_deterministic() {
        let s = spec();
        let p = CaveParams::default();
        assert_eq!(
            void_at(&s, Lithology::Sedimentary, 1.0, 10.0, 2.0, 30.0, &p),
            void_at(&s, Lithology::Sedimentary, 1.0, 10.0, 2.0, 30.0, &p)
        );
    }
}
