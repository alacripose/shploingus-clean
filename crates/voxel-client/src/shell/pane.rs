//! **The pane: its header, its controls, and the slot below them.**
//!
//! Reference: `docs/portal_simulator.py` — `PaneHeader` 2516–2582 (the title 2522–2532,
//! `◧` 2534–2542, `⬓` 2544–2552, `⇱` 2554–2562, `×` 2564–2572, `update_title` 2576–2582) and
//! `WindowPane` 2653–2718 (the header slot 2666–2675, content-or-placeholder 2677–2685, the
//! 40×40 floor 2686–2688, click-to-focus 2712–2718). Step 1 of `docs/SHELL-PORT.md`'s map.
//!
//! # What changes, and why it is named
//!
//! * **The header is one row of MD1's dense-pointer targets.** The reference's header is 26
//!   tall with 22×20 buttons; D-1 (ruled 2026-10-03) makes the header one
//!   [`pointer_target_dp`] row and every control a 40 dp lane with the 20 dp glyph and
//!   [`min_gap_dp`] between neighbours. The reference is denser still; the floor wins.
//! * **The marks are the reference's own Unicode glyphs**, declared in `voxel-ui`'s
//!   [`PANE_MARKS`](voxel_ui::PANE_MARKS) by the user's ruling of 2026-10-03.
//! * **Hover is the tree's ruled interaction language** — one elevation step (`UI-SPEC.md`
//!   §8.1: *"Hover 1 step"*), not the reference's stylesheet colours.
//! * **The undock control is absent when the window holds one pane** — the reference's
//!   `update_titlebar_and_taskbar` hides `⇱` at one pane (3141–3164); the caller states it
//!   with `undock_shown`.
//! * **The tooltips are not drawn.** The reference titles each button (*"Split pane right
//!   (vertical divider)"*, 2536). This tree has no tooltip surface and UI-SPEC §8.2's
//!   keyboard/interaction states are still owed; the strings live in `PANE_MARKS`' sources
//!   so they can be drawn when that surface exists.
//!
//! # What this module is not
//!
//! It draws a pane's header and answers a press on it. It does not own the arrangement, the
//! window, focus, or the content hosted in the slot — filling a slot (the reference's
//! `set_app`, 2697–2711) is an arrangement mutation and belongs to `window.rs`; the body's
//! paint and the pane boundaries belong to the window's layout pass, which lays out the same
//! rects this module hit-tests (`SHELL-PORT.md` §5: one rect arithmetic).

use voxel_ui::{
    contrast_over_backdrop, min_gap_dp, pane_mark_for, pointer_glyph_dp, pointer_target_dp,
    spacing_dp, title_size_dp, Align, Element, Ink, Palette, Paint, PaneMark, Rect, Role,
    UiResult, PLACEHOLDER_MARK, WHITE,
};

/// The header's height: **one row of MD1's dense-pointer targets**, the same derivation the
/// title bar carries.
pub fn header_height_dp() -> f64 {
    pointer_target_dp()
}

/// The header's rect, at the pane's top — the reference adds the header with stretch 0 and the
/// content with stretch 1 (2666–2685).
pub fn header_rect(pane: Rect) -> Rect {
    Rect::new(pane.x, pane.y, pane.width_dp, header_height_dp())
}

/// The slot below the header — the rect the hosted content (or the placeholder) occupies.
pub fn body(pane: Rect) -> Rect {
    Rect::new(
        pane.x,
        pane.y + header_height_dp(),
        pane.width_dp,
        (pane.height_dp - header_height_dp()).max(0.0),
    )
}

/// The title an **empty** slot's header carries — the reference's own string, built from the
/// declared placeholder mark (2576–2582: `"◫  New Split Slot"`).
pub fn empty_pane_title() -> String {
    format!("{PLACEHOLDER_MARK}  New Split Slot")
}

/// The name an empty slot contributes to a **multi-pane window's** composed title — the
/// reference's own `"◫ Split"` (3155), which is deliberately shorter than
/// [`empty_pane_title`] because it sits in a `'   |   '`-joined row of names.
pub fn split_slot_title() -> String {
    format!("{PLACEHOLDER_MARK} Split")
}

/// The controls the header carries, in the reference's own order — split right, split down,
/// undock, close (2534–2572) — with the undock control present only when the window holds more
/// than one pane.
pub fn actions(undock_shown: bool) -> Vec<PaneMark> {
    let mut out = vec![PaneMark::SplitHorizontal, PaneMark::SplitVertical];
    if undock_shown {
        out.push(PaneMark::Undock);
    }
    out.push(PaneMark::Close);
    out
}

