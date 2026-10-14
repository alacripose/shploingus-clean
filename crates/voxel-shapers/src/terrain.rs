//! Terrain — the elevation surface.
//!
//! **Source.** Mandelbrot, B. *The Fractal Geometry of Nature.* 1982, for the
//! fBm-relief premise; Musgrave, Kolb & Mace. *The Synthesis and Rendering of
//! Eroded Fractal Terrains.* SIGGRAPH 1989, for the **ridged** term that gives a
//! mountain range crests instead of domes.
//!
//! # The coupling, which is the point (WG-Q5)
//!
//! Elevation is **not** an fBm field with a mountain-shaped noise added. It is
//!
//! ```text
//! elevation = datum + continental_fbm · relief/2
//!                   + tectonic_uplift
//!                   + ridged_mountains · relief · convergent_strength
//! ```
//!
//! where the third term is **gated by the tectonic term**. Without the gate,
//! mountains appear everywhere relief is high, including in the middle of a
//! stable plate, and that is precisely the "independent noise" failure WG-Q5
//! names. With the gate, they appear along convergent boundaries — where the
//! world's own history put them — and the terrain is a *consequence* of the
//! plate map rather than a decoration on top of it.

use crate::spec::WorldSpec;
use crate::tectonics::{self, BoundaryKind};
use voxel_noise::{Fractal, Kind, Vec3};

/// Everything terrain says about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TerrainSample {
    /// Surface height above the datum, in metres.
    pub elevation_m: f64,
    /// The sea-level-relative height: negative is below the water line.
    pub above_sea_m: f64,
    /// The magnitude of the horizontal gradient, in metres per metre.
    pub slope: f64,
    /// The unit surface normal.
    pub normal: Vec3,
    /// True when the surface is above the sea.
    pub is_land: bool,
    /// The tectonic uplift that contributed, kept for the debug lanes and for the
    /// coupling test.
    pub uplift_m: f64,
    /// How much of this column's relief came from the ridged mountain term.
    pub mountain_m: f64,
}

/// The continental field's fractal.
fn continental(spec: &WorldSpec) -> Fractal {
    // Five octaves: enough to make a coastline ragged, few enough that a
    // continent stays a continent.
    Fractal::new(
        spec.config.layer_count.clamp(3, 7),
        2.0,
        0.5,
        Kind::Perlin,
    )
}

/// Surface elevation at a column, in metres above the datum.
pub fn elevation_m(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let cfg = &spec.config;
    let f = cfg.continent_frequency;

    // **`stationary_stage_seed`, and this is what makes the world tick.** Terrain is a
    // *field*: its argument is a position, and the tick has no place in that argument —
    // a terrain that moved with the tick would be a terrain that reshuffles, which is
    // the failure `WorldSpec::stationary_stage_seed` exists to prevent (and which
    // `tick-over-world` measured as 270 of 270 surface voxels changing when the spec
    // tick moved). The world's own history — erosion, deposition, the earthworks
    // record — is what is *supposed* to move with the tick, and it lives elsewhere.
    let cont = continental(spec).fbm(
        spec.stationary_stage_seed("terrain"),
        Vec3::new(x * f, 0.0, z * f),
    );

    let plate = tectonics::sample(spec, x, z);

    // Mountains are **gated** on convergence, not multiplied by the uplift's
    // magnitude: a divergent boundary at high boundary_strength must not raise
    // ridges just because it is a boundary.
    let convergent = if plate.boundary_kind == BoundaryKind::Convergent {
        plate.boundary_strength
    } else {
        0.0
    };

    let mountains = if convergent > 0.0 {
        let ridged = Fractal::new(5, 2.0, 0.5, Kind::Perlin);
        // Higher frequency than the continents: ranges are narrow and tall.
        // A **field**, so the same stationary seed as the continents above.
        let r = ridged.ridged(
            spec.stationary_stage_seed("mountains"),
            Vec3::new(x * f * 6.0, 0.0, z * f * 6.0),
        );
        r * cfg.relief_m * 0.55 * convergent
    } else {
        0.0
    };

    cfg.base_elevation_m + cont * cfg.relief_m * 0.5 + plate.uplift_m + mountains
}

