//! The chunk field path — the world sampled through the field plane.
//!
//! This module is where the cubic core's generation shape meets this tree's shapers:
//! a **chunk owner** samples declared fields on their lattices ([`FieldSource::for_owner`])
//! and the material rules read those fields, instead of the column composition being the
//! generator. `column` becomes a producer of individual field laws — a sampling detail —
//! and `voxel-stream`'s `WorldSource` is the caller that materialises the octree.
//!
//! # What the laws are, and what they are not
//!
//! A field's **law** here is a pure function of the world spec and a coordinate, and the
//! laws are this tree's already-written shapers (`terrain::elevation_m`, `geology::stratum_at`,
//! `substrate::soil_depth_m`, `climate::moisture`, `terrain::sample`). Nothing is invented:
//! the field plane supplies the **declaration** (resolution, sampling policy, seam policy,
//! identity) and the laws supply the values the shapers already produced.
//!
//! # The one named divergence from the reference's storage
//!
//! The reference's `FieldSampler` answers from a **stored sample set**, because its values
//! come from admitted manifest rows. This tree's world is procedural — the shapers are the
//! source of truth (C-7, C-119) — so [`FieldSource`] answers by evaluating the declared
//! field's law at the **canonical coordinate its contract names**. The declaration
//! vocabulary, the canonicalisation rules, the refusal rules and the identity are the
//! ported ones; only the storage is the law. This is recorded rather than hidden because a
//! stored-sample world and a law world are different facts, and the manifest path
//! ([`FieldSampler`](crate::field_sampler::FieldSampler)) is still the door for admitted data.
//!
//! # Named, not silently decided: the vertical axis
//!
//! The reference and `voxel-model::world::region` put the vertical axis on **z**. This
//! tree's shapers, streamer and client are consistently **y-vertical** (columns are
//! `(x, z)`, `fy > surface` is the above-ground test). That disagreement is a material
//! decision the user owns, so the declarations here use **pass-through vertical
//! dependence** (`UsesZ { resolution: 1 }`) — a query is read where it is asked, with no
//! axis pinned — and neither convention is baked in. When the axis is ruled, a
//! `HorizontalOnly` declaration pins the ruled axis and the canonicalisation does the rest.
//!
//! # Named, not silently dropped: the atmosphere
//!
//! The roster in [`crate::field_sampler::FieldId`] carries the climate, atmosphere and
//! hydrology fields (Pressure, Wind, Storms, Humidity, Groundwater, Precipitation,
//! Radiation …). They have **no producer here yet**, so querying them refuses by name;
//! they are produced by the climate/atmosphere stage cut, as *world* fields feeding the
//! systems — never a screen-space tint — and Radiation is the field through which the
//! sky/light pipeline (OpenPBR/NPR) reads the world.

use voxel_model::coord::{ChunkPos, VoxelPos};
use voxel_model::matter::Rational;
use voxel_model::{MaterialId, Refusal, RefusalKind, Refused};

use crate::caves::{self, Void};
use crate::climate;
use crate::ecology::{self, Biome};
use crate::column::{self, materials};
use crate::field_sampler::{
    canonical_query_of, DraftFieldSampler, DraftFieldSpec, FieldId, FieldResolution,
    FieldSamplerIdentity, FieldValue, FixedValue, ResolutionLevel, SamplingPolicy, SeamPolicy, Unit,
    VerticalDependence,
};
use crate::flora;
use crate::geology::{self, Lithology};
use crate::habitat;
use crate::process::{self, Site};
use crate::spec::WorldSpec;
use crate::substrate;
use crate::terrain;

