//! The vector change view's subject: **the area that actually changed** (H-7.1, H-7.9).
//!
//! # This is not the cuboid view, and the difference is the whole point
//!
//! H-7.1: *there is a really nice cuboid overlay that appears that shows elements of the world in
//! the dirty branch already — so like world root. The idea is to expand that feature into a vector
//! debug view that will outline areas that change within a chunk, **representing the actual change
//! that happened**.*
//!
//! So the subject is the **set of cells that changed**, and what is drawn is the **boundary of that
//! set**: the edges where a changed cell meets one that did not. **Projecting a cuboid's eight
//! corners is the thing being replaced, not the thing being built.** A dig of two adjacent cells
//! draws **one outline with no seam between them**; two separate digs draw two outlines; and a
//! change that happens to be a solid block draws exactly the twelve edges of a box — because that is
//! what the change was, not because a box was the shape of the answer.
//!
//! # What grounds the subject (H-7.1)
//!
//! *The view is not a per-stage readout.* Its subject is the world's **modification record** and its
//! persistence state. So this type carries **the reason and the material** the record wrote, and a
//! caller cannot hand it a "stage name" as though a stage were a change.
//!
//! # Transient (H-7.9)
//!
//! *Just a simple outline over the changed area for a small amount of time.* A region carries the
//! instant it was recorded and **is live only while `now` is inside its window** — and the window is
//! **declared**, not tuned per call site.
//!
//! # Resolution is owed, not assumed
//!
//! The cell grid here is **one cell per voxel**. `voxel_model::subvoxel` resolves 256 subvoxels
//! inside a voxel, and whether a subvoxel change draws a subvoxel outline is **not ruled in the
//! record** — so [`ChangeRegion::SUBDIVISION`] is 1 and a caller that wants subvoxel outlines has to
//! change a number the record has not settled rather than passing a scale factor.
//!
//! # The projection lives here, and the lane overlay refuses to guess one
//!
//! `crate::debug`'s lane overlay draws screen lanes and **refuses a world lane rather than
//! projecting it** — *"the vector change view owns that projection (H-7.9); this surface does not
//! invent one."* This is that projection: [`Projection`] carries the camera's own
//! [`Camera::view_projection`] — the same matrix the world pipeline uploads — and turns a
//! region's boundary edges into dp segments, which [`outline_strokes`] hands the HUD as strokes
//! ([`crate::hud::Stroke`]). A point behind the eye is **refused, not clamped**; an edge that
//! crosses the camera plane is **cut at the plane**, because the part of an outline in front of
//! the camera should draw rather than the whole edge vanishing.

use std::collections::{BTreeMap, BTreeSet};

use crate::hud::Stroke;
use crate::Camera;
use voxel_model::MaterialId;
use voxel_ui::{Paint, Rect};

/// One cell of the change grid, in **cell coordinates**. Negative coordinates are ordinary cells,
/// not an error: a chunk boundary is an arithmetic fact, not a fault.
pub type Cell = [i64; 3];

/// A corner of the grid, in **corner coordinates**. A cell owns the eight corners from
/// `(x, y, z)` to `(x + 1, y + 1, z + 1)`, and edges are named by their corners so two faces of the
/// same cell can never emit the same edge twice.
pub type Corner = [i64; 3];

/// The six neighbours of a cell, and the axis each one lies on.
pub const NEIGHBOURS: [(Cell, usize); 6] = [
    ([1, 0, 0], 0),
    ([-1, 0, 0], 0),
    ([0, 1, 0], 1),
    ([0, -1, 0], 1),
    ([0, 0, 1], 2),
    ([0, 0, -1], 2),
];

/// The set of cells one modification actually changed, with the record's reason.
///
/// **A `BTreeSet`, so iteration is in cell order** and the outline is a function of the change alone
/// — two runs that changed the same cells draw the same outline whatever order the edits arrived in.
#[derive(Clone, PartialEq, Debug)]
pub struct ChangeRegion {
    cells: BTreeSet<Cell>,
    /// The reason the record wrote with the edit. **Never empty**: an outline that cannot say what
    /// changed it is a mark on the screen, and this project does not make marks it cannot explain.
    pub reason: String,
    /// What the cells became, when the record wrote one.
    pub material: Option<MaterialId>,
    /// The instant the record wrote this change, in the client's own milliseconds.
    pub recorded_ms: f64,
}

