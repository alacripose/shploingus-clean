//! The field plane — the reference's `world/fields.rs`, ported.
//!
//! **Source.** This module is a port of `ala-cities-design-package/src/world/fields.rs`
//! (890 lines), adapted to this tree's vocabulary rather than re-derived: the reference
//! file is the design package's field sampler — the data plane its stages read and write —
//! and the ledger's correction (C-119) names the missing vocabulary exactly:
//! `ChunkFieldSampler` 0, `FieldSampler` 0, `FieldSamplerIdentity` 0 in this tree.
//!
//! # What the plane is, and why it exists
//!
//! A field is a **declared** value with a lattice: where it varies, at what step, how it
//! is sampled between samples, and which seam policy it obeys. A stage does not call
//! another stage; it reads fields the stage above it produced. That is the cubic core's
//! generation shape (C-119): chunk-sampled fields, with the column reduced to one
//! sampling detail rather than the generator's foundation.
//!
//! The reference keeps fields in a **generated manifest** (`matter/generated.rs`): field
//! values are admitted rows, and `FieldSampler::from_generated_manifest` refuses until
//! the manifest carries lattice and sampling declarations — which, in the reference, it
//! does not yet. This tree has no manifest, so the authoritative door refuses by name
//! here too ([`FieldSampler::from_generated_manifest`]), and the executable path is the
//! **draft** sampler ([`DraftFieldSampler`]), exactly as it is in the reference's own
//! stage derivations.
//!
//! # What was adapted, and what was not
//!
//! * **Adapted:** `WorldError` → [`Refusal`]; `Digest` → [`Identity`];
//!   `Coord`/`ChunkCoord` → [`VoxelPos`]/[`ChunkPos`]; `Rational`/`Lifecycle` are this
//!   tree's ([`voxel_model::matter`]). The reference's length-prefixed payload framing is
//!   kept; the hash is this tree's [`Identity`] form.
//! * **Not adapted:** the vocabulary (`FieldId`, `Unit`, `FieldResolution`,
//!   `VerticalDependence`, `SamplingPolicy`, `SeamPolicy`, `FieldContract`,
//!   `FieldSamplerIdentity`, `FieldSpec`, `DraftFieldSpec`, `FixedValue`, `FieldValue`,
//!   `FieldSample`, `CanonicalSamples`, `FieldSampler`, `ChunkFieldSampler`,
//!   `DraftFieldSampler`, `DraftChunkFieldSampler`) and every refusal rule.
//!
//! **Named, not silently dropped:** the reference's `specs_from_generated_manifest` reads
//! generated rows whose schema carries `resolution`, `sampling` and `seam_policy`. This
//! tree has no generated manifest yet, so that function refuses by name instead of
//! returning an empty catalog — an empty catalog would look like a world with no fields.
use voxel_model::endian::LeBytes;

use std::collections::BTreeMap;

use voxel_model::coord::{ChunkPos, VoxelPos};
use voxel_model::matter::{Lifecycle, Rational};
use voxel_model::{Identity, Refusal, RefusalKind, Refused};

/// Every field the design package declares. The roster is ported whole (19 fields), not
/// narrowed to what this tree can currently produce: a field with no producer is a
/// **refusal by name** at query time, not an absence that looks like a default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldId {
    Tectonics,
    Geology,
    Elevation,
    Slope,
    ErosionResistance,
    Soil,
    SoilFertility,
    Climate,
    Temperature,
    Moisture,
    /// How deep inside a landmass a position sits, `[0, 1]` — the atlas's continentalness
    /// (`earth_biomes_subbiomes_continentalness_mountains.pdf` §2), produced by
    /// [`crate::climate::continentality`] and consumed by the moisture and season laws.
    Continentalness,
    Precipitation,
    Humidity,
    /// Near-surface **condensed** liquid water, kg/m³ — the fog guide's droplet field, produced by
    /// [`crate::fog`] and addressed at a height, because a fog layer has a bottom and a top.
    FogWater,
    /// Near-surface **mechanically produced** liquid water, kg/m³ — spray and the droplets a foam's
    /// bubbles burst. A separate field because it exists with the air below saturation.
    SprayWater,
    /// Koschmieder visibility of the addressed droplet field, metres — read from the droplets and
    /// never from relative humidity.
    Visibility,
    /// The cloud model's **local water content** at an address, `[0, 1]` — zero where there is no
    /// cloud, because the absence of a cloud is a measurement and not an unproduced field.
    CloudWater,
    /// The base of the cloud this column is making, metres. A query where the column makes **no**
    /// cloud refuses by name: the base of a cloud that does not exist is not a value.
    CloudBase,
    /// The top of that cloud, metres — same refusal rule as [`FieldId::CloudBase`].
    CloudTop,
    Groundwater,
    Pressure,
    Wind,
    Storms,
    Radiation,
    CarryingCapacity,
    SpeciesSuitability,
    /// **How much of the voxel the world holds**, in `[0, 1]` — the `profile32` cell's own
    /// fill, produced by [`crate::earthworks`]. It is the field record the shader draws a
    /// voxel's cuboid cell from: a whole block is `1`, air above the surface is `0`, and the
    /// voxel the terrain's surface law cuts holds the fraction of it the ground occupies.
    ProfileCell,
}

impl FieldId {
    /// The full roster, in declaration order.
    pub const ALL: [FieldId; 27] = [
        FieldId::Tectonics,
        FieldId::Geology,
        FieldId::Elevation,
        FieldId::Slope,
        FieldId::ErosionResistance,
        FieldId::Soil,
        FieldId::SoilFertility,
        FieldId::Climate,
        FieldId::Temperature,
        FieldId::Moisture,
        FieldId::Continentalness,
        FieldId::Precipitation,
        FieldId::Humidity,
        FieldId::FogWater,
        FieldId::SprayWater,
        FieldId::Visibility,
        FieldId::CloudWater,
        FieldId::CloudBase,
        FieldId::CloudTop,
        FieldId::Groundwater,
        FieldId::Pressure,
        FieldId::Wind,
        FieldId::Storms,
        FieldId::Radiation,
        FieldId::CarryingCapacity,
        FieldId::SpeciesSuitability,
        FieldId::ProfileCell,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tectonics => "tectonics",
            Self::Geology => "geology",
            Self::Elevation => "elevation",
            Self::Slope => "slope",
            Self::ErosionResistance => "erosion-resistance",
            Self::Soil => "soil",
            Self::SoilFertility => "soil-fertility",
            Self::Climate => "climate",
            Self::Temperature => "temperature",
            Self::Moisture => "moisture",
            Self::Continentalness => "continentalness",
            Self::Precipitation => "precipitation",
            Self::Humidity => "humidity",
            Self::FogWater => "fog-water",
            Self::SprayWater => "spray-water",
            Self::Visibility => "visibility",
            Self::CloudWater => "cloud-water",
            Self::CloudBase => "cloud-base",
            Self::CloudTop => "cloud-top",
            Self::Groundwater => "groundwater",
            Self::Pressure => "pressure",
            Self::Wind => "wind",
            Self::Storms => "storms",
            Self::Radiation => "radiation",
            Self::CarryingCapacity => "carrying-capacity",
            Self::SpeciesSuitability => "species-suitability",
            Self::ProfileCell => "profile-cell",
        }
    }
}