/// The fields this world's material path samples.
///
/// The list is exactly the fields the material rules read, plus the fields their laws
/// read: a declaration with no reader would be decoration, and a reader with no
/// declaration is a refusal by name.
pub fn declared_fields() -> Refused<Vec<DraftFieldSpec>> {
    let measured = |id: FieldId, unit: Unit, source: &str| -> Refused<DraftFieldSpec> {
        Ok(DraftFieldSpec {
            id,
            unit,
            // Step 1 on every axis: this tree's shapers are laws about the metre they are
            // asked about, and a coarser lattice would be a *different world* until the
            // stage cut declares where the world may be sampled coarsely. The resolution
            // vocabulary is the reference's; the step is the honest one for a law world.
            resolution: FieldResolution::new(VoxelPos::new(0, 0, 0), 8, 1, Some(1)),
            vertical_dependence: VerticalDependence::UsesZ { resolution: 1 },
            sampling: SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 1,
            },
            seam_policy: SeamPolicy::GlobalLattice,
            tolerance: Rational::new(1, 1000)?,
            source_ids: vec![source.to_owned()],
        })
    };
    Ok(vec![
        measured(
            FieldId::Elevation,
            Unit::Metre,
            "docs/SHAPERS.md §1 terrain (Mandelbrot 1982; Musgrave, Kolb & Mace 1989)",
        )?,
        measured(
            FieldId::Slope,
            Unit::Ratio,
            "docs/SHAPERS.md §1 terrain (Mandelbrot 1982; Musgrave, Kolb & Mace 1989)",
        )?,
        measured(
            FieldId::Moisture,
            Unit::Ratio,
            "docs/SHAPERS.md §1 climate (Whittaker 1960)",
        )?,
        measured(
            FieldId::Soil,
            Unit::Metre,
            "docs/SHAPERS.md §1 substrate (Heimsath, Dietrich, Nishiizumi & Finkel 1997)",
        )?,
        Ok(DraftFieldSpec {
            id: FieldId::Geology,
            unit: Unit::Dimensionless,
            resolution: FieldResolution::new(VoxelPos::new(0, 0, 0), 8, 1, Some(1)),
            vertical_dependence: VerticalDependence::UsesZ { resolution: 1 },
            sampling: SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 1,
            },
            seam_policy: SeamPolicy::GlobalLattice,
            // A class field makes no tolerance claim about a value nobody measured: the
            // class *is* the value, and `0/1` would claim exactness about it.
            tolerance: Rational::ZERO,
            source_ids: vec![
                "docs/SHAPERS.md §1 geology (Allen & Allen 2013)".to_owned(),
            ],
        })?,
        Ok(DraftFieldSpec {
            id: FieldId::Climate,
            unit: Unit::Dimensionless,
            resolution: FieldResolution::new(VoxelPos::new(0, 0, 0), 8, 1, Some(1)),
            vertical_dependence: VerticalDependence::UsesZ { resolution: 1 },
            sampling: SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 1,
            },
            seam_policy: SeamPolicy::GlobalLattice,
            // A class field: the Holdridge formation *is* the value (ECO-0001).
            tolerance: Rational::ZERO,
            source_ids: vec![
                "docs/SHAPERS.md §1 ecology (Holdridge 1947)".to_owned(),
            ],
        })?,
        // **The process roster's fields**, declared here because the roster produces them
        // and this plane is where a field is read. Their declarations are the same shape
        // as the material fields': a step-1 lattice, which is what a law that reads the
        // metre it is asked about can honestly promise.
        Ok(DraftFieldSpec {
            id: FieldId::Tectonics,
            unit: Unit::Dimensionless,
            resolution: FieldResolution::new(VoxelPos::new(0, 0, 0), 8, 1, Some(1)),
            vertical_dependence: VerticalDependence::UsesZ { resolution: 1 },
            sampling: SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 1,
            },
            seam_policy: SeamPolicy::GlobalLattice,
            // A plate **id**, not a measurement: the id *is* the value.
            tolerance: Rational::ZERO,
            source_ids: vec![
                "docs/SHAPERS.md §1 tectonics (Kreischer & Bommes 2019)".to_owned(),
            ],
        })?,
        measured(
            FieldId::ErosionResistance,
            Unit::Ratio,
            "docs/SHAPERS.md §2 erosion (Whipple & Tucker 1999; Lague 2014)",
        )?,
        measured(
            FieldId::Temperature,
            Unit::DegreesCelsius,
            "docs/SHAPERS.md §1 climate (Whittaker 1960; Köppen 1884)",
        )?,
        measured(
            FieldId::Groundwater,
            Unit::Metre,
            "docs/SHAPERS.md §1 hydrology (O'Callaghan & Mark 1984)",
        )?,
        measured(
            FieldId::Pressure,
            Unit::Pascal,
            "docs/SHAPERS.md §1 atmosphere (US Standard Atmosphere 1976)",
        )?,
        measured(
            FieldId::Precipitation,
            Unit::Metre,
            "docs/SHAPERS.md §1 weather (Smith & Barstad 2004)",
        )?,
        measured(
            FieldId::Wind,
            Unit::Dimensionless,
            "docs/SHAPERS.md §1 weather (Smith & Barstad 2004)",
        )?,
        measured(
            FieldId::CarryingCapacity,
            Unit::Ratio,
            "docs/SHAPERS.md §1 ecology (Lieth 1975)",
        )?,
        measured(
            FieldId::SpeciesSuitability,
            Unit::Ratio,
            "docs/SHAPERS.md §1 ecology (Holdridge 1947)",
        )?,
    ])
}

/// The law of a field: a pure function of the world spec and a canonical coordinate.
///
/// The laws *read each other through the query interface*, which is the reference's stage
/// coupling in this tree's vocabulary: `Soil` reads `Elevation`, `Slope`, `Moisture` and
/// `Geology`; `Geology` reads `Elevation`; nothing reads `Soil`, so the recursion is
/// bounded by the dependency order, as the stage graph requires.
type FieldLaw = fn(&dyn FieldQuery, &WorldSpec, VoxelPos) -> Refused<FieldValue>;

fn law_of(field: FieldId) -> Option<FieldLaw> {
    match field {
        FieldId::Elevation => Some(elevation_law),
        FieldId::Slope => Some(slope_law),
        FieldId::Moisture => Some(moisture_law),
        FieldId::Soil => Some(soil_law),
        FieldId::Geology => Some(geology_law),
        FieldId::Climate => Some(climate_law),
        // **The process roster's fields.** Each is produced by the stage that declares it
        // (`process::field_at`), so the plane and the runtime answer one value for one
        // field. `ErosionResistance` is a law about the unit centred on the query; the
        // rest are site laws and read where they are asked.
        FieldId::Tectonics => Some(tectonics_law),
        FieldId::ErosionResistance => Some(erosion_resistance_law),
        FieldId::Temperature => Some(temperature_law),
        FieldId::Groundwater => Some(groundwater_law),
        FieldId::Pressure => Some(pressure_law),
        FieldId::Precipitation => Some(precipitation_law),
        FieldId::Wind => Some(wind_law),
        FieldId::CarryingCapacity => Some(carrying_capacity_law),
        FieldId::SpeciesSuitability => Some(species_suitability_law),
        // Declared in the nineteen-name roster and produced by **no stage** in this cut
        // (`process::owning_stage` is `None`). A refusal by name is the honest answer; a
        // zero would be a value somebody could believe.
        FieldId::SoilFertility
        | FieldId::Humidity
        | FieldId::Storms
        | FieldId::Radiation => None,
    }
}

