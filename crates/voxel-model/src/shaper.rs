//! The two kinds of voxel shaper, and the type distinction between them.
//!
//! A voxel in this world is a **4×4×4 grid of substvoxels** ([`crate::subvoxel`]),
//! and a shaper is what decides how a *block* becomes a *shape*. Until this module
//! existed there was nothing here at all: `WorldSource::shape_at` answered
//! `VoxelShape::solid(m)` for every voxel in the world, which is the honest
//! statement that **the whole 4×4×4 level was empty of content** — a block was a
//! cube of one material, and the level below it carried nothing.
//!
//! # The two kinds, and why they are two traits and not one
//!
//! The distinction the world model rests on is *what a shaper is allowed to
//! change*:
//!
//! | Kind | Writes | May change the silhouette? | At 4×4×4 | At 256× |
//! |---|---|---|---|---|
//! | **Physical** ([`PhysicalShaper`]) | *occupancy* — which cells hold matter | **yes** | carve and fill cells | a displacement read at the addressed lattice |
//! | **Material** ([`MaterialShaper`]) | *materials* only | **no** | change which material a cell holds | a shading normal bent by the pattern |
//!
//! This is not a flag on one trait. A [`MaterialShaper`] is handed a
//! [`SlotWriter`], and a `SlotWriter` **has no carve method** — so a material
//! shaper that tried to change a block's shape could not be written, let alone
//! run. That is the same shape as the rest of this codebase: `Established` is not
//! a `bool`, a `SourcedReflectance` cannot exist without a source, and a shaper
//! that is not allowed to carve is not *forbidden* from carving, it is **unable**
//! to. The refusal in [`SlotWriter::set`] is the belt to that braces: it catches
//! the one route the type cannot (a caller reaching through a *stored* shape)
//! and names it.
//!
//! Why the distinction earns two traits rather than a comment:
//!
//! * **It is what makes the greedy mesher's merge rule correct.** A physical
//!   shaper changes which cells are occupied, so it changes the surface, so it
//!   changes the mesh. A material shaper cannot, so the *silhouette* of a
//!   material-shaped block is a cube — and a pillar is a cube whose **shading
//!   wraps**, not a block that has been cut round. That is the whole point: a
//!   round silhouette would cost sub-subvoxel geometry, and a round *shading*
//!   costs one function of a position.
//! * **It is what makes performance predictable.** The 256× lattice is addressed,
//!   never stored (C-2). A physical change at 256 could not be stored either, so
//!   the only physical shaper that can act there is one that reads a
//!   **displacement** from the lattice at the moment a surface is shaded. That is
//!   a property of the *kind*, and stating it in the type is what stops the next
//!   person from trying to store it.
//!
//! # Determinism, which is not negotiable
//!
//! Every shaper here is a pure function of `(seed, tick)` and the voxel it is
//! handed. It sees no clock, no thread order and no neighbour it was not given,
//! and [`VoxelContext`] is the complete list of what it may read — the six
//! materials touching it, whether it is open to the sky, its own shape, and the
//! tick. A shaper that wanted to consult the weather would be unable to, which is
//! F12 and WG-Q5 in the same place.
//!
//! # Where the concrete shapers live
//!
//! Here is the *mechanism*. `slab` and `pillar` are in `voxel-shapers`, which is
//! where content belongs (`DESIGN-SPEC.md` §6), and each of them names its source
//! exactly as `docs/SHAPERS.md` requires.

use crate::coord::{SUBVOXEL_COUNT, SUBVOXEL_SIDE};
use crate::material::MaterialId;
use crate::profile::Profile;
use crate::refusal::{Refusal, RefusalKind, RefusalSet, Refused};
use crate::subvoxel::{Face, SubvoxelIndex, VoxelShape};

/// Which of the two kinds a shaper is. Used for reporting and for the registry's
/// own bookkeeping; the *authority* is the trait, not this enum.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShaperKind {
    /// Changes occupancy: a block's silhouette after this shaper is not the one
    /// it had before.
    Physical,
    /// Changes materials only. The silhouette is exactly the one it was handed.
    Material,
}

impl ShaperKind {
    pub const fn name(self) -> &'static str {
        match self {
            ShaperKind::Physical => "physical",
            ShaperKind::Material => "material",
        }
    }
}

/// **Which of the two resolutions a shaper acts at.**
///
/// A shaper kind is *what* it may change; a level is *which grid* it changes it in.
/// The two are independent, which is why they are two enums and not four variants:
///
/// | | [`Level::Subvoxel`] — 4×4×4, **stored** | [`Level::Detail`] — 256×, **addressed** (C-2) |
/// |---|---|---|
/// | [`ShaperKind::Physical`] | which cells hold matter — a [`ShapeWriter`] | a **surface displacement** — a [`DisplacementWriter`] |
/// | [`ShaperKind::Material`] | which material a cell holds — a [`SlotWriter`] | a **shading** variation — a [`ShadingWriter`] |
///
/// The distinction is not decoration. `Level::Subvoxel` is 128 bytes per voxel and
/// is what the octree holds; `Level::Detail` is **2⁴⁵ samples per chunk** if it
/// were ever stored (`DENSE_SAMPLES_PER_CHUNK`) and is therefore only ever a
/// *function*. A shaper that wants to act at `Detail` has to say so, because it is
/// declaring that it can be evaluated rather than stored — and a shaper that cannot
/// be evaluated at an address has no business claiming that level.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Level {
    /// The 4×4×4 grid, stored: 64 cells of material per voxel.
    Subvoxel,
    /// The 256 lattice, addressed: 16 samples per subvoxel axis — the 16×16 face
    /// the base is named for.
    Detail,
}

impl Level {
    pub const ALL: [Level; 2] = [Level::Subvoxel, Level::Detail];

    pub const fn name(self) -> &'static str {
        match self {
            Level::Subvoxel => "4x4x4",
            Level::Detail => "256",
        }
    }

    /// Samples along one axis at this level.
    pub const fn side(self) -> u32 {
        match self {
            Level::Subvoxel => crate::coord::SUBVOXEL_SIDE,
            Level::Detail => crate::coord::DETAIL_SIDE,
        }
    }
}

/// Which levels a shaper declares it acts at. A **mask**, so "both" is a value and
/// not two booleans that can disagree.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct LevelMask(u8);

impl LevelMask {
    pub const NONE: LevelMask = LevelMask(0);
    pub const SUBVOXEL: LevelMask = LevelMask(1);
    pub const DETAIL: LevelMask = LevelMask(2);
    pub const BOTH: LevelMask = LevelMask(3);

    pub const fn of(level: Level) -> Self {
        match level {
            Level::Subvoxel => Self::SUBVOXEL,
            Level::Detail => Self::DETAIL,
        }
    }

    pub const fn union(self, other: LevelMask) -> Self {
        LevelMask(self.0 | other.0)
    }

    pub const fn has(self, level: Level) -> bool {
        self.0 & Self::of(level).0 != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// How many levels this mask names: 0, 1 or 2.
    pub const fn count(self) -> u32 {
        (self.0 & 1) as u32 + ((self.0 >> 1) & 1) as u32
    }
}

/// **How many shapers act at each of the four (kind, level) cells.** See
/// [`ShaperSet::tally`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Tally {
    pub physical_subvoxel: usize,
    pub physical_detail: usize,
    pub material_subvoxel: usize,
    pub material_detail: usize,
}

impl Tally {
    /// Counts in `[Kind::Physical, Kind::Material] × [Level::Subvoxel, Level::Detail]`
    /// order, which is the order a report prints them in.
    pub const fn cells(&self) -> [[usize; 2]; 2] {
        [
            [self.physical_subvoxel, self.physical_detail],
            [self.material_subvoxel, self.material_detail],
        ]
    }

    pub const fn total(&self) -> usize {
        self.physical_subvoxel + self.physical_detail + self.material_subvoxel + self.material_detail
    }

    /// How many of the four cells have a shaper in them. A world with 4 is using
    /// the whole taxonomy; a world with 1 is using a quarter of it, and the number
    /// says so rather than the world looking complete.
    pub const fn occupied_cells(&self) -> usize {
        let mut n = 0;
        let cells = self.cells();
        let mut i = 0;
        while i < 2 {
            let mut j = 0;
            while j < 2 {
                if cells[i][j] > 0 {
                    n += 1;
                }
                j += 1;
            }
            i += 1;
        }
        n
    }
}

