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
//!   the 48 dp *touch* floor to MD1's dense-pointer pair, so the declared bar's height is
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
//! # The two forms, and the one that is data
//!
//! A window whose pin state is **on** draws the **minimal** bar: no title, smaller marks, a
//! shorter strip. That is what lets a pinned HUD panel give its body the row back, and it is
//! declared rather than hard-coded — [`MINIMAL_TITLEBAR`] is an XML document parsed by
//! [`super::xml`], so the numbers live in one readable place and [`BarStyle::minimal`] is
//! the fallback the parser is checked against.
//!
//! **The strip's height and its targets are the declared bar's** — D-1's dense-pointer row, so
//! a pinned window's controls are the same 40 dp targets, and the debug lanes' "a claimed
//! control is a pointer target" rule keeps holding. What is minimal is the *content*: the
//! window's name is gone and the marks are 14 dp instead of 20, which is what makes a pinned
//! panel read as an instrument rather than as a window with a title.
//!
//! # What this module is not
//!
//! It draws the bar and answers a press on it. It does not own window state, focus, the
//! arrangement, or the desktop — `window.rs` composes the window, `desktop.rs` owns focus,
//! and the caller hands it the title it should show (the reference's `set_title`, 2476).
//! **The pin state is the window's**, and this module is handed the answer.

use std::sync::OnceLock;

use voxel_package::recipe::{BehaviorPolicy, LayoutPolicy, WindowSpec};
use voxel_ui::{
    contrast_over_backdrop, glyph_for, min_gap_dp, pin_mark, pointer_glyph_dp, pointer_target_dp,
    spacing_dp, title_size_dp, Align, Element, GlyphAction, Ink, Paint, Palette, Rect, UiResult,
    WHITE,
};

use super::xml::Document;

/// **The pinned window's title bar, declared.** No title row, smaller marks, a shorter strip.
///
/// The document is the same constrained XML the map's controls are (`super::xml`), which is
/// why that grammar moved out of `webview` and into the shell: two surfaces read it now.
pub const MINIMAL_TITLEBAR: &str = r#"
<titlebar form="minimal" height="40" lane="40" glyph="14" gap="8" title="none"/>
"#;

/// Which of the two bars a window draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarForm {
    /// The ported bar: one row of dense-pointer targets, the window's name, its controls.
    Declared,
    /// The pinned window's strip: no name, smaller marks, shorter.
    Minimal,
}

/// **One bar's geometry**, read by the composite, the drawing and the hit tests alike.
///
/// The composite asks for the height before anything else exists, so the form has to be a
/// value that can be computed from the declaration plus the pin state alone.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BarStyle {
    /// Which form this is; the drawing branches on it, the arithmetic reads the numbers.
    pub form: BarForm,
    /// **The window's pin state**, which chose this form. Carried so the pin's own mark can
    /// state it without a second argument the caller could get wrong.
    pub pinned: bool,
    /// The strip's height.
    pub height_dp: f64,
    /// One control's lane width, and the lane height (a lane fills the bar).
    pub lane_dp: f64,
    /// One control's mark size inside its lane.
    pub glyph_dp: f64,
    /// The gap between adjacent lanes.
    pub gap_dp: f64,
    /// Whether the window's name is drawn at the start of the strip.
    pub shows_title: bool,
}

impl BarStyle {
    /// The ported bar: the dense-pointer row and the window's name.
    pub fn declared() -> Self {
        BarStyle {
            form: BarForm::Declared,
            pinned: false,
            height_dp: pointer_target_dp(),
            lane_dp: pointer_target_dp(),
            glyph_dp: pointer_glyph_dp(),
            gap_dp: min_gap_dp(),
            shows_title: true,
        }
    }

    /// The fallback the [`MINIMAL_TITLEBAR`] document is checked against — the numbers a
    /// pinned window draws when the declaration cannot be read.
    pub fn minimal() -> Self {
        BarStyle {
            form: BarForm::Minimal,
            pinned: true,
            height_dp: pointer_target_dp(),
            lane_dp: pointer_target_dp(),
            glyph_dp: 14.0,
            gap_dp: min_gap_dp(),
            shows_title: false,
        }
    }