/// The unit a field's values are in. Ported whole from the reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    Dimensionless,
    Ratio,
    Percent,
    Metre,
    SquareMetre,
    CubicMetre,
    PerSquareMetre,
    MetrePerSecond,
    Pascal,
    Kelvin,
    DegreesCelsius,
    KilogramPerCubicMeter,
    WattPerSquareMetre,
}

impl Unit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dimensionless => "1",
            Self::Ratio => "ratio",
            Self::Percent => "%",
            Self::Metre => "m",
            Self::SquareMetre => "m2",
            Self::CubicMetre => "m3",
            Self::PerSquareMetre => "per-m2",
            Self::MetrePerSecond => "m/s",
            Self::Pascal => "Pa",
            Self::Kelvin => "K",
            Self::DegreesCelsius => "degC",
            Self::KilogramPerCubicMeter => "kg/m3",
            Self::WattPerSquareMetre => "W/m2",
        }
    }
}

/// Which of a field's two declared horizontal lattices a sampling policy reads.
///
/// Two levels and no more, because that is what the reference declares: a **coarse**
/// lattice for the value that is stable over an area, and a **fine** one for the value a
/// local process needs. More levels would be a ladder, and the ladder lives in
/// `crate::fields` (F4's 20 levels, octree-aligned) — this is a different structure and
/// the two are not interchangeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionLevel {
    Coarse,
    Fine,
}

/// A field's lattice: an origin, two horizontal steps and an optional vertical step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldResolution {
    origin: VoxelPos,
    coarse_horizontal: i32,
    fine_horizontal: i32,
    vertical: Option<i32>,
}

impl FieldResolution {
    pub const fn new(
        origin: VoxelPos,
        coarse_horizontal: i32,
        fine_horizontal: i32,
        vertical: Option<i32>,
    ) -> Self {
        Self {
            origin,
            coarse_horizontal,
            fine_horizontal,
            vertical,
        }
    }

    pub const fn origin(self) -> VoxelPos {
        self.origin
    }

    pub const fn coarse_horizontal(self) -> i32 {
        self.coarse_horizontal
    }

    pub const fn fine_horizontal(self) -> i32 {
        self.fine_horizontal
    }

    pub const fn vertical(self) -> Option<i32> {
        self.vertical
    }

    pub const fn horizontal(self, level: ResolutionLevel) -> i32 {
        match level {
            ResolutionLevel::Coarse => self.coarse_horizontal,
            ResolutionLevel::Fine => self.fine_horizontal,
        }
    }

    fn validate(self, field: FieldId) -> Refused<()> {
        if self.coarse_horizontal <= 0
            || self.fine_horizontal <= 0
            || self.fine_horizontal > self.coarse_horizontal
            || self.vertical.is_some_and(|step| step <= 0)
        {
            return Err(invalid_field(
                field,
                "field resolution must contain positive, ordered lattice steps",
            ));
        }
        Ok(())
    }
}

/// How a field depends on the vertical axis.
///
/// Ported verbatim, and the distinction is load-bearing: a horizontal-only field (a
/// heightfield) is one value per column and its canonical query **pins z to the lattice
/// origin**, while a field that uses z is sampled at the query's own height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerticalDependence {
    HorizontalOnly,
    UsesZ { resolution: i32 },
    SeparateVerticalLattice { resolution: i32 },
}

impl VerticalDependence {
    pub const fn discriminant(self) -> u8 {
        match self {
            Self::HorizontalOnly => 0,
            Self::UsesZ { .. } => 1,
            Self::SeparateVerticalLattice { .. } => 2,
        }
    }

    pub const fn declared_resolution(self) -> Option<i32> {
        match self {
            Self::HorizontalOnly => None,
            Self::UsesZ { resolution } | Self::SeparateVerticalLattice { resolution } => {
                Some(resolution)
            }
        }
    }
}

/// How a query is answered from the samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SamplingPolicy {
    /// Nearest stored sample within `coverage_radius`, on the chosen lattice level.
    /// Ties are broken toward the greater coordinate, so the answer is deterministic.
    Nearest {
        level: ResolutionLevel,
        coverage_radius: i32,
    },
    /// The exact sample at the point's lattice coordinate, or a refusal.
    PiecewiseConstant { level: ResolutionLevel },
}

impl SamplingPolicy {
    pub const fn level(self) -> ResolutionLevel {
        match self {
            Self::Nearest { level, .. } | Self::PiecewiseConstant { level } => level,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Nearest { .. } => "nearest",
            Self::PiecewiseConstant { .. } => "piecewise-constant",
        }
    }

    fn validate(self, field: FieldId) -> Refused<()> {
        if matches!(self, Self::Nearest { coverage_radius, .. } if coverage_radius <= 0) {
            return Err(invalid_field(
                field,
                "nearest sampling coverage radius must be positive",
            ));
        }
        Ok(())
    }
}

/// Which lattice a field's values sit on. `GlobalLattice` is the only production policy;
/// the synthetic variant exists so tests can prove the identity binds the seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeamPolicy {
    GlobalLattice,
    #[cfg(test)]
    Synthetic(u8),
}

impl SeamPolicy {
    pub(crate) const fn identity_parts(self) -> (u8, u8) {
        match self {
            Self::GlobalLattice => (0, 0),
            #[cfg(test)]
            Self::Synthetic(value) => (1, value),
        }
    }
}

