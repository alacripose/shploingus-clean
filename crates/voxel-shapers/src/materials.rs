//! The world's materials — the admitted rows, their shapers, and the rules that read them.
//!
//! These were `materials`, inside the column module. Nothing here is about a column:
//! this is the id set, the registry, the 256-level shading and displacement declarations, and
//! the two predicates — [`is_fluid`] and [`blocks_movement`] — that the movement check, the
//! pick and the mesher share. The column representation they used to live in is gone; the
//! field plane ([`crate::chunk_fields`]) is the world's only material law.
//!
//! # The rows are a placeholder, and the ledger says so
//!
//! F2 measured the old project's material registry and found **zero admitted materials**: the
//! registry machinery was complete and the content did not exist. [`registry`] admits
//! **thirteen**, each a single component at ratio 1, and they keep the current world and its
//! tests running — but they are **not authority**. Five of the thirteen (C-159.10) are the
//! **surface covers**: the material a biome's ground is made of, so the visible world varies
//! with the region instead of being one pale playa. `LEDGER.md` C-122 records the ruling: they
//! are an AI fabrication, and the target is a **derived** material library (real-world materials
//! with base patterns at the 4×4×4 slots and the addressed 256 level). A shape's material roles
//! must not be chosen from these materials; the roster's inset/seam/calyx slots are destinations
//! for the derived library's patterns. Anything else is a refusal
//! ([`voxel_model::RefusalKind::UnadmittedMaterial`]), which is the honest state rather than a
//! grey cube.

use crate::ecology::Biome;
use crate::geology::Lithology;
use crate::habitat::Habitat;
use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
use voxel_model::materials::spectra;
use voxel_model::{
    Component, ComponentId, DetailIndex, DisplacementField, Material, MaterialId, MaterialRegistry,
    OpticalClass, Ratio, Reflectance, Refusal, RefusalKind, Refused, ShadingField,
    SourcedReflectance,
};

/// Neighbourhood radius hydrology uses per column, in cells. Declared: shrinking
/// it shrinks the watershed, it does not approximate the same one.
pub const DEFAULT_HYDRO_RADIUS: usize = 12;

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

/// The admitted registry: thirteen rows of the world's own, each one component at ratio 1, and
/// the corpus's rows admitted on top of them, one per declared substance.
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

// ---- the 256-level displacement field (C-44) -------------------------
//
// `DisplacementField` is the **physical** shaper's declaration at the 256 level: what
// a surface does *between* the stored cells, keyed by material and face. It is the
// other half of the pair `ShaperSet::declare` returns, and it is what lets a block
// face carry a shape a rectangle cannot state — a pillar's cap, a boulder's flanks, a
// stalagmite's own law.
//
// **It is empty, and that is this world's honest state rather than a stub.** The
// roster that would declare it (`voxel-shapers::voxel`: `Column::pillar`, `Boulder`,
// `Flowstone`, the speleothems) is built and tested, and **generation places none of
// it**: `voxel-stream`'s world source writes whole voxels (`VoxelShape::solid`), so no
// block of this world holds a sub-block feature to state. Declaring a law here would
// draw relief the world does not contain, which is the one thing this layer must not
// do — so the field is empty, its entry count is zero, and the frame pays no
// triangles for a stage that is wired in. The day a shaper is placed at generation
// time it declares through the same writer and reaches the picture with no change to
// the client.

/// The domain tag of the world's displacement field — the seed-side sibling of
/// [`SHADING_FIELD_DOMAIN`], so the two fields cannot be the same field by accident.
pub const DISPLACEMENT_FIELD_DOMAIN: u64 = 0x0441_5D00_0000_0001;

