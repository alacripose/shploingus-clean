//! **The splitter: the pane tree, laid out and drawn.**
//!
//! Reference: `docs/portal_simulator.py` — `PortalSplitter` 2632–2652 (the 40×40
//! `minimumSizeHint` 2641–2642, `addWidget`/`insertWidget` stretch factors 2644–2652) and the
//! stylesheet's own treatment (`QSplitter::handle`: a 4 px line, `window_border`, hover
//! `accent`). Step 1 of `docs/SHELL-PORT.md`'s map.
//!
//! **The tree is `arrangement.rs`'s** — this module does not build a second one. `split_pane`'s
//! three cases, the collapse, and the floor's refusal are already the data model (C-110 §110.2:
//! *"faithful in the data, not in the drawing"*); what lands here is the layout pass and the
//! handles, **one arithmetic for drawing and hit-testing** (`SHELL-PORT.md` §5) — the class of
//! defect the derived shell's pane-rect mismatch was.
//!
//! # What changes, and why it is named
//!
//! * **The handles sit on the seams, not between the panes.** The reference's splitter consumes
//!   4 dp per handle (`setHandleWidth(4)`, Qt's own layout). This port draws each handle
//!   **centred on the seam** so the pane rects and `arrangement.rs::check_floor` keep one
//!   arithmetic: a handle that consumed space would make the floor check disagree with the
//!   layout by up to `(n−1) × 4` dp, and the disagreement would only show at the floor.
//! * **A hovered seam brightens one ink step** (the tree's ruled interaction language), not the
//!   reference's `accent` — an **unadmitted role** this tree refuses by name until `theme.rs`
//!   (step 2) admits it.
//! * **Proportional stretch is preserved.** The reference divides `[1000 // num] * num` and
//!   loses the remainder (`check_divisible`, `arrangement.rs`); the division here keeps it,
//!   and handles any shares the tree carries — including the ones a seam drag writes.
//! * **Each seam carries its address in the tree** — which splitter it divides, which boundary
//!   of that splitter, and the splitter's own span. The layout is the only pass that knows
//!   this; without it a seam drag would have to re-walk the tree with a second arithmetic and
//!   the two could disagree about which boundary was grabbed.
//!
//! # What this module is not
//!
//! It lays out and draws. Dragging a seam to change the shares, and everything else that
//! mutates the arrangement, belongs to `window.rs` — the seam drag there resolves the address
//! this module stamped and calls `arrangement.rs::drag_seam`; the layout below never mutates.

use crate::shell::arrangement::{Arrangement, Node, Orientation, PaneRef, WindowId, MIN_EXTENT};

use voxel_ui::{Element, Ink, Paint, Rect, UiRefusal, UiResult};

/// **The handle's thickness, in dp.** The reference's `setHandleWidth(4)` (2637) — a drawn
/// detail of the reference's own treatment, not an MD1 value (MD1 names no splitter handle).
pub const HANDLE_DP: f64 = 4.0;

/// One seam, with the address of the boundary it sits on.
///
/// **The address is what makes a drag possible without a second walk.** `splitter` indexes the
/// tree's splitters in the same order [`Arrangement::shares`](crate::shell::arrangement::Arrangement::shares)
/// reads them; `boundary` is the seam between that splitter's child `boundary` and child
/// `boundary + 1`; `span` is the splitter's extent along its own axis — the number a share
/// drag's arithmetic is stated in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seam {
    /// The handle's rect: [`HANDLE_DP`] thick, centred on the boundary.
    pub rect: Rect,
    /// The splitter's orientation. A horizontal split's seam is a vertical bar (and the drag
    /// that moves it changes widths).
    pub orientation: Orientation,
    /// Which splitter, in tree order — the same order `Arrangement::shares` walks.
    pub splitter: usize,
    /// Which boundary of that splitter: between child `boundary` and child `boundary + 1`.
    pub boundary: usize,
    /// The splitter's extent along its own axis, in dp.
    pub span: f64,
}

/// A tree laid out once: the pane rects, and the seams their handles sit on.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// `(rect, pane)` in tree order — **the rects the drawing and the hit tests both use**.
    ///
    /// **A pane is a [`PaneRef`], not a window id**, because an empty slot has no id: it is
    /// addressed by the path this walk stamps ([`PaneRef::Slot`]), which is why the layout is
    /// also the only place a slot's address is minted.
    pub panes: Vec<(Rect, PaneRef)>,
    /// Every internal seam, with its address. The order is the layout walk's, and `seam_at`
    /// indexes this list; the address inside the seam is what a drag resolves.
    pub seams: Vec<Seam>,
}

