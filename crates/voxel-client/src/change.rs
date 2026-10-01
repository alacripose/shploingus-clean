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

use std::collections::{BTreeMap, BTreeSet};

use voxel_model::MaterialId;

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

    /// The regions drawn this instant, keyed by the instant they were recorded.
    ///
    /// **A `BTreeMap` keyed by the record's own instant**, so the live set is a function of the
    /// record and the clock and not of the order the changes happened to arrive in.
    pub fn live<'a>(regions: &'a [ChangeRegion], now_ms: f64) -> Vec<&'a ChangeRegion> {
        let mut by_instant: BTreeMap<OrderedF64, &'a ChangeRegion> = BTreeMap::new();
        for region in regions {
            if region.is_live(now_ms) {
                by_instant.insert(OrderedF64(region.recorded_ms), region);
            }
        }
        by_instant.into_values().collect()
    }
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
}