/// The **continental term** of the elevation law at a column, in metres: exactly the
/// `cont · relief_m · 0.5` contribution [`elevation_m`] adds, from the same fractal, the
/// same seed and the same frequency.
///
/// **Why this is public, and not a second field.** The `earth_biomes_subbiomes_continentalness_mountains.pdf`
/// atlas asks a climate for *continentalness* — how deep inside a landmass a column sits — and the only
/// honest source of that here is the field that already decides where the land **is**. A reader that
/// re-derived it from a new fractal would be a second, unrelated coastline (the WG-Q5 failure this
/// crate's rules name). So the term is exposed where it is computed, and [`crate::climate::continentality`]
/// reads it. It carries the tectonic uplift and the ridged mountains **not at all**: those are local
/// relief, and continentalness is the regional question of where the ocean is.
pub fn continental_field_m(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let cfg = &spec.config;
    let f = cfg.continent_frequency;
    continental(spec).fbm(
        spec.stationary_stage_seed("terrain"),
        Vec3::new(x * f, 0.0, z * f),
    ) * cfg.relief_m
        * 0.5
}

/// Whether the **regional** terrain at a column is sea: the continental term plus the base
/// elevation, against the sea line, with uplift and mountains left out on purpose.
///
/// This is the test [`crate::climate::continentality`] runs to its horizon. It is coarser than
/// [`is_solid`] at a voxel — a delta or a lagoon reads as land here — and that is the scale the
/// question is asked at: a coastline is a regional shape, not a voxel.
pub fn is_regional_sea(spec: &WorldSpec, x: f64, z: f64) -> bool {
    spec.config.base_elevation_m + continental_field_m(spec, x, z) < spec.config.sea_level_m
}

/// Sample a column: elevation plus the derived slope and normal.
pub fn sample(spec: &WorldSpec, x: f64, z: f64) -> TerrainSample {
    let elev = elevation_m(spec, x, z);
    let cfg = &spec.config;

    // Central difference at 1 m. A one-sided difference biases the normal toward
    // the sampling direction, which shows up as a light direction that is always
    // slightly wrong.
    const H: f64 = 1.0;
    let dx = (elevation_m(spec, x + H, z) - elevation_m(spec, x - H, z)) / (2.0 * H);
    let dz = (elevation_m(spec, x, z + H) - elevation_m(spec, x, z - H)) / (2.0 * H);

    let slope = (dx * dx + dz * dz).sqrt();

    let plate = tectonics::sample(spec, x, z);
    let convergent = if plate.boundary_kind == BoundaryKind::Convergent {
        plate.boundary_strength
    } else {
        0.0
    };
    let mountain_m = if convergent > 0.0 {
        let ridged = Fractal::new(5, 2.0, 0.5, Kind::Perlin);
        ridged.ridged(
            spec.stationary_stage_seed("mountains"),
            Vec3::new(x * cfg.continent_frequency * 6.0, 0.0, z * cfg.continent_frequency * 6.0),
        ) * cfg.relief_m
            * 0.55
            * convergent
    } else {
        0.0
    };

    // The surface normal of the height field y = h(x, z): (-dh/dx, 1, -dh/dz),
    // normalised.
    let normal = Vec3::new(-dx, 1.0, -dz).normalise();

    let above_sea_m = elev - cfg.sea_level_m;

    TerrainSample {
        elevation_m: elev,
        above_sea_m,
        slope,
        normal,
        is_land: above_sea_m > 0.0,
        uplift_m: plate.uplift_m,
        mountain_m,
    }
}