/// Which face, as a stable index. The order is [`Face::ALL`]'s, so a table keyed by
/// this and a table keyed by `Face` enumerate the same way.
const fn face_slot(face: Face) -> usize {
    match face {
        Face::NegX => 0,
        Face::PosX => 1,
        Face::NegY => 2,
        Face::PosY => 3,
        Face::NegZ => 4,
        Face::PosZ => 5,
    }
}

/// The tallest a **profile** may stand above its own plane, in subvoxels: `2.0`, which is
/// half a block.
///
/// The bound is the block's, and its source is geometric rather than arithmetic: a
/// cross-section may not exceed the circle inscribed in the block, so the tallest cap a
/// block can hold is that circle's radius — half a block, two subvoxels. It is a
/// *different* bound from [`MAX_DISPLACEMENT_SUBVOXELS`], which is about a trim crossing
/// a cell boundary, and the two were conflated in the first draft: a `RoundColumn` of
/// radius 0.5 declares a 2-subvoxel apex, and the shared bound refused every hemisphere
/// this repository could draw.
///
/// The distinction is the same one the two **kinds** draw. A bare amplitude is a trim of
/// a surface somebody else shaped; a profile *is* the shape.
pub const MAX_PROFILE_SUBVOXELS: f64 = crate::profile::MAX_RADIUS * SUBVOXEL_SIDE as f64;

/// A displacement amplitude may not exceed half a subvoxel.
///
/// The reason is arithmetic and not taste. A displacement moves a surface off the
/// cell it belongs to; past half a subvoxel it would cross the cell boundary and
/// the *cell next door* would have to move to stay connected, which is a shape
/// change — a [`PhysicalShaper`]'s **`Level::Subvoxel`** job. A larger relief is
/// therefore a *carve*, said in a shape rather than in a displacement, and the
/// refusal below says so by name rather than clamping to a half.
pub const MAX_DISPLACEMENT_SUBVOXELS: f64 = 0.5;

/// One face's surface displacement at the 256 level.
///
/// The **shape** is carried by an optional [`Profile`] — an analytic law — and the
/// amplitude is either derived from that profile or declared outright. Carrying the
/// law and not just a number is what makes this the *physical* level rather than a
/// decorative one: the refinement stage samples the law at a 256-address and moves
/// a vertex by the result, so the geometry follows the same function the shading
/// does. A field with no profile is one whose *sign* comes from the lattice and
/// whose magnitude is the amplitude — a rough surface, a ripple band.
#[derive(Clone, PartialEq, Debug)]
pub struct DisplacementEntry {
    pub material: MaterialId,
    pub face: Face,
    /// The relief's scale in subvoxels: for a profile, its apex height converted
    /// from block units (4 subvoxels per block); for a bare amplitude, the relief's
    /// own magnitude.
    pub amplitude_subvoxels: f64,
    /// The law the surface follows, when it has one.
    pub profile: Option<Profile>,
}

impl DisplacementEntry {
    /// The displacement at a face coordinate, in **subvoxels**, positive outward.
    ///
    /// For a profile this is the profile's own protrusion sampled at that point —
    /// the exact surface, not an approximation of it. For a bare amplitude the
    /// magnitude is returned and the caller supplies the sign from the lattice,
    /// because a field with no law has no shape of its own.
    ///
    /// The coordinate is refused outside `[0, 1]` rather than clamped: a clamp here
    /// would silently move a vertex to a place nobody asked for.
    pub fn displacement_at(&self, u: f64, v: f64) -> Refused<f64> {
        if !(0.0..=1.0).contains(&u) || !u.is_finite() || !(0.0..=1.0).contains(&v) || !v.is_finite()
        {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("face coordinate ({u}, {v}) outside [0, 1]"),
            ));
        }
        match &self.profile {
            None => Ok(self.amplitude_subvoxels),
            Some(profile) => {
                let du = u - 0.5;
                let dv = v - 0.5;
                let r = (du * du + dv * dv).sqrt();
                // Block units to subvoxels: a block is four subvoxels on a side.
                let scale = SUBVOXEL_SIDE as f64;
                Ok(profile.cap_height(r) * scale)
            }
        }
    }
}

/// **The physical shaper at 256.** What the surface does *between* the stored
/// cells, keyed by material and face.
///
/// This is a **declaration**, not a per-voxel value, and that is the whole design:
/// a displacement field's parameters are a property of the *world's pipeline* the
/// way sea level is, and the field's value at a point is a pure function of
/// `(domain, seed, world position, address)`. Storing a displacement per voxel
/// would be the thing C-2 forbids on the other level — 2⁴⁵ samples per chunk — and
/// the way out is the same one the lattice takes: **declare the function, evaluate
/// it at an address, forget it.**
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DisplacementField {
    domain: u64,
    /// `(material, face) -> entry`, in declaration order, with the key's face as a
    /// stable slot so a lookup does not need `Face` to be `Ord`.
    entries: Vec<((u16, usize), DisplacementEntry)>,
    /// How many distinct materials this field touches — a number the audit states.
    materials: usize,
}

impl DisplacementField {
    /// A field that moves nothing. The default, and the honest value for a world
    /// whose shapers all act at `Level::Subvoxel`.
    pub const fn none() -> Self {
        DisplacementField { domain: 0, entries: Vec::new(), materials: 0 }
    }

    /// The domain tag. Folded into the lattice's seed so two displacement fields
    /// in one world cannot be the same field by accident.
    pub const fn domain(&self) -> u64 {
        self.domain
    }

    pub fn set_domain(&mut self, domain: u64) {
        self.domain = domain;
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// How many distinct materials have a face that moves.
    pub fn materials(&self) -> usize {
        self.materials
    }

    /// The entry for `(material, face)`, or `None` when that face does not move.
    ///
    /// `None` and not a zero: a face that does not move and a face that moves by
    /// zero are the same picture and different claims, and the caller that wants
    /// to know "does anything happen here" must be able to ask.
    pub fn entry(&self, material: MaterialId, face: Face) -> Option<&DisplacementEntry> {
        let key = (material.0, face_slot(face));
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, e)| e)
    }

    /// The amplitude for a face, or `None` when it does not move.
    pub fn amplitude(&self, material: MaterialId, face: Face) -> Option<f64> {
        self.entry(material, face).map(|e| e.amplitude_subvoxels)
    }

    /// The displacement at a face coordinate, in subvoxels, or `None` when the face
    /// does not move. See [`DisplacementEntry::displacement_at`].
    pub fn displacement_at(
        &self,
        material: MaterialId,
        face: Face,
        u: f64,
        v: f64,
    ) -> Option<Refused<f64>> {
        self.entry(material, face).map(|e| e.displacement_at(u, v))
    }

    /// **Declare the law.** The amplitude is *derived* from the profile: a profile's
    /// apex height in block units, converted at four subvoxels to the block.
    ///
    /// This is the constructor a shaper with a real shape uses, and it is why the
    /// amplitude is not a number anybody picks: `cap_height_total()` is the shape's
    /// own height, and a profile whose apex exceeds the half-subvoxel bound is
    /// refused with the same reasoning a bare amplitude would be — a relief that
    /// large is a carve.
    pub fn declare_profile(&mut self, material: MaterialId, face: Face, profile: Profile) -> Refused<()> {
        let amplitude = profile.cap_height_total() * SUBVOXEL_SIDE as f64;
        self.insert(material, face, amplitude, Some(profile))
    }

    /// **Declare a magnitude.** For a relief with no law of its own — a lattice's
    /// roughness, a ripple band — where the sign comes from the address.
    pub fn declare(&mut self, material: MaterialId, face: Face, amplitude_subvoxels: f64) -> Refused<()> {
        self.insert(material, face, amplitude_subvoxels, None)
    }

    fn insert(
        &mut self,
        material: MaterialId,
        face: Face,
        amplitude_subvoxels: f64,
        profile: Option<Profile>,
    ) -> Refused<()> {
        if material.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "a displacement field was declared for the empty id, which has no surface",
            ));
        }
        if !amplitude_subvoxels.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("displacement {amplitude_subvoxels} is not a finite number"),
            ));
        }
        // **Two bounds, because they are two questions.** A bare amplitude is a *trim* of
        // a surface, and past half a subvoxel it would cross the cell boundary and the
        // cell next door would have to move to stay connected — that is a shape change,
        // and a carve at the stored level. A **profile** is the body's own surface inside
        // its block, and the tallest one a block can hold is the circle inscribed in it:
        // half a block. Sharing one number between those two questions was the first
        // draft's error, and it made every hemisphere half a block too short to declare.
        let bound = match profile {
            None => MAX_DISPLACEMENT_SUBVOXELS,
            Some(_) => MAX_PROFILE_SUBVOXELS,
        };
        if amplitude_subvoxels.abs() > bound {
            let why = match profile {
                None => "a trim that large would cross the cell boundary and the cell next door would have to move, which is a carve at the 4x4x4 level".to_string(),
                Some(_) => format!(
                    "a profile may not stand more than half a block above its own plane, because a cross-section may not exceed the circle inscribed in the block ({} subvoxels)",
                    MAX_PROFILE_SUBVOXELS
                ),
            };
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("displacement {amplitude_subvoxels} subvoxels exceeds the bound of {bound}: {why}"),
            ));
        }
        let key = (material.0, face_slot(face));
        if let Some((_, existing)) = self.entries.iter().find(|(k, _)| *k == key) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "{material:?} on {face:?} already moves by {}; two shapers may not displace one face",
                    existing.amplitude_subvoxels
                ),
            ));
        }
        if !self.entries.iter().any(|(k, _)| k.0 == material.0) {
            self.materials += 1;
        }
        self.entries.push((key, DisplacementEntry { material, face, amplitude_subvoxels, profile }));
        Ok(())
    }

    /// Every declared entry, in declaration order — what a report prints.
    pub fn entries(&self) -> impl Iterator<Item = &DisplacementEntry> + '_ {
        self.entries.iter().map(|(_, e)| e)
    }

    /// How many declared faces carry a **law** rather than a bare magnitude. A
    /// number worth printing: it is the difference between a surface that follows a
    /// shape and one that merely wobbles.
    pub fn profiled_faces(&self) -> usize {
        self.entries.iter().filter(|(_, e)| e.profile.is_some()).count()
    }
}

