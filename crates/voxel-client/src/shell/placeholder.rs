//! **The empty slot: what a pane shows before a window lands in it.**
//!
//! Reference: `docs/portal_simulator.py` — `PanePlaceholder` 2583–2631 (the mark 2587–2598,
//! the note 2600–2605, the app grid 2607–2622), and the stylesheet's own treatment
//! (`QFrame#panePlaceholder`: a **dashed** 2 px border, radius 6, margin 8, padding 12).
//! Step 1 of `docs/SHELL-PORT.md`'s map.
//!
//! # What changes, and why it is named
//!
//! * **The grid lists this tree's windows, not the reference's apps.** The reference hard-codes
//!   ten app buttons (2611–2618: *"👑 Autonomy"*, *"🏢 Biz DB"*, …); this game's slots are
//!   filled from its **package** (H-9.7), so the caller supplies `(id, label)` offers and this
//!   module draws and hit-tests them.
//! * **The note names the record's unit.** The reference says *"Drag a window here or launch an
//!   app:"*; this tree has one kind of window (UI-SPEC §1), so the note reads *"Drag a window
//!   here or launch one:"*, and an empty offer list says **why** it is empty rather than
//!   drawing an empty grid.
//! * **The mark is drawn in the ink ladder, not in `accent`.** The reference draws the slot's
//!   mark in `theme.accent` (4105–4109); **`Role::Accent` is an unadmitted role in this tree's
//!   palette** and `Palette::get` refuses it by name. The mark therefore draws at full ink
//!   until `theme.rs` (step 2) admits the role — recorded, not defaulted.
//! * **The border is drawn dashed.** The renderer has one rule primitive, so the dash is a run
//!   of short rules — the reference's own treatment, not a solid line standing in for it.
//! * **Every offer is a pointer target**: 40 dp rows, 8 dp gaps, three per row like the
//!   reference's `idx // 3`/`idx % 3` grid.
//!
//! # What this module is not
//!
//! It draws the slot and answers a press. **Filling the slot is an arrangement mutation** —
//! the reference's `set_app` (2697–2711) replaces the pane's widget; here the caller mutates
//! the pane tree through `arrangement.rs` and the slot stops being empty.

use voxel_ui::{
    contrast_over_backdrop, min_gap_dp, pointer_target_dp, spacing_dp, text_size_dp,
    title_size_dp, Align, Element, Ink, Palette, Paint, Rect, Role, UiResult, PLACEHOLDER_MARK,
    WHITE,
};

/// The dash and gap of the slot's border, in dp. **A drawn detail of the reference's own
/// treatment**, not an MD1 value: MD1 names no dash pattern, and the stylesheet's border is
/// dashed.
const DASH_DP: f64 = 4.0;

/// How many offers sit on one row — the reference's grid is three across (2619–2622).
pub fn columns() -> usize {
    3
}

/// The slot's frame: the pane's body inset by the grid's own step (the reference's margin 8 +
/// padding 12; the tree's 8 dp grid absorbs the sum).
pub fn frame(pane: Rect) -> Rect {
    let inset = spacing_dp();
    Rect::new(
        pane.x + inset,
        pane.y + inset,
        (pane.width_dp - 2.0 * inset).max(0.0),
        (pane.height_dp - 2.0 * inset).max(0.0),
    )
}

/// The slot's mark: the reference's own `◫`, at the display size.
pub fn mark_rect(frame: Rect) -> Rect {
    let size = title_size_dp();
    Rect::new(frame.x + (frame.width_dp - size) / 2.0, frame.y, size, size)
}

/// The note under the mark. The reference centres it; the tree's grid does the same.
pub fn note_rect(frame: Rect) -> Rect {
    Rect::new(
        frame.x + spacing_dp(),
        mark_rect(frame).bottom() + spacing_dp(),
        (frame.width_dp - 2.0 * spacing_dp()).max(0.0),
        text_size_dp(),
    )
}