impl Layout {
    /// The pane a point lands in — the deepest pane's rect, the same rects that were drawn.
    pub fn pane_at(&self, at: (f64, f64)) -> Option<&PaneRef> {
        self.panes
            .iter()
            .find(|(rect, _)| rect.contains(at))
            .map(|(_, pane)| pane)
    }

    /// The window pane a point lands in, if the pane under it holds a window — the membership
    /// form of [`Layout::pane_at`] for the callers (the wheel, a right-click) that act on a
    /// window rather than on a pane.
    pub fn window_at(&self, at: (f64, f64)) -> Option<&WindowId> {
        self.pane_at(at).and_then(PaneRef::window)
    }

    /// The seam a point lands on, if any. **Asked before [`Layout::pane_at`] when a press could
    /// be a resize** — a seam overlays the panes, so it is the narrower test.
    pub fn seam_at(&self, at: (f64, f64)) -> Option<usize> {
        self.seams.iter().position(|seam| seam.rect.contains(at))
    }

    /// The rect a pane occupies, from the layout that was drawn.
    pub fn pane_rect(&self, pane: &PaneRef) -> Option<Rect> {
        self.panes
            .iter()
            .find(|(_, held)| held == pane)
            .map(|(rect, _)| *rect)
    }

    /// The rect a **window** pane occupies, by the window's id — the id form of
    /// [`Layout::pane_rect`] for callers that hold a window rather than a pane address.
    pub fn window_rect(&self, window: &WindowId) -> Option<Rect> {
        self.pane_rect(&PaneRef::Window(window.clone()))
    }

    /// The seams' elements: a 4 dp rule each, `Divider` ink, brightened one ink step when
    /// hovered. The window's opacity carries through so a translucent window's seams are not
    /// more solid than the window.
    pub fn seam_elements(&self, hovered: Option<usize>, opacity: f64, out: &mut Vec<Element>) {
        let opacity = opacity.clamp(0.0, 1.0);
        for (index, seam) in self.seams.iter().enumerate() {
            let ink = if hovered == Some(index) { Ink::Secondary } else { Ink::Divider };
            let paint = ink.paint();
            out.push(Element::Rule {
                rect: seam.rect,
                paint: Paint { rgb: paint.rgb, alpha: paint.alpha * opacity },
            });
        }
    }
}

/// Lay out an arrangement inside `rect` — **the pass the drawing and the hit tests share**.
///
/// The floor is `arrangement.rs`'s own arithmetic (`span / count` against
/// [`MIN_EXTENT`](crate::shell::arrangement::MIN_EXTENT)), refused with the same
/// `PaneBelowFloor` refusal, so a tree that cannot be drawn cannot be laid out either and the
/// two never disagree.
pub fn layout(arrangement: &Arrangement, rect: Rect) -> UiResult<Layout> {
    let mut out = Layout { panes: Vec::new(), seams: Vec::new() };
    // **The splitter counter is the walk's, and it matches `Arrangement::shares`**: a splitter
    // is numbered when the walk reaches it, before its children's splitters — the same preorder
    // `collect_shares` reads. A seam's address is only as good as that agreement.
    let mut next_splitter = 0usize;
    // **The path is the walk's too**, and it is the same address [`PaneRef::Slot`] carries and
    // `Arrangement::slot_at` resolves: a slot's identity is the route the walk took to reach it.
    let mut path = Vec::new();
    layout_node(arrangement.root(), rect, &mut path, &mut next_splitter, &mut out)?;
    Ok(out)
}

