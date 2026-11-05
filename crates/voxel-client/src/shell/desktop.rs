//! **The desktop: the window list, focus, and z-order.**
//!
//! Reference: `docs/portal_simulator.py` — `PortalDesktop` 3475–3597: the window list and
//! `_focused` 3479–3483, `add_window` 3508–3514, `remove_window` 3516–3527, `focus_window`
//! 3529–3538, `toggle_maximize` 3540–3564, `minimize` 3566–3573, `restore` 3575–3585, the
//! background press 3588–3590 and `clear_focus` 3587–3593. Step 2 of `docs/SHELL-PORT.md`'s map.
//!
//! # The wallpaper is not ported, and the ruling is the user's
//!
//! The reference's `paintEvent` (3494–3507) fills the whole desktop with `theme.desktop`, draws
//! a dotted accent grid, and writes two literal strings (*"PORTAL"* / *"Qt interface simulator"*).
//! **None of it is ported.** The user's ruling, 2026-10-03: *"the desktop background over the
//! game would be dumb."* This tree's shell floats over the **rendered world** — that world is the
//! background, and a wallpaper drawn over it would be a second ground covering the first, in a
//! shell whose whole claim is that its surfaces are declared (UI-SPEC §1). So the desktop draws
//! **no ground at all**: no fill, no grid, no caption, and no `ExplorerView` icon list (D-3,
//! ruled with the same words).
//!
//! The consequence is carried where it belongs: `theme.rs`'s `desktop` field — the reference's
//! wallpaper colour — is answered by **the world**, not by a palette value, and `Role::Canvas`
//! stays unadmitted *by design* rather than by debt.
//!
//! # What the desktop owns, and what it deliberately does not
//!
//! It owns **the list, the z-order, and which one has focus** — the three things the reference
//! keeps on the desktop and none of which a window can answer about itself. It does not own the
//! windows' declarations (the package does), it does not draw a window ([`PortalWindow`] owns
//! its own chrome), and it does not lay out panes (`splitter.rs`). `PortalWindow` is a value
//! type with no registry; **this is where the registry lives**.
//!
//! # What changes, and why it is named
//!
//! * **`focus_window` (3529–3538) is one value, not three consequences.** The reference clears
//!   every other window's focus visual, calls `raise_()`, and refreshes the taskbar. Here focus
//!   is the id plus the place in the order: the per-window visuals follow from
//!   `WindowView::focused` at draw time (`window.rs`), and the bar's refresh follows from the
//!   same order at draw time. **The `set_focus_visual` loop has no equivalent** — a value cannot
//!   hold a stale visual.
//! * **`toggle_maximize` takes the desktop rect and the bar height** rather than reading
//!   `self.width()` / `self.portal.taskbar.height()` (3547–3553): the desktop does not know
//!   either, and the arithmetic is `window.rs`'s (one implementation, not two).
//! * **`remove_window`'s "focus the last window" (3524–3525) is stated as the rule it is.** The
//!   reference focuses `self.windows[-1]` — the last-*added* window. Here the list **is** the
//!   z-order, so the window that takes focus is the topmost one that is still drawn: the same
//!   window in the reference's steady state, and the right one after a minimize has removed
//!   something from the top of a list the reference does not reorder.
//! * **The background press clears focus** (3588–3590) and it is a verdict
//!   ([`DesktopPress`]): the reference does it inside its own `mousePressEvent`, so a press on
//!   the desktop is either a press on a window (the window's machine answers it) or a press on
//!   nothing (focus clears).
//!
//! # What this module is not
//!
//! It draws nothing. It does not own the bar or the toast, and it keeps no pixel arithmetic at
//! all. It is the **list and its rules**.

use std::collections::BTreeMap;

use crate::shell::window::PortalWindow;
use voxel_ui::{Rect, UiResult};

/// What a press on the desktop means: a window, or the background.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DesktopPress {
    /// A press on a drawn window — the window's own machine answers it (`window.rs::press`).
    Window(String),
    /// A press on nothing: **focus clears** (3588–3590).
    Background,
}

/// The desktop: the windows it holds, in z-order (back to front), and which one is focused.
///
/// **The order is the z-order and the list is the order.** The reference keeps `self.windows`
/// in creation order and calls `raise_()` to change stacking; two lists that must agree is the
/// class of defect this tree names (one name, two meanings). So there is one list, it is drawn
/// in order, and focus moves a window to its end.
#[derive(Default)]
pub struct Desktop {
    windows: BTreeMap<String, PortalWindow>,
    /// Back to front. The focused window is the last one that is drawn.
    order: Vec<String>,
    focused: Option<String>,
}

impl Desktop {
    /// A desktop with no windows — the reference's own starting state (3479–3483).
    pub fn new() -> Self {
        Desktop {
            windows: BTreeMap::new(),
            order: Vec::new(),
            focused: None,
        }
    }

    /// The windows, back to front — the order the caller draws them in.
    pub fn order(&self) -> &[String] {
        &self.order
    }

    /// The window with this id, if the desktop holds one.
    pub fn get(&self, id: &str) -> Option<&PortalWindow> {
        self.windows.get(id)
    }

