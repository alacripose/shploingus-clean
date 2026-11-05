//! **The shaped runs: text kept shaped for as long as it still says the same thing.**
//!
//! # Why this is not part of the painter
//!
//! Shaping is the expensive half of drawing text — the shell's own measurement put **2.8 ms** of a
//! 3.0 ms `HudRenderer::prepare` in it, over 45 runs — and it is the half that depends on nothing
//! the painter knows. It needs a string, a type size and a wrap width. It does not need a rect, a
//! colour, an alignment or a band, which are the placement decisions [`crate::hud`] owns.
//!
//! So the owner lives here, with the two rules it exists for:
//!
//! * a run is shaped **once per distinct thing it says**, at a given size and wrap width, however
//!   many frames, bands or windows ask for it; and
//! * a run survives between frames only while something asked for it — the store holds one frame's
//!   distinct strings, not a session's, so a HUD whose counters move cannot grow it without bound.
//!
//! # What a run's identity is, and what it is not
//!
//! [`RunKey`] is the text, the type size in pixels and the wrap width in pixels — the whole set of
//! inputs to the shaping call in [`ShapedRuns::ensure`]. It is **not** the rect, the paint or the
//! alignment: a readout that moves to another window, changes ink or switches to right alignment is
//! the same shaped run, and moving it must not cost a re-shape. Those are placement, and placement
//! is recomputed every frame by [`crate::hud::HudRenderer::prepare`] regardless.
//!
//! Sizes and widths are stored as the **bits** of the `f32` they were shaped at rather than as the
//! number, because a key has to be `Eq`: two runs are the same run when they were shaped at the same
//! width, and `==` on floats answers a question about tolerance that shaping never asked.

use std::collections::HashMap;

/// The line box, as a multiple of the type size.
///
/// A shaping convention rather than a design decision, so it lives beside the shaping call and not
/// in `voxel-ui`'s constant table. It is `pub(crate)` because the painter needs the same number to
/// centre a line's glyphs in the rect the layout gave it.
pub(crate) const LINE_BOX: f32 = 1.25;

/// **What a shaped buffer is valid for**: the text, and the two pixel measurements it was shaped at.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RunKey {
    text: String,
    size_px: u32,
    wrap_px: u32,
}

impl RunKey {
    /// The key for a run of `text` at `size_px`, wrapped at `wrap_px`.
    ///
    /// The wrap width is a width, not a height: the height is one line box plus room for the
    /// descenders of the box below it, which is a constant of the shaping call and not something a
    /// caller can vary.
    pub(crate) fn of(text: &str, size_px: f32, wrap_px: f32) -> Self {
        RunKey {
            text: text.to_string(),
            size_px: size_px.to_bits(),
            wrap_px: wrap_px.to_bits(),
        }
    }

    /// The type size the run was shaped at, in pixels.
    pub(crate) fn size_px(&self) -> f32 {
        f32::from_bits(self.size_px)
    }
}

/// One shaped buffer, and whether this frame asked for it.
struct Shaped {
    buffer: glyphon::Buffer,
    used: bool,
}

/// **The shaped runs, and the count of what this frame had to shape.**
///
/// A frame is bracketed by [`Self::begin`] and [`Self::settle`]. Between them, every text run the
/// painter places calls [`Self::ensure`]; afterwards the [buffer](Self::get) is read back to place
/// the glyphs, and `settle` drops whatever nobody asked for.
#[derive(Default)]
pub(crate) struct ShapedRuns {
    shaped: HashMap<RunKey, Shaped>,
    misses: usize,
}

impl ShapedRuns {
    /// Open a frame: nothing has been shaped yet.
    pub(crate) fn begin(&mut self) {
        self.misses = 0;
    }