fn layout_node(
    node: &Node,
    rect: Rect,
    path: &mut Vec<usize>,
    next_splitter: &mut usize,
    out: &mut Layout,
) -> UiResult<()> {
    match node {
        Node::Pane(id) => {
            out.panes.push((rect, PaneRef::Window(id.clone())));
            Ok(())
        }
        Node::Slot => {
            out.panes.push((rect, PaneRef::Slot(path.clone())));
            Ok(())
        }
        Node::Split { orientation, children } => {
            let splitter = *next_splitter;
            *next_splitter += 1;
            let count = children.len().max(1) as f64;
            let span = match orientation {
                Orientation::Horizontal => rect.width_dp,
                Orientation::Vertical => rect.height_dp,
            };
            if span / count < MIN_EXTENT as f64 {
                return Err(UiRefusal::PaneBelowFloor {
                    span: span as u32,
                    count: count as u32,
                    floor: MIN_EXTENT,
                    axis: orientation.constrained_axis(),
                });
            }
            let total: f64 = children.iter().map(|(share, _)| *share as f64).sum();
            let total = if total <= 0.0 { 1.0 } else { total };
            let mut cursor = match orientation {
                Orientation::Horizontal => rect.x,
                Orientation::Vertical => rect.y,
            };
            for (index, (share, child)) in children.iter().enumerate() {
                let extent = span * (*share as f64) / total;
                let child_rect = match orientation {
                    Orientation::Horizontal => Rect::new(cursor, rect.y, extent, rect.height_dp),
                    Orientation::Vertical => Rect::new(rect.x, cursor, rect.width_dp, extent),
                };
                path.push(index);
                layout_node(child, child_rect, path, next_splitter, out)?;
                path.pop();
                cursor += extent;
                if index + 1 < children.len() {
                    out.seams.push(Seam {
                        rect: seam_at_cursor(*orientation, rect, cursor),
                        orientation: *orientation,
                        splitter,
                        boundary: index,
                        span,
                    });
                }
            }
            Ok(())
        }
    }
}