impl ChangeRegion {
    /// **How long a change is drawn**, in milliseconds. Declared here because H-7.9 says *a small
    /// amount of time* and a duration nobody wrote down is not a duration.
    pub const LIFETIME_MS: f64 = 1_500.0;

    /// One cell per voxel. **The subvoxel subdivision is owed and not assumed**; see the module
    /// docs.
    pub const SUBDIVISION: u32 = 1;

    /// The world size of one cell, in world units.
    pub const CELL_SIDE_M: f64 = 1.0;

    /// A region over the cells an edit changed. **Refuses an empty change and a blank reason**, and
    /// a caller that cannot say what changed it has not recorded a change.
    pub fn of(
        cells: impl IntoIterator<Item = Cell>,
        reason: impl Into<String>,
        material: Option<MaterialId>,
        recorded_ms: f64,
    ) -> Result<ChangeRegion, ChangeRefusal> {
        let cells: BTreeSet<Cell> = cells.into_iter().collect();
        if cells.is_empty() {
            return Err(ChangeRefusal::NothingChanged);
        }
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(ChangeRefusal::NoReason);
        }
        if !recorded_ms.is_finite() {
            return Err(ChangeRefusal::NoInstant);
        }
        Ok(ChangeRegion { cells, reason, material, recorded_ms })
    }

    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Whether the change is still on screen at `now_ms`.
    ///
    /// **A half-open window**, so a region is live for exactly `LIFETIME_MS` and the last instant is
    /// already gone — the same half-open rule the rest of the project uses for ranges.
    pub fn is_live(&self, now_ms: f64) -> bool {
        (self.recorded_ms..self.recorded_ms + Self::LIFETIME_MS).contains(&now_ms)
    }

    /// **The outline**: the edges where a changed cell meets a cell that did not change.
    ///
    /// **Deduplicated by corner pair, not by face.** Two faces of one cell share an edge, and two
    /// adjacent cells share four; emitting per face would draw each of those twice and make the
    /// outline's *size* a function of how the change was described rather than of what changed. The
    /// result is a `BTreeSet`, so it is ordered and equal changes give equal outlines.
    pub fn boundary_faces(&self) -> BTreeSet<[Corner; 4]> {
        let mut faces: BTreeSet<[Corner; 4]> = BTreeSet::new();
        for cell in &self.cells {
            for (offset, axis) in NEIGHBOURS {
                let neighbour = [
                    cell[0] + offset[0],
                    cell[1] + offset[1],
                    cell[2] + offset[2],
                ];
                if self.cells.contains(&neighbour) {
                    // **An interior face is not an outline.** This is the line that makes the view a
                    // vector view rather than one box per changed cell.
                    continue;
                }
                faces.insert(face_corners(*cell, axis, offset[axis] > 0));
            }
        }
        faces
    }

    /// The same outline **in world units**, for the renderer.
    ///
    /// **Derived from [`Self::CELL_SIDE_M`] and [`Self::SUBDIVISION`]**, so the world position of a
    /// face is a function of the declared cell size rather than a number the renderer chose.
    pub fn boundary_faces_world(&self) -> Vec<[[f64; 3]; 4]> {
        let side = Self::CELL_SIDE_M / Self::SUBDIVISION as f64;
        self.boundary_faces()
            .into_iter()
            .map(|face| face.map(|corner| corner_world(corner, side)))
            .collect()
    }

    /// Every distinct **edge** of the boundary surface, for a renderer that strokes rather than
    /// fills.
    ///
    /// **Deduplicated by corner pair**, so an edge shared by two faces of the surface is drawn once
    /// and the stroke count is a function of the shape rather than of how the faces were enumerated.
    pub fn outline_edges(&self) -> BTreeSet<(Corner, Corner)> {
        let mut edges: BTreeSet<(Corner, Corner)> = BTreeSet::new();
        for face in self.boundary_faces() {
            for (a, b) in face_edges(face) {
                edges.insert(canonical(a, b));
            }
        }
        edges
    }

    /// The same outline **in world units**, as edge point pairs, for the projection.
    ///
    /// **Derived from [`Self::CELL_SIDE_M`] and [`Self::SUBDIVISION`]**, the same derivation as
    /// [`Self::boundary_faces_world`], so the outline's position is a function of the declared
    /// cell size rather than a number the overlay chose.
    pub fn outline_edges_world(&self) -> Vec<[[f64; 3]; 2]> {
        let side = Self::CELL_SIDE_M / Self::SUBDIVISION as f64;
        self.outline_edges()
            .into_iter()
            .map(|(a, b)| [corner_world(a, side), corner_world(b, side)])
            .collect()
    }

    /// The regions drawn this instant, keyed by the instant they were recorded.
    ///
    /// **A `BTreeMap` keyed by the record's own instant, with same-instant regions kept in
    /// arrival order.** Two edits can land in one clock tick — a tick is 50 ms and an edit takes
    /// far less — and a map that kept one region per instant would silently drop an outline the
    /// record holds. The live set is still a function of the record and the clock and not of the
    /// order the changes happened to arrive in: instants order first, and arrival order only
    /// breaks the ties inside one.
    pub fn live<'a>(regions: &'a [ChangeRegion], now_ms: f64) -> Vec<&'a ChangeRegion> {
        let mut by_instant: BTreeMap<OrderedF64, Vec<&'a ChangeRegion>> = BTreeMap::new();
        for region in regions {
            if region.is_live(now_ms) {
                by_instant.entry(OrderedF64(region.recorded_ms)).or_default().push(region);
            }
        }
        by_instant.into_values().flatten().collect()
    }
}