/// True when a voxel is solid ground.
///
/// The sample point is the voxel **centre** (`y + 0.5`) for the same reason
/// [`Vec3::at_voxel`] is: a generator that samples corners produces a world
/// offset by half a block from the one the mesher draws.
pub fn is_solid(spec: &WorldSpec, x: i32, y: i32, z: i32) -> bool {
    let surface = elevation_m(spec, x as f64 + 0.5, z as f64 + 0.5);
    (y as f64 + 0.5) <= surface
}

/// The sea floor's height, which is the terrain's own height where it is below
/// sea level. Named so that "below sea level" is a decision rather than a
/// comparison repeated at every call site.
pub fn sea_floor_m(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    elevation_m(spec, x, z).min(spec.config.sea_level_m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(31337, Config::default())
    }

    #[test]
    fn sampling_is_deterministic() {
        let s = spec();
        assert_eq!(sample(&s, 12.0, 34.0), sample(&s, 12.0, 34.0));
    }

    #[test]
    fn the_world_has_relief_and_both_land_and_sea() {
        let s = spec();
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut land = 0;
        let mut total = 0;
        let mut x = -6000.0;
        while x < 6000.0 {
            let mut z = -6000.0;
            while z < 6000.0 {
                let t = sample(&s, x, z);
                assert!(t.elevation_m.is_finite());
                lo = lo.min(t.elevation_m);
                hi = hi.max(t.elevation_m);
                if t.is_land {
                    land += 1;
                }
                total += 1;
                z += 211.0;
            }
            x += 211.0;
        }
        assert!(hi - lo > s.config.relief_m * 0.5, "relief was only {}", hi - lo);
        let frac_land = land as f64 / total as f64;
        assert!((0.2..0.95).contains(&frac_land), "land fraction was {frac_land}");
    }

    #[test]
    fn mountains_only_appear_where_plates_converge() {
        // The coupling test (WG-Q5). Sample the world, split columns into
        // "near a convergent boundary" and "not near any boundary", and check
        // that the mountain term is substantially larger in the first group.
        let s = spec();
        let mut on_convergent = Vec::new();
        let mut deep_interior = Vec::new();
        let mut x = -24000.0;
        while x < 24000.0 {
            let mut z = -24000.0;
            while z < 24000.0 {
                let t = sample(&s, x, z);
                let plate = tectonics::sample(&s, x, z);
                if plate.boundary_strength > 0.9
                    && plate.boundary_kind == BoundaryKind::Convergent
                {
                    on_convergent.push(t.mountain_m);
                }
                if plate.boundary_strength < 0.02 {
                    deep_interior.push(t.mountain_m);
                }
                z += 311.0;
            }
            x += 311.0;
        }
        assert!(on_convergent.len() > 20, "too few convergent samples to judge");
        assert!(deep_interior.len() > 20, "too few interior samples to judge");
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let mc = mean(&on_convergent);
        let mi = mean(&deep_interior);
        assert!(mc > 5.0, "convergent columns have no mountains: mean {mc}");
        // The interior tail is **nonzero by construction**, and that is stated
        // rather than hidden: the falloff is exponential, and an exponential
        // does not reach zero at any finite distance. What is checked is that it
        // is negligible — under 1% of the belt's own mean, and under half a metre
        // of a 40 m relief. Asserting an exact zero here would be asserting a
        // fiction, and it is the fiction an earlier revision asserted.
        assert!(
            mi < 0.2,
            "columns a belt-width from any seam have a mountain term of {mi}"
        );
        assert!(
            mi < mc * 0.01,
            "mountains track tectonics only weakly: interior {mi} vs seam {mc}"
        );
    }

    #[test]
    fn slopes_and_normals_are_well_formed() {
        let s = spec();
        let mut x = -500.0;
        while x < 500.0 {
            let t = sample(&s, x, 12.0);
            assert!(t.slope >= 0.0 && t.slope.is_finite());
            let l = t.normal.length();
            assert!((l - 1.0).abs() < 1e-9, "normal length was {l}");
            assert!(t.normal.y > 0.0, "a surface normal pointed downward");
            x += 37.0;
        }
    }

    #[test]
    fn a_voxel_below_the_surface_is_solid_and_one_above_is_not() {
        let s = spec();
        let (x, z) = (120.0, -40.0);
        let surface = elevation_m(&s, x + 0.5, z + 0.5);
        let below = (surface.floor() as i32) - 2;
        let above = (surface.ceil() as i32) + 2;
        assert!(is_solid(&s, x as i32, below, z as i32));
        assert!(!is_solid(&s, x as i32, above, z as i32));
    }

    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_relief_and_where_the_mountains_are() {
        // What the world actually is, in numbers, before anything is claimed
        // about "mountains": the relief, the land fraction, the tallest mountain
        // term, and how far the nearest one is from the shipped spawn.
        //
        // **Seed 2468, named explicitly.** The first version of this measurement
        // used the tests' own `spec()` — seed **31337** — and its numbers were
        // read as though they described the shipped world. They did not, and the
        // client's own column probe caught it: 36 m of "land" here, water at
        // y 15 there. A measurement of the wrong world is worse than no
        // measurement, because it looks like one. Recorded as C-46.
        let s = WorldSpec::new(2468, Config::default());
        println!("seed {} — the shipped world", s.seed);
        let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut peak, mut peak_at) = (0.0f64, (0.0f64, 0.0f64));
        let (mut land, mut total) = (0u32, 0u32);
        let mut nearest: Option<(f64, f64, f64, f64)> = None;
        let mut x = -8000.0;
        while x <= 8000.0 {
            let mut z = -8000.0;
            while z <= 8000.0 {
                let t = sample(&s, x, z);
                min = min.min(t.elevation_m);
                max = max.max(t.elevation_m);
                if t.elevation_m > s.config.sea_level_m {
                    land += 1;
                }
                total += 1;
                if t.mountain_m > peak {
                    peak = t.mountain_m;
                    peak_at = (x, z);
                }
                if t.mountain_m > 8.0 {
                    let d = (x * x + z * z).sqrt();
                    if nearest.is_none_or(|n| d < n.0) {
                        nearest = Some((d, x, z, t.elevation_m));
                    }
                }
                z += 250.0;
            }
            x += 250.0;
        }
        println!(
            "relief over {} columns: min {min:.1} m, max {max:.1} m, land {:.1}%",
            total,
            100.0 * land as f64 / total as f64
        );
        println!(
            "peak mountain term {:.1} m at ({:.0}, {:.0})",
            peak, peak_at.0, peak_at.1
        );
        match nearest {
            Some((d, x, z, e)) => println!(
                "nearest mountain term above 8 m: {d:.0} m from the origin, at ({x:.0}, {z:.0}), elevation {e:.1} m"
            ),
            None => println!("no mountain term above 8 m within 8 km of the origin"),
        }
        let spawn = elevation_m(&s, 0.5, -95.5);
        println!(
            "spawn column (0.5, -95.5): elevation {spawn:.2} m -> derived eye y {:.2}",
            spawn + 3.0
        );
        println!("the spawn line, x = 0, with the world's own topmost non-empty voxel:");
        let mut z = -256.0;
        while z <= 256.0 {
            let t = sample(&s, 0.0, z);
            let zi = z as i32;
            let mut top = None;
            for y in (0..64).rev() {
                let m = crate::chunk_fields::material_at(&s, 0, y, zi).unwrap();
                if !m.is_empty() {
                    top = Some((y, m));
                    break;
                }
            }
            println!(
                "  z {z:>5.0}: elevation {:>6.1} m, mountain {:>5.1} m, topmost solid {top:?}, {}",
                t.elevation_m,
                t.mountain_m,
                if t.is_land { "land" } else { "sea" }
            );
            z += 16.0;
        }
    }

    #[test]
    fn sea_floor_never_exceeds_sea_level() {
        let s = spec();
        let mut x = -2000.0;
        while x < 2000.0 {
            assert!(sea_floor_m(&s, x, 0.0) <= s.config.sea_level_m);
            x += 113.0;
        }
    }
}
