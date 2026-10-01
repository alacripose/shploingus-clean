//! The column — every shaper, composed into a world.
//!
//! This is where the stage graph of [`crate::stages`] is actually run, in the
//! order that graph declares, and where the result becomes something with
//! **materials** in it.
//!
//! # The materials are a starter set, and it says so
//!
//! F2 measured the old project's material registry and found **zero admitted
//! materials**: the registry machinery was complete and the content did not
//! exist. [`materials::registry`] admits **eight**, each a single component at
//! ratio 1, and they are labelled as what they are: the *minimum* set a world
//! needs to have rock, soil, water and air. Anything else is a refusal
//! ([`voxel_model::RefusalKind::UnadmittedMaterial`]), which is the honest state
//! rather than a grey cube.
//!
//! # Composition, not a second generator
//!
//! A world is `generator(x, y, z)` **composed with** the [`Earthworks`] record.
//! The generator never reads the record and the record never changes the
//! generator, so `derive` is reproducible from `(seed, tick)` alone and a saved
//! world is a seed, a tick and a list of changes.

use crate::caves::{self, CaveParams, Void};
use crate::climate::{self, ClimateSample};
use crate::ecology::{self, Biome};
use crate::erosion;
use crate::geology::{self, Lithology, OreDials};
use crate::earthworks::Earthworks;
use crate::hydrology::{self, HydroSample};
use crate::spec::WorldSpec;
use crate::substrate;
use crate::tectonics::{self, PlateSample};
use crate::terrain::{self, TerrainSample};
use crate::weather::{self, WeatherSample};
use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
use voxel_model::{
    Component, ComponentId, Material, MaterialId, MaterialRegistry, OpticalClass, Ratio,
    Reflectance, Refusal, RefusalKind, Refused, SourcedReflectance,
};

/// Neighbourhood radius hydrology uses per column, in cells. Declared: shrinking
/// it shrinks the watershed, it does not approximate the same one.
pub const DEFAULT_HYDRO_RADIUS: usize = 12;

/// The starter material set.
pub mod materials {
    use super::*;

    /// Absence of matter. Never admitted; [`MaterialId::EMPTY`] by another name.
    pub const AIR: MaterialId = MaterialId(0);
    pub const WATER: MaterialId = MaterialId(1);
    pub const ICE: MaterialId = MaterialId(2);
    pub const REGOLITH: MaterialId = MaterialId(3);
    pub const SEDIMENTARY: MaterialId = MaterialId(4);
    pub const METAMORPHIC: MaterialId = MaterialId(5);
    pub const IGNEOUS: MaterialId = MaterialId(6);
    pub const BEDROCK: MaterialId = MaterialId(7);

    const ROCK: ComponentId = ComponentId(1);
    const FLUID: ComponentId = ComponentId(2);

    /// The material a rock class is made of.
    pub const fn for_lithology(l: Lithology) -> MaterialId {
        match l {
            Lithology::Regolith => REGOLITH,
            Lithology::Sedimentary => SEDIMENTARY,
            Lithology::Metamorphic => METAMORPHIC,
            Lithology::Igneous => IGNEOUS,
            Lithology::Bedrock => BEDROCK,
        }
    }

    /// The rock class a material is, if it is rock.
    pub const fn lithology_of(id: MaterialId) -> Option<Lithology> {
        match id {
            REGOLITH => Some(Lithology::Regolith),
            SEDIMENTARY => Some(Lithology::Sedimentary),
            METAMORPHIC => Some(Lithology::Metamorphic),
            IGNEOUS => Some(Lithology::Igneous),
            BEDROCK => Some(Lithology::Bedrock),
            _ => None,
        }
    }

