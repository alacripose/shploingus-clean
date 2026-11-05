//! **The drop preview: where a dragged window will land, drawn before it lands.**
//!
//! Reference: `docs/portal_simulator.py` — `DropPreviewOverlay` 2719–2777 (the label 2731–2734,
//! the pane geometry 2744–2760, the window-edge geometry 2761–2774) and the zone rule inside
//! `continue_window_drag` 3190–3246 (the outer 24 px edge first; otherwise the pane under the
//! cursor and its nearest edge by normalised dominance; the reference's `"right"` fallback).
//! Step 1 of `docs/SHELL-PORT.md`'s map.
//!
//! **The preview's rect comes from the same layout the panes were drawn with** —
//! [`splitter::Layout`](crate::shell::splitter::Layout) — which is the port's one-arithmetic
//! rule applied to the drop: a preview that resolved against a different rect would show where
//! a pane is *not*.
//!
//! # What changes, and why it is named
//!
//! * **The fill is the ink ladder, not `accent` at 25 %.** The reference fills the overlay with
//!   `rgba(76, 142, 216, 0.25)` and borders it in `accent`; **`Role::Accent` refuses by name**
//!   until `theme.rs` (step 2), so the wash is the ladder's divider step over the window and the
//!   border is a step up from it. The shape and the words are the reference's.
//! * **The label is the reference's own words** (`"◧  Split Left"` / `"◨  Dock Right"`,
//!   2742–2774) with the declared zone marks, drawn at full ink; the reference's filled pill
//!   waits for the accent role with the rest of the theme.
//! * **The 20 dp floor on half-rects is kept**: the reference's `max(20, …)` is why a preview is
//!   never thinner than a target.
//!
//! # What this module is not
//!
//! It answers *where* a drop would land and draws the preview. Tracking the drag, hiding the
//! preview, and performing the dock are `window.rs`'s; the reference's own split of those
//! responsibilities is the same (`show_drop_preview`/`hide_drop_preview`, 3264–3269).

use crate::shell::arrangement::PaneRef;
use crate::shell::placeholder::dashed_edges;
use crate::shell::splitter::Layout;
use voxel_ui::{
    contrast_over_backdrop, drop_zone_mark, spacing_dp, text_size_dp, Align, Element, Ink, Paint,
    Palette, Rect, Role, UiResult, WHITE,
};

/// **The reference's outer-edge band, in dp** (3195–3204: `<= 24`), which is also its
/// edge-trigger buttons' size (2857–2885: `setFixedSize(24, 24)`).
pub const EDGE_TRIGGER_DP: f64 = 24.0;

/// The reference's floor on a half-rect (`max(20, …)`, 2746–2774).
const HALF_FLOOR_DP: f64 = 20.0;

/// One of the four zones the reference docks into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
}

impl DropZone {
    pub const ALL: [DropZone; 4] = [
        DropZone::Left,
        DropZone::Right,
        DropZone::Top,
        DropZone::Bottom,
    ];

    /// The name the marks table is keyed by.
    pub const fn name(self) -> &'static str {
        match self {
            DropZone::Left => "left",
            DropZone::Right => "right",
            DropZone::Top => "top",
            DropZone::Bottom => "bottom",
        }
    }

    /// The capitalised name the label carries (`"Split Left"`).
    pub const fn label(self) -> &'static str {
        match self {
            DropZone::Left => "Left",
            DropZone::Right => "Right",
            DropZone::Top => "Top",
            DropZone::Bottom => "Bottom",
        }
    }

    /// The declared Unicode mark for this zone.
    pub fn mark(self) -> &'static str {
        drop_zone_mark(self.name()).expect("the four zones are declared")
    }
}

/// Where a drop would land: a zone, and the pane it splits (or none, for a window edge).
///
/// **The pane is a [`PaneRef`]**, so a drop can target an empty slot as well as a window pane:
/// the reference's `target_pane` is whatever `WindowPane` the cursor is over, app or not.
#[derive(Clone, Debug, PartialEq)]
pub struct DropTarget {
    pub zone: DropZone,
    pub pane: Option<PaneRef>,
}

