//! **The taskbar's running entries and its clock — under D-2, on the upper bar.**
//!
//! Reference: `docs/portal_simulator.py` — `PortalTaskbar` 3598–3729: the construction
//! 3599–3636 (48 tall, the Start button, the running list, the clock at 72 minimum),
//! `update_clock` 3657–3662, `refresh` 3663–3690 (the per-window label and its checked state),
//! `task_clicked` 3691–3698 (minimize when focused, restore when minimized, focus otherwise).
//! Step 2 of `docs/SHELL-PORT.md`'s map.
//!
//! # D-2, and what it means for this module
//!
//! The ruling of 2026-10-03: **one surface, top, and the clock kept.** The upper bar carries the
//! declared tab set *and* the running-window entries; there is no bottom strip; the reference's
//! clock (`update_clock`, 3657–3662) is kept in the bar; its Start menu and auto-hide opacity
//! are not ported (the launcher is the package's windows plus the bar).
//!
//! So this module **does not draw a taskbar**. It ports the taskbar's *data*: the running
//! entries — one per window, labelled from the pane set exactly as `refresh` labels them — and
//! the clock's text. The caller (the bar's drawing, `mount.rs` today and `portal.rs` at step 3)
//! places them on the one surface that exists.
//!
//! # What changes, and why it is named
//!
//! * **The entry label is the reference's own derivation** (3663–3684): one pane shows its
//!   window's title; several show the pane titles joined with the reference's ` | `; a window
//!   with no pane falls back to its own title. The tree's one-kind-of-window rule (H-9.7) makes
//!   the reference's *app or `◫ Split`* branch the window's title instead, because a pane here
//!   always holds a window.
//! * **The checked state is the reference's predicate** (3685): a window is checked when it is
//!   focused **and not minimized**. That is a *value* here — [`Running::checked`] — and the
//!   drawing decides what a checked entry looks like (the bar's underline rule today).
//! * **`task_clicked` (3691–3698) is one verdict**: minimized → restore, focused → minimize,
//!   otherwise → focus. The reference reaches the same three cases through the desktop's
//!   methods; the verdict keeps the choice in one place so the bar's press handler is one
//!   match.
//! * **The clock's format is the reference's own setting** (`clock_24hr`, `show_date`,
//!   3657–3662), and here it is a **value the caller supplies** ([`Clock::text`] takes the
//!   formatted string): `voxel-client` has no wall-clock dependency, and the bin that owns the
//!   event loop is the only place that may read the system time. What this module owns is the
//!   *format rule* — [`ClockFormat`] — so the 24-hour and date choices are declared rather than
//!   improvised at the call site.

use crate::shell::mount::{BarEntry, Status};
use crate::shell::window::PortalWindow;

/// One running entry on the bar: which window it names, what it says, and whether it is
/// checked (the reference's `refresh`, 3663–3690).
#[derive(Clone, Debug, PartialEq)]
pub struct Running {
    pub id: String,
    pub label: String,
    /// Focused **and not minimized** — the reference's `b.setChecked(...)` predicate (3685).
    pub checked: bool,
}

/// `task_clicked` (3691–3698) as a verdict: what a press on a running entry means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskPress {
    /// The window is minimized: bring it back (3575–3585).
    Restore,
    /// The window is focused: minimize it (3566–3573).
    Minimize,
    /// Otherwise: focus it (3529–3538).
    Focus,
}

/// The verdict for a press on a running entry, from the window's state and whether it is the
/// focused one — the reference's three cases in its own order.
pub fn task_press(window: &PortalWindow, focused: bool) -> TaskPress {
    if window.status == Status::Minimized {
        TaskPress::Restore
    } else if focused {
        TaskPress::Minimize
    } else {
        TaskPress::Focus
    }
}

/// The reference's label derivation (3674–3684), from the pane set's titles.
///
/// One pane shows its title; several join them with the reference's own separator; **an empty
/// pane set falls back to the window's title**, which is the tree's one-kind-of-window answer
/// to the reference's `else: txt = "Window"` — a window here always has a name.
pub fn label(pane_titles: &[String], window_title: &str) -> String {
    match pane_titles.len() {
        0 => window_title.to_string(),
        1 => pane_titles[0].clone(),
        _ => pane_titles.join(" | "),
    }
}

/// `refresh` (3663–3690) as data: one entry per window, in the desktop's order, each labelled
/// from its pane set and checked by the reference's predicate.
pub fn running<'a>(
    windows: impl IntoIterator<Item = &'a PortalWindow>,
    focused: Option<&str>,
    titles_of: impl Fn(&str) -> Vec<String>,
) -> Vec<Running> {
    windows
        .into_iter()
        .map(|window| {
            let id = window.spec.id.as_str();
            Running {
                id: id.to_string(),
                label: label(&titles_of(id), &window.spec.title),
                checked: focused == Some(id) && window.status != Status::Minimized,
            }
        })
        .collect()
}

/// The clock's format, from the reference's two settings (`clock_24hr`, `show_date` —
/// 3657–3662). **The format is declared here and the time is supplied by the caller**: this
/// crate has no wall-clock dependency, and the bin that owns the event loop is the only place
/// that may read the system clock.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ClockFormat {
    /// 24-hour when set (the reference's default, 3658); 12-hour otherwise.
    pub twenty_four_hour: bool,
    /// The date line above the time, when set (the reference's `show_date`, 3659–3661).
    pub show_date: bool,
}