/// A field query: the one interface the material rules and the laws are written against.
pub trait FieldQuery {
    fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue>;
}

/// A world's declared field plane, sampled per **owner chunk**.
pub struct FieldSource<'a> {
    spec: &'a WorldSpec,
    specs: Vec<DraftFieldSpec>,
}

impl<'a> FieldSource<'a> {
    pub fn new(spec: &'a WorldSpec) -> Refused<Self> {
        let specs = declared_fields()?;
        // The declarations are validated through the ported constructor before anything
        // is sampled from them, so an inconsistent lattice cannot reach a query.
        DraftFieldSampler::new(specs.clone(), Vec::new())?;
        Ok(Self { spec, specs })
    }

    pub const fn world(&self) -> &'a WorldSpec {
        self.spec
    }

    pub fn spec(&self, field: FieldId) -> Refused<&DraftFieldSpec> {
        self.specs
            .iter()
            .find(|spec| spec.id == field)
            .ok_or_else(|| undeclared(field))
    }

    /// The whole declaration set's identity — the thing a resident digest can bind to.
    pub fn identity(&self) -> Refused<FieldSamplerIdentity> {
        DraftFieldSampler::new(self.specs.clone(), Vec::new()).map(|sampler| sampler.identity())
    }

    /// The owner-scoped handle: a chunk samples its fields as *that chunk*.
    pub const fn for_owner(&self, _owner: ChunkPos) -> ChunkFieldSource<'_> {
        ChunkFieldSource { source: self }
    }
}

impl FieldQuery for FieldSource<'_> {
    fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue> {
        // The producer check comes first. Every [`FieldId`] is a field the
        // reference vocabulary declares, so the owed atmosphere/hydrology roster
        // must say **that** ("declared but has no producer in this cut") rather
        // than reading as a malformed declaration. A field that has a producer
        // but no declaration in this plane is a programming error and is refused
        // as an invalid specification.
        let law = law_of(field).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::Unimplemented,
                format!(
                    "field {} is declared but has no producer in this cut",
                    field.as_str()
                ),
            )
        })?;
        let spec = self.spec(field)?;
        let canonical = canonical_query_of(spec, coord)?;
        law(self, self.spec, canonical)
    }
}

/// A chunk-scoped field plane. The same values as its source — the owner is the scope a
/// residency policy will read when it exists, exactly as in the reference.
pub struct ChunkFieldSource<'a> {
    source: &'a FieldSource<'a>,
}

impl ChunkFieldSource<'_> {
    pub fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue> {
        FieldQuery::sample(self.source, field, coord)
    }
}

/// A chunk owner **is** a query: the material rules take any [`FieldQuery`], and a chunk
/// that had to be unwrapped to be read would be an owner in name only.
impl FieldQuery for ChunkFieldSource<'_> {
    fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue> {
        FieldQuery::sample(self.source, field, coord)
    }
}

/// The direct query: the canonicalisation of a step-1 lattice is the identity, so a
/// single-voxel caller can read the same laws without building a declaration set. The
/// equivalence is pinned by `the_direct_query_and_the_declared_plane_agree`, not assumed.
struct DirectQuery<'a> {
    spec: &'a WorldSpec,
}

impl FieldQuery for DirectQuery<'_> {
    fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue> {
        let law = law_of(field).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::Unimplemented,
                format!(
                    "field {} is declared but has no producer in this cut",
                    field.as_str()
                ),
            )
        })?;
        law(self, self.spec, coord)
    }
}

// ---- the laws -------------------------------------------------------------------------

fn elevation_law(
    _fields: &dyn FieldQuery,
    spec: &WorldSpec,
    coord: VoxelPos,
) -> Refused<FieldValue> {
    let metres = terrain::elevation_m(spec, coord.x as f64 + 0.5, coord.z as f64 + 0.5);
    Ok(FieldValue::Fixed(fixed_from_f64(metres)?))
}

fn slope_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    let slope = terrain::sample(spec, coord.x as f64 + 0.5, coord.z as f64 + 0.5).slope;
    Ok(FieldValue::Fixed(fixed_from_f64(slope)?))
}

fn moisture_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    let moisture = climate::moisture(spec, coord.x as f64 + 0.5, coord.z as f64 + 0.5);
    Ok(FieldValue::Fixed(fixed_from_f64(moisture)?))
}

fn geology_law(fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    let surface = f64_of(
        fields.sample(FieldId::Elevation, coord)?,
        FieldId::Elevation,
    )?;
    let lithology = geology::stratum_at(
        spec,
        coord.x as f64 + 0.5,
        coord.z as f64 + 0.5,
        surface,
        coord.y as f64 + 0.5,
    )
    .lithology;
    Ok(FieldValue::Integer(lithology_code(lithology)))
}

