//! **The frame's own work, and the one place that decides how much of it happens.**
//!
//! A streaming frame had no owner of its budget. A chunk arrived, and the frame rebuilt every
//! resident mesh (5–6 ms idle, **70 ms under eight busy cores**) and re-sent every corner
//! (**143 ms** on the same frame) because both were all-or-nothing. Measuring was bounded and the
//! other two were not, so the frame's cost was whatever the world happened to demand.
//!
//! [`FrameWork`] is that owner, and it is deliberately a **fixed partition** rather than a
//! feedback loop: a slice of quads to build, a slice of bytes to upload, and a slice of cells to
//! measure, every frame, each independent of the others. Three properties follow from the shape
//! rather than from tuning:
//!
//! * **A burst cannot blow past the ceiling.** No site may do more than its slice, so the work a
//!   frame pays for is bounded by the sum of the slices however much is due.
//! * **Nothing is dropped.** The remainder is carried by the work itself — the build's cursor
//!   ([`crate::SceneBuild`]), the upload's cursor ([`CornerUpload`]), the field's queue — so it
//!   resumes next frame instead of being recomputed or forgotten.
//! * **No class starves another.** Geometry and measurement cannot consume the upload's slice or
//!   the reverse, because each is stated separately and none of them is derived from what the
//!   frame has left.
//!
//! The millisecond figures beside the constants are **measurements on this machine**, and
//! [`FrameWork::stated`]'s own test holds the three of them under the ceiling so the partition
//! cannot drift past it by editing a number.

use std::time::Duration;

/// How much geometry, upload and measurement work one frame may do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameWork {
    geometry_quads: usize,
    upload_bytes: u64,
    measure_cells: usize,
}

impl FrameWork {
    /// **The stated ceiling**, in milliseconds: the shipped frame's own work fits inside this.
    ///
    /// It is a ceiling on the whole frame, not on the sum of the slices: the slices are chosen so
    /// that the work they admit, at the measured rates below, leaves room for the shell, the sky,
    /// the camera and the draw on a machine running eight busy cores.
    pub const CEILING_MS: u64 = 50;

    /// Quads one frame may build: **1 536**, about **2.0 ms** at the measured 1.3 µs a quad.
    pub const GEOMETRY_QUADS: usize = 1_536;

    /// Bytes one frame may upload: **1.5 MB**, about **11 ms** at the measured 140 MB/s under
    /// eight busy cores (and 0.9 ms idle).
    pub const UPLOAD_BYTES: u64 = 1_500_000;

    /// Cells one frame may adopt: **2 048**, about **2 ms** of `CellField::at`-shaped lookups.
    pub const MEASURE_CELLS: usize = 2_048;

    /// Measured cost of one quad of geometry, in microseconds.
    pub const QUAD_US: f64 = 1.3;

    /// Measured cost of one megabyte of upload, in milliseconds, **under eight busy cores**.
    pub const UPLOAD_MB_MS: f64 = 7.4;

    /// Measured cost of one cell's adoption and queue work, in microseconds.
    pub const CELL_US: f64 = 1.0;

    /// The partition every frame uses.
    pub const fn stated() -> Self {
        Self {
            geometry_quads: Self::GEOMETRY_QUADS,
            upload_bytes: Self::UPLOAD_BYTES,
            measure_cells: Self::MEASURE_CELLS,
        }
    }

    /// The stated ceiling, as a duration the report can print.
    pub const fn ceiling(&self) -> Duration {
        Duration::from_millis(Self::CEILING_MS)
    }

    /// Quads the frame's geometry may build.
    pub fn geometry(&self) -> usize {
        self.geometry_quads
    }

    /// Bytes the frame may upload.
    pub fn upload(&self) -> u64 {
        self.upload_bytes
    }

    /// Cells the frame may adopt.
    pub fn measure(&self) -> usize {
        self.measure_cells
    }

    /// **What the three slices cost at the measured rates**, in milliseconds.
    pub fn budget_ms(&self) -> f64 {
        self.geometry_quads as f64 * Self::QUAD_US / 1e3
            + self.upload_bytes as f64 / 1e6 * Self::UPLOAD_MB_MS
            + self.measure_cells as f64 * Self::CELL_US / 1e3
    }
}