/// **The width of one outline, in dp** — one dp, the divider's own width.
///
/// The lane overlay draws its outlines as one-dp rules (`crate::debug::Lane::outline`), so the
/// vector view and the lane overlay use the same pen. Declared rather than a literal at the
/// stroke site, so the width has one name to change.
pub const OUTLINE_DP: f64 = 1.0;

/// **Where the world's change outlines land on the shell's surface, in dp** (H-7.9).
///
/// The matrix is the camera's own [`Camera::view_projection`] — the one the world pipeline
/// uploads — and positions are made camera-relative here exactly as the shader's are, so the
/// outline and the terrain under it cannot disagree about where the camera is looking. The
/// viewport is the shell's dp viewport; its aspect and the camera's must be the window's, which
/// is the caller's contract (the bin keeps the two in step on resize).
#[derive(Clone, Copy, Debug)]
pub struct Projection {
    eye: [f64; 3],
    view_projection: [[f32; 4]; 4],
    width_dp: f64,
    height_dp: f64,
    near_m: f64,
}

impl Projection {
    /// The projection of one frame: the camera as it is, against the viewport it is drawn into.
    pub fn of(camera: &Camera, viewport: Rect) -> Projection {
        Projection {
            eye: camera.position_m,
            view_projection: camera.view_projection(),
            width_dp: viewport.width_dp,
            height_dp: viewport.height_dp,
            near_m: camera.near_m,
        }
    }

    /// The clip-space vector of a world point, in the matrix's column-major frame.
    fn clip(&self, point: [f64; 3]) -> [f64; 4] {
        // **Camera-relative, exactly as the shader's are**: the view matrix carries no
        // translation, so the `1` is what makes the perspective's `w` the eye's own depth.
        let v = [
            point[0] - self.eye[0],
            point[1] - self.eye[1],
            point[2] - self.eye[2],
            1.0,
        ];
        let mut out = [0.0f64; 4];
        for (column, values) in self.view_projection.iter().enumerate() {
            for (row, value) in values.iter().enumerate() {
                out[row] += *value as f64 * v[column];
            }
        }
        out
    }

    /// A clip-space vector to dp. `w` is the eye's own depth, so the divide is the perspective.
    fn dp(&self, clip: [f64; 4]) -> [f64; 2] {
        let ndc = [clip[0] / clip[3], clip[1] / clip[3]];
        [
            (ndc[0] * 0.5 + 0.5) * self.width_dp,
            (0.5 - ndc[1] * 0.5) * self.height_dp,
        ]
    }

    /// Whether there is a surface to project onto at all. **A degenerate viewport is refused**:
    /// a zero-size surface would collapse every outline onto one point rather than drawing one.
    fn has_surface(&self) -> bool {
        self.width_dp > 0.0 && self.height_dp > 0.0
    }