/// The Holdridge formation a column's climate supports — the field the surface material
/// is read from (C-159.10).
///
/// It reads the **region's** climate rather than the moment's: [`ecology::holdridge_inputs`]
/// aggregates biotemperature and mean annual precipitation from the ticked shapers, and the
/// aggregate does not move with the tick (`holdridge_inputs_are_sampled_and_do_not_move_within_a_year`),
/// so the tick 0 here is the honest absence of a seasonal reading. The elevation comes from
/// the `Elevation` law, exactly as `geology_law` and `soil_law` take it, so the formation and
/// the ground it decides cannot be reading two different columns.
///
/// **This is why the plane has a `Climate` producer rather than a biome material constant**:
/// the formation is already the shaper's own classifier, and a second classifier here would
/// be a second answer to "what biome is this".
fn climate_law(fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    let surface = f64_of(fields.sample(FieldId::Elevation, coord)?, FieldId::Elevation)?;
    let (biotemperature_c, mean_annual_precipitation_mm) = ecology::holdridge_inputs(
        spec,
        coord.x as f64 + 0.5,
        coord.z as f64 + 0.5,
        surface,
        0,
    );
    let biome = ecology::biome_at(
        spec,
        surface,
        biotemperature_c,
        mean_annual_precipitation_mm,
    );
    Ok(FieldValue::Integer(biome_code(biome)))
}

fn soil_law(fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    let surface = f64_of(
        fields.sample(FieldId::Elevation, coord)?,
        FieldId::Elevation,
    )?;
    let slope = f64_of(fields.sample(FieldId::Slope, coord)?, FieldId::Slope)?;
    let moisture = f64_of(
        fields.sample(FieldId::Moisture, coord)?,
        FieldId::Moisture,
    )?;
    // **The stratum at this voxel's own depth**, exactly as `ColumnProfile::material_at`
    // read it: a column whose rock changes with depth changes its soil with depth too,
    // and a single per-column soil depth would be a different world.
    let lithology = geology::stratum_at(
        spec,
        coord.x as f64 + 0.5,
        coord.z as f64 + 0.5,
        surface,
        coord.y as f64 + 0.5,
    )
    .lithology;
    let depth = substrate::soil_depth_m(spec, lithology, slope, moisture);
    Ok(FieldValue::Fixed(fixed_from_f64(depth)?))
}

// ---- the fields the process roster produces --------------------------------------------
//
// Nine fields were declared in the nineteen-name roster and **produced by nobody**: the
// structural, atmosphere and hydrology fields this module's own doc names as owed. They
// are produced now by the **process roster** ([`process::field_at`]), which is that
// roster's production caller: each law below is one line that hands the field to the stage
// that declares it, so the plane and the runtime cannot answer two different values for
// one field. The stage is found from its own `outputs` declaration, so this cannot drift.
//
// **Tick 0 is the honest absence of a seasonal reading**, the same convention `climate_law`
// already uses for `holdridge_inputs`: the plane is a function of the world spec and the
// coordinate, and a ticked value read at a tick the plane is not asked about would be a
// different world.

fn process_field(field: FieldId, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process::field_at(spec, field, Site::new(coord.x, coord.y, coord.z), 0)
}

fn tectonics_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process_field(FieldId::Tectonics, spec, coord)
}

fn erosion_resistance_law(
    _fields: &dyn FieldQuery,
    spec: &WorldSpec,
    coord: VoxelPos,
) -> Refused<FieldValue> {
    process_field(FieldId::ErosionResistance, spec, coord)
}

fn temperature_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process_field(FieldId::Temperature, spec, coord)
}

fn groundwater_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process_field(FieldId::Groundwater, spec, coord)
}

fn pressure_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process_field(FieldId::Pressure, spec, coord)
}

fn precipitation_law(
    _fields: &dyn FieldQuery,
    spec: &WorldSpec,
    coord: VoxelPos,
) -> Refused<FieldValue> {
    process_field(FieldId::Precipitation, spec, coord)
}

fn wind_law(_fields: &dyn FieldQuery, spec: &WorldSpec, coord: VoxelPos) -> Refused<FieldValue> {
    process_field(FieldId::Wind, spec, coord)
}

fn carrying_capacity_law(
    _fields: &dyn FieldQuery,
    spec: &WorldSpec,
    coord: VoxelPos,
) -> Refused<FieldValue> {
    process_field(FieldId::CarryingCapacity, spec, coord)
}

fn species_suitability_law(
    _fields: &dyn FieldQuery,
    spec: &WorldSpec,
    coord: VoxelPos,
) -> Refused<FieldValue> {
    process_field(FieldId::SpeciesSuitability, spec, coord)
}

// ---- the material rules ---------------------------------------------------------------