    /// **The bar a window draws, from its pin state.**
    ///
    /// This is the only place the state chooses the chrome: `window.rs` asks it for the
    /// composite, the drawing asks it for the marks, and `Mount::control_lanes` asks it for
    /// the lanes a test or a click resolves against — one answer, three readers.
    pub fn of(pinned: bool) -> Self {
        if pinned {
            minimal()
        } else {
            BarStyle::declared()
        }
    }

    /// The bar's rect inside the window.
    pub fn rect(self, window: Rect) -> Rect {
        Rect::new(window.x, window.y, window.width_dp, self.height_dp)
    }
}

/// **The minimal bar, as the document declares it**, parsed once.
///
/// A document this build cannot read is the fallback plus a name: the bar is chrome, and a
/// pinned window whose strip refused to exist would hide the window's own controls. The test
/// `the_minimal_bar_document_is_the_style_it_names` is what keeps the two honest.
fn minimal() -> BarStyle {
    static PARSED: OnceLock<Option<BarStyle>> = OnceLock::new();
    let parsed = PARSED.get_or_init(|| parse_minimal(MINIMAL_TITLEBAR).ok());
    parsed.unwrap_or_else(BarStyle::minimal)
}

/// Read one `<titlebar>` document into a [`BarStyle`]. Refused by name when a number is not a
/// number or the root is not the element this module declares.
fn parse_minimal(source: &str) -> Result<BarStyle, String> {
    let document = Document::parse(source)?;
    let root = &document.root;
    if root.name != "titlebar" {
        return Err(format!(
            "the root is <{}>; the minimal bar is <titlebar>",
            root.name
        ));
    }
    let form = match root.attr("form") {
        Some("minimal") => BarForm::Minimal,
        other => {
            return Err(format!(
                "<titlebar> form={other:?} is not a bar form; the one this module declares is \
                 \"minimal\""
            ))
        }
    };
    let title = match root.attr("title") {
        Some("none") => false,
        Some("shown") => true,
        other => {
            return Err(format!(
                "<titlebar> title={other:?} is not a title rule; the rules are \"none\" and \
                 \"shown\""
            ))
        }
    };
    Ok(BarStyle {
        form,
        // The document declares the *form*; the form is the pinned bar, so the declaration's
        // own reading of it is pinned. `BarStyle::of` is the only chooser.
        pinned: true,
        height_dp: root.number("height")?,
        lane_dp: root.number("lane")?,
        glyph_dp: root.number("glyph")?,
        gap_dp: root.number("gap")?,
        shows_title: title,
    })
}

/// The height of the **declared** title bar: one row of MD1's dense-pointer targets, the same
/// derivation `TITLE_BAR_DP` carries for the touch floor (a title bar is a single-line row
/// holding its controls, so it is the target's own height).
///
/// A pinned window is shorter — ask [`BarStyle::of`] when the window's state is known.
pub fn bar_height_dp() -> f64 {
    pointer_target_dp()
}

/// Whether the declaration permits a title bar at all — the tree's `title-position` token,
/// the reference's `TitleBarPosition.NONE` case (2784–2800).
pub fn has_bar(spec: &WindowSpec) -> bool {
    spec.layout_policy.permits(LayoutPolicy::TITLE_POSITION)
}

/// The controls this bar carries, in the reference's own order — **pin**, minimize, maximize,
/// close, left to right — and **not one more**: a control the declaration does not permit is
/// absent rather than drawn disabled.
///
/// **The pin is always there.** It is the shell's runtime state rather than a declared
/// permission, so every bar offers it; the other three are `LayoutPolicy` permits.
pub fn controls(spec: &WindowSpec) -> Vec<GlyphAction> {
    if !has_bar(spec) {
        return Vec::new();
    }
    GlyphAction::ALL
        .into_iter()
        .filter(|action| match flag(*action) {
            Some(policy) => spec.layout_policy.permits(policy),
            None => true,
        })
        .collect()
}

/// The layout bit that gates a control, or `None` for a control the declaration does not gate.
fn flag(action: GlyphAction) -> Option<LayoutPolicy> {
    match action {
        GlyphAction::Pin => None,
        GlyphAction::Minimize => Some(LayoutPolicy::MINIMIZE),
        GlyphAction::Maximize => Some(LayoutPolicy::MAXIMIZE),
        GlyphAction::Close => Some(LayoutPolicy::CLOSE),
    }
}

