//! **The 1/32 profile: the surface law between a block's shape and its shading.**
//!
//! A voxel holds a 4×4×4 shape ([`crate::subvoxel`]) and the appearance layer shades
//! its faces through a 16×16 detail lattice. Neither of those is a *surface law*: the
//! 4×4×4 grid is 64 cells, coarse enough that a 40 cm trunk reads as a staircase, and
//! the detail lattice is shading, addressed rather than stored, and carries no
//! silhouette. `docs/AUDIT-2026-10-24` names the gap and this module fills it.
//!
//! Thirty-two divisions per block is the world's own finest surface denominator —
//! [`crate::coord::FILL_SIDE`], eight subvoxels to the unit, 31.25 mm at 1 m voxels.
//! A profile here is **evaluated, never stored** (C-2): a body is an analytic
//! cross-section ([`crate::profile::Profile`]) plus the few numbers that place it, and
//! [`Shaped::coverage`] answers *"how much of this 1/32 cell is inside the body"* for
//! any cell the renderer asks about. A dense 32³ grid would be 32 768 cells — 4 KiB as
//! bits — per block, which is not a tree renderer; the law is four multiplies and a
//! square root per sample.
//!
//! # What a shaper profile is, and what it is not
//!
//! `docs/GAIA_comprehensive.md` §5 draws the line: a profile is an **envelope**, not
//! biomass. This module keeps that line in the types. Nothing here returns mass, and a
//! coverage of one in a cell means only that the cell is **inside the declared body**.
//!
//! It is also not the shape the physics reads. Movement, collision and the light solver
//! see the 4×4×4 shape; a profile at 1/32 refines what is *drawn* inside a block that
//! the coarse shape already says is occupied. A tree therefore cannot be walked through
//! because its profile is thin in a cell — the body's coarse cells are still solid.
//!
//! # Growth is continuous, so animating it needs no rebuild
//!
//! [`Shaped::stage`] is the developmental stage in 1/1024ths, and every dimension of
//! the body is a function of it: a tree growing from stage 0.4 to 0.5 is the *same*
//! record with one integer moved, and the same block geometry re-drawn. Between two
//! stages a renderer blends the two coverages ([`blend`]) rather than rebuilding a
//! mesh, which is what "animated growth blending" costs: two evaluations per sample.

use crate::coord::FILL_SIDE;
use crate::profile::Profile;

/// Divisions per block axis. Deliberately [`FILL_SIDE`]: a fluid's fill levels and a
/// profile's cells are the same denominator, so "half full" and "half a trunk" mean
/// the same fraction of a block.
pub const PROFILE_SIDE: u32 = FILL_SIDE;

/// Cells in one block's 1/32 grid, if anyone ever asked for all of them. Nothing
/// stores them; the number is here so a caller that walks the grid cannot pretend the
/// walk is cheap.
pub const PROFILE_CELLS: usize = (PROFILE_SIDE as usize).pow(3);

/// One profile cell, in block units.
pub const PROFILE_CELL_M: f64 = 1.0 / PROFILE_SIDE as f64;

/// A developmental stage of one, in the fixed point [`Shaped::stage`] carries.
pub const STAGE_ONE: u16 = 1024;

/// Which axis a body's own "up" is.
///
/// A trunk is vertical in the world, but a branch is not, and a profile must be able
/// to describe one. The block frame is the only frame here; a rotated organ is a body
/// this world cannot yet address, and saying so is better than a quaternion nobody
/// applies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// The axis's own unit vector.
    pub const fn unit(self) -> [f64; 3] {
        match self {
            Axis::X => [1.0, 0.0, 0.0],
            Axis::Y => [0.0, 1.0, 0.0],
            Axis::Z => [0.0, 0.0, 1.0],
        }
    }

    /// Which of the two in-plane coordinates runs with `x`, `y`, `z`'s order, so a
    /// caller can turn a cell into an axial and a radial pair without a matrix.
    const fn planes(self) -> (usize, usize) {
        match self {
            Axis::X => (1, 2),
            Axis::Y => (0, 2),
            Axis::Z => (0, 1),
        }
    }

    const fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