/// The material a voxel holds, **read through the field plane**.
///
/// This is the same order of tests `column::ColumnProfile::material_at` documents — above
/// ground: sea or air; below: soil, then rock or a void — with its inputs coming from
/// field queries instead of a column profile. `the_field_plane_materializes_exactly_what_the_column_profile_says`
/// pins the equivalence against the column definition over a whole chunk.
pub fn material_of<Q: FieldQuery + ?Sized>(
    fields: &Q,
    spec: &WorldSpec,
    x: i32,
    y: i32,
    z: i32,
) -> Refused<MaterialId> {
    let fx = x as f64 + 0.5;
    let fy = y as f64 + 0.5;
    let fz = z as f64 + 0.5;
    let sea = spec.config.sea_level_m;
    let coord = VoxelPos::new(x, y, z);

    let surface = f64_of(fields.sample(FieldId::Elevation, coord)?, FieldId::Elevation)?;
    if fy > surface {
        // Above ground: sea where the ground is below sea level; a tree where one reaches
        // this voxel — `flora` is a pure function of position (C-159.11), the same call
        // the column definition makes — and air otherwise.
        if fy <= sea {
            return Ok(materials::WATER);
        }
        if let Some(material) = flora::tree_material_at(spec, x, y, z, surface) {
            return Ok(material);
        }
        return Ok(materials::AIR);
    }

    let depth = surface - fy;
    let soil = f64_of(fields.sample(FieldId::Soil, coord)?, FieldId::Soil)?;
    if depth <= soil && soil > 0.0 {
        // **The biome decides the cover and the terrain expression its variant**
        // (C-159.10, C-160) — the same composition the column definition reads, which is
        // what keeps `the_field_plane_materializes_exactly_what_the_column_profile_says`
        // true. The slope and moisture come from the declared fields the plane already
        // carries, and `ColumnMemo` caches them per column, so the habitat costs no new
        // producers and no new terrain evaluations.
        let biome = biome_of(fields.sample(FieldId::Climate, coord)?)?;
        let slope = f64_of(fields.sample(FieldId::Slope, coord)?, FieldId::Slope)?;
        let moisture = f64_of(fields.sample(FieldId::Moisture, coord)?, FieldId::Moisture)?;
        let habitat = habitat::habitat_of(spec, surface, slope, moisture);
        match materials::surface_of(biome, habitat) {
            Some(cover) => return Ok(cover),
            // `Steep`: bare ground shows the lithology's own rock, read exactly where
            // the rock branch below reads it.
            None => {
                let lithology = lithology_of(fields.sample(FieldId::Geology, coord)?)?;
                return crate::corpus::rock_voxel(spec, lithology, fx, fy, fz, depth);
            }
        }
    }

    let lithology = lithology_of(fields.sample(FieldId::Geology, coord)?)?;
    let params = column::cave_params(spec);
    Ok(match caves::void_at(spec, lithology, fx, fy, fz, surface, &params) {
        Void::Open => materials::AIR,
        Void::Flooded => materials::WATER,
        // The rock is the **declared rock this region's bed is made of**, with an ore
        // where the vein field puts one — the same call the column path makes, so the two
        // cannot draw two different rocks at one voxel (`corpus::rock_voxel`).
        Void::Rock => crate::corpus::rock_voxel(spec, lithology, fx, fy, fz, depth)?,
    })
}

/// The **generated** material at a voxel — the single-voxel entry point, for picks and for
/// the mesher's boundary reads.
pub fn material_at(spec: &WorldSpec, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
    material_of(&DirectQuery { spec }, spec, x, y, z)
}

// ---- value encodings ------------------------------------------------------------------

/// An `f64` as an exact fixed-point value.
///
/// Every shaper value is a finite `f64`; its bits *are* a dyadic rational, so this is a
/// re-encoding and not a rounding. A value whose exact form does not fit the plane's
/// integer range is refused rather than rounded — the same rule the tree applies
/// everywhere else.
pub fn fixed_from_f64(value: f64) -> Refused<FixedValue> {
    if !value.is_finite() {
        return Err(Refusal::new(
            RefusalKind::NonFiniteValue,
            "a field value must be finite",
        ));
    }
    if value == 0.0 {
        return Ok(FixedValue::new(0, 1).expect("a positive scale"));
    }
    let bits = value.to_bits();
    let negative = bits >> 63 == 1;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i64;
    if exponent_bits == 0 {
        return Err(Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "a subnormal field value is outside the plane's exact range",
        ));
    }
    let mantissa = (bits & ((1u64 << 52) - 1)) as i128 + (1i128 << 52);
    let exponent = exponent_bits - 1023 - 52;
    let (mut raw, scale) = if exponent >= 0 {
        let shift = u32::try_from(exponent).map_err(|_| {
            Refusal::new(RefusalKind::OutOfDeclaredRange, "field exponent out of range")
        })?;
        if shift > 74 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "field exponent exceeds the plane's exact range",
            ));
        }
        (mantissa << shift, 1i128)
    } else {
        let shift = u32::try_from(-exponent).map_err(|_| {
            Refusal::new(RefusalKind::OutOfDeclaredRange, "field exponent out of range")
        })?;
        if shift > 126 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "field scale exceeds the plane's exact range",
            ));
        }
        (mantissa, 1i128 << shift)
    };
    if negative {
        raw = -raw;
    }
    FixedValue::new(raw, scale).ok_or_else(|| {
        Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "field value cannot be represented exactly",
        )
    })
}

/// The `f64` a fixed-point value stands for. Exact for the values the shapers produce —
/// the scale is a power of two and the numerator fits an `f64` mantissa — which
/// `the_fixed_encoding_round_trips_through_f64` pins rather than assumes.
pub fn f64_of_fixed(value: FixedValue) -> f64 {
    value.raw() as f64 / value.scale() as f64
}

fn f64_of(value: FieldValue, field: FieldId) -> Refused<f64> {
    match value {
        FieldValue::Fixed(fixed) => Ok(f64_of_fixed(fixed)),
        FieldValue::Rational(rational) => Ok(rational.numerator() as f64 / rational.denominator() as f64),
        FieldValue::Integer(_) => Err(Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!("field {} holds an integer, not a measured value", field.as_str()),
        )),
    }
}

pub(crate) fn lithology_code(lithology: Lithology) -> i128 {
    match lithology {
        Lithology::Regolith => 0,
        Lithology::Sedimentary => 1,
        Lithology::Metamorphic => 2,
        Lithology::Igneous => 3,
        Lithology::Bedrock => 4,
    }
}

