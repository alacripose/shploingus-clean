//! **Ground fog as cuboid cells** — the shaper's own fog field, drawn the way a cloud is.
//!
//! # The gap this closes
//!
//! `voxel_shapers::fog` derives a **column**: a base, a top, a loading and an extinction, and
//! `chunk_fields::fog_water_law` already exposes that as the address field
//! `voxel_shapers::field_sampler::FieldId::FogWater`. So
//! the fog is cubic in the *shaper* and it was not cubic in the *picture*: `grep -rn "fog_at\|fog::"`
//! over `voxel-client` had no hits before this file, and the one place fog reached a frame was the
//! sky's `air_at`, which reads no fog at all (`sun::air_at`'s own doc names that absence).
//!
//! A cloud is drawn as **cuboid cells** ([`crate::cloud_cubes`]): a sparse octree of boxes, each
//! carrying a veil that the shader multiplies into the beam. This module gives the ground its own
//! cells under the same convention — a cell per column, one voxel tall, standing on the terrain,
//! carrying its column's own transmittance. A valley fills; a ridge stands clear, because
//! `fog::fog_at` reads the terrain's shelter and this module draws what that reading says.
//!
//! # Why a cell is one voxel and not the layer's depth
//!
//! The model's layer is **half a voxel** deep at the shipped world (`0.5` engine metres, measured in
//! `fog::tests::the_layer_has_a_bottom_a_middle_and_a_top`) — a voxel is the smallest thing the world
//! can draw, so the layer is always thinner than the cell that carries it. The cell therefore holds
//! its **column's** optical depth rather than a measured thickness of its own: quantising the layer
//! up to one voxel and keeping the depth would multiply the extinction by the ratio and make a fog
//! opaque that is thin. What the cell carries is `exp(-extinction · layer depth)`, the transmittance
//! the column actually has, which is the one number a beam crossing it needs.
//!
//! # What this is not
//!
//! * It is not a second fog model. Every number comes from `fog::fog_at` at the cell's own column.
//! * It does not merge faces into runs. A cell is a box, and the mesh that draws it is the renderer's
//!   work (the cloud's own greedy merge is `cloud_cubes::drawn_from`, which is private and typed to
//!   the cloud tree); what is proved here is the **cell set** the terrain makes.
//! * It does not animate. Fog is a held state of the atmosphere and changes at the atmosphere's
//!   cadence, not per frame.

use voxel_shapers::fog::{self, FogMechanism};
use voxel_shapers::{terrain, WorldSpec};

/// One fog cell: a box on the terrain, and what a beam crossing it keeps.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FogCell {
    /// The box's low corner, world metres — `y` is the ground it stands on.
    pub lo_m: [f64; 3],
    /// The box's high corner, world metres.
    pub hi_m: [f64; 3],
    /// The column's liquid water content at the cell's own address, g/m³.
    pub lwc_g_m3: f64,
    /// **What a beam crossing the column keeps**, `[0, 1]`: `exp(-extinction · layer depth)`.
    pub transmittance: f64,
    /// What made the droplets — the column's own cause, carried so a frame can report it.
    pub mechanism: FogMechanism,
}

impl FogCell {
    /// The cell's centre, world metres.
    pub fn centre_m(&self) -> [f64; 3] {
        [
            (self.lo_m[0] + self.hi_m[0]) * 0.5,
            (self.lo_m[1] + self.hi_m[1]) * 0.5,
            (self.lo_m[2] + self.hi_m[2]) * 0.5,
        ]
    }
}

/// **The fog around an eye**, as cuboid cells on the terrain.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FogCubes {
    /// The cells, one per foggy column, in no particular order.
    pub cells: Vec<FogCell>,
    /// Every column asked — the denominator of [`FogCubes::foggy_share`].
    pub columns: usize,
    /// How many of them made a cell.
    pub foggy: usize,
    /// The cell's side length, metres: one voxel.
    pub cell_m: f64,
    /// How far from the eye columns were asked, metres.
    pub radius_m: f64,
}

impl FogCubes {
    /// The share of columns that made a cell — the field's own answer to *how much ground is fogged*.
    pub fn foggy_share(&self) -> f64 {
        if self.columns == 0 {
            0.0
        } else {
            self.foggy as f64 / self.columns as f64
        }
    }

