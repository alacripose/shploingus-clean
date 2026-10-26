//! **The pinned layer: the windows that stay while the shell is hidden.**
//!
//! # What H-9.8 asks for, and what this module is
//!
//! UI-SPEC §1: *"The **HUD is the pinned subset** of the same system (H-9.8): a pinned window
//! stays in the user's view while the shell is hidden. So the shell and the HUD are two layers
//! of one window set, not two window systems."* §4 adds the mechanism: pinning is *"a **layer
//! move**, not a second rendering path: the same window, hosted by the HUD layer instead of the
//! shell layer."*
//!
//! So this file owns exactly three things, and none of them is a second window:
//!
//! 1. **The layer choice.** [`frame`] draws the shell layer (windows, bar, menu) when the shell
//!    is shown and the pinned layer (pinned windows only) when it is hidden. The windows are
//!    drawn by [`Mount`], always; this is a filter, not a painter.
//! 2. **The content views.** A pinned window hosts a panel — the four HUD panels the corner HUD
//!    used to draw, built from the same [`HudState`] and the same `voxel-ui` rules. The mount
//!    never builds a panel; the layer that owns the content supplies it.
//! 3. **The toast's order on screen.** A toast is the shell's transient surface, not a window
//!    (H-12.2, C-54.3), so it belongs to neither layer and [`frame`] appends it after whichever
//!    layer was chosen — the shell's message stays on screen with the shell up and down.
//!
//! # Why the four pinned windows replaced four corner panels
//!
//! The corner HUD was a second placement of one record: `world`, `streamer`, `determinism` and
//! `refusals` were four panels in four corners, and they are also four **declared** windows in
//! `alpha-shell.shspec` with declared geometry and pinned-ness. Drawing both would be the
//! failure §1 names — two window systems, one of which cannot be moved, themed or closed. The
//! panels survive; the corner arithmetic is gone, and the geometry now comes from the package.
//!
//! **The cost, stated:** the declared geometry is for a 1280 × 720 window with the 48 dp bar
//! (the shspec says so). At another size the windows keep their declarations rather than
//! reflowing, because *"a shell that resized them would be choosing an arrangement, and the
//! arrangement is a package"* (UI-SPEC §3).

use super::console::Console;
use super::mount::{ContentViews, Mount};
use crate::hud::{self, HudState};
use voxel_ui::{Element, Palette, Rect, UiResult};

/// The content views the pinned windows host, built from the frame's measurements.
///
/// **The four panels are the HUD's own**, taken from [`crate::hud`] rather than restated here:
/// a second panel builder would be a second set of ink steps and row heights, and the first
/// time they disagreed the shell and the HUD would report the same frame differently.
///
/// The **fifth is the console's**, and it is the only one not built from `HudState`: the
/// console is shell state, so it arrives as the [`Console`] the mount owns (BLD-0018). It is
/// passed in rather than read from the frame because a view is built before the frame is
/// drawn, and a console read at draw time would be a different console than the one written.
pub fn views(state: &HudState<'_>, console: &Console, api: Option<super::api::ApiView>) -> ContentViews {
    ContentViews {
        world: hud::world_panel(state),
        streamer: hud::streamer_panel(state),
        determinism: hud::determinism_panel(state),
        refusals: hud::refusal_panel(state),
        console: Some(console.clone()),
        // **The sixth is the API window's**, and it is the only one that is not a panel here: the
        // document and the matrix are produced by their own crates when a subject is
        // back-compiled, and this file composes them rather than restating them.
        api,
    }
}