/// The formation's class code in the field plane.
///
/// The codes are this module's interface encoding, exactly as [`lithology_code`] is: the
/// field plane carries integers, and the meaning lives where the class is defined.
pub(crate) fn biome_code(biome: Biome) -> i128 {
    match biome {
        Biome::PolarDesert => 0,
        Biome::Tundra => 1,
        Biome::BorealForest => 2,
        Biome::CoolTemperateForest => 3,
        Biome::TemperateForest => 4,
        Biome::Grassland => 5,
        Biome::Desert => 6,
        Biome::SubtropicalScrub => 7,
        Biome::TropicalDryForest => 8,
        Biome::TropicalRainforest => 9,
        Biome::Alpine => 10,
    }
}

fn biome_of(value: FieldValue) -> Refused<Biome> {
    let FieldValue::Integer(code) = value else {
        return Err(Refusal::new(
            RefusalKind::InvalidFieldSpec,
            "the climate field must hold a formation class code",
        ));
    };
    match code {
        0 => Ok(Biome::PolarDesert),
        1 => Ok(Biome::Tundra),
        2 => Ok(Biome::BorealForest),
        3 => Ok(Biome::CoolTemperateForest),
        4 => Ok(Biome::TemperateForest),
        5 => Ok(Biome::Grassland),
        6 => Ok(Biome::Desert),
        7 => Ok(Biome::SubtropicalScrub),
        8 => Ok(Biome::TropicalDryForest),
        9 => Ok(Biome::TropicalRainforest),
        10 => Ok(Biome::Alpine),
        other => Err(Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!("biome code {other} is not a declared class"),
        )),
    }
}

fn lithology_of(value: FieldValue) -> Refused<Lithology> {
    let FieldValue::Integer(code) = value else {
        return Err(Refusal::new(
            RefusalKind::InvalidFieldSpec,
            "the geology field must hold a lithology class code",
        ));
    };
    match code {
        0 => Ok(Lithology::Regolith),
        1 => Ok(Lithology::Sedimentary),
        2 => Ok(Lithology::Metamorphic),
        3 => Ok(Lithology::Igneous),
        4 => Ok(Lithology::Bedrock),
        other => Err(Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!("lithology code {other} is not a declared class"),
        )),
    }
}