/// The note's text for a given offer count.
pub fn note(offers: usize) -> &'static str {
    if offers == 0 {
        "no declared window is free to place here"
    } else {
        "Drag a window here or launch one:"
    }
}

/// The offer lanes, three per row, each a [`pointer_target_dp`] row, centred as rows — a row
/// of one or two is centred rather than left-hung. The block sits below the note.
pub fn offer_lanes(frame: Rect, offers: usize) -> Vec<Rect> {
    let per_row = columns();
    if offers == 0 {
        return Vec::new();
    }
    let gap = min_gap_dp();
    let column = (frame.width_dp - (per_row as f64 - 1.0) * gap) / per_row as f64;
    if column <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(offers);
    let mut y = note_rect(frame).bottom() + 2.0 * spacing_dp();
    let mut index = 0usize;
    while index < offers {
        let in_row = (offers - index).min(per_row);
        let row_width = in_row as f64 * column + (in_row as f64 - 1.0) * gap;
        let mut x = frame.x + (frame.width_dp - row_width) / 2.0;
        for _ in 0..in_row {
            out.push(Rect::new(x, y, column, pointer_target_dp()));
            x += column + gap;
        }
        index += in_row;
        y += pointer_target_dp() + gap;
    }
    out
}

/// The index of the offer a press lands on, or `None`.
pub fn press(frame: Rect, offers: usize, at: (f64, f64)) -> Option<usize> {
    offer_lanes(frame, offers)
        .into_iter()
        .position(|lane| lane.contains(at))
}

/// The slot's elements: the frame's surface (the alternative surface role — the reference's
/// own `t.panel`), its dashed border, the mark, the note, and the offer rows.
pub fn elements(
    pane: Rect,
    offers: &[(String, String)],
    level: u8,
    opacity: f64,
    palette: &Palette,
) -> UiResult<Vec<Element>> {
    let frame = frame(pane);
    if frame.width_dp <= 0.0 || frame.height_dp <= 0.0 {
        return Ok(Vec::new());
    }
    let surface = palette.get(Role::SurfaceAlt)?.elevated(level)?;
    let opacity = opacity.clamp(0.0, 1.0);
    let backdrop = surface.blend_over(WHITE, opacity);
    contrast_over_backdrop(Ink::Secondary.composited_over(backdrop), backdrop)?;
    contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;

    let mut out = Vec::with_capacity(8 + 2 * offers.len());
    out.push(Element::Surface {
        rect: frame,
        paint: Paint { rgb: surface, alpha: opacity },
    });
    // The dashed border: runs of short rules along all four edges.
    for dash in dashed_edges(frame) {
        out.push(Element::Rule { rect: dash, paint: Ink::Divider.paint() });
    }
    out.push(Element::Text {
        rect: mark_rect(frame),
        text: PLACEHOLDER_MARK.to_string(),
        paint: Ink::Full.paint(),
        size_dp: title_size_dp(),
        align: Align::Start,
    });
    out.push(Element::Text {
        rect: note_rect(frame),
        text: note(offers.len()).to_string(),
        paint: Ink::Secondary.paint(),
        size_dp: text_size_dp(),
        align: Align::Start,
    });

    let button = palette.get(Role::Chrome)?.elevated(level)?;
    for (index, lane) in offer_lanes(frame, offers.len()).into_iter().enumerate() {
        let (_, label) = &offers[index];
        out.push(Element::Surface {
            rect: lane,
            paint: Paint { rgb: button, alpha: opacity },
        });
        out.push(Element::Text {
            rect: Rect::new(
                lane.x + spacing_dp() / 2.0,
                lane.y + (lane.height_dp - text_size_dp()) / 2.0,
                (lane.width_dp - spacing_dp()).max(0.0),
                text_size_dp(),
            ),
            text: label.clone(),
            paint: Ink::Full.paint(),
            size_dp: text_size_dp(),
            align: Align::Start,
        });
    }
    Ok(out)
}

