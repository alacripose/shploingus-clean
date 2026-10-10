//! **The title bar, ported from the reference's own composite elements.**
//!
//! Reference: `docs/portal_simulator.py` — `WindowElement` 2428–2432,
//! `ControlButtonGroupElement` 2434–2454, `TitleBarElement` 2455–2515 (`set_title` 2476,
//! press 2479–2489, move 2490–2497, release 2498–2505, double-click-maximize 2506–2515).
//! This is step 1 of `docs/SHELL-PORT.md`'s module map.
//!
//! # What changes, and why it is named here
//!
//! * **The bar is one row of MD1's dense-pointer targets.** The reference's `title_height`
//!   is 34 with 34-wide controls; D-1 (ruled 2026-10-03, `docs/SHELL-PORT.md` §6) translates
//!   the 48 dp *touch* floor to MD1's dense-pointer pair, so [`bar_height_dp`] is
//!   `POINTER_TARGET_DP` 40 and every control is a 40 dp lane with the 20 dp glyph. The
//!   reference is denser still; the floor wins over the literal, as D-1 says.
//! * **The controls carry the HIG separation.** The reference's group draws its buttons
//!   adjacent (spacing 0); `MIN_GAP_DP` — declared in `voxel-ui` and drawn by nothing before
//!   this module — requires 8 dp between targets, so the lanes are laid out with it.
//! * **The title's inset is the grid's 8 dp** (`SPACING_DP`), not the reference's 10.
//! * **A press is one question with one answer.** The reference asks it three times
//!   (`mousePressEvent`'s `childAt`, the drag path, the double-click path); here it is
//!   [`press`], and the hit map is the *same* lane list the drawing uses — the port's whole
//!   point (`SHELL-PORT.md` §5: one rect arithmetic for drawing and hit-testing).
//!
//! # What this module is not
//!
//! It draws the bar and answers a press on it. It does not own window state, focus, the
//! arrangement, or the desktop — `window.rs` composes the window, `desktop.rs` owns focus,
//! and the caller hands it the title it should show (the reference's `set_title`, 2476).

use voxel_package::recipe::{BehaviorPolicy, LayoutPolicy, WindowSpec};
use voxel_ui::{
    contrast_over_backdrop, glyph_for, min_gap_dp, pointer_glyph_dp, pointer_target_dp,
    spacing_dp, title_size_dp, Align, Element, GlyphAction, Ink, Palette, Paint, Rect,
    UiResult, WHITE,
};

/// The height of the ported title bar: **one row of MD1's dense-pointer targets**, the same
/// derivation `TITLE_BAR_DP` carries for the touch floor (a title bar is a single-line row
/// holding its controls, so it is the target's own height).
pub fn bar_height_dp() -> f64 {
    pointer_target_dp()
}

/// Whether the declaration permits a title bar at all — the tree's `title-position` token,
/// the reference's `TitleBarPosition.NONE` case (2784–2800).
pub fn has_bar(spec: &WindowSpec) -> bool {
    spec.layout_policy.permits(LayoutPolicy::TITLE_POSITION)
}

/// The controls this bar carries, in the reference's own order — minimize, maximize, close,
/// left to right (2534–2570) — and **not one more**: a control the declaration does not
/// permit is absent rather than drawn disabled.
pub fn controls(spec: &WindowSpec) -> Vec<GlyphAction> {
    GlyphAction::ALL
        .into_iter()
        .filter(|action| spec.layout_policy.permits(flag(*action)))
        .collect()
}

fn flag(action: GlyphAction) -> LayoutPolicy {
    match action {
        GlyphAction::Minimize => LayoutPolicy::MINIMIZE,
        GlyphAction::Maximize => LayoutPolicy::MAXIMIZE,
        GlyphAction::Close => LayoutPolicy::CLOSE,
    }
}

/// The bar's rect inside the window.
pub fn rect(window: Rect) -> Rect {
    Rect::new(window.x, window.y, window.width_dp, bar_height_dp())
}