/// The action lanes, **right-aligned** (the reference adds the buttons last, after the title's
/// stretch), each a [`pointer_target_dp`] lane with [`min_gap_dp`] between neighbours. A pane
/// too narrow for its own controls keeps the ones nearest the end — the close control is the
/// last to disappear.
pub fn action_lanes(pane: Rect, undock_shown: bool) -> Vec<(PaneMark, Rect)> {
    let header = header_rect(pane);
    let lane = pointer_target_dp();
    let gap = min_gap_dp();
    let list = actions(undock_shown);
    let mut out = Vec::with_capacity(list.len());
    for (index, action) in list.iter().rev().enumerate() {
        let x = header.right() - (index as f64 + 1.0) * lane - index as f64 * gap;
        if x < header.x {
            break;
        }
        out.push((*action, Rect::new(x, header.y, lane, header.height_dp)));
    }
    out.reverse();
    out
}

/// The title's rect: the header's inline space **up to where the controls begin**, so a title
/// is clipped by its own box rather than drawn under a control.
pub fn title_rect(pane: Rect, undock_shown: bool) -> Rect {
    let header = header_rect(pane);
    let lanes = action_lanes(pane, undock_shown);
    let right = lanes
        .first()
        .map(|(_, lane)| lane.x - spacing_dp())
        .unwrap_or(header.right() - spacing_dp());
    Rect::new(
        header.x + spacing_dp(),
        header.y + (header.height_dp - title_size_dp()) / 2.0,
        (right - header.x - spacing_dp()).max(0.0),
        title_size_dp(),
    )
}

/// What a press on the pane means. The reference answers focus on the pane itself
/// (`mousePressEvent`, 2712–2718) and each button answers its own action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanePress {
    /// A header control was pressed.
    Action(PaneMark),
    /// The pane's body was pressed: **focus the window that holds it** (2712–2718).
    Body,
    /// Nothing: a point outside the pane.
    None,
}

/// The verdict for a press at `at`, in the pane's own coordinates.
pub fn press(pane: Rect, undock_shown: bool, at: (f64, f64)) -> PanePress {
    if !pane.contains(at) {
        return PanePress::None;
    }
    if header_rect(pane).contains(at) {
        for (action, lane) in action_lanes(pane, undock_shown) {
            if lane.contains(at) {
                return PanePress::Action(action);
            }
        }
        return PanePress::None;
    }
    PanePress::Body
}

/// The header's elements: its surface (the declared elevation tinted onto `Chrome`), the rule
/// under it, the title, and each control's glyph. A hovered lane is drawn **one elevation step
/// raised** — the tree's ruled hover language — with its glyph at full ink.
pub fn header_elements(
    pane: Rect,
    title: &str,
    undock_shown: bool,
    hovered: Option<PaneMark>,
    level: u8,
    opacity: f64,
    palette: &Palette,
) -> UiResult<Vec<Element>> {
    let header = header_rect(pane);
    let chrome = palette.get(Role::Chrome)?.elevated(level)?;
    let opacity = opacity.clamp(0.0, 1.0);

    let mut out = Vec::with_capacity(4 + 2 * actions(undock_shown).len());
    out.push(Element::Surface {
        rect: header,
        paint: Paint { rgb: chrome, alpha: opacity },
    });
    out.push(Element::Rule {
        rect: Rect::new(header.x, header.bottom() - 1.0, header.width_dp, 1.0),
        paint: Ink::Divider.paint(),
    });

    let backdrop = chrome.blend_over(WHITE, opacity);
    contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;
    out.push(Element::Text {
        rect: title_rect(pane, undock_shown),
        text: title.to_string(),
        paint: Ink::Full.paint(),
        size_dp: title_size_dp(),
        align: Align::Start,
    });

    let glyph_size = pointer_glyph_dp();
    for (action, lane) in action_lanes(pane, undock_shown) {
        let mark = pane_mark_for(action);
        let is_hovered = hovered == Some(action);
        if is_hovered {
            out.push(Element::Surface {
                rect: lane,
                paint: Paint { rgb: chrome.elevated(1)?, alpha: opacity },
            });
        }
        out.push(Element::Text {
            rect: Rect::new(
                lane.x + (lane.width_dp - glyph_size) / 2.0,
                lane.y + (lane.height_dp - glyph_size) / 2.0,
                glyph_size,
                glyph_size,
            ),
            text: mark.text.to_string(),
            paint: if is_hovered { Ink::Full } else { Ink::Secondary }.paint(),
            size_dp: glyph_size,
            align: Align::Start,
        });
    }
    Ok(out)
}