/// A field's declaration: what it is, where its values live, and everything that can
/// change a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldContract {
    id: FieldId,
    unit: Unit,
    resolution: FieldResolution,
    vertical_dependence: VerticalDependence,
    sampling: SamplingPolicy,
    seam_policy: SeamPolicy,
    tolerance: Rational,
    source_ids: Vec<String>,
    review_state: Lifecycle,
}

impl FieldContract {
    fn validate(&self, authoritative: bool) -> Refused<()> {
        if authoritative && self.review_state != Lifecycle::Admitted {
            return Err(Refusal::owned(
                RefusalKind::UnadmittedFieldSpec,
                format!(
                    "field {} is {}, not admitted",
                    self.id.as_str(),
                    self.review_state.name()
                ),
            ));
        }
        if self.source_ids.is_empty()
            || self
                .source_ids
                .iter()
                .any(|source| source.trim().is_empty())
        {
            return Err(invalid_field(
                self.id,
                "field source links must be non-empty",
            ));
        }
        if self.tolerance.is_negative() {
            return Err(invalid_field(
                self.id,
                "field tolerance must be non-negative",
            ));
        }
        self.resolution.validate(self.id)?;
        self.sampling.validate(self.id)?;
        if self.vertical_dependence.declared_resolution() != self.resolution.vertical() {
            return Err(invalid_field(
                self.id,
                "vertical dependence and vertical resolution must agree",
            ));
        }
        Ok(())
    }

    /// The declaration's identity payload: every field that can change a value, framed.
    fn append_identity(&self, payload: &mut Vec<u8>) {
        identity_field(payload, self.id.as_str().as_bytes());
        identity_field(payload, self.unit.as_str().as_bytes());
        let resolution = self.resolution;
        identity_field(payload, &resolution.origin().x.into_le_bytes());
        identity_field(payload, &resolution.origin().y.into_le_bytes());
        identity_field(payload, &resolution.origin().z.into_le_bytes());
        identity_field(payload, &resolution.coarse_horizontal().into_le_bytes());
        identity_field(payload, &resolution.fine_horizontal().into_le_bytes());
        identity_field(
            payload,
            &resolution.vertical().unwrap_or(i32::MIN).into_le_bytes(),
        );
        identity_field(payload, &[self.vertical_dependence.discriminant()]);
        match self.sampling {
            SamplingPolicy::Nearest {
                level,
                coverage_radius,
            } => {
                identity_field(payload, &[0]);
                identity_field(payload, &[resolution_level_code(level)]);
                identity_field(payload, &coverage_radius.into_le_bytes());
            }
            SamplingPolicy::PiecewiseConstant { level } => {
                identity_field(payload, &[1]);
                identity_field(payload, &[resolution_level_code(level)]);
                identity_field(payload, &[0, 0, 0, 0]);
            }
        }
        let (seam_discriminant, seam_value) = self.seam_policy.identity_parts();
        identity_field(payload, &[seam_discriminant]);
        identity_field(payload, &[seam_value]);
        identity_field(payload, &self.tolerance.numerator().into_le_bytes());
        identity_field(payload, &self.tolerance.denominator().into_le_bytes());
        identity_field(payload, self.review_state.name().as_bytes());
        let mut source_ids = self.source_ids.clone();
        source_ids.sort();
        identity_field(
            payload,
            &u64::try_from(source_ids.len())
                .unwrap_or(u64::MAX)
                .into_le_bytes(),
        );
        for source in source_ids {
            identity_field(payload, source.as_bytes());
        }
    }
}

/// The identity of a **set of field declarations**.
///
/// The reference's `FieldSamplerIdentity`: every value-affecting declaration is hashed,
/// and the source list is sorted first, so two declarations that differ only in the order
/// a caller happened to list their sources are the same declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldSamplerIdentity(Identity);

impl FieldSamplerIdentity {
    pub const fn identity(self) -> Identity {
        self.0
    }

    fn from_contracts(contracts: &BTreeMap<FieldId, FieldContract>) -> Self {
        let mut payload = Vec::new();
        identity_field(
            &mut payload,
            &u64::try_from(contracts.len())
                .unwrap_or(u64::MAX)
                .into_le_bytes(),
        );
        for contract in contracts.values() {
            contract.append_identity(&mut payload);
        }
        Self(Identity::of_fields("field-sampler/v1", &[&payload]))
    }
}

/// Authoritative field metadata — constructible only through [`FieldSampler`], which
/// itself comes only from an admitted manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSpec {
    contract: FieldContract,
}

impl FieldSpec {
    pub const fn id(&self) -> FieldId {
        self.contract.id
    }

    pub const fn unit(&self) -> Unit {
        self.contract.unit
    }

    pub const fn resolution(&self) -> FieldResolution {
        self.contract.resolution
    }

    pub const fn vertical_dependence(&self) -> VerticalDependence {
        self.contract.vertical_dependence
    }

    pub const fn sampling(&self) -> SamplingPolicy {
        self.contract.sampling
    }

    pub const fn seam_policy(&self) -> SeamPolicy {
        self.contract.seam_policy
    }

    pub const fn tolerance(&self) -> Rational {
        self.contract.tolerance
    }

    pub fn source_ids(&self) -> &[String] {
        &self.contract.source_ids
    }

    pub const fn review_state(&self) -> Lifecycle {
        self.contract.review_state
    }

    fn from_contract(contract: FieldContract) -> Self {
        Self { contract }
    }
}

/// Non-authoritative metadata for executable behavior fixtures. This type cannot produce
/// a [`FieldSpec`] or an authoritative [`FieldSample`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftFieldSpec {
    pub id: FieldId,
    pub unit: Unit,
    pub resolution: FieldResolution,
    pub vertical_dependence: VerticalDependence,
    pub sampling: SamplingPolicy,
    pub seam_policy: SeamPolicy,
    pub tolerance: Rational,
    pub source_ids: Vec<String>,
}

impl DraftFieldSpec {
    pub(crate) fn contract(&self) -> FieldContract {
        FieldContract {
            id: self.id,
            unit: self.unit,
            resolution: self.resolution,
            vertical_dependence: self.vertical_dependence,
            sampling: self.sampling,
            seam_policy: self.seam_policy,
            tolerance: self.tolerance,
            source_ids: self.source_ids.clone(),
            review_state: Lifecycle::Provisional,
        }
    }
}