/// The frame's border as dashes: [`DASH_DP`] on, [`DASH_DP`] off, along each edge.
///
/// Shared with `drop.rs`, which draws the reference's dashed preview border with the same dash.
pub(crate) fn dashed_edges(frame: Rect) -> Vec<Rect> {
    let mut out = Vec::new();
    let step = 2.0 * DASH_DP;
    let mut x = frame.x;
    while x < frame.right() {
        let width = DASH_DP.min(frame.right() - x);
        out.push(Rect::new(x, frame.y, width, 1.0));
        out.push(Rect::new(x, frame.bottom() - 1.0, width, 1.0));
        x += step;
    }
    let mut y = frame.y;
    while y < frame.bottom() {
        let height = DASH_DP.min(frame.bottom() - y);
        out.push(Rect::new(frame.x, y, 1.0, height));
        out.push(Rect::new(frame.right() - 1.0, y, 1.0, height));
        y += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane() -> Rect {
        Rect::new(0.0, 0.0, 600.0, 400.0)
    }

    fn offers(n: usize) -> Vec<(String, String)> {
        (0..n)
            .map(|index| (format!("w{index}"), format!("Window {index}")))
            .collect()
    }

    #[test]
    fn an_empty_slot_says_why_it_is_empty() {
        let out = elements(pane(), &[], 0, 1.0, &Palette::declared()).expect("draws");
        let texts: Vec<&str> = out
            .iter()
            .filter_map(|element| match element {
                Element::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&PLACEHOLDER_MARK));
        assert!(texts.contains(&"no declared window is free to place here"));
        assert!(!texts.iter().any(|text| text.contains("launch one")), "no empty grid, no invitation");
    }

    #[test]
    fn the_offers_are_laid_out_three_per_row_and_a_short_row_is_centred() {
        let frame = frame(pane());
        let lanes = offer_lanes(frame, 4);
        assert_eq!(lanes.len(), 4);
        // The first three share a row; the fourth is alone and centred.
        assert_eq!(lanes[0].y, lanes[1].y);
        assert_eq!(lanes[1].y, lanes[2].y);
        assert!(lanes[3].y > lanes[2].y);
        let centre = lanes[3].x + lanes[3].width_dp / 2.0;
        let frame_centre = frame.x + frame.width_dp / 2.0;
        assert!((centre - frame_centre).abs() < 1e-9, "a short row is centred");
    }

    #[test]
    fn a_press_inside_an_offer_returns_its_index_and_elsewhere_is_none() {
        let frame = frame(pane());
        let lanes = offer_lanes(frame, 4);
        let centre = (lanes[2].x + 1.0, lanes[2].y + 1.0);
        assert_eq!(press(frame, 4, centre), Some(2));
        let above = (frame.x + 1.0, frame.y + 1.0);
        assert_eq!(press(frame, 4, above), None);
    }

    #[test]
    fn the_border_is_drawn_as_dashes_not_a_single_rule() {
        let frame = frame(pane());
        let out = elements(pane(), &[], 0, 1.0, &Palette::declared()).expect("draws");
        let rules: Vec<Rect> = out
            .iter()
            .filter_map(|element| match element {
                Element::Rule { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        assert!(rules.len() > 10, "a dashed border is many short rules, found {}", rules.len());
        assert!(
            !rules.iter().any(|rule| rule.x == frame.x && rule.width_dp > DASH_DP + 1e-9),
            "no single rule spans an edge"
        );
    }

    #[test]
    fn the_note_names_the_record_s_unit_and_the_offer_rows_meet_the_target() {
        assert_eq!(note(1), "Drag a window here or launch one:");
        let out = elements(pane(), &offers(2), 0, 1.0, &Palette::declared()).expect("draws");
        let labels: Vec<&str> = out
            .iter()
            .filter_map(|element| match element {
                Element::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"Window 0") && labels.contains(&"Window 1"));
        for lane in offer_lanes(frame(pane()), 2) {
            assert_eq!(lane.height_dp, pointer_target_dp());
        }
    }
}
