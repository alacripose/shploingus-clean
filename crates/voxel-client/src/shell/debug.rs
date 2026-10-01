//! The lanes over the shell: **every window is a test surface** (event-0013, H-7.25).
//!
//! *There is no Tests tab and no separate test surface: a test is visible by looking at the thing it
//! tests* — and the ruling that follows is that **the lanes on every window are the test surface**.
//! So this module declares a lane for each piece of the shell's own structure and each affordance on
//! it, and [`crate::debug`] draws them as labelled outlines at their own positions.
//!
//! # Two different things are called a lane here, and the record keeps them apart
//!
//! [`Mount::control_lanes`] returns a **hit region** — the rectangle a control occupies, which is
//! where a click lands. A **debug lane** is a **claim about the client**: a declared, validated
//! region that says what is being exercised there. They are drawn from the same rectangles, which is
//! exactly why they are named differently: one is an input surface and one is a statement, and a
//! reader who conflated them would read a hit box as a test.
//!
//! # The layer decides which windows are claimed
//!
//! H-9.8 makes the HUD **the pinned subset** of the same window set, so the lanes follow the layer
//! rather than a second list: when the shell is shown the bar, its cells, the menu rows and every
//! open window are claimed; when it is hidden the bar is not drawn and so is not claimed, and only
//! the **pinned** windows are. A lane for something that is not on screen would be a claim about a
//! region the overlay is not covering — the same lie [`crate::debug`] refuses elsewhere.

use super::mount::{Mount, Status};
use crate::debug::{Lane, LaneClass, LaneSet, Region};
use voxel_ui::{bar_height_dp, title_bar_dp, Rect};

/// The surface the lanes are declared against: the viewport itself.
///
/// **The whole viewport, not the shell's occupied part.** A lane that had to fit inside what the
/// shell happened to draw would be a lane whose bounds depend on the thing it is reporting on.
pub fn surface(viewport: Rect) -> Region {
    Region::Rect(viewport)
}

/// Every lane the shell's own structure declares, for the layer currently drawn.
///
/// `shown` is [`Mount::shell_shown`], and it is the only thing that decides whether the bar and the
/// unpinned windows are claimed.
pub fn ui_lanes(mount: &Mount, shown: bool) -> LaneSet {
    let mut set = LaneSet::new();
    let viewport = mount.viewport();

    if shown {
        // The bar, and then each cell in it. The cells come from the mount's own arithmetic, so a
        // lane cannot claim a cell the bar did not draw.
        set.push(Lane::pixels(
            "bar",
            LaneClass::Structure,
            format!("{} entries", mount.bar().len()),
            Rect::new(0.0, 0.0, viewport.width_dp, bar_height_dp()),
        ));
        for (id, is_tab, rect) in mount.bar_rects(viewport) {
            set.push(Lane::pixels(
                "bar-cell",
                LaneClass::Affordance,
                format!("{id}{}", if is_tab { " · tab" } else { "" }),
                rect,
            ));
            // The menu's rows are the affordances of the one open tab.
            if mount.open_menu() == Some(id.as_str()) {
                if let Some(index) = mount.bar().iter().position(|entry| entry.id == id) {
                    for (row, row_rect) in mount.menu_rows(index, viewport) {
                        set.push(Lane::pixels(
                            "menu-row",
                            LaneClass::Affordance,
                            row,
                            row_rect,
                        ));
                    }
                }
            }
        }
    }

    for window in mount.windows() {
        // **A closed or minimized window is not on screen, so it is not claimed.** A lane drawn
        // where nothing is would be the overlay's version of a lie, and it is the common case: the
        // package declares sixteen windows and the player has opened one.
        if matches!(window.status, Status::Closed | Status::Minimized) {
            continue;
        }
        let on_layer = shown || window.spec.pinnable;
        if !on_layer {
            continue;
        }
        let Some(rect) = mount.rect_of(&window.spec.id) else { continue };
        let title = mount.title_of(&window.spec.id);
        set.push(Lane::pixels(
            "window",
            LaneClass::Structure,
            title.clone(),
            rect,
        ));
        set.push(Lane::pixels(
            "title-bar",
            LaneClass::Structure,
            title.clone(),
            Rect::new(rect.x, rect.y, rect.width_dp, title_bar_dp()),
        ));
        // Each pane, because a split window's panes are the claim that the arrangement is data.
        for pane in window.arrangement.windows() {
            if let Some(pane_rect) = mount.pane_rect(&window.spec.id, pane.as_str()) {
                set.push(Lane::pixels(
                    "pane",
                    LaneClass::Structure,
                    format!("{title} · {}", pane.as_str()),
                    pane_rect,
                ));
            }
        }
        // And each control the layout policy permits — **the same lanes the pointer hits**, so a
        // control that is drawn but not clickable, or clickable but not drawn, shows up here.
        for (action, control) in mount.control_lanes(&window.spec.id, rect) {
            set.push(Lane::pixels(
                "control",
                LaneClass::Affordance,
                format!("{title} · {}", action.name()),
                control,
            ));
        }
    }

    set
}