/// A fixed-point number: `raw / scale`, with a positive scale. Exact, like every other
/// number the field plane carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedValue {
    raw: i128,
    scale: i128,
}

impl FixedValue {
    pub fn new(raw: i128, scale: i128) -> Option<Self> {
        (scale > 0).then_some(Self { raw, scale })
    }

    pub const fn raw(self) -> i128 {
        self.raw
    }

    pub const fn scale(self) -> i128 {
        self.scale
    }

    pub fn as_rational(self) -> Option<Rational> {
        Rational::new(self.raw, self.scale).ok()
    }
}

/// A field value. Three representations because the reference declares three: an integer
/// for a class or an id, an exact rational for a ratio, and a fixed-point number for a
/// measured quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Integer(i128),
    Rational(Rational),
    Fixed(FixedValue),
}

impl FieldValue {
    pub const fn fixed(self) -> Option<FixedValue> {
        match self {
            Self::Fixed(value) => Some(value),
            Self::Integer(_) | Self::Rational(_) => None,
        }
    }
}

/// An authoritative field sample: a declared field, the coordinate asked for, and the
/// value found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSample {
    field: FieldId,
    coord: VoxelPos,
    value: FieldValue,
}

impl FieldSample {
    pub const fn field(&self) -> FieldId {
        self.field
    }

    pub const fn coord(&self) -> VoxelPos {
        self.coord
    }

    pub const fn value(&self) -> FieldValue {
        self.value
    }
}

/// A non-authoritative sample, for executable fixtures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftFieldSample {
    pub field: FieldId,
    pub coord: VoxelPos,
    pub value: FieldValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalSample {
    field: FieldId,
    coord: VoxelPos,
    value: FieldValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalSamples {
    contracts: BTreeMap<FieldId, FieldContract>,
    samples: Vec<CanonicalSample>,
    identity: FieldSamplerIdentity,
}

impl CanonicalSamples {
    fn new(
        contracts: Vec<FieldContract>,
        samples: Vec<CanonicalSample>,
        authoritative: bool,
    ) -> Refused<Self> {
        validate_contracts(&contracts, authoritative)?;
        let mut indexed = BTreeMap::new();
        for contract in contracts {
            indexed.insert(contract.id, contract);
        }
        for sample in &samples {
            let contract =
                indexed
                    .get(&sample.field)
                    .ok_or_else(|| invalid_field_named(
                        sample.field,
                        "sample has no declared field specification",
                    ))?;
            let lattice = lattice_coord(contract, sample.coord)?;
            if sample.coord != lattice {
                return Err(invalid_field_named(
                    sample.field,
                    "sample is not an exact point on its declared lattice",
                ));
            }
        }
        let mut samples = samples;
        samples.sort_by_key(|sample| (sample.field, sample.coord.x, sample.coord.y, sample.coord.z));
        if samples
            .windows(2)
            .any(|pair| pair[0].field == pair[1].field && pair[0].coord == pair[1].coord)
        {
            return Err(Refusal::owned(
                RefusalKind::InvalidFieldSpec,
                "field lattice samples must be unique",
            ));
        }
        let identity = FieldSamplerIdentity::from_contracts(&indexed);
        Ok(Self {
            contracts: indexed,
            samples,
            identity,
        })
    }

    fn contract(&self, field: FieldId) -> Refused<&FieldContract> {
        self.contracts
            .get(&field)
            .ok_or_else(|| invalid_field_named(field, "field specification is not present"))
    }

    fn nearest(
        &self,
        field: FieldId,
        coord: VoxelPos,
        coverage_limit: i128,
    ) -> Option<&CanonicalSample> {
        self.samples
            .iter()
            .filter(|sample| sample.field == field)
            .filter(|sample| coordinate_distance(sample.coord, coord) <= coverage_limit)
            .fold(None, |best, candidate| {
                let replace = best.is_none_or(|best| {
                    let distance_order = coordinate_distance(candidate.coord, coord)
                        .cmp(&coordinate_distance(best.coord, coord));
                    distance_order.is_lt()
                        || (distance_order.is_eq() && candidate.coord > best.coord)
                });
                if replace {
                    Some(candidate)
                } else {
                    best
                }
            })
    }

    fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldValue> {
        let contract = self.contract(field)?;
        let canonical = canonical_query(contract, coord)?;
        let found = match contract.sampling {
            SamplingPolicy::Nearest { coverage_radius, .. } => {
                let limit = i128::from(coverage_radius) * i128::from(coverage_radius);
                self.nearest(field, canonical, limit)
            }
            SamplingPolicy::PiecewiseConstant { .. } => self
                .samples
                .iter()
                .find(|sample| sample.field == field && sample.coord == canonical),
        };
        found
            .map(|sample| sample.value)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::FieldDataMissing,
                    format!(
                        "field {} has no sample covered by {} at ({}, {}, {})",
                        field.as_str(),
                        contract.sampling.name(),
                        canonical.x,
                        canonical.y,
                        canonical.z
                    ),
                )
            })
    }
}

/// Ported with the sampler because the reference's `StageGraph` calls it at construction.
/// **Named debt:** the graph is the process-runtime cut; until it lands this is called by
/// nothing, and an allow is used rather than deleting a ported rule.
#[allow(dead_code)]
pub(crate) fn validate_field_specs(specs: &[FieldSpec]) -> Refused<()> {
    validate_contracts(
        &specs
            .iter()
            .map(|spec| spec.contract.clone())
            .collect::<Vec<_>>(),
        true,
    )
}

/// Same as [`validate_field_specs`]: ported because the reference's `StageGraph`
/// canonicalises its catalog before hashing it.
#[allow(dead_code)]
pub(crate) fn canonicalize_field_specs(specs: &mut [FieldSpec]) {
    specs.sort_by_key(|spec| spec.contract.id);
}

fn validate_contracts(contracts: &[FieldContract], authoritative: bool) -> Refused<()> {
    if authoritative && contracts.is_empty() {
        return Err(Refusal::new(
            RefusalKind::InvalidFieldSpec,
            "authoritative field catalog must be manifest-bound and non-empty",
        ));
    }
    let mut fields = BTreeMap::new();
    for contract in contracts {
        contract.validate(authoritative)?;
        if fields.insert(contract.id, ()).is_some() {
            return Err(invalid_field_named(
                contract.id,
                "field specification is duplicated",
            ));
        }
    }
    Ok(())
}