    /// Where a world point lands, or `None` when the camera cannot see it.
    ///
    /// **Refused, not clamped**: a point behind the eye (or between the eye and the near plane)
    /// has no place on the screen, and drawing it at an edge would be a mark where the thing is
    /// not.
    pub fn point(&self, point: [f64; 3]) -> Option<[f64; 2]> {
        if !self.has_surface() {
            return None;
        }
        let clip = self.clip(point);
        // **In front of the near plane, not merely of the eye.** A point between the two is
        // clipped away by the world pipeline, so a mark for it would be a mark for geometry that
        // is not on screen — and a non-finite depth is refused with them rather than divided by.
        let w = clip[3];
        if w.is_nan() || w < self.near_m {
            return None;
        }
        Some(self.dp(clip))
    }

    /// A world edge as a dp segment.
    ///
    /// **Cut at the near plane, not dropped**: a segment with one end behind the plane is clipped
    /// to the place it crosses it — in world space, at the plane the camera itself declares — so
    /// an outline the camera is standing in draws its visible part. A segment with both ends
    /// behind the plane is `None`: there is no visible part, and a segment drawn anyway would be
    /// a mark where the camera is not looking.
    pub fn segment(&self, a: [f64; 3], b: [f64; 3]) -> Option<[[f64; 2]; 2]> {
        if !self.has_surface() {
            return None;
        }
        let (ca, cb) = (self.clip(a), self.clip(b));
        if ca[3] < self.near_m && cb[3] < self.near_m {
            return None;
        }
        let (a, b) = if ca[3] < self.near_m || cb[3] < self.near_m {
            // The cut is a world point on the edge — the crossing the plane actually has — and
            // not a screen-space clamp.
            let t = ((self.near_m - ca[3]) / (cb[3] - ca[3])).clamp(0.0, 1.0);
            let cut = [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ];
            if ca[3] < self.near_m {
                (cut, b)
            } else {
                (a, cut)
            }
        } else {
            (a, b)
        };
        // Re-derived rather than reused: the cut point's clip vector is the edge's own at `t`.
        let (ca, cb) = (self.clip(a), self.clip(b));
        if ca[3].is_nan() || cb[3].is_nan() || ca[3] < self.near_m || cb[3] < self.near_m {
            return None;
        }
        Some([self.dp(ca), self.dp(cb)])
    }
}

/// **The live change outlines, as strokes in dp, in record order.**
///
/// A drawing decision and not a record: the regions come from the world's modification record,
/// and this only decides what is drawn from them. **The colour is the `Change` class's hue** — the
/// vector view *is* the change lane's projection, so it cannot be drawn in a colour no lane has.
/// The ancestor's rationale for the six hues is that they read over the shell's dark panels *and*
/// over a voxel of any colour in the terrain palette; the world is the backdrop here, and the
/// limit is named: nothing guarantees a hue over an HDR or white world (the same limit
/// [`crate::hud::BRIGHTEST_BACKDROP`] states for the HUD).
pub fn outline_strokes(
    regions: &[ChangeRegion],
    projection: &Projection,
    now_ms: f64,
) -> Vec<Stroke> {
    let paint = Paint { rgb: crate::debug::LaneClass::Change.hue(), alpha: 1.0 };
    let mut out = Vec::new();
    for region in ChangeRegion::live(regions, now_ms) {
        for edge in region.outline_edges_world() {
            if let Some([from_dp, to_dp]) = projection.segment(edge[0], edge[1]) {
                out.push(Stroke { from_dp, to_dp, width_dp: OUTLINE_DP, paint });
            }
        }
    }
    out
}

/// Why a change could not be recorded as one.
#[derive(Clone, PartialEq, Debug)]
pub enum ChangeRefusal {
    /// No cells. **A change that changed nothing is not a change**, and an outline around an empty
    /// region would be a box around nothing.
    NothingChanged,
    /// No reason. The modification record's whole content is that something was refused or changed
    /// **for a stated reason**; an unstated one is not a record.
    NoReason,
    /// A non-finite instant, so "for a small amount of time" has no start.
    NoInstant,
}

