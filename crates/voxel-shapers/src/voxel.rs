//! The voxel shapers: how a **block** becomes a **shape**.
//!
//! [`voxel_model::shaper`] holds the two kinds and the two levels as types. This
//! module holds the shapes, and — per `docs/SHAPERS.md` §2 — **every one of them
//! names the source it is built from**, because a shaper that is an unexplained pile
//! of noise is not allowed to exist. `LEDGER.md` C-42 is the source list.
//!
//! # Why this module was the missing half of the world model
//!
//! Until it existed, `voxel-stream`'s `WorldSource::shape_at` answered
//! `VoxelShape::solid(m)` for **every voxel in the world**. The 4×4×4 level was
//! therefore present, tested, meshed at 128³ — and completely empty of content:
//! every block was a cube of one material. Closing that is not a rendering problem
//! and not a two-shape problem. It is a **roster** problem, and the roster is what
//! C-42 is about.
//!
//! # The shapes, by family, with their laws
//!
//! ## Cuts — parametric, and at both levels
//!
//! [`Cut`] is a **set of kept bands** on an axis, positioned in **256ths of a
//! block**, so a cut is neither confined to a 25/50/75 % plane nor limited to one
//! plane. `Cut::planking` keeps `n` evenly-thick bands, which is a plank stack;
//! `Cut::keep` keeps an arbitrary interval, which is an off-centre third; and
//! `Cut::discard` is the complement, which is a trough cut into a floor.
//!
//! | Shaper | Kind | Level | What it does |
//! |---|---|---|---|
//! | [`Slab`] | physical | **both** | carves the cells outside the bands, and **displaces** the survivors to the exact plane |
//! | [`LoopCut`] | material | 4×4×4 | seams the cutting layer, silhouette untouched |
//!
//! `Slab` is the clearest demonstration of why the 256 level had to be real. The
//! stored grid can only put a cut plane on a *subvoxel boundary* — four positions
//! per block. The **declared displacement** is what puts it on an arbitrary
//! position: its amplitude is the **residual** between the band edge and the
//! nearest cell boundary, which is *derived* from the cut and chosen by nobody. A
//! cut at 96/256 of a block is a real cut at 96/256, not a rounded one.
//!
//! ## Bodies — one profile, every round shape
//!
//! [`Column`] is the general **body** shaper: an axis, a [`Profile`](voxel_model::Profile),
//! the body's own material, an inset material, and a mode. A cell is inside the body
//! when its **centre's distance from the body's axis** is within the profile's radius
//! at that cell's axial position. Material mode re-slots the cells outside; physical
//! mode carves them.
//!
//! | Shape | Profile | Source |
//! |---|---|---|
//! | [`Column::pillar`] | `RoundColumn` | Blinn 1978 (a shading normal is perturbed independently of geometry); Catmull & Clark 1978 (the rounded-cube limit surface); Maekawa 1999 (a shaft is an offset of a circle) |
//! | [`Column::stalagmite`] | `Stalagmite { da, radius }` | **Szymczak, Ladd, Lipar & Pekarovič 2025**, *Shapes of ideal stalagmites*, PNAS 122(42) e2513263122 — the tangent law `cos θ(r) = Da·√(1 − (r/R)²)`, one parameter, three forms |
//! | [`Column::stalactite`] | `Frustum`, thin end down | **Short, Baygents & Goldstein 2005**, *Stalactite growth as a free-boundary problem*, PRL 94 018501 — the geometric law and its platonic ideal. **Not implemented**: the exact free-boundary ideal. What is here is a tapering body with an **input** tip radius, and the gap is stated rather than dressed |
//! | [`Column::cave_column`] | `Frustum`, thin in the middle | NPS speleothem morphology; Hill & Forti (1997), *Cave Minerals of the World* — a column is a stalactite and a stalagmite that have met, so its waist is where they joined |
//! | [`Boulder`] | `Spheroid` | **Zingg 1935** (form from three axis ratios); **Barrett 1980**, *The shape of rock particles, a critical review*, Sedimentology 27:291–303 (form, roundness and texture are **three independent axes**) |
//!
//! ## The 256 level on its own
//!
//! | Shaper | Kind | Source |
//! |---|---|---|
//! | [`Flowstone`] | physical @ **256 only** | **Camporeale et al. 2012**, *Hydrodynamic-Driven Stability Analysis of Morphological Patterns on Speleothems*, PRL 108 238501 — the ripples on flowstone are a **hydrodynamic instability** with a preferred wavelength, not noise |
//! | [`Coral`] | physical | the standard reef-survey **growth-form** list (branching, columnar, encrusting, massive, laminar/tabulate) with the **corallite** diameter as the species-scale constant |
//!
//! # What is deliberately not here
//!
//! * **Bladed** clasts (Zingg's fourth class) are not a surface of revolution and
//!   are refused rather than approximated.
//! * The **stalactite's** exact platonic ideal is not implemented; see above.
//! * Neither the stalagmite's nor the stalactite's shape has been **looked at**.
//!   Per `LEDGER.md` C-40 that is stated rather than implied, and it is the largest
//!   debt in this file.

use voxel_model::shaper::{
    DisplacementWriter, LevelMask, MaterialShaper, Neighbourhood, PhysicalShaper, ShapeWriter,
    ShaperSet, SlotWriter, VoxelContext,
};
use voxel_model::{
    Face, MaterialId, Profile, Refusal, RefusalKind, Refused, SubvoxelIndex, VoxelShape,
    SUBVOXEL_SIDE,
};

// ===========================================================================
// The axis
// ===========================================================================

/// One of the world's three axes. Used for a cut plane and for a body's axis, and
/// named `Z` rather than "horizontal" because both uses need the same three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub const fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    /// The other two axes, in ascending index order. The cross-section of a body
    /// whose axis is `self` lies in these two.
    pub const fn others(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::X, Axis::Z),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }

    /// The two faces a cut on this axis moves between: the low one and the high one.
    pub const fn faces(self) -> (Face, Face) {
        match self {
            Axis::X => (Face::NegX, Face::PosX),
            Axis::Y => (Face::NegY, Face::PosY),
            Axis::Z => (Face::NegZ, Face::PosZ),
        }
    }

    fn coord(self, at: SubvoxelIndex) -> u8 {
        match self {
            Axis::X => at.x,
            Axis::Y => at.y,
            Axis::Z => at.z,
        }
    }
}

/// A subvoxel cell's coordinate along an axis.
fn coord_of(at: SubvoxelIndex, axis: Axis) -> u8 {
    axis.coord(at)
}

/// A cell's centre in block units along one axis: `0.125, 0.375, 0.625, 0.875`.
pub fn cell_centre(c: u8) -> f64 {
    (c as f64 + 0.5) / SUBVOXEL_SIDE as f64
}

/// A cell centre's offset from the block's centre, `−0.375 … +0.375`.
fn centred(c: u8) -> f64 {
    cell_centre(c) - 0.5
}

/// How far a cell's centre is from a body's axis.
pub fn radial_offset(at: SubvoxelIndex, axis: Axis) -> f64 {
    let (a, b) = axis.others();
    let da = centred(coord_of(at, a));
    let db = centred(coord_of(at, b));
    (da * da + db * db).sqrt()
}

/// The axial fraction a cell sits at, in `0..1` — what selects a profile's radius.
pub fn axial_fraction(at: SubvoxelIndex, axis: Axis) -> f64 {
    cell_centre(coord_of(at, axis))
}

// ===========================================================================
// The cut, parametric
// ===========================================================================

/// A cut position is in **256ths of a block**, which is the addressed level's own
/// side. Using the detail lattice's resolution as the cut's resolution is not a
/// coincidence: it is the resolution at which a cut can actually be *realised*
/// ([`Slab`]'s displacement), so a finer coordinate would be a claim the geometry
/// could not keep.
pub const CUT_SCALE: u16 = 256;

/// The 256ths one cell of the stored lattice spans. The cut's edges are read against
/// this grid, and the residual that becomes a displacement is measured against it.
pub const PER_CELL: u16 = CUT_SCALE / SUBVOXEL_SIDE as u16;

/// Half a cell, in 256ths: the one threshold the two levels agree on, because it is
/// the half-subvoxel the addressed level can move a face by.
pub const HALF_CELL: u16 = PER_CELL / 2;

/// **One face's share of a cut**, in subvoxels: which cell, which face of it, and how
/// far that face moves.
///
/// Positive is outward, so a cut's trims are negative where the surface is *shaved*
/// inward and positive where it has to *grow* out to an edge that fell past the cell.
/// Carrying the sign and the face is what makes a trim a thing the refinement stage
/// can execute rather than a magnitude somebody has to interpret.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Trim {
    /// The cell whose face moves, as an index on the cut's axis.
    pub cell: u8,
    /// Which face of it.
    pub face: Face,
    /// Signed displacement in subvoxels, positive outward.
    pub displacement_subvoxels: f64,
}

impl Trim {
    /// The distance the face moves, without its direction.
    pub fn magnitude(&self) -> f64 {
        self.displacement_subvoxels.abs()
    }
}

/// The 256ths of a cell a band covers.
fn coverage(band: &Band, cell_low: u16, cell_high: u16) -> u16 {
    band.high
        .min(cell_high)
        .saturating_sub(band.low.max(cell_low))
}

/// A kept interval along an axis, in 256ths, low-inclusive high-exclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Band {
    pub low: u16,
    pub high: u16,
}

impl Band {
    pub fn new(low: u16, high: u16) -> Refused<Self> {
        if low >= high {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a band of [{low}, {high}) is empty; a kept interval must have positive width"),
            ));
        }
        if high > CUT_SCALE {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a band reaching {high} exceeds the block's {CUT_SCALE}"),
            ));
        }
        Ok(Band { low, high })
    }

    pub const fn width(self) -> u16 {
        self.high - self.low
    }

    pub fn contains(self, position: u16) -> bool {
        position >= self.low && position < self.high
    }
}

/// **A cut is a set of kept bands**, not one plane.
///
/// The first attempt at this was `layers ∈ {1,2,3}` of four — a hardcoded 25 %,
/// 50 % or 75 % and nothing else, and one plane at most. That is a claim a player
/// would hit within a minute (C-41 correction 2). This type is the replacement, and
/// the three things it has to be able to say are:
///
/// * an **arbitrary position**, because a cut at 96/256 is where somebody wanted it;
/// * a **band**, because a slab that keeps a middle slice is two cuts;
/// * **several bands**, because a stack of planks is a floor.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cut {
    axis: Axis,
    bands: Vec<Band>,
}

impl Cut {
    /// Keep exactly one interval. `Cut::keep(Axis::Z, 0, 96)` is a slab three
    /// eighths of a block tall, at an arbitrary eighth.
    pub fn keep(axis: Axis, low: u16, high: u16) -> Refused<Self> {
        Ok(Cut { axis, bands: vec![Band::new(low, high)?] })
    }