/// The control lanes, **right-aligned** (the reference puts the group at the layout's end),
/// each a [`pointer_target_dp`] lane with [`min_gap_dp`] between neighbours.
///
/// A window too narrow for its own controls draws the ones that fit and drops the rest from
/// the **start** of the group — the close control is the last thing to disappear, because it
/// is the one that is always permitted when a bar exists.
pub fn control_lanes(window: Rect, spec: &WindowSpec) -> Vec<(GlyphAction, Rect)> {
    let bar = rect(window);
    let list = controls(spec);
    let lane = pointer_target_dp();
    let gap = min_gap_dp();
    let mut out = Vec::with_capacity(list.len());
    for (index, action) in list.iter().rev().enumerate() {
        let x = bar.right() - (index as f64 + 1.0) * lane - index as f64 * gap;
        if x < bar.x {
            break;
        }
        out.push((*action, Rect::new(x, bar.y, lane, bar.height_dp)));
    }
    out.reverse();
    out
}

/// The title's rect: the bar's inline space **up to where the controls begin**, so a long
/// title is clipped by its own box rather than drawn under a control (the reference's label
/// takes the layout's stretch, 2470–2474).
pub fn title_rect(window: Rect, spec: &WindowSpec) -> Rect {
    let bar = rect(window);
    let lanes = control_lanes(window, spec);
    let right = lanes
        .first()
        .map(|(_, lane)| lane.x - spacing_dp())
        .unwrap_or(bar.right() - spacing_dp());
    Rect::new(
        bar.x + spacing_dp(),
        bar.y + (bar.height_dp - title_size_dp()) / 2.0,
        (right - bar.x - spacing_dp()).max(0.0),
        title_size_dp(),
    )
}

/// What a press on the title bar means. The reference answers this across three handlers;
/// here it is one verdict the pointer layer consumes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TitlePress {
    /// A control was pressed. **Never a drag** — the reference refuses to start one when the
    /// child under the cursor is a button (`childAt`, 2481–2484).
    Control(GlyphAction),
    /// The bar itself: start a window drag (2479–2489), where the behaviour policy allows it.
    Drag,
    /// A double-click where `allow_maximize` permits it (2506–2515).
    Maximize,
    /// No bar, no permit, or a point outside the bar.
    None,
}

/// The verdict for a press (or a double-click) at `at`, in window coordinates.
///
/// The control test comes first for the double-click too: in the reference a double-click on
/// a button is delivered to the button, so the bar's own maximize path only ever sees
/// off-control points — folding both into one order keeps that behaviour deliberate rather
/// than accidental.
pub fn press(spec: &WindowSpec, window: Rect, at: (f64, f64), double: bool) -> TitlePress {
    if !has_bar(spec) {
        return TitlePress::None;
    }
    if !rect(window).contains(at) {
        return TitlePress::None;
    }
    for (action, lane) in control_lanes(window, spec) {
        if lane.contains(at) {
            return TitlePress::Control(action);
        }
    }
    if double {
        return if spec.behavior_policy.permits(BehaviorPolicy::MAXIMIZE) {
            TitlePress::Maximize
        } else {
            TitlePress::None
        };
    }
    if spec.behavior_policy.permits(BehaviorPolicy::DRAG) {
        TitlePress::Drag
    } else {
        TitlePress::None
    }
}