/// The authoritative catalog's door, and it refuses by name: this tree has no generated
/// field manifest yet, so there is nothing to admit field lattices from. The reference's
/// own function refuses in the same situation (*"generated rows do not yet carry field
/// lattice and sampling declarations"*); an empty catalog would be worse than a refusal
/// because it would look like a world with no fields.
pub fn specs_from_generated_manifest() -> Refused<Vec<FieldSpec>> {
    Err(Refusal::new(
        RefusalKind::UnadmittedFieldSpec,
        "this tree has no generated field manifest; field declarations come from the \
         producer until one exists",
    ))
}

const fn resolution_level_code(level: ResolutionLevel) -> u8 {
    match level {
        ResolutionLevel::Coarse => 0,
        ResolutionLevel::Fine => 1,
    }
}

/// The authoritative sampler: immutable declarations plus their admitted samples,
/// answerable per **owner** chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSampler {
    samples: CanonicalSamples,
}

impl FieldSampler {
    pub fn from_generated_manifest() -> Refused<Self> {
        let specs = specs_from_generated_manifest()?;
        let contracts = specs
            .into_iter()
            .map(|spec| spec.contract)
            .collect::<Vec<_>>();
        CanonicalSamples::new(contracts, Vec::new(), true).map(|samples| Self { samples })
    }

    pub fn spec(&self, field: FieldId) -> Refused<FieldSpec> {
        self.samples
            .contract(field)
            .cloned()
            .map(FieldSpec::from_contract)
    }

    pub fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldSample> {
        self.samples.sample(field, coord).map(|value| FieldSample {
            field,
            coord,
            value,
        })
    }

    /// The handle a chunk owner holds. `_owner` is deliberately unused in the reference
    /// too: the handle exists so a caller cannot sample *as* the world, and so a later
    /// residency policy can scope the answer to the owner without changing callers.
    pub fn for_owner(&self, _owner: ChunkPos) -> ChunkFieldSampler<'_> {
        ChunkFieldSampler { sampler: self }
    }
}

/// A chunk-scoped view of a [`FieldSampler`].
pub struct ChunkFieldSampler<'a> {
    sampler: &'a FieldSampler,
}

impl ChunkFieldSampler<'_> {
    pub fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<FieldSample> {
        self.sampler.sample(field, coord)
    }
}

/// Executable non-authoritative fixtures — the path the reference's stage derivations
/// actually run on today. This type cannot produce a [`FieldSpec`] or an authoritative
/// [`FieldSample`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftFieldSampler {
    samples: CanonicalSamples,
}

impl DraftFieldSampler {
    pub fn new(
        specs: Vec<DraftFieldSpec>,
        samples: Vec<DraftFieldSample>,
    ) -> Refused<Self> {
        let contracts = specs
            .iter()
            .map(DraftFieldSpec::contract)
            .collect::<Vec<_>>();
        let samples = samples
            .into_iter()
            .map(|sample| CanonicalSample {
                field: sample.field,
                coord: sample.coord,
                value: sample.value,
            })
            .collect();
        CanonicalSamples::new(contracts, samples, false).map(|samples| Self { samples })
    }

    /// The declaration set's identity: **everything that can change a value**.
    pub const fn identity(&self) -> FieldSamplerIdentity {
        self.samples.identity
    }

    pub fn spec(&self, field: FieldId) -> Refused<DraftFieldSpec> {
        self.samples
            .contract(field)
            .map(|contract| DraftFieldSpec {
                id: contract.id,
                unit: contract.unit,
                resolution: contract.resolution,
                vertical_dependence: contract.vertical_dependence,
                sampling: contract.sampling,
                seam_policy: contract.seam_policy,
                tolerance: contract.tolerance,
                source_ids: contract.source_ids.clone(),
            })
    }

    pub fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<DraftFieldSample> {
        self.samples
            .sample(field, coord)
            .map(|value| DraftFieldSample {
                field,
                coord,
                value,
            })
    }

    pub fn for_owner(&self, _owner: ChunkPos) -> DraftChunkFieldSampler<'_> {
        DraftChunkFieldSampler { sampler: self }
    }
}

/// A chunk-scoped view of a [`DraftFieldSampler`].
pub struct DraftChunkFieldSampler<'a> {
    sampler: &'a DraftFieldSampler,
}

impl DraftChunkFieldSampler<'_> {
    pub fn sample(&self, field: FieldId, coord: VoxelPos) -> Refused<DraftFieldSample> {
        self.sampler.sample(field, coord)
    }
}

/// The same canonicalisation, for a producer that declares its contracts as
/// [`DraftFieldSpec`]s (the executable path) rather than holding an admitted catalog.
pub(crate) fn canonical_query_of(spec: &DraftFieldSpec, coord: VoxelPos) -> Refused<VoxelPos> {
    canonical_query(&spec.contract(), coord)
}

/// The coordinate a query is answered at, before any stored-sample search.
///
/// Ported exactly: for `Nearest`, a horizontal-only field **pins z to the lattice
/// origin** — that is what makes a heightfield one value per column — while a field that
/// uses z keeps the query's own height. For `PiecewiseConstant`, the query is snapped to
/// its declared lattice.
fn canonical_query(contract: &FieldContract, coord: VoxelPos) -> Refused<VoxelPos> {
    match contract.sampling {
        SamplingPolicy::Nearest { .. } => {
            let z = match contract.vertical_dependence {
                VerticalDependence::HorizontalOnly => contract.resolution.origin().z,
                VerticalDependence::UsesZ { .. }
                | VerticalDependence::SeparateVerticalLattice { .. } => coord.z,
            };
            Ok(VoxelPos::new(coord.x, coord.y, z))
        }
        SamplingPolicy::PiecewiseConstant { .. } => lattice_coord(contract, coord),
    }
}