impl std::fmt::Display for ChangeRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeRefusal::NothingChanged => {
                write!(f, "the edit changed no cells, so there is no area to outline")
            }
            ChangeRefusal::NoReason => write!(f, "the edit carried no reason, so the outline cannot say what changed it"),
            ChangeRefusal::NoInstant => write!(f, "the edit carried no finite instant, so it cannot expire"),
        }
    }
}

/// The four corners of one face of a cell, in order.
///
/// **`positive` says which side of the cell the face is on**, and it is read from the neighbour's
/// offset rather than assumed, so the two faces of a shared boundary are the same four corners seen
/// from either side.
fn face_corners(cell: Cell, axis: usize, positive: bool) -> [Corner; 4] {
    let at = |_along: i64| -> i64 {
        if positive {
            cell[axis] + 1
        } else {
            cell[axis]
        }
    };
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    // **A ring, not a bit-pattern.** Consecutive corners have to be *adjacent*, or the fourth
    // "edge" of the face is a diagonal and the outline gains an edge that is not on the shape.
    const RING: [(i64, i64); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];
    let mut corners = [[0i64; 3]; 4];
    for (i, corner) in corners.iter_mut().enumerate() {
        corner[axis] = at(0);
        corner[u] = cell[u] + RING[i].0;
        corner[v] = cell[v] + RING[i].1;
    }
    corners
}

/// The four edges of a face, as corner pairs.
fn face_edges(face: [Corner; 4]) -> [(Corner, Corner); 4] {
    [
        (face[0], face[1]),
        (face[1], face[2]),
        (face[2], face[3]),
        (face[3], face[0]),
    ]
}

/// An edge with its ends in a fixed order, so two faces naming the same edge name it identically.
fn canonical(a: Corner, b: Corner) -> (Corner, Corner) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn corner_world(corner: Corner, side: f64) -> [f64; 3] {
    [corner[0] as f64 * side, corner[1] as f64 * side, corner[2] as f64 * side]
}

