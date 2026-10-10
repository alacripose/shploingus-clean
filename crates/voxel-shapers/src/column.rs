//! The column — every shaper, composed into a world.
//!
//! This is where the stage graph of [`crate::stages`] is actually run, in the
//! order that graph declares, and where the result becomes something with
//! **materials** in it.
//!
//! # The materials are a placeholder, and the ledger says so
//!
//! F2 measured the old project's material registry and found **zero admitted
//! materials**: the registry machinery was complete and the content did not
//! exist. [`materials::registry`] admits **thirteen**, each a single component at
//! ratio 1, and they keep the current world and its tests running — but they are
//! **not authority**. Five of the thirteen (C-159.10) are the **surface covers**:
//! the material a biome's ground is made of, so the visible world varies with the
//! region instead of being one pale playa. `LEDGER.md` C-122 records the ruling: they are an AI
//! fabrication, and the target is a **derived** material library (real-world
//! materials with base patterns at the 4×4×4 slots and the addressed 256 level).
//! A shape's material roles must not be chosen from these materials; the roster's
//! inset/seam/calyx slots are destinations for the derived library's patterns.
//! Anything else is a refusal ([`voxel_model::RefusalKind::UnadmittedMaterial`]),
//! which is the honest state rather than a grey cube.
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
use crate::habitat::{self, Habitat};
use crate::earthworks::Earthworks;
use crate::hydrology::{self, HydroSample};
use crate::spec::WorldSpec;
use crate::substrate;
use crate::tectonics::{self, PlateSample};
use crate::terrain::{self, TerrainSample};
use crate::weather::{self, WeatherSample};
use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
use voxel_model::materials::spectra;
use voxel_model::{
    Component, ComponentId, DetailIndex, Material, MaterialId, MaterialRegistry, OpticalClass,
    Ratio, Reflectance, Refusal, RefusalKind, Refused, ShadingField, SourcedReflectance,
};

/// Neighbourhood radius hydrology uses per column, in cells. Declared: shrinking
/// it shrinks the watershed, it does not approximate the same one.
pub const DEFAULT_HYDRO_RADIUS: usize = 12;