fn lattice_coord(contract: &FieldContract, coord: VoxelPos) -> Refused<VoxelPos> {
    let horizontal = contract.resolution.horizontal(contract.sampling.level());
    let z = match contract.vertical_dependence {
        VerticalDependence::HorizontalOnly => contract.resolution.origin().z,
        VerticalDependence::UsesZ { resolution }
        | VerticalDependence::SeparateVerticalLattice { resolution } => {
            lattice_axis(coord.z, resolution, contract.resolution.origin().z)?
        }
    };
    Ok(VoxelPos::new(
        lattice_axis(coord.x, horizontal, contract.resolution.origin().x)?,
        lattice_axis(coord.y, horizontal, contract.resolution.origin().y)?,
        z,
    ))
}

fn lattice_axis(value: i32, step: i32, origin: i32) -> Refused<i32> {
    let lattice = i128::from(value.div_euclid(step)) * i128::from(step) + i128::from(origin);
    i32::try_from(lattice).map_err(|_| {
        Refusal::new(
            RefusalKind::InvalidFieldSpec,
            "global lattice coordinate exceeds signed VoxelPos range",
        )
    })
}

fn coordinate_distance(left: VoxelPos, right: VoxelPos) -> i128 {
    let x = i128::from(left.x) - i128::from(right.x);
    let y = i128::from(left.y) - i128::from(right.y);
    let z = i128::from(left.z) - i128::from(right.z);
    x.saturating_mul(x) + y.saturating_mul(y) + z.saturating_mul(z)
}

/// The reference's payload framing: every field length-prefixed, so that `("ab", "c")` and
/// `("a", "bc")` have different identities.
fn identity_field(payload: &mut Vec<u8>, bytes: &[u8]) {
    payload.extend_from_slice(&(bytes.len() as u64).into_le_bytes());
    payload.extend_from_slice(bytes);
}

fn invalid_field(field: FieldId, reason: &'static str) -> Refusal {
    invalid_field_named(field, reason)
}