/// **One block of one body**, as a surface law.
///
/// The record is the whole per-block cost: a [`Profile`] carried by value, the axis it
/// runs along, how long and how fat the body is at a full stage, the two surface
/// channels, a seed and the stage. It is what a chunk mesh or an instance buffer would
/// carry per profiled block, which is why it holds no grid.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shaped {
    /// The cross-section law, in block units.
    pub profile: Profile,
    /// The axis the body runs along inside this block.
    pub axis: Axis,
    /// Where the axis sits in the block's two in-plane coordinates, in block units.
    /// A body centred in its block is `[0.5, 0.5]`; a trunk standing at a block
    /// corner is `[0.0, 0.0]`, and a tall trunk spanning several blocks carries a
    /// different centre in each of them.
    pub centre: [f64; 2],
    /// How far along the axis the body reaches at stage one: `1.0` is the whole block.
    pub span: f64,
    /// The profile's own radius at stage one, as a multiple of the law's radius.
    /// A species' girth; `1.0` is the profile as declared.
    pub girth: f64,
    /// Foliage's gap fraction: the share of the inside that is *not* filled. Wood is
    /// zero; a crown uses the measured range named in `docs/trees.md`.
    pub void: f64,
    /// Amplitude of the radial flute, as a fraction of the radius. Bark and ridges
    /// are a surface feature at this resolution; wood without a flute is zero.
    pub ridge: f64,
    /// The body's stable seed: the flutes and the foliage gaps must not move when the
    /// same body is drawn again.
    pub seed: u64,
    /// The developmental stage, in 1/[`STAGE_ONE`]ths.
    pub stage: u16,
}

impl Shaped {
    /// A body that fills the block along `axis`, as the profile declares it: no girth
    /// change, solid, smooth, at a full stage.
    pub fn of(profile: Profile, axis: Axis) -> Self {
        Shaped {
            profile,
            axis,
            centre: [0.5, 0.5],
            span: 1.0,
            girth: 1.0,
            void: 0.0,
            ridge: 0.0,
            seed: 0,
            stage: STAGE_ONE,
        }
    }

    /// The stage as a fraction in `[0, 1]`.
    pub fn stage_fraction(&self) -> f64 {
        (self.stage.min(STAGE_ONE) as f64) / STAGE_ONE as f64
    }

    /// The same body at another stage.
    pub fn grown(&self, stage: u16) -> Self {
        Shaped { stage: stage.min(STAGE_ONE), ..*self }
    }

    /// How far along the axis the body reaches, in block units.
    ///
    /// A growing tree is the same form at every stage: both its length and its radius
    /// scale with the stage, so its silhouette is unchanged and only its size moves.
    /// A species whose girth grows faster than its height says so in [`Self::girth`],
    /// which is declared data and not a law this module invents.
    pub fn reach(&self) -> f64 {
        self.span.clamp(0.0, 1.0) * self.stage_fraction()
    }

    /// The body's radius at an axial fraction `along` of its **grown** reach.
    pub fn radius_at(&self, along: f64) -> f64 {
        let law = self.profile.cross_section_radius(along.clamp(0.0, 1.0));
        law * self.girth * self.stage_fraction()
    }

    /// Split a block-unit position into `(axial fraction, distance from the axis)`.
    fn axial_and_radial(&self, at: [f64; 3]) -> (f64, f64) {
        let (i, j) = self.axis.planes();
        let axial = at[self.axis.index()];
        let reach = self.reach();
        let along = if reach > 0.0 { axial / reach } else { 0.0 };
        let di = at[i] - self.centre[0];
        let dj = at[j] - self.centre[1];
        (along, (di * di + dj * dj).sqrt())
    }

    /// **How much of the 1/32 cell at `at` is inside this body**, in `[0, 1]`.
    ///
    /// `at` is the cell's centre in block units, so a caller that walks the grid passes
    /// `(i + 0.5) / 32`. The edges are anti-aliased over one cell: a cell wholly inside
    /// answers one, one wholly outside answers zero, and the half-covered cells in
    /// between answer their share, which is what keeps a profile's silhouette from
    /// stepping at 32 divisions.
    ///
    /// [`Self::void`] then removes a deterministic share of the inside — a crown is an
    /// envelope with air in it, and the air must be the *same* air every time the body
    /// is drawn.
    pub fn coverage(&self, at: [f64; 3]) -> f64 {
        let (along, d) = self.axial_and_radial(at);
        let axial_edge = if self.reach() <= 0.0 {
            0.0
        } else {
            let width = PROFILE_CELL_M / self.reach();
            edge(along, width).min(edge(1.0 - along, width))
        };
        if axial_edge <= 0.0 {
            return 0.0;
        }
        let radius = self.radius_at(along) * (1.0 + self.ridge * (self.flute(at) - 0.5));
        let radial_edge = edge(radius - d, PROFILE_CELL_M);
        let inside = radial_edge.min(axial_edge);
        if self.void <= 0.0 {
            return inside;
        }
        if !self.filled(at) {
            return 0.0;
        }
        inside
    }