/// What is on screen this frame: the shell layer when the shell is shown, the pinned layer when
/// it is hidden — **and the toast surface on top of whichever layer was chosen**.
///
/// The mount is borrowed mutably because laying out a layer is what records the refusals —
/// [`Mount::refusals`] is where a window that could not be drawn says so.
///
/// The viewport is passed through because a maximized window is sized against it; the pinned
/// layer ignores it for placement, because a pinned window sits where the package declared it.
///
/// **The toast is appended after the layer, and that is the third thing this file owns.** A
/// toast is the shell speaking rather than a window (H-12.2/C-54.3), so it belongs to neither
/// layer and must be legible over both: the layering here is the whole of "on screen in both
/// states", and [`Mount::toast_elements`] is the painter.
pub fn frame(
    mount: &mut Mount,
    palette: &Palette,
    viewport: Rect,
    views: &ContentViews,
) -> UiResult<Vec<Element>> {
    let mut out = if mount.shell_shown() {
        mount.shell_elements(palette, viewport, views)?
    } else {
        mount.pinned_elements(palette, viewport, views)?
    };
    mount.toast_elements(palette, viewport, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_package::recipe;
    use voxel_ui::{title_bar_dp, Panel, Readout, Role};

    /// A two-window package: one pinned, one not. The smallest thing that can tell the two
    /// layers apart.
    ///
    /// **The height is a parameter, not a guess.** `alpha-shell.shspec` declares each pinned
    /// window as *one 48 dp title bar plus the panel's own height* — §5.7's *"the two numbers
    /// must be the same number"* — and §5.7's other half is that the surface *"computes its
    /// height before it draws"*. So the caller measures the panel it hosts and states that
    /// geometry here, instead of a fixture hard-coding a rect the content guard would refuse.
    fn pin_recipe(height: usize) -> String {
        format!("shploingus.recipe/3\n\
package\tpin-test\tv0\n\
window\tworld\n\
\ttitle\tWorld\n\
\tcontent\tworld-view\n\
\tlane\tworld\n\
\textent\tsquare-pixels\n\
\tgeometry\t16 48 336 {height}\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t0\n\
\ttheme-role\thud\n\
\tpinnable\ttrue\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag\n\
\trecord-id\twindow-world\n\
window\trefusals\n\
\ttitle\tRefusals\n\
\tcontent\ttable\n\
\tlane\ttab\n\
\textent\tsquare-pixels\n\
\tgeometry\t80 96 480 {height}\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag\n\
\trecord-id\twindow-refusals\n\
pin\tworld\n"
        )
    }

    fn mount() -> Mount {
        // Measure the panel this fixture hosts, then declare the geometry from it.
        let height = (title_bar_dp() + views().world.height_dp()).ceil() as usize;
        let package = recipe::compile(pin_recipe(height).as_bytes()).expect("the pin recipe compiles");
        let loaded = recipe::load(&package).expect("the pin package loads");
        let mut mount = Mount::new(&loaded).expect("the pin package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
        mount
    }

    fn views() -> ContentViews {
        let panel = |title: &str| {
            Panel::new(
                "view",
                Role::Chrome,
                title,
                vec![Readout::measured("a", "1".to_string())],
            )
        };
        ContentViews {
            world: panel("World"),
            streamer: panel("Streamer"),
            determinism: panel("Determinism"),
            refusals: panel("Refusals"),
            console: None,
            api: None,
        }
    }

    fn titles(elements: &[Element]) -> Vec<&str> {
        elements.iter().filter_map(|element| element.text()).collect()
    }

    #[test]
    fn a_pinned_window_stays_in_the_view_while_the_shell_is_hidden() {
        // The row's meaning, exercised: hide the shell, and the pinned window is still drawn
        // while an open non-pinned window is not.
        let mut mount = mount();
        mount.open("refusals").expect("declared");
        assert!(mount.shell_shown());

        let viewport = mount.viewport();
        let shown = frame(&mut mount, &Palette::declared(), viewport, &views())
            .expect("the shell layer lays out");
        let shown_titles = titles(&shown);
        assert!(shown_titles.contains(&"World"), "{shown_titles:?}");
        assert!(shown_titles.contains(&"Refusals"), "{shown_titles:?}");

        mount.toggle_shell();
        assert!(!mount.shell_shown());
        let hidden = frame(&mut mount, &Palette::declared(), viewport, &views())
            .expect("the pinned layer lays out");
        let hidden_titles = titles(&hidden);
        assert!(hidden_titles.contains(&"World"), "the pinned window stays: {hidden_titles:?}");
        assert!(
            !hidden_titles.contains(&"Refusals"),
            "a window that is not pinned hides with the shell: {hidden_titles:?}"
        );
    }

    #[test]
    fn the_pinned_layer_draws_no_bar_and_no_menu() {
        let mut mount = mount();
        mount.toggle_shell();
        mount.pointer_pressed(super::super::mount::Button::Left, (10.0, 10.0));
        let viewport = mount.viewport();
        let elements = frame(&mut mount, &Palette::declared(), viewport, &views())
            .expect("the pinned layer lays out");
        // The bar's surface is a full-width strip at the top; the pinned layer draws none.
        let strip = elements.iter().any(|element| {
            matches!(
                element,
                Element::Surface { rect, .. } if rect.x == 0.0 && rect.y == 0.0 && rect.width_dp == mount.viewport().width_dp
            )
        });
        assert!(!strip, "the bar is the shell layer's");
    }

    #[test]
    fn a_toast_is_drawn_on_both_layers_and_after_the_one_it_covers() {
        // The toast is the shell speaking rather than a window (H-12.2/C-54.3): `frame` appends it
        // after whichever layer was chosen, so it is legible with the shell up and with it down —
        // and a message the player's own action produced cannot hide behind a window.
        let mut mount = mount();
        mount.announce(
            super::super::console::Level::Event,
            "world",
            "placed regolith at (1, 2, 3)",
            Some(0),
        );
        mount.set_tick(0);

        let viewport = mount.viewport();
        let shown = frame(&mut mount, &Palette::declared(), viewport, &views())
            .expect("the shell layer lays out");
        let shown_texts = titles(&shown);
        let toast_at = shown_texts
            .iter()
            .position(|text| *text == "placed regolith at (1, 2, 3)")
            .expect("the toast is on the shell layer");
        let window_at = shown_texts
            .iter()
            .position(|text| *text == "World")
            .expect("the pinned window's title is drawn");
        assert!(
            toast_at > window_at,
            "the toast is drawn after the windows it floats over: {shown_texts:?}"
        );

        mount.toggle_shell();
        let hidden = frame(&mut mount, &Palette::declared(), viewport, &views())
            .expect("the pinned layer lays out");
        assert!(
            titles(&hidden).contains(&"placed regolith at (1, 2, 3)"),
            "a toast shows with the shell down too, or the shell could go silent: {:?}",
            titles(&hidden)
        );
    }
}