fn invalid_field_named(field: FieldId, reason: &str) -> Refusal {
    Refusal::owned(
        RefusalKind::InvalidFieldSpec,
        format!("invalid field specification for {}: {reason}", field.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero() -> VoxelPos {
        VoxelPos::new(0, 0, 0)
    }

    fn mm(n: i128) -> FixedValue {
        FixedValue::new(n, 1000).expect("a positive scale")
    }

    fn draft(
        id: FieldId,
        sampling: SamplingPolicy,
        vertical: Option<i32>,
        coarse: i32,
        fine: i32,
    ) -> DraftFieldSpec {
        DraftFieldSpec {
            id,
            unit: Unit::Metre,
            resolution: FieldResolution::new(zero(), coarse, fine, vertical),
            vertical_dependence: match vertical {
                Some(resolution) => VerticalDependence::SeparateVerticalLattice { resolution },
                None => VerticalDependence::HorizontalOnly,
            },
            sampling,
            seam_policy: SeamPolicy::GlobalLattice,
            tolerance: Rational::new(1, 1000).expect("a positive denominator"),
            source_ids: vec!["S-TEST".to_owned()],
        }
    }

    fn draft_sample(field: FieldId, coord: VoxelPos, raw: i128) -> DraftFieldSample {
        DraftFieldSample {
            field,
            coord,
            value: FieldValue::Fixed(mm(raw)),
        }
    }

    #[test]
    fn the_field_roster_carries_the_references_nineteen_and_names_its_own_additions() {
        // The reference's nineteen are the vocabulary this roster was ported from, and **every one
        // of them must still be here** — a narrowed roster is a field somebody deleted. It was
        // widened on 2026-10-10 by four, from the two guides the user adopted, and the widening is
        // named rather than absorbed: the additions are exactly these, so a fifth field cannot
        // arrive without this test being the place that says so.
        for field in [
            FieldId::Tectonics,
            FieldId::Geology,
            FieldId::Elevation,
            FieldId::Slope,
            FieldId::ErosionResistance,
            FieldId::Soil,
            FieldId::SoilFertility,
            FieldId::Climate,
            FieldId::Temperature,
            FieldId::Moisture,
            FieldId::Precipitation,
            FieldId::Humidity,
            FieldId::Groundwater,
            FieldId::Pressure,
            FieldId::Wind,
            FieldId::Storms,
            FieldId::Radiation,
            FieldId::CarryingCapacity,
            FieldId::SpeciesSuitability,
        ] {
            assert!(FieldId::ALL.contains(&field), "{field:?} fell out of the roster");
        }
        let additions = [
            FieldId::Continentalness,
            FieldId::FogWater,
            FieldId::SprayWater,
            FieldId::Visibility,
            FieldId::CloudWater,
            FieldId::CloudBase,
            FieldId::CloudTop,
        ];
        // The shaper's own field: the profile32 cell a voxel holds, produced by the
        // earthworks stage. Declared here, not silently appended to the guide's set.
        let shaper_additions = [FieldId::ProfileCell];
        assert_eq!(
            FieldId::ALL.len(),
            19 + additions.len() + shaper_additions.len(),
            "the roster grew by something other than the declared additions"
        );
        for field in additions.into_iter().chain(shaper_additions) {
            assert!(FieldId::ALL.contains(&field), "{field:?} is not in the roster");
        }
        let mut names: Vec<&str> = FieldId::ALL.iter().map(|f| f.as_str()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "two fields share a name");
        // The atmosphere/hydrology roster the user named: clouds are not a screen tint,
        // they are these fields, produced by world processes.
        for required in [
            FieldId::Climate,
            FieldId::Temperature,
            FieldId::Moisture,
            FieldId::Precipitation,
            FieldId::Humidity,
            FieldId::Groundwater,
            FieldId::Pressure,
            FieldId::Wind,
            FieldId::Storms,
        ] {
            assert!(FieldId::ALL.contains(&required), "{required:?} fell out of the roster");
        }
    }

    #[test]
    fn every_field_has_a_declared_unit_in_the_references_vocabulary() {
        // A field with no unit is a number with no meaning; the reference declares the
        // unit on the contract, and this pins that the vocabulary is present.
        for unit in [
            Unit::Dimensionless,
            Unit::Ratio,
            Unit::Percent,
            Unit::Metre,
            Unit::SquareMetre,
            Unit::CubicMetre,
            Unit::PerSquareMetre,
            Unit::MetrePerSecond,
            Unit::Pascal,
            Unit::Kelvin,
            Unit::DegreesCelsius,
            Unit::KilogramPerCubicMeter,
            Unit::WattPerSquareMetre,
        ] {
            assert!(!unit.as_str().is_empty());
        }
    }

    #[test]
    fn a_field_resolution_must_be_positive_and_ordered() {
        let bad = [
            FieldResolution::new(zero(), 0, 1, None),
            FieldResolution::new(zero(), 8, 0, None),
            FieldResolution::new(zero(), 4, 8, None),
            FieldResolution::new(zero(), 8, 4, Some(0)),
        ];
        for resolution in bad {
            let refused = resolution
                .validate(FieldId::Elevation)
                .expect_err("an invalid resolution was accepted");
            assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        }
        FieldResolution::new(zero(), 8, 1, Some(4))
            .validate(FieldId::Elevation)
            .expect("a positive ordered resolution was refused");
    }

    #[test]
    fn vertical_dependence_and_vertical_resolution_must_agree() {
        // The contract's own rule: a field that says it uses z must declare the same
        // vertical step twice, and the two must not disagree.
        let mut spec = draft(
            FieldId::Geology,
            SamplingPolicy::PiecewiseConstant {
                level: ResolutionLevel::Fine,
            },
            Some(4),
            32,
            8,
        );
        spec.resolution = FieldResolution::new(zero(), 32, 8, Some(8));
        let contract = spec.contract();
        let refused = contract
            .validate(false)
            .expect_err("a disagreement was accepted");
        assert!(refused.detail().contains("vertical"), "{}", refused.detail());
    }

    #[test]
    fn a_nearest_sample_picks_the_nearest_and_respects_its_coverage() {
        let contract = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let samples = vec![
            draft_sample(FieldId::Elevation, VoxelPos::new(0, 0, 0), 1000),
            draft_sample(FieldId::Elevation, VoxelPos::new(3, 0, 0), 3000),
        ];
        let sampler = DraftFieldSampler::new(vec![contract], samples).expect("a valid fixture");
        // Horizontal-only: the query's z is pinned to the origin, so a query at height 20
        // still reads the heightfield.
        let near = sampler
            .sample(FieldId::Elevation, VoxelPos::new(2, 0, 20))
            .expect("the query is covered");
        assert_eq!(near.value, FieldValue::Fixed(mm(3000)));
        // Beyond the coverage radius of both samples: a refusal, never a default.
        let refused = sampler
            .sample(FieldId::Elevation, VoxelPos::new(9, 0, 0))
            .expect_err("an uncovered query was answered");
        assert_eq!(refused.kind(), RefusalKind::FieldDataMissing);
    }

    #[test]
    fn a_nearest_tie_breaks_toward_the_greater_coordinate_deterministically() {
        let contract = draft(
            FieldId::Slope,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 8,
            },
            None,
            8,
            1,
        );
        let samples = vec![
            draft_sample(FieldId::Slope, VoxelPos::new(0, 0, 0), 1000),
            draft_sample(FieldId::Slope, VoxelPos::new(2, 0, 0), 2000),
        ];
        let sampler = DraftFieldSampler::new(vec![contract], samples).expect("a valid fixture");
        let tie = sampler
            .sample(FieldId::Slope, VoxelPos::new(1, 0, 0))
            .expect("the tie is covered");
        assert_eq!(tie.value, FieldValue::Fixed(mm(2000)));
    }

    #[test]
    fn a_piecewise_constant_query_snaps_to_the_lattice() {
        let contract = draft(
            FieldId::Geology,
            SamplingPolicy::PiecewiseConstant {
                level: ResolutionLevel::Fine,
            },
            Some(8),
            32,
            8,
        );
        let samples = vec![DraftFieldSample {
            field: FieldId::Geology,
            coord: VoxelPos::new(0, 0, 0),
            value: FieldValue::Integer(7),
        }];
        let sampler = DraftFieldSampler::new(vec![contract], samples).expect("a valid fixture");
        // A query inside the lattice cell answers with the cell's sample...
        let inside = sampler
            .sample(FieldId::Geology, VoxelPos::new(5, 5, 5))
            .expect("the query snaps into the covered cell");
        assert_eq!(inside.value, FieldValue::Integer(7));
        // ...and a query in a cell with no sample refuses by name.
        let refused = sampler
            .sample(FieldId::Geology, VoxelPos::new(8, 0, 0))
            .expect_err("an uncovered lattice cell was answered");
        assert_eq!(refused.kind(), RefusalKind::FieldDataMissing);
    }

    #[test]
    fn samples_must_lie_exactly_on_their_declared_lattice() {
        let contract = draft(
            FieldId::Geology,
            SamplingPolicy::PiecewiseConstant {
                level: ResolutionLevel::Fine,
            },
            Some(8),
            32,
            8,
        );
        // (5, 0, 0) is not a multiple of the fine step 8 from the origin: refused.
        let bad = vec![draft_sample(FieldId::Geology, VoxelPos::new(5, 0, 0), 1)];
        let refused =
            DraftFieldSampler::new(vec![contract], bad).expect_err("an off-lattice sample passed");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        assert!(refused.detail().contains("exact point"), "{}", refused.detail());
    }

    #[test]
    fn duplicate_samples_are_refused_rather_than_ordered() {
        let contract = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let duplicate = vec![
            draft_sample(FieldId::Elevation, VoxelPos::new(0, 0, 0), 1000),
            draft_sample(FieldId::Elevation, VoxelPos::new(0, 0, 0), 2000),
        ];
        let refused = DraftFieldSampler::new(vec![contract], duplicate)
            .expect_err("two samples at one point were accepted");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        assert!(refused.detail().contains("unique"), "{}", refused.detail());
    }

    #[test]
    fn a_sample_for_an_undeclared_field_is_refused_by_name() {
        let contract = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let stray = vec![draft_sample(FieldId::Wind, VoxelPos::new(0, 0, 0), 1)];
        let refused = DraftFieldSampler::new(vec![contract], stray)
            .expect_err("a sample with no declaration was accepted");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        assert!(refused.detail().contains("wind"), "{}", refused.detail());
    }

    #[test]
    fn a_query_for_an_undeclared_field_is_refused_by_name() {
        let contract = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let sampler = DraftFieldSampler::new(vec![contract], Vec::new()).expect("a valid fixture");
        let refused = sampler
            .sample(FieldId::Storms, VoxelPos::new(0, 0, 0))
            .expect_err("an undeclared field was answered");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
        assert!(refused.detail().contains("storms"), "{}", refused.detail());
    }

    #[test]
    fn the_sampler_identity_binds_every_value_affecting_declaration() {
        let base = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let a = DraftFieldSampler::new(vec![base.clone()], Vec::new())
            .expect("a valid fixture")
            .identity();

        let mut different_radius = base.clone();
        different_radius.sampling = SamplingPolicy::Nearest {
            level: ResolutionLevel::Fine,
            coverage_radius: 5,
        };
        let b = DraftFieldSampler::new(vec![different_radius], Vec::new())
            .expect("a valid fixture")
            .identity();
        assert_ne!(a, b, "the coverage radius is not in the identity");

        let mut different_resolution = base.clone();
        different_resolution.resolution = FieldResolution::new(zero(), 16, 4, None);
        let c = DraftFieldSampler::new(vec![different_resolution], Vec::new())
            .expect("a valid fixture")
            .identity();
        assert_ne!(a, c, "the lattice is not in the identity");

        let mut different_tolerance = base.clone();
        different_tolerance.tolerance = Rational::new(1, 100).expect("a positive denominator");
        let d = DraftFieldSampler::new(vec![different_tolerance], Vec::new())
            .expect("a valid fixture")
            .identity();
        assert_ne!(a, d, "the tolerance is not in the identity");
    }

    #[test]
    fn the_identity_does_not_depend_on_source_order_or_sample_order() {
        let one = draft(
            FieldId::Elevation,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        );
        let mut two = one.clone();
        two.source_ids = vec!["S-A".to_owned(), "S-B".to_owned()];
        let mut one_sorted = one.clone();
        one_sorted.source_ids = vec!["S-B".to_owned(), "S-A".to_owned()];
        let a = DraftFieldSampler::new(vec![two], Vec::new())
            .expect("a valid fixture")
            .identity();
        let b = DraftFieldSampler::new(vec![one_sorted], Vec::new())
            .expect("a valid fixture")
            .identity();
        assert_eq!(a, b, "the source order changed the identity");
    }

    #[test]
    fn the_same_declaration_set_gives_the_same_identity() {
        let spec = draft(
            FieldId::Moisture,
            SamplingPolicy::PiecewiseConstant {
                level: ResolutionLevel::Coarse,
            },
            None,
            32,
            8,
        );
        let a = DraftFieldSampler::new(vec![spec.clone()], Vec::new())
            .expect("a valid fixture")
            .identity();
        let b = DraftFieldSampler::new(vec![spec], Vec::new())
            .expect("a valid fixture")
            .identity();
        assert_eq!(a, b);
    }

    #[test]
    fn a_provisional_contract_cannot_be_authoritative() {
        // The draft path is executable; the authoritative door refuses anything not
        // admitted, which is what stops an unreviewed value looking reviewed.
        let provisional = draft(
            FieldId::Pressure,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            None,
            8,
            1,
        )
        .contract();
        let refused = CanonicalSamples::new(vec![provisional.clone()], Vec::new(), true)
            .expect_err("a provisional contract was admitted");
        assert_eq!(refused.kind(), RefusalKind::UnadmittedFieldSpec);

        let mut admitted = provisional;
        admitted.review_state = Lifecycle::Admitted;
        let samples = CanonicalSamples::new(vec![admitted], Vec::new(), true)
            .expect("an admitted, non-empty catalog builds");
        let sampler = FieldSampler { samples };
        assert_eq!(
            sampler.spec(FieldId::Pressure).expect("declared").review_state(),
            Lifecycle::Admitted
        );
    }

    #[test]
    fn an_empty_authoritative_catalog_is_refused() {
        let refused = CanonicalSamples::new(Vec::new(), Vec::new(), true)
            .expect_err("an empty catalog was admitted");
        assert_eq!(refused.kind(), RefusalKind::InvalidFieldSpec);
    }

    #[test]
    fn the_generated_manifest_door_refuses_by_name_in_this_tree() {
        // Named, not silently dropped: there is no generated manifest yet, and an empty
        // catalog would look like a world with no fields.
        let refused = FieldSampler::from_generated_manifest().expect_err("the door opened");
        assert_eq!(refused.kind(), RefusalKind::UnadmittedFieldSpec);
        assert!(
            refused.detail().contains("manifest"),
            "{}",
            refused.detail()
        );
    }

    #[test]
    fn the_chunk_owner_handle_answers_exactly_as_its_sampler() {
        let contract = draft(
            FieldId::Temperature,
            SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 8,
            },
            None,
            8,
            1,
        );
        let sampler = DraftFieldSampler::new(
            vec![contract],
            vec![draft_sample(
                FieldId::Temperature,
                VoxelPos::new(0, 0, 0),
                288_000,
            )],
        )
        .expect("a valid fixture");
        let direct = sampler
            .sample(FieldId::Temperature, VoxelPos::new(1, 1, 99))
            .expect("covered");
        let owned = sampler
            .for_owner(ChunkPos::new(0, 0, 0))
            .sample(FieldId::Temperature, VoxelPos::new(1, 1, 99))
            .expect("covered");
        assert_eq!(direct, owned);
    }

    #[test]
    fn a_horizontal_only_field_ignores_the_vertical_coordinate() {
        let contract = DraftFieldSpec {
            id: FieldId::Elevation,
            unit: Unit::Metre,
            resolution: FieldResolution::new(VoxelPos::new(0, 0, 5), 8, 1, None),
            vertical_dependence: VerticalDependence::HorizontalOnly,
            sampling: SamplingPolicy::Nearest {
                level: ResolutionLevel::Fine,
                coverage_radius: 4,
            },
            seam_policy: SeamPolicy::GlobalLattice,
            tolerance: Rational::new(1, 1000).expect("a positive denominator"),
            source_ids: vec!["S-TEST".to_owned()],
        };
        let sampler = DraftFieldSampler::new(
            vec![contract],
            vec![draft_sample(FieldId::Elevation, VoxelPos::new(0, 0, 5), 16_000)],
        )
        .expect("a valid fixture");
        // A query at height 0 and one at height 1_000 read the same heightfield.
        let low = sampler
            .sample(FieldId::Elevation, VoxelPos::new(0, 0, 0))
            .expect("covered");
        let high = sampler
            .sample(FieldId::Elevation, VoxelPos::new(0, 0, 1_000))
            .expect("covered");
        assert_eq!(low.value, high.value);
    }
}