    /// [`Self::coverage`] for a cell index on the grid.
    pub fn cell(&self, u: u32, v: u32, w: u32) -> f64 {
        self.coverage(centre(u, v, w))
    }

    /// The coverage of a whole 4×4×4 subvoxel: the mean of its eight corner-profile
    /// cells. This is the bridge back to the coarse shape, and it is here so a caller
    /// can check a profile against the grid that holds it.
    pub fn subvoxel(&self, at: usize) -> f64 {
        let s = PROFILE_SIDE as usize / crate::coord::SUBVOXEL_SIDE as usize;
        let (ux, uy, uz) = (
            at % crate::coord::SUBVOXEL_SIDE as usize,
            (at / crate::coord::SUBVOXEL_SIDE as usize) % crate::coord::SUBVOXEL_SIDE as usize,
            at / (crate::coord::SUBVOXEL_SIDE as usize * crate::coord::SUBVOXEL_SIDE as usize),
        );
        let mut sum = 0.0;
        for k in 0..2 {
            for j in 0..2 {
                for i in 0..2 {
                    sum += self.cell(
                        (ux * s + i * (s - 1)) as u32,
                        (uy * s + j * (s - 1)) as u32,
                        (uz * s + k * (s - 1)) as u32,
                    );
                }
            }
        }
        sum / 8.0
    }

    /// **The outward normal at a cell**, for the half of shading this law owns.
    ///
    /// A body of revolution `r = R(t)` has `n ∝ r̂ − R′(t)·t̂` once normalised, which is
    /// the exact normal of the surface and not a bent approximation of a cube. Returns
    /// `None` on the axis, where the radial direction is undefined.
    pub fn normal(&self, at: [f64; 3]) -> Option<[f64; 3]> {
        let at = [at[0], at[1], at[2]];
        let (along, d) = self.axial_and_radial(at);
        if d <= f64::EPSILON {
            return None;
        }
        let (i, j) = self.axis.planes();
        let step = 1.0 / 256.0;
        let (di, dj) = (at[i] - self.centre[0], at[j] - self.centre[1]);
        let lo = self.radius_at((along - step).max(0.0));
        let hi = self.radius_at((along + step).min(1.0));
        let slope = (hi - lo) / (2.0 * step * self.reach().max(f64::EPSILON));
        let radial = [di / d, dj / d];
        let mut n = [0.0; 3];
        n[i] = radial[0];
        n[j] = radial[1];
        n[self.axis.index()] = -slope;
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len <= f64::EPSILON {
            return None;
        }
        Some([n[0] / len, n[1] / len, n[2] / len])
    }

    /// The flute channel: a stable value per angular segment, so a trunk's ridges do
    /// not crawl when the same body is drawn on another frame.
    fn flute(&self, at: [f64; 3]) -> f64 {
        let (i, j) = self.axis.planes();
        let angle = (at[j] - self.centre[1]).atan2(at[i] - self.centre[0]);
        let segment = ((angle + std::f64::consts::PI) / std::f64::consts::TAU * 16.0).floor();
        let h = splitmix(self.seed ^ (segment as u64) << 40);
        h as f64 / u64::MAX as f64
    }

    /// Whether this cell is one of the filled ones, under [`Self::void`].
    ///
    /// The gap is decided on a lattice of [`CLUMP`] cells, not on single cells: a crown
    /// whose air fell out one 31 mm cell at a time would dither into a grey veil, while
    /// a clump four cells across is 12.5 cm of leaf mass with air between the clumps,
    /// which is what foliage looks like from a distance and what its measured void
    /// fraction is actually a measurement of.
    fn filled(&self, at: [f64; 3]) -> bool {
        let key = clump_key(at);
        let h = splitmix(self.seed ^ key);
        (h as f64 / u64::MAX as f64) >= self.void
    }
}