/// The world's 256-level displacement field: what each material's faces do between the
/// stored cells.
///
/// Empty today, for the reason the section above states — **nothing is placed with a
/// sub-block shape yet**. It is a function rather than a constant so the seam has one
/// home: a shaper set that declares relief is declared through here and nowhere else,
/// and the day it is not empty is one edit in one place. Fallible, like
/// [`shading_field`], because a declaration that cannot be made must refuse rather
/// than return a field nobody can act on.
pub fn displacement_field() -> Refused<DisplacementField> {
    let mut field = DisplacementField::none();
    field.set_domain(DISPLACEMENT_FIELD_DOMAIN);
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
/// disagree about where the sea is ([`blocks_movement`] is the same lesson).
pub fn is_fluid(m: MaterialId) -> bool {
    m == WATER
}

/// Whether a material blocks movement. **The one rule about solidity.**
///
/// Exposed as its own function because a caller can hold a material *without*
/// holding a world: the client's collision asks the streamer's live record, which
/// already knows about the player's edits, and has to answer the same question the
/// field plane answers. Two copies of this rule is how a camera ends up walking
/// through a block the player just placed.
pub fn blocks_movement(m: MaterialId) -> bool {
    // Water is not solid; ice is. Air is not. A fluid is never something a body
    // stands on, which is why the fluid rule and the solidity rule are read from
    // the same predicate.
    m != AIR && !is_fluid(m)
}

/// The mean density of the admitted set, as a sanity check a caller can assert on.
pub fn mean_admitted_density() -> Refused<Mass> {
    let registry = registry()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn the_starter_registry_is_self_consistent() {
        let r = registry().unwrap();
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
        assert!(r.material(WATER).is_ok());
        assert_eq!(
            r.material(MaterialId(500)).unwrap_err().kind(),
            RefusalKind::UnadmittedMaterial
        );
        // Water is less dense than rock, which is the one physical claim here.
        let water = r.material(WATER).unwrap().bulk_density_kg_m3();
        let rock = r.material(IGNEOUS).unwrap().bulk_density_kg_m3();
        assert!(water.to_f64() < rock.to_f64());
    }
    
    #[test]
    fn the_terrain_expression_moves_the_cover_and_steep_shows_rock() {
        // C-160's table, pinned: the habitat is not a label, it is a change to what the
        // ground is made of.
        assert_eq!(
            surface_of(Biome::Grassland, Habitat::Lowland),
            Some(GRASS)
        );
        assert_eq!(
            surface_of(Biome::Grassland, Habitat::Dry),
            Some(DRY_GRASS)
        );
        assert_eq!(
            surface_of(Biome::Tundra, Habitat::Wet),
            Some(GRASS)
        );
        assert_eq!(
            surface_of(Biome::TemperateForest, Habitat::Montane),
            Some(CONIFER)
        );
        assert_eq!(
            surface_of(Biome::Desert, Habitat::Montane),
            Some(REGOLITH)
        );
        // Steep is not a cover: the caller resolves it to the lithology.
        assert_eq!(surface_of(Biome::Grassland, Habitat::Steep), None);
        // And a cover the table does not know passes through untouched.
        assert_eq!(
            surface_under(WATER, Habitat::Dry),
            Some(WATER)
        );
    }
    
    #[test]
    fn water_is_a_fluid_and_ice_is_a_block() {
        // **The one rule the edit refusal reads.** Water is world state; ice is a
        // mineable solid, and its material row waits for a real spectrum (C-143.3).
        assert!(is_fluid(WATER));
        assert!(!is_fluid(ICE), "ice blocks movement, so it is a block");
        assert!(!is_fluid(AIR), "air is nothing, not a fluid");
        assert!(!is_fluid(REGOLITH));
        // And solidity reads the same predicate, so the two cannot disagree:
        // a fluid is never something a body stands on.
        assert!(!blocks_movement(WATER));
        assert!(blocks_movement(ICE));
        assert!(blocks_movement(IGNEOUS));
        assert!(!blocks_movement(AIR));
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
        let field = shading_field().expect("the field declares");
        let registry = registry().expect("the registry admits");
        assert_eq!(field.domain(), SHADING_FIELD_DOMAIN);
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
    
    #[test]
    fn the_world_declares_no_relief_at_the_256_level_and_says_so() {
        // **The other end of the wire stage 3 is now connected to.** `Scene::build` is
        // handed this field every frame; what it must not find here is relief the world
        // does not contain. Generation writes whole voxels (`VoxelShape::solid`), so the
        // declaration is empty — and the honest form of "the stage is live and costs
        // nothing yet" is these counts rather than the absence of a call.
        let field = displacement_field().expect("the field declares");
        assert_eq!(field.domain(), DISPLACEMENT_FIELD_DOMAIN);
        assert!(field.is_empty(), "relief declared for a world made of whole voxels");
        assert_eq!(field.len(), 0);
        assert_eq!(field.materials(), 0);
        assert_eq!(field.profiled_faces(), 0);
        // **An empty field is the absence of a law, not a law of zero.** A face it does not
        // name has no entry at all, which is what makes `refine` leave that quad's rectangle
        // alone rather than translate it by nothing — the two are the same picture and
        // different claims, and the frame's own cost depends on which one is true.
        for m in registry().expect("the registry admits").materials() {
            for face in voxel_model::Face::ALL {
                assert!(field.entry(m.id, face).is_none(), "{} on {face:?}", m.name);
            }
        }
    }
}