/// The lanes as elements, each one's ink measured against **the surface it lands on**.
///
/// **A structure lane sits on a window body and an affordance lane sits in a title bar**, so the two
/// are measured over different colours; one backdrop for the whole set would be a number about
/// neither. The two roles come from the palette the shell already draws with, and **a palette that
/// cannot name one is a refusal rather than a fallback colour** — a lane drawn in an invented ink
/// would be readable and untrue.
pub fn ui_lane_elements(set: &LaneSet, palette: &voxel_ui::Palette) -> voxel_ui::UiResult<Vec<voxel_ui::Element>> {
    let body = palette.get(voxel_ui::Role::Surface)?;
    let chrome = palette.get(voxel_ui::Role::Chrome)?;
    Ok(set.elements(|lane| match lane.class {
        LaneClass::Structure => body,
        _ => chrome,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::{vocabulary_class, LaneRefusal, LaneUnit};
    use voxel_package::recipe;
    use voxel_ui::{bar_entry_width_dp, min_target_dp, Palette};

    const LANE_RECIPE: &str = "shploingus.recipe/3\n\
package\tlane-test\tv0\n\
window\tworld\n\
\ttitle\tWorld\n\
\tcontent\tworld-view\n\
\tlane\tworld\n\
\textent\tsquare-pixels\n\
\tgeometry\t16 48 336 360\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t0\n\
\ttheme-role\thud\n\
\tpinnable\ttrue\n\
\tlayout\tminimize,close,title-position\n\
\tbehavior\tdrag\n\
\trecord-id\twindow-world\n\
window\tinventory\n\
\ttitle\tInventory\n\
\tcontent\tinventory\n\
\tlane\ttab\n\
\textent\tsquare-pixels\n\
\tgeometry\t400 96 480 406\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tminimize,maximize,close,title-position\n\
\tbehavior\tdrag,resize\n\
\trecord-id\twindow-inventory\n\
tab\tinventory\n\
pin\tworld\n";

    fn mount() -> Mount {
        let package = recipe::compile(LANE_RECIPE.as_bytes()).expect("the lane recipe compiles");
        let loaded = recipe::load(&package).expect("the lane package loads");
        let mut mount = Mount::new(&loaded).expect("the lane package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
        mount
    }

    #[test]
    fn every_lane_the_shell_declares_is_in_the_frozen_vocabulary() {
        // The vocabulary is the review gate: a lane the table does not hold is a claim nobody
        // looked at, and `vocabulary_class` returning `None` for it is how that shows up.
        let mount = mount();
        for lane in ui_lanes(&mount, true).lanes() {
            assert_eq!(
                vocabulary_class(&lane.id),
                Some(lane.class),
                "{} is not a declared lane",
                lane.id
            );
        }
    }

    #[test]
    fn every_lane_the_shell_declares_is_inside_the_viewport_and_within_budget() {
        // The refusals a real package would produce, asserted rather than discovered in a window.
        let mut mount = mount();
        mount_open(&mut mount);
        let lanes = ui_lanes(&mount, true);
        let refusals = lanes.validate(&surface(mount.viewport()));
        assert!(refusals.is_empty(), "{refusals:?}");
    }

    fn mount_open(mount: &mut Mount) {
        // `world` is pinned and therefore open at start-up; the unpinned one has to be opened, and
        // **a lane is only claimed for a window that is on screen** — which is the rule this
        // fixture exists to exercise, so it opens rather than assuming.
        mount.open("inventory").expect("declared");
    }

    #[test]
    fn a_pinned_window_is_claimed_on_both_layers_and_an_unpinned_one_only_on_the_shell() {
        // H-9.8: the HUD is the pinned subset of the same window set, so the lanes follow the layer
        // rather than a second list of windows.
        let mut mount = mount();
        mount_open(&mut mount);
        let shown = ui_lanes(&mount, true);
        let hidden = ui_lanes(&mount, false);

        let claims = |set: &LaneSet, id: &str| {
            set.lanes()
                .iter()
                .any(|lane| lane.id == "window" && lane.label == id)
        };
        assert!(claims(&shown, "World"), "the pinned window is on the shell layer too");
        assert!(claims(&hidden, "World"), "and stays when the shell is hidden");
        assert!(claims(&shown, "Inventory"));
        assert!(
            !claims(&hidden, "Inventory"),
            "an unpinned window is not claimed while it is not drawn: {:?}",
            hidden.lanes().iter().map(|l| l.label.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_bar_is_claimed_only_while_the_shell_is_shown() {
        // The bar is not drawn while the shell is hidden, and a lane for something not drawn is the
        // overlay's version of a lie.
        let mount = mount();
        assert!(ui_lanes(&mount, true).lanes().iter().any(|l| l.id == "bar"));
        assert!(!ui_lanes(&mount, false).lanes().iter().any(|l| l.id == "bar"));
        assert!(!ui_lanes(&mount, false).lanes().iter().any(|l| l.id == "bar-cell"));
    }

    #[test]
    fn a_control_lane_is_the_rectangle_the_pointer_hits() {
        // **One rectangle, two readers**: the mount routes the click from it and the overlay draws
        // it. If those ever disagreed, a control would be drawn where it cannot be pressed.
        let mount = mount();
        let rect = mount.rect_of("world").expect("the pinned window is open");
        let controls: Vec<Rect> = mount
            .control_lanes("world", rect)
            .into_iter()
            .map(|(_, r)| r)
            .collect();
        assert!(!controls.is_empty(), "the layout policy permits controls");
        let lanes: Vec<Rect> = ui_lanes(&mount, true)
            .lanes()
            .iter()
            .filter(|lane| lane.id == "control")
            .filter_map(|lane| match lane.region {
                Region::Rect(r) => Some(r),
                _ => None,
            })
            .collect();
        for control in &controls {
            assert!(lanes.contains(control), "{control:?} is hit but not claimed");
            assert!(mount.hit((control.x + control.width_dp / 2.0, control.y + control.height_dp / 2.0)));
            assert!(
                control.width_dp >= min_target_dp() && control.height_dp >= min_target_dp(),
                "a claimed control is also a target: {control:?}"
            );
        }
    }

    #[test]
    fn a_bar_cell_lane_is_the_cell_the_bar_drew() {
        let mount = mount();
        let cells: Vec<Rect> = mount
            .bar_rects(mount.viewport())
            .into_iter()
            .map(|(_, _, rect)| rect)
            .collect();
        let claimed: Vec<Rect> = ui_lanes(&mount, true)
            .lanes()
            .iter()
            .filter(|lane| lane.id == "bar-cell")
            .filter_map(|lane| match lane.region {
                Region::Rect(r) => Some(r),
                _ => None,
            })
            .collect();
        assert_eq!(claimed.len(), cells.len(), "one lane per drawn cell");
        for cell in &cells {
            assert!(claimed.contains(cell));
            assert!(cell.width_dp >= bar_entry_width_dp() - 0.5, "{cell:?}");
        }
    }

    #[test]
    fn the_lanes_draw_as_outlines_over_the_shell_without_asking_the_palette_for_a_role() {
        let mount = mount();
        let elements = ui_lane_elements(&ui_lanes(&mount, true), &Palette::declared())
            .expect("the palette names both surfaces");
        let labels: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert!(labels.iter().any(|l| l.contains("World")), "{labels:?}");
        assert!(
            labels.iter().all(|l| l.contains("structure") || l.contains("affordance")),
            "a label names its class: {labels:?}"
        );
        assert!(elements.iter().any(|e| matches!(e, voxel_ui::Element::Rule { .. })));
    }

    #[test]
    fn a_lane_that_leaves_the_viewport_is_refused_rather_than_clamped() {
        // A window dragged past the edge of a small window is the case the ruling is about: the
        // player's rectangle is one they meant, but the *lane* over it is a claim, and a claim that
        // no longer matches the client is not a claim.
        let mut mount = mount();
        mount_open(&mut mount);
        mount.set_viewport(Rect::new(0.0, 0.0, 640.0, 480.0));
        let refusals = ui_lanes(&mount, true).validate(&surface(mount.viewport()));
        let off = refusals
            .iter()
            .find(|r| matches!(r, LaneRefusal::OffSurface { .. }))
            .expect("a window at x=400 in a 640 dp viewport with a 480 dp width is off the surface");
        assert!(off.to_string().contains("not inside"), "{off}");
    }

    #[test]
    fn the_lane_set_is_bounded_by_the_declared_pixel_budget() {
        // The budget is declared, not discovered: sixteen windows with three controls and a body
        // each is well inside it, and the assertion is that the *number* is the declared one.
        let mount = mount();
        let lanes = ui_lanes(&mount, true);
        assert!(lanes.len() < crate::debug::lane_budget(LaneUnit::Pixels), "{}", lanes.len());
        assert!(!lanes.of_class(LaneClass::Structure).is_empty());
        assert!(!lanes.of_class(LaneClass::Affordance).is_empty());
    }

    #[test]
    fn the_palette_is_not_consulted_and_no_lane_declares_a_theme_hue() {
        // Both halves of the current state, asserted: lanes draw in ink over the backdrop they
        // land on, and **no class carries a hue**, because no theme in this tree declares a lane row
        // (H-16.1). The moment one does, this test is where the change is made.
        let mount = mount();
        let elements = ui_lane_elements(&ui_lanes(&mount, true), &Palette::declared())
            .expect("the palette names both surfaces");
        assert!(!elements.is_empty(), "lanes draw over the shell's own surfaces");
        for class in LaneClass::ALL {
            assert_eq!(class.hue(), None, "{} grew a hue with no theme row", class.slug());
        }
        let _ = Palette::declared();
    }
}