/// The writer a **material** shaper declares its 256-level field through.
pub struct ShadingWriter<'a> {
    field: &'a mut ShadingField,
}

impl ShadingWriter<'_> {
    pub(crate) fn new(field: &mut ShadingField) -> ShadingWriter<'_> {
        ShadingWriter { field }
    }

    pub fn declare(&mut self, material: MaterialId, roughness_gain: f64, reflectance_gain: f64) -> Refused<()> {
        self.field.declare(material, roughness_gain, reflectance_gain)
    }

    pub fn set_domain(&mut self, domain: u64) {
        self.field.set_domain(domain);
    }
}

/// The writer a **physical** shaper declares its 256-level field through.
pub struct DisplacementWriter<'a> {
    field: &'a mut DisplacementField,
}

impl DisplacementWriter<'_> {
    pub(crate) fn new(field: &mut DisplacementField) -> DisplacementWriter<'_> {
        DisplacementWriter { field }
    }

    /// Make `material`'s `face` move by `amplitude_subvoxels`, positive being
    /// **outward**.
    pub fn declare(&mut self, material: MaterialId, face: Face, amplitude_subvoxels: f64) -> Refused<()> {
        self.field.declare(material, face, amplitude_subvoxels)
    }

    /// Make `material`'s `face` follow `profile` — the exact surface of a body,
    /// sampled at an address by whoever refines the mesh. The amplitude is derived
    /// from the profile and never chosen.
    pub fn declare_profile(&mut self, material: MaterialId, face: Face, profile: Profile) -> Refused<()> {
        self.field.declare_profile(material, face, profile)
    }

    /// A profile on every face of `material`.
    pub fn declare_profile_all_faces(&mut self, material: MaterialId, profile: Profile) -> Refused<()> {
        for face in Face::ALL {
            self.declare_profile(material, face, profile)?;
        }
        Ok(())
    }

    /// Make every face of `material` move by the same amount.
    pub fn declare_all_faces(&mut self, material: MaterialId, amplitude_subvoxels: f64) -> Refused<()> {
        for face in Face::ALL {
            self.declare(material, face, amplitude_subvoxels)?;
        }
        Ok(())
    }

    /// Set the domain tag the field's noise is folded under.
    pub fn set_domain(&mut self, domain: u64) {
        self.field.set_domain(domain);
    }
}

/// **The material shaper at 256.** What the shading does *between* the stored
/// cells — per material, a scale on each of the lattice's channels.
///
/// The lattice already varies a material's roughness and reflectance (a mean-zero
/// deviation, `voxel-appearance::detail`). A material shaper at this level says
/// **how much** of that variation this material shows, which is a property of the
/// material and not of an address — and it is why this is a small table rather
/// than a stored field.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ShadingField {
    domain: u64,
    gains: Vec<(u16, f64, f64)>,
}

impl ShadingField {
    pub const fn none() -> Self {
        ShadingField { domain: 0, gains: Vec::new() }
    }

    pub const fn domain(&self) -> u64 {
        self.domain
    }

    pub fn set_domain(&mut self, domain: u64) {
        self.domain = domain;
    }

    pub fn is_empty(&self) -> bool {
        self.gains.is_empty()
    }

    pub fn len(&self) -> usize {
        self.gains.len()
    }

    /// `(roughness gain, reflectance gain)` for a material, or `None` when the
    /// material is not varied at this level at all.
    pub fn gain(&self, material: MaterialId) -> Option<(f64, f64)> {
        self.gains
            .iter()
            .find(|(m, _, _)| *m == material.0)
            .map(|(_, r, f)| (*r, *f))
    }

    /// Declare a material's variation. A gain outside `[0, 1]` is refused: above 1
    /// the deviation could push a parameter out of its own declared range, and the
    /// appearance layer refuses *that* rather than clamping, so a gain above 1 can
    /// only ever produce a refusal at shade time.
    pub fn declare(&mut self, material: MaterialId, roughness_gain: f64, reflectance_gain: f64) -> Refused<()> {
        if material.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "a shading field was declared for the empty id",
            ));
        }
        for (name, value) in [("roughness", roughness_gain), ("reflectance", reflectance_gain)] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("a {name} gain of {value} is outside [0, 1]"),
                ));
            }
        }
        if self.gains.iter().any(|(m, _, _)| *m == material.0) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("{material:?} already has a shading gain; two shapers may not vary one material"),
            ));
        }
        self.gains.push((material.0, roughness_gain, reflectance_gain));
        Ok(())
    }

    /// Every declared gain, in declaration order.
    pub fn entries(&self) -> impl Iterator<Item = (MaterialId, f64, f64)> + '_ {
        self.gains.iter().map(|(m, r, f)| (MaterialId(*m), *r, *f))
    }
}

/// What a shaper is allowed to know about the world around its voxel.
///
/// A shaper never gets a world. It gets this, and the reason is determinism: a
/// shaper holding a world can read a value that depends on which chunks happen to
/// be resident, and then the world stops being a pure function of its seed. So
/// the environment is *summarised* by the caller, into exactly the six things a
/// shaper has a use for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Neighbourhood {
    /// The material touching each face, in [`Face::ALL`] order. [`MaterialId::EMPTY`]
    /// when there is nothing there.
    pub faces: [MaterialId; 6],
    /// True when nothing solid stands between this voxel and the sky. A surface
    /// feature needs it; a cave feature needs it to be false.
    pub open_to_sky: bool,
}

impl Neighbourhood {
    /// A voxel buried in one material on every side and under cover.
    pub const fn buried(material: MaterialId) -> Self {
        Neighbourhood { faces: [material; 6], open_to_sky: false }
    }

    /// A voxel with air on every side, in the open.
    pub const fn exposed() -> Self {
        Neighbourhood { faces: [MaterialId::EMPTY; 6], open_to_sky: true }
    }

    /// The material on one face.
    pub fn face(&self, face: Face) -> MaterialId {
        let index = match face {
            Face::NegX => 0,
            Face::PosX => 1,
            Face::NegY => 2,
            Face::PosY => 3,
            Face::NegZ => 4,
            Face::PosZ => 5,
        };
        self.faces[index]
    }

    /// How many of the six faces touch nothing.
    pub fn open_faces(&self) -> u32 {
        self.faces.iter().filter(|m| m.is_empty()).count() as u32
    }

    /// True when something solid is overhead — the shape a cave ceiling has, and
    /// the condition a stalactite hangs from.
    pub fn has_ceiling(&self) -> bool {
        !self.face(Face::PosZ).is_empty()
    }

    /// True when something solid is underneath — the condition a stalagmite
    /// stands on.
    pub fn has_floor(&self) -> bool {
        !self.face(Face::NegZ).is_empty()
    }
}