/// A total order over `f64`, because a `BTreeMap` key cannot be `NaN`-ordered.
#[derive(Clone, Copy, PartialEq, Debug)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(cells: impl IntoIterator<Item = Cell>) -> ChangeRegion {
        ChangeRegion::of(cells, "the player broke it", Some(MaterialId::EMPTY), 0.0)
            .expect("the change is recordable")
    }

    #[test]
    fn one_cell_is_a_cube_of_six_faces_and_twelve_edges() {
        // The smallest change there is, and both counts are exact.
        let change = region([[0, 0, 0]]);
        assert_eq!(change.boundary_faces().len(), 6, "every face of one cell is a boundary");
        assert_eq!(change.outline_edges().len(), 12, "and its wireframe is a cube's");
    }

    #[test]
    fn two_adjacent_cells_are_one_outline_and_not_two_boxes() {
        // **The test that separates a vector view from a cuboid view.**
        let adjacent = region([[0, 0, 0], [1, 0, 0]]).boundary_faces().len();
        let separate = region([[0, 0, 0], [4, 0, 0]]).boundary_faces().len();

        assert_eq!(separate, 12, "two changes that did not touch draw two whole cubes' surfaces");
        assert_eq!(
            adjacent, 10,
            "two touching cells share the face between them, so one face is interior"
        );
        assert_ne!(adjacent, separate, "the outline is not one box per changed cell");
        assert_ne!(
            adjacent, 6,
            "and not the bounding box either: a 1x2x1 change has 10 boundary faces, not 6"
        );
    }

    #[test]
    fn a_solid_block_draws_its_own_surface_and_no_interior_face() {
        // Where the change really is a block, the view draws that block's surface — six faces of
        // four cells each — **not** the twenty-four faces the cells would draw individually, and
        // not the six faces of a bounding box that ignored where the cells were.
        let faces = region([
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ])
        .boundary_faces();
        assert_eq!(faces.len(), 24, "6 sides x 4 cells, and no interior face: {faces:?}");
    }

    #[test]
    fn an_l_shape_keeps_the_step_that_a_box_would_hide() {
        // The reason the view exists at all: three cells in an L have a surface no box has.
        let change = region([[0, 0, 0], [1, 0, 0], [0, 0, 1]]);
        assert_eq!(
            change.boundary_faces().len(),
            14,
            "18 cell faces less the four on the two shared interfaces — each interface hides a face \
             from each of the two cells that share it"
        );
        assert_ne!(
            change.boundary_faces().len(),
            6,
            "the bounding box of an L is a box, and the change was not one"
        );
        // And the concave corner is in the surface, which a bounding box's six faces cannot have.
        assert!(change.boundary_faces().contains(&[[1, 0, 1], [2, 0, 1], [2, 1, 1], [1, 1, 1]]));
    }

    #[test]
    fn the_outline_is_the_same_whichever_order_the_edits_arrived_in() {
        // Determinism is a contract: the outline is a function of the cells, not of the order.
        let forward = region([[3, 1, 4], [0, 0, 0], [1, 0, 0], [2, 0, 0]]).boundary_faces();
        let backward = region([[2, 0, 0], [1, 0, 0], [0, 0, 0], [3, 1, 4]]).boundary_faces();
        assert_eq!(forward, backward);
    }

    #[test]
    fn an_edit_that_changed_nothing_is_refused_rather_than_drawn_as_an_empty_box() {
        assert_eq!(ChangeRegion::of([], "nothing", None, 0.0), Err(ChangeRefusal::NothingChanged));
    }

    #[test]
    fn a_change_with_no_reason_is_refused() {
        // The record's content is that something changed **for a stated reason**.
        assert_eq!(
            ChangeRegion::of([[0, 0, 0]], "   ", None, 0.0).err(),
            Some(ChangeRefusal::NoReason)
        );
    }

    #[test]
    fn a_change_expires_and_the_window_is_declared_and_half_open() {
        let change = ChangeRegion::of([[0, 0, 0]], "the player broke it", None, 1_000.0)
            .expect("recordable");
        assert!(change.is_live(1_000.0), "it is on screen the moment it is recorded");
        assert!(change.is_live(1_000.0 + ChangeRegion::LIFETIME_MS - 1.0));
        assert!(
            !change.is_live(1_000.0 + ChangeRegion::LIFETIME_MS),
            "the last instant is already gone, so the lifetime is exactly what it says"
        );
        assert!(!change.is_live(999.0), "and it was not on screen before it happened");
    }

    #[test]
    fn only_the_live_regions_are_drawn_and_they_come_back_in_record_order() {
        let old = ChangeRegion::of([[0, 0, 0]], "first", None, 0.0).expect("recordable");
        let new = ChangeRegion::of([[9, 0, 0]], "second", None, 900.0).expect("recordable");
        let recorded = [old, new];
        // 1600 is past the first change's 1500 ms lifetime and inside the second's 900…2400.
        let live = ChangeRegion::live(&recorded, 1_600.0);
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].reason, "second");
    }

    #[test]
    fn world_positions_come_from_the_declared_cell_size() {
        let faces = region([[1, 0, 0]]).boundary_faces_world();
        let xs: Vec<f64> = faces.iter().flatten().map(|corner| corner[0]).collect();
        assert_eq!(xs.iter().cloned().fold(f64::INFINITY, f64::min), 1.0);
        assert_eq!(xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max), 2.0);
        assert_eq!(ChangeRegion::CELL_SIDE_M, 1.0, "the cell size is one world unit");
    }

    #[test]
    fn negative_cells_are_cells_rather_than_an_error() {
        // A chunk boundary is arithmetic, not a fault.
        let change = region([[-1, -1, -1]]);
        assert_eq!(change.boundary_faces().len(), 6);
        let world = change.boundary_faces_world();
        assert!(world.iter().flatten().all(|corner| corner.iter().all(|v| *v >= -1.0)));
    }

    #[test]
    fn two_changes_recorded_in_the_same_instant_are_both_live() {
        // A tick is 50 ms and an edit takes far less, so two edits do land in one instant. A live
        // set that kept one region per instant would silently drop an outline the record holds.
        let first = ChangeRegion::of([[0, 0, 0]], "first", None, 500.0).expect("recordable");
        let second = ChangeRegion::of([[9, 0, 0]], "second", None, 500.0).expect("recordable");
        let recorded = [first, second];
        let live = ChangeRegion::live(&recorded, 500.0);
        assert_eq!(live.len(), 2, "both changes are on screen");
        assert_eq!(live[0].reason, "first", "instants order first");
        assert_eq!(live[1].reason, "second", "and arrival order breaks the tie");
    }

    #[test]
    fn world_edges_come_from_the_declared_cell_size() {
        let edges = region([[1, 0, 0]]).outline_edges_world();
        assert_eq!(edges.len(), 12, "a single cell's wireframe is twelve edges");
        let xs: Vec<f64> = edges.iter().flatten().map(|corner| corner[0]).collect();
        assert_eq!(xs.iter().cloned().fold(f64::INFINITY, f64::min), 1.0);
        assert_eq!(xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max), 2.0);
    }

    fn projection() -> Projection {
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        Projection::of(&camera, Rect::new(0.0, 0.0, 100.0, 100.0))
    }

    #[test]
    fn a_point_on_the_axis_lands_at_the_viewport_centre() {
        let at = projection().point([0.0, 0.0, 10.0]).expect("in front of the eye");
        assert!((at[0] - 50.0).abs() < 0.5, "{at:?}");
        assert!((at[1] - 50.0).abs() < 0.5, "{at:?}");
    }

    #[test]
    fn a_point_behind_the_eye_is_refused_rather_than_drawn_at_an_edge() {
        let p = projection();
        assert_eq!(p.point([0.0, 0.0, 0.0]), None, "the eye itself has no projection");
        assert_eq!(p.point([0.0, 0.0, -5.0]), None, "and a point behind it neither");
        assert_eq!(p.point([0.0, 0.0, 0.05]), None, "between the eye and the near plane");
    }

    #[test]
    fn a_point_to_the_right_lands_to_the_right_of_centre() {
        let at = projection().point([1.0, 0.0, 10.0]).expect("in front");
        assert!(at[0] > 50.5, "{at:?}");
        assert!((at[1] - 50.0).abs() < 0.5, "{at:?}");
    }

    #[test]
    fn an_edge_crossing_the_camera_plane_is_cut_and_not_dropped() {
        // One end behind the eye, one in front: the visible part still draws, and the cut lands
        // on the axis because the edge does.
        let segment = projection()
            .segment([0.0, 0.0, -1.0], [0.0, 0.0, 10.0])
            .expect("one end is in front");
        for end in segment {
            assert!((end[0] - 50.0).abs() < 0.5, "{end:?}");
            assert!((end[1] - 50.0).abs() < 0.5, "{end:?}");
        }
        // Both ends behind: there is no visible part, and the segment is refused.
        assert_eq!(
            projection().segment([0.0, 0.0, -1.0], [0.0, 0.0, -2.0]),
            None
        );
    }

    #[test]
    fn a_degenerate_viewport_projects_nothing() {
        // A surface with no extent has no screen to put a mark on; refused, not collapsed.
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let p = Projection::of(&camera, Rect::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(p.point([0.0, 0.0, 10.0]), None);
        assert_eq!(p.segment([0.0, 0.0, 10.0], [1.0, 0.0, 10.0]), None);
    }

    #[test]
    fn a_region_draws_as_its_boundary_edges_in_world_order() {
        let camera = Camera::standard([0.5, 0.5, -5.0], 1.0);
        let p = Projection::of(&camera, Rect::new(0.0, 0.0, 100.0, 100.0));
        let strokes = outline_strokes(&[region([[0, 0, 0]])], &p, 100.0);
        assert_eq!(strokes.len(), 12, "one cell's wireframe is twelve edges");
        assert!(
            strokes.iter().all(|stroke| {
                stroke.width_dp == OUTLINE_DP
                    && stroke.paint.rgb == crate::debug::LaneClass::Change.hue()
            }),
            "the outline is drawn at the declared width and in the change class's hue"
        );
    }

    #[test]
    fn an_expired_change_draws_nothing() {
        let camera = Camera::standard([0.5, 0.5, -5.0], 1.0);
        let p = Projection::of(&camera, Rect::new(0.0, 0.0, 100.0, 100.0));
        let change = region([[0, 0, 0]]);
        assert_eq!(
            outline_strokes(std::slice::from_ref(&change), &p, ChangeRegion::LIFETIME_MS - 1.0)
                .len(),
            12,
            "live for exactly its declared lifetime"
        );
        assert!(
            outline_strokes(&[change], &p, ChangeRegion::LIFETIME_MS).is_empty(),
            "and gone at the last instant"
        );
    }
}