    /// **Discard** one interval: the complement, as one or two bands. This is the
    /// cut somebody makes *into* something — a gutter down a block, a notch in a top.
    pub fn discard(axis: Axis, low: u16, high: u16) -> Refused<Self> {
        let hole = Band::new(low, high)?;
        let mut bands = Vec::new();
        if hole.low > 0 {
            bands.push(Band { low: 0, high: hole.low });
        }
        if hole.high < CUT_SCALE {
            bands.push(Band { low: hole.high, high: CUT_SCALE });
        }
        if bands.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "discarding the whole block is not a cut; it is an empty shape",
            ));
        }
        Ok(Cut { axis, bands })
    }

    /// The 50 % cut, as a named convenience. It is *one* cut among many now and no
    /// longer the only one this type can express.
    pub fn half(axis: Axis) -> Self {
        Cut { axis, bands: vec![Band { low: 0, high: CUT_SCALE / 2 }] }
    }

    /// A horizontal slab: `keep` along `Z`, the axis a player thinks of as up.
    pub fn slab(low: u16, high: u16) -> Refused<Self> {
        Self::keep(Axis::Z, low, high)
    }

    /// **A stack of planks.** `count` bands of `thickness` 256ths, evenly spaced
    /// across the block, starting at the low end. This is the shape a cut *is* in a
    /// floor, a ladder or a bookshelf, and it is the case one plane cannot express.
    pub fn planking(axis: Axis, count: u8, thickness: u16) -> Refused<Self> {
        if count == 0 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a plank stack of zero planks keeps nothing",
            ));
        }
        let pitch = CUT_SCALE / count as u16;
        if thickness == 0 || thickness > pitch {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a plank {thickness} thick in a pitch of {pitch} does not fit; it would overlap the next plank"
                ),
            ));
        }
        let mut bands = Vec::with_capacity(count as usize);
        for i in 0..count as u16 {
            let low = i * pitch;
            let high = low + thickness;
            bands.push(Band::new(low, high)?);
        }
        Ok(Cut { axis, bands })
    }

    pub const fn axis(&self) -> Axis {
        self.axis
    }

    pub fn bands(&self) -> &[Band] {
        &self.bands
    }

    /// The fraction of the block's cross-section this cut keeps.
    pub fn kept_fraction(&self) -> f64 {
        self.bands.iter().map(|b| b.width() as f64).sum::<f64>() / CUT_SCALE as f64
    }

    /// True when the cut keeps everything — which happens only for a band spanning
    /// the block, and is worth being able to ask because such a cut is a no-op.
    pub fn is_uniform(&self) -> bool {
        self.bands.len() == 1 && self.bands[0].low == 0 && self.bands[0].high == CUT_SCALE
    }

    /// Whether a cell survives: **a band must cover at least half of it**.
    ///
    /// Half is not a chosen threshold; it is the only one the two levels can agree
    /// on. A displacement moves a face by half a subvoxel at most, so a cell can be
    /// kept *and* trimmed to a band edge only while that edge lies within half a cell
    /// of the cell's own face — and "within half a cell of its own face" is exactly
    /// "the band covers at least half of the cell". A cell that only a sliver of the
    /// band reaches is a cell whose trim would exceed half a subvoxel, and
    /// [`Cut::trims`] refuses that cut rather than keeping the cell and lying about
    /// where its surface is.
    ///
    /// A **tie keeps the cell**. A band covering exactly half a cell is one whose
    /// edge lands on the cell's own middle, and the trim it needs is exactly the
    /// half-subvoxel maximum: the cell just below the cut and the cell just above it
    /// are both kept, and they shave toward each other until they meet at the plane.
    pub fn survives(&self, at: SubvoxelIndex) -> bool {
        let c = self.axis.coord(at) as u16;
        let (cell_low, cell_high) = (c * PER_CELL, (c + 1) * PER_CELL);
        self.bands.iter().any(|b| coverage(b, cell_low, cell_high) >= HALF_CELL)
    }

    /// The cell indices that survive, in ascending order.
    pub fn surviving_cells(&self) -> Vec<u8> {
        (0..SUBVOXEL_SIDE as u8).filter(|c| self.survives(cell_at(self.axis, *c))).collect()
    }

    /// **What the cut asks of the addressed level**: for every band edge, the one
    /// face of one kept cell that has to move, and by how much.
    ///
    /// The rule is the **nearest face** rule, and it is forced rather than chosen: a
    /// face may move half a subvoxel and no more, so an edge trims **the face nearest
    /// to it on the side the material is** — normally the kept cell's own face, and
    /// when the edge has fallen past that cell's middle, the neighbouring cell's face
    /// by the complement. That is why the residual is measured from a *cell face* and
    /// not from the block: it is what lets one declaration cover a stack of planks
    /// that all sit off the same half-cell grid.
    ///
    /// | the edge is | nearest face | cell | displacement |
    /// |---|---|---|---|
    /// | a band's low edge, within half a cell of the cell's low bound | the cell's negative face | the cell | shaved **in** by `o` |
    /// | a band's low edge, past the middle | the next cell's negative face | the next cell | grown **out** by `1 − o` |
    /// | a band's high edge, past the middle | the cell's positive face | the cell | shaved **in** by `1 − o` |
    /// | a band's high edge, within half a cell of the cell's low bound | the previous cell's positive face | the previous cell | grown **out** by `o` |
    ///
    /// where `o` is the edge's offset inside its cell, in cells, and "out" is
    /// positive — the sign [`DisplacementField`] uses. On an exact half-cell edge both
    /// edges take the containing cell, which is the same tie [`Cut::survives`] makes.
    ///
    /// A band **thinner than half a cell** is refused **by name**: it covers no cell
    /// by half, so there is no cell to hold it, and the face that would trim it would
    /// have to move further than the level allows. Rounding it instead would be a
    /// quietly different shape than the one that was asked for.
    pub fn trims(&self) -> Refused<Vec<Trim>> {
        let (low_face, high_face) = self.axis.faces();
        let mut out = Vec::new();
        for band in &self.bands {
            for (edge, kept_above) in [(band.low, true), (band.high, false)] {
                if edge == 0 || edge == CUT_SCALE {
                    // A block's own bound is not a cut, and there is no face there to
                    // move.
                    continue;
                }
                let cell = (edge / PER_CELL) as i32;
                let offset = (edge % PER_CELL) as f64 / PER_CELL as f64;
                // Both ties take the cell the edge is *inside*, so a half-cell cut
                // keeps both of its cells and shaves them toward each other.
                let (index, face, displacement) = if kept_above {
                    if offset <= 0.5 {
                        (cell, low_face, -offset)
                    } else {
                        (cell + 1, low_face, 1.0 - offset)
                    }
                } else if offset >= 0.5 {
                    (cell, high_face, -(1.0 - offset))
                } else {
                    (cell - 1, high_face, offset)
                };
                if displacement == 0.0 {
                    // An edge that lands exactly on a cell face asks for nothing, and a
                    // declaration nothing needs is a claim that cannot be falsified.
                    continue;
                }
                let in_block = (0..SUBVOXEL_SIDE as i32).contains(&index);
                let half = in_block
                    && coverage(
                        band,
                        index as u16 * PER_CELL,
                        (index as u16 + 1) * PER_CELL,
                    ) >= HALF_CELL;
                if !half {
                    return Err(Refusal::owned(
                        RefusalKind::OutOfDeclaredRange,
                        format!(
                            "the cut's edge at {edge}/{CUT_SCALE} leaves a band of {} that covers less than half a cell ({HALF_CELL}/{CUT_SCALE}) where it lands; a kept cell needs half a cell of band to sit in, and a face moves half a subvoxel at most, so the sliver is refused rather than rounded to a shape nobody asked for",
                            band.width()
                        ),
                    ));
                }
                out.push(Trim { cell: index as u8, face, displacement_subvoxels: displacement });
            }
        }
        Ok(out)
    }

    /// The largest amount any edge of this cut asks a face to move, in **subvoxels**
    /// — one cell is one subvoxel — measured from the nearest cell boundary, so it
    /// never exceeds [`MAX_DISPLACEMENT_SUBVOXELS`].
    ///
    /// A property of the cut alone, and therefore infallible: whether the cut can be
    /// *realised* is [`Cut::trims`]'s question, and they are different questions. A
    /// cut that lands on the grid has no residual and must declare nothing, because a
    /// declaration nothing needs is a claim that cannot be falsified.
    pub fn max_residual(&self) -> f64 {
        let per = PER_CELL as f64;
        self.bands
            .iter()
            .flat_map(|b| [b.low, b.high])
            .filter(|edge| *edge != 0 && *edge != CUT_SCALE)
            .map(|edge| {
                let offset = (edge % PER_CELL) as f64;
                offset.min(per - offset) / per
            })
            .fold(0.0, f64::max)
    }

    /// True when any band edge is off the cell grid — the cut has addressed work to
    /// do, and it is the question a shaper's `acts_at` has to answer without
    /// refusing.
    pub fn is_parametric(&self) -> bool {
        self.max_residual() > 0.0
    }

    /// The cells a cut **seams**: for every band edge that is not the block's own
    /// bound, the surviving cell that carries it — the cell **above** a low edge and
    /// the cell **below** a high edge.
    ///
    /// Those are the cells the plane actually passes through, which is what a seam
    /// marks, and it is the same attribution [`Cut::trims`] moves a face on. A cut
    /// into the middle of a block seams twice: once on each wall of the hole.
    pub fn seam_cells(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for band in &self.bands {
            for (edge, above) in [(band.low, true), (band.high, false)] {
                if edge == 0 || edge == CUT_SCALE {
                    continue;
                }
                let cell = if above {
                    (edge / PER_CELL) as u8
                } else {
                    (edge.saturating_sub(1) / PER_CELL) as u8
                };
                if self.survives(cell_at(self.axis, cell)) && !out.contains(&cell) {
                    out.push(cell);
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// A synthetic cell index on an axis, so a cut can ask its own question without a
/// caller building a `SubvoxelIndex`.
fn cell_at(axis: Axis, c: u8) -> SubvoxelIndex {
    let mut at = SubvoxelIndex { x: 0, y: 0, z: 0 };
    match axis {
        Axis::X => at.x = c,
        Axis::Y => at.y = c,
        Axis::Z => at.z = c,
    }
    at
}

// ===========================================================================
// Slab — physical, both levels
// ===========================================================================

/// **Physical.** Cut a block down to the bands a [`Cut`] keeps, and **displace**
/// the survivors' surfaces to the exact plane.
///
/// The two halves of this shaper are the two levels answering one question:
///
/// * the **4×4×4** half carves the cells whose centres fell outside the bands, so
///   the silhouette becomes a slab of roughly the right thickness;
/// * the **256** half declares a displacement whose amplitude is the **residual**
///   between the band edge and the nearest cell boundary — so the surface ends up
///   on the edge the cut named, and a cut at 96/256 is at 96/256.
///
/// Nothing here rounds the cut to a quarter. The stored grid's four positions are a
/// *sampling* limit, and the addressed level is what removes it — which is C-41
/// corrections 1 and 2 answered by one mechanism.
#[derive(Clone, PartialEq, Debug)]
pub struct Slab {
    pub cut: Cut,
    /// The material of the cells that survive, which is the key its displacement is
    /// declared under. A cut knows what it is cutting.
    pub rock: MaterialId,
}

impl Slab {
    pub fn new(cut: Cut, rock: MaterialId) -> Refused<Self> {
        if rock.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "a slab needs the material of what it cuts; the empty id has no surface",
            ));
        }
        Ok(Slab { cut, rock })
    }
}

impl PhysicalShaper for Slab {
    fn name(&self) -> &'static str {
        "slab"
    }

    fn source(&self) -> &'static str {
        "S-16 (fourteen parametric primitives) and S-14 (press and saw, as a shaper tool) in docs/design/slot-dispositions.jsonl R21; Requicha 1980, Representations for Rigid Solids, ACM Computing Surveys 12(4) — a slab is a cube intersected with an axis-aligned half-space, and a band is the intersection of two"
    }

    fn acts_at(&self) -> LevelMask {
        // Derived from the cut, and that derivation is a finding. Both levels are at
        // work when the plane is off the grid — the stored level carves the cells and
        // the addressed level makes the plane exact — but a cut that lands on the
        // grid is already exact at 4×4×4, and claiming `Detail` for it would be a
        // claim with nothing behind it. It *was* claimed unconditionally, and
        // `ShaperSet::declare` refused the shaper by name for it, which is the check
        // working exactly as intended.
        if self.cut.is_parametric() {
            LevelMask::BOTH
        } else {
            LevelMask::SUBVOXEL
        }
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        !ctx.shape.is_empty()
    }

    fn carve(&self, _ctx: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
        let cut = &self.cut;
        out.carve_where(|at| !cut.survives(at));
        Ok(())
    }

    fn displace(&self, out: &mut DisplacementWriter<'_>) -> Refused<()> {
        // Every trim the cut asks for, at its own amplitude and its own sign: the
        // residual *is* the displacement, in subvoxels, and nobody chooses it.
        let trims = self.cut.trims()?;
        if trims.is_empty() {
            return Ok(());
        }
        out.set_domain(CUT_DOMAIN ^ self.cut.axis.index() as u64);
        // One declaration per face. The field is a whole-world statement keyed by
        // material and face — not a per-cell value — so two edges that want *different*
        // amounts on one face cannot both be honest, and the second is refused rather
        // than allowed to quietly overlap the first. (Two edges that agree, which is
        // what a plank stack off one grid produces, declare once.)
        let mut declared: Vec<(Face, f64)> = Vec::new();
        for trim in &trims {
            match declared.iter().find(|(face, _)| *face == trim.face) {
                Some((_, seen)) if (*seen - trim.displacement_subvoxels).abs() > f64::EPSILON => {
                    return Err(Refusal::owned(
                        RefusalKind::OutOfDeclaredRange,
                        format!(
                            "this cut wants two different displacements on one face ({seen} and {}) subvoxels; a displacement field is keyed by material and face, so it can carry one — align the cut or split it under another domain",
                            trim.displacement_subvoxels
                        ),
                    ));
                }
                Some(_) => {}
                None => {
                    declared.push((trim.face, trim.displacement_subvoxels));
                    out.declare(self.rock, trim.face, trim.displacement_subvoxels)?;
                }
            }
        }
        Ok(())
    }
}