impl Default for Neighbourhood {
    fn default() -> Self {
        Self::exposed()
    }
}

/// Everything a shaper may read, and nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoxelContext {
    /// The voxel's world position. World, not chunk-local, because two chunks
    /// must agree about a boundary voxel and a chunk-local coordinate would let
    /// them disagree.
    pub at: [i32; 3],
    /// The shape as every shaper above this one has left it.
    pub shape: VoxelShape,
    pub around: Neighbourhood,
    /// The world seed. A shaper that wants variation takes it from here and
    /// never from a clock.
    pub seed: u64,
    /// The tick. A shaper that wants *time* — a season, a tide — takes it here.
    pub tick: u64,
}

impl VoxelContext {
    pub const fn new(
        at: [i32; 3],
        shape: VoxelShape,
        around: Neighbourhood,
        seed: u64,
        tick: u64,
    ) -> Self {
        VoxelContext { at, shape, around, seed, tick }
    }

    /// True when something solid is overhead — the condition a stalactite hangs
    /// from. Delegates to [`Neighbourhood::has_ceiling`].
    pub fn has_ceiling(&self) -> bool {
        self.around.has_ceiling()
    }

    /// True when something solid is underneath — the condition a stalagmite
    /// stands on.
    pub fn has_floor(&self) -> bool {
        self.around.has_floor()
    }
}

/// The writer a **physical** shaper is given. It may carve and it may fill.
///
/// The two methods that change occupancy are deliberately the two that this type
/// has; the two that do not are the two it does not need.
pub struct ShapeWriter<'a> {
    shape: &'a mut VoxelShape,
}

impl<'a> ShapeWriter<'a> {
    pub(crate) fn new(shape: &'a mut VoxelShape) -> Self {
        ShapeWriter { shape }
    }

    pub fn get(&self, at: SubvoxelIndex) -> MaterialId {
        self.shape.get(at)
    }

    /// Overwrite a cell's material in place. Occupancy-neutral: it refuses to
    /// turn an empty cell solid or a solid cell empty, because that is
    /// [`Self::fill`]'s and [`Self::carve`]'s job and doing it here would make
    /// the two paths disagree about which one the caller meant.
    pub fn set(&mut self, at: SubvoxelIndex, material: MaterialId) -> Refused<()> {
        let was = self.shape.get(at);
        if was.is_empty() != material.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::ShaperKindViolation,
                format!(
                    "ShapeWriter::set changed occupancy at {:?}: {:?} -> {material:?}; use fill or carve",
                    (at.x, at.y, at.z),
                    was
                ),
            ));
        }
        self.shape.set(at, material);
        Ok(())
    }

    /// Put matter into an empty cell. A **physical** change.
    pub fn fill(&mut self, at: SubvoxelIndex, material: MaterialId) -> Refused<()> {
        if material.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::UnadmittedMaterial,
                "fill was asked to place the empty id, which is a carve",
            ));
        }
        self.shape.set(at, material);
        Ok(())
    }

    /// Take matter out of a cell. A **physical** change, and the only one that
    /// may reduce the block to nothing.
    pub fn carve(&mut self, at: SubvoxelIndex) {
        self.shape.set(at, MaterialId::EMPTY);
    }

    /// Carve every cell the predicate admits.
    pub fn carve_where(&mut self, mut predicate: impl FnMut(SubvoxelIndex) -> bool) {
        for at in SubvoxelIndex::all() {
            if predicate(at) {
                self.carve(at);
            }
        }
    }

    /// Fill every empty cell the predicate admits.
    pub fn fill_where(&mut self, material: MaterialId, mut predicate: impl FnMut(SubvoxelIndex) -> bool) -> Refused<()> {
        if material.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::UnadmittedMaterial,
                "fill_where was asked to place the empty id",
            ));
        }
        for at in SubvoxelIndex::all() {
            if self.shape.get(at).is_empty() && predicate(at) {
                self.shape.set(at, material);
            }
        }
        Ok(())
    }
}

/// The writer a **material** shaper is given. It has no carve and no fill, which
/// is what makes the kind a property of the type rather than a promise.
pub struct SlotWriter<'a> {
    cells: &'a mut [MaterialId; SUBVOXEL_COUNT],
}

impl<'a> SlotWriter<'a> {
    pub(crate) fn new(cells: &'a mut [MaterialId; SUBVOXEL_COUNT]) -> Self {
        SlotWriter { cells }
    }

    pub fn get(&self, at: SubvoxelIndex) -> MaterialId {
        self.cells[at.flat()]
    }

    /// True when the cell holds matter — which is to say, when this shaper is
    /// allowed to touch it.
    pub fn occupied(&self, at: SubvoxelIndex) -> bool {
        !self.cells[at.flat()].is_empty()
    }

    /// Re-slot a cell's material.
    ///
    /// **Refuses any change in occupancy**, in either direction. The type already
    /// cannot carve; this stops the one remaining route, a caller handing in a
    /// shape and asking a material shaper to empty a cell "because it is allowed
    /// to write there". It is not: emptying a cell is a silhouette change, and
    /// silhouette changes belong to physical shapers. The refusal names the
    /// shaper's own kind so the message says what to do instead.
    pub fn set(&mut self, at: SubvoxelIndex, material: MaterialId) -> Refused<()> {
        let was = self.cells[at.flat()];
        if was.is_empty() != material.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::ShaperKindViolation,
                format!(
                    "a material shaper tried to change occupancy at {:?}: {:?} -> {material:?}",
                    (at.x, at.y, at.z),
                    was
                ),
            ));
        }
        self.cells[at.flat()] = material;
        Ok(())
    }

    /// Re-slot every cell holding `from` as `to`. The common shape of a material
    /// shaper, and the only one that cannot be got wrong by an off-by-one.
    pub fn replace(&mut self, from: MaterialId, to: MaterialId) -> Refused<usize> {
        let mut changed = 0usize;
        for flat in 0..SUBVOXEL_COUNT {
            if self.cells[flat] == from && !from.is_empty() {
                self.cells[flat] = to;
                changed += 1;
            }
        }
        if !from.is_empty() && !to.is_empty() {
            return Ok(changed);
        }
        Err(Refusal::owned(
            RefusalKind::ShaperKindViolation,
            "replace would have changed occupancy; a material shaper cannot",
        ))
    }

    /// The whole grid, for a shaper that wants to read the pattern it is
    /// rewriting (which is most of them: a pillar's inset is relative to what is
    /// already there).
    pub fn cells(&self) -> &[MaterialId; SUBVOXEL_COUNT] {
        self.cells
    }
}

/// A shaper that **changes the shape of a block**: which of its 64 cells hold
/// matter, and — at the other level — how far the surface between them moves.
pub trait PhysicalShaper: Send + Sync {
    /// A short stable name, used in a report and in a refusal message.
    fn name(&self) -> &'static str;

    /// The source this shaper is built from. Empty is not allowed by the
    /// registry: `docs/SHAPERS.md`'s rule is that no shaper exists as an
    /// unexplained pile of noise, and it applies to a `slab` exactly as much as
    /// to a tectonic field.
    fn source(&self) -> &'static str;

    /// **Which levels this shaper acts at.** Required, with no default, and that
    /// is deliberate: a shaper that claims `Level::Detail` is claiming it can be
    /// *evaluated at an address* rather than stored, and a shaper that cannot do
    /// that has no business at that level. A test per shaper checks the claim
    /// against the shaper's own effect, because a declaration nothing verifies is
    /// a comment.
    fn acts_at(&self) -> LevelMask;

    /// Whether this shaper has anything to say about this voxel. Asked **before**
    /// a shape is opened for writing, so a shaper that does not apply cannot
    /// touch one.
    fn applies(&self, ctx: &VoxelContext) -> bool;

    /// Change the shape. May leave it untouched, and often will. A shaper that
    /// acts only at [`Level::Detail`] returns `Ok(())` here, and the default says
    /// so rather than making every such shaper write an empty method.
    fn carve(&self, _ctx: &VoxelContext, _out: &mut ShapeWriter<'_>) -> Refused<()> {
        Ok(())
    }

    /// Declare the **256-level** surface displacement. Called **once per world**,
    /// not per voxel: a displacement field's parameters are a property of the
    /// pipeline the way sea level is, and the field's value at a point is a pure
    /// function of the world seed and the address. A shaper that acts only at
    /// `Level::Subvoxel` returns `Ok(())` here.
    fn displace(&self, _out: &mut DisplacementWriter<'_>) -> Refused<()> {
        Ok(())
    }
}