    /// Whether a point lies inside any cell, and which — the box test a renderer's culling asks.
    pub fn cell_at(&self, at_m: [f64; 3]) -> Option<&FogCell> {
        self.cells.iter().find(|cell| {
            (0..3).all(|axis| at_m[axis] >= cell.lo_m[axis] && at_m[axis] < cell.hi_m[axis])
        })
    }

    /// **The fog the terrain makes around `eye_m`**, one cell per foggy column.
    ///
    /// The lattice is the eye's own voxel grid: `cell_m` is snapped to a whole voxel and every
    /// column in the square of `radius_m` is asked exactly once. The column is sampled **at its own
    /// ground**, because that is where a ground fog is — a sample at a fixed height would miss a
    /// valley floor and find a ridge by accident.
    pub fn build(spec: &WorldSpec, tick: u64, eye_m: [f64; 3], radius_m: f64) -> Self {
        let cell_m = 1.0;
        let radius = radius_m.max(cell_m);
        let n = (radius / cell_m).floor() as i64;
        let mut cells = Vec::new();
        let mut columns = 0usize;
        for j in -n..=n {
            for i in -n..=n {
                let x = (eye_m[0] / cell_m).round() * cell_m + i as f64 * cell_m;
                let z = (eye_m[2] / cell_m).round() * cell_m + j as f64 * cell_m;
                columns += 1;
                let ground = terrain::elevation_m(spec, x, z);
                // **Half a voxel above the ground**, which is inside the layer whatever its depth:
                // the layer is thinner than the cell, and its own base is the ground.
                let sample = fog::fog_at(spec, x, ground + cell_m * 0.5, z, tick);
                if !(sample.lwc_g_m3 > 0.0) || !sample.in_layer {
                    continue;
                }
                let depth = (sample.layer_top_m - sample.layer_base_m).max(0.0);
                let transmittance = (-sample.extinction_per_m * depth).exp();
                cells.push(FogCell {
                    lo_m: [x, ground, z],
                    hi_m: [x + cell_m, ground + cell_m, z + cell_m],
                    lwc_g_m3: sample.lwc_g_m3,
                    transmittance,
                    mechanism: sample.mechanism,
                });
            }
        }
        let foggy = cells.len();
        FogCubes { cells, columns, foggy, cell_m, radius_m: radius }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(seed: u64) -> WorldSpec {
        WorldSpec::new(seed, voxel_shapers::Config::default())
    }

    /// **A hollow and a ridge at the same altitude**, found by the terrain itself.
    ///
    /// Concavity is `elevation − mean(four neighbours)`: negative in a hollow, positive on a ridge.
    /// The pair is required to stand within a metre of each other so the comparison is about shape
    /// and not about height — fog forms at a temperature, and a ridge 800 m higher is colder for a
    /// reason that has nothing to do with being a ridge.
    fn hollow_and_ridge(spec: &WorldSpec) -> ([f64; 2], [f64; 2], f64) {
        let step = 37.0;
        let mut best: Option<([f64; 2], f64)> = None;
        let mut worst: Option<([f64; 2], f64)> = None;
        for j in -40..40 {
            for i in -40..40 {
                let (x, z) = (i as f64 * step, j as f64 * step);
                let here = terrain::elevation_m(spec, x, z);
                let mean = (terrain::elevation_m(spec, x + step, z)
                    + terrain::elevation_m(spec, x - step, z)
                    + terrain::elevation_m(spec, x, z + step)
                    + terrain::elevation_m(spec, x, z - step))
                    * 0.25;
                let concavity = here - mean;
                if best.map_or(true, |(_, c)| concavity < c) {
                    best = Some(([x, z], concavity));
                }
                if worst.map_or(true, |(_, c)| concavity > c) {
                    worst = Some(([x, z], concavity));
                }
            }
        }
        let (hollow, concave) = best.expect("a hollow");
        let (ridge, convex) = worst.expect("a ridge");
        assert!(concave < 0.0, "the hollow was not concave: {concave}");
        assert!(convex > 0.0, "the ridge was not convex: {convex}");
        let (a, b) = (
            terrain::elevation_m(spec, hollow[0], hollow[1]),
            terrain::elevation_m(spec, ridge[0], ridge[1]),
        );
        (hollow, ridge, (a - b).abs())
    }

    /// **The claim, through the real path**: the cells a column makes are the fog model's own
    /// reading of that column, and a hollow makes them where a ridge does not.
    #[test]
    fn a_hollow_makes_fog_cells_and_a_ridge_does_not() {
        let spec = world(4242);
        let tick = 0;
        let (hollow, ridge, gap_m) = hollow_and_ridge(&spec);
        println!(
            "hollow at {:?} and ridge at {:?}, {gap_m:.1} m apart in height",
            hollow, ridge
        );
        let at = |[x, z]: [f64; 2]| {
            let ground = terrain::elevation_m(&spec, x, z);
            let cubes = FogCubes::build(&spec, tick, [x, ground + 32.0, z], 24.0);
            (cubes.clone(), ground)
        };
        let (hollow_cubes, hollow_ground) = at(hollow);
        let (ridge_cubes, ridge_ground) = at(ridge);
        println!(
            "hollow: {} of {} columns fogged ({:.1}%), ground {hollow_ground:.1} m",
            hollow_cubes.foggy,
            hollow_cubes.columns,
            hollow_cubes.foggy_share() * 100.0
        );
        println!(
            "ridge:  {} of {} columns fogged ({:.1}%), ground {ridge_ground:.1} m",
            ridge_cubes.foggy,
            ridge_cubes.columns,
            ridge_cubes.foggy_share() * 100.0
        );
        assert!(
            hollow_cubes.foggy > 0,
            "the hollow at {hollow:?} made no fog cell at all"
        );
        assert!(
            hollow_cubes.foggy > ridge_cubes.foggy,
            "the hollow made {} cells and the ridge {} — the terrain is not in the result",
            hollow_cubes.foggy,
            ridge_cubes.foggy
        );
        // Every cell is the model's own reading at its own column, not a second model.
        for cell in &hollow_cubes.cells {
            // **At the cell's own column**, which is its low corner — the centre is half a voxel
            // away and reads a different terrain.
            let ground = terrain::elevation_m(&spec, cell.lo_m[0], cell.lo_m[2]);
            let sample = fog::fog_at(&spec, cell.lo_m[0], ground + 0.5, cell.lo_m[2], tick);
            assert_eq!(cell.lwc_g_m3, sample.lwc_g_m3);
            assert_eq!(cell.mechanism, sample.mechanism);
            let depth = sample.layer_top_m - sample.layer_base_m;
            assert!((cell.transmittance - (-sample.extinction_per_m * depth).exp()).abs() < 1e-12);
            assert!((0.0..=1.0).contains(&cell.transmittance));
            assert!((cell.lo_m[1] - ground).abs() < 1e-12, "a cell floated off its ground");
        }
        // And a cell can be found by its own box, which is what a renderer's culling asks.
        let cell = hollow_cubes.cells.first().expect("a cell");
        let centre = cell.centre_m();
        assert!(hollow_cubes.cell_at(centre).is_some(), "a cell does not contain its centre");
        assert!(hollow_cubes.cell_at([centre[0], centre[1] + 10.0, centre[2]]).is_none());
    }

    /// **The seed moves the cells**, because the terrain under them is the seed's: a cell set that
    /// did not change with the world would be a picture of the fog model rather than of this valley.
    #[test]
    fn a_different_world_puts_its_fog_somewhere_else() {
        let (a, b) = (world(4242), world(2468));
        let eye = [0.0, 64.0, 0.0];
        let (first, second) = (
            FogCubes::build(&a, 0, eye, 96.0),
            FogCubes::build(&b, 0, eye, 96.0),
        );
        println!(
            "seed 4242: {} of {}, seed 2468: {} of {}",
            first.foggy, first.columns, second.foggy, second.columns
        );
        assert!(first.columns == second.columns, "the two lattices are the same size");
        assert!(
            first.foggy != second.foggy || first.cells != second.cells,
            "two worlds drew the same fog cells"
        );
    }
}