/// **The zone rule, ported from `continue_window_drag`** (3190–3246), against the layout that
/// was drawn.
///
/// `at` is in the target window's own coordinates. The reference's order is kept exactly:
/// the **outer band** first (left, right, then top measured from *below* the title bar, then
/// bottom), then the pane under the cursor with the nearest edge by normalised dominance, and
/// the reference's `"right"` fallback when no pane is under the point.
pub fn zone_at(window: Rect, titlebar: f64, layout: &Layout, at: (f64, f64)) -> Option<DropTarget> {
    if !window.contains(at) {
        return None;
    }
    // **The pointer is the desk's; the reference's band arithmetic is the window's.** Qt hands
    // `continue_window_drag` a widget-local `pos` (3172–3230), so the four band comparisons are
    // local numbers — a shell comparing the global pointer against them would find the left
    // band only at the desk's left edge. The translation Qt gets for free is done here once.
    let x = at.0 - window.x;
    let y = at.1 - window.y;
    let band = EDGE_TRIGGER_DP;
    let zone = if x <= band {
        DropZone::Left
    } else if x >= window.width_dp - band {
        DropZone::Right
    } else if y - titlebar <= band {
        DropZone::Top
    } else if window.height_dp - y <= band {
        DropZone::Bottom
    } else {
        // The pane under the cursor, by the same rects that were drawn.
        match layout
            .panes
            .iter()
            .find(|(rect, _)| rect.contains(at))
            .map(|(rect, pane)| {
                let dx = (at.0 - rect.x) / rect.width_dp.max(1.0);
                let dy = (at.1 - rect.y) / rect.height_dp.max(1.0);
                let left = dx;
                let right = 1.0 - dx;
                let top = dy;
                let bottom = 1.0 - dy;
                let minimum = left.min(right).min(top).min(bottom);
                let zone = if minimum == left {
                    DropZone::Left
                } else if minimum == right {
                    DropZone::Right
                } else if minimum == top {
                    DropZone::Top
                } else {
                    DropZone::Bottom
                };
                DropTarget {
                    zone,
                    pane: Some(pane.clone()),
                }
            }) {
            Some(target) => return Some(target),
            // The reference's fallback: no pane found means the window's right edge.
            None => DropZone::Right,
        }
    };
    Some(DropTarget { zone, pane: None })
}

/// The preview's rect — the reference's geometry (2744–2774), in the window's coordinates.
///
/// A pane target splits **the pane's own rect**; a window edge docks into the window's
/// **content area** (below the title bar), which is the reference's `tb_h` arithmetic.
pub fn preview_rect(
    window: Rect,
    titlebar: f64,
    target: &DropTarget,
    pane_rect: Option<Rect>,
) -> Rect {
    match pane_rect {
        Some(pane) => {
            let half_w = pane.width_dp / 2.0;
            let half_h = pane.height_dp / 2.0;
            match target.zone {
                DropZone::Left => {
                    Rect::new(pane.x, pane.y, half_w.max(HALF_FLOOR_DP), pane.height_dp)
                }
                DropZone::Right => Rect::new(
                    pane.x + half_w,
                    pane.y,
                    (pane.width_dp - half_w).max(HALF_FLOOR_DP),
                    pane.height_dp,
                ),
                DropZone::Top => {
                    Rect::new(pane.x, pane.y, pane.width_dp, half_h.max(HALF_FLOOR_DP))
                }
                DropZone::Bottom => Rect::new(
                    pane.x,
                    pane.y + half_h,
                    pane.width_dp,
                    (pane.height_dp - half_h).max(HALF_FLOOR_DP),
                ),
            }
        }
        None => {
            let content = (window.height_dp - titlebar).max(1.0);
            let half_w = window.width_dp / 2.0;
            let half_h = content / 2.0;
            match target.zone {
                DropZone::Left => Rect::new(
                    window.x,
                    window.y + titlebar,
                    half_w.max(HALF_FLOOR_DP),
                    content,
                ),
                DropZone::Right => Rect::new(
                    window.x + half_w,
                    window.y + titlebar,
                    (window.width_dp - half_w).max(HALF_FLOOR_DP),
                    content,
                ),
                DropZone::Top => Rect::new(
                    window.x,
                    window.y + titlebar,
                    window.width_dp,
                    half_h.max(HALF_FLOOR_DP),
                ),
                DropZone::Bottom => Rect::new(
                    window.x,
                    window.y + titlebar + half_h,
                    window.width_dp,
                    (content - half_h).max(HALF_FLOOR_DP),
                ),
            }
        }
    }
}

/// The preview's own words: **Split** into a pane, **Dock** onto a window edge (2742–2774), with
/// the declared zone mark in front — `"◧  Split Left"`.
pub fn label(target: &DropTarget) -> String {
    let verb = if target.pane.is_some() {
        "Split"
    } else {
        "Dock"
    };
    format!("{}  {} {}", target.zone.mark(), verb, target.zone.label())
}