/// **One scene's corner upload, in slices**, so the frame decides how many bytes it sends.
///
/// The cursors are the remainder: a 20 MB scene sent 1.5 MB at a time takes fourteen frames, and
/// the frames in between draw the **previous** scene from the other buffer pair rather than a
/// half-written one. Nothing is serialised ahead of time — each slice is written out of the
/// scene's own vectors at the offset it belongs at — so the working set does not grow by a
/// staging copy of the scene.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CornerUpload {
    /// The scene build these bytes belong to.
    pub build: u64,
    vertices: usize,
    indices: usize,
    vertex_done: usize,
    index_done: usize,
    bytes_done: u64,
}

/// One slice of an upload: a byte range of the vertex buffer and/or the index buffer.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UploadSlice {
    /// Vertices `vertex_from..vertex_to`, at that vertex offset in the buffer.
    pub vertex_from: usize,
    pub vertex_to: usize,
    /// Indices `index_from..index_to`, at that index offset in the buffer.
    pub index_from: usize,
    pub index_to: usize,
    /// Bytes this slice asks the frame to write.
    pub bytes: u64,
}

impl CornerUpload {
    /// A fresh upload of a scene with `vertices` corners and `indices` indices.
    pub fn new(build: u64, vertices: usize, indices: usize) -> Self {
        Self { build, vertices, indices, vertex_done: 0, index_done: 0, bytes_done: 0 }
    }

    pub fn vertices(&self) -> usize {
        self.vertices
    }

    pub fn indices(&self) -> usize {
        self.indices
    }

    /// **The buffers this upload needs, sized once**: `(vertex bytes, index bytes)` for the whole
    /// scene, not for a slice.
    ///
    /// The bug this names cost more than it reads: the buffers used to be grown to each slice's
    /// end, so an upload was reallocated six or seven times, **losing every byte already written
    /// into them** and paying a fresh 16 MB GPU allocation per growth — the loaded draw's 45–70 ms.
    /// A pair is sized from this before the first slice lands.
    pub fn capacity(&self, stride: u64) -> (u64, u64) {
        (self.vertices as u64 * stride, self.indices as u64 * 4)
    }

    /// Bytes still to send.
    pub fn pending_bytes(&self, stride: u64) -> u64 {
        (self.vertices - self.vertex_done) as u64 * stride
            + (self.indices - self.index_done) as u64 * 4
    }

    pub fn done(&self) -> bool {
        self.vertex_done == self.vertices && self.index_done == self.indices
    }

    /// **The next slice, bounded by `budget` bytes.** Vertices first, then indices with what is
    /// left, so a frame's slice is one contiguous write per buffer at most.
    pub fn next(&mut self, stride: u64, budget: u64) -> UploadSlice {
        let mut left = budget;
        // **At least one of something per call**, or a budget smaller than one corner would spin
        // forever: a slice always makes progress, which is what keeps a class from starving.
        let vertex_from = self.vertex_done;
        let take_vertices = if vertex_from < self.vertices {
            ((left / stride) as usize).min(self.vertices - vertex_from).max(1)
        } else {
            0
        };
        let vertex_to = vertex_from + take_vertices;
        left = left.saturating_sub(take_vertices as u64 * stride);
        let index_from = self.index_done;
        let take_indices = if index_from < self.indices {
            // One item per call, not one per buffer: a budget too small for a corner must still
            // move the upload rather than spin on a zero, and once a corner has moved the
            // indices wait for the bytes they need.
            let fit = (left / 4) as usize;
            let fit = if take_vertices == 0 { fit.max(1) } else { fit };
            fit.min(self.indices - index_from)
        } else {
            0
        };
        let index_to = index_from + take_indices;
        self.vertex_done = vertex_to;
        self.index_done = index_to;
        let bytes = take_vertices as u64 * stride + take_indices as u64 * 4;
        self.bytes_done += bytes;
        UploadSlice { vertex_from, vertex_to, index_from, index_to, bytes }
    }