/// The pane's own chrome: the header, and nothing else. The body's paint and the pane's
/// boundaries are the window's layout pass (one rect arithmetic), and the content in the slot
/// is the host's (`window.rs`) or the placeholder's.
pub fn elements(
    pane: Rect,
    title: &str,
    undock_shown: bool,
    hovered: Option<PaneMark>,
    level: u8,
    opacity: f64,
    palette: &Palette,
) -> UiResult<Vec<Element>> {
    header_elements(pane, title, undock_shown, hovered, level, opacity, palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane() -> Rect {
        Rect::new(200.0, 100.0, 600.0, 400.0)
    }

    #[test]
    fn an_empty_slot_carries_the_reference_s_own_title() {
        assert_eq!(empty_pane_title(), format!("{PLACEHOLDER_MARK}  New Split Slot"));
        assert_eq!(split_slot_title(), format!("{PLACEHOLDER_MARK} Split"));
    }

    #[test]
    fn the_actions_are_the_reference_s_four_and_undock_hides_with_one_pane() {
        assert_eq!(
            actions(true),
            vec![
                PaneMark::SplitHorizontal,
                PaneMark::SplitVertical,
                PaneMark::Undock,
                PaneMark::Close
            ]
        );
        assert_eq!(
            actions(false),
            vec![PaneMark::SplitHorizontal, PaneMark::SplitVertical, PaneMark::Close]
        );
    }

    #[test]
    fn the_lanes_are_pointer_targets_flush_with_the_end_and_carry_the_gap() {
        let p = pane();
        let lanes = action_lanes(p, true);
        assert_eq!(lanes.len(), 4);
        for (action, lane) in &lanes {
            assert_eq!(lane.width_dp, pointer_target_dp(), "{action:?} is not a target wide");
            assert_eq!(lane.height_dp, header_height_dp());
        }
        assert_eq!(lanes[3].0, PaneMark::Close);
        assert_eq!(lanes[3].1.right(), p.right());
        for pair in lanes.windows(2) {
            assert_eq!(pair[1].1.x - pair[0].1.right(), min_gap_dp());
        }
    }

    #[test]
    fn a_press_lower_than_the_header_is_the_body_and_focuses_it() {
        let p = pane();
        let below = (p.x + 10.0, p.y + header_height_dp() + 10.0);
        assert_eq!(press(p, true, below), PanePress::Body);
        let outside = (p.x - 5.0, p.y + 5.0);
        assert_eq!(press(p, true, outside), PanePress::None);
    }

    #[test]
    fn a_press_on_a_control_answers_that_control_and_a_hidden_one_never_answers() {
        let p = pane();
        let lanes = action_lanes(p, true);
        let (action, lane) = lanes[0];
        let centre = (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0);
        assert_eq!(press(p, true, centre), PanePress::Action(action));
        // With one pane the undock lane does not exist; the coordinate answers something
        // else, but never the absent control.
        let (_, undock_lane) = lanes[2];
        let undock_centre = (
            undock_lane.x + undock_lane.width_dp / 2.0,
            undock_lane.y + undock_lane.height_dp / 2.0,
        );
        assert_ne!(
            press(p, false, undock_centre),
            PanePress::Action(PaneMark::Undock)
        );
    }

    #[test]
    fn the_header_is_one_pointer_row_and_the_body_starts_under_it() {
        let p = pane();
        assert_eq!(header_rect(p).height_dp, pointer_target_dp());
        let body = body(p);
        assert_eq!(body.y, p.y + pointer_target_dp());
        assert_eq!(body.height_dp, p.height_dp - pointer_target_dp());
        assert_eq!(body.width_dp, p.width_dp);
    }

    #[test]
    fn a_hovered_lane_is_drawn_one_step_raised_and_its_glyph_at_full_ink() {
        let p = pane();
        let palette = Palette::declared();
        let hovered = elements(p, "a window", true, Some(PaneMark::Close), 0, 1.0, &palette)
            .expect("the header draws");
        let plain = elements(p, "a window", true, None, 0, 1.0, &palette)
            .expect("the header draws");
        assert_eq!(hovered.len(), plain.len() + 1, "the hover adds one surface");
        let chrome = palette.get(Role::Chrome).unwrap();
        let raised = chrome.elevated(1).unwrap();
        let hover_surface = hovered
            .iter()
            .filter_map(|element| match element {
                Element::Surface { rect, paint } if *rect == action_lanes(p, true)[3].1 => {
                    Some(*paint)
                }
                _ => None,
            })
            .next()
            .expect("the hovered lane is drawn");
        assert_eq!(hover_surface.rgb, raised, "hover is one elevation step");
    }

    #[test]
    fn the_title_never_runs_under_the_controls() {
        let p = pane();
        let title = title_rect(p, true);
        let lanes = action_lanes(p, true);
        assert!(title.right() <= lanes[0].1.x);
        assert!(title.x >= p.x);
    }
}