/// The placeholder material set — **not authority**; see the module docs and
/// `LEDGER.md` C-122. The derived material library replaces its content.
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

    /// The surface covers (C-159.10). These are not rock classes: each is the
    /// **ground a formation is covered with**, chosen by [`surface_for_biome`] and
    /// carrying its own USGS sample, so walking from a grassland into a desert
    /// changes the colour under the eye.
    pub const GRASS: MaterialId = MaterialId(8);
    pub const DRY_GRASS: MaterialId = MaterialId(9);
    pub const LEAF: MaterialId = MaterialId(10);
    pub const CONIFER: MaterialId = MaterialId(11);
    pub const SAND: MaterialId = MaterialId(12);
    /// The trunk of a tree (C-159.11). Not chosen by any biome: it is what
    /// [`crate::flora`] stands, wherever a tree exists.
    pub const WOOD: MaterialId = MaterialId(13);

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

    /// **The one rule mapping a formation to its ground cover** (C-159.10).
    ///
    /// Both material paths read this — the column profile through its cached biome and
    /// the field plane through its `Climate` law — so "what is this biome's surface"
    /// has exactly one spelling. Alpine and polar desert are the exception that needs
    /// no new material: bare scree and gravel are what `REGOLITH` already is.
    pub const fn surface_for_biome(biome: Biome) -> MaterialId {
        match biome {
            Biome::Desert => SAND,
            Biome::Grassland => GRASS,
            Biome::Tundra | Biome::SubtropicalScrub => DRY_GRASS,
            Biome::BorealForest => CONIFER,
            Biome::CoolTemperateForest
            | Biome::TemperateForest
            | Biome::TropicalDryForest
            | Biome::TropicalRainforest => LEAF,
            Biome::Alpine | Biome::PolarDesert => REGOLITH,
        }
    }

    /// What the column's **terrain expression** does to a cover (C-160) — the sub-biome
    /// layer's material consequence.
    ///
    /// `None` is `Steep`: bare ground is not a cover at all, it is the **lithology's own
    /// rock**, and the caller resolves it because it holds the column's stratum (the
    /// column path) or reads the `Geology` field (the plane path). Everything else maps
    /// a cover to its drier, wetter or colder expression; an unknown cover passes
    /// through, so a future material is never silently restyled.
    pub const fn surface_under(cover: MaterialId, habitat: Habitat) -> Option<MaterialId> {
        match habitat {
            Habitat::Lowland => Some(cover),
            Habitat::Steep => None,
            Habitat::Wet => Some(match cover {
                DRY_GRASS => GRASS,
                other => other,
            }),
            Habitat::Dry => Some(match cover {
                GRASS | LEAF => DRY_GRASS,
                other => other,
            }),
            Habitat::Montane => Some(match cover {
                GRASS => DRY_GRASS,
                LEAF => CONIFER,
                SAND => REGOLITH,
                other => other,
            }),
        }
    }

    /// The formation's cover under its terrain expression — the composition of
    /// [`surface_for_biome`] and [`surface_under`], so no caller can apply the two rules
    /// in the other order and drift.
    pub const fn surface_of(biome: Biome, habitat: Habitat) -> Option<MaterialId> {
        surface_under(surface_for_biome(biome), habitat)
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

    /// The admitted registry: twelve materials, each one component at ratio 1.
    ///
    /// Returns a `Refusal` rather than panicking if the set ever stops being
    /// internally consistent — the registry is the thing that decides what exists,
    /// so it is the last place that should `unwrap`.
    pub fn registry() -> Refused<MaterialRegistry> {
        let mut r = MaterialRegistry::new();

        // ---- what each material reflects, and where that came from ----------
        //
        // Until this round no material carried a reflectance at all, and the
        // consequence was measured: `openpbr::derive` refuses to invent one, so
        // the ambient and half-Lambert terms were not drawn, and the only term
        // left in those frames — the rim — carried no reflectance of its own
        // either (it was scaled by `(1 - roughness)`, exactly zero for the least
        // dense opaque material, so `regolith` — 83 % of the world's visible quads
        // — was drawn black by arithmetic, `docs/LEDGER.md` C-38). The rim now
        // carries the block's own reflectance, and every other term does too, so
        // a material with no reflectance is drawn by *nothing* rather than by a
        // leftover: the values below are still the difference between a world and
        // a black rectangle.
        //
        // Six of the seven now carry a **sampled** reflectance: the whole USGS
        // Spectral Library v7 spectrum is retained in `voxel_model::materials::
        // spectra`, and the value here is the declared derivation — the mean of
        // the sample's own channels in each visible window, exact (C-140/C-141).
        // The seventh, ice, stays broadband: the only H2O-ice AREF in v7 has no
        // present channels across the visible windows at all, and a snow sample
        // is a different material (C-39), so there is nothing to derive from yet
        // (C-143).
        let hundredths = |n: u64| -> Refused<Ratio> { Ratio::new(n, 100) };
        let sourced = |n: u64, source: &'static str| -> Refused<SourcedReflectance> {
            Ok(SourcedReflectance { value: Reflectance::broadband(hundredths(n)?)?, source })
        };

        // **A row is one substance, and its component says so** (C-141). The three
        // generic components this used to build ("silicate rock", "water/ice",
        // "plant matter") were the fabricated scheme C-122 refuses as authority, and
        // they could not carry an appearance at all: two materials sharing a component
        // would have to share the one reflectance the component holds. So each row owns
        // a component named for itself and carrying its own measured reflectance, which
        // is also what makes the row's value and its composition's blend the same claim.
        let one =
            |id: MaterialId, name: &str, density: u64, optical, reflectance: SourcedReflectance| {
                Material {
                    id,
                    name: name.to_string(),
                    components: vec![Component {
                        id: ComponentId(id.0),
                        ratio: Ratio::ONE,
                        density_kg_m3: Ratio::from_u64(density),
                        reflectance: Some(reflectance),
                    }],
                    optical_class: optical,
                    reflectance: Some(reflectance),
                    // **None is the honest claim for all of these.** They reflect; none of
                    // them emits, and a body is what emits. This stays `None` per-material
                    // rather than defaulted at derive time so that "emits nothing" and
                    // "nobody has measured whether it emits" do not become one answer.
                    emission: None,
                }
            };

        let materials = [
            // Open ocean is the surface, not a puddle on a table; the value is
            // the open-ocean seawater spectrum's own visible-window mean
            // (record 13635), dark and blue-leaning by measurement.
            one(
                WATER,
                "water",
                1000,
                OpticalClass::Transmissive,
                spectra::seawater_open_ocean()?,
            ),
            // The low end of bare sea ice, because this material is *bare ice*:
            // the 0.7–0.9 figures are snow-covered ice, which is a different
            // material and not one the shapers produce. Still broadband: the
            // only H2O-ice AREF in splib07a v7 has no present channels across
            // the visible windows, so no sampled colour exists to take (C-143).
            one(
                ICE,
                "ice",
                917,
                OpticalClass::Transmissive,
                sourced(
                    50,
                    "bare sea-ice albedo is 0.5–0.7, snow-covered up to 0.9 (NSIDC/Carleton \
                     cryosphere figures); the low end is taken because this is bare ice. \
                     Broadband, so achromatic; sampled colour waits for an ice spectrum \
                     (the v7 AREF has no visible channels, C-143)",
                )?,
            ),
            one(
                REGOLITH,
                "regolith",
                1400,
                OpticalClass::Opaque,
                spectra::stonewall_playa_dry_mud()?,
            ),
            one(
                SEDIMENTARY,
                "sedimentary rock",
                2400,
                OpticalClass::Opaque,
                spectra::limestone_cu02()?,
            ),
            one(
                METAMORPHIC,
                "metamorphic rock",
                2750,
                OpticalClass::Opaque,
                spectra::actinolite_hornfels()?,
            ),
            one(
                IGNEOUS,
                "igneous rock",
                2900,
                OpticalClass::Opaque,
                spectra::basalt_fresh()?,
            ),
            one(
                BEDROCK,
                "bedrock",
                3000,
                OpticalClass::Opaque,
                spectra::magnetite_skarn()?,
            ),
            // ---- the surface covers (C-159.10) --------------------------------
            //
            // These five are what the **biome** decides: [`surface_for_biome`] maps
            // every formation to one of them, so a region's ground carries the
            // region's own sampled colour instead of the single REGOLITH stand-in.
            // Four are new Chapter V samples (`materials/spectra/sources.tsv`); SAND
            // is `sand_dwo3`, retained since C-141 as the alternate regolith candidate
            // and now the desert's own surface.
            one(SAND, "sand", 1600, OpticalClass::Opaque, spectra::sand_dwo3()?),
            one(
                GRASS,
                "grass",
                1250,
                OpticalClass::Opaque,
                spectra::grass_mixed_green()?,
            ),
            one(
                DRY_GRASS,
                "dry grass",
                1250,
                OpticalClass::Opaque,
                spectra::grass_golden_dry()?,
            ),
            one(
                LEAF,
                "leaf",
                900,
                OpticalClass::Opaque,
                spectra::aspen_green_top()?,
            ),
            one(
                CONIFER,
                "conifer needles",
                900,
                OpticalClass::Opaque,
                spectra::spruce_needles()?,
            ),
            // The trunk (C-159.11): a gray pine branch — bark and wood of a standing
            // conifer, from Chapter V, because a milled 2×4 (Chapter A's `Wood_Beam`)
            // is a processed material and not what a tree is made of.
            one(
                WOOD,
                "wood",
                650,
                OpticalClass::Opaque,
                spectra::gray_pine_branch()?,
            ),
        ];

        for m in materials {
            r.admit_component(ComponentId(m.id.0), m.name.clone());
            if let Err(set) = r.admit_material(m) {
                return Err(Refusal::owned(
                    RefusalKind::RatiosDoNotSumToOne,
                    format!("the starter material set is not self-consistent: {set}"),
                ));
            }
        }

        // ---- the corpus: one admitted material per declared substance (C-122) ----
        //
        // The registry is additive from here: the world's own ids 1..=13 above do not
        // move, and every declared substance the world does not draw yet becomes a row
        // with its own composition and its own sourced colour. `corpus.rs` owns that
        // table and the reason each row is shaped the way it is.
        crate::corpus::admit(&mut r)?;
        Ok(r)
    }

    // ---- the 256-level shading field (C-161) ----------------------------
    //
    // `ShadingField` is the material shaper's declaration at the 256 level: **how
    // much** of `voxel-appearance::detail::DetailLattice`'s own mean-zero deviation
    // each material shows. It is the "material slot's own pattern" C-122 describes —
    // carried by the material, read per fragment off the fragment's address, so a
    // greedy-merged rectangle shows its blocks with **no extra quads**.
    //
    // The gains belong to the **derived library** row by row, and that content is
    // owed (MATERIALS.md 4.9/4.10). Until it lands, this is the world's standing
    // declaration: every opaque material shows the same share of the lattice, and a
    // **transmissive** material shows none — a fluid boundary is a surface with no
    // composition pattern, and water's look is its Fresnel reflection (C-159.2).

    /// The domain tag of the world's shading field, folded into the lattice's seed
    /// (`ShadingField::domain`). A constant because the pattern is a property of the
    /// **material and the instance address** and not of the world's seed: a block of
    /// sand patterns the same way in every world, the way a texture does.
    pub const SHADING_FIELD_DOMAIN: u64 = 0x5A17_0000_0000_0001;

    /// How much of the lattice's deviation the standing declaration gives an opaque
    /// material, per channel.
    ///
    /// Each channel is in `[-1, 1)`, and a material shaper may declare a gain in
    /// `[0, 1]` — above 1 the deviation could push a parameter out of the range
    /// OpenPBR refuses rather than clamps (`ShadingField::declare` says so). One half
    /// is declared here because a deviation at the model's own full scale moves a
    /// value all the way to its domain edge — every cell reads as a *different*
    /// material — and zero is the flat block the C-160 report was about. The
    /// library's rows replace both numbers per material.
    pub const STANDING_ROUGHNESS_GAIN: f64 = 0.5;
    /// See [`STANDING_ROUGHNESS_GAIN`].
    pub const STANDING_REFLECTANCE_GAIN: f64 = 0.5;

    /// The world's 256-level shading field, declared for every material [`registry`]
    /// admits.
    pub fn shading_field() -> Refused<ShadingField> {
        let registry = registry()?;
        let mut field = ShadingField::none();
        field.set_domain(SHADING_FIELD_DOMAIN);
        for m in registry.materials() {
            let (roughness, reflectance) = if m.optical_class == OpticalClass::Opaque {
                (STANDING_ROUGHNESS_GAIN, STANDING_REFLECTANCE_GAIN)
            } else {
                (0.0, 0.0)
            };
            field.declare(m.id, roughness, reflectance)?;
        }
        Ok(field)
    }

    // ---- the surface shapers (C-162) ------------------------------------
    //
    // White noise at a millimetre is not yet a material's pattern: it is static.
    // The user's ruling (2026-10-05) is that the 256 level stays white noise and
    // each material *shapes* it: *"for sand it'd be speckled, for trees it'd be
    // bark, metal could be brushed/shiny"*. So a shaper is a **stencil** on the
    // lattice address (which axes a value repeats across) and a **curve** on the
    // value (what the noise is read through) — nothing is stored, and the lattice
    // keeps C-2's pure-function contract.
    //
    // The derived library still owns the *content* (`MATERIALS.md` 4.9/4.10): when
    // it lands, every row brings its own shaper and these ids become its ids. Until
    // then this is the mechanism, declared per material **role** rather than by the
    // placeholder ids' names.

    /// How a material's 256-level noise is shaped into its own pattern.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum SurfacePattern {
        /// Isotropic, sparse bright and dark specks — sand, dust, regolith.
        Speckle,
        /// Clumps twice the cell across on every axis — leaves, grass.
        Foliage,
        /// Grain stretched four cells **along z**, a tree's own axis — bark.
        Bark,
        /// Grain stretched **along x** — brushed stone now, the library's metals
        /// later (the user's own example: *"metal could be brushed/shiny"*).
        Brushed,
    }

    impl SurfacePattern {
        /// The id a material row carries to the shader (`pattern.z`), which
        /// dispatches on it; `shading.wgsl`'s transcription names the same numbers,
        /// and a test in `voxel-client` holds the two together.
        pub const fn index(self) -> u32 {
            match self {
                SurfacePattern::Speckle => 0,
                SurfacePattern::Foliage => 1,
                SurfacePattern::Bark => 2,
                SurfacePattern::Brushed => 3,
            }
        }

        /// The lattice address a shaped sample reads at: an axis **shifted right by
        /// `k`** reads its values in runs `2^k` cells long, which is the stretch. It
        /// is a shift and not a bit-mask because the ladder quantises the address
        /// again for every rung (C-163): a mask's cleared bits are re-cleared by the
        /// rung's own mask — they compose to the *max*, so bark's stretch vanished at
        /// every rung above the floor. A shift moves the axis into a coarser address
        /// space **before** the rung quantises it, so the character — grain four times
        /// longer than it is thick — holds at every drawn scale (C-164).
        ///
        /// So the shaper's stretch is **relative to the cell the ladder draws**, not an
        /// absolute length: at the floor rung (31.25 mm) bark's grain runs ~125 mm along
        /// z; twice as far out it runs 250 mm, and it still reads as bark.
        ///
        /// The stencil is applied **before** the ladder quantises the address (the
        /// call order `pattern::Pattern::deviation` and `shading.wgsl` both state),
        /// which is what lets a rung coarser than the shift still see the stretch.
        pub const fn stencil(self, at: DetailIndex) -> DetailIndex {
            match self {
                SurfacePattern::Speckle => at,
                SurfacePattern::Foliage => {
                    DetailIndex { x: at.x >> 1, y: at.y >> 1, z: at.z >> 1 }
                }
                SurfacePattern::Bark => DetailIndex { x: at.x, y: at.y, z: at.z >> 2 },
                SurfacePattern::Brushed => DetailIndex { x: at.x >> 2, y: at.y, z: at.z },
            }
        }

        /// What the noise is read through. Speckle squares its value — sparse
        /// specks rather than uniform static, which is what makes it read as grain
        /// — and the directional shapers keep the value, because their character is
        /// the stretch and not the amplitude.
        pub fn curve(self, s: f64) -> f64 {
            match self {
                SurfacePattern::Speckle => s * s.abs(),
                SurfacePattern::Foliage | SurfacePattern::Bark | SurfacePattern::Brushed => s,
            }
        }
    }

    /// Which shape a material's 256-level pattern is drawn with.
    ///
    /// Total over the admitted set: every material answers, and the transmissive
    /// ones answer too even though the standing field gives them no gains — a row
    /// that is never varied still carries a complete declaration.
    pub const fn pattern_of(m: MaterialId) -> SurfacePattern {
        match m {
            SAND | REGOLITH => SurfacePattern::Speckle,
            GRASS | DRY_GRASS | LEAF | CONIFER => SurfacePattern::Foliage,
            WOOD => SurfacePattern::Bark,
            // The rock classes, plus ICE and WATER, which the gains keep flat. The
            // library's metals will land on `Brushed`; stone reads as brushed until
            // its own rows exist.
            _ => SurfacePattern::Brushed,
        }
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
    /// The **terrain expression inside the biome** (C-160) — the sub-biome layer, read
    /// from the same slope and moisture the material rule reads.
    pub habitat: Habitat,
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

    // **The biome's two numbers are aggregates, sampled from the ticked shapers** —
    // twelve monthly means (each clamped to Holdridge's 0..30 °C) and a mean annual
    // total. The instantaneous `clim`/`wx` above stay instantaneous for every consumer
    // that wants *this tick*; the biome wants the region's climate (ECO-0001, ruled
    // 2026-10-02). Before this, `clim.temperature_c` and `wx.precipitation_mm` were fed
    // straight in and four of four probed columns flickered between biomes twice a year.
    let (biotemperature_c, mean_annual_precipitation_mm) =
        ecology::holdridge_inputs(spec, x, z, terr.elevation_m, tick);
    let biome = ecology::biome_at(
        spec,
        terr.elevation_m,
        biotemperature_c,
        mean_annual_precipitation_mm,
    );
    // The sub-biome layer (C-160): the **terrain expression** inside the formation,
    // read from the slope and moisture this column already computed.
    let habitat = habitat::habitat_of(spec, terr.elevation_m, terr.slope, clim.moisture);

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
        habitat,
        bare_rock,
    }
}