    /// Bytes this upload has asked the frames to write so far.
    pub fn bytes_done(&self) -> u64 {
        self.bytes_done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The partition fits the ceiling.** Three numbers written beside the rates they cost at, held
    /// against the ceiling: editing a slice past what the frame can pay fails here rather than in
    /// a measurement nobody repeats.
    #[test]
    fn the_slices_cost_less_than_the_ceiling() {
        let work = FrameWork::stated();
        let cost = work.budget_ms();
        assert!(
            cost < FrameWork::CEILING_MS as f64,
            "the frame's own work costs {cost:.1} ms of a {} ms ceiling",
            FrameWork::CEILING_MS
        );
    }

    /// **A burst is sliced, not paid.** A scene of 57 000 quads and 20 MB of corners, with a
    /// thousand cells due, is admitted a slice at a time: no frame asks for more than its own
    /// slice, and the two are finished in the number of frames the slices imply.
    #[test]
    fn a_burst_is_carried_across_frames_and_never_exceeds_a_slice() {
        let work = FrameWork::stated();
        let quads = 57_000usize;
        let mut built = 0usize;
        let mut frames = 0;
        while built < quads {
            let slice = work.geometry().min(quads - built);
            assert!(slice <= work.geometry(), "a frame built {slice} quads");
            built += slice;
            frames += 1;
        }
        assert_eq!(built, quads, "the remainder was dropped");
        assert_eq!(frames, quads.div_ceil(work.geometry()));

        // The upload: one scene, sent in slices of the frame's own budget.
        let stride = 68u64;
        let mut upload = CornerUpload::new(7, quads * 4, quads * 6);
        let mut sent = 0u64;
        let mut frames = 0;
        while !upload.done() {
            let slice = upload.next(stride, work.upload());
            assert!(slice.bytes <= work.upload(), "a frame sent {} B", slice.bytes);
            sent += slice.bytes;
            frames += 1;
            assert!(frames < 10_000, "the upload never finished");
        }
        assert_eq!(sent, (quads * 4) as u64 * stride + (quads * 6) as u64 * 4);
        assert!(frames > 1, "a 20 MB scene fitted in one frame's slice");
    }

    /// **The upload's buffers are the scene's, not the slice's.** A capacity taken from the first
    /// slice is a capacity that grows mid-upload — reallocating, and losing the prefix — which is
    /// the defect this pass repaired.
    #[test]
    fn the_capacity_is_the_whole_scene_and_not_a_slice() {
        let stride = 68u64;
        let mut upload = CornerUpload::new(3, 1_000, 1_500);
        let (vertices, indices) = upload.capacity(stride);
        assert_eq!(vertices, 1_000 * stride);
        assert_eq!(indices, 1_500 * 4);
        // A slice moves the cursor and leaves the capacity where it was.
        let slice = upload.next(stride, 4_096);
        assert!(slice.vertex_to < 1_000);
        assert_eq!(upload.capacity(stride), (vertices, indices));
    }

    /// **A slice always makes progress**, even when the budget is smaller than one corner — the
    /// rule that keeps a starved class from spinning on a zero.
    #[test]
    fn a_slice_smaller_than_a_corner_still_moves_one() {
        let mut upload = CornerUpload::new(1, 4, 6);
        let slice = upload.next(68, 1);
        assert_eq!(slice.vertex_to - slice.vertex_from, 1);
        assert_eq!(slice.index_from, 0);
        assert_eq!(slice.index_to, 0);
        // A budget that still cannot hold two corners moves one more, and no indices.
        let slice = upload.next(68, 16);
        assert_eq!(slice.vertex_to - slice.vertex_from, 1);
        assert_eq!(slice.index_to, 0);
        // Enough for the two corners that remain and four of the six indices.
        let slice = upload.next(68, 2 * 68 + 4 * 4);
        assert_eq!(slice.vertex_to, 4);
        assert_eq!(slice.index_to, 4);
        assert!(!upload.done());
        while !upload.done() {
            upload.next(68, 1_000);
        }
    }
}