/// **The animated blend between two stages**: the coverage of the body as the
/// renderer would draw it at `t` in `[0, 1]`.
///
/// This is not a third shape. A growing tree's intermediate is *between* two stages of
/// the same law, and the honest interpolation of a silhouette is its coverage, so the
/// answer at the endpoints is exactly the two stages and at any `t` it is the fraction
/// of the cell each stage covers. A renderer drawing a growth animation therefore never
/// rebuilds a block; it moves one number.
pub fn blend(from: &Shaped, to: &Shaped, t: f64, at: [f64; 3]) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let a = from.coverage(at);
    let b = to.coverage(at);
    a + (b - a) * t
}

/// The centre of a profile cell, in block units.
pub fn centre(u: u32, v: u32, w: u32) -> [f64; 3] {
    [
        (u.min(PROFILE_SIDE) as f64 + 0.5) * PROFILE_CELL_M,
        (v.min(PROFILE_SIDE) as f64 + 0.5) * PROFILE_CELL_M,
        (w.min(PROFILE_SIDE) as f64 + 0.5) * PROFILE_CELL_M,
    ]
}

/// The anti-aliased edge of a scalar field whose zero is the surface: `1` a full cell
/// inside, `0` outside, linear across the one cell the boundary crosses.
fn edge(signed_inside: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return if signed_inside >= 0.0 { 1.0 } else { 0.0 };
    }
    (signed_inside / width + 0.5).clamp(0.0, 1.0)
}

/// How many 1/32 cells across one leaf clump is. Twelve and a half centimetres, which
/// is coarse enough to read as a clump and fine enough to fill a crown envelope.
const CLUMP: u32 = 4;

/// A stable 64-bit hash of a position quantised to the clump lattice, so a clump's air
/// is a property of where it is and not of the order it was visited in.
fn clump_key(at: [f64; 3]) -> u64 {
    let q = |v: f64| (v / (PROFILE_CELL_M * CLUMP as f64)).floor() as i64 as u64;
    splitmix(q(at[0]) ^ q(at[1]).rotate_left(21) ^ q(at[2]).rotate_left(42))
}