/// The domain tag `Slab`'s displacement is folded under, so a cut's relief can
/// never be the same field as another shaper's.
const CUT_DOMAIN: u64 = 0x5348_504C_4F49_4E47;

// ===========================================================================
// LoopCut — material
// ===========================================================================

/// **Material.** The same [`Cut`], as a **seam**: the surviving cells adjacent to
/// each band edge are re-slotted, and nothing is carved.
///
/// This is the pair that shows the two kinds side by side — identical cut, identical
/// axis, identical position, and one of them moves the silhouette while the other
/// cannot. It is also the honest reading of the words "loop cut", which in a mesh
/// editor *subdivides and does not delete*.
///
/// Strictly gated to exposed voxels: a seam inside solid rock is a material change no
/// pixel can see, and writing it would cost sixteen cell writes per buried voxel for
/// a difference that could only ever be a digest mismatch.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LoopCut {
    pub cut: Cut,
    pub seam: MaterialId,
}

impl LoopCut {
    pub fn new(cut: Cut, seam: MaterialId) -> Refused<Self> {
        if seam.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "a loop-cut seam cannot be the empty id: a seam is a material difference, and the empty id is no material at all",
            ));
        }
        Ok(LoopCut { cut, seam })
    }
}

impl MaterialShaper for LoopCut {
    fn name(&self) -> &'static str {
        "loop-cut"
    }

    fn source(&self) -> &'static str {
        "S-14 (press and saw, as a shaper tool) in docs/design/slot-dispositions.jsonl R21; Requicha 1980, Representations for Rigid Solids — the cut plane of a half-space intersection, expressed at the material level"
    }

    fn acts_at(&self) -> LevelMask {
        LevelMask::SUBVOXEL
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        ctx.around.open_faces() > 0 && !ctx.shape.is_empty()
    }

    fn slot(&self, _ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
        for cell in self.cut.seam_cells() {
            let axial = cell_at(self.cut.axis(), cell);
            for at in SubvoxelIndex::all() {
                if coord_of(at, self.cut.axis()) == coord_of(axial, self.cut.axis())
                    && out.occupied(at)
                {
                    out.set(at, self.seam)?;
                }
            }
        }
        Ok(())
    }
}

// ===========================================================================
// Column — the general body
// ===========================================================================

/// Which of the two things a [`Column`] does with the cells outside its profile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColumnMode {
    /// Re-slot them, and leave the silhouette a cube. This is the **`pillar`** the
    /// user named: a block that *reads* as a round column because its material
    /// pattern is the column's cross-section and its shading normal follows the
    /// profile — with the block's own edges intact.
    Material,
    /// Carve them, so the block genuinely becomes the body. This is a
    /// **stalagmite**, a **boulder**, a **stalactite**: shapes whose *silhouette* is
    /// the law.
    Physical,
}

/// **The general body shaper**: an axis, a profile, and what to do outside it.
///
/// A cell is outside the body when its **centre's distance from the body's axis**
/// exceeds the profile's radius at that cell's **axial** position. That one rule
/// produces every shape in the table in this module's header, including the tapered
/// ones — which is the point of correction 3: a stalagmite is not a cylinder with a
/// different radius, it is a *different law sampled the same way*.
#[derive(Clone, PartialEq, Debug)]
pub struct Column {
    pub axis: Axis,
    pub profile: Profile,
    /// The body's own material. What a physical column leaves behind, and what a
    /// material column keeps inside its pattern.
    pub body: MaterialId,
    /// What the cells outside the pattern become in [`ColumnMode::Material`].
    pub inset: MaterialId,
    pub mode: ColumnMode,
}

impl Column {
    pub fn new(axis: Axis, profile: Profile, body: MaterialId, inset: MaterialId, mode: ColumnMode) -> Refused<Self> {
        if body.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "a body needs a material; the empty id is the absence of matter and cannot be a column",
            ));
        }
        if profile.is_flat() {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a flat profile is not a body; there is no outside to carve or to re-slot",
            ));
        }
        Ok(Column { axis, profile, body, inset, mode })
    }

    /// A **pillar**: a round column as a *material* pattern, silhouette intact. The
    /// smallest body, and the one the user asked for by name.
    pub fn pillar(body: MaterialId, inset: MaterialId) -> Refused<Self> {
        Self::new(
            Axis::Z,
            Profile::RoundColumn { radius: 0.5 },
            body,
            inset,
            ColumnMode::Material,
        )
    }

    /// A **stalagmite** from the world's own quantities: a base radius and a drip
    /// rate. The Damköhler number — and therefore the shape and its regime — is
    /// **derived** from those two, which is the whole reason the law is worth
    /// implementing: nothing about the profile is chosen.
    pub fn stalagmite(base_radius: f64, drip_m3_s: f64, body: MaterialId) -> Refused<Self> {
        let da = Profile::da_from_flow(base_radius, drip_m3_s)?;
        let profile = Profile::stalagmite(da, base_radius)?;
        Self::new(Axis::Z, profile, body, MaterialId::EMPTY, ColumnMode::Physical)
    }

    /// A **stalactite**: a body that tapers **downward** to a thin tip.
    ///
    /// The exact platonic ideal of Short, Baygents & Goldstein is a free-boundary
    /// solution this repository does not implement; what is here is a frustum whose
    /// tip radius is an **input** from the world's drip (a thin film hangs from a
    /// thin tip), and the gap is named in the source string rather than hidden.
    pub fn stalactite(tip_radius: f64, root_radius: f64, body: MaterialId) -> Refused<Self> {
        if !tip_radius.is_finite() || !root_radius.is_finite() || tip_radius >= root_radius {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a stalactite's tip {tip_radius} must be thinner than its root {root_radius}"
                ),
            ));
        }
        let profile = Profile::Frustum { low_radius: tip_radius, high_radius: root_radius };
        Self::new(Axis::Z, profile, body, MaterialId::EMPTY, ColumnMode::Physical)
    }

    /// A **cave column**: a stalactite and a stalagmite that have met, so the
    /// thinnest part is the **waist** where they joined rather than either end.
    ///
    /// That shape is not a frustum — it is wide, then thin, then wide — and it is
    /// built here as a *pair* of frustums sharing a waist, evaluated as the union of
    /// two bodies. A single `Profile` cannot express it, which is a real
    /// limitation of "one body, one law" and is why this constructor returns a
    /// **pair**.
    pub fn cave_column(waist: f64, foot: f64, head: f64, body: MaterialId) -> Refused<(Self, Self)> {
        if !(waist < foot && waist < head) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a column waists at {waist} and meets the floor at {foot} and the roof at {head}; the waist must be the thinnest part"),
            ));
        }
        let lower = Self::new(
            Axis::Z,
            Profile::Frustum { low_radius: foot, high_radius: waist },
            body,
            MaterialId::EMPTY,
            ColumnMode::Physical,
        )?;
        let upper = Self::new(
            Axis::Z,
            Profile::Frustum { low_radius: waist, high_radius: head },
            body,
            MaterialId::EMPTY,
            ColumnMode::Physical,
        )?;
        Ok((lower, upper))
    }

    /// True when this cell's centre lies outside the body.
    ///
    /// The radius is the profile's **cross-section** at this cell's axial position
    /// and not its declared base radius: those differ for every tapered body, and
    /// using the base radius was a real bug — it made a spheroid a cylinder and a
    /// conical stalagmite a cylinder with a spike. See
    /// [`Profile::cross_section_radius`].
    pub fn is_outside(&self, at: SubvoxelIndex) -> bool {
        let radius = self
            .profile
            .cross_section_radius(axial_fraction(at, self.axis));
        radial_offset(at, self.axis) > radius
    }

    /// How many of the 64 cells this body takes.
    pub fn inside_count(&self) -> usize {
        SubvoxelIndex::all().filter(|at| !self.is_outside(*at)).count()
    }
}