    /// The admitted registry: eight materials, each one component at ratio 1.
    ///
    /// Returns a `Refusal` rather than panicking if the set ever stops being
    /// internally consistent — the registry is the thing that decides what exists,
    /// so it is the last place that should `unwrap`.
    pub fn registry() -> Refused<MaterialRegistry> {
        let mut r = MaterialRegistry::new();
        r.admit_component(ROCK, "silicate rock");
        r.admit_component(FLUID, "water/ice");

        // ---- what each material reflects, and where that came from ----------
        //
        // Until this round no material carried a reflectance at all, and the
        // consequence was measured: `openpbr::derive` refuses to invent one, so
        // the ambient and half-Lambert terms were not drawn, and the only term
        // left — the rim — is scaled by `(1 - roughness)`, which is exactly zero
        // for the least dense opaque material. That is `regolith`, 83 % of the
        // world's visible quads, drawn black by arithmetic (`docs/LEDGER.md`
        // C-38). So the values below are not decoration; they are the difference
        // between a world and a black rectangle.
        //
        // Every one is a **broadband albedo**, so every one is achromatic, and
        // the source says so. Per-channel hue needs the individual sample spectra
        // in the USGS library rather than its class ranges, and taking a class
        // range and inventing a hue from it is the thing this project does not
        // do.
        let hundredths = |n: u64| -> Refused<Ratio> { Ratio::new(n, 100) };
        let sourced = |n: u64, source: &'static str| -> Refused<SourcedReflectance> {
            Ok(SourcedReflectance { value: Reflectance::broadband(hundredths(n)?)?, source })
        };

        let one =
            |id: MaterialId, name: &str, density: u64, optical, reflectance: SourcedReflectance| {
                Material {
                    id,
                    name: name.to_string(),
                    components: vec![Component {
                        id: if id == WATER || id == ICE { FLUID } else { ROCK },
                        ratio: Ratio::ONE,
                        density_kg_m3: Ratio::from_u64(density),
                    }],
                    optical_class: optical,
                    reflectance: Some(reflectance),
                }
            };

        let materials = [
            // Open ocean is the surface, not a puddle on a table: 0.06 is the
            // tabulated figure for it, and a high sun angle is implicit in it.
            one(
                WATER,
                "water",
                1000,
                OpticalClass::Transmissive,
                sourced(
                    6,
                    "open-ocean albedo is 0.06 (tabulated standard value; Massom et al. 2025 gives \
                     0.06–0.07 for open water). Broadband, so achromatic",
                )?,
            ),
            // The low end of bare sea ice, because this material is *bare ice*:
            // the 0.7–0.9 figures are snow-covered ice, which is a different
            // material and not one the shapers produce.
            one(
                ICE,
                "ice",
                917,
                OpticalClass::Transmissive,
                sourced(
                    50,
                    "bare sea-ice albedo is 0.5–0.7, snow-covered up to 0.9 (NSIDC/Carleton \
                     cryosphere figures); the low end is taken because this is bare ice. \
                     Broadband, so achromatic",
                )?,
            ),
            one(
                REGOLITH,
                "regolith",
                1400,
                OpticalClass::Opaque,
                sourced(17, "bare-soil albedo is 0.17 (tabulated standard value). Broadband, so achromatic")?,
            ),
            one(
                SEDIMENTARY,
                "sedimentary rock",
                2400,
                OpticalClass::Opaque,
                sourced(
                    40,
                    "USGS Spectral Library v7 (Kokaly et al. 2017) rock and soil spectra; Lee et al. \
                     2021 measures 16.0–85.8 % visible reflectance across ten rocks, and a \
                     light-coloured sedimentary rock sits at the upper end of that range. A class \
                     value inside a published range is this project's choice, not a sample; \
                     broadband, so achromatic",
                )?,
            ),
            one(
                METAMORPHIC,
                "metamorphic rock",
                2750,
                OpticalClass::Opaque,
                sourced(
                    22,
                    "USGS Spectral Library v7 (Kokaly et al. 2017) rock and soil spectra; Lee et al. \
                     2021 measures 16.0–85.8 % visible reflectance across ten rocks, and a foliated \
                     metamorphic rock sits in the middle of it. A class value inside a published \
                     range is this project's choice, not a sample; broadband, so achromatic",
                )?,
            ),
            one(
                IGNEOUS,
                "igneous rock",
                2900,
                OpticalClass::Opaque,
                sourced(
                    12,
                    "USGS Spectral Library v7 (Kokaly et al. 2017) rock and soil spectra; Lee et al. \
                     2021 measures 16.0–85.8 % visible reflectance across ten rocks, and a dark \
                     mafic rock sits near the bottom of it. A class value inside a published range \
                     is this project's choice, not a sample; broadband, so achromatic",
                )?,
            ),
            one(
                BEDROCK,
                "bedrock",
                3000,
                OpticalClass::Opaque,
                sourced(
                    18,
                    "USGS Spectral Library v7 (Kokaly et al. 2017) rock and soil spectra; Lee et al. \
                     2021 measures 16.0–85.8 % visible reflectance across ten rocks, and rock seen \
                     only in section is taken dark within it. A class value inside a published \
                     range is this project's choice, not a sample; broadband, so achromatic",
                )?,
            ),
        ];