/// A shaper that **changes only a block's material slots**, and — at the other
/// level — how much the lattice varies them.
pub trait MaterialShaper: Send + Sync {
    fn name(&self) -> &'static str;
    fn source(&self) -> &'static str;

    /// See [`PhysicalShaper::acts_at`].
    fn acts_at(&self) -> LevelMask;

    fn applies(&self, ctx: &VoxelContext) -> bool;

    /// Re-slot materials. Cannot carve: see [`SlotWriter`].
    fn slot(&self, ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()>;

    /// Declare the **256-level** shading variation. Called once per world, for the
    /// same reason [`PhysicalShaper::displace`] is.
    fn shade(&self, _out: &mut ShadingWriter<'_>) -> Refused<()> {
        Ok(())
    }
}

/// One shaper of either kind, so a pipeline can hold both in one ordered list.
///
/// The order is the **stage graph** `docs/SHAPERS.md` §2 describes: a shaper
/// reads only what the shapers above it wrote, and the list's order *is* that
/// statement. Physical shapers conventionally run first, because a material
/// shaper's inset pattern is relative to whatever silhouette it is given, but the
/// list does not enforce that and the concrete pipelines say why they chose what
/// they chose.
pub enum Shaper {
    Physical(Box<dyn PhysicalShaper>),
    Material(Box<dyn MaterialShaper>),
}

impl Shaper {
    pub fn name(&self) -> &'static str {
        match self {
            Shaper::Physical(s) => s.name(),
            Shaper::Material(s) => s.name(),
        }
    }

    pub fn source(&self) -> &'static str {
        match self {
            Shaper::Physical(s) => s.source(),
            Shaper::Material(s) => s.source(),
        }
    }

    pub fn kind(&self) -> ShaperKind {
        match self {
            Shaper::Physical(_) => ShaperKind::Physical,
            Shaper::Material(_) => ShaperKind::Material,
        }
    }

    /// Which levels this shaper declares it acts at.
    pub fn acts_at(&self) -> LevelMask {
        match self {
            Shaper::Physical(s) => s.acts_at(),
            Shaper::Material(s) => s.acts_at(),
        }
    }

    pub fn applies(&self, ctx: &VoxelContext) -> bool {
        match self {
            Shaper::Physical(s) => s.applies(ctx),
            Shaper::Material(s) => s.applies(ctx),
        }
    }

    /// Run the **256-level declaration** of this shaper, if it has one. Called
    /// once per world rather than once per voxel — see [`PhysicalShaper::displace`].
    fn declare(&self, displacement: &mut DisplacementField, shading: &mut ShadingField) -> Refused<()> {
        if !self.acts_at().has(Level::Detail) {
            return Ok(());
        }
        match self {
            Shaper::Physical(s) => s.displace(&mut DisplacementWriter::new(displacement)),
            Shaper::Material(s) => s.shade(&mut ShadingWriter::new(shading)),
        }
    }

    /// Run this shaper. The receiver is built from the shaper's own kind, so a
    /// shaper can only ever be handed the writer its kind permits.
    fn apply(&self, ctx: &VoxelContext, shape: &mut VoxelShape) -> Refused<()> {
        if !self.applies(ctx) {
            return Ok(());
        }
        match self {
            Shaper::Physical(s) => {
                let mut w = ShapeWriter::new(shape);
                s.carve(ctx, &mut w)
            }
            Shaper::Material(s) => {
                let mut w = SlotWriter::new(shape.cells_mut());
                s.slot(ctx, &mut w)
            }
        }
    }
}

/// An ordered list of shapers, applied to one voxel.
#[derive(Default)]
pub struct ShaperSet {
    shapers: Vec<Shaper>,
}

impl ShaperSet {
    pub fn new() -> Self {
        ShaperSet { shapers: Vec::new() }
    }

    /// Add a physical shaper to the end of the order.
    pub fn with_physical(mut self, shaper: impl PhysicalShaper + 'static) -> Self {
        self.shapers.push(Shaper::Physical(Box::new(shaper)));
        self
    }

    /// Add a material shaper to the end of the order.
    pub fn with_material(mut self, shaper: impl MaterialShaper + 'static) -> Self {
        self.shapers.push(Shaper::Material(Box::new(shaper)));
        self
    }

    /// Add one of either kind.
    pub fn push(&mut self, shaper: Shaper) {
        self.shapers.push(shaper);
    }

    pub fn len(&self) -> usize {
        self.shapers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapers.is_empty()
    }

    /// Every shaper, in the order it runs.
    pub fn shapers(&self) -> &[Shaper] {
        &self.shapers
    }

    /// How many of each kind — the number a report states, so that "there are
    /// two kinds and this world has three of the other one" is visible.
    pub fn kinds(&self) -> (usize, usize) {
        let mut physical = 0;
        let mut material = 0;
        for s in &self.shapers {
            match s.kind() {
                ShaperKind::Physical => physical += 1,
                ShaperKind::Material => material += 1,
            }
        }
        (physical, material)
    }

    /// **How many shapers act at each of the four (kind, level) cells.**
    ///
    /// This exists because the two kinds are not the whole taxonomy: a world can
    /// have three physical shapers and not one of them acting at the 256 level,
    /// and that is a fact a report should be able to state rather than a fact a
    /// person has to infer. A shaper acting at both levels is counted in two
    /// cells, which is what a mask means.
    pub fn tally(&self) -> Tally {
        let mut tally = Tally::default();
        for s in &self.shapers {
            let mask = s.acts_at();
            match s.kind() {
                ShaperKind::Physical => {
                    if mask.has(Level::Subvoxel) {
                        tally.physical_subvoxel += 1;
                    }
                    if mask.has(Level::Detail) {
                        tally.physical_detail += 1;
                    }
                }
                ShaperKind::Material => {
                    if mask.has(Level::Subvoxel) {
                        tally.material_subvoxel += 1;
                    }
                    if mask.has(Level::Detail) {
                        tally.material_detail += 1;
                    }
                }
            }
        }
        tally
    }

    /// **The 256 level, for real.** Run every shaper's declaration once, and
    /// refuse a shaper that *claimed* `Level::Detail` and then changed nothing.
    ///
    /// A declaration nothing verifies is a comment, so this is the check that
    /// makes `acts_at` mean something: the fields' sizes are taken before and
    /// after each shaper, and a Detail-claiming shaper that moved neither is
    /// refused **by name**. The alternative — trusting the mask — is how a level
    /// becomes decorative.
    pub fn declare(&self) -> Result<(DisplacementField, ShadingField), RefusalSet> {
        let mut displacement = DisplacementField::none();
        let mut shading = ShadingField::none();
        let mut refusals = RefusalSet::new();
        for shaper in &self.shapers {
            let before = (displacement.len(), shading.len());
            let mask = shaper.acts_at();
            if mask.is_empty() {
                refusals.refuse(
                    RefusalKind::MissingStage,
                    format!(
                        "shaper {:?} declares no level; a shaper that acts nowhere is not a shaper",
                        shaper.name()
                    ),
                );
                continue;
            }
            if let Err(refusal) = shaper.declare(&mut displacement, &mut shading) {
                refusals.push(refusal);
                continue;
            }
            let after = (displacement.len(), shading.len());
            if mask.has(Level::Detail) && after == before {
                refusals.refuse(
                    RefusalKind::MissingStage,
                    format!(
                        "shaper {:?} claims the {} level and declared nothing there",
                        shaper.name(),
                        Level::Detail.name()
                    ),
                );
            }
        }
        if refusals.is_empty() {
            return Ok((displacement, shading));
        }
        Err(refusals)
    }

    /// **Refuse a registry that cannot say what it is.** Every shaper must name a
    /// source, and no two may share a name. Called once, before a world is built,
    /// so a shaper added without a citation stops the world rather than shipping
    /// inside it.
    ///
    /// Returns a [`RefusalSet`](crate::refusal::RefusalSet) rather than one
    /// refusal, because a registry with four uncited shapers has four problems
    /// and fixing them one run at a time is how the fifth gets missed.
    pub fn audit(&self) -> Result<(), crate::refusal::RefusalSet> {
        let mut set = crate::refusal::RefusalSet::new();
        for (index, s) in self.shapers.iter().enumerate() {
            if s.source().trim().is_empty() {
                set.refuse(
                    RefusalKind::MissingStage,
                    format!(
                        "shaper #{} ({}) names no source; docs/SHAPERS.md forbids a shaper that is an unexplained pile of noise",
                        index,
                        s.name()
                    ),
                );
            }
            if s.name().trim().is_empty() {
                set.refuse(RefusalKind::MissingStage, format!("shaper #{index} has no name"));
            }
            for earlier in &self.shapers[..index] {
                if earlier.name() == s.name() {
                    set.refuse(
                        RefusalKind::MissingStage,
                        format!("two shapers are both called {:?}", s.name()),
                    );
                }
            }
        }
        if set.is_empty() {
            return Ok(());
        }
        Err(set)
    }