fn undeclared(field: FieldId) -> Refusal {
    Refusal::owned(
        RefusalKind::InvalidFieldSpec,
        format!(
            "invalid field specification for {}: field specification is not present",
            field.as_str()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_sampler::DraftFieldSample;
    use crate::spec::Config;

    fn world() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    #[test]
    fn the_declared_roster_is_the_material_fields_plus_the_process_rosters_own() {
        // This used to assert the roster was **exactly** the six fields the material path
        // reads. It is not any more, and the reason is the change this round made: nine
        // fields the process roster produces are declared so the plane can answer them.
        // The teeth are kept, and strengthened: **every declared field must have a law**,
        // so a declaration nobody can read — the decoration the module doc forbids —
        // still fails here.
        let specs = declared_fields().expect("the declarations are consistent");
        assert_eq!(specs.len(), 15, "the material fields plus nine roster fields");
        let ids: Vec<FieldId> = specs.iter().map(|spec| spec.id).collect();
        for required in [
            FieldId::Elevation,
            FieldId::Slope,
            FieldId::Moisture,
            FieldId::Soil,
            FieldId::Geology,
            FieldId::Climate,
            FieldId::Tectonics,
            FieldId::ErosionResistance,
            FieldId::Temperature,
            FieldId::Groundwater,
            FieldId::Pressure,
            FieldId::Precipitation,
            FieldId::Wind,
            FieldId::CarryingCapacity,
            FieldId::SpeciesSuitability,
        ] {
            assert!(ids.contains(&required), "{required:?} is not declared");
        }
        for spec in &specs {
            assert!(
                law_of(spec.id).is_some(),
                "{:?} is declared with no law to read it",
                spec.id
            );
        }
        // The declaration set validates through the ported constructor, including the
        // rule that vertical dependence and vertical resolution must agree.
        DraftFieldSampler::new(specs, Vec::new()).expect("the set is a valid draft catalog");
    }

    #[test]
    fn the_planes_roster_fields_are_the_shapers_own_numbers() {
        // End to end through the plane: each process-produced field must be the value of
        // the shaper that owns it, computed here independently, so the seam cannot agree
        // with itself by both halves being wrong.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let coord = VoxelPos::new(6, 20, -3);
        let (x, y, z) = (coord.x as f64 + 0.5, coord.y as f64 + 0.5, coord.z as f64 + 0.5);
        let fixed = |field: FieldId| match FieldQuery::sample(&source, field, coord)
            .unwrap_or_else(|refusal| panic!("{field:?} refused: {}", refusal.detail()))
        {
            FieldValue::Fixed(value) => f64_of_fixed(value),
            other => panic!("{field:?} is a measured field and came back {other:?}"),
        };
        let integer = |field: FieldId| match FieldQuery::sample(&source, field, coord)
            .unwrap_or_else(|refusal| panic!("{field:?} refused: {}", refusal.detail()))
        {
            FieldValue::Integer(value) => value,
            other => panic!("{field:?} is a class field and came back {other:?}"),
        };

        assert_eq!(
            integer(FieldId::Tectonics),
            crate::tectonics::sample(&spec, x, z).plate_id as i128
        );
        let climate = crate::climate::sample(&spec, x, z, 0);
        assert!((fixed(FieldId::Temperature) - climate.temperature_c).abs() < 1e-12);
        let weather = crate::weather::sample(&spec, x, z, 0);
        assert!((fixed(FieldId::Precipitation) - weather.precipitation_mm / 1000.0).abs() < 1e-12);
        assert!((fixed(FieldId::Wind) - weather.wind_bearing_rad).abs() < 1e-12);
        assert!(
            (fixed(FieldId::Groundwater)
                - crate::hydrology::sample(&spec, x, z, process::FIELD_PLANE_HYDRO_RADIUS).water_depth_m)
                .abs()
                < 1e-12
        );
        assert!(
            (fixed(FieldId::Pressure) - crate::atmosphere::sample(&spec, y).pressure_kpa * 1000.0)
                .abs()
                < 1e-6
        );

        // The two capacities, from the ecology laws over the same substrate and climate.
        let surface = crate::terrain::elevation_m(&spec, x, z);
        let (biotemperature, precipitation) =
            crate::ecology::holdridge_inputs(&spec, x, z, surface, 0);
        let slope = crate::terrain::sample(&spec, x, z).slope;
        let stratum = crate::geology::stratum_at(&spec, x, z, surface, y);
        let moisture = crate::climate::moisture(&spec, x, z);
        let soil = crate::substrate::soil_depth_m(&spec, stratum.lithology, slope, moisture);
        assert!(
            (fixed(FieldId::CarryingCapacity)
                - crate::ecology::carrying_capacity(biotemperature, precipitation, soil)
                    .expect("finite"))
                .abs()
                < 1e-12
        );
        assert!(
            (fixed(FieldId::SpeciesSuitability)
                - crate::ecology::species_suitability(biotemperature, precipitation, soil)
                    .expect("finite"))
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn the_field_identity_is_the_declaration_set_s_identity_and_is_stable() {
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let again = FieldSource::new(&spec).expect("the plane builds");
        assert_eq!(
            source.identity().expect("identity"),
            again.identity().expect("identity"),
            "the same declarations gave two identities"
        );
        let via_draft = DraftFieldSampler::new(declared_fields().expect("declared"), Vec::new())
            .expect("valid")
            .identity();
        assert_eq!(source.identity().expect("identity"), via_draft);
    }

    #[test]
    fn the_field_plane_materializes_exactly_what_the_column_profile_says() {
        // The load-bearing equivalence: the field path is the *same world* as the column
        // definition it replaces, or the cut changed the world rather than the foundation.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let owner = source.for_owner(ChunkPos::new(0, 1, 0));
        let pos = ChunkPos::new(0, 1, 0);
        let o = pos.origin();
        let mut compared = 0u32;
        for lz in 0..voxel_model::coord::CHUNK_SIDE {
            for lx in 0..voxel_model::coord::CHUNK_SIDE {
                let (x, z) = (o.x + lx, o.z + lz);
                let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                let profile = column::profile_with_surface(&spec, x as f64 + 0.5, z as f64 + 0.5, surface);
                for ly in 0..voxel_model::coord::CHUNK_SIDE {
                    let y = o.y + ly;
                    let through_fields = material_of(&owner, &spec, x, y, z)
                        .expect("the material path is total here");
                    let through_columns = profile
                        .material_at(&spec, x, y, z)
                        .expect("the column path is total here");
                    assert_eq!(
                        through_fields, through_columns,
                        "the field plane and the column definition disagree at ({x}, {y}, {z})"
                    );
                    compared += 1;
                }
            }
        }
        assert!(compared > 30_000, "the comparison was too small to mean anything");
    }

    #[test]
    fn the_field_plane_grows_the_trees_the_column_definition_grows() {
        // A tree's voxels are **above** the ground, which is a branch the two paths reach
        // with different inputs (the field path reads `Elevation`, the profile is handed
        // it). This walks one real tree's trunk and canopy through both, with each
        // neighbour column's own profile — the canopy spans columns, and every one of
        // them must agree about what is in the air.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let owner = source.for_owner(ChunkPos::new(0, 0, 0));
        let mut found = None;
        'search: for cell_x in 0..64 {
            for cell_z in 0..64 {
                if let Some(tree) = flora::tree_of_cell(&spec, cell_x, cell_z) {
                    found = Some(tree);
                    break 'search;
                }
            }
        }
        let tree = found.expect("no tree in the first 64×64 cells of this world");
        let mut compared = 0u32;
        let mut tree_voxels = 0u32;
        for dy in 0..=(tree.trunk_height + 2) {
            for dx in -3..=3 {
                for dz in -3..=3 {
                    let (x, z) = (tree.base_x + dx, tree.base_z + dz);
                    let y = tree.base_y + dy;
                    let through_fields = material_of(&owner, &spec, x, y, z)
                        .expect("the material path is total here");
                    let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                    let profile = column::profile_with_surface(
                        &spec,
                        x as f64 + 0.5,
                        z as f64 + 0.5,
                        surface,
                    );
                    let through_columns = profile
                        .material_at(&spec, x, y, z)
                        .expect("the column path is total here");
                    assert_eq!(
                        through_fields, through_columns,
                        "the two paths disagree about the tree at ({x}, {y}, {z})"
                    );
                    if through_fields == materials::WOOD
                        || through_fields == materials::LEAF
                        || through_fields == materials::CONIFER
                    {
                        tree_voxels += 1;
                    }
                    compared += 1;
                }
            }
        }
        assert!(compared > 300, "the tree walk was too small to mean anything");
        assert!(tree_voxels > 0, "the walk found no tree voxels at all");
    }

    #[test]
    fn the_direct_query_and_the_declared_plane_agree() {
        // `material_at` reads the laws directly (no declaration set) because a step-1
        // lattice canonicalises to the identity; this pins that the shortcut is the same
        // world, not a second rule.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let owner = source.for_owner(ChunkPos::new(0, 1, 0));
        for (x, y, z) in [(0, 20, 0), (7, 32, 3), (-1, 15, 9), (31, 48, 31)] {
            let direct = material_at(&spec, x, y, z).expect("direct");
            let plane = material_of(&owner, &spec, x, y, z).expect("plane");
            assert_eq!(direct, plane, "the direct query disagreed at ({x}, {y}, {z})");
        }
    }

    #[test]
    fn the_owner_handle_answers_exactly_as_its_source() {
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let coord = VoxelPos::new(4, 16, 9);
        let direct = FieldQuery::sample(&source, FieldId::Elevation, coord).expect("elevation");
        let owned = source
            .for_owner(ChunkPos::new(0, 0, 0))
            .sample(FieldId::Elevation, coord)
            .expect("elevation");
        assert_eq!(direct, owned);
    }

    #[test]
    fn a_field_with_no_producer_refuses_by_name() {
        // Four fields in the nineteen-name roster are named by **no stage's declaration**
        // (`process::owning_stage` is `None` for them); the refusal names the field rather
        // than answering a zero (never a screen tint). `Pressure` and `Wind` used to be on
        // this list and are now produced by the atmosphere and weather bodies.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        for owed in [
            FieldId::Storms,
            FieldId::Radiation,
            FieldId::Humidity,
            FieldId::SoilFertility,
        ] {
            let refused = FieldQuery::sample(&source, owed, VoxelPos::new(0, 0, 0))
                .expect_err("an owed field was answered");
            assert_eq!(refused.kind(), RefusalKind::Unimplemented);
            assert!(
                refused.detail().contains(owed.as_str()),
                "{}",
                refused.detail()
            );
        }
    }

    #[test]
    fn an_undeclared_field_query_is_refused_by_name() {
        // `SoilFertility` is in the nineteen-name roster and in no declaration here.
        // `CarryingCapacity` used to stand in for it and is now declared and produced.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let refused = FieldQuery::sample(&source, FieldId::SoilFertility, VoxelPos::new(0, 0, 0))
            .expect_err("an undeclared field was answered");
        assert_eq!(refused.kind(), RefusalKind::Unimplemented);
        assert!(
            refused.detail().contains("soil-fertility"),
            "{}",
            refused.detail()
        );
    }

    #[test]
    fn the_fixed_encoding_round_trips_through_f64() {
        for value in [16.0, 16.0004, -0.5, 12_345.6789, 0.000_976_562_5, 0.0] {
            let fixed = fixed_from_f64(value).expect("representable");
            assert_eq!(
                f64_of_fixed(fixed),
                value,
                "the encoding was lossy at {value}"
            );
        }
        assert_eq!(
            fixed_from_f64(f64::NAN).expect_err("NaN").kind(),
            RefusalKind::NonFiniteValue
        );
    }

    #[test]
    fn a_lithology_code_round_trips_and_an_unknown_code_refuses() {
        for lithology in [
            Lithology::Regolith,
            Lithology::Sedimentary,
            Lithology::Metamorphic,
            Lithology::Igneous,
            Lithology::Bedrock,
        ] {
            assert_eq!(
                lithology_of(FieldValue::Integer(lithology_code(lithology))).expect("code"),
                lithology
            );
        }
        let refused = lithology_of(FieldValue::Integer(9)).expect_err("an unknown code");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        let refused = lithology_of(FieldValue::Fixed(
            FixedValue::new(1, 1).expect("a positive scale"),
        ))
        .expect_err("a measured value where a class belongs");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
    }

    #[test]
    fn the_plane_is_deterministic_across_two_builds() {
        // F1/F12 one layer down: the same world spec gives the same samples, and the
        // identity does not depend on when it was built.
        let spec = world();
        let a = FieldSource::new(&spec).expect("the plane builds");
        let b = FieldSource::new(&spec).expect("the plane builds");
        for coord in [VoxelPos::new(0, 16, 0), VoxelPos::new(31, 33, 31)] {
            for field in [FieldId::Elevation, FieldId::Soil, FieldId::Geology] {
                assert_eq!(
                    FieldQuery::sample(&a, field, coord).expect("sample"),
                    FieldQuery::sample(&b, field, coord).expect("sample"),
                    "{field:?} moved between two builds at {coord:?}"
                );
            }
        }
    }

    #[test]
    fn a_sample_built_by_the_producer_lands_on_the_declared_lattice() {
        // The producer's declarations are step 1, so every integer coordinate is on the
        // lattice; this pins that a DraftFieldSampler accepts the samples the producer
        // makes, which is the check a coarser lattice would fail by name.
        let spec = world();
        let source = FieldSource::new(&spec).expect("the plane builds");
        let coord = VoxelPos::new(3, 17, 5);
        let samples: Vec<DraftFieldSample> = [
            FieldId::Elevation,
            FieldId::Slope,
            FieldId::Moisture,
            FieldId::Soil,
            FieldId::Geology,
        ]
        .into_iter()
        .map(|field| DraftFieldSample {
            field,
            coord,
            value: FieldQuery::sample(&source, field, coord).expect("sample"),
        })
        .collect();
        DraftFieldSampler::new(declared_fields().expect("declared"), samples)
            .expect("the producer's samples are on the declared lattice");
    }
}