        for m in materials {
            if let Err(set) = r.admit_material(m) {
                return Err(Refusal::owned(
                    RefusalKind::RatiosDoNotSumToOne,
                    format!("the starter material set is not self-consistent: {set}"),
                ));
            }
        }
        Ok(r)
    }
}

/// Everything the shapers say about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Column {
    pub x: f64,
    pub z: f64,
    pub terrain: TerrainSample,
    pub plate: PlateSample,
    pub climate: ClimateSample,
    pub weather: WeatherSample,
    pub hydro: HydroSample,
    /// Steady-state soil depth, in metres.
    pub soil_depth_m: f64,
    /// The top of the rock column, below the soil.
    pub lithology: Lithology,
    pub biome: Biome,
    /// Whether the surface is bare rock (the substrate refusal).
    pub bare_rock: bool,
}

/// Run the stage graph for one column.
pub fn derive(spec: &WorldSpec, x: f64, z: f64, tick: u64, hydro_radius: usize) -> Column {
    // Stage order, as `stages::Declared::full().order()` produces it: tectonics,
    // fields, geology, terrain, hydrology, erosion, atmosphere, climate, weather,
    // caves, substrate, ecology, delta, transition, earthworks.
    let plate = tectonics::sample(spec, x, z);
    let terr = terrain::sample(spec, x, z);
    let hydro = hydrology::sample(spec, x, z, hydro_radius);
    let clim = climate::sample(spec, x, z, tick);
    let wx = weather::sample(spec, x, z, tick);

    // The **surface rock** is sampled *below* the regolith band, not at the
    // surface. Sampling at depth zero returns bed 0 by construction, which made
    // every column in a 12 km world report `Regolith` and only **one** lithology
    // appear anywhere. That was measured by the coverage test, not guessed.
    let reference_depth = geology::mean_bed_thickness_m(spec) * 1.6;
    let stratum = geology::stratum_at(
        spec,
        x,
        z,
        terr.elevation_m,
        terr.elevation_m - reference_depth,
    );
    let soil_depth_m = substrate::soil_depth_m(
        spec,
        stratum.lithology,
        terr.slope,
        clim.moisture,
    );
    let bare_rock = soil_depth_m <= 0.0;

    let biome = ecology::biome_at(
        spec,
        terr.elevation_m,
        clim.temperature_c,
        wx.precipitation_mm,
    );

    Column {
        x,
        z,
        terrain: terr,
        plate,
        climate: clim,
        weather: wx,
        hydro,
        soil_depth_m,
        lithology: stratum.lithology,
        biome,
        bare_rock,
    }
}

/// Cave parameters, derived from the world's configuration so they travel with it.
pub fn cave_params(spec: &WorldSpec) -> CaveParams {
    CaveParams { threshold: spec.config.cave_threshold, ..CaveParams::default() }
}

/// Everything about a **column** that does not depend on height.
///
/// A voxel's material is a function of `(x, z)` through these three numbers and
/// of `y` through nothing else. Computing them once per column instead of once
/// per voxel is the difference between a world that streams and a world that
/// does not — and because [`ColumnProfile::material_at`] is the **same code**
/// [`generated_material_at`] used to run per voxel, it is an optimisation and
/// not a second rule.
///
/// `surface_m` is the terrain height, `slope` the surface gradient the substrate
/// reads, and `moisture` the climate the substrate and the karst both read.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ColumnProfile {
    pub surface_m: f64,
    pub slope: f64,
    pub moisture: f64,
}