impl PhysicalShaper for Column {
    fn name(&self) -> &'static str {
        match self.profile {
            Profile::Stalagmite { .. } => "stalagmite",
            Profile::Spheroid { .. } => "boulder",
            Profile::Frustum { .. } => "stalactite",
            _ => "column",
        }
    }

    fn source(&self) -> &'static str {
        // One string per profile, because a shaper's source is its *law* and two
        // laws are two sources. A single generic string here would be exactly the
        // "unexplained pile of noise" `docs/SHAPERS.md` §2 forbids.
        match self.profile {
            Profile::Stalagmite { .. } => {
                "Szymczak, Ladd, Lipar & Pekarovič 2025, Shapes of ideal stalagmites, PNAS 122(42) e2513263122 — the invariant tangent law cos θ(r) = Da·√(1 − (r/R)²), Da = πR²k/Q, with the three regimes the law itself produces; the paper's closed elliptic-integral form is not implemented, its Eq. 10 is integrated under a substitution that removes the endpoint singularity"
            }
            Profile::Spheroid { .. } => {
                "Zingg 1935, Beitrag zur Schotteranalyse (form from the three axis ratios); Barrett 1980, The shape of rock particles, a critical review, Sedimentology 27:291–303 (form, roundness and texture are three independent axes); Powers 1953, A new roundness scale for sedimentary particles, J. Sedimentary Petrology 23(2) (roundness as the ratio of the corner radius of curvature to the inscribed circle's radius)"
            }
            Profile::Frustum { .. } => {
                "Short, Baygents & Goldstein 2005, Stalactite growth as a free-boundary problem: a geometric law and its platonic ideal, Phys. Rev. Lett. 94 018501 and Phys. Fluids 17 083101 — NAMED GAP: the exact free-boundary ideal is not implemented; this is a tapering body whose tip radius is an input from the drip, and calling it the ideal would be false"
            }
            _ => {
                "Blinn 1978, Simulation of Wrinkled Surfaces, SIGGRAPH '78 (a shading normal is perturbed independently of geometry); Catmull & Clark 1978, Recursively Generated B-Spline Surfaces, CAD 10(6) (the rounded-cube limit surface); Maekawa 1999, An overview of offset curves and surfaces, CAD 31(3)"
            }
        }
    }

    fn acts_at(&self) -> LevelMask {
        match self.mode {
            // A body that is carved also declares its own surface, because a carve
            // alone can only land on the grid.
            ColumnMode::Physical => LevelMask::BOTH,
            ColumnMode::Material => LevelMask::SUBVOXEL,
        }
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        !ctx.shape.is_empty() && ctx.around.open_faces() > 0
    }

    fn carve(&self, _ctx: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
        if self.mode != ColumnMode::Physical {
            return Ok(());
        }
        let column = self;
        out.carve_where(|at| column.is_outside(at));
        Ok(())
    }

    fn displace(&self, out: &mut DisplacementWriter<'_>) -> Refused<()> {
        if self.mode != ColumnMode::Physical {
            return Ok(());
        }
        // The exact surface, as a law the refinement stage samples. The amplitude is
        // the profile's own apex height — and a body whose apex exceeds the
        // half-subvoxel bound is refused by the field with the reasoning that a
        // relief that large is a carve, which is exactly what happened above.
        out.set_domain(COLUMN_DOMAIN ^ self.axis.index() as u64);
        for face in Face::ALL {
            out.declare_profile(self.body, face, self.profile)?;
        }
        Ok(())
    }
}

impl MaterialShaper for Column {
    fn name(&self) -> &'static str {
        "column-pattern"
    }

    fn source(&self) -> &'static str {
        "the same profile as the body, expressed as material slots rather than silhouette — see Column's own PhysicalShaper::source for the law, which is the same law"
    }

    fn acts_at(&self) -> LevelMask {
        LevelMask::SUBVOXEL
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        self.mode == ColumnMode::Material && !ctx.shape.is_empty() && ctx.around.open_faces() > 0
    }

    fn slot(&self, _ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
        if self.inset.is_empty() {
            return Ok(());
        }
        let column = self;
        for at in SubvoxelIndex::all() {
            if out.occupied(at) && column.is_outside(at) && out.get(at) != column.inset {
                out.set(at, column.inset)?;
            }
        }
        Ok(())
    }
}

/// The domain tag a body's displacement is folded under.
const COLUMN_DOMAIN: u64 = 0x424F_4459_0000_0000;

// ===========================================================================
// Boulder — clastic rock, three independent axes
// ===========================================================================

/// **Zingg (1935)** form classes, from the three axis ratios `b/a` and `c/b`, with
/// the diagram's own `2/3` boundaries.
///
/// The fourth class, **bladed**, is reported and then refused by
/// [`Boulder::profile`]: it is not a surface of revolution, so this module cannot
/// give it an exact normal, and approximating it would be the exact kind of
/// plausible-looking wrongness the refusal rule exists to prevent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZinggForm {
    Equant,
    Oblate,
    Prolate,
    Bladed,
}

impl ZinggForm {
    /// Zingg's diagram: `b/a` and `c/b`, each split at `2/3`.
    pub fn of(a: f64, b: f64, c: f64) -> Refused<Self> {
        for (name, value) in [("a", a), ("b", b), ("c", c)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("axis {name} = {value} is not a length"),
                ));
            }
        }
        let (ba, cb) = (b / a, c / b);
        const THRESHOLD: f64 = 2.0 / 3.0;
        Ok(match (ba > THRESHOLD, cb > THRESHOLD) {
            (true, true) => ZinggForm::Equant,
            (true, false) => ZinggForm::Oblate,
            (false, true) => ZinggForm::Prolate,
            (false, false) => ZinggForm::Bladed,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            ZinggForm::Equant => "equant",
            ZinggForm::Oblate => "oblate",
            ZinggForm::Prolate => "prolate",
            ZinggForm::Bladed => "bladed",
        }
    }
}

/// **Powers (1953)** roundness, as the **index** it is defined by: the average
/// radius of curvature of the corners over the radius of the largest inscribed
/// circle.
///
/// Powers' own six class *names* are defined by reference photographs rather than by
/// numeric boundaries, so the boundaries this type derives are declared here and
/// named as this repository's choice inside a published definition — the same
/// discipline C-39 used for the reflectance values.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Roundness {
    /// In `[0, 1]`: 0 is a sharp corner, 1 is a corner whose curvature radius is the
    /// inscribed circle's.
    pub index: f64,
}

impl Roundness {
    pub fn new(index: f64) -> Refused<Self> {
        if !index.is_finite() || !(0.0..=1.0).contains(&index) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a roundness index of {index} is outside [0, 1]"),
            ));
        }
        Ok(Roundness { index })
    }

    /// Powers' six class names, derived from the index. The boundaries are this
    /// repository's, and the class names are Powers'.
    pub fn class(&self) -> &'static str {
        match self.index {
            i if i < 0.12 => "very angular",
            i if i < 0.25 => "angular",
            i if i < 0.40 => "sub-angular",
            i if i < 0.60 => "sub-rounded",
            i if i < 0.85 => "rounded",
            _ => "well rounded",
        }
    }
}

/// A clast's **form** as a spheroid the profile library can give an exact normal to.
///
/// Zingg's axes are body-relative and this world has to stand the rock *somewhere*,
/// so the mapping picks the orientation that is representable and says so:
///
/// | Zingg form | radius | height | why |
/// |---|---|---|---|
/// | **equant** | `b/2` | `c/2` | a ball; the two agree by the form's own definition |
/// | **oblate** | `b/2` | `c/2` | a disc, short in the one axis the form says is short |
/// | **prolate** | `c/2` | `a/2` | a rod, **standing up**, because a horizontal rod is not a surface of revolution |
/// | **bladed** | — | — | **refused**: not a surface of revolution at all, so no exact normal exists in this library |
fn spheroid_for(form: ZinggForm, axes: (f64, f64, f64)) -> Refused<Profile> {
    let (a, b, c) = axes;
    let profile = match form {
        ZinggForm::Equant | ZinggForm::Oblate => Profile::Spheroid { radius: b / 2.0, height: c / 2.0 },
        ZinggForm::Prolate => Profile::Spheroid { radius: c / 2.0, height: a / 2.0 },
        ZinggForm::Bladed => {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a bladed clast ({a} x {b} x {c}) is not a surface of revolution; this library has no exact normal for one, and approximating the form would be a plausible-looking wrongness"
                ),
            ))
        }
    };
    // The block is the body's bound: a rock that does not fit its block is not a
    // block shape, and the profile's own radius check is the reason this is not
    // duplicated here.
    let radius = profile.max_radius();
    if !(voxel_model::profile::MIN_RADIUS..=voxel_model::MAX_RADIUS).contains(&radius) {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!(
                "a {:?} clast's radius {radius} is outside the block; a clast is placed in a block or it is a feature, not a clast",
                form
            ),
        ));
    }
    Ok(profile)
}

/// **A clastic rock**, from its three independent shape axes: **form** (Zingg),
/// **roundness** (Powers) and the block it has to fit in.
///
/// Barrett's (1980) review is the reason this is three fields and not one
/// "rockiness" number: collapsing them cannot tell a *smooth rod* from a *rough
/// disc*, and both are ordinary rocks.
#[derive(Clone, PartialEq, Debug)]
pub struct Boulder {
    pub form: ZinggForm,
    /// The longest, intermediate and shortest axis lengths, in block units.
    pub axes: (f64, f64, f64),
    pub roundness: Roundness,
    pub body: MaterialId,
    pub mode: ColumnMode,
}

impl Boulder {
    pub fn new(axes: (f64, f64, f64), roundness: Roundness, body: MaterialId, mode: ColumnMode) -> Refused<Self> {
        let form = ZinggForm::of(axes.0, axes.1, axes.2)?;
        // Refuse at construction rather than in the middle of a world.
        spheroid_for(form, axes)?;
        Ok(Boulder { form, axes, roundness, body, mode })
    }

    /// The spheroid this rock's **form** is, or a refusal for `Bladed`.
    ///
    /// Roundness is deliberately *not* folded in here: it is a property of the
    /// **corner**, which is a 256-level detail, and folding it into the body's
    /// radius would be the one-number collapse Barrett warns against.
    pub fn profile(&self) -> Refused<Profile> {
        spheroid_for(self.form, self.axes)
    }

    /// The 256-level **roughness** amplitude this rock's roundness implies, in
    /// subvoxels: a sharp rock has relief, a well-rounded one does not.
    ///
    /// Derived from the index and from the block, with no chosen constant:
    /// `(1 − index)` of a half-subvoxel is the most a corner may stand out and still
    /// be inside the body it was carved from.
    pub fn relief_subvoxels(&self) -> f64 {
        (1.0 - self.roundness.index) * voxel_model::MAX_DISPLACEMENT_SUBVOXELS
    }
}

impl PhysicalShaper for Boulder {
    fn name(&self) -> &'static str {
        "boulder"
    }

    fn source(&self) -> &'static str {
        "Zingg 1935, Beitrag zur Schotteranalyse; Barrett 1980, The shape of rock particles, a critical review, Sedimentology 27:291–303; Powers 1953, A new roundness scale for sedimentary particles, J. Sedimentary Petrology 23(2):117–119"
    }

    fn acts_at(&self) -> LevelMask {
        // The form is a carve; the roundness is a 256-level relief. Both, and the
        // distinction is the point of Barrett's three axes.
        LevelMask::BOTH
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        !ctx.shape.is_empty() && ctx.around.open_faces() > 0
    }

    fn carve(&self, _ctx: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
        let Ok(profile) = self.profile() else {
            // `Bladed` has no profile. A carve that cannot be computed must not
            // quietly do nothing and leave a cube behind wearing a boulder's name.
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a {:?} clast is not a surface of revolution, so this shaper cannot carve it exactly; refusing rather than approximating the form",
                    self.form
                ),
            ));
        };
        if self.mode != ColumnMode::Physical {
            return Ok(());
        }
        let axis = Axis::Z;
        out.carve_where(|at| {
            radial_offset(at, axis) > profile.cross_section_radius(axial_fraction(at, axis))
        });
        Ok(())
    }

    fn displace(&self, out: &mut DisplacementWriter<'_>) -> Refused<()> {
        let relief = self.relief_subvoxels();
        if relief <= 0.0 {
            return Ok(());
        }
        out.set_domain(BOULDER_DOMAIN ^ self.form as u64);
        out.declare_all_faces(self.body, relief)
    }
}