    /// The window with this id, mutably — the arrangement mutations go through here.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PortalWindow> {
        self.windows.get_mut(id)
    }

    /// Whether the desktop holds a window with this id.
    pub fn holds(&self, id: &str) -> bool {
        self.windows.contains_key(id)
    }

    /// Which window has focus (the reference's `_focused`, 3481).
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// `focus_window` (3529–3538): the focused window is the last drawn one.
    ///
    /// **A window the desktop does not hold is not focused** (3530–3531), and that is a no-op
    /// rather than a refusal: focus is a request, and a request naming a window that has closed
    /// is answered by doing nothing.
    pub fn focus(&mut self, id: &str) {
        if !self.windows.contains_key(id) {
            return;
        }
        self.order.retain(|held| held != id);
        self.order.push(id.to_string());
        self.focused = Some(id.to_string());
    }

    /// `clear_focus` (3587–3593): nothing is focused. A background press reaches here.
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// `add_window` (3508–3514): the window joins the list, is raised, and is focused.
    ///
    /// **The reference's `window.setParent(self)` is the caller's** — a window here is a value,
    /// not a widget — and what the desktop owns is membership and order. Adding a window whose
    /// id it already holds **replaces** it and raises the new one: the reference's `add_window`
    /// is only ever called for a fresh window, and an id is stable identity (UI-SPEC §2), so
    /// two windows cannot share one.
    pub fn add(&mut self, window: PortalWindow) {
        let id = window.spec.id.clone();
        self.windows.insert(id.clone(), window);
        self.order.retain(|held| held != &id);
        self.order.push(id.clone());
        self.focused = Some(id);
    }

    /// `remove_window` (3516–3527): the window leaves, and **the topmost window still drawn
    /// takes focus** when the removed one had it.
    ///
    /// The reference focuses `self.windows[-1]`; the rule stated here is the same rule with the
    /// order made explicit, so a minimized top window does not silently swallow focus.
    pub fn remove(&mut self, id: &str) -> Option<PortalWindow> {
        let window = self.windows.remove(id)?;
        self.order.retain(|held| held != id);
        if self.focused.as_deref() == Some(id) {
            self.focused = self
                .order
                .iter()
                .rev()
                .find(|held| self.is_drawn(held))
                .cloned();
        }
        Some(window)
    }

    /// Whether a window in this desktop is drawn at all this frame (a minimized or closed one
    /// is not — the same predicate `window.rs` carries).
    pub fn is_drawn(&self, id: &str) -> bool {
        self.windows.get(id).is_some_and(PortalWindow::is_drawn)
    }

    /// `toggle_maximize` (3540–3564): the window's own arithmetic runs, and **the maximized or
    /// restored window is focused** (3563).
    pub fn toggle_maximize(&mut self, id: &str, desktop: Rect, taskbar: f64) -> UiResult<()> {
        let Some(window) = self.windows.get_mut(id) else {
            return Ok(());
        };
        window.toggle_maximize(desktop, taskbar)?;
        self.focus(id);
        Ok(())
    }

    /// `minimize` (3566–3573): the window stops being drawn and **loses focus if it had it**.
    pub fn minimize(&mut self, id: &str) {
        if let Some(window) = self.windows.get_mut(id) {
            window.minimize();
        }
        if self.focused.as_deref() == Some(id) {
            self.focused = None;
        }
    }

    /// `restore` (3575–3585): the window is drawn again and is focused.
    pub fn restore(&mut self, id: &str) {
        if let Some(window) = self.windows.get_mut(id) {
            window.restore();
        }
        self.focus(id);
    }

    /// The topmost **drawn** window whose rect holds the point — the z-order read front to
    /// back, which is the order a press resolves in.
    pub fn window_at(&self, at: (f64, f64)) -> Option<&str> {
        self.order
            .iter()
            .rev()
            .find(|id| {
                self.is_drawn(id) && self.windows.get(*id).is_some_and(|w| w.rect.contains(at))
            })
            .map(String::as_str)
    }

    /// `mousePressEvent` (3588–3590) as a verdict: a press on a window, or on the background.
    ///
    /// The reference clears focus on **any** left press and lets the widget tree deliver the
    /// press afterwards; the window's own handler then focuses itself (3321–3323). Folding both
    /// into one verdict keeps that behaviour deliberate: the caller hands [`DesktopPress::Window`]
    /// to the window's machine, and a [`DesktopPress::Background`] clears focus.
    pub fn press(&self, at: (f64, f64)) -> DesktopPress {
        match self.window_at(at) {
            Some(id) => DesktopPress::Window(id.to_string()),
            None => DesktopPress::Background,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::arrangement::{Orientation, Side, WindowId};
    use crate::shell::mount::Status;
    use voxel_package::recipe::{
        BehaviorPolicy, Content, ExtentUnit, Lane, LayoutPolicy, WindowSpec,
    };

    fn spec(id: &str) -> WindowSpec {
        WindowSpec {
            id: id.to_string(),
            title: format!("window {id}"),
            content: Content::ALL[0],
            lane: Lane::ALL[0],
            extent_unit: ExtentUnit::ALL[0],
            geometry: [100, 80, 640, 406],
            min_size: [260, 160],
            opacity: 1000,
            elevation: 0,
            theme_role: "surface".to_string(),
            pinnable: false,
            layout_policy: LayoutPolicy::TITLE_POSITION,
            behavior_policy: BehaviorPolicy::DRAG,
            record_id: "C-1".to_string(),
        }
    }

    fn window(id: &str) -> PortalWindow {
        PortalWindow::new(spec(id), Rect::new(100.0, 80.0, 640.0, 406.0)).expect("a window")
    }

    fn desktop_with(ids: &[&str]) -> Desktop {
        let mut desktop = Desktop::new();
        for id in ids {
            desktop.add(window(id));
        }
        desktop
    }

    #[test]
    fn adding_a_window_focuses_it_and_puts_it_on_top() {
        let mut desktop = desktop_with(&["a", "b"]);
        assert_eq!(desktop.order(), &["a", "b"]);
        assert_eq!(desktop.focused(), Some("b"));
        desktop.focus("a");
        assert_eq!(desktop.order(), &["b", "a"], "focus raises");
        assert_eq!(desktop.focused(), Some("a"));
    }

    #[test]
    fn focusing_a_window_the_desktop_does_not_hold_is_a_no_op() {
        let mut desktop = desktop_with(&["a"]);
        desktop.focus("zz");
        assert_eq!(desktop.focused(), Some("a"));
        assert_eq!(desktop.order(), &["a"]);
    }

    #[test]
    fn removing_the_focused_window_focuses_the_topmost_one_still_drawn() {
        let mut desktop = desktop_with(&["a", "b", "c"]);
        desktop.minimize("c");
        assert_eq!(
            desktop.focused(),
            None,
            "minimizing the focused window clears focus"
        );
        desktop.focus("b");
        let removed = desktop.remove("b").expect("the desktop held b");
        assert_eq!(removed.spec.id, "b");
        assert_eq!(
            desktop.focused(),
            Some("a"),
            "c is minimized, so a is the top drawn"
        );
        assert_eq!(desktop.order(), &["a", "c"]);
    }

    #[test]
    fn removing_the_last_window_leaves_nothing_focused() {
        let mut desktop = desktop_with(&["a"]);
        desktop.remove("a");
        assert_eq!(desktop.focused(), None);
        assert!(desktop.order().is_empty());
    }

    #[test]
    fn minimize_clears_focus_and_restore_returns_it() {
        let mut desktop = desktop_with(&["a", "b"]);
        desktop.minimize("b");
        assert_eq!(desktop.focused(), None);
        assert!(!desktop.is_drawn("b"));
        desktop.restore("b");
        assert_eq!(desktop.focused(), Some("b"));
        assert!(desktop.is_drawn("b"));
    }

    #[test]
    fn toggle_maximize_runs_the_windows_own_arithmetic_and_focuses_it() {
        let mut desktop = desktop_with(&["a", "b"]);
        let desktop_rect = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        desktop
            .toggle_maximize("a", desktop_rect, 48.0)
            .expect("maximizes");
        let window = desktop.get("a").expect("a is held");
        assert_eq!(window.status, Status::Maximized);
        assert_eq!(desktop.focused(), Some("a"));
        desktop
            .toggle_maximize("a", desktop_rect, 48.0)
            .expect("restores");
        assert_eq!(desktop.get("a").expect("a is held").status, Status::Normal);
    }

    #[test]
    fn a_press_resolves_the_topmost_window_and_the_background_clears_focus() {
        let mut desktop = desktop_with(&["a", "b"]);
        // Both windows sit at the same rect; the topmost (b) answers.
        assert_eq!(
            desktop.press((200.0, 200.0)),
            DesktopPress::Window("b".to_string())
        );
        assert_eq!(desktop.press((2000.0, 2000.0)), DesktopPress::Background);
        desktop.clear_focus();
        assert_eq!(desktop.focused(), None);
        // A minimized window does not answer a press.
        desktop.minimize("b");
        assert_eq!(
            desktop.press((200.0, 200.0)),
            DesktopPress::Window("a".to_string())
        );
    }

    #[test]
    fn the_desktop_draws_no_ground_at_all() {
        // The user's ruling, as a test: no wallpaper, no grid, no caption. The world behind the
        // shell is the background, and this module has no drawing surface to cover it with —
        // which is why there is nothing here to assert beyond the list's own behaviour.
        let desktop = desktop_with(&["a"]);
        assert_eq!(desktop.order().len(), 1);
    }

    #[test]
    fn a_docked_window_keeps_its_own_desktop_entry() {
        // Membership is the desktop's; the arrangement is the window's. A window docked into
        // another keeps its own entry — the reference reaches the same state by re-parenting
        // the widget while the window object survives.
        let mut desktop = desktop_with(&["host", "guest"]);
        let guest = WindowId::new("guest").expect("id");
        desktop
            .get_mut("host")
            .expect("host is held")
            .split(None, Orientation::Horizontal, Side::After, guest)
            .expect("splits");
        assert!(desktop.holds("guest"));
        assert_eq!(desktop.order().len(), 2);
    }
}