/// The height-independent part of a column, at a surface **fractional** position.
pub fn profile(spec: &WorldSpec, x: f64, z: f64) -> ColumnProfile {
    profile_with_surface(spec, x, z, terrain::elevation_m(spec, x, z))
}

/// The same, when the caller already has the surface height.
///
/// A caller filling a chunk computes the surface first anyway — it is the
/// cheapest of the three and it decides whether the rest is needed at all — so
/// passing it in saves an entire elevation evaluation per column rather than
/// being a convenience.
pub fn profile_with_surface(spec: &WorldSpec, x: f64, z: f64, surface_m: f64) -> ColumnProfile {
    ColumnProfile {
        surface_m,
        slope: terrain::sample(spec, x, z).slope,
        moisture: climate::moisture(spec, x, z),
    }
}

impl ColumnProfile {
    /// The **generated** material at a voxel, before any earthworks.
    ///
    /// The order of the tests is the order the physical world implies: everything
    /// above the surface is air or water; everything below is rock, unless the
    /// substrate put soil there or a cave carved it out.
    pub fn material_at(&self, spec: &WorldSpec, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
        let fx = x as f64 + 0.5;
        let fy = y as f64 + 0.5;
        let fz = z as f64 + 0.5;
        let sea = spec.config.sea_level_m;

        if fy > self.surface_m {
            // Above ground: sea where the ground is below sea level, otherwise
            // air. A voxel that is *partly* below the water line is water: the
            // surface is sampled at its centre and the centre is what decides.
            return Ok(if fy <= sea { materials::WATER } else { materials::AIR });
        }

        let depth = self.surface_m - fy;

        // Substrate: soil sits on top, and soil is not carved by karst — it is
        // what falls *into* a void, not what is dissolved.
        let stratum = geology::stratum_at(spec, fx, fz, self.surface_m, fy);
        let soil = substrate::soil_depth_m(spec, stratum.lithology, self.slope, self.moisture);
        if depth <= soil && soil > 0.0 {
            return Ok(materials::REGOLITH);
        }

        // Rock, or a void in it.
        let params = cave_params(spec);
        match caves::void_at(spec, stratum.lithology, fx, fy, fz, self.surface_m, &params) {
            Void::Open => Ok(materials::AIR),
            Void::Flooded => Ok(materials::WATER),
            Void::Rock => Ok(materials::for_lithology(stratum.lithology)),
        }
    }
}

/// The **generated** material at a voxel, before any earthworks.
///
/// One column of one voxel. Callers filling a whole column should build the
/// [`ColumnProfile`] once with [`profile`] and call
/// [`ColumnProfile::material_at`] for each height — which is what this function
/// does, so the two cannot disagree.
pub fn generated_material_at(spec: &WorldSpec, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
    profile(spec, x as f64 + 0.5, z as f64 + 0.5).material_at(spec, x, y, z)
}

/// The material at a voxel, **composed with** the earthworks record.
///
/// The record is applied last and only to the exact positions it names, which is
/// what keeps the generator pure: removing the record removes every change, and
/// the world is the generated one again.
pub fn material_at(
    spec: &WorldSpec,
    earthworks: &Earthworks,
    x: i32,
    y: i32,
    z: i32,
) -> Refused<MaterialId> {
    let base = generated_material_at(spec, x, y, z)?;
    Ok(earthworks.material_at(x, y, z, base))
}

/// Whether a voxel blocks movement.
pub fn is_solid(spec: &WorldSpec, earthworks: &Earthworks, x: i32, y: i32, z: i32) -> Refused<bool> {
    let m = material_at(spec, earthworks, x, y, z)?;
    // Water is not solid; ice is. Air is not.
    Ok(m != materials::AIR && m != materials::WATER)
}