impl ClockFormat {
    /// The reference's own defaults: `clock_24hr: True`, `show_date: False` (3741–3745).
    pub const REFERENCE: Self = Self { twenty_four_hour: true, show_date: false };

    /// How many lines the clock draws — the reference's `MM/dd/yyyy\nHH:mm` is two (3661).
    pub fn lines(self) -> usize {
        if self.show_date {
            2
        } else {
            1
        }
    }
}

/// The clock's text, from the caller's `time` and `date` strings — the caller formats, this
/// states **which of the reference's two shapes** the format means, so the choice is one place.
///
/// `time` is the caller's `HH:mm` or `h:mm AM/PM`; `date` its `MM/dd/yyyy`. A format that shows
/// the date with no date string draws the time alone rather than a blank line, because a blank
/// line in a clock is a clock that looks broken.
pub fn clock_text(format: ClockFormat, time: &str, date: Option<&str>) -> String {
    match (format.show_date, date) {
        (true, Some(date)) if !date.trim().is_empty() => format!("{date}\n{time}"),
        _ => time.to_string(),
    }
}

/// The bar entries a running list produces, in the reference's own form: the entries the bar
/// draws beside its declared tabs (D-2's *"the upper bar carries the tab set and the running
/// entries"*).
///
/// **A running entry is not a tab** (`tab: false`): a tab sorts declared windows and opens a
/// menu; a running entry names a window that is open, and its press is [`task_press`]. That is
/// the same distinction the recipe's `bar` rows carry (H-15.2).
pub fn entries(running: &[Running]) -> Vec<BarEntry> {
    running
        .iter()
        .map(|entry| BarEntry { id: entry.id.clone(), tab: false, label: entry.label.clone() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_package::recipe::{
        BehaviorPolicy, Content, ExtentUnit, Lane, LayoutPolicy, WindowSpec,
    };
    use voxel_ui::Rect;

    fn spec(id: &str) -> WindowSpec {
        WindowSpec {
            id: id.to_string(),
            title: format!("window {id}"),
            content: Content::ALL[0],
            lane: Lane::ALL[0],
            extent_unit: ExtentUnit::ALL[0],
            geometry: [0, 0, 640, 406],
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
        PortalWindow::new(spec(id), Rect::new(0.0, 0.0, 640.0, 406.0)).expect("a window")
    }

    #[test]
    fn a_label_is_the_reference_s_own_derivation() {
        assert_eq!(label(&[], "window a"), "window a");
        assert_eq!(label(&["one".to_string()], "window a"), "one");
        assert_eq!(
            label(&["one".to_string(), "two".to_string()], "window a"),
            "one | two",
            "the reference's own separator"
        );
    }

    #[test]
    fn an_entry_is_checked_only_when_focused_and_not_minimized() {
        let a = window("a");
        let b = window("b");
        let mut b_min = window("b");
        b_min.minimize();
        let titles = |id: &str| vec![format!("title {id}")];
        let list = running([&a, &b, &b_min], Some("b"), titles);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].label, "title a");
        assert!(!list[0].checked);
        assert!(list[1].checked, "focused and normal is checked");
        assert!(!list[2].checked, "focused but minimized is not checked");
    }

    #[test]
    fn a_press_names_the_reference_s_three_cases_in_order() {
        let normal = window("a");
        assert_eq!(task_press(&normal, true), TaskPress::Minimize);
        assert_eq!(task_press(&normal, false), TaskPress::Focus);
        let mut minimized = window("b");
        minimized.minimize();
        // A minimized window restores whether or not it is the focused one — the reference
        // tests the state first (3692).
        assert_eq!(task_press(&minimized, true), TaskPress::Restore);
        assert_eq!(task_press(&minimized, false), TaskPress::Restore);
    }

    #[test]
    fn the_clock_s_format_is_the_reference_s_two_settings() {
        assert_eq!(ClockFormat::default(), ClockFormat { twenty_four_hour: false, show_date: false });
        let reference = ClockFormat::REFERENCE;
        assert!(reference.twenty_four_hour);
        assert!(!reference.show_date);
        assert_eq!(reference.lines(), 1);
        let dated = ClockFormat { twenty_four_hour: true, show_date: true };
        assert_eq!(dated.lines(), 2);
        assert_eq!(clock_text(reference, "09:41", Some("10/03/2026")), "09:41");
        assert_eq!(clock_text(dated, "09:41", Some("10/03/2026")), "10/03/2026\n09:41");
        // A date line with no date string draws the time alone rather than a blank line.
        assert_eq!(clock_text(dated, "09:41", None), "09:41");
        assert_eq!(clock_text(dated, "09:41", Some("  ")), "09:41");
    }

    #[test]
    fn a_running_entry_is_a_bar_entry_that_is_not_a_tab() {
        let a = window("a");
        let list = running([&a], Some("a"), |id| vec![id.to_string()]);
        let entries = entries(&list);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].tab, "a running entry names a window; a tab sorts declared ones");
        assert_eq!(entries[0].label, "a");
    }
}