/// The domain tag a boulder's relief is folded under.
const BOULDER_DOMAIN: u64 = 0x524F_434B_0000_0000;

// ===========================================================================
// Flowstone — the 256 level on its own
// ===========================================================================

/// **Physical, at the 256 level only.** Ripples on a flowstone sheet, as a band of
/// relief whose magnitude is declared.
///
/// **Camporeale, C. et al. (2012), *Hydrodynamic-Driven Stability Analysis of
/// Morphological Patterns on Speleothems*, Phys. Rev. Lett. 108, 238501** shows
/// that the ripples are a **hydrodynamic instability** with a preferred wavelength
/// rather than noise — which is why this shaper declares an *amplitude and a
/// wavelength* and not a fractal. The wavelength is an **input**, and the source
/// string says so: the paper's preferred-wavelength expression was not reproduced
/// here, so quoting a number would be quoting something nobody in this repository
/// computed.
///
/// This shaper declares **no 4×4×4 work at all**, which makes it the cleanest
/// statement of what the second level is for: there is nothing here a cube could
/// express.
#[derive(Clone, PartialEq, Debug)]
pub struct Flowstone {
    pub body: MaterialId,
    /// The relief's half-amplitude, in subvoxels.
    pub amplitude_subvoxels: f64,
    /// The ripple wavelength, in block units — an input, not a derived constant.
    pub wavelength_blocks: f64,
}

impl Flowstone {
    pub fn new(body: MaterialId, amplitude_subvoxels: f64, wavelength_blocks: f64) -> Refused<Self> {
        if body.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "flowstone needs a material to be made of",
            ));
        }
        if !wavelength_blocks.is_finite() || wavelength_blocks <= 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a ripple wavelength of {wavelength_blocks} blocks is not a wavelength"),
            ));
        }
        Ok(Flowstone { body, amplitude_subvoxels, wavelength_blocks })
    }
}

impl PhysicalShaper for Flowstone {
    fn name(&self) -> &'static str {
        "flowstone"
    }

    fn source(&self) -> &'static str {
        "Camporeale et al. 2012, Hydrodynamic-Driven Stability Analysis of Morphological Patterns on Speleothems, Phys. Rev. Lett. 108 238501 — the ripples are an instability with a preferred wavelength, so this is a band and not a fractal. NAMED GAP: the paper's preferred-wavelength expression is not reproduced here, so the wavelength is an input rather than a derived constant"
    }

    fn acts_at(&self) -> LevelMask {
        LevelMask::DETAIL
    }

    fn applies(&self, _ctx: &VoxelContext) -> bool {
        // No 4x4x4 work, so nothing to apply to. The declaration below is the whole
        // shaper.
        false
    }

    fn displace(&self, out: &mut DisplacementWriter<'_>) -> Refused<()> {
        out.set_domain(FLOWSTONE_DOMAIN);
        // A bare amplitude: a ripple band has a magnitude and a wavelength but no
        // law that says which way each fragment leans — that comes from the lattice
        // at the address, which is exactly what a bare amplitude is for.
        out.declare_all_faces(self.body, self.amplitude_subvoxels)
    }
}

/// The domain tag flowstone's ripples are folded under.
const FLOWSTONE_DOMAIN: u64 = 0x464C_4F57_0000_0000;

// ===========================================================================
// Coral — the growth forms
// ===========================================================================

/// The standard reef-survey **growth forms**. Each is a body count and a body
/// shape, and the `bodies` below is where that becomes geometry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoralForm {
    /// Several thin columns from one base: the fastest-growing form.
    Branching,
    /// One thick column.
    Columnar,
    /// A ball or boulder shape, slow-growing and storm-resistant.
    Massive,
    /// Branches fused into a platform with a **flat top**.
    Tabulate,
    /// A plate: wide and thin.
    Laminar,
    /// A thin crust over whatever it grew on.
    Encrusting,
}

/// How far a branching colony's branch axes sit from the block's axis: the centre of a
/// **corner column** of the stored lattice, which is the one place in a block far
/// enough from the middle to leave the middle open.
const BRANCH_OFFSET: f64 = voxel_model::MAX_RADIUS - 1.0 / (2.0 * SUBVOXEL_SIDE as f64);

/// A branch's girth at its base and at its tip, as fractions of the colony's radius.
///
/// The fractions are derived and not decorative: the nearest cell centre to a branch's
/// axis is `0.3536` away (a block's innermost cell against a corner column), so any
/// girth below `0.7071` of the colony radius keeps the middle of the block outside
/// every branch — and `0.6` is below that at every radius a block can hold, `0.5`
/// being the largest.
const BRANCH_FOOT: f64 = 0.4;
const BRANCH_GIRTH: f64 = 0.6;

impl CoralForm {
    pub const ALL: [CoralForm; 6] = [
        CoralForm::Branching,
        CoralForm::Columnar,
        CoralForm::Massive,
        CoralForm::Tabulate,
        CoralForm::Laminar,
        CoralForm::Encrusting,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            CoralForm::Branching => "branching",
            CoralForm::Columnar => "columnar",
            CoralForm::Massive => "massive",
            CoralForm::Tabulate => "tabulate",
            CoralForm::Laminar => "laminar",
            CoralForm::Encrusting => "encrusting",
        }
    }

    /// The bodies this form is, along the block's vertical axis.
    ///
    /// Branching is a **set** and not one body, which is the reason this returns a
    /// list: a branch is not a shape a single profile can be. `Da` for the tabulate
    /// form is greater than 1 on purpose — a tabulate coral's flat table is exactly
    /// the flat cap the stalagmite law produces when `Da > 1`, which is a nice
    /// coincidence of the *physics* being similar (accretion from above).
    pub fn bodies(self, radius: f64) -> Refused<Vec<(f64, f64, Profile)>> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a corallum radius of {radius} is not a length"),
            ));
        }
        let out = match self {
            CoralForm::Branching => vec![
                // One branch at each **corner column** of the block, splaying upward.
                // A branching colony is several bodies that meet at the base, and the
                // corner is the one place in a block that leaves the middle open —
                // which is what makes the form read as branches instead of as a ball.
                (
                    -BRANCH_OFFSET,
                    -BRANCH_OFFSET,
                    Profile::Frustum { low_radius: radius * BRANCH_FOOT, high_radius: radius * BRANCH_GIRTH },
                ),
                (
                    BRANCH_OFFSET,
                    -BRANCH_OFFSET,
                    Profile::Frustum { low_radius: radius * BRANCH_FOOT, high_radius: radius * BRANCH_GIRTH },
                ),
                (
                    -BRANCH_OFFSET,
                    BRANCH_OFFSET,
                    Profile::Frustum { low_radius: radius * BRANCH_FOOT, high_radius: radius * BRANCH_GIRTH },
                ),
                (
                    BRANCH_OFFSET,
                    BRANCH_OFFSET,
                    Profile::Frustum { low_radius: radius * BRANCH_FOOT, high_radius: radius * BRANCH_GIRTH },
                ),
            ],
            CoralForm::Columnar => vec![(0.0, 0.0, Profile::Cylinder { radius })],
            CoralForm::Massive => vec![(0.0, 0.0, Profile::Spheroid { radius, height: radius })],
            // A flat table: the stalagmite law above Da = 1 *is* a flat-capped cap.
            CoralForm::Tabulate => vec![(0.0, 0.0, Profile::stalagmite(2.5, radius)?)],
            CoralForm::Laminar => vec![(0.0, 0.0, Profile::Spheroid { radius, height: radius * 0.3 })],
            CoralForm::Encrusting => vec![(0.0, 0.0, Profile::Spheroid { radius, height: radius * 0.15 })],
        };
        Ok(out)
    }
}

/// **Coral**, as its growth form: a **set** of bodies, tested for containment per
/// cell.
///
/// This is the shape that made "one shaper, one profile" insufficient, and the
/// resolution is honest rather than clever: a branching colony *is* several bodies,
/// so the shaper holds several. Containment is the union — a cell is coral if it is
/// inside **any** body.
#[derive(Clone, PartialEq, Debug)]
pub struct Coral {
    pub form: CoralForm,
    /// The colony's own radius, in block units.
    pub radius: f64,
    /// The **corallite** diameter — the species-scale unit of the skeleton, and the
    /// sourced constant this shaper has.
    pub corallite_blocks: f64,
    pub body: MaterialId,
    pub calyx: MaterialId,
}

impl Coral {
    pub fn new(form: CoralForm, radius: f64, corallite_blocks: f64, body: MaterialId, calyx: MaterialId) -> Refused<Self> {
        if body.is_empty() || calyx.is_empty() {
            return Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                "coral needs both a skeleton material and a calyx material; a corallite is a place a polyp sat, which is a second material",
            ));
        }
        if !corallite_blocks.is_finite() || corallite_blocks <= 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a corallite diameter of {corallite_blocks} blocks is not a length"),
            ));
        }
        // Build the bodies once, so a malformed form refuses at construction and not
        // in the middle of a world.
        form.bodies(radius)?;
        Ok(Coral { form, radius, corallite_blocks, body, calyx })
    }

    /// True when a cell's centre is inside any of the form's bodies.
    pub fn is_inside(&self, at: SubvoxelIndex) -> Refused<bool> {
        let axis = Axis::Z;
        let (dx_axis, dy_axis) = axis.others();
        let axial = axial_fraction(at, axis);
        // The offset is applied as a *translation* of the body, not as a rotation:
        // a branch grows where it grew, and a translated circle is still a circle.
        let du = centred(coord_of(at, dx_axis));
        let dv = centred(coord_of(at, dy_axis));
        for (ox, oy, profile) in self.form.bodies(self.radius)? {
            let d = ((du - ox) * (du - ox) + (dv - oy) * (dv - oy)).sqrt();
            if d <= profile.cross_section_radius(axial) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl PhysicalShaper for Coral {
    fn name(&self) -> &'static str {
        "coral"
    }

    fn source(&self) -> &'static str {
        "the standard reef-survey growth forms (branching, columnar, massive, tabulate, laminar, encrusting); the corallite is the species-scale unit of the skeleton and its diameter is a measured constant; the tabulate form uses the stalagmite law's own flat cap (Szymczak et al. 2025), because a tabulate table and a flat-topped stalagmite are both accretion from above"
    }

    fn acts_at(&self) -> LevelMask {
        LevelMask::SUBVOXEL
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        // Coral needs water above it, not air. Modelling that honestly needs the
        // hydrology stage, which this shaper cannot see (`VoxelContext` carries the
        // six neighbours and nothing else), so for now it applies wherever it is
        // exposed and underwater. Recorded as a limitation.
        !ctx.shape.is_empty() && ctx.around.open_faces() > 0
    }

    fn carve(&self, _ctx: &VoxelContext, out: &mut ShapeWriter<'_>) -> Refused<()> {
        let inside = |at: SubvoxelIndex| self.is_inside(at).unwrap_or(false);
        out.carve_where(|at| !inside(at));
        Ok(())
    }
}