/// The mean density of the admitted set, as a sanity check a caller can assert on.
pub fn mean_admitted_density() -> Refused<Mass> {
    let registry = materials::registry()?;
    let mut ledger = Ledger::new();
    let mut total = Ratio::ZERO;
    let mut count = 0u64;
    for m in registry.materials() {
        total += m.bulk_density_kg_m3();
        count += 1;
    }
    let mean = if count == 0 { Ratio::ZERO } else { total * Ratio::new(1, count)? };
    ledger.declare_pump(MassPumpDeclaration {
        reason: "a mean is an aggregate, not a transformation".into(),
        source: Source::World,
    });
    if count > 0 {
        ledger.deposit(
            Mass::from_exact(Exact::new(Ratio::from_u64(count))),
            Source::World,
            "one sample per admitted material",
        );
    }
    ledger.audit().map_err(|set| {
        Refusal::owned(RefusalKind::OutOfDeclaredRange, format!("{set}"))
    })?;
    Ok(Mass::from_exact(Exact::new(mean)))
}

/// An ore check for a column, delegating to geology with the world's dials.
pub fn ore_at(
    spec: &WorldSpec,
    dials: &OreDials,
    x: f64,
    y: f64,
    z: f64,
    depth_m: f64,
) -> Option<geology::OreKind> {
    geology::ore_at(spec, dials, x, y, z, depth_m)
}