/// The bar's rect inside the window, in the declared form.
pub fn rect(window: Rect) -> Rect {
    BarStyle::declared().rect(window)
}

/// The control lanes, **right-aligned** (the reference puts the group at the layout's end),
/// each a `style.lane_dp` lane with `style.gap_dp` between neighbours.
///
/// A window too narrow for its own controls draws the ones that fit and drops the rest from
/// the **start** of the group — the close control is the last thing to disappear, because it
/// is the one that is always permitted when a bar exists.
pub fn control_lanes(window: Rect, spec: &WindowSpec, style: BarStyle) -> Vec<(GlyphAction, Rect)> {
    let bar = style.rect(window);
    let list = controls(spec);
    let lane = style.lane_dp;
    let gap = style.gap_dp;
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
/// takes the layout's stretch, 2470–2474). Empty in the minimal form, which draws no title.
pub fn title_rect(window: Rect, spec: &WindowSpec, style: BarStyle) -> Rect {
    let bar = style.rect(window);
    if !style.shows_title {
        return Rect::new(bar.x + spacing_dp(), bar.y, 0.0, 0.0);
    }
    let lanes = control_lanes(window, spec, style);
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
pub fn press(
    spec: &WindowSpec,
    window: Rect,
    at: (f64, f64),
    double: bool,
    style: BarStyle,
) -> TitlePress {
    if !has_bar(spec) {
        return TitlePress::None;
    }
    if !style.rect(window).contains(at) {
        return TitlePress::None;
    }
    for (action, lane) in control_lanes(window, spec, style) {
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
///
/// In the minimal form the title is not drawn at all and the marks are `style.glyph_dp`. The
/// pin's mark is the window's state: filled when pinned, hollow when not.
pub fn elements(
    spec: &WindowSpec,
    window: Rect,
    title: &str,
    focused: bool,
    palette: &Palette,
    style: BarStyle,
) -> UiResult<Vec<Element>> {
    if !has_bar(spec) {
        return Ok(Vec::new());
    }
    let bar = style.rect(window);
    let chrome = palette
        .get(voxel_ui::Role::Chrome)?
        .elevated(spec.elevation)?;
    let opacity = (spec.opacity as f64 / 1000.0).clamp(0.0, 1.0);

    let mut out = Vec::with_capacity(4 + controls(spec).len());
    out.push(Element::Surface {
        rect: bar,
        paint: Paint {
            rgb: chrome,
            alpha: opacity,
        },
    });
    out.push(Element::Rule {
        rect: Rect::new(bar.x, bar.bottom() - 1.0, bar.width_dp, 1.0),
        paint: Ink::Divider.paint(),
    });

    let backdrop = chrome.blend_over(WHITE, opacity);
    let ink = if focused { Ink::Full } else { Ink::Secondary };
    contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
    if style.shows_title {
        out.push(Element::Text {
            rect: title_rect(window, spec, style),
            text: title.to_string(),
            paint: ink.paint(),
            size_dp: title_size_dp(),
            align: Align::Start,
        });
    }

    let glyph_size = style.glyph_dp;
    for (action, lane) in control_lanes(window, spec, style) {
        let mark = match action {
            GlyphAction::Pin => pin_mark(style.pinned),
            other => glyph_for(other).text,
        };
        out.push(Element::Text {
            rect: Rect::new(
                lane.x + (lane.width_dp - glyph_size) / 2.0,
                lane.y + (lane.height_dp - glyph_size) / 2.0,
                glyph_size,
                glyph_size,
            ),
            text: mark.to_string(),
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

    fn style() -> BarStyle {
        BarStyle::declared()
    }

    #[test]
    fn a_policy_without_title_position_draws_no_bar_and_answers_nothing() {
        let s = spec(LayoutPolicy::CLOSE.0, BehaviorPolicy::DRAG.0);
        assert!(!has_bar(&s));
        let out = elements(&s, bar_window(), "t", true, &Palette::declared(), style()).unwrap();
        assert!(out.is_empty(), "no bar means no elements");
        assert_eq!(
            press(&s, bar_window(), (120.0, 60.0), false, style()),
            TitlePress::None
        );
    }

    #[test]
    fn the_controls_are_the_declared_ones_in_the_reference_order_and_not_one_more() {
        assert_eq!(
            controls(&spec(all_controls(), 0)),
            vec![
                GlyphAction::Pin,
                GlyphAction::Minimize,
                GlyphAction::Maximize,
                GlyphAction::Close
            ]
        );
        // **The pin is not gated**: a window that permits no chrome still offers it, because
        // pinning is the shell's runtime state rather than a declared permission.
        assert_eq!(
            controls(&spec(LayoutPolicy::TITLE_POSITION.0, 0)),
            vec![GlyphAction::Pin]
        );
        assert_eq!(controls(&spec(0, 0)), Vec::new(), "no bar, no controls");
    }

    #[test]
    fn the_lanes_are_targets_flush_with_the_end_and_separated_by_eight() {
        let s = spec(all_controls(), 0);
        let window = bar_window();
        let lanes = control_lanes(window, &s, style());
        assert_eq!(lanes.len(), 4);
        for (action, lane) in &lanes {
            assert_eq!(
                lane.width_dp,
                pointer_target_dp(),
                "{action:?} is not a target wide"
            );
            assert_eq!(lane.height_dp, bar_height_dp());
        }
        // The close control ends at the window's end, and the group runs left in declared order.
        assert_eq!(lanes[3].0, GlyphAction::Close);
        assert_eq!(lanes[3].1.right(), window.right());
        assert_eq!(lanes[2].0, GlyphAction::Maximize);
        assert_eq!(lanes[1].0, GlyphAction::Minimize);
        assert_eq!(lanes[0].0, GlyphAction::Pin);
        // HIG's separation, drawn rather than declared.
        assert_eq!(lanes[1].1.x - lanes[0].1.right(), min_gap_dp());
        assert_eq!(lanes[3].1.x - lanes[2].1.right(), min_gap_dp());
    }

    #[test]
    fn a_window_narrower_than_its_controls_keeps_the_close_control() {
        let s = spec(all_controls(), 0);
        let tiny = Rect::new(0.0, 0.0, 50.0, 60.0);
        let lanes = control_lanes(tiny, &s, style());
        assert_eq!(lanes.len(), 1, "one 40 dp lane fits in 50");
        assert_eq!(lanes[0].0, GlyphAction::Close);
    }

    #[test]
    fn the_minimal_bar_document_is_the_style_it_names() {
        // **The declaration is the value.** A number edited in one place and not the other is
        // the failure this test exists for: the strip would draw at one height and its lanes
        // would answer at another.
        let parsed = parse_minimal(MINIMAL_TITLEBAR).expect("the minimal bar declares");
        assert_eq!(parsed, BarStyle::minimal());
        assert_eq!(minimal(), BarStyle::minimal());
        assert!(!parsed.shows_title, "a minimal bar draws no window name");
        assert!(
            parsed.glyph_dp < pointer_glyph_dp(),
            "its marks are smaller than the declared bar's"
        );
        assert_eq!(
            parsed.lane_dp,
            pointer_target_dp(),
            "and its controls are still dense-pointer targets"
        );
        // A document that is not this element, or names a rule this module does not know, is
        // refused by name rather than drawn with invented numbers.
        assert!(parse_minimal("<bar height=\"28\"/>").is_err());
        assert!(parse_minimal("<titlebar form=\"fancy\"/>").is_err());
        assert!(parse_minimal("<titlebar form=\"minimal\" title=\"big\"/>").is_err());
    }

    #[test]
    fn a_pinned_window_draws_no_title_and_smaller_marks_through_the_same_lanes() {
        let s = spec(all_controls(), BehaviorPolicy::DRAG.0);
        let window = bar_window();
        let minimal = BarStyle::of(true);
        let out = elements(&s, window, "a window", true, &Palette::declared(), minimal).unwrap();
        // The surface and the rule, then one mark per control — and no title.
        assert_eq!(out.len(), 2 + controls(&s).len());
        assert!(
            !out.iter()
                .any(|element| matches!(element, Element::Text { text, .. } if text == "a window")),
            "the minimal bar does not name the window"
        );
        for element in &out[2..] {
            match element {
                Element::Text { size_dp, .. } => {
                    assert_eq!(*size_dp, minimal.glyph_dp);
                    assert!(minimal.glyph_dp < pointer_glyph_dp());
                }
                other => panic!("the marks are text, found {other:?}"),
            }
        }
        // The lanes are the minimal ones, and the bar's own rect is the shorter one.
        let lanes = control_lanes(window, &s, minimal);
        assert_eq!(lanes.len(), 4);
        assert_eq!(lanes[3].1.right(), window.right());
        assert_eq!(lanes[0].1.height_dp, minimal.height_dp);
        assert_eq!(minimal.rect(window).height_dp, minimal.height_dp);
    }

    #[test]
    fn the_pin_mark_states_which_side_the_window_is_on() {
        let s = spec(all_controls(), 0);
        let window = bar_window();
        let mark_of = |pinned: bool| {
            let style = BarStyle::of(pinned);
            let out = elements(&s, window, "t", true, &Palette::declared(), style).unwrap();
            // The pin is the first control lane, after the surface, the rule and (in the
            // declared form) the title.
            let offset = if style.shows_title { 3 } else { 2 };
            match &out[offset] {
                Element::Text { text, .. } => text.clone(),
                other => panic!("the pin's mark is text, found {other:?}"),
            }
        };
        assert_eq!(mark_of(false), voxel_ui::UNPINNED_MARK);
        assert_eq!(mark_of(true), voxel_ui::PINNED_MARK);
        assert_ne!(mark_of(true), mark_of(false));
    }

    #[test]
    fn the_title_never_runs_under_the_controls() {
        let s = spec(all_controls(), 0);
        let window = bar_window();
        let title = title_rect(window, &s, style());
        let lanes = control_lanes(window, &s, style());
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
        let lanes = control_lanes(window, &s, style());
        let (action, lane) = lanes[2];
        let centre = (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0);
        assert_eq!(
            press(&s, window, centre, false, style()),
            TitlePress::Control(action)
        );
        // Off the group, on the bar: a drag.
        let bar_point = (window.x + spacing_dp(), window.y + bar_height_dp() / 2.0);
        assert_eq!(
            press(&s, window, bar_point, false, style()),
            TitlePress::Drag
        );
        // The same point with drag not permitted answers nothing.
        let still = spec(all_controls(), 0);
        assert_eq!(
            press(&still, window, bar_point, false, style()),
            TitlePress::None
        );
        // **And the pin answers in the minimal bar too**, at the shorter lane it draws in.
        let minimal = BarStyle::of(true);
        let pin_lane = control_lanes(window, &s, minimal)[0].1;
        let pin_at = (
            pin_lane.x + pin_lane.width_dp / 2.0,
            pin_lane.y + pin_lane.height_dp / 2.0,
        );
        assert_eq!(
            press(&s, window, pin_at, false, minimal),
            TitlePress::Control(GlyphAction::Pin)
        );
    }

    #[test]
    fn a_double_press_maximizes_only_where_the_policy_permits() {
        let window = bar_window();
        let point = (window.x + 100.0, window.y + 10.0);
        let may = spec(all_controls(), BehaviorPolicy::MAXIMIZE.0);
        assert_eq!(
            press(&may, window, point, true, style()),
            TitlePress::Maximize
        );
        let may_not = spec(all_controls(), BehaviorPolicy::DRAG.0);
        assert_eq!(
            press(&may_not, window, point, true, style()),
            TitlePress::None
        );
    }

    #[test]
    fn the_unfocused_title_dims_one_step_and_the_contrast_is_measured() {
        let s = spec(all_controls(), BehaviorPolicy::DRAG.0);
        let window = bar_window();
        let focused = elements(&s, window, "t", true, &Palette::declared(), style()).unwrap();
        let unfocused = elements(&s, window, "t", false, &Palette::declared(), style()).unwrap();
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
        let out = elements(&s, bar_window(), "t", true, &Palette::declared(), style()).unwrap();
        let glyph = out.last().unwrap();
        match glyph {
            Element::Text { size_dp, .. } => assert_eq!(*size_dp, pointer_glyph_dp()),
            other => panic!("the last element is a glyph, found {other:?}"),
        }
    }
}