/// SplitMix64's finaliser: deterministic, and the only randomness in this module.
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trunk(radius: f64) -> Shaped {
        Shaped::of(Profile::Cylinder { radius }, Axis::Y)
    }

    #[test]
    fn a_trunk_is_round_at_thirty_seconds_and_not_a_cube() {
        // A 0.3-block radius column: 19 cells across at 1/32, which is enough for a
        // circle to be a circle and a square to be visibly wrong. The corner of the
        // block is outside; the axis is inside.
        let body = trunk(0.3);
        assert_eq!(body.cell(16, 16, 16), 1.0, "the axis of the trunk is inside it");
        assert_eq!(body.cell(1, 1, 1), 0.0, "the corner of the block is outside it");
        // A cell at the surface: radius 0.3 is cell 9.6, so cell 9 is half covered and
        // cell 12 is entirely inside.
        let half = body.cell(25, 16, 16);
        assert!(half > 0.0 && half < 1.0, "the boundary cell is a share: {half}");
        assert_eq!(body.cell(23, 16, 16), 1.0, "inside the surface");
        assert_eq!(body.cell(26, 16, 16), 0.0, "outside the surface");
        assert_eq!(body.cell(0, 16, 16), 0.0, "the block's own edge is outside it");
    }

    #[test]
    fn a_growing_tree_keeps_its_form_and_only_changes_size() {
        // The same record at two stages: both its reach and its radius scale, so the
        // silhouette is the same shape at half the size.
        let full = trunk(0.3);
        let half = full.grown(STAGE_ONE / 2);
        assert!((half.reach() - 0.5).abs() < 1e-12, "half the height at half the stage");
        assert!((half.radius_at(0.5) - 0.15).abs() < 1e-12, "and half the radius");
        assert!(
            (half.radius_at(0.5) / half.reach() - full.radius_at(0.5) / full.reach()).abs() < 1e-12,
            "the form is the same at both stages"
        );
        // And the drawn occupancy agrees: a cell just inside the young trunk is inside
        // it and outside the full one's surface radius.
        assert_eq!(half.cell(18, 8, 16), 1.0, "a cell inside the young trunk");
        assert_eq!(half.cell(16, 8, 16), 1.0, "the young trunk's base is still solid");
        assert_eq!(half.cell(16, 30, 16), 0.0, "above a half-grown trunk there is nothing");
        assert_eq!(full.cell(16, 30, 16), 1.0, "and the full trunk reaches there");
    }

    #[test]
    fn the_blend_is_the_two_stages_at_its_ends_and_between_them_in_the_middle() {
        let young = trunk(0.3).grown(STAGE_ONE / 4);
        let old = trunk(0.3);
        let at = centre(16, 12, 16);
        assert_eq!(blend(&young, &old, 0.0, at), young.coverage(at));
        assert_eq!(blend(&young, &old, 1.0, at), old.coverage(at));
        let middle = blend(&young, &old, 0.5, at);
        assert!(middle > young.coverage(at) && middle < old.coverage(at));
    }

    #[test]
    fn the_void_channel_leaves_the_declared_share_of_the_envelope_empty() {
        // A crown is an envelope with air in it. The declared 0.9 gap must show up as
        // roughly a tenth of the inside being filled — and it must be the same tenth
        // on the next evaluation, or the crown would boil as it is drawn.
        let mut crown = Shaped::of(Profile::Spheroid { radius: 0.5, height: 0.5 }, Axis::Y);
        crown.void = 0.9;
        let envelope = Shaped { void: 0.0, ..crown };
        // Averaged over eight bodies, because one body is ~200 clumps and a tenth of
        // 200 is a coin toss with a wide spread; the law is the share, not one draw.
        let mut share = 0.0;
        for seed in 0..8u64 {
            crown.seed = 0x5EED + seed;
            let mut inside = 0usize;
            let mut filled = 0usize;
            for w in 0..PROFILE_SIDE {
                for v in 0..PROFILE_SIDE {
                    for u in 0..PROFILE_SIDE {
                        if envelope.cell(u, v, w) > 0.0 {
                            inside += 1;
                            filled += usize::from(crown.cell(u, v, w) > 0.0);
                        }
                    }
                }
            }
            assert!(inside > 0, "a crown the size of the block has cells in it");
            share += filled as f64 / inside as f64 / 8.0;
        }
        assert!((share - 0.1).abs() < 0.02, "the filled share of the envelope is {share}, not 0.1");
        // The same cell, asked twice, is the same cell: the hash is of the position.
        assert_eq!(crown.cell(16, 16, 16), crown.cell(16, 16, 16));
    }

    #[test]
    fn a_thin_body_is_absent_from_the_coarse_subvoxel_it_would_have_to_fill() {
        // The bridge back to the 4×4×4 shape: a two-cell-wide twig averages to a small
        // share of its subvoxel, which is the number a caller checks against the coarse
        // grid rather than pretending the twig is a solid block.
        let twig = Shaped::of(Profile::Cylinder { radius: 0.06 }, Axis::Y);
        let thin = twig.subvoxel(42);
        assert!(thin < 0.75, "a 12 cm twig is not most of a 25 cm subvoxel: {thin}");
    }

    #[test]
    fn the_normal_is_the_profiles_own_and_not_a_bend() {
        // On a cylinder the exact normal is radial: no axial term, unit length, and on
        // the +x side it points along +x.
        let body = trunk(0.3);
        let n = body.normal([0.8, 0.5, 0.5]).expect("off the axis");
        assert!((n[0] - 1.0).abs() < 1e-9 && n[1].abs() < 1e-9 && n[2].abs() < 1e-9, "{n:?}");
        assert!(body.normal([0.5, 0.5, 0.5]).is_none(), "the axis has no radial direction");
        // A frustum leans: its normal carries the taper's axial term.
        let cone = Shaped::of(Profile::Frustum { low_radius: 0.4, high_radius: 0.1 }, Axis::Y);
        let leaned = cone.normal([0.8, 0.5, 0.5]).expect("off the axis");
        assert!(leaned[1] > 0.0, "a narrowing column's normal tips upward: {leaned:?}");
    }
}