impl MaterialShaper for Coral {
    fn name(&self) -> &'static str {
        "coral-calyx"
    }

    fn source(&self) -> &'static str {
        "the corallite: the standard growth-form literature's skeletal unit, and the second material a coral needs — a polyp's seat is not the same substance as the frame it sits in"
    }

    fn acts_at(&self) -> LevelMask {
        LevelMask::SUBVOXEL
    }

    fn applies(&self, ctx: &VoxelContext) -> bool {
        !ctx.shape.is_empty() && ctx.around.open_faces() > 0
    }

    fn slot(&self, _ctx: &VoxelContext, out: &mut SlotWriter<'_>) -> Refused<()> {
        // A calyx is where a corallite's wall is: for a surface coral, that is the
        // cells whose distance from the body's surface is under a corallite radius.
        // Approximated as "inside the body but near the outside", which is the
        // coarse form of the same statement and is named as coarse.
        let near = self.corallite_blocks / 2.0;
        for at in SubvoxelIndex::all() {
            if !out.occupied(at) || out.get(at) == self.calyx {
                continue;
            }
            if self.is_inside(at).unwrap_or(false)
                && radial_offset(at, Axis::Z) > self.radius - near
                && self.calyx != MaterialId::EMPTY
            {
                out.set(at, self.calyx)?;
            }
        }
        Ok(())
    }
}

// ===========================================================================
// The set
// ===========================================================================

/// A starter set: one body, one cut, and both kinds present. Called by the world
/// and by tests, so that "the shapers are wired in" is not a claim about nothing.
pub fn standard(body: MaterialId, inset: MaterialId, seam: MaterialId) -> Refused<ShaperSet> {
    let mut set = ShaperSet::new();
    if let Ok(pillar) = Column::pillar(body, inset) {
        set = set.with_material(pillar);
    }
    if !seam.is_empty() {
        set = set.with_material(LoopCut::new(Cut::half(Axis::Z), seam)?);
    }
    set.audit().map_err(|set| {
        set.all().first().cloned().unwrap_or_else(|| {
            Refusal::new(RefusalKind::MissingStage, "the shaper set refused for no named reason")
        })
    })?;
    Ok(set)
}

/// Where a voxel sits in a chunk's shape pipeline, from six face materials.
pub fn neighbourhood_from(faces: [MaterialId; 6], open_to_sky: bool) -> Neighbourhood {
    Neighbourhood { faces, open_to_sky }
}

/// The material on a face that is **air** — named so a call site reads as what it
/// means.
pub const OPEN: MaterialId = MaterialId::EMPTY;