/// The preview's elements: the wash, its dashed border, and the label on a pill.
///
/// The wash and the border are the tree's ladder steps (the reference's accent is unadmitted
/// until `theme.rs`); the words are the reference's own. **The label sits on a declared surface
/// rather than directly on the wash** because its backdrop must be one a contrast can be
/// measured against: the reference's pill is `accent`, and this port's is `Chrome` until the
/// role is admitted — the pill is drawn, only its colour waits.
pub fn elements(
    window: Rect,
    titlebar: f64,
    target: &DropTarget,
    pane_rect: Option<Rect>,
    opacity: f64,
    palette: &Palette,
) -> UiResult<Vec<Element>> {
    let opacity = opacity.clamp(0.0, 1.0);
    let rect = preview_rect(window, titlebar, target, pane_rect);
    let mut out = Vec::new();
    out.push(Element::Surface {
        rect,
        paint: Paint {
            rgb: WHITE,
            alpha: Ink::Divider.opacity() * opacity,
        },
    });
    for dash in dashed_edges(rect) {
        out.push(Element::Rule {
            rect: dash,
            paint: Ink::Secondary.paint(),
        });
    }
    let pill = Rect::new(
        rect.x + spacing_dp(),
        rect.y + spacing_dp(),
        (rect.width_dp - 2.0 * spacing_dp()).max(0.0),
        text_size_dp() + spacing_dp(),
    );
    let chrome = palette.get(Role::Chrome)?;
    let backdrop = chrome.blend_over(WHITE, opacity);
    contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;
    out.push(Element::Surface {
        rect: pill,
        paint: Paint {
            rgb: chrome,
            alpha: opacity,
        },
    });
    out.push(Element::Text {
        rect: Rect::new(
            pill.x + spacing_dp() / 2.0,
            pill.y + (pill.height_dp - text_size_dp()) / 2.0,
            (pill.width_dp - spacing_dp()).max(0.0),
            text_size_dp(),
        ),
        text: label(target),
        paint: Ink::Full.paint(),
        size_dp: text_size_dp(),
        align: Align::Start,
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::arrangement::{Arrangement, Orientation, PaneRef, Side, WindowId};
    use crate::shell::splitter;

    fn window() -> Rect {
        Rect::new(0.0, 0.0, 600.0, 400.0)
    }

    fn two_panes() -> Layout {
        let mut arrangement = Arrangement::of(WindowId::new("a").expect("id"));
        arrangement
            .split(
                None,
                Orientation::Horizontal,
                Side::After,
                WindowId::new("b").expect("id"),
            )
            .expect("splits");
        splitter::layout(&arrangement, window()).expect("lays out")
    }

    #[test]
    fn the_outer_band_wins_over_the_pane_under_it() {
        let layout = two_panes();
        let target = zone_at(window(), 40.0, &layout, (10.0, 200.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Left);
        assert!(
            target.pane.is_none(),
            "the outer band docks to the window, not the pane"
        );
        let target = zone_at(window(), 40.0, &layout, (590.0, 200.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Right);
    }

    #[test]
    fn the_bands_are_the_window_s_own_wherever_the_window_sits() {
        // The reference's `pos` is widget-local (3172); a window at (400, 300) must find its own
        // left band at x = 410 — nowhere near the desk's left edge, which is where a global
        // comparison of the same number put it (and the old arithmetic even read the desk's
        // bottom band for it).
        let offset = Rect::new(400.0, 300.0, 600.0, 400.0);
        let mut arrangement = Arrangement::of(WindowId::new("a").expect("id"));
        arrangement
            .split(
                None,
                Orientation::Horizontal,
                Side::After,
                WindowId::new("b").expect("id"),
            )
            .expect("splits");
        let layout = splitter::layout(&arrangement, offset).expect("lays out");
        let target = zone_at(offset, 40.0, &layout, (410.0, 500.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Left);
        assert!(target.pane.is_none(), "the outer band docks to the window");
        // The top band is below the window's own bar: local y = 50.
        let target = zone_at(offset, 40.0, &layout, (700.0, 350.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Top);
    }

    #[test]
    fn the_top_band_is_measured_from_below_the_title_bar_as_the_reference_measures_it() {
        let layout = two_panes();
        // The reference's `p_local.y() - tb_h <= 24` is true for every point at or above the
        // band below the title bar, including points inside the title bar itself.
        let target = zone_at(window(), 40.0, &layout, (300.0, 50.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Top);
        let target = zone_at(window(), 40.0, &layout, (300.0, 390.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Bottom);
    }

    #[test]
    fn inside_a_pane_the_nearest_edge_dominates_and_the_pane_is_the_target() {
        let layout = two_panes();
        // Pane `a` is 0..300 wide; a quarter in, mid-height: the left edge is nearest.
        let target = zone_at(window(), 40.0, &layout, (75.0, 250.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Left);
        assert_eq!(
            target.pane,
            Some(PaneRef::Window(WindowId::new("a").expect("id")))
        );
        // Near the pane's right edge: right.
        let target = zone_at(window(), 40.0, &layout, (280.0, 250.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Right);
    }

    #[test]
    fn a_tie_prefers_left_then_right_then_top_as_the_reference_orders_it() {
        let layout = two_panes();
        // The pane's true centre is equidistant (0.5 exactly); the reference's `min_d == d_l`
        // chain answers left.
        let target = zone_at(window(), 40.0, &layout, (150.0, 200.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Left);
    }

    #[test]
    fn no_pane_under_the_point_falls_back_to_the_reference_s_right() {
        // A layout whose panes do not cover the point (a hole is impossible in the tree, so the
        // test uses an empty layout — the reference's `if not drop_zone: drop_zone = "right"`).
        let empty = Layout {
            panes: Vec::new(),
            seams: Vec::new(),
        };
        let target = zone_at(window(), 40.0, &empty, (300.0, 200.0)).expect("inside");
        assert_eq!(target.zone, DropZone::Right);
        assert!(target.pane.is_none());
    }

    #[test]
    fn the_preview_splits_the_panes_own_rect_and_docks_into_the_window_s_content() {
        let layout = two_panes();
        let pane = layout
            .window_rect(&WindowId::new("a").expect("id"))
            .expect("pane a");
        let target = DropTarget {
            zone: DropZone::Left,
            pane: Some(PaneRef::Window(WindowId::new("a").expect("id"))),
        };
        let rect = preview_rect(window(), 40.0, &target, Some(pane));
        assert_eq!(rect.x, pane.x);
        assert_eq!(rect.width_dp, pane.width_dp / 2.0);
        assert_eq!(rect.height_dp, pane.height_dp);

        let target = DropTarget {
            zone: DropZone::Bottom,
            pane: None,
        };
        let rect = preview_rect(window(), 40.0, &target, None);
        assert_eq!(rect.y, window().y + 40.0 + (400.0 - 40.0) / 2.0);
        assert_eq!(rect.x, window().x);
    }

    #[test]
    fn a_tiny_pane_s_preview_still_meets_the_twenty_dp_floor() {
        let pane = Rect::new(10.0, 10.0, 30.0, 30.0);
        let target = DropTarget {
            zone: DropZone::Left,
            pane: None,
        };
        let rect = preview_rect(window(), 40.0, &target, Some(pane));
        assert_eq!(rect.width_dp, 20.0, "the reference's max(20, …)");
    }

    #[test]
    fn the_label_is_the_reference_s_own_words_with_the_declared_marks() {
        let split = DropTarget {
            zone: DropZone::Left,
            pane: Some(PaneRef::Window(WindowId::new("a").expect("id"))),
        };
        let dock = DropTarget {
            zone: DropZone::Bottom,
            pane: None,
        };
        assert_eq!(label(&split), "◧  Split Left");
        assert_eq!(label(&dock), "⬓  Dock Bottom");
        for zone in DropZone::ALL {
            assert_eq!(voxel_ui::drop_zone_mark(zone.name()).unwrap(), zone.mark());
        }
    }

    #[test]
    fn the_preview_is_a_dashed_wash_with_the_label_on_a_declared_pill() {
        let target = DropTarget {
            zone: DropZone::Right,
            pane: None,
        };
        let out =
            elements(window(), 40.0, &target, None, 1.0, &Palette::declared()).expect("draws");
        let surfaces = out
            .iter()
            .filter(|element| matches!(element, Element::Surface { .. }))
            .count();
        let rules = out
            .iter()
            .filter(|element| matches!(element, Element::Rule { .. }))
            .count();
        assert_eq!(surfaces, 2, "the wash and the label's pill");
        assert!(rules > 10, "the border is dashed, found {rules} rules");
        match out.last() {
            Some(Element::Text { text, paint, .. }) => {
                assert_eq!(text, "◨  Dock Right");
                assert_eq!(paint.alpha, Ink::Full.opacity());
            }
            other => panic!("the label is the last element, found {other:?}"),
        }
    }

    #[test]
    fn a_point_outside_the_window_has_no_target() {
        let layout = two_panes();
        assert!(zone_at(window(), 40.0, &layout, (900.0, 900.0)).is_none());
    }
}