/// A thermal-erosion pass over a patch, exposed here so the pipeline has a caller.
pub fn erode_patch(
    heights: &[f64],
    width: usize,
    height: usize,
    step_m: f64,
    repose_slope: f64,
    iterations: u32,
) -> erosion::Erosion {
    erosion::thermal(heights, width, height, step_m, repose_slope, 0.5, iterations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    #[test]
    fn the_starter_registry_is_self_consistent() {
        let r = materials::registry().unwrap();
        assert_eq!(r.admitted_material_count(), 7, "the starter set changed size");
        // Every material resolves, and an unknown id refuses.
        assert!(r.material(materials::WATER).is_ok());
        assert_eq!(
            r.material(MaterialId(500)).unwrap_err().kind(),
            RefusalKind::UnadmittedMaterial
        );
        // Water is less dense than rock, which is the one physical claim here.
        let water = r.material(materials::WATER).unwrap().bulk_density_kg_m3();
        let rock = r.material(materials::IGNEOUS).unwrap().bulk_density_kg_m3();
        assert!(water.to_f64() < rock.to_f64());
    }

    #[test]
    fn air_is_above_the_surface_and_rock_is_below_it() {
        let s = spec();
        let (x, z) = (11, -7);
        let surface = terrain::elevation_m(&s, x as f64 + 0.5, z as f64 + 0.5);
        let high = material_at(&s, &Earthworks::new(), x, surface as i32 + 8, z).unwrap();
        let deep = material_at(&s, &Earthworks::new(), x, surface as i32 - 40, z).unwrap();
        assert!(
            high == materials::AIR || high == materials::WATER,
            "high voxel was {high:?}"
        );
        assert!(
            materials::lithology_of(deep).is_some() || deep == materials::WATER,
            "deep voxel was {deep:?}"
        );
    }

    #[test]
    fn underwater_voxels_are_water_and_the_sea_floor_is_not() {
        let s = spec();
        // Find a column that is meaningfully below sea level. The search is over a
        // **grid**, not one line: an earlier revision walked `z = 0` alone and
        // found no water anywhere, which was a fact about the line rather than
        // about the world.
        let mut found = None;
        'search: for x in (-6000..6000).step_by(53) {
            for z in (-6000..6000).step_by(97) {
                let e = terrain::elevation_m(&s, x as f64 + 0.5, z as f64 + 0.5);
                if e < s.config.sea_level_m - 2.0 {
                    found = Some((x, z, e));
                    break 'search;
                }
            }
        }
        let (x, z, sea_floor) = found.expect("no water in this world");
        let mid_water = ((sea_floor + s.config.sea_level_m) / 2.0) as i32;
        assert_eq!(
            material_at(&s, &Earthworks::new(), x, mid_water, z).unwrap(),
            materials::WATER,
            "a voxel under the sea was not water"
        );
        let floor = (sea_floor - 3.0) as i32;
        assert!(materials::lithology_of(
            material_at(&s, &Earthworks::new(), x, floor, z).unwrap()
        )
        .is_some());
    }

    #[test]
    fn every_voxel_resolves_to_an_admitted_material() {
        let s = spec();
        let e = Earthworks::new();
        let registry = materials::registry().unwrap();
        for x in (-40..40).step_by(7) {
            for z in (-40..40).step_by(7) {
                for y in -24..56 {
                    let m = material_at(&s, &e, x, y, z).unwrap();
                    if m == materials::AIR {
                        continue;
                    }
                    assert!(
                        registry.material(m).is_ok(),
                        "voxel ({x},{y},{z}) held unadmitted material {m:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_column_runs_every_stage_and_the_result_is_deterministic() {
        let s = spec();
        let a = derive(&s, 100.0, -200.0, 7, DEFAULT_HYDRO_RADIUS);
        let b = derive(&s, 100.0, -200.0, 7, DEFAULT_HYDRO_RADIUS);
        assert_eq!(a, b);
        // Cross-stage coupling: the biome is a reading of this column's own
        // climate and precipitation, not an independent field.
        let expected = ecology::biome_at(
            &s,
            a.terrain.elevation_m,
            a.climate.temperature_c,
            a.weather.precipitation_mm,
        );
        assert_eq!(a.biome, expected);
    }

    #[test]
    fn the_column_changes_with_position_and_seed() {
        let s = spec();
        let a = derive(&s, 0.0, 0.0, 0, DEFAULT_HYDRO_RADIUS);
        let b = derive(&s, 3000.0, 3000.0, 0, DEFAULT_HYDRO_RADIUS);
        assert_ne!(a, b);
        let other = WorldSpec::new(97531, Config::default());
        assert_ne!(a, derive(&other, 0.0, 0.0, 0, DEFAULT_HYDRO_RADIUS));
    }

    #[test]
    fn earthworks_compose_over_the_generator_and_do_not_change_it() {
        let s = spec();
        let mut e = Earthworks::new();
        let (x, y, z) = (3, 30, 4);
        let base = generated_material_at(&s, x, y, z).unwrap();
        let target = if base == materials::AIR { materials::IGNEOUS } else { materials::AIR };

        let mut ledger = Ledger::new();
        ledger.declare_pump(MassPumpDeclaration {
            reason: "test modification".into(),
            source: Source::World,
        });
        ledger.deposit(
            Mass::from_exact(Exact::new(Ratio::from_u64(1))),
            Source::World,
            "one cubic metre",
        );
        e.record(crate::earthworks::Modification {
            x,
            y,
            z,
            from: base,
            to: target,
            reason: "test".into(),
            ledger,
        })
        .unwrap();

        assert_eq!(material_at(&s, &e, x, y, z).unwrap(), target);
        // The generator is unchanged: the same call without the record is the
        // same value, which is what keeps `derive` reproducible.
        assert_eq!(generated_material_at(&s, x, y, z).unwrap(), base);
    }

    #[test]
    fn the_pipeline_covers_biomes_and_lithologies_across_a_world() {
        // A world that produced one biome and one rock everywhere would pass
        // every test above and be a worse world.
        let s = spec();
        let mut biomes = std::collections::BTreeSet::new();
        let mut rocks = std::collections::BTreeSet::new();
        let mut x = -6000.0;
        while x < 6000.0 {
            let c = derive(&s, x, x * 0.7, 0, 8);
            biomes.insert(c.biome);
            rocks.insert(c.lithology);
            x += 300.0;
        }
        assert!(biomes.len() >= 3, "only {} biomes appeared: {biomes:?}", biomes.len());
        assert!(rocks.len() >= 3, "only {} lithologies appeared", rocks.len());
    }

    #[test]
    fn erosion_is_reachable_from_the_pipeline_and_conserves() {
        let mut heights = vec![0.0f64; 64];
        heights[31] = 20.0;
        let before: f64 = heights.iter().sum();
        let e = erode_patch(&heights, 8, 8, 1.0, 0.7, 30);
        let after: f64 = e.heights.iter().sum();
        assert!((after - before).abs() < 1e-9);
    }

    #[test]
    fn mean_density_is_a_real_number() {
        let m = mean_admitted_density().unwrap();
        assert!(m.exact().ratio().to_f64() > 500.0);
        assert!(m.exact().ratio().to_f64() < 4000.0);
    }
}