/// A shape for a voxel the shapers have nothing to say about — which, for most of a
/// world, is most of it.
pub fn plain(material: MaterialId) -> VoxelShape {
    if material.is_empty() {
        VoxelShape::empty()
    } else {
        VoxelShape::solid(material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::shaper::Level;
    use voxel_model::{Profile, Regime, MAX_RADIUS};

    const ROCK: MaterialId = MaterialId(6);
    const SEAM: MaterialId = MaterialId(4);
    const INSET: MaterialId = MaterialId(5);

    fn at(x: u8, y: u8, z: u8) -> SubvoxelIndex {
        SubvoxelIndex::new(x, y, z).unwrap()
    }

    fn exposed(shape: VoxelShape) -> VoxelContext {
        VoxelContext::new([0, 0, 0], shape, Neighbourhood::exposed(), 1, 0)
    }

    fn sealed(shape: VoxelShape) -> VoxelContext {
        VoxelContext::new([0, 0, 0], shape, Neighbourhood::buried(ROCK), 1, 0)
    }

    fn solid() -> VoxelContext {
        exposed(VoxelShape::solid(ROCK))
    }

    // ------------------------------------------------------- the cut, parametric

    #[test]
    fn a_cut_is_a_position_in_256ths_and_not_one_of_three_quarters() {
        // Correction 2, measured. The old type could express 64, 128 and 192 and
        // nothing else; this one expresses 96, and 96 is not any of the three.
        let eighths = Cut::slab(0, 32).unwrap(); // one eighth, from the bottom
        assert_eq!(eighths.kept_fraction(), 0.125);
        assert_eq!(eighths.surviving_cells(), vec![0]);
        // And it is realisable: the plane lands on cell 0's own middle, so cell 0 is
        // kept and its top face shaves the maximum half a subvoxel inward.
        assert_eq!(
            eighths.trims().unwrap(),
            vec![Trim { cell: 0, face: Face::PosZ, displacement_subvoxels: -0.5 }]
        );
        let three = Cut::slab(0, 96).unwrap();
        assert_eq!(three.kept_fraction(), 96.0 / 256.0);
        assert_eq!(three.surviving_cells(), vec![0, 1], "96/256 is one and a half cells");

        // An off-centre third: 85..171 is a third of the block, in the middle.
        let middle = Cut::slab(85, 171).unwrap();
        assert_eq!(middle.kept_fraction(), 86.0 / 256.0);
        assert_eq!(middle.surviving_cells(), vec![1, 2]);
        // And it is a different world from the 50% cut, which is the claim.
        assert_ne!(middle.surviving_cells(), Cut::half(Axis::Z).surviving_cells());
        assert_eq!(Cut::half(Axis::Z).surviving_cells(), vec![0, 1]);
    }

    #[test]
    fn a_band_that_keeps_a_middle_slice_is_two_cuts_and_the_type_says_so() {
        let trough = Cut::discard(Axis::Z, 96, 160).unwrap();
        assert_eq!(trough.bands().len(), 2, "a notch leaves two bands");
        assert_eq!(trough.bands()[0], Band { low: 0, high: 96 });
        assert_eq!(trough.bands()[1], Band { low: 160, high: 256 });
        assert_eq!(trough.surviving_cells(), vec![0, 1, 2, 3]);

        // Discarding everything is not a cut, it is an empty shape.
        assert!(Cut::discard(Axis::Z, 0, 256).is_err());
        // And an empty band is not a band.
        assert!(Band::new(10, 10).is_err());
        assert!(Band::new(10, 9).is_err());
        assert!(Band::new(0, 257).is_err());
    }

    #[test]
    fn a_plank_stack_keeps_several_bands_and_that_is_what_a_single_plane_cannot_say() {
        let planks = Cut::planking(Axis::Z, 4, 48).unwrap();
        assert_eq!(planks.bands().len(), 4);
        assert_eq!(planks.kept_fraction(), 4.0 * 48.0 / 256.0);
        assert_eq!(planks.surviving_cells(), vec![0, 1, 2, 3]);
        let two = Cut::planking(Axis::X, 2, 100).unwrap();
        assert_eq!(two.bands().len(), 2);
        assert_eq!(two.bands()[0], Band { low: 0, high: 100 });
        assert_eq!(two.bands()[1], Band { low: 128, high: 228 });
        // A plank thicker than its pitch would overlap the next one.
        assert!(Cut::planking(Axis::Z, 4, 65).is_err());
        assert!(Cut::planking(Axis::Z, 0, 10).is_err());
        assert!(Cut::planking(Axis::Z, 4, 0).is_err());
    }

    #[test]
    fn the_cut_s_residual_is_derived_from_its_own_position_and_nobody_chooses_it() {
        // The number that makes a parametric cut real. A cut on a cell boundary has
        // no residual and must declare no displacement; a cut at 96/256 sits half a
        // cell into cell 1, so cell 1's own top face shaves half a subvoxel inward.
        assert_eq!(Cut::slab(0, 128).unwrap().max_residual(), 0.0);
        assert!(Cut::slab(0, 128).unwrap().trims().unwrap().is_empty());

        let half_cell = Cut::slab(0, 96).unwrap();
        assert_eq!(half_cell.max_residual(), 0.5);
        assert_eq!(
            half_cell.trims().unwrap(),
            vec![Trim { cell: 1, face: Face::PosZ, displacement_subvoxels: -0.5 }]
        );

        // 112 is three-quarters of a cell: cell 1 is kept — the band covers more than
        // half of it — and its top face shaves the remaining quarter inward.
        let three_quarters = Cut::slab(0, 112).unwrap();
        assert_eq!(three_quarters.max_residual(), 0.25);
        assert_eq!(
            three_quarters.trims().unwrap(),
            vec![Trim { cell: 1, face: Face::PosZ, displacement_subvoxels: -0.25 }]
        );

        // A cut that discards the middle trims both of its walls: the cell below the
        // hole shaves its top face down, the cell above shaves its low face up.
        let notch = Cut::discard(Axis::Z, 96, 160).unwrap();
        let trims = notch.trims().unwrap();
        assert_eq!(trims.len(), 2);
        assert_eq!(trims[0].cell, 1);
        assert_eq!(trims[0].face, Face::PosZ);
        assert_eq!(trims[1].cell, 2);
        assert_eq!(trims[1].face, Face::NegZ);
        assert_eq!(trims[0].magnitude(), 0.5);
        assert_eq!(trims[1].magnitude(), 0.5);
        // Both are shaves: a cut removes material and never adds any.
        assert!(trims.iter().all(|t| t.displacement_subvoxels < 0.0));
        // A band anchored at the block's own bound trims once: the bound is not a cut,
        // so there is no face there to move.
        assert_eq!(Cut::slab(0, 48).unwrap().trims().unwrap().len(), 1);
    }

    #[test]
    fn a_band_thinner_than_half_a_cell_is_refused_by_name_and_not_rounded() {
        // The resolution limit of the two levels together, stated rather than
        // rounded: a kept cell needs half a cell of band to sit in, and a face moves
        // half a subvoxel at most. A sliver has neither, so it is refused — a world
        // that quietly widened it would be showing a shape nobody asked for.
        let sliver = Cut::slab(0, 20).unwrap();
        assert!(sliver.surviving_cells().is_empty(), "no cell is covered by half");
        assert!(sliver.max_residual() > 0.0, "it has a residual, and no cell to spend it on");
        let err = sliver.trims().unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("20/256"), "{}", err.detail());

        // And the boundary case lands: exactly half a cell is the largest
        // displacement there is, and the tie keeps the cell.
        let boundary = Cut::slab(0, 32).unwrap();
        assert_eq!(boundary.surviving_cells(), vec![0]);
        assert_eq!(boundary.trims().unwrap()[0].displacement_subvoxels, -0.5);
    }

    #[test]
    fn a_cut_that_lands_on_the_grid_declares_nothing_because_it_needs_nothing() {
        // A declaration nothing needs is a claim that cannot be falsified, so the
        // shaper must not make one. A 50 % cut is already exact at the grid.
        let (displacement, _) = {
            let set = ShaperSet::new()
                .with_physical(Slab::new(Cut::half(Axis::Z), ROCK).unwrap());
            set.declare().unwrap()
        };
        assert!(displacement.is_empty(), "a grid-aligned cut should declare nothing");

        let (off_grid, _) = {
            let set = ShaperSet::new()
                .with_physical(Slab::new(Cut::slab(0, 96).unwrap(), ROCK).unwrap());
            set.declare().unwrap()
        };
        assert!(!off_grid.is_empty(), "an off-grid cut must declare its residual");
        assert_eq!(off_grid.profiled_faces(), 0, "a cut has a magnitude, not a law");
        // The amplitude *is* the residual, in subvoxels — one cell is one subvoxel —
        // and the face is the one nearest the edge. A magnitude multiplied by four
        // here would be a block-eighths displacement claiming to be a subvoxel one.
        assert_eq!(off_grid.amplitude(ROCK, Face::PosZ), Some(-0.5));
        assert_eq!(
            off_grid.amplitude(ROCK, Face::NegZ),
            None,
            "the low edge is the block's own bound and has no face to move"
        );
    }

    #[test]
    fn a_slab_carves_the_cells_outside_its_bands_and_moves_nothing_inside_them() {
        let set = ShaperSet::new().with_physical(Slab::new(Cut::slab(0, 96).unwrap(), ROCK).unwrap());
        let out = set.apply(&solid()).unwrap();
        for cell in SubvoxelIndex::all() {
            assert_eq!(out.get(cell).is_empty(), cell.z >= 2, "z = {}", cell.z);
        }
        // Half a block kept, half gone, and the survivors are untouched rock.
        assert_eq!(out.get(at(0, 0, 0)), ROCK);
        assert_eq!(out.get(at(3, 3, 1)), ROCK);
        assert!(out.get(at(0, 0, 2)).is_empty());
        // A band in the *middle* keeps the same count from a different place, which
        // is what "not a 50% cut" means geometrically.
        let middle = ShaperSet::new()
            .with_physical(Slab::new(Cut::slab(85, 171).unwrap(), ROCK).unwrap())
            .apply(&solid())
            .unwrap();
        assert_ne!(out, middle);
        assert!(middle.get(at(0, 0, 0)).is_empty());
        assert_eq!(middle.get(at(0, 0, 1)), ROCK);
        assert_eq!(middle.get(at(0, 0, 2)), ROCK);
        assert!(middle.get(at(0, 0, 3)).is_empty());
    }

    #[test]
    fn a_plank_cut_leaves_a_stack_and_a_loop_cut_leaves_the_stack_intact() {
        // The same cut, the two kinds.
        let cut = Cut::planking(Axis::Z, 2, 64).unwrap();
        let planks = ShaperSet::new()
            .with_physical(Slab::new(cut.clone(), ROCK).unwrap())
            .apply(&solid())
            .unwrap();
        let seamed = ShaperSet::new()
            .with_material(LoopCut::new(cut, SEAM).unwrap())
            .apply(&solid())
            .unwrap();
        // Two planks of one cell each, at the bottom of each half of the block: the
        // cell above each plank is the air the bands do not reach.
        assert_eq!(planks.cells().iter().filter(|m| m.is_empty()).count(), 32);
        for cell in SubvoxelIndex::all() {
            assert_eq!(planks.get(cell).is_empty(), cell.z == 1 || cell.z == 3, "z = {}", cell.z);
        }
        // The material shaper moved nothing — no cell changed occupancy — and seamed
        // the two cells the planes actually pass through.
        assert_eq!(seamed.cells().iter().filter(|m| m.is_empty()).count(), 0);
        assert_eq!(seamed.cells().iter().filter(|m| **m == SEAM).count(), 32);
        for cell in SubvoxelIndex::all() {
            let want = if cell.z == 0 || cell.z == 2 { SEAM } else { ROCK };
            assert_eq!(seamed.get(cell), want, "z = {}", cell.z);
        }
    }

    #[test]
    fn a_seam_of_nothing_is_refused_because_it_is_a_carve() {
        let err = LoopCut::new(Cut::half(Axis::Z), MaterialId::EMPTY).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn a_loop_cut_leaves_rock_inside_the_rock_alone() {
        let set = ShaperSet::new().with_material(LoopCut::new(Cut::half(Axis::Z), SEAM).unwrap());
        assert_eq!(set.apply(&sealed(VoxelShape::solid(ROCK))).unwrap(), VoxelShape::solid(ROCK));
    }

    // ------------------------------------------------------------- the bodies

    #[test]
    fn a_pillar_pattern_is_the_sampled_law_and_not_a_bitmap() {
        // Correction 3's premise: the pattern is a sample. At the full radius the
        // law's disc leaves exactly the block's eight corners outside, which is the
        // pattern the first attempt hardcoded — and now it is derived.
        // The disc is inscribed in the block, so what it leaves out is the four
        // **corner columns** — sixteen cells — and sixteen is a number the law gives
        // rather than a bitmap: a cell centre at 0.375 on both in-plane axes is
        // 0.5303 from the axis, past the 0.5 that is half the block, while one at
        // 0.375 and 0.125 is 0.3953 and inside it.
        let pillar = Column::pillar(ROCK, INSET).unwrap();
        assert_eq!(pillar.inside_count(), 64 - 16);
        for cell in SubvoxelIndex::all() {
            let extreme = |c: u8| c == 0 || c == 3;
            assert_eq!(
                pillar.is_outside(cell),
                extreme(cell.x) && extreme(cell.y),
                "({},{},{})",
                cell.x,
                cell.y,
                cell.z
            );
        }
        // And it re-slots exactly those cells, changing no occupancy.
        let set = ShaperSet::new().with_material(pillar);
        let out = set.apply(&solid()).unwrap();
        assert_eq!(out.cells().iter().filter(|m| **m == INSET).count(), 16);
        assert_eq!(out.cells().iter().filter(|m| m.is_empty()).count(), 0);
        // The derived recess on a face is the **projection of the body's own pattern
        // onto that face**, and a cylinder does not project the same way onto its caps
        // as onto its sides: a cap sees the corner columns as four corners, while a
        // side sees them as **two whole rows** — eight cells — because matter at
        // `x = 3` is outside a cylinder of radius 0.5 for every `z` once `y` is at an
        // end. Both are the same body, read from different sides, and the appearance
        // layer wraps a normal around whatever the projection says.
        for face in Face::ALL {
            let want = match face {
                Face::PosZ | Face::NegZ => 4,
                _ => 8,
            };
            assert_eq!(out.recessed_cells(face), want, "{face:?}");
        }
    }

    #[test]
    fn a_stalagmite_profile_tapered_so_the_pattern_differs_per_layer_is_the_whole_point() {
        // A *tapered* column. The wide end keeps its corners and the thin end has
        // none, because the pattern is sampled per layer — the thing a single
        // cylinder bitmap cannot express.
        let tapered = Column::new(
            Axis::Z,
            Profile::Frustum { low_radius: 0.5, high_radius: 0.12 },
            ROCK,
            INSET,
            ColumnMode::Material,
        )
        .unwrap();
        let radius_at_layer = |z: u8| tapered.profile.radius_at(axial_fraction(at(0, 0, z), Axis::Z));
        let outside_at_layer = |z: u8| {
            SubvoxelIndex::all()
                .filter(|c| c.z == z)
                .filter(|c| tapered.is_outside(*c))
                .count()
        };
        assert_eq!(outside_at_layer(0), 4, "the full-radius base keeps four corners out");
        assert_eq!(outside_at_layer(3), 16, "the thin tip is outside on every cell");
        assert!(radius_at_layer(0) > radius_at_layer(3));
        assert!(tapered.inside_count() < 64 - 8);

        // And as a material shaper it changes no occupancy while producing a
        // per-layer pattern — the silhouette is still a cube.
        let set = ShaperSet::new().with_material(tapered);
        let out = set.apply(&solid()).unwrap();
        assert_eq!(out.cells().iter().filter(|m| m.is_empty()).count(), 0);
        // The bottom face sees the base's pattern and the top face the tip's, so the
        // two are different pictures of one law. A tapered column re-slots per layer,
        // so a face's recess belongs to the layer it is on and not to the body.
        assert_ne!(out.recessed_cells(Face::NegZ), out.recessed_cells(Face::PosZ));
        assert_eq!(out.recessed_cells(Face::NegZ), 4);
    }

    #[test]
    fn a_stalagmite_gets_its_shape_from_the_world_s_drip_rate_and_not_from_a_cone() {
        // The law, end to end: a radius and a drip give a Damköhler number, the
        // number gives a regime, and the regime is a *different silhouette*.
        let slow = Column::stalagmite(0.4, 1e-11, ROCK).unwrap();
        let fast = Column::stalagmite(0.4, 1e-7, ROCK).unwrap();
        let reg = |c: &Column| match c.profile {
            Profile::Stalagmite { .. } => c.profile.regime(),
            _ => panic!("not a stalagmite"),
        };
        assert_eq!(reg(&slow), Regime::FlatTop);
        assert_eq!(reg(&fast), Regime::Conical);
        // A flat-topped one has a flat disc on top; a conical one has a point, so it
        // keeps far fewer cells and its apex is higher.
        assert!(slow.profile.cap_height_total() < fast.profile.cap_height_total());
        assert!(slow.inside_count() > fast.inside_count());
    }

    #[test]
    fn a_stalactite_refuses_a_tip_that_is_not_thinner_than_its_root() {
        assert!(Column::stalactite(0.1, 0.4, ROCK).is_ok());
        let err = Column::stalactite(0.4, 0.1, ROCK).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("thinner"));
    }

    #[test]
    fn a_cave_column_has_a_waist_and_its_own_thinnest_part_is_the_test() {
        let (lower, upper) = Column::cave_column(0.12, 0.45, 0.35, ROCK).unwrap();
        assert_eq!(lower.profile.radius_at(0.0), 0.45);
        assert_eq!(lower.profile.radius_at(1.0), 0.12);
        assert_eq!(upper.profile.radius_at(0.0), 0.12);
        assert_eq!(upper.profile.radius_at(1.0), 0.35);
        // The waist is the thinnest part of the pair, which is what makes it a column
        // and not two cones.
        assert!(lower.profile.radius_at(1.0) <= lower.profile.radius_at(0.0));
        assert!(upper.profile.radius_at(0.0) <= upper.profile.radius_at(1.0));
        // A "column" whose middle is fatter than an end is refused.
        let err = Column::cave_column(0.5, 0.2, 0.3, ROCK).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("thinnest"));
    }

    // ------------------------------------------------------------- the rocks

    #[test]
    fn a_clast_is_three_independent_axes_and_the_form_comes_from_their_ratios() {
        // Zingg's diagram, at its own 2/3 boundaries.
        assert_eq!(ZinggForm::of(3.0, 2.5, 2.0).unwrap(), ZinggForm::Equant);
        assert_eq!(ZinggForm::of(3.0, 2.5, 1.0).unwrap(), ZinggForm::Oblate);
        assert_eq!(ZinggForm::of(3.0, 1.5, 1.2).unwrap(), ZinggForm::Prolate);
        // Exactly on Zingg's 2/3 boundary: `of` compares strictly, so `c/b = 2/3` is
        // *not* "greater than 2/3" and the pair (b/a small, c/b on the line) lands in
        // Bladed. The rule is stated here rather than left incidental — a clast
        // exactly on a class boundary is on the boundary in Zingg's own diagram too —
        // and the two cases either side of the line are pinned so the boundary is
        // visible.
        assert_eq!(ZinggForm::of(3.0, 1.5, 1.0).unwrap(), ZinggForm::Bladed);
        assert_eq!(ZinggForm::of(3.0, 1.5, 0.8).unwrap(), ZinggForm::Bladed);
        assert_eq!(ZinggForm::of(3.0, 1.5, 1.02).unwrap(), ZinggForm::Prolate);
        assert!(ZinggForm::of(0.0, 1.0, 1.0).is_err());
        assert!(ZinggForm::of(f64::NAN, 1.0, 1.0).is_err());

        // And the form's profile is the form, with bladed refused rather than
        // approximated — a stated limitation and not a hidden one.
        let equant = Boulder::new((0.3, 0.28, 0.26), Roundness::new(0.4).unwrap(), ROCK, ColumnMode::Physical).unwrap();
        assert_eq!(equant.form, ZinggForm::Equant);
        assert!(matches!(equant.profile().unwrap(), Profile::Spheroid { .. }));
        // A bladed clast is refused at **construction**: it is not a surface of
        // revolution, this library has no exact normal for one, and approximating the
        // form would be a plausible-looking wrongness.
        let err = Boulder::new(
            (0.3, 0.15, 0.08),
            Roundness::new(0.4).unwrap(),
            ROCK,
            ColumnMode::Physical,
        )
        .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("bladed"), "{}", err.detail());
        // And the shaper is the second line of defence: the fields are public, so a
        // bladed clast can be assembled without the constructor — and a carve then
        // refuses too, rather than leaving a cube behind wearing a boulder's name.
        let bladed = Boulder {
            form: ZinggForm::Bladed,
            axes: (0.3, 0.15, 0.08),
            roundness: Roundness::new(0.4).unwrap(),
            body: ROCK,
            mode: ColumnMode::Physical,
        };
        assert!(bladed.profile().is_err());
        let set = ShaperSet::new().with_physical(bladed);
        assert!(set.apply(&solid()).is_err());
    }

    #[test]
    fn powers_roundness_is_the_index_and_its_classes_follow_from_it() {
        assert_eq!(Roundness::new(0.0).unwrap().class(), "very angular");
        assert_eq!(Roundness::new(0.2).unwrap().class(), "angular");
        assert_eq!(Roundness::new(0.3).unwrap().class(), "sub-angular");
        assert_eq!(Roundness::new(0.5).unwrap().class(), "sub-rounded");
        assert_eq!(Roundness::new(0.7).unwrap().class(), "rounded");
        assert_eq!(Roundness::new(1.0).unwrap().class(), "well rounded");
        assert!(Roundness::new(-0.01).is_err());
        assert!(Roundness::new(1.01).is_err());
        assert!(Roundness::new(f64::NAN).is_err());
        // A well-rounded clast has no relief at all; a very angular one has the
        // whole half-subvoxel. Derived from the index, chosen by nobody.
        let sharp = Boulder::new((0.4, 0.38, 0.36), Roundness::new(0.0).unwrap(), ROCK, ColumnMode::Physical).unwrap();
        let smooth = Boulder::new((0.4, 0.38, 0.36), Roundness::new(1.0).unwrap(), ROCK, ColumnMode::Physical).unwrap();
        assert_eq!(sharp.relief_subvoxels(), voxel_model::MAX_DISPLACEMENT_SUBVOXELS);
        assert_eq!(smooth.relief_subvoxels(), 0.0);
    }

    #[test]
    fn a_boulder_carves_the_cells_outside_its_form() {
        // A clast whose axes are the block's: a sphere of radius 0.5 centred in the
        // block, touching every face at its middle. (A sphere *inscribed* in a block
        // leaves far more than the block's eight corners out — a cell centre at 0.375
        // on two axes and 0.125 on the third is 0.5449 from the middle — so the test
        // re-derives the count from the law instead of pinning a number.)
        let boulder =
            Boulder::new((1.0, 1.0, 1.0), Roundness::new(0.8).unwrap(), ROCK, ColumnMode::Physical)
                .unwrap();
        let set = ShaperSet::new().with_physical(boulder);
        let out = set.apply(&solid()).unwrap();
        let outside = SubvoxelIndex::all()
            .filter(|c| {
                let d2: f64 = Axis::ALL
                    .iter()
                    .map(|axis| centred(coord_of(*c, *axis)).powi(2))
                    .sum();
                d2.sqrt() > voxel_model::MAX_RADIUS
            })
            .count();
        assert_eq!(out.cells().iter().filter(|m| m.is_empty()).count(), outside);
        assert!(outside > 0 && outside < 64, "a sphere is a shape, not the block: {outside}");
        // The block's own corner goes and its middle stays; a cell with two long axes
        // and one short one goes too, because the body is round and not a cube.
        assert!(out.get(at(0, 0, 0)).is_empty());
        assert_eq!(out.get(at(1, 1, 1)), ROCK);
        assert!(out.get(at(0, 0, 1)).is_empty());
    }

    // ----------------------------------------------------- the 256-only shaper

    #[test]
    fn flowstone_declares_a_ripple_and_carves_nothing_which_is_what_the_second_level_is_for() {
        let flowstone = Flowstone::new(ROCK, 0.3, 0.25).unwrap();
        let set = ShaperSet::new().with_physical(flowstone);
        assert_eq!(set.tally().physical_detail, 1);
        assert_eq!(set.tally().physical_subvoxel, 0, "no 4x4x4 work at all");
        let (displacement, _) = set.declare().unwrap();
        assert_eq!(displacement.len(), 6, "declared on all six faces");
        assert_eq!(displacement.profiled_faces(), 0, "a band, not a law");
        // And the 4x4x4 half leaves the block exactly as it found it, because this
        // shaper's whole statement is that a cube cannot express a ripple.
        assert_eq!(set.apply(&solid()).unwrap(), VoxelShape::solid(ROCK));
        assert!(Flowstone::new(ROCK, 0.3, 0.0).is_err());
    }

    #[test]
    fn a_slab_claims_the_addressed_level_only_when_it_has_work_there() {
        // `Flowstone` declares only at 256, so it is the shaper that would fail the
        // set's generic check if its declaration were dropped.
        let set = ShaperSet::new().with_physical(Flowstone::new(ROCK, 0.3, 0.25).unwrap());
        set.declare().unwrap();

        // A `Slab` on a grid-aligned cut used to **claim** `Level::Detail` and declare
        // nothing, and `ShaperSet::declare` refused it by name. The claim was the
        // mistake: a cut that lands on the grid has no addressed work, and its mask is
        // now derived from its own residual rather than asserted.
        let aligned = ShaperSet::new().with_physical(Slab::new(Cut::half(Axis::Z), ROCK).unwrap());
        assert_eq!(aligned.shapers()[0].acts_at(), LevelMask::SUBVOXEL);
        assert!(!aligned.shapers()[0].acts_at().has(Level::Detail));
        let (displacement, _) = aligned.declare().unwrap();
        assert!(displacement.is_empty());

        // And an off-grid cut claims both levels and pays for both.
        let parametric =
            ShaperSet::new().with_physical(Slab::new(Cut::slab(0, 96).unwrap(), ROCK).unwrap());
        assert_eq!(parametric.shapers()[0].acts_at(), LevelMask::BOTH);
        let (field, _) = parametric.declare().unwrap();
        assert_eq!(field.amplitude(ROCK, Face::PosZ), Some(-0.5));
    }

    // ---------------------------------------------------------------- the coral

    #[test]
    fn branching_coral_is_several_bodies_and_a_single_profile_cannot_be_it() {
        // The shape that made "one shaper, one profile" insufficient.
        let branching = CoralForm::Branching.bodies(0.4).unwrap();
        assert_eq!(branching.len(), 4);
        let columnar = CoralForm::Columnar.bodies(0.4).unwrap();
        assert_eq!(columnar.len(), 1);

        let coral = Coral::new(CoralForm::Branching, 0.4, 0.1, ROCK, INSET).unwrap();
        // A branch runs near a corner of the face, so the corners survive and the
        // face's centre between the branches does not.
        let centre_outside = coral.is_inside(at(1, 1, 0)).unwrap();
        let corner_inside = coral.is_inside(at(0, 0, 0)).unwrap();
        assert!(!centre_outside, "the gap between branches is not coral");
        assert!(corner_inside, "a branch sits near a corner");
        // A "coral" of no material is refused, and so is a corallite of no size.
        assert!(Coral::new(CoralForm::Massive, 0.4, 0.1, ROCK, MaterialId::EMPTY).is_err());
        assert!(Coral::new(CoralForm::Massive, 0.4, 0.0, ROCK, INSET).is_err());
        // Every form builds, which is the roster claim.
        for form in CoralForm::ALL {
            assert!(!form.bodies(0.3).unwrap().is_empty(), "{}", form.name());
        }
        // And the tabulate form uses the stalagmite law's own flat cap.
        let tabulate = CoralForm::Tabulate.bodies(0.4).unwrap();
        match tabulate[0].2 {
            Profile::Stalagmite { da, .. } => {
                assert!(da > 1.0);
                assert!(tabulate[0].2.flat_cap_radius() > 0.0);
                assert_eq!(tabulate[0].2.regime(), Regime::FlatTop);
            }
            _ => panic!("a tabulate coral should be the flat-capped law"),
        }
    }

    // ------------------------------------------------------------------- the set

    #[test]
    fn the_standard_set_names_its_sources_and_is_deterministic_in_seed_and_tick() {
        let set = standard(ROCK, INSET, SEAM).unwrap();
        set.audit().unwrap();
        for s in set.shapers() {
            assert!(s.source().len() > 60, "{} has a thin source", s.name());
        }
        let a = set.apply(&solid()).unwrap();
        let b = set.apply(&solid()).unwrap();
        assert_eq!(a, b);
        for (seed, tick) in [(1u64, 0u64), (99, 7), (u64::MAX, u64::MAX)] {
            let ctx = VoxelContext::new(
                [0, 0, 0],
                VoxelShape::solid(ROCK),
                Neighbourhood::exposed(),
                seed,
                tick,
            );
            assert_eq!(set.apply(&ctx).unwrap(), a, "seed {seed}, tick {tick}");
        }
        // And the two kinds are both present, which is what makes it a starter set
        // rather than one shape.
        assert_eq!(set.kinds(), (0, 2));
    }

    #[test]
    fn a_column_of_nothing_is_refused_and_so_is_a_flat_body() {
        assert!(Column::pillar(MaterialId::EMPTY, INSET).is_err());
        let flat = Column::new(Axis::Z, Profile::Flat, ROCK, INSET, ColumnMode::Material);
        assert!(flat.is_err());
        assert!(flat.unwrap_err().detail().contains("no outside"));
    }

    #[test]
    fn plain_is_the_shape_of_a_voxel_the_shapers_have_nothing_to_say_about() {
        assert_eq!(plain(ROCK), VoxelShape::solid(ROCK));
        assert!(plain(MaterialId::EMPTY).is_empty());
        assert!(OPEN.is_empty());
    }

    #[test]
    fn the_axis_helpers_agree_with_the_model_s_own_face_order() {
        assert_eq!(Axis::Z.others(), (Axis::X, Axis::Y));
        assert_eq!(Axis::X.others(), (Axis::Y, Axis::Z));
        assert_eq!(Axis::Z.faces(), (Face::NegZ, Face::PosZ));
        assert_eq!(Axis::X.faces(), (Face::NegX, Face::PosX));
        assert_eq!(cell_centre(0), 0.125);
        assert_eq!(cell_centre(3), 0.875);
        assert!((radial_offset(at(0, 0, 0), Axis::Z) - (0.375f64 * 0.375 * 2.0).sqrt()).abs() < 1e-12);
        assert_eq!(axial_fraction(at(0, 0, 0), Axis::Z), 0.125);
        // The mask is the mask, and a body that only carves does not claim 256.
        let material = Column::pillar(ROCK, INSET).unwrap();
        assert_eq!(<Column as MaterialShaper>::acts_at(&material), LevelMask::SUBVOXEL);
        assert_eq!(<Column as PhysicalShaper>::acts_at(&material), LevelMask::SUBVOXEL);
        assert_eq!(MAX_RADIUS, 0.5);
    }
}