/// Cave parameters, derived from the world's configuration so they travel with it.
pub fn cave_params(spec: &WorldSpec) -> CaveParams {
    CaveParams { threshold: spec.config.cave_threshold, ..CaveParams::default() }
}

/// Everything about a **column** that does not depend on height.
///
/// A voxel's material is a function of `(x, z)` through these numbers and
/// of `y` through nothing else. Computing them once per column instead of once
/// per voxel is the difference between a world that streams and a world that
/// does not — and because [`ColumnProfile::material_at`] is the **same code**
/// [`generated_material_at`] used to run per voxel, it is an optimisation and
/// not a second rule.
///
/// `surface_m` is the terrain height, `slope` the surface gradient the substrate
/// reads, `moisture` the climate the substrate and the karst both read, and
/// **`biome`** (C-159.10) the formation the region's climate supports — which is
/// what decides the surface material. It is read here, once per column, at
/// **tick 0**: the biome is the region's climate and not the moment's
/// (`LEDGER.md` ECO-0001), so the ground does not change colour with the season.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ColumnProfile {
    pub surface_m: f64,
    pub slope: f64,
    pub moisture: f64,
    pub biome: Biome,
    /// The **terrain expression inside the biome** (C-160) — the sub-biome layer,
    /// computed from the slope and moisture above so both material paths read one value.
    pub habitat: Habitat,
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
    // The biome is the **region's climate**, read through the same two aggregates
    // `derive` reads (ECO-0001) — and read at tick 0 because it does not move with the
    // tick. The field plane's `Climate` law computes exactly this from the same
    // elevation, so the two paths cannot disagree about a column's ground.
    let (biotemperature_c, mean_annual_precipitation_mm) =
        ecology::holdridge_inputs(spec, x, z, surface_m, 0);
    let slope = terrain::sample(spec, x, z).slope;
    let moisture = climate::moisture(spec, x, z);
    ColumnProfile {
        surface_m,
        slope,
        moisture,
        biome: ecology::biome_at(
            spec,
            surface_m,
            biotemperature_c,
            mean_annual_precipitation_mm,
        ),
        // The sub-biome layer (C-160) reads the two values above; the plane computes it
        // from the `Slope` and `Moisture` fields it already carries.
        habitat: habitat::habitat_of(spec, surface_m, slope, moisture),
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
            // Above ground: sea where the ground is below sea level, otherwise a tree
            // where one reaches this voxel (`crate::flora` — a pure function of position),
            // otherwise air. A voxel that is *partly* below the water line is water: the
            // surface is sampled at its centre and the centre is what decides.
            if fy <= sea {
                return Ok(materials::WATER);
            }
            if let Some(material) = crate::flora::tree_material_at(spec, x, y, z, self.surface_m)
            {
                return Ok(material);
            }
            return Ok(materials::AIR);
        }

        let depth = self.surface_m - fy;

        // Substrate: soil sits on top, and soil is not carved by karst — it is
        // what falls *into* a void, not what is dissolved.
        let stratum = geology::stratum_at(spec, fx, fz, self.surface_m, fy);
        let soil = substrate::soil_depth_m(spec, stratum.lithology, self.slope, self.moisture);
        // **The biome decides the cover and the terrain expression decides its
        // variant** (C-159.10, C-160). Before this the soil branch returned the biome's
        // cover wherever soil existed; a rock face inside a grassland read as grass.
        if depth <= soil && soil > 0.0 {
            if let Some(cover) = materials::surface_of(self.biome, self.habitat) {
                return Ok(cover);
            }
            // `None` is `Steep`: bare ground shows the lithology's own rock — the same
            // material the rock branch below returns, and the same one the field plane
            // reads from its `Geology` field. Since C-172 that rock is the **declared
            // substance this region's bed is made of** (`corpus::rock_voxel`), not the
            // generic class.
            return crate::corpus::rock_voxel(spec, stratum.lithology, fx, fy, fz, depth);
        }

        // Rock, or a void in it.
        let params = cave_params(spec);
        match caves::void_at(spec, stratum.lithology, fx, fy, fz, self.surface_m, &params) {
            Void::Open => Ok(materials::AIR),
            Void::Flooded => Ok(materials::WATER),
            // The region's declared rock, or its ore where a vein is present — the one
            // rule the field plane also calls (`corpus::rock_voxel`, C-172).
            Void::Rock => crate::corpus::rock_voxel(spec, stratum.lithology, fx, fy, fz, depth),
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

/// Whether a material is a **fluid** — world state, not a mineable block.
///
/// Water is the fluid this world places (`fy <= sea` above ground, and flooded
/// voids below it). Ice is **not** a fluid here even though the registry's
/// component is "water/ice": ice is solid, it blocks movement, and mining it is
/// not the filed defect — the user's words are about *water*, and "Wait for a real
/// ice spectrum" keeps its material row on hold (C-143.3, C-152.1).
///
/// This is **the one rule** the edit path, the mesh and the movement check share:
/// a second spelling of "is this water" is how the collider and the pick come to
/// disagree about where the sea is (`blocks_movement` is the same lesson).
pub fn is_fluid(m: MaterialId) -> bool {
    m == materials::WATER
}

/// Whether a material blocks movement. **The one rule about solidity.**
///
/// Exposed as its own function because a caller can hold a material *without*
/// holding a world: the client's collision asks the streamer's live record, which
/// already knows about the player's edits, and has to answer the same question
/// [`is_solid`] answers. Two copies of this rule is how a camera ends up walking
/// through a block the player just placed — the first copy asks the generator, the
/// second asks the record, and they disagree the moment anyone digs.
pub fn blocks_movement(m: MaterialId) -> bool {
    // Water is not solid; ice is. Air is not. A fluid is never something a body
    // stands on, which is why the fluid rule and the solidity rule are read from
    // the same predicate.
    m != materials::AIR && !is_fluid(m)
}

/// Whether a voxel blocks movement.
pub fn is_solid(spec: &WorldSpec, earthworks: &Earthworks, x: i32, y: i32, z: i32) -> Refused<bool> {
    Ok(blocks_movement(material_at(spec, earthworks, x, y, z)?))
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
        // **The size is the claim, and the claim grew on purpose** (C-122): thirteen rows
        // are the world's own, and the rest is one admitted material per declared
        // substance, so the count is asserted as the sum rather than as a constant that
        // would have to be edited whenever content lands — and would hide a row that
        // never arrived if it were not.
        assert_eq!(
            r.admitted_material_count(),
            13 + crate::corpus::ROWS.len(),
            "the admitted set changed size"
        );
        for material in r.materials() {
            assert!(
                material.reflectance.is_some(),
                "{} is admitted with no reflectance, so it would be drawn black by \
                 arithmetic (C-38)",
                material.name
            );
        }
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
    fn the_terrain_expression_moves_the_cover_and_steep_shows_rock() {
        // C-160's table, pinned: the habitat is not a label, it is a change to what the
        // ground is made of.
        assert_eq!(
            materials::surface_of(Biome::Grassland, Habitat::Lowland),
            Some(materials::GRASS)
        );
        assert_eq!(
            materials::surface_of(Biome::Grassland, Habitat::Dry),
            Some(materials::DRY_GRASS)
        );
        assert_eq!(
            materials::surface_of(Biome::Tundra, Habitat::Wet),
            Some(materials::GRASS)
        );
        assert_eq!(
            materials::surface_of(Biome::TemperateForest, Habitat::Montane),
            Some(materials::CONIFER)
        );
        assert_eq!(
            materials::surface_of(Biome::Desert, Habitat::Montane),
            Some(materials::REGOLITH)
        );
        // Steep is not a cover: the caller resolves it to the lithology.
        assert_eq!(materials::surface_of(Biome::Grassland, Habitat::Steep), None);
        // And a cover the table does not know passes through untouched.
        assert_eq!(
            materials::surface_under(materials::WATER, Habitat::Dry),
            Some(materials::WATER)
        );
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
        // Since C-172 a deep voxel is the **region's declared rock, or an ore where the
        // vein field puts one** — not the generic class — so the claim the rule still
        // supports is that it is solid ground: not air, not water, and something a body
        // stands on. Asking for a lithology here would fail on a coal seam the way it used
        // to fail on nothing.
        assert!(
            deep != materials::AIR && deep != materials::WATER && blocks_movement(deep),
            "deep voxel was {deep:?}"
        );
    }

    #[test]
    fn water_is_a_fluid_and_ice_is_a_block() {
        // **The one rule the edit refusal reads.** Water is world state; ice is a
        // mineable solid, and its material row waits for a real spectrum (C-143.3).
        assert!(is_fluid(materials::WATER));
        assert!(!is_fluid(materials::ICE), "ice blocks movement, so it is a block");
        assert!(!is_fluid(materials::AIR), "air is nothing, not a fluid");
        assert!(!is_fluid(materials::REGOLITH));
        // And solidity reads the same predicate, so the two cannot disagree:
        // a fluid is never something a body stands on.
        assert!(!blocks_movement(materials::WATER));
        assert!(blocks_movement(materials::ICE));
        assert!(blocks_movement(materials::IGNEOUS));
        assert!(!blocks_movement(materials::AIR));
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
        let floor_material = material_at(&s, &Earthworks::new(), x, floor, z).unwrap();
        // The sea floor is **ground, not a class**: since C-172 it is the region's declared
        // rock (or an ore), so the rule the test keeps is that it is not water and a body
        // stands on it.
        assert_ne!(floor_material, materials::WATER, "the sea floor was water");
        assert!(
            blocks_movement(floor_material),
            "the sea floor was {floor_material:?}, which a body does not stand on"
        );
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
        // Cross-stage coupling: the biome is a reading of this column's own climate and
        // precipitation — sampled across the year — and not an independent field.
        let (biotemperature_c, mean_annual_precipitation_mm) =
            ecology::holdridge_inputs(&s, a.x, a.z, a.terrain.elevation_m, 7);
        let expected = ecology::biome_at(
            &s,
            a.terrain.elevation_m,
            biotemperature_c,
            mean_annual_precipitation_mm,
        );
        assert_eq!(a.biome, expected);
    }

    #[test]
    fn the_biome_is_the_same_at_every_tick_of_the_year() {
        // **ECO-0001's regression, inverted.** C-93.2 measured four of four probed
        // columns changing biome within one world year and returning to where they
        // started — `(0, 0)` ran `boreal_forest ↔ tundra` on days 21 and 76 — because
        // the classifier was fed one tick of a season. Since the ruling of 2026-10-02 the
        // biome is classified on biotemperature and mean annual precipitation, sampled
        // from the same ticked shapers, so it must not move at all.
        let s = spec();
        let columns = [(0.0, 0.0), (1200.0, -800.0), (-3000.0, 2500.0), (6000.0, 6000.0)];
        // A small hydrology radius: this test reads the biome, which does not depend on
        // the watershed, and the sweep is 384 derivations.
        let hydro_radius = 2;
        let day = climate::YEAR_TICKS / 96;
        let at_zero: Vec<_> = columns
            .iter()
            .map(|(x, z)| derive(&s, *x, *z, 0, hydro_radius).biome)
            .collect();
        let mut t = 0u64;
        while t < climate::YEAR_TICKS {
            for (i, (x, z)) in columns.iter().enumerate() {
                let now = derive(&s, *x, *z, t, hydro_radius).biome;
                assert_eq!(
                    now, at_zero[i],
                    "the biome at ({x}, {z}) moved from {:?} to {now:?} at tick {t}",
                    at_zero[i]
                );
            }
            t += day;
        }
        // And a later year is the same world's climate: the cycle is periodic.
        for (i, (x, z)) in columns.iter().enumerate() {
            let later = derive(&s, *x, *z, climate::YEAR_TICKS * 3 + 7, hydro_radius).biome;
            assert_eq!(later, at_zero[i], "({x}, {z}) changed biome between world years");
        }
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
            actor: voxel_model::world::ActorId::player(),
            task: None,
            verb: voxel_model::world::PhysicalVerb::Place,
            reservation: None,
            transition: crate::earthworks::hand_change_id(
                x,
                y,
                z,
                base,
                target,
                &voxel_model::world::ActorId::player(),
                voxel_model::world::PhysicalVerb::Place,
            ),
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
        //
        // **The probe is a grid, and that was measured.** ECO-0001 replaced the
        // instantaneous inputs with Holdridge's aggregates — biotemperature and mean
        // annual precipitation, sampled from the ticked shapers — and the old
        // diagonal (`z = 0.7x`) then saw only `{Tundra, BorealForest}`. The diagonal
        // was thinner than the world, which is the fact the water test below also
        // records about itself.
        //
        // **The world's real set was then measured twice, and the first measurement
        // was of a defect.** `examples/biome-census.rs` swept this grid and read
        // `{Tundra, BorealForest, Alpine}` with a warmest biotemperature of
        // **2.536 °C** — because the climate envelope was applying latitude twice
        // (C-159.7). With that repaired the same sweep reads **eight** formations
        // (`logs/c159-biome-census-3.txt`), so the assertion below is now comfortably
        // clear rather than barely met — and the old figure is kept here only as the
        // record of what the defect looked like from this test.
        let s = spec();
        let mut biomes = std::collections::BTreeSet::new();
        let mut rocks = std::collections::BTreeSet::new();
        let mut x = -6000.0;
        while x <= 6000.0 {
            let mut z = -6000.0;
            while z <= 6000.0 {
                // A small hydrology radius: neither the biome nor the lithology
                // reads the watershed, and this is 1681 derivations.
                let c = derive(&s, x, z, 0, 2);
                biomes.insert(c.biome);
                rocks.insert(c.lithology);
                z += 300.0;
            }
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

    #[test]
    fn the_shading_field_varies_the_world_it_is_drawn_in() {
        // C-161's declaration. A field that varies **nothing** would pass every
        // count in this file and draw the flat blocks the report was about, so the
        // test asks for the picture's two halves: opaque materials are varied,
        // transmissive ones are not.
        let field = materials::shading_field().expect("the field declares");
        let registry = materials::registry().expect("the registry admits");
        assert_eq!(field.domain(), materials::SHADING_FIELD_DOMAIN);
        let mut opaque = 0;
        let mut transmissive = 0;
        for m in registry.materials() {
            let (roughness, reflectance) =
                field.gain(m.id).unwrap_or_else(|| panic!("{} has no gains", m.name));
            if m.optical_class == OpticalClass::Opaque {
                assert_eq!((roughness, reflectance), (0.5, 0.5), "{}", m.name);
                opaque += 1;
            } else {
                assert_eq!((roughness, reflectance), (0.0, 0.0), "{}", m.name);
                transmissive += 1;
            }
        }
        assert!(opaque > 0 && transmissive > 0, "the registry lost its two classes");
        // And every admitted material is declared — `none` for a material is "not
        // varied at this level at all", which is a claim this declaration does not
        // make about any of them.
        assert_eq!(field.len(), registry.admitted_material_count());
    }
}