    /// Run every shaper over one voxel, in order.
    ///
    /// The shape is the accumulator, which is what makes the order meaningful: a
    /// physical shaper that carves a `slab` leaves the material shaper below it a
    /// different silhouette to decorate, and both are pure functions of the
    /// context they were handed.
    pub fn apply(&self, ctx: &VoxelContext) -> Refused<VoxelShape> {
        let mut shape = ctx.shape;
        for shaper in &self.shapers {
            shaper.apply(ctx, &mut shape)?;
        }
        Ok(shape)
    }
}

impl core::fmt::Debug for ShaperSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(self.shapers.iter().map(|s| (s.kind().name(), s.name())))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusal::RefusalSet;

    const ROCK: MaterialId = MaterialId(6);
    const SOIL: MaterialId = MaterialId(3);

    fn at(x: u8, y: u8, z: u8) -> SubvoxelIndex {
        SubvoxelIndex::new(x, y, z).unwrap()
    }

    fn ctx(shape: VoxelShape) -> VoxelContext {
        VoxelContext::new([0, 0, 0], shape, Neighbourhood::exposed(), 7, 0)
    }

    /// A physical shaper that empties one cell: the smallest real change.
    struct CarveOne;
    impl PhysicalShaper for CarveOne {
        fn name(&self) -> &'static str {
            "carve-one"
        }
        fn source(&self) -> &'static str {
            "test"
        }
        fn acts_at(&self) -> LevelMask {
            LevelMask::SUBVOXEL
        }

        fn applies(&self, _ctx: &VoxelContext) -> bool {
            true
        }
        fn carve(&self, _ctx: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
            out.carve(at(0, 0, 0));
            Ok(())
        }
    }

    /// A material shaper that re-slots one cell.
    struct SlotOne;
    impl MaterialShaper for SlotOne {
        fn name(&self) -> &'static str {
            "slot-one"
        }
        fn source(&self) -> &'static str {
            "test"
        }
        fn acts_at(&self) -> LevelMask {
            LevelMask::SUBVOXEL
        }

        fn applies(&self, _ctx: &VoxelContext) -> bool {
            true
        }
        fn slot(&self, _ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
            out.set(at(1, 1, 1), SOIL)
        }
    }

    /// A material shaper that tries to fill an empty cell. This is the shape the
    /// type system exists to make impossible; it is written here so the *runtime*
    /// belt is tested too, not just the type.
    struct SneakyFill;
    impl MaterialShaper for SneakyFill {
        fn name(&self) -> &'static str {
            "sneaky-fill"
        }
        fn source(&self) -> &'static str {
            "test"
        }
        fn acts_at(&self) -> LevelMask {
            LevelMask::SUBVOXEL
        }

        fn applies(&self, _ctx: &VoxelContext) -> bool {
            true
        }
        fn slot(&self, _ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
            out.set(at(0, 0, 0), ROCK)
        }
    }

    #[test]
    fn a_physical_shaper_changes_occupancy_and_a_material_shaper_cannot() {
        let solid = VoxelShape::solid(ROCK);

        let carved = ShaperSet::new().with_physical(CarveOne).apply(&ctx(solid)).unwrap();
        assert!(carved.get(at(0, 0, 0)).is_empty());
        assert_eq!(carved.get(at(1, 1, 1)), ROCK);

        let slotted = ShaperSet::new().with_material(SlotOne).apply(&ctx(solid)).unwrap();
        assert_eq!(slotted.get(at(1, 1, 1)), SOIL);
        // The silhouette is **exactly** the one it was handed, cell for cell.
        for cell in SubvoxelIndex::all() {
            let was = solid.get(cell).is_empty();
            let now = slotted.get(cell).is_empty();
            assert_eq!(was, now, "a material shaper changed occupancy at {cell:?}");
        }
    }

    #[test]
    fn a_material_shaper_reaching_for_an_empty_cell_is_refused_by_name() {
        // The type stops the reach; this proves the runtime stops the *route* the
        // type cannot see, and that the refusal says which kind went wrong.
        let mut shape = VoxelShape::empty();
        shape.set(at(1, 1, 1), ROCK);
        let r = ShaperSet::new()
            .with_material(SneakyFill)
            .apply(&ctx(shape))
            .unwrap_err();
        assert_eq!(r.kind(), RefusalKind::ShaperKindViolation);

        // The same shape from the physical side is fine: it is that shaper's job.
        let ok = ShaperSet::new().with_physical(CarveOne).apply(&ctx(shape)).unwrap();
        assert_eq!(ok.get(at(1, 1, 1)), ROCK);
    }

    #[test]
    fn replace_refuses_to_change_occupancy_in_either_direction() {
        let mut cells = [MaterialId::EMPTY; SUBVOXEL_COUNT];
        cells[0] = ROCK;
        {
            let mut w = SlotWriter::new(&mut cells);
            let changed = w.replace(ROCK, SOIL).unwrap();
            assert_eq!(changed, 1);
        }
        assert_eq!(cells[0], SOIL);

        // Emptying a cell through `replace` is a silhouette change and is refused.
        let err = SlotWriter::new(&mut cells).replace(SOIL, MaterialId::EMPTY).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::ShaperKindViolation);
        // So is filling one.
        let err = SlotWriter::new(&mut cells).replace(MaterialId::EMPTY, SOIL).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::ShaperKindViolation);
    }

    #[test]
    fn a_shape_writer_set_cannot_smuggle_an_occupancy_change() {
        let mut shape = VoxelShape::solid(ROCK);
        let mut w = ShapeWriter::new(&mut shape);
        // In place is fine.
        w.set(at(0, 0, 0), SOIL).unwrap();
        assert_eq!(w.get(at(0, 0, 0)), SOIL);
        // Filling an occupied cell is a no-op change of material, allowed.
        w.fill(at(0, 0, 0), ROCK).unwrap();
        // Emptying through `set` is refused; `carve` is the method that means it.
        let err = w.set(at(0, 0, 0), MaterialId::EMPTY).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::ShaperKindViolation);
        // And `fill` may not place the empty id: that would be a carve wearing a
        // fill's name.
        let err = w.fill(at(0, 0, 0), MaterialId::EMPTY).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn the_order_of_the_set_is_the_order_of_the_effects() {
        // Order matters exactly when two shapers **interact**, and the interaction
        // worth pinning is the two kinds meeting: a physical shaper that adds
        // matter to empty cells, and a material shaper that re-slots the matter
        // it finds. Fill-then-slot leaves every new cell with the slotter's
        // material; slot-then-fill leaves the newly filled cells with the
        // filler's, because the slotter never saw them. Both orders are legal and
        // they are different worlds — which is why the set is ordered and not a
        // bag.
        struct FillMissing;
        impl PhysicalShaper for FillMissing {
            fn name(&self) -> &'static str {
                "fill-missing"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::SUBVOXEL
            }

            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn carve(&self, _c: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
                // Read and write in one loop rather than through `fill_where`:
                // the predicate would have to borrow the writer the method has
                // already taken mutably, and the borrow checker refusing that is
                // the right answer.
                for at in SubvoxelIndex::all() {
                    if out.get(at).is_empty() {
                        out.fill(at, ROCK)?;
                    }
                }
                Ok(())
            }
        }
        struct SlotsEverythingOccupied;
        impl MaterialShaper for SlotsEverythingOccupied {
            fn name(&self) -> &'static str {
                "slots-everything-occupied"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::SUBVOXEL
            }

            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn slot(&self, _c: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
                for at in SubvoxelIndex::all() {
                    if out.occupied(at) {
                        out.set(at, SOIL)?;
                    }
                }
                Ok(())
            }
        }

        // Start from a block with exactly one empty cell, so "did the slotter see
        // it" is a single bit.
        let mut start = VoxelShape::solid(ROCK);
        start.set(at(2, 2, 2), MaterialId::EMPTY);
        let c = ctx(start);

        let fill_first = ShaperSet::new()
            .with_physical(FillMissing)
            .with_material(SlotsEverythingOccupied)
            .apply(&c)
            .unwrap();
        let slot_first = ShaperSet::new()
            .with_material(SlotsEverythingOccupied)
            .with_physical(FillMissing)
            .apply(&c)
            .unwrap();

        // Fill-first: the hole was filled with rock, and the slotter then saw it.
        assert_eq!(fill_first.get(at(2, 2, 2)), SOIL);
        assert!(fill_first.cells().iter().all(|m| *m == SOIL));
        // Slot-first: the slotter never saw the hole, so the filler's rock stands.
        assert_eq!(slot_first.get(at(2, 2, 2)), ROCK);
        assert_ne!(fill_first, slot_first);
        // Both ended up fully occupied, which is the physical shaper's doing and
        // is true in both orders.
        assert!(!fill_first.is_empty() && !slot_first.is_empty());
    }

    #[test]
    fn the_same_context_always_produces_the_same_shape() {
        let set = ShaperSet::new()
            .with_physical(CarveOne)
            .with_material(SlotOne);
        let c = ctx(VoxelShape::solid(ROCK));
        assert_eq!(set.apply(&c).unwrap(), set.apply(&c).unwrap());
        // A different tick is a different world, and a different seed too — but
        // these two shapers read neither, which is a property and not an
        // oversight: they are geometry, not weather.
        let mut other = c;
        other.tick = 999;
        other.seed = 4;
        assert_eq!(set.apply(&c).unwrap(), set.apply(&other).unwrap());
    }

    #[test]
    fn a_shaper_without_a_source_stops_the_set_at_audit() {
        struct Unsourced;
        impl PhysicalShaper for Unsourced {
            fn name(&self) -> &'static str {
                "unsourced"
            }
            fn source(&self) -> &'static str {
                "   "
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::SUBVOXEL
            }

            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn carve(&self, _c: &VoxelContext, _out: &mut ShapeWriter<'_>) -> Refused<()> {
                Ok(())
            }
        }
        let set = ShaperSet::new().with_physical(Unsourced).with_material(SlotOne);
        let err: RefusalSet = set.audit().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err.all()[0].detail().contains("unsourced"));
        assert!(err.all()[0].detail().contains("SHAPERS"));

        // And a set that does name its sources audits clean.
        ShaperSet::new()
            .with_physical(CarveOne)
            .with_material(SlotOne)
            .audit()
            .unwrap();
    }

    #[test]
    fn two_shapers_with_one_name_stop_the_set() {
        struct AlsoSlotOne;
        impl MaterialShaper for AlsoSlotOne {
            fn name(&self) -> &'static str {
                "slot-one"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::SUBVOXEL
            }

            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn slot(&self, _c: &VoxelContext, _out: &mut SlotWriter<'_>) -> Refused<()> {
                Ok(())
            }
        }
        let set = ShaperSet::new()
            .with_material(SlotOne)
            .with_material(AlsoSlotOne);
        let err = set.audit().unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(set.kinds(), (0, 2));
    }

    #[test]
    fn a_shaper_that_does_not_apply_never_opens_a_writer() {
        struct NeverApplies;
        impl PhysicalShaper for NeverApplies {
            fn name(&self) -> &'static str {
                "never"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::SUBVOXEL
            }

            fn applies(&self, _c: &VoxelContext) -> bool {
                false
            }
            fn carve(&self, _c: &VoxelContext, _out: &mut ShapeWriter<'_>) -> Refused<()> {
                panic!("applies() returned false and carve was called anyway");
            }
        }
        let set = ShaperSet::new().with_physical(NeverApplies);
        assert_eq!(set.apply(&ctx(VoxelShape::solid(ROCK))).unwrap(), VoxelShape::solid(ROCK));
    }

    #[test]
    fn the_neighbourhood_answers_the_questions_a_shaper_asks() {
        let n = Neighbourhood::exposed();
        assert_eq!(n.open_faces(), 6);
        assert!(!n.has_ceiling());
        assert!(!n.has_floor());

        let mut ceiling = Neighbourhood::exposed();
        ceiling.faces[5] = ROCK; // +z
        let c = VoxelContext::new([0, 1, 2], VoxelShape::solid(ROCK), ceiling, 1, 0);
        assert!(c.has_ceiling());
        assert!(!c.has_floor());
        assert_eq!(c.around.face(Face::PosZ), ROCK);
        assert_eq!(c.around.face(Face::NegZ), MaterialId::EMPTY);

        let buried = Neighbourhood::buried(ROCK);
        assert_eq!(buried.open_faces(), 0);
        assert!(buried.has_ceiling() && buried.has_floor());
    }

    // ------------------------------------------------- the two levels, for real

    #[test]
    fn the_taxonomy_is_two_kinds_crossed_with_two_levels_and_every_cell_is_reachable() {
        // The whole point of `Level` being separate from `ShaperKind`: all four
        // cells must be *expressible*, or the taxonomy is a claim rather than a
        // structure. This builds one shaper per cell and counts them.
        struct PhysicalBoth;
        impl PhysicalShaper for PhysicalBoth {
            fn name(&self) -> &'static str {
                "physical-both"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::BOTH
            }
            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn carve(&self, _c: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
                out.carve(at(0, 0, 0));
                Ok(())
            }
            fn displace(&self, out: &mut DisplacementWriter<'_>) -> Refused<()> {
                out.set_domain(0xD15B);
                out.declare_all_faces(ROCK, 0.25)
            }
        }
        struct MaterialDetail;
        impl MaterialShaper for MaterialDetail {
            fn name(&self) -> &'static str {
                "material-detail"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::DETAIL
            }
            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn slot(&self, _c: &VoxelContext, _out: &mut SlotWriter<'_>) -> Refused<()> {
                Ok(())
            }
            fn shade(&self, out: &mut ShadingWriter<'_>) -> Refused<()> {
                out.set_domain(0x5A4D);
                out.declare(SOIL, 0.5, 0.25)
            }
        }

        let set = ShaperSet::new()
            .with_physical(PhysicalBoth)
            .with_material(SlotOne)
            .with_material(MaterialDetail);
        let tally = set.tally();
        assert_eq!(tally.physical_subvoxel, 1);
        assert_eq!(tally.physical_detail, 1);
        assert_eq!(tally.material_subvoxel, 1);
        assert_eq!(tally.material_detail, 1);
        assert_eq!(tally.occupied_cells(), 4, "all four (kind, level) cells are in use");
        assert_eq!(tally.total(), 4);

        let (displacement, shading) = set.declare().unwrap();
        assert_eq!(displacement.domain(), 0xD15B);
        assert_eq!(shading.domain(), 0x5A4D);
        assert_eq!(displacement.materials(), 1);
        assert_eq!(shading.len(), 1);
        assert_eq!(displacement.amplitude(ROCK, Face::PosZ), Some(0.25));
        assert_eq!(displacement.amplitude(ROCK, Face::NegX), Some(0.25));
        assert_eq!(displacement.amplitude(SOIL, Face::PosZ), None);
        assert_eq!(shading.gain(SOIL), Some((0.5, 0.25)));
        assert_eq!(shading.gain(ROCK), None);
        // And the 4x4x4 half still runs.
        assert!(set.apply(&ctx(VoxelShape::solid(ROCK))).unwrap().get(at(0, 0, 0)).is_empty());
    }

    #[test]
    fn a_shaper_that_claims_the_256_level_and_declares_nothing_is_refused_by_name() {
        // The check that makes `acts_at` mean something. Without it the level is a
        // comment, and a world would report "4 of 4 cells in use" while doing
        // nothing at all at 256.
        struct ClaimsDetailDoesNothing;
        impl PhysicalShaper for ClaimsDetailDoesNothing {
            fn name(&self) -> &'static str {
                "claims-detail"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::DETAIL
            }
            fn applies(&self, _c: &VoxelContext) -> bool {
                true
            }
            fn carve(&self, _c: &VoxelContext, _out: &mut ShapeWriter<'_>) -> Refused<()> {
                Ok(())
            }
        }
        let set = ShaperSet::new().with_physical(ClaimsDetailDoesNothing);
        let err = set.declare().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err.all()[0].detail().contains("claims-detail"));
        assert!(err.all()[0].detail().contains("256"));

        // And a shaper that declares no level at all is refused too, because a
        // shaper that acts nowhere is not a shaper.
        struct ActsNowhere;
        impl PhysicalShaper for ActsNowhere {
            fn name(&self) -> &'static str {
                "acts-nowhere"
            }
            fn source(&self) -> &'static str {
                "test"
            }
            fn acts_at(&self) -> LevelMask {
                LevelMask::NONE
            }
            fn applies(&self, _c: &VoxelContext) -> bool {
                false
            }
        }
        let err = ShaperSet::new().with_physical(ActsNowhere).declare().unwrap_err();
        assert!(err.all()[0].detail().contains("acts nowhere"));
    }

    #[test]
    fn a_displacement_may_not_cross_a_cell_boundary_and_the_refusal_says_why() {
        // Half a subvoxel is the bound. Past it the surface would leave its own
        // cell, and the neighbour would have to move to stay connected — which is
        // a carve, and a carve belongs to the 4x4x4 level. The refusal names that
        // rather than clamping, because a clamp would silently make a world that
        // is not the one that was asked for.
        let mut field = DisplacementField::none();
        // Both signs of the bound are inside it — the bound is on the magnitude.
        assert!(field.declare(ROCK, Face::PosZ, MAX_DISPLACEMENT_SUBVOXELS).is_ok());
        assert!(field.declare(ROCK, Face::NegZ, -MAX_DISPLACEMENT_SUBVOXELS).is_ok());
        let err = field.declare(ROCK, Face::PosX, 0.5000001).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("carve"));
        let err = field.declare(SOIL, Face::PosZ, f64::NAN).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::NonFiniteValue);
        let err = field.declare(MaterialId::EMPTY, Face::PosZ, 0.1).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn two_shapers_may_not_displace_one_face_and_the_second_is_refused() {
        // A silent last-write-wins here would make the world depend on the order
        // of two shapers nobody declared an order between.
        let mut field = DisplacementField::none();
        field.declare(ROCK, Face::PosZ, 0.1).unwrap();
        let err = field.declare(ROCK, Face::PosZ, 0.2).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("already moves"));
        // A different face of the same material is a different key and is fine.
        assert!(field.declare(ROCK, Face::NegZ, 0.2).is_ok());
        assert_eq!(field.len(), 2);
        // One material, two faces: still one material.
        assert_eq!(field.materials(), 1);
        // And the entries enumerate in declaration order with the faces resolved.
        let entries: Vec<(MaterialId, Face, f64)> = field
            .entries()
            .map(|e| (e.material, e.face, e.amplitude_subvoxels))
            .collect();
        assert_eq!(entries[0], (ROCK, Face::PosZ, 0.1));
        assert_eq!(entries[1], (ROCK, Face::NegZ, 0.2));
        // Neither was declared with a law, so neither carries a shape.
        assert_eq!(field.profiled_faces(), 0);
    }

    #[test]
    fn a_shading_gain_outside_the_unit_range_is_refused() {
        let mut field = ShadingField::none();
        assert!(field.declare(ROCK, 1.0, 0.0).is_ok());
        assert_eq!(field.declare(ROCK, 0.5, 0.5).unwrap_err().kind(), RefusalKind::OutOfDeclaredRange);
        for bad in [-0.1, 1.1, f64::INFINITY] {
            let err = field.declare(SOIL, bad, 0.5).unwrap_err();
            assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        }
        let err = field.declare(SOIL, 0.5, f64::NAN).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert_eq!(field.declare(MaterialId::EMPTY, 0.5, 0.5).unwrap_err().kind(), RefusalKind::UnadmittedMaterial);
        assert_eq!(field.gain(ROCK), Some((1.0, 0.0)));
        assert_eq!(field.len(), 1);
    }

    #[test]
    fn the_level_mask_is_a_mask_and_not_two_booleans_that_can_disagree() {
        assert!(!LevelMask::NONE.has(Level::Subvoxel));
        assert!(!LevelMask::NONE.has(Level::Detail));
        assert_eq!(LevelMask::NONE.count(), 0);
        for (mask, sub, det, count) in [
            (LevelMask::SUBVOXEL, true, false, 1),
            (LevelMask::DETAIL, false, true, 1),
            (LevelMask::BOTH, true, true, 2),
        ] {
            assert_eq!(mask.has(Level::Subvoxel), sub);
            assert_eq!(mask.has(Level::Detail), det);
            assert_eq!(mask.count(), count);
        }
        assert_eq!(LevelMask::SUBVOXEL.union(LevelMask::DETAIL), LevelMask::BOTH);
        // The levels' own sides are the settled scale contract, not new numbers.
        assert_eq!(Level::Subvoxel.side(), 4);
        assert_eq!(Level::Detail.side(), 16);
        assert_eq!(Level::Subvoxel.name(), "4x4x4");
        assert_eq!(Level::Detail.name(), "256");
    }

    #[test]
    fn a_profile_displaces_with_its_own_shape_and_derives_its_own_amplitude() {
        // The 256 level carrying a **law** and not a number, and **two bounds** because
        // they answer two questions. A hemisphere of radius 0.5 block has an apex of
        // `cap_height(0) = 0.5` block = **2 subvoxels**, and it is *accepted*: half a
        // block is the tallest cap a block can hold, because a cross-section may not
        // exceed the circle inscribed in it. The first draft shared one bound between a
        // bare trim and a profile and refused this — which would have refused every
        // domed pillar in the world.
        let mut field = DisplacementField::none();
        let largest = Profile::RoundColumn { radius: crate::profile::MAX_RADIUS };
        field.declare_profile(ROCK, Face::PosZ, largest).unwrap();
        assert_eq!(field.amplitude(ROCK, Face::PosZ), Some(2.0));
        assert_eq!(MAX_PROFILE_SUBVOXELS, 2.0);

        // Past that it is not a shape the block holds, and it is refused by name rather
        // than scaled down to one that fits.
        let mut field = DisplacementField::none();
        let too_tall = Profile::Spheroid { radius: 0.6, height: 0.6 };
        let err = field.declare_profile(ROCK, Face::PosZ, too_tall).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("inscribed"), "{}", err.detail());

        // A **bare** amplitude is the other question, and keeps the other bound: it is a
        // trim of a surface somebody else shaped, so past half a subvoxel the cell next
        // door would have to move to stay connected — a carve.
        let mut field = DisplacementField::none();
        let err = field.declare(ROCK, Face::PosZ, 0.75).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("carve"), "{}", err.detail());

        // A profile small enough to be a relief is accepted, and its displacement
        // is the profile's own surface: zero at the face's centre, its apex height
        // in subvoxels at the axis, and *falling* outward from there.
        let mut field = DisplacementField::none();
        let dome = Profile::Spheroid { radius: 0.12, height: 0.12 };
        field.declare_profile(ROCK, Face::PosZ, dome).unwrap();
        assert_eq!(field.profiled_faces(), 1);
        let apex = field.displacement_at(ROCK, Face::PosZ, 0.5, 0.5).unwrap().unwrap();
        assert!((apex - 0.12 * 4.0).abs() < 1e-12, "apex was {apex}");
        let quarter = field
            .displacement_at(ROCK, Face::PosZ, 0.5 + 0.05, 0.5)
            .unwrap()
            .unwrap();
        assert!(quarter < apex && quarter > 0.0);
        let beyond = field
            .displacement_at(ROCK, Face::PosZ, 1.0, 1.0)
            .unwrap()
            .unwrap();
        assert_eq!(beyond, 0.0, "outside the body there is no surface");
        // The coordinate is refused outside the face rather than clamped.
        assert!(field.displacement_at(ROCK, Face::PosZ, 1.2, 0.5).unwrap().is_err());
        // And a face that does not move answers `None` rather than zero.
        assert!(field.displacement_at(ROCK, Face::NegZ, 0.5, 0.5).is_none());
        // The amplitude is the derived apex, in subvoxels.
        assert_eq!(field.amplitude(ROCK, Face::PosZ), Some(0.12 * 4.0));

        // A bare magnitude keeps the profile slot empty and returns its own value.
        let mut bare = DisplacementField::none();
        bare.declare(ROCK, Face::PosX, -0.2).unwrap();
        assert_eq!(bare.profiled_faces(), 0);
        assert_eq!(bare.displacement_at(ROCK, Face::PosX, 0.9, 0.1).unwrap().unwrap(), -0.2);
        assert_eq!(bare.entry(ROCK, Face::PosX).unwrap().profile, None);
    }
}