    /// **The run for `key`, shaped if it is not held.** This is the only place text is shaped, so the
    /// cost of a frame is the number of runs that changed and never the number of runs on screen.
    pub(crate) fn ensure(&mut self, font: &mut glyphon::FontSystem, key: &RunKey) {
        if let Some(held) = self.shaped.get_mut(key) {
            held.used = true;
            return;
        }
        let size_px = key.size_px();
        let mut buffer =
            glyphon::Buffer::new(font, glyphon::Metrics::new(size_px, size_px * LINE_BOX));
        buffer.set_size(
            Some(f32::from_bits(key.wrap_px)),
            // One line box, rounded up: a readout that wrapped would be a readout that acquired a
            // second line nobody laid out.
            Some(size_px * LINE_BOX * 2.0),
        );
        // **One family, named where glyphs are shaped.** The shell's rows are columns — a log
        // line's source against its text, a readout's label against its value — and a fixed
        // advance is what keeps the second column where the first one ended.
        buffer.set_text(
            &key.text,
            &glyphon::Attrs::new().family(glyphon::Family::Monospace),
            glyphon::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(font, false);
        self.shaped
            .insert(key.clone(), Shaped { buffer, used: true });
        self.misses += 1;
    }

    /// The shaped buffer for `key`. A key that was not [`Self::ensure`]d in this frame is a caller
    /// mistake rather than a state to draw around, so it is a panic and not a blank box.
    pub(crate) fn get(&self, key: &RunKey) -> &glyphon::Buffer {
        &self
            .shaped
            .get(key)
            .expect("the run was placed this frame, so `ensure` shaped it")
            .buffer
    }

    /// How many runs this frame had to shape. Zero is the settled frame: every run it placed was
    /// already held, so nothing was learned and nothing needs re-uploading.
    pub(crate) fn misses(&self) -> usize {
        self.misses
    }

    /// Close a frame: keep what was asked for and drop the rest, so the store is bounded by the runs
    /// on screen and not by the runs the session has seen.
    pub(crate) fn settle(&mut self) {
        self.shaped
            .retain(|_, held| std::mem::replace(&mut held.used, false));
    }

    /// How many runs are held. For tests and for the size a caller may want to report; nothing on
    /// the draw path reads it.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.shaped.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> glyphon::FontSystem {
        glyphon::FontSystem::new()
    }

    #[test]
    fn a_run_is_shaped_once_however_many_times_it_is_placed() {
        // The whole claim of the module: three placements of one sentence at one size cost one
        // shape, and the fourth — a different string — costs the second.
        let mut font = font();
        let mut runs = ShapedRuns::default();
        runs.begin();
        let key = RunKey::of("605 / 605 chunks", 13.0, 240.0);
        for _ in 0..3 {
            runs.ensure(&mut font, &key);
        }
        assert_eq!(runs.misses(), 1, "one shape for three placements");
        assert_eq!(runs.len(), 1);
        runs.ensure(&mut font, &RunKey::of("604 / 605 chunks", 13.0, 240.0));
        assert_eq!(runs.misses(), 2, "different text is a different run");
    }

    #[test]
    fn placement_is_not_identity_and_sizes_are() {
        // A run that moves, changes ink or changes alignment is the *same* run: nothing in the key
        // mentions where it goes or what colour it is. A run at another size is not: the shaping
        // measured advances in pixels, so a different size is a different answer.
        let mut font = font();
        let mut runs = ShapedRuns::default();
        runs.begin();
        runs.ensure(&mut font, &RunKey::of("13 fps", 13.0, 240.0));
        runs.ensure(&mut font, &RunKey::of("13 fps", 13.0, 240.0));
        assert_eq!(runs.misses(), 1, "the same run, placed again");
        runs.ensure(&mut font, &RunKey::of("13 fps", 11.0, 240.0));
        runs.ensure(&mut font, &RunKey::of("13 fps", 13.0, 180.0));
        assert_eq!(runs.misses(), 3, "a new size and a new wrap are new runs");
    }

    #[test]
    fn a_run_nobody_asked_for_is_dropped_when_the_frame_closes() {
        // A HUD's counters move every frame, so a store that kept every string it had ever seen
        // would be a leak with a nice name. One frame's runs is the bound, and the next frame's
        // reuse is what makes the store worth having at all.
        let mut font = font();
        let mut runs = ShapedRuns::default();
        let kept = RunKey::of("World", 13.0, 240.0);
        let gone = RunKey::of("t9432", 13.0, 240.0);
        runs.begin();
        runs.ensure(&mut font, &kept);
        runs.ensure(&mut font, &gone);
        runs.settle();
        assert_eq!(runs.len(), 2, "both were asked for this frame");

        runs.begin();
        runs.ensure(&mut font, &kept);
        assert_eq!(runs.misses(), 0, "the kept run is still held");
        assert_eq!(
            runs.len(),
            2,
            "the dropped one is still held until the frame closes"
        );
        runs.settle();
        assert_eq!(
            runs.len(),
            1,
            "and the frame that did not ask for it releases it"
        );

        runs.begin();
        runs.ensure(&mut font, &gone);
        assert_eq!(
            runs.misses(),
            1,
            "a run that left and came back is shaped again"
        );
    }

    #[test]
    fn the_held_buffer_is_the_one_that_was_shaped() {
        // The read side, exercised: what `get` hands the painter is the shaped layout of the text
        // that was asked for, not an empty buffer standing in for it.
        let mut font = font();
        let mut runs = ShapedRuns::default();
        runs.begin();
        let key = RunKey::of("resident digest", 13.0, 240.0);
        runs.ensure(&mut font, &key);
        let buffer = runs.get(&key);
        let widths: Vec<f32> = buffer.layout_runs().map(|run| run.line_w).collect();
        assert_eq!(widths.len(), 1, "one line, as laid out");
        assert!(widths[0] > 0.0, "and it has a width: {widths:?}");
    }
}