/// The handle's rect: [`HANDLE_DP`] thick, centred on the seam, spanning the cross axis.
fn seam_at_cursor(orientation: Orientation, rect: Rect, cursor: f64) -> Rect {
    match orientation {
        Orientation::Horizontal => Rect::new(
            cursor - HANDLE_DP / 2.0,
            rect.y,
            HANDLE_DP,
            rect.height_dp,
        ),
        Orientation::Vertical => Rect::new(
            rect.x,
            cursor - HANDLE_DP / 2.0,
            rect.width_dp,
            HANDLE_DP,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::arrangement::Side;

    fn id(name: &str) -> WindowId {
        WindowId::new(name).expect("a window id")
    }

    fn area() -> Rect {
        Rect::new(0.0, 0.0, 600.0, 400.0)
    }

    #[test]
    fn one_pane_is_the_whole_rect_and_has_no_seams() {
        let arrangement = Arrangement::of(id("a"));
        let layout = layout(&arrangement, area()).expect("lays out");
        assert_eq!(layout.panes.len(), 1);
        assert_eq!(layout.panes[0].0, area());
        assert!(layout.seams.is_empty());
    }

    #[test]
    fn two_equal_shares_meet_at_a_seam_centred_on_the_boundary() {
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("b"))
            .expect("splits");
        let layout = layout(&arrangement, area()).expect("lays out");
        assert_eq!(layout.panes.len(), 2);
        assert_eq!(layout.panes[0].0.width_dp, 300.0);
        assert_eq!(layout.panes[1].0.x, 300.0);
        assert_eq!(layout.seams.len(), 1);
        let seam = layout.seams[0];
        assert_eq!(seam.orientation, Orientation::Horizontal);
        assert_eq!(seam.rect.x, 300.0 - HANDLE_DP / 2.0);
        assert_eq!(seam.rect.width_dp, HANDLE_DP);
        assert_eq!(
            seam.rect.height_dp,
            area().height_dp,
            "a horizontal split's seam is a vertical bar"
        );
        // **The address travels with the seam**: splitter 0, boundary 0, and the splitter's own
        // span — what the seam drag's arithmetic reads instead of walking the tree again.
        assert_eq!(seam.splitter, 0);
        assert_eq!(seam.boundary, 0);
        assert_eq!(seam.span, area().width_dp);
    }

    #[test]
    fn a_seam_carries_the_splitter_and_boundary_it_divides() {
        // Three across: two seams on one splitter, at boundaries 0 and 1, the same span.
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("b"))
            .expect("splits");
        arrangement
            .split(Some(&id("b")), Orientation::Horizontal, Side::After, id("c"))
            .expect("splits");
        let wide = layout(&arrangement, area()).expect("lays out");
        assert_eq!(wide.seams.len(), 2);
        assert_eq!((wide.seams[0].splitter, wide.seams[0].boundary), (0, 0));
        assert_eq!((wide.seams[1].splitter, wide.seams[1].boundary), (0, 1));
        assert!(wide.seams.iter().all(|seam| seam.span == area().width_dp));
        // A nested split's seam names the nested splitter — the same order `shares` walks.
        arrangement
            .split(Some(&id("a")), Orientation::Vertical, Side::After, id("d"))
            .expect("wraps");
        let nested_layout = layout(&arrangement, area()).expect("lays out");
        let nested = nested_layout
            .seams
            .iter()
            .find(|seam| seam.orientation == Orientation::Vertical)
            .expect("the nested seam");
        assert_eq!(nested.splitter, 1, "the nested splitter is the second in tree order");
        // The nested splitter is vertical, so its span is the height it divides — the root's
        // 600 dp width is a different axis and a different number.
        assert_eq!(nested.span, area().height_dp, "the nested splitter's own span, not the root's");
    }

    #[test]
    fn the_floor_is_the_arrangements_own_arithmetic() {
        // Three panes across one row: `split` inserts into the root splitter it agrees with.
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("b"))
            .expect("splits");
        arrangement
            .split(Some(&id("b")), Orientation::Horizontal, Side::After, id("c"))
            .expect("splits");
        assert_eq!(arrangement.panes(), 3);
        let narrow = Rect::new(0.0, 0.0, 100.0, 400.0);
        assert!(matches!(
            layout(&arrangement, narrow),
            Err(UiRefusal::PaneBelowFloor { count: 3, floor: 40, .. })
        ));
        // The same tree in a span that divides passes, and every pane meets the floor.
        let wide = Rect::new(0.0, 0.0, 120.0, 400.0);
        let layout = layout(&arrangement, wide).expect("lays out");
        assert!(layout.panes.iter().all(|(rect, _)| rect.width_dp >= 40.0));
    }

    #[test]
    fn a_point_on_a_seam_picks_the_seam_and_never_two_panes() {
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Vertical, Side::After, id("b"))
            .expect("splits");
        let layout = layout(&arrangement, area()).expect("lays out");
        let seam = layout.seams[0].rect;
        let on_seam = (seam.x + 10.0, seam.y + HANDLE_DP / 2.0);
        assert!(layout.seam_at(on_seam).is_some());
        // The pane test still answers one pane — the seam is an overlay, not a third region.
        assert!(layout.pane_at(on_seam).is_some());
        let inside_b = (200.0, 300.0);
        assert_eq!(layout.pane_at(inside_b), Some(&PaneRef::Window(id("b"))));
        assert!(layout.seam_at(inside_b).is_none());
    }

    #[test]
    fn a_nested_split_lays_out_inside_its_parent_s_half() {
        // H[a, c], then a vertical wrap of `a`: H[V[a, b], c].
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("c"))
            .expect("splits");
        arrangement
            .split(Some(&id("a")), Orientation::Vertical, Side::After, id("b"))
            .expect("wraps");
        let layout = layout(&arrangement, area()).expect("lays out");
        assert_eq!(layout.panes.len(), 3);
        let rect_of = |name: &str| layout.window_rect(&id(name)).expect("a pane");
        assert_eq!(rect_of("a").x, 0.0);
        assert_eq!(rect_of("a").width_dp, 300.0);
        assert_eq!(rect_of("a").height_dp, 200.0);
        assert_eq!(rect_of("b").y, 200.0);
        assert_eq!(rect_of("c").x, 300.0);
        assert_eq!(rect_of("c").height_dp, 400.0);
        // Two seams: the outer vertical one and the nested horizontal one.
        assert_eq!(layout.seams.len(), 2);
    }

    #[test]
    fn a_pane_the_tree_does_not_hold_has_no_rect() {
        let layout = layout(&Arrangement::of(id("a")), area()).expect("lays out");
        assert!(layout.window_rect(&id("zz")).is_none());
    }

    #[test]
    fn a_hovered_seam_brightens_one_ink_step() {
        let mut arrangement = Arrangement::of(id("a"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("b"))
            .expect("splits");
        let layout = layout(&arrangement, area()).expect("lays out");
        let mut plain = Vec::new();
        layout.seam_elements(None, 1.0, &mut plain);
        let mut hovered = Vec::new();
        layout.seam_elements(Some(0), 1.0, &mut hovered);
        let alpha = |elements: &[Element]| match elements.first() {
            Some(Element::Rule { paint, .. }) => paint.alpha,
            other => panic!("the seam is a rule, found {other:?}"),
        };
        assert_eq!(alpha(&plain), Ink::Divider.opacity());
        assert_eq!(alpha(&hovered), Ink::Secondary.opacity());
    }
}