/// The bar's elements: its surface (the declared elevation tinted onto `Chrome`, as
/// `window_elements` draws a window's chrome), the rule under it, the title — dimmed one ink
/// step when the window is not focused, measured against the composited backdrop — and the
/// control glyphs, each centred in its own lane.
pub fn elements(
    spec: &WindowSpec,
    window: Rect,
    title: &str,
    focused: bool,
    palette: &Palette,
) -> UiResult<Vec<Element>> {
    if !has_bar(spec) {
        return Ok(Vec::new());
    }
    let bar = rect(window);
    let chrome = palette.get(voxel_ui::Role::Chrome)?.elevated(spec.elevation)?;
    let opacity = (spec.opacity as f64 / 1000.0).clamp(0.0, 1.0);

    let mut out = Vec::with_capacity(4 + controls(spec).len());
    out.push(Element::Surface {
        rect: bar,
        paint: Paint { rgb: chrome, alpha: opacity },
    });
    out.push(Element::Rule {
        rect: Rect::new(bar.x, bar.bottom() - 1.0, bar.width_dp, 1.0),
        paint: Ink::Divider.paint(),
    });

    let backdrop = chrome.blend_over(WHITE, opacity);
    let ink = if focused { Ink::Full } else { Ink::Secondary };
    contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
    out.push(Element::Text {
        rect: title_rect(window, spec),
        text: title.to_string(),
        paint: ink.paint(),
        size_dp: title_size_dp(),
        align: Align::Start,
    });

    let glyph_size = pointer_glyph_dp();
    for (action, lane) in control_lanes(window, spec) {
        let glyph = glyph_for(action);
        out.push(Element::Text {
            rect: Rect::new(
                lane.x + (lane.width_dp - glyph_size) / 2.0,
                lane.y + (lane.height_dp - glyph_size) / 2.0,
                glyph_size,
                glyph_size,
            ),
            text: glyph.text.to_string(),
            paint: ink.paint(),
            size_dp: glyph_size,
            align: Align::Start,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_package::recipe::{Content, ExtentUnit, Lane};

    /// A declaration with the two policies set and everything else fixed, so each test reads
    /// only the policy bits it is about.
    fn spec(layout: u16, behavior: u16) -> WindowSpec {
        WindowSpec {
            id: "w".to_string(),
            title: "a window".to_string(),
            content: Content::ALL[0],
            lane: Lane::ALL[0],
            extent_unit: ExtentUnit::ALL[0],
            geometry: [0, 0, 640, 406],
            min_size: [40, 40],
            opacity: 1000,
            elevation: 0,
            theme_role: "surface".to_string(),
            pinnable: false,
            layout_policy: LayoutPolicy(layout),
            behavior_policy: BehaviorPolicy(behavior),
            record_id: "C-1".to_string(),
        }
    }

    fn all_controls() -> u16 {
        LayoutPolicy::MINIMIZE.0
            | LayoutPolicy::MAXIMIZE.0
            | LayoutPolicy::CLOSE.0
            | LayoutPolicy::TITLE_POSITION.0
    }

    fn bar_window() -> Rect {
        Rect::new(100.0, 50.0, 640.0, 406.0)
    }

    #[test]
    fn a_policy_without_title_position_draws_no_bar_and_answers_nothing() {
        let s = spec(LayoutPolicy::CLOSE.0, BehaviorPolicy::DRAG.0);
        assert!(!has_bar(&s));
        let out = elements(&s, bar_window(), "t", true, &Palette::declared()).unwrap();
        assert!(out.is_empty(), "no bar means no elements");
        assert_eq!(press(&s, bar_window(), (120.0, 60.0), false), TitlePress::None);
    }

    #[test]
    fn the_controls_are_the_declared_ones_in_the_reference_order_and_not_one_more() {
        assert_eq!(
            controls(&spec(all_controls(), 0)),
            vec![GlyphAction::Minimize, GlyphAction::Maximize, GlyphAction::Close]
        );
        assert_eq!(controls(&spec(LayoutPolicy::CLOSE.0, 0)), vec![GlyphAction::Close]);
        assert!(controls(&spec(0, 0)).is_empty(), "the empty mask permits nothing");
    }

    #[test]
    fn the_lanes_are_targets_flush_with_the_end_and_separated_by_eight() {
        let s = spec(all_controls(), 0);
        let window = bar_window();
        let lanes = control_lanes(window, &s);
        assert_eq!(lanes.len(), 3);
        for (action, lane) in &lanes {
            assert_eq!(lane.width_dp, pointer_target_dp(), "{action:?} is not a target wide");
            assert_eq!(lane.height_dp, bar_height_dp());
        }
        // The close control ends at the window's end, and the group runs left in declared order.
        assert_eq!(lanes[2].0, GlyphAction::Close);
        assert_eq!(lanes[2].1.right(), window.right());
        assert_eq!(lanes[1].0, GlyphAction::Maximize);
        assert_eq!(lanes[0].0, GlyphAction::Minimize);
        // HIG's separation, drawn rather than declared.
        assert_eq!(lanes[1].1.x - lanes[0].1.right(), min_gap_dp());
        assert_eq!(lanes[2].1.x - lanes[1].1.right(), min_gap_dp());
    }

    #[test]
    fn a_window_narrower_than_its_controls_keeps_the_close_control() {
        let s = spec(all_controls(), 0);
        let tiny = Rect::new(0.0, 0.0, 50.0, 60.0);
        let lanes = control_lanes(tiny, &s);
        assert_eq!(lanes.len(), 1, "one 40 dp lane fits in 50");
        assert_eq!(lanes[0].0, GlyphAction::Close);
    }

    #[test]
    fn the_title_never_runs_under_the_controls() {
        let s = spec(all_controls(), 0);
        let window = bar_window();
        let title = title_rect(window, &s);
        let lanes = control_lanes(window, &s);
        let first_lane = lanes[0].1.x;
        assert!(
            title.right() <= first_lane,
            "the title ends before the control group begins"
        );
        assert!(title.x >= window.x, "and it starts inside the window");
    }

    #[test]
    fn a_press_on_a_control_is_never_a_drag() {
        let s = spec(all_controls(), BehaviorPolicy::DRAG.0);
        let window = bar_window();
        let lanes = control_lanes(window, &s);
        let (action, lane) = lanes[2];
        let centre = (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0);
        assert_eq!(press(&s, window, centre, false), TitlePress::Control(action));
        // Off the group, on the bar: a drag.
        let bar_point = (window.x + spacing_dp(), window.y + bar_height_dp() / 2.0);
        assert_eq!(press(&s, window, bar_point, false), TitlePress::Drag);
        // The same point with drag not permitted answers nothing.
        let still = spec(all_controls(), 0);
        assert_eq!(press(&still, window, bar_point, false), TitlePress::None);
    }

    #[test]
    fn a_double_press_maximizes_only_where_the_policy_permits() {
        let window = bar_window();
        let point = (window.x + 100.0, window.y + 10.0);
        let may = spec(all_controls(), BehaviorPolicy::MAXIMIZE.0);
        assert_eq!(press(&may, window, point, true), TitlePress::Maximize);
        let may_not = spec(all_controls(), BehaviorPolicy::DRAG.0);
        assert_eq!(press(&may_not, window, point, true), TitlePress::None);
    }

    #[test]
    fn the_unfocused_title_dims_one_step_and_the_contrast_is_measured() {
        let s = spec(all_controls(), BehaviorPolicy::DRAG.0);
        let window = bar_window();
        let focused = elements(&s, window, "t", true, &Palette::declared()).unwrap();
        let unfocused = elements(&s, window, "t", false, &Palette::declared()).unwrap();
        let title_ink = |out: &[Element]| match &out[2] {
            Element::Text { paint, .. } => *paint,
            other => panic!("the title is the third element, found {other:?}"),
        };
        assert_eq!(title_ink(&focused).alpha, Ink::Full.opacity());
        assert_eq!(title_ink(&unfocused).alpha, Ink::Secondary.opacity());
        // Both were measured against the composited backdrop; a bar that could not carry its
        // own title would have refused rather than drawn it.
        assert_eq!(focused.len(), 3 + controls(&s).len());
    }

    #[test]
    fn the_bar_is_one_pointer_row_and_its_glyphs_are_the_dense_pointer_pair() {
        assert_eq!(bar_height_dp(), pointer_target_dp());
        let s = spec(all_controls(), BehaviorPolicy::DRAG.0);
        let out = elements(&s, bar_window(), "t", true, &Palette::declared()).unwrap();
        let glyph = out.last().unwrap();
        match glyph {
            Element::Text { size_dp, .. } => assert_eq!(*size_dp, pointer_glyph_dp()),
            other => panic!("the last element is a glyph, found {other:?}"),
        }
    }
}
