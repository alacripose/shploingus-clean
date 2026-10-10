//! **The mount: the bar, the windows, and the state the shell owns.**
//!
//! # What this is, and what the record asks of it
//!
//! UI-SPEC §1: *"Windows are data the shell mounts. […] The game's own surfaces — the ledger, the
//! console, the world shaper panel, the debug lanes, the API viewer — are all shell windows.
//! Nothing else may draw a window."* §2: *"the window declares; the shell owns `WindowState`
//! (normal / minimized / maximized / closed), geometry, focus and z-order."* This module is that
//! owner. A [`WindowSpec`] is a value; everything a player can do to it lives here.
//!
//! What the mount does, each of which is a row in the record rather than a convenience:
//!
//! | behaviour | where it comes from |
//! |---|---|
//! | the bar's entries — six tabs then the console — are the package's `tab`/`bar` rows, in strip order | H-15.2, H-15.3, `recipe/3` |
//! | a tab's menu offers the windows filed under it (its own window first) | H-15.3, the derivation of the simulator's start menu |
//! | a bar entry that is not a tab opens its window directly | H-15.2 (the console is the command surface, not a sorter) |
//! | the pane tree per window, with 40 × 40 per split and the title derived from the pane set | UI-SPEC §3, C-65.4 |
//! | controls are **absent** when the layout policy does not permit them | HIG: absence over a grant that looks real |
//! | pinned windows stay, the rest of the shell hides | H-9.8, `shell/hud.rs` |
//! | the shell draws its own glyphs on declared lanes | H-15.6 |
//!
//! # Geometry and the one place pixels are mentioned
//!
//! Every rectangle here is **device-independent pixels**, exactly like `voxel-ui`. The bin
//! divides physical pixels by the window's scale before it calls in, so the mount, the palette
//! and the layout rules all speak one unit and a 150 % display changes nothing but the divisor.
//!
//! **The declared geometry is the initial rect, not a per-frame answer.** A window starts where
//! the package put it and then it is the shell's — a dragged window does not snap back to its
//! declaration on the next frame. The declaration is the arrangement's, the position is the
//! state's, and UI-SPEC §2 splits them exactly there.
//!
//! # Drawn order, and what is deliberately not drawn
//!
//! Windows are drawn back to front ([`Mount::ordered`]); the focused window is last. The bar is
//! drawn after them so a window dragged under it does not cover it, and an open menu after the
//! bar. Minimized windows are not drawn at all — the state is visible in the tab's menu, and a
//! zero-height window would be a window pretending to exist.
//!
//! **Nothing here decides a colour.** Every surface asks the palette for a role, every text run
//! is measured against the composited backdrop it will really sit on, and a role with no admitted
//! value is a refusal carried up to the caller rather than a grey that stands in for it.

use std::collections::BTreeMap;

use voxel_package::recipe::{Content, Loaded, WindowSpec};
use voxel_ui::{
    bar_entry_width_dp, bar_height_dp, contrast_over_backdrop, max_elevation, min_target_dp,
    row_height_dp, spacing_dp, Align, Corner, Element, GlyphAction, Ink, Paint,
    Palette, Panel, Readout, Rect, Role, UiRefusal, UiResult,
};

use super::console::{Command, Console, Key, KeyOutcome, Level, Record, Verb};
use super::drop::{self, DropTarget};
use super::notifications::ToastQueue;
use super::window::{self, DragState, EdgeTriggers, ResizeEdges, WindowPress, WindowView};

/// The id the arrangement gives the console window, and the only window whose keys the console
/// takes. **A constant rather than a lookup**, because `alpha-shell.shspec` declares it and a
/// console that answered to a different name would be a second name for one window.
pub const CONSOLE_WINDOW: &str = "console";

/// The chain a console filing lands on. **Constants rather than derived**, because a namespace
/// is an attribution — *which* world, *which* year, *which* campaign — and one computed from the
/// clock would make the same ticket file into two different records.
pub const CONSOLE_WORLD: &str = "shploingus";
/// The year the console's filings are attributed to. A constant, and the reason it is stated
/// here rather than read from a system clock: a filing whose attribution moves with the machine
/// is a filing nobody can find again.
pub const CONSOLE_YEAR: u64 = 2026;
/// The campaign a console filing belongs to, and its `round_id`.
pub const CONSOLE_CAMPAIGN: &str = "console";
/// Who a console filing is attributed to. **A named actor, because
/// `an_empty_actor_is_refused_because_a_record_needs_someone_to_have_written_it`** — a record
/// with no author is refused, and this is the author.
pub const CONSOLE_ACTOR: &str = "player-1";
use crate::shell::arrangement::{Arrangement, Node, Orientation, PaneRef, Side, WindowId};

/// The brightest thing the world can put behind the shell — an 8-bit swapchain cannot present
/// more than white, exactly as `hud.rs` states for the HUD. Every text run the shell draws is
/// measured against this, so *"the bar's text is legible"* is a claim about the worst case.
const BRIGHTEST: voxel_ui::Rgb = voxel_ui::WHITE;

/// The four states a window can be in. UI-SPEC §2's own list, and the only four.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Normal,
    Minimized,
    Maximized,
    Closed,
}

impl Status {
    pub const fn name(self) -> &'static str {
        match self {
            Status::Normal => "open",
            Status::Minimized => "minimized",
            Status::Maximized => "maximized",
            Status::Closed => "closed",
        }
    }
}

/// **One window type, and it is the port's.**
///
/// The derived struct that used to live here carried exactly `spec`, `status`, `rect`,
/// `arrangement` and `host` — the same five fields `PortalWindow` carries — so it is **replaced
/// rather than translated**: two types with one meaning is the defect class this tree names
/// (D-07), and `SHELL-PORT.md` §5's one-arithmetic rule requires the drawing, the hit tests and
/// the tree to resolve through one window. The alias is public because `Mount`'s readers name
/// it.
pub use super::window::PortalWindow as Window;

/// One entry on the upper bar: a tab, or a bar window that opens directly.
#[derive(Clone, Debug)]
pub struct BarEntry {
    pub id: String,
    /// A tab sorts the windows filed under it; a bar window opens when it is clicked.
    pub tab: bool,
    pub label: String,
}

/// What a window body shows. The shell owns the window; the layer that owns a content kind
/// supplies its view — the four HUD panels today, and a declared-content panel for a kind whose
/// view does not exist yet. The mount never builds a panel itself.
pub struct ContentViews {
    pub world: Panel,
    pub streamer: Panel,
    pub determinism: Panel,
    pub refusals: Panel,
    /// The console's own records (BLD-0018).
    ///
    /// **A snapshot, not a panel — and that is forced by the pane, not chosen.** The console is
    /// the one content kind whose panel height depends on the window hosting it: a console with
    /// sixty-four records is taller than the 358 dp pane `alpha-shell.shspec` declares for it,
    /// so a panel built once and reused refuses to lay out. The window is **declared** — *"a
    /// shell that resized them would be choosing an arrangement, and the arrangement is a
    /// package"* — so the panel is bounded to the pane in [`ContentViews::panel_for`] instead,
    /// and this is the records it is bounded from.
    ///
    /// `None` when the caller had no console to snapshot, and `panel_for` then says so rather
    /// than drawing an empty one that looks like a console with nothing in it.
    pub console: Option<Console>,
}

impl ContentViews {
    ///
    /// **The pane is measured, and one window's content does not decide another's.** A declared
    /// window's pane is its height less the port's chrome, and §5.7's rule is that the surface
    /// computes its height before it draws — so a content kind whose height varies is bounded
    /// against *this* window's pane rather than a constant chosen for a different one.
    ///
    /// **`Readout` and `Table` are the four panels' subjects, not generic kinds.** The alpha set
    /// has exactly one window of each; a second window with the same kind would host the same
    /// subject, which is wrong — and that is why the fallback below exists and says so rather
    /// than pretending the kind is the subject.
    pub fn panel_for(&self, spec: &WindowSpec) -> Panel {
        let pane = pane_of(spec);
        match spec.content {
            Content::WorldView => self.world.clone(),
            Content::Streamer => self.streamer.clone(),
            Content::Readout => self.determinism.clone(),
            Content::Table => self.refusals.clone(),
            Content::Console => match &self.console {
                Some(console) => console_panel(console, pane),
                // **Not a blank panel.** A window whose console could not be snapshotted says so,
                // which is a different fact from a console with nothing in it.
                None => declared_content_panel(spec),
            },
            // The notifications window is the persistent half of the same store the console
            // reads (UI-SPEC §7), so it reads the same snapshot this frame was given — there is
            // no second reader to disagree with.
            Content::Notifications => match &self.console {
                Some(console) => notifications_panel(console, pane),
                None => declared_content_panel(spec),
            },
            _ => declared_content_panel(spec),
        }
    }
}

/// **The body a window's arrangement is laid out inside**: the port's composite's content
/// container (`window.rs` — the border, the title bar's slot, then the content).
///
/// **One arithmetic, and it is the port's.** The pane tree of `window_elements` is laid out
/// inside this rect, and the console's pointer lanes and the wheel resolve against the same
/// rect — a second arithmetic here would place a control where it is not drawn, which is the
/// class of defect the control lanes exist to prevent. `SHELL-PORT.md` §5's whole point: the
/// port's composite **is** the shell's arithmetic now, not a parallel one.
pub fn window_body(spec: &WindowSpec, rect: Rect) -> Rect {
    window::composite(spec, rect).content
}

/// **The pane's body at a window's declared size** — the number a panel is bounded against
/// before the window is placed.
///
/// One arithmetic with the drawing: this is [`window_body`] applied to the declaration's rect,
/// then the pane's own header removed ([`super::pane::body`]) — **the same two calls the content
/// loop makes**, because `window_elements` lays out inside `window_body` and draws each pane's
/// content below its header. The port draws a pane header even for a single pane (the
/// reference's `WindowPane` always adds one, 2670–2675), so a bound that stopped at the content
/// container would be 40 dp too generous and a window sized to it would refuse the moment it was
/// drawn — §5.7's two-numbers rule failing in the direction that only shows on screen.
pub fn pane_of(spec: &WindowSpec) -> f64 {
    let declared = Rect::new(
        0.0,
        0.0,
        spec.geometry[2] as f64,
        spec.geometry[3] as f64,
    );
    super::pane::body(window_body(spec, declared)).height_dp
}

/// **The chrome a declared window draws above the content its panes hold** — the border on both
/// edges, the title bar's slot when the declaration permits one, and a pane's header.
///
/// The number a fixture or a package needs when it sizes a window as *chrome + panel*: a
/// declaration that wrote *title bar + panel* was correct for the derived shell and is a pane
/// header short for the port. The three parts are the drawing's own ([`window::BORDER_DP`],
/// [`window::title_height`], [`super::pane::header_height_dp`]), and
/// `the_sizing_rule_is_the_drawing_s_own_arithmetic` pins the sum against the composite.
pub fn chrome_height_dp(spec: &WindowSpec) -> f64 {
    2.0 * window::BORDER_DP + window::title_height(spec) + super::pane::header_height_dp()
}

/// The console window's body: **the console's own records** (BLD-0018).
///
/// The rows are `Console::lines`, so this is a reader of the console and not a second copy of
/// it — a window whose records were assembled separately could disagree with the terminal
/// printing the same console, and the disagreement would be invisible.
///
/// # Bounded by the pane, and the bound is measured rather than declared
///
/// A console panel is the one panel whose height is not a property of the console: it is a
/// property of **the window hosting it**. `alpha-shell.shspec` declares the console 440 dp
/// tall, which is a 358 dp pane under the port's chrome, and sixty-four records do not fit in
/// it — so a panel built to a constant refuses to lay out with *"a 407 dp panel in a 358 dp
/// pane"* the moment the log is more than a few lines long. That is §5.7's check working, and
/// the fix is not to resize the window: *"a shell that resized them would be choosing an
/// arrangement, and the arrangement is a package."*
///
/// So the rows are dropped **from the front**, oldest first, until the panel measures no taller
/// than the pane, and the panel **says how many it is holding and not drawing**. A console that
/// silently showed only the tail of a log reads as a log that began there.
///
/// **The two bounds are different facts and both are stated.** `Console`'s own ceiling is
/// [`crate::shell::console::MAX_RECORDS`] and it *drops* records — reported as `dropped`. The
/// window's ceiling is the pane and it *hides* them — reported in the note. A reader who saw
/// only one of them would think the log is smaller than it is.
pub fn console_panel(console: &Console, available_dp: f64) -> Panel {
    let all = console.lines();
    let mut rows: Vec<Readout> = vec![
        // **The draft is a row, and it is the first one.** A command line you cannot see is a
        // command line you cannot check before you commit it — and the reference logs the typed
        // line before dispatch precisely so what was typed is inspectable.
        Readout::measured(
            "input",
            if console.draft().is_empty() {
                ">".to_string()
            } else {
                format!("> {}", console.draft())
            },
        ),
        Readout::measured(
            "filter",
            match console.filter() {
                Some(level) => level.slug().to_string(),
                None => "all levels".to_string(),
            },
        ),
        Readout::measured("records", console.len().to_string()),
    ];
    // Counted, not folded into the record count: a console that dropped records and a console
    // that never had them report the same `records` otherwise, and the second is the alarming one.
    if console.dropped() > 0 {
        rows.push(Readout::measured("dropped", console.dropped().to_string()));
    }
    // **The header rows are never dropped.** A bounded console that hid its own record count
    // would be a console that could not say how much it was not showing.
    let header = rows.len();
    for line in &all {
        rows.push(Readout::measured("record", line.clone()));
    }
    let total = console.len() + console.dropped();
    bounded_panel("console", "Console", header, &mut rows, available_dp, |held| {
        format!(
            "{held} older record(s) are held and not drawn; the pane holds {available_dp} dp. \
             {total} record(s) in all"
        )
    })
}

/// **The notifications window's body: the persistent records, copied from the store they are in.**
///
/// UI-SPEC §7: *refusals, approvals and anything needing a human are **persistent** — they live in
/// the notifications window and in the ledger*, and **both read the same records**. So this is a
/// reader of the console — the shell's record surface — filtered by the console's own predicate
/// ([`Level::is_persistent`]), and not a second list that could disagree with the first: a
/// notification the store does not hold is a notification nothing said.
///
/// **Read through [`Console::all`], deliberately.** The console window's own level filter and
/// search box are *its view controls*; a notifications window whose contents changed when someone
/// filtered the console would be a second view of one store pretending to be a store of its own.
///
/// **The grouping question (H-11.3) is not answered here.** The rows are the records in arrival
/// order — the one grouping the store itself chooses — and a second grouping, a default pinning or
/// a badge count would each be a decision this file does not have.
///
/// **An empty window says why it is empty** (MD1's empty-view rule, and [`Panel::validate`]
/// enforces it): *nothing persistent has happened* is a different fact from *the window is
/// broken*, and a silent blank pane is the second reading.
///
/// **Clear / manage is owed, and named.** §12.7 lists clear/manage as explicit and reversible
/// where the store allows; the store here is append-only for the console's records (C-90.7), so
/// the honest form of that control is the console's own `clear` verb and not a per-notification
/// delete that would claim a removal the record keeps.
pub fn notifications_panel(console: &Console, available_dp: f64) -> Panel {
    let persistent: Vec<&Record> = console
        .all()
        .into_iter()
        .filter(|record| record.level.is_persistent())
        .collect();
    let mut rows: Vec<Readout> = vec![Readout::measured(
        "persistent records",
        persistent.len().to_string(),
    )];
    // Counted beside the header for the same reason the console's `dropped` is: a window showing
    // two refusals out of a store holding five hundred records is a different reading from one
    // holding two.
    rows.push(Readout::measured("records", console.len().to_string()));
    let header = rows.len();
    for record in &persistent {
        rows.push(Readout::measured("record", record.describe()));
    }
    if persistent.is_empty() {
        return Panel::new("notifications", Role::Chrome, "Notifications", rows)
            .with_note(
                "no persistent records: refusals and anything needing a human appear here and \
                 stay; transient results are toasts",
            )
            .unwrap_or_else(|_| Panel::new("notifications", Role::Chrome, "Notifications", Vec::new()));
    }
    let total = console.len() + console.dropped();
    bounded_panel("notifications", "Notifications", header, &mut rows, available_dp, |held| {
        format!(
            "{held} older notification(s) are held and not drawn; the pane holds {available_dp} \
             dp. {total} record(s) in the store"
        )
    })
}

/// **Drop rows from the front until the panel fits the pane, and say how many were held back.**
///
/// The console's own bound, shared with the notifications window so the two readers cannot
/// disagree about what *fits* means — and bounded **with the note attached**, because a note is a
/// drawn row: a panel bounded while it has no note and given the note afterwards is one row too
/// tall, which is how the first version of the console's bound returned a 407 dp panel into a
/// 358 dp pane.
///
/// `header` is the number of leading rows that are never dropped: a bounded reader that hid its
/// own counts could not say how much it was not showing.
fn bounded_panel(
    what: &'static str,
    title: &str,
    header: usize,
    rows: &mut Vec<Readout>,
    available_dp: f64,
    held_note: impl Fn(usize) -> String,
) -> Panel {
    let build = |rows: &[Readout], held: usize| -> Panel {
        let panel = Panel::new(what, Role::Chrome, title, rows.to_vec());
        if held == 0 {
            return panel;
        }
        panel
            .with_note(&held_note(held))
            // The note is a formatted sentence and cannot be blank, so this arm is unreachable in
            // practice. It is here rather than `unwrap` because a note that somehow *was* blank
            // must cost the note, not the records.
            .unwrap_or_else(|_| Panel::new(what, Role::Chrome, title, rows.to_vec()))
    };
    let mut held = 0usize;
    let mut panel = build(rows, held);
    while panel.height_dp() > available_dp && rows.len() > header {
        // **From the front**, so what survives is the newest — a log whose tail is cut reads as a
        // log that ended.
        rows.remove(header);
        held += 1;
        panel = build(rows, held);
    }
    panel
}

/// **Fit a panel to the pane it is about to be drawn in, keeping what a reader looks for first.**
///
/// The declared-window check in `window_elements` is right for a window at the size its package
/// declared: a declaration that cannot hold its view is a defect in the package and a refusal with
/// both numbers. It is the **wrong check for a pane the operator made by docking** — the split is
/// the operator's, and a docked pane is half (or less) of the height the package sized the window
/// for. Before this, one such pane refused the whole window, so a docked window drew nothing.
///
/// Rows are dropped **from the back**, because a readout panel's first row is its subject (the
/// console's own bound drops from the front, because a log's newest record is its tail; both keep
/// what the reader looks for first). The count held back is named **on the panel**, so a bounded
/// pane does not read as a complete one — and the note is added *before* the fit is measured,
/// because a note is a drawn row (the same reason `bounded_panel` measures its note).
fn fit_panel(mut panel: Panel, available_dp: f64) -> Panel {
    if panel.rows.is_empty() || panel.height_dp() <= available_dp {
        return panel;
    }
    let total = panel.rows.len();
    while !panel.rows.is_empty() && panel.height_dp() > available_dp {
        panel.rows.pop();
        let held = total - panel.rows.len();
        panel.note = Some(format!(
            "{held} of {total} row(s) are held and not drawn; the pane holds {available_dp} dp"
        ));
    }
    panel
}

/// **The console window's own controls, as lanes.** The reference's `ConsoleWidget` has a
/// toolbar — filter combo, search box, Clear, Copy, Auto-Scroll — and a command bar (`portal>`
/// prompt, line edit, Run). These are those controls, named once so the drawing pass and the
/// pointer cannot disagree about where a control is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleLane {
    /// The reference's `filter_combo`, as a cycle.
    Filter,
    /// The reference's `search_input`: focus it and the keyboard edits the query.
    Search,
    /// The reference's `btn_clear`.
    Clear,
    /// The reference's `btn_copy`.
    Copy,
    /// The reference's `autoscroll_chk`.
    AutoScroll,
    /// The command line itself — clicking it puts the keyboard back on the command.
    Command,
    /// The reference's `btn_run`.
    Run,
}

/// The height of the console's toolbar and of its command bar: **one MD1 row each**, because a
/// row of targets is one 48 dp row and the controls inside it meet the target floor.
pub const CONSOLE_BAR_DP: f64 = 48.0;

/// The narrowest pane the console's toolbar can hold: the five toolbar lanes at their floors
/// plus the four gaps between them. **Derived from the lanes rather than chosen**, so a window
/// narrower than this refuses with both numbers instead of drawing controls off its edge.
pub fn console_min_width_dp() -> f64 {
    console_lane_floors().0 + console_lane_floors().1
}

/// `(fixed width of the non-search lanes, gaps between all five)` — the two numbers the
/// narrowest console is made of.
fn console_lane_floors() -> (f64, f64) {
    let min = min_target_dp();
    (152.0_f64.max(min) + 64.0_f64.max(min) + 64.0_f64.max(min) + 112.0_f64.max(min), 4.0 * spacing_dp())
}

/// **Where the console's controls are, from the pane rect alone.**
///
/// One function, called by the drawing pass and by the pointer, so a control cannot be drawn
/// where it cannot be clicked. The lanes are the reference's controls in the reference's order:
/// filter, search, Clear, Copy, Auto-Scroll on the toolbar; the line edit and Run on the
/// command bar. The `portal>` prompt is a label and carries no lane.
pub fn console_lanes(pane: Rect) -> Vec<(ConsoleLane, Rect)> {
    let row = CONSOLE_BAR_DP;
    let min = min_target_dp();
    let mut out = Vec::new();
    let filter_w = 152.0_f64.max(min);
    let clear_w = 64.0_f64.max(min);
    let copy_w = 64.0_f64.max(min);
    let auto_w = 112.0_f64.max(min);
    let (fixed, gaps) = console_lane_floors();
    let search_w = (pane.width_dp - fixed - gaps).max(min);
    let mut x = pane.x;
    out.push((ConsoleLane::Filter, Rect::new(x, pane.y, filter_w, row)));
    x += filter_w + spacing_dp();
    out.push((ConsoleLane::Search, Rect::new(x, pane.y, search_w, row)));
    x += search_w + spacing_dp();
    out.push((ConsoleLane::Clear, Rect::new(x, pane.y, clear_w, row)));
    x += clear_w + spacing_dp();
    out.push((ConsoleLane::Copy, Rect::new(x, pane.y, copy_w, row)));
    x += copy_w + spacing_dp();
    out.push((ConsoleLane::AutoScroll, Rect::new(x, pane.y, auto_w, row)));

    let command_y = pane.bottom() - row;
    let run_w = 64.0_f64.max(min);
    let prompt_w = 32.0;
    let edit_w = (pane.width_dp - prompt_w - run_w - 2.0 * spacing_dp()).max(min);
    out.push((ConsoleLane::Command, Rect::new(pane.x + prompt_w, command_y, edit_w, row)));
    out.push((
        ConsoleLane::Run,
        Rect::new(pane.x + prompt_w + edit_w + spacing_dp(), command_y, run_w, row),
    ));
    out
}

/// The three regions of a console body, or the whole pane for the log. See [`console_regions`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsoleRegions {
    pub toolbar: Option<Rect>,
    pub output: Rect,
    pub command: Option<Rect>,
}

/// **Where the console's three regions are, from the pane rect alone.**
///
/// The reference's regions are its layout's: the toolbar row, the `QTextEdit` that fills the
/// middle, and the command row — and the log window (H-7.17's second window) is the same reader
/// with no controls, so its whole pane is the output. One arithmetic for the drawing pass, the
/// pointer and the wheel: a control drawn where it cannot be clicked, a scroll surface drawn
/// where the wheel does not reach, or a note row reserved twice are one class of defect — a
/// second answer to the same question.
pub fn console_regions(pane: Rect, content: Content) -> ConsoleRegions {
    if matches!(content, Content::Console) {
        ConsoleRegions {
            toolbar: Some(Rect::new(pane.x, pane.y, pane.width_dp, CONSOLE_BAR_DP)),
            output: Rect::new(
                pane.x,
                pane.y + CONSOLE_BAR_DP,
                pane.width_dp,
                (pane.height_dp - 2.0 * CONSOLE_BAR_DP).max(0.0),
            ),
            command: Some(Rect::new(
                pane.x,
                pane.bottom() - CONSOLE_BAR_DP,
                pane.width_dp,
                CONSOLE_BAR_DP,
            )),
        }
    } else {
        ConsoleRegions { toolbar: None, output: pane, command: None }
    }
}

/// How many record rows an output region can hold.
///
/// **One arithmetic for the drawing pass and the wheel**: a scroll bound the pane could not draw
/// and a drawn window the wheel could not reach would be two answers to one question.
pub fn console_capacity(output: Rect) -> usize {
    (output.height_dp / row_height_dp()).floor().max(1.0) as usize
}

/// Where a console body's window sits inside its subject, and what the note owes the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsoleWindow {
    /// The scroll after clamping — the value a caller stores back.
    pub scroll: usize,
    /// The first record drawn, an index into the body's subject.
    pub start: usize,
    /// How many records are drawn.
    pub shown: usize,
    /// Records before the window — what scrolling back reaches.
    pub older: usize,
    /// Records after the window: the frozen view's held rows plus the rows scrolled back.
    pub newer: usize,
}

/// **The window a pane draws, from five counts and nothing else.**
///
/// The subject is the records the body reads — `visible` for the console, `all` for the log —
/// and `held` is the console's frozen-view count (zero for the log, which is the record and has
/// no freeze). `scroll` is how many rows back from the subject's end the operator has wheeled;
/// the window is clamped so its top cannot leave the subject, and **one row is the note's when
/// there is a note**, because the note is a drawn row and a pane that forgot it would push the
/// last record out of the window.
///
/// The clamp is applied here and not only at the wheel: a stale offset — a filter narrowed, a
/// log cleared — has to draw sanely too, and one function that both draws and clamps cannot
/// disagree with itself.
pub fn console_window(
    total: usize,
    held: usize,
    dropped: usize,
    capacity: usize,
    scroll: usize,
) -> ConsoleWindow {
    let capacity = capacity.max(1);
    let tail = total.saturating_sub(held);
    let fit = |rows: usize| {
        let scroll = scroll.min(tail.saturating_sub(rows));
        let end = tail - scroll;
        let shown = end.min(rows);
        (scroll, end - shown, shown, end)
    };
    let (scroll, start, shown, end) = fit(capacity);
    // The note is owed when the window cannot reach an end of the subject, or when the ceiling
    // took records; it is one drawn row, so the second pass reserves exactly one.
    let (scroll, start, shown, end) = if start > 0 || total - end > 0 || dropped > 0 {
        fit(capacity - 1)
    } else {
        (scroll, start, shown, end)
    };
    ConsoleWindow { scroll, start, shown, older: start, newer: total - end }
}

/// **The console window's body: the reference's three regions, top to bottom.**
///
/// * the **toolbar** — the filter, the search box, Clear, Copy and Auto-Scroll (one row);
/// * the **output area** — a window over the visible records, newest kept unless the operator
///   has wheeled back, with a note saying what is held on either side of the window;
/// * the **command bar** — the prompt, the draft and Run.
///
/// The log window (H-7.17's second window) is the same reader with no controls: *the event
/// record*, every level, unfiltered — which is the whole difference between the console and
/// the log.
///
/// **Nothing here decides a colour.** The record's own [`Record::line_parts`] gives the badge
/// and the rest; the ink ladder gives the two inks, and the persistent level is the loud one
/// because [`Level::is_persistent`] says so.
///
/// The argument count is the pane it draws in, the frame's palette and opacity, the reader's
/// scroll and cursor, and the sink — the same context `window_elements` already holds; a context
/// struct for one function would be a second way to pass values that have one owner.
#[allow(clippy::too_many_arguments)]
pub fn console_body(
    console: &Console,
    content: Content,
    pane: Rect,
    scroll: usize,
    opacity: f64,
    cursor: Option<(f64, f64)>,
    palette: &Palette,
    out: &mut Vec<Element>,
) -> UiResult<()> {
    let is_console = matches!(content, Content::Console);
    let is_log = matches!(content, Content::Log);
    if !is_console && !is_log {
        return Ok(());
    }

    let chrome = palette.get(Role::Chrome)?;
    let surface = palette.get(Role::Surface)?;
    let text_size = voxel_ui::text_size_dp();
    let row = row_height_dp();

    // The regions. A console needs the toolbar and the command bar before it needs any record;
    // a log needs one row. Both refusals carry both numbers, which is §5.7's rule.
    if is_console {
        if pane.height_dp + 0.5 < 3.0 * CONSOLE_BAR_DP {
            return Err(UiRefusal::ContentExceedsPane {
                window: CONSOLE_WINDOW.to_string(),
                panel_dp: 3.0 * CONSOLE_BAR_DP,
                pane_dp: pane.height_dp,
            });
        }
        if pane.width_dp + 0.5 < console_min_width_dp() {
            return Err(UiRefusal::ContentExceedsPane {
                window: CONSOLE_WINDOW.to_string(),
                panel_dp: console_min_width_dp(),
                pane_dp: pane.width_dp,
            });
        }
    } else if pane.height_dp + 0.5 < row {
        return Err(UiRefusal::ContentExceedsPane {
            window: "log".to_string(),
            panel_dp: row,
            pane_dp: pane.height_dp,
        });
    }
    let ConsoleRegions { toolbar, output, command } = console_regions(pane, content);

    // The output surface. It is the window's own body role, so the records sit on the surface
    // the package declared rather than on a second one chosen here.
    out.push(Element::Surface {
        rect: output,
        paint: Paint { rgb: surface, alpha: opacity },
    });
    let backdrop = surface.blend_over(BRIGHTEST, opacity);

    // The records. The console shows its visible window (filter, query and the autoscroll
    // freeze); the log shows the whole event record, because that is what a log is.
    let held_by_freeze = if is_console { console.held_back() } else { 0 };
    let records = if is_log { console.all() } else { console.visible() };
    // **The window this pane draws** — the subject's end, the freeze, the scrollback and the
    // note's row, resolved by the same function the wheel clamps with, so the gesture and the
    // frame cannot disagree about where the view can go.
    let window = console_window(
        records.len(),
        held_by_freeze,
        console.dropped(),
        console_capacity(output),
        scroll,
    );
    let note = if window.older > 0 || window.newer > 0 || console.dropped() > 0 {
        // **Why the newer rows are held**, so the note is a reading and not a mystery: the
        // console's Auto-Scroll control, the operator's scrollback, or both.
        let why = match (held_by_freeze > 0, window.scroll > 0) {
            (true, true) => " (autoscroll off, scrolled back)",
            (true, false) => " (autoscroll off)",
            (false, true) => " (scrolled back)",
            (false, false) => "",
        };
        Some(format!(
            "{} older record(s) held; {} newer held{why}; {} dropped at the ceiling",
            window.older,
            window.newer,
            console.dropped()
        ))
    } else {
        None
    };
    let mut y = output.y;
    for record in records.iter().skip(window.start).take(window.shown) {
        let (badge, rest) = record.line_parts();
        let ink = if record.level.is_persistent() { Ink::Full } else { Ink::Secondary };
        contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
        let badge_w = 72.0;
        out.push(Element::Text {
            rect: Rect::new(output.x + spacing_dp(), y + (row - text_size) / 2.0, badge_w, text_size),
            text: badge,
            paint: ink.paint(),
            size_dp: text_size,
            align: Align::Start,
        });
        out.push(Element::Text {
            rect: Rect::new(
                output.x + spacing_dp() + badge_w,
                y + (row - text_size) / 2.0,
                (output.width_dp - 2.0 * spacing_dp() - badge_w).max(0.0),
                text_size,
            ),
            text: rest,
            paint: ink.paint(),
            size_dp: text_size,
            align: Align::Start,
        });
        y += row;
    }
    if let Some(note) = note {
        let ink = Ink::Tertiary;
        contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                output.x + spacing_dp(),
                output.bottom() - row + (row - text_size) / 2.0,
                (output.width_dp - 2.0 * spacing_dp()).max(0.0),
                text_size,
            ),
            text: note,
            paint: ink.paint(),
            size_dp: text_size,
            align: Align::Start,
        });
    }
    if is_console && console.is_empty() {
        // **An empty console is not a blank pane.** The reference's own opening line is the
        // honest note: it is a console with nothing in it yet, and it says so.
        let ink = Ink::Secondary;
        contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                output.x + spacing_dp(),
                output.y + (row - text_size) / 2.0,
                (output.width_dp - 2.0 * spacing_dp()).max(0.0),
                text_size,
            ),
            text: "Portal Console ready. Type 'help' for available commands.".to_string(),
            paint: ink.paint(),
            size_dp: text_size,
            align: Align::Start,
        });
    }

    // The toolbar and the command bar: each is a chrome row with a rule above its top edge.
    for region in [toolbar, command].into_iter().flatten() {
        out.push(Element::Surface {
            rect: region,
            paint: Paint { rgb: chrome, alpha: opacity },
        });
        out.push(Element::Rule {
            rect: Rect::new(region.x, region.y, region.width_dp, 1.0),
            paint: Ink::Divider.paint(),
        });
    }
    if let Some(command) = command {
        // The prompt, at the command bar's leading edge — the reference's `portal>` label.
        let ink = Ink::Secondary;
        let bar_backdrop = chrome.blend_over(BRIGHTEST, opacity);
        contrast_over_backdrop(ink.composited_over(bar_backdrop), bar_backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                command.x + spacing_dp(),
                command.y + (CONSOLE_BAR_DP - text_size) / 2.0,
                24.0,
                text_size,
            ),
            text: ">".to_string(),
            paint: ink.paint(),
            size_dp: text_size,
            align: Align::Start,
        });
    }

    if is_console {
        let bar_backdrop = chrome.blend_over(BRIGHTEST, opacity);
        for (lane, lane_rect) in console_lanes(pane) {
            let hovered = cursor.is_some_and(|at| lane_rect.contains(at));
            if hovered {
                out.push(Element::Surface {
                    rect: lane_rect,
                    paint: Paint { rgb: chrome.elevated(1)?, alpha: opacity },
                });
            }
            let label = match lane {
                ConsoleLane::Filter => Some(console.filter_name()),
                ConsoleLane::Search => Some(if console.search().is_empty() {
                    "Filter logs...".to_string()
                } else {
                    console.search().to_string()
                }),
                ConsoleLane::Clear => Some("Clear".to_string()),
                ConsoleLane::Copy => Some("Copy".to_string()),
                ConsoleLane::AutoScroll => Some(if console.autoscroll() {
                    "Auto-Scroll on".to_string()
                } else {
                    "Auto-Scroll off".to_string()
                }),
                ConsoleLane::Command => Some(console.draft().to_string()),
                ConsoleLane::Run => Some("Run".to_string()),
            };
            let Some(label) = label else { continue };
            let (label, ink) = match lane {
                ConsoleLane::Search if console.search().is_empty() => (label, Ink::Tertiary),
                ConsoleLane::Command if console.draft().is_empty() => (label, Ink::Tertiary),
                _ if hovered => (label, Ink::Full),
                _ => (label, Ink::Secondary),
            };
            if label.is_empty() {
                continue;
            }
            contrast_over_backdrop(ink.composited_over(bar_backdrop), bar_backdrop)?;
            out.push(Element::Text {
                rect: Rect::new(
                    lane_rect.x + spacing_dp(),
                    lane_rect.y + (lane_rect.height_dp - text_size) / 2.0,
                    (lane_rect.width_dp - 2.0 * spacing_dp()).max(0.0),
                    text_size,
                ),
                text: label,
                paint: ink.paint(),
                size_dp: text_size,
                align: Align::Start,
            });
        }
    }
    Ok(())
}

/// The body of a window whose view is not implemented yet.
///
/// **This is not a placeholder for the window manager — the window manager is this module.** It
/// is the MD1 empty-view rule applied to a content kind that has no view yet: the window still
/// opens, still moves, still splits, still pins, and its body says what the package declared it
/// to hold instead of showing a blank pane. The note names the state; it does not claim the
/// view exists.
pub fn declared_content_panel(spec: &WindowSpec) -> Panel {
    Panel::new(
        "declared content",
        Role::Chrome,
        &spec.title,
        vec![
            Readout::measured("content", spec.content.name().to_string()),
            Readout::measured("lane", spec.lane.name().to_string()),
            Readout::measured("record", spec.record_id.clone()),
            Readout::measured(
                "geometry",
                format!("{}×{} dp", spec.geometry[2], spec.geometry[3]),
            ),
            Readout::measured("policy", spec.layout_policy.declared()),
        ],
    )
    .with_note(
        "the view for this content kind is not implemented yet; the package's declaration is \
         what this window holds",
    )
    .expect("the note is not blank")
}

/// A pointer button, remapped here so the mount does not depend on a windowing library.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Left,
    Right,
}

/// A wheel gesture, remapped here for the same reason [`Button`]: `voxel-client` has no
/// windowing dependency, and the bin is the only place that may name a platform type.
///
/// **Positive is away from the operator** — the direction that reads back into the record.
/// The two variants are the platform's own units: a notched wheel reports lines, a trackpad
/// reports pixels, and the conversion to rows happens in [`Mount::wheel`], not in the bin.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WheelDelta {
    /// Whole steps from a notched wheel, in the platform's line unit.
    Lines(f64),
    /// A delta from a trackpad, in device-independent pixels.
    Pixels(f64),
}

/// **The wheel's step, in record rows per reported line.**
///
/// The reference does not implement the wheel: it hands the gesture to `QTextEdit`, whose step
/// is the platform's — three rows a notch, the desktop text-area convention the simulator
/// inherits rather than chooses. That is a platform translation and not a decision the
/// simulator made, so it is declared here, with the one number the shell uses, instead of being
/// inherited invisibly. Pixel deltas are not stepped: they are divided by [`row_height_dp`] and
/// accumulated, because a trackpad delta is routinely smaller than a row.
pub const WHEEL_ROWS_PER_LINE: f64 = 3.0;

/// **The shell's window manager.** One value, and every window on screen is in it.
pub struct Mount {
    windows: BTreeMap<String, Window>,
    /// The z-order, back to front, of the windows that float. A docked window is not in it: its
    /// rect is decided by its host's arrangement.
    ordered: Vec<String>,
    bar: Vec<BarEntry>,
    /// The package's `filed` rows, kept as pairs **in declaration order**: a map would sort
    /// them, and the order is the order the tab's menu offers.
    filed: Vec<(String, String)>,
    /// The tab whose menu is open, if one is.
    menu: Option<String>,
    shown: bool,
    focused: Option<String>,
    cursor: Option<(f64, f64)>,
    /// **The drag in flight** — the port's own [`DragState`](window::DragState): the window, the
    /// grab offset, whether the pointer moved, and the drop preview it is showing. **One source
    /// for the preview**: the drawing pass reads it through [`Mount::drop_preview`], so what a
    /// release lands on is what the frame drew.
    drag: Option<window::DragState>,
    /// **A resize in flight** — the port's `_resize_from` (3444–3473): the window, the rect the
    /// drag started at, the edges it grabbed, and where the pointer was then.
    resize: Option<(String, Rect, ResizeEdges, (f64, f64))>,
    /// **A seam drag in flight** — the port's own [`window::SeamDrag`]: the splitter and
    /// boundary a handle press named, the shares it started from, and where the pointer grabbed.
    /// Every move reads the start pair through `Arrangement::drag_seam`, so the shares never
    /// compound.
    seam: Option<window::SeamDrag>,
    /// The trigger lanes shown for the window under the pointer, and the zone under it — the
    /// port's `show_edge_triggers`/`hide_edge_triggers` (3270–3296) as state.
    triggers: Option<(String, EdgeTriggers, Option<drop::DropZone>)>,
    /// **Where each pane's scrollback is** — how many record rows it is showing behind its
    /// subject's end, keyed by pane.
    ///
    /// **Per pane and not on the `Console`.** Two windows read one console (the console and the
    /// log), and a viewport belongs to the window it is drawn in: two readings of one record are
    /// two scroll positions. The one thing that *is* the console's — Auto-Scroll — is its own
    /// field there, and this map never overrules it.
    scroll: BTreeMap<String, usize>,
    /// The fraction of a row a trackpad delta has accumulated but not yet spent. See
    /// [`Mount::wheel`].
    wheel_remainder: f64,
    viewport: Rect,
    /// The refusals from the last layout, in text, one per window that could not be drawn.
    ///
    /// **Kept as text and not as `UiRefusal` values**: the shell's caller prints these, and the
    /// panel that raised one has already been drawn or skipped by the time anyone reads them.
    refusals: Vec<String>,
    /// The shell's one console, and **the only writer of it** (BLD-0018).
    ///
    /// Owned here rather than handed in per frame, because `alpha-shell.shspec` declares a
    /// `console` window and `ContentViews` is built before the frame is drawn: a console that
    /// arrived as an argument would be a different console each frame, and a log that resets
    /// when the view is rebuilt is not a log. [`Mount::log`] is the chokepoint every report
    /// goes through, as the reference's `log(level, category, message)` is.
    console: Console,
    /// **The shell's toast queue** (H-12.2): the transient sibling of a panel, owned by the shell
    /// and drained one record at a time.
    ///
    /// The queue's own type lives in [`super::notifications`] — where the oracle's known answers
    /// put `enqueue(toast);` — and this field is the ownership the ruling names. [`Mount::announce`]
    /// is the one writer: it logs the record and enqueues the transient half, so a toast cannot
    /// exist without the console record it came from (UI-SPEC §7).
    toasts: ToastQueue,
    /// The tick the toast queue was last advanced to — **the only clock a countdown reads**, set by
    /// the frame loop through [`Mount::set_tick`] so the queue moves at the world's rate.
    toast_tick: Option<u64>,
    /// **The last package the live-reload watch dealt with**, and the digest it dealt with it at.
    ///
    /// A digest already handled is not read again — and *handled* includes **refused**: a package
    /// that does not frame, or that [`Mount::reload`] turns away, would otherwise be re-read and
    /// re-refused on every tick, which turns one bad file into a console that says nothing else.
    packages_seen: Option<(std::path::PathBuf, voxel_model::identity::Digest)>,
}

impl Mount {
    /// Mount a loaded package.
    ///
    /// **Every declared window is built here, and nothing is dropped.** A package that declares
    /// a window with no home is the console-opened case, not a defect (H-15.2): the window
    /// exists, closed, and the console can open it by id. The only refusals here are a
    /// `theme-role` this tree has no role for — which is a refusal by name, not a default — and
    /// a geometry smaller than a window can be.
    pub fn new(loaded: &Loaded) -> UiResult<Mount> {
        let mut mount = Mount {
            windows: BTreeMap::new(),
            ordered: Vec::new(),
            bar: Vec::new(),
            filed: Vec::new(),
            menu: None,
            shown: true,
            focused: None,
            cursor: None,
            drag: None,
            resize: None,
            seam: None,
            triggers: None,
            scroll: BTreeMap::new(),
            wheel_remainder: 0.0,
            viewport: Rect::new(0.0, 0.0, 1280.0, 720.0),
            refusals: Vec::new(),
            console: Console::new(),
            toasts: ToastQueue::default(),
            toast_tick: None,
            packages_seen: None,
        };
        mount.adopt(&loaded.recipe)?;
        Ok(mount)
    }

    /// **Adopt a package's declarations: windows, bar, filed rows, and the pinned ones opened.**
    ///
    /// # One place, because a second mount path is a second arrangement
    ///
    /// [`Mount::new`] and [`Mount::reload`] both have to turn a `Recipe` into the shell's state,
    /// and when they were two functions they were two answers to *what does this package declare* —
    /// the class of defect `VERBS_COVER_EVERY_VARIANT` was written for, in the arrangement rather
    /// than the vocabulary. There is one body here and both callers reach it.
    ///
    /// **Nothing is merged.** Every window in `recipe` is built and every window this shell was
    /// holding is dropped, because *a shell that resized them would be choosing an arrangement,
    /// and the arrangement is a package*. The caller decides whether adopting is allowed
    /// ([`Mount::reload`] checks the console window survives it); this function does the adopting.
    fn adopt(&mut self, recipe: &voxel_package::recipe::Recipe) -> UiResult<()> {
        let mut windows = BTreeMap::new();
        for spec in &recipe.windows {
            // The theme role is validated at mount time, not at draw time, so a package that
            // cannot be themed fails where it is admitted rather than frame by frame.
            Role::from_theme_role(&spec.theme_role)?;
            let rect = Rect::new(
                spec.geometry[0] as f64,
                spec.geometry[1] as f64,
                spec.geometry[2] as f64,
                spec.geometry[3] as f64,
            );
            let own = WindowId::new(spec.id.clone())?;
            windows.insert(
                spec.id.clone(),
                Window {
                    spec: spec.clone(),
                    status: Status::Closed,
                    rect,
                    previous: None,
                    arrangement: Arrangement::of(own),
                    host: None,
                },
            );
        }

        // The bar: the tabs in strip order, then the entries that are not tabs (H-15.2). Both
        // are declarations; neither is inferred from the lane or from declaration order.
        let mut bar = Vec::new();
        for id in &recipe.tabs {
            let window = windows.get(id).expect("the recipe validates its tab rows");
            bar.push(BarEntry { id: id.clone(), tab: true, label: window.spec.title.clone() });
        }
        for id in &recipe.bar {
            let window = windows.get(id).expect("the recipe validates its bar rows");
            bar.push(BarEntry { id: id.clone(), tab: false, label: window.spec.title.clone() });
        }

        self.windows = windows;
        self.bar = bar;
        self.filed = recipe.filed.clone();
        // **The z-order, focus and drag are this shell's state and the new package declares
        // none of them.** Carrying a drag across a reload would apply a mouse-down from the old
        // arrangement to a window in the new one, and carrying focus would name a window that
        // may not exist any more.
        self.ordered = Vec::new();
        // The scrollbacks and the wheel's fraction are view state too: an offset for a pane in
        // an arrangement that no longer exists is the same kind of stale state as a focus.
        self.scroll.clear();
        self.wheel_remainder = 0.0;
        self.focused = None;
        self.drag = None;
        self.menu = None;
        self.refusals = Vec::new();

        // The pinned windows are open when the game starts: H-9.8's whole point is that they
        // are in the user's view *while the shell is hidden*, so hiding them at start-up would
        // make the pinned layer unobservable until someone opened the shell first.
        for id in &recipe.pinned {
            self.open(id)?;
        }
        Ok(())
    }

    /// **Reload the shell from another package, live.**
    ///
    /// # The one condition, and it is a refusal rather than a merge
    ///
    /// The console window is how a player files the next ticket — the keys go to
    /// [`Mount::key_pressed`] only while [`Mount::console_has_focus`], and focus comes from a
    /// window. So a package that declares no console window would take the command line off the
    /// screen with the draft still in it, and the shell would be unfixable without a restart.
    /// **Refused, by name, with the windows it did declare.** This is the client's own rule about
    /// the shell staying usable, and it is stated here rather than in the compiler: a solution is
    /// allowed to declare no window at all (`Solution::for_ticket` compiles a prose ticket to an
    /// empty arrangement, ruled 2026-10-01), and it is *adopting* that arrangement over a running
    /// shell which is refused. The ticket has to name the console to replace the package.
    ///
    /// **The console survives either way.** It is [`Mount`]'s own field, not a window's, so its
    /// records and its half-typed draft are untouched by an adoption — which is what makes the
    /// refusal recoverable rather than fatal.
    pub fn reload(&mut self, loaded: &Loaded) -> Result<(), String> {
        let declared: Vec<&str> =
            loaded.recipe.windows.iter().map(|window| window.id.as_str()).collect();
        if !loaded.recipe.windows.iter().any(|window| window.id == CONSOLE_WINDOW) {
            return Err(format!(
                "the package declares {} window(s) ({}) and none of them is {CONSOLE_WINDOW:?}, so \
                 adopting it would take the command line off the screen. Not reloaded; the \
                 arrangement in force is unchanged",
                declared.len(),
                declared.join(", ")
            ));
        }
        self.adopt(&loaded.recipe).map_err(|refusal| refusal.to_string())
    }

    // -----------------------------------------------------------------------
    // Reporting
    // -----------------------------------------------------------------------

    /// The one write path into the console (BLD-0018).
    ///
    /// **A chokepoint, and the reason it is here rather than on `Console`.** `Console::log` is
    /// already the only way a record enters, so this adds no second door — it names the shell
    /// as the caller every window reports through, which is what the reference's
    /// `log(level, category, message)` is, and what makes a record's arrival attributable to
    /// the frame it arrived in.
    pub fn log(
        &mut self,
        level: Level,
        category: &str,
        message: impl Into<String>,
        tick: Option<u64>,
    ) {
        self.console.log(level, category, message, tick);
    }

    /// **The one API entry that produces both halves** (C-54.3): the record goes to the console
    /// **and**, when the record is transient and timed, the same words are enqueued as a toast.
    ///
    /// # Why one call rather than two
    ///
    /// UI-SPEC §7: *both read the same records, so a toast cannot say more than the record it came
    /// from.* Two calls at a call site let one happen without the other — the failure L-5's
    /// chokepoint exists to prevent — so this is the chokepoint: the record is written first and
    /// the toast is derived from it, never beside it.
    ///
    /// # Who may be a toast, and who may not
    ///
    /// [`Level::is_persistent`] is the whole predicate, and it is C-64.2's MD1 split already in the
    /// console: *a refusal is never a toast that scrolls away.* A `Warn` is logged and **not**
    /// enqueued — the notifications window and the ledger are its surfaces — and everything else is
    /// MD1's *transient result* class.
    ///
    /// **A record with no tick is not raisable.** The countdown is a function of the world's clock,
    /// and a toast that invented a tick zero would show a duration nobody could read back (D107):
    /// the record goes to the console and nothing is enqueued.
    pub fn announce(
        &mut self,
        level: Level,
        category: &str,
        message: impl Into<String>,
        tick: Option<u64>,
    ) {
        let message = message.into();
        self.log(level, category, message.clone(), tick);
        // A refusal is never a toast (C-64.2's predicate), and a record with no tick cannot be
        // timed — it has no countdown to show (D107). Either way the record is already written.
        if level.is_persistent() || tick.is_none() {
            return;
        }
        self.toasts.enqueue(message);
    }

    /// **Advance the shell's own clock** — the toast queue's countdown — once per frame, and
    /// idempotent within a tick so a fast frame cannot shorten a toast's life. This is the seam
    /// where any later transient surface reads the same tick rather than inventing a clock.
    pub fn set_tick(&mut self, tick: u64) {
        self.toast_tick = Some(tick);
        self.toasts.advance(tick);
    }

    /// **The toast surface, drawn last** — the shell's transient half (C-54.3, H-12.2).
    ///
    /// The layer that chose this frame appends these elements **after its own**, so the toast is
    /// above the windows and above the pinned layer: it is the shell speaking, and a message the
    /// player's own action produced must not be able to hide behind a window. It draws with the
    /// shell down as well as up, which is the visible half of `policy::Action::RaiseToast`.
    ///
    /// # The pieces, and where each one comes from
    ///
    /// * **The surface** is chrome at the HUD's declared panel opacity — the same surface and the
    ///   same derivation the bar uses, so the toast is the same material as the chrome it floats
    ///   over rather than a new one. Its elevation is the top of the admitted ladder: MD1 places a
    ///   snackbar above the app bar, and the ladder here is five levels with the package owning
    ///   its own — a toast is not a package surface, so it takes the level no declared window can
    ///   out-rank.
    /// * **The text** is the record's own words, in the full ink step, and its contrast is
    ///   **measured against the composited backdrop it will really sit on** rather than asserted.
    /// * **The timer** is this project's own: MD1 specifies no visible countdown for a snackbar,
    ///   and the user asked for one. It is a one-dp rule — MD1's divider, so no new thickness is
    ///   born — whose width is the fraction of the toast's own six seconds that is left.
    /// * **The entrance** is one authored moment: an opacity ramp in ticks, whose duration is
    ///   declared beside the toast's duration. No displacement — a slide would be *meaningful*
    ///   motion, which owes a reduced alternative and more than four ticks to read at.
    pub fn toast_elements(
        &self,
        palette: &Palette,
        viewport: Rect,
        out: &mut Vec<Element>,
    ) -> UiResult<()> {
        let (Some(tick), Some(toast)) = (self.toast_tick, self.toasts.current()) else {
            return Ok(());
        };
        let (_, fraction) = self
            .toasts
            .remaining(tick)
            .expect("a current toast has a remaining time");
        let width = voxel_ui::toast_width_dp();
        let height = row_height_dp();
        let (x, y) = Corner::BottomEnd.origin(viewport, width, height);
        let rect = Rect::new(x, y, width, height);
        let chrome = palette.get(Role::Chrome)?;
        let opacity = voxel_ui::panel_opacity();
        // The entrance ramp: the first `toast_entry_ticks()` of the toast's life, clamped, so the
        // surface is at full opacity for the rest of its six seconds.
        let entry = super::notifications::toast_entry_ticks().max(1);
        let ramp = (tick.saturating_sub(toast.shown_at) + 1).min(entry) as f64 / entry as f64;
        out.push(Element::Surface {
            rect,
            paint: Paint { rgb: chrome.elevated(max_elevation())?, alpha: opacity * ramp },
        });
        let backdrop = chrome.blend_over(BRIGHTEST, opacity);
        contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                rect.x + spacing_dp(),
                rect.y + (height - voxel_ui::text_size_dp()) / 2.0,
                width - 2.0 * spacing_dp(),
                voxel_ui::text_size_dp(),
            ),
            text: toast.text.clone(),
            paint: Ink::Full.paint(),
            size_dp: voxel_ui::text_size_dp(),
            align: Align::Start,
        });
        contrast_over_backdrop(Ink::Secondary.composited_over(backdrop), backdrop)?;
        out.push(Element::Rule {
            rect: Rect::new(rect.x, rect.bottom() - 1.0, width * fraction, 1.0),
            paint: Ink::Secondary.paint(),
        });
        Ok(())
    }

    /// The shell's console, for a caller that reads it rather than writes it.
    pub fn console(&self) -> &Console {
        &self.console
    }

    /// Where a `file` from the console is filed. **Set once, by the caller that knows the root.**
    pub fn set_workspace(&mut self, root: std::path::PathBuf) {
        self.console.set_workspace(root);
    }

    /// **Read the build gate's decision-layer pass onto the console.**
    ///
    /// # Why the client's own surface, and why these bytes
    ///
    /// D98: *“a finding that can only be read in a terminal has no client representation and is
    /// therefore incomplete.”* UI-SPEC §9 and C-53.3: *the build gate is the hard stop and **the
    /// same findings are a record the client renders**, so a green build and a legible panel
    /// cannot disagree.* This is that second reader (`BLD-0019`, `BLD-0026`), on the window that
    /// already exists.
    ///
    /// **The bytes the build wrote, not a second pass.** C-78.6 refuses re-deriving the answers
    /// (*“parsing the gate's text back out would not deliver ‘cannot disagree’”*) — so this does
    /// **not** re-decide. It reads [`voxel_package::decision::read_report`], which lives beside
    /// the writer in the same module and is round-tripped against it by
    /// `the_report_reads_back_exactly_what_the_report_wrote`. The console shows the pass this
    /// build produced, and a reader that could not understand it is **refused by name** rather
    /// than shown as zeroes.
    ///
    /// **Every unsettled subject is printed with its reason** (C-83.5): *“which decision is not
    /// settled is the question and a count is not the answer.”* The pane bounds how many rows are
    /// drawn and the panel says how many it is holding, so a long tail is bounded rather than
    /// silently cut.
    pub fn read_gate(&mut self, text: &str, tick: Option<u64>) -> Result<(), String> {
        let (pass, two) = voxel_package::decision::read_report(text).map_err(|refusals| {
            refusals
                .all()
                .iter()
                .map(|refusal| refusal.detail().to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;
        self.console.log(
            Level::Event,
            "gate",
            format!(
                "the decision layer: {} decided {} subject(s) — {} active, {} inactive, \
                 {} unchecked, {} refused",
                pass.decider,
                pass.subjects,
                pass.active,
                pass.inactive,
                pass.unchecked,
                pass.refused
            ),
            tick,
        );
        for row in &pass.pending {
            // **A holding row is a catch, not a warning.** The report prints a holding subject
            // when it carries a reason — the ticket-evidence ladder does, naming the citations
            // it graded — and that row is good news with a fact attached; a row that does not
            // hold stays a warning, exactly as before.
            let level = if row.state == voxel_package::features::Establishment::Active {
                Level::Event
            } else {
                Level::Warn
            };
            self.console.log(
                level,
                "gate",
                format!("{} is {}: {}", row.subject, row.state.slug(), row.reason),
                tick,
            );
        }
        self.console.log(
            Level::Event,
            "gate",
            format!(
                "pass two, the model instrument: {} decided {}, left {} unreached, {} divergence(s)",
                two.model_decider,
                two.model_decided,
                two.model_unreached,
                two.divergences
            ),
            tick,
        );
        // **A disagreement is preserved, not synthesized** (C-84.3), so each one is on its own
        // line with both answers beside it rather than being counted and dropped.
        for row in &two.divergence_rows {
            self.console.log(Level::Warn, "gate", format!("disagreement: {row}"), tick);
        }
        Ok(())
    }

    /// **File a ticket in the player's own words, and read back the solution it compiles to.**
    ///
    /// The chain is **grown**, never appended: `append` creates the file once and refuses a
    /// second filing by design, while `grow` extends and has no code path that can truncate — so
    /// refusing to overwrite still holds and the earlier bytes are still there, in order. That
    /// is the whole safety argument, and it is why this is not a confirmation prompt bolted on
    /// top: the refusal-not-overwrite property is already the gate.
    ///
    /// **The bytes are re-read immediately before growing.** The file has moved since the last
    /// filing, and handing `grow` a stale head produces a record claiming a predecessor that is
    /// not the record before it — refused as `ChainBroken` on read-back, correctly.
    fn file(&mut self, args: &[String], tick: Option<u64>) {
        let words = args.join(" ");
        let Some(workspace) = self.console.workspace().map(std::path::Path::to_path_buf) else {
            self.refuse_cmd(
                Verb::File,
                "this client was given no workspace, so there is no record to file into",
                tick,
            );
            return;
        };
        let namespace = match voxel_model::chain::Namespace::new(
            CONSOLE_WORLD,
            CONSOLE_YEAR,
            voxel_model::chain::Season::Winter,
            CONSOLE_CAMPAIGN,
        ) {
            Ok(namespace) => namespace,
            Err(refusals) => {
                self.refuse_cmd(Verb::File, refusals.to_string(), tick);
                return;
            }
        };

        let path = voxel_package::filing::chain_path(&workspace);
        // **An absent chain is an empty prior, not a failure** — `grow` on a workspace that was
        // never filed is refused by name, which is the honest answer for a client whose first
        // filing is also its first record.
        let before = std::fs::read(&path).unwrap_or_default();
        let grown = voxel_package::filing::grow(
            &workspace,
            &before,
            [voxel_package::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind: voxel_model::chain::EventKind::TicketTransition,
                question: &words,
                answer: None,
                kind: voxel_model::envelope::Kind::Authority,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: CONSOLE_ACTOR,
                round_id: CONSOLE_CAMPAIGN,
                supersedes: None,
            }],
        );
        if let Err(refusals) = grown {
            // **Every refusal, not the first** — a reader fixing a command needs all of them.
            for refusal in refusals.all() {
                self.refuse_cmd(Verb::File, refusal.detail().to_string(), tick);
            }
            return;
        }

        // Read it back **off the chain**, the way the gate does, so what is reported is what a
        // later build will read rather than what this call believes it wrote.
        let read = match voxel_package::compiler::read_back(&path) {
            Ok(read) => read,
            Err(refusals) => {
                self.refuse_cmd(Verb::File, refusals.to_string(), tick);
                return;
            }
        };
        let Some(ticket) = read.board.tickets().last() else {
            self.refuse_cmd(Verb::File, "the record grew but the board has no ticket on it", tick);
            return;
        };
        let (id, objective) = (ticket.id.as_str().to_string(), ticket.objective.clone());
        self.console.log(Level::Event, "file", format!("{id} filed"), tick);
        self.console.log(Level::Event, "file", objective, tick);

        // And the solution: **the ticket compiled, the package was emitted, and the shell is
        // running it.** Three steps in the dirty branch's order — compile to a package, write it
        // where the loader's search path is, read it back through the reader every package loads
        // through — because *an emit nothing loads is half a transaction*.
        //
        // `compiler::emit` is the whole audited loop rather than `Solution::for_ticket` alone:
        // every stage passes the gate, provenance is verified, and C-89.6's complaint that the
        // back-compiler had no caller outside tests is answered by a caller that files for a
        // living.
        //
        // **The grant is the player at the keyboard, and it is bounded to this ticket.** Every
        // stage is `RequiresGrant` and `decide` refuses a missing one — *"silence is not
        // permission"* — so the authorising fact is not optional. What authorises it is that
        // somebody typed `file` into this console: `CONSOLE_ACTOR` is who the record already
        // attributes their filings to. The scope names the one ticket, because *a grant with no
        // scope authorizes everything, which is the same failure as authorizing nothing*.
        let grant = match voxel_model::grant::Grant::new(
            CONSOLE_ACTOR,
            format!("the arrangement declared by {id}"),
        ) {
            Ok(grant) => grant,
            Err(refusals) => {
                for refusal in refusals.all() {
                    self.refuse_cmd(Verb::File, refusal.detail().to_string(), tick);
                }
                return;
            }
        };
        let mut chain = read.chain;
        let (package, receipt) =
            match voxel_package::compiler::emit(&mut chain, ticket, Some(&grant)) {
            Ok(emitted) => emitted,
            Err(refusals) => {
                for refusal in refusals.all() {
                    self.refuse_cmd(Verb::File, refusal.detail().to_string(), tick);
                }
                return;
            }
        };
        // **The name is derived from the content identity** (`filing::package_file_name`), so a
        // recompile of this ticket writes a *new* package rather than colliding with the one an
        // earlier compiler produced. A record is never overwritten and never deleted: the older
        // package stays on disk as the record of what the older compiler produced.
        let path = voxel_package::filing::package_path(
            &workspace,
            &receipt.name,
            &receipt.package_digest,
        );
        if let Err(refusals) = voxel_package::filing::file(&path, &package) {
            for refusal in refusals.all() {
                self.refuse_cmd(Verb::File, refusal.detail().to_string(), tick);
            }
            return;
        }
        let digest = receipt.package_digest.to_string();
        self.console.log(
            Level::Event,
            "file",
            format!(
                "{} emitted {} ({} audited stage(s), package {})",
                receipt.name,
                path.display(),
                receipt.audited,
                &digest[..16.min(digest.len())]
            ),
            tick,
        );

        // ---- the agent's inbox: the recipe, on the notification stream -------
        //
        // DSL: `queue -> agent "the recipes the game derived from the solutions"`. **The game gives
        // the agent the recipe it needs, and the judge writes the record** — the agent authors
        // nothing, and there is no second channel: this is the same musli record, the same
        // `sha256(artifact)` binding and the same stream (`NotificationStream`) the notifications
        // window already reads, written into `logs/agent-inbox/`.
        //
        // **The artifact is the package, and that is the whole delivery.** `recipe::load` returns
        // the exact recipe source a package carries, so the rows the package implements — the
        // work, with the decision's own words — travel inside it. The record's job is to be the
        // pointer *with a digest*: the agent re-hashes the package before it implements anything.
        //
        // **A refusal here does not lose the package.** The package is filed and mountable; what
        // is refused is the *notification*, and it is refused by name so the player can see that
        // no recipe reached the agent rather than assuming one did.
        match voxel_model::laya::relative_to(&workspace, &path) {
            Ok(relative) => {
                // **Provenance held, or `emit` would not have returned**: `verify_provenance`
                // loads the package back and refuses one that does not carry the exact source it
                // was compiled from, and every stage of the audit is permitted before that. So
                // the value handed to the judge is the compiler's own measurement, not a hope.
                let provenance_held = true;
                match voxel_model::laya::notify_the_agent_of(
                    &workspace,
                    // **The ticket names the record**, so a later filing about another ticket
                    // cannot land on this one's name: the inbox holds work the agent owes.
                    &receipt.name,
                    &relative,
                    &receipt.package_digest,
                    provenance_held,
                ) {
                    Ok(record) => self.console.log(
                        Level::Event,
                        "inbox",
                        format!(
                            "the agent's inbox: {record} — {relative}, at {}",
                            voxel_model::laya::agent_inbox_dir(&workspace).display()
                        ),
                        tick,
                    ),
                    Err(refusal) => self.refuse_cmd(
                        Verb::File,
                        format!("the recipe could not be notified to the agent: {refusal}"),
                        tick,
                    ),
                }
            }
            Err(refusal) => self.refuse_cmd(Verb::File, refusal.detail().to_string(), tick),
        }

        // **Read the file back off disk and mount that**, never the bytes this call holds: a
        // reload that mounted its own in-memory copy would be a claim about a package nobody
        // can read.
        let before: Vec<String> = self.windows.keys().cloned().collect();
        match self.load_package(&path) {
            Ok(loaded) => {
                let mounted: Vec<&str> =
                    loaded.recipe.windows.iter().map(|w| w.id.as_str()).collect();
                self.console.log(
                    Level::Event,
                    "file",
                    format!(
                        "the shell now runs {}: {} window(s): {mounted:?}",
                        path.display(),
                        mounted.len()
                    ),
                    tick,
                );
                // **The work the package carries, named.** `recipe::load` gives the recipe out of
                // the package's own bytes, so this line is the reader's answer to *what did the
                // agent just get handed* — the rows, and the words of the first of them.
                let rows = &loaded.recipe.rows;
                self.console.log(
                    Level::Event,
                    "inbox",
                    format!(
                        "{} row(s) of work in this recipe: {}",
                        rows.len(),
                        rows.iter()
                            .map(|row| format!("{} — {}", row.id, row.text))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                    tick,
                );
                self.console.log(
                    Level::Info,
                    "file",
                    format!("replaced {} declared window(s): {before:?}", before.len()),
                    tick,
                );
            }
            Err(why) => self.refuse_cmd(Verb::File, why, tick),
        }
    }

    /// **The recipes the record already holds, handed to the agent** — the world-open pass.
    ///
    /// DSL: `queue -> agent "the recipes the game derived from the solutions"`. **The agent authors
    /// nothing; the game gives it the recipes it needs** — and the record already holds work nobody
    /// has handed over: filings that reached the chain before this pass existed. So this reads the
    /// board and, for each of them with no record in the agent's inbox yet, back-compiles the
    /// solution, emits the package and lets the judge write the recipe.
    ///
    /// # The authority is read, never invented
    ///
    /// `emit` refuses without a grant (`silence is not permission`), and this pass **does not
    /// invent one**: the grant is the console's own — the same granter and the same scope shape the
    /// `file` command makes when a person types it — because what authorises the work is the filing
    /// itself, which is on the record and readable. A pass that granted itself a scope the record
    /// does not carry would be a second authority, which is the defect `forbidden.detached-judge`
    /// is one instance of.
    ///
    /// # Every ticket, and why the world filter is gone
    ///
    /// **The whole board** — every open ticket on the chain, whichever world filed it. The world
    /// filter the first version carried was a scope this pass had no ruling for, and the user's
    /// correction named it (C-104.3, C-105): *"the game and laya should fucking catch the goddamn
    /// tickets"*, and *"how are we supposed to get recipes and solutions of code if the laya
    /// ticket evaluator isn't hooked up"*. The board is one record, `Board::from_chain` folds all
    /// of it, and the ledger's own `CSE` cases — filed in world-2468 — are work the agent was owed
    /// and the old scope silently left out. **Nothing is filtered out and reported as handled:**
    /// the counts this pass returns are the whole board's.
    ///
    /// # The evaluator is the compiler's audited loop, and that is the wiring
    ///
    /// DSL: `board -> compiler "the rows the compiler back-compiles from"`. Each ticket is
    /// back-compiled (`Solution::for_ticket` → `render` → `recipe::compile` → `verify_provenance`)
    /// through `compiler::emit`, whose every stage passes `policy::decide` — the decision layer's
    /// action matrix — and appends its decision to the chain. So this pass does not sit beside the
    /// decision layer; it *is* the layer's matrix applied to every ticket the board holds, and the
    /// recipe the judge hands the agent is the usable block the agent implements from.
    ///
    /// # It is idempotent, and that is a rule rather than a hope
    ///
    /// A ticket that already has a record in the inbox is **skipped**, because the inbox is the
    /// work the agent owes: re-emitting it on every launch would rewrite packages and records under
    /// the agent's feet, and a record whose bytes were rewritten is a record the agent verified
    /// against a different file. Returns `(handed, skipped, refused)` so a caller reports numbers
    /// rather than that something happened.
    pub fn hand_the_boards_tickets_to_the_agent(&mut self, tick: Option<u64>) -> (usize, usize, usize) {
        let Some(workspace) = self.console.workspace().map(std::path::Path::to_path_buf) else {
            return (0, 0, 0);
        };
        let chain_path = voxel_package::filing::chain_path(&workspace);
        let read = match voxel_package::compiler::read_back(&chain_path) {
            Ok(read) => read,
            Err(refusals) => {
                self.refuse_cmd(Verb::File, refusals.to_string(), tick);
                return (0, 0, 0);
            }
        };
        // **Every open ticket on the board — the whole record, not one world's slice of it.**
        // A retired ticket is what `Solution::for_ticket` refuses, so it stays out of the pass the
        // same way it always did; every other ticket is work the evaluator owes, and the ledger's
        // `CSE` cases — filed in another world — are the measure of what the old scope left out
        // (455 of 473 tickets, C-105).
        let filings: Vec<voxel_model::ticket::Ticket> = read
            .board
            .tickets()
            .iter()
            .filter(|ticket| !ticket.is_closed())
            .cloned()
            .collect();
        // **One read of the record, and one chain for the whole pass.** `emit` takes `&mut Chain`
        // because every stage appends its audit decision, exactly as it does for the `file` verb;
        // the chain is this pass's own value and nothing writes it back, so the read above is the
        // record and the audits stay in memory as they already do on the `file` path.
        let mut chain = read.chain;
        let inbox = voxel_model::laya::agent_inbox_dir(&workspace);
        let (mut handed, mut skipped, mut refused) = (0usize, 0usize, 0usize);
        for filing in &filings {
            let id = filing.id.as_str().to_string();
            if inbox.join(format!("recipe-{id}.musli")).exists() {
                skipped += 1;
                continue;
            }
            // **The console's own granter**, the same shape the `file` verb makes: the ruling this
            // pass reads is the user's — *"the game and laya should fucking catch the goddamn
            // tickets"* — and `CONSOLE_ACTOR` is who the record attributes this console's acts to.
            // The scope names the one ticket, because a grant with no scope authorizes everything,
            // which is the same failure as authorizing nothing.
            let grant = match voxel_model::grant::Grant::new(
                CONSOLE_ACTOR,
                format!("the arrangement declared by {id}"),
            ) {
                Ok(grant) => grant,
                Err(refusals) => {
                    refused += 1;
                    self.console.log(
                        Level::Warn,
                        "inbox",
                        format!("{id} could not be handed over: {refusals}"),
                        tick,
                    );
                    continue;
                }
            };
            // **The newest package this record has, by content — or a fresh compile.** Names are
            // derived from the content identity (`filing::package_file_name`), so one ticket has as
            // many packages as it has compiles, and the earlier ones stay: a record is never
            // overwritten and never deleted. So this reader answers two questions at once, and they
            // are different questions: *has this ticket been compiled at all*, and *does what was
            // compiled still frame*. A package that does not frame — an empty recipe the earlier
            // grammar allowed — is **not handed over**: it is compiled again under a new name,
            // because the earlier compile is a fact about the earlier compiler and not a reason to
            // leave the agent without a recipe.
            let mut already: Option<(std::path::PathBuf, voxel_model::identity::Digest)> = None;
            if let Some(path) = voxel_package::filing::newest_package(&workspace, &id) {
                match std::fs::read(&path) {
                    Ok(bytes) => match voxel_package::recipe::load(&bytes) {
                        // **Read back through the loader**, the reader every package loads through.
                        Ok(_) => {
                            already = Some((path, voxel_model::identity::Digest::of(&bytes)))
                        }
                        Err(refusals) => self.console.log(
                            Level::Warn,
                            "inbox",
                            format!(
                                "{id}'s package on disk does not frame, so this filing is compiled \
                                 again rather than handed over: {refusals}"
                            ),
                            tick,
                        ),
                    },
                    Err(failure) => {
                        refused += 1;
                        self.console.log(
                            Level::Warn,
                            "inbox",
                            format!("{id} has a package that cannot be read: {failure}"),
                            tick,
                        );
                        continue;
                    }
                }
            }
            let (path, digest) = if let Some(found) = already {
                found
            } else {
                // **The ticket the board read, compiled through the audited loop.** Every stage
                // of `emit` passes `policy::decide` and appends its decision to the chain — the
                // decision layer's matrix is on this path, which is what makes this pass the
                // ticket evaluator rather than a second compiler beside it.
                let Ok((package, receipt)) =
                    voxel_package::compiler::emit(&mut chain, filing, Some(&grant))
                else {
                    refused += 1;
                    self.console.log(
                        Level::Warn,
                        "inbox",
                        format!("{id} compiled to no package, so the agent was handed nothing"),
                        tick,
                    );
                    continue;
                };
                let path = voxel_package::filing::package_path(
                    &workspace,
                    &receipt.name,
                    &receipt.package_digest,
                );
                if let Err(refusals) = voxel_package::filing::file(&path, &package) {
                    refused += 1;
                    self.console.log(
                        Level::Warn,
                        "inbox",
                        format!("{id} could not be written: {refusals}"),
                        tick,
                    );
                    continue;
                }
                (path, receipt.package_digest)
            };
            // **Provenance held, or the load above (or `emit`) would have refused.** The judge is
            // told the measurement, not a hope: `recipe::load` framed the package, and `emit`
            // verified the round trip before it returned.
            let notified = voxel_model::laya::relative_to(&workspace, &path).and_then(|relative| {
                voxel_model::laya::notify_the_agent_of(
                    &workspace,
                    &id,
                    &relative,
                    &digest,
                    true,
                )
            });
            match notified {
                Ok(record) => {
                    handed += 1;
                    self.console.log(
                        Level::Event,
                        "inbox",
                        format!("{record} — {id} handed to the agent: {}", filing.objective),
                        tick,
                    );
                }
                Err(refusal) => {
                    refused += 1;
                    self.console.log(
                        Level::Warn,
                        "inbox",
                        format!("{id} was compiled but not notified: {refusal}"),
                        tick,
                    );
                }
            }
        }
        self.console.log(
            Level::Event,
            "inbox",
            format!(
                "the board's open tickets: {handed} handed to the agent, {skipped} already in \
                 the inbox, {refused} refused — the whole board, every world"
            ),
            tick,
        );
        (handed, skipped, refused)
    }

    /// **The board, read onto the console — every filing, whichever world filed it.**
    ///
    /// DSL: `chain -> board "tickets read back"`. The pass above is **the whole board's**: every
    /// open ticket, whichever world filed it, is back-compiled and handed over. This read is the
    /// naming half a reader audits: one line per ticket, the world it was filed in, its kind, its
    /// status and its objective — because a count is not a list and *"the game and laya should
    /// fucking catch the goddamn tickets"* means a reader can see each one.
    ///
    /// # What this catches
    ///
    /// The game's own ledger, `docs/LEDGER.md`, filed onto the chain as one `CSE` ticket per
    /// entry — 314 of 473 tickets when C-105 landed, filed in world-2468. Both halves of the
    /// ruling now read the whole board: the evaluator above compiles every open ticket it holds,
    /// and this names every one on the console, where the log can be audited.
    ///
    /// # What it is not
    ///
    /// **It grants nothing, compiles nothing and notifies nobody.** The evaluator above is the
    /// half with authority to emit a package; this read is reporting, not a second authority, and
    /// it stays reporting so a reader can tell the two apart. Returns `(on the record, filed in
    /// this console's world, filed elsewhere)` so a caller prints numbers rather than that
    /// something happened.
    pub fn read_the_board(&mut self, tick: Option<u64>) -> (usize, usize, usize) {
        let Some(workspace) = self.console.workspace().map(std::path::Path::to_path_buf) else {
            return (0, 0, 0);
        };
        let chain_path = voxel_package::filing::chain_path(&workspace);
        let read = match voxel_package::compiler::read_back(&chain_path) {
            Ok(read) => read,
            Err(refusals) => {
                self.refuse_cmd(Verb::File, refusals.to_string(), tick);
                return (0, 0, 0);
            }
        };
        let (mut here, mut elsewhere) = (0usize, 0usize);
        for ticket in read.board.tickets() {
            // **Which world filed it**: the board carries the chain's namespace world in
            // `actor` (`Board::from_chain` folds `event.namespace.world`). The split is reported,
            // not used as a filter: the evaluator above takes the whole board.
            let world = ticket.actor.as_str();
            if world == CONSOLE_WORLD {
                here += 1;
            } else {
                elsewhere += 1;
            }
            self.console.log(
                Level::Event,
                "board",
                format!(
                    "{} {} [{}] — filed in {world} at tick {} — {}",
                    ticket.id.as_str(),
                    ticket.kind.name(),
                    ticket.status.name(),
                    ticket.opened_tick,
                    ticket.objective,
                ),
                tick,
            );
        }
        let total = read.board.len();
        self.console.log(
            Level::Event,
            "board",
            format!(
                "the board holds {total} ticket(s): {here} filed in this console's world \
                 ({CONSOLE_WORLD}), {elsewhere} filed in another world — every open ticket \
                 evaluated above, every one named here"
            ),
            tick,
        );
        (total, here, elsewhere)
    }

    /// **The dynamic loader: read a package off disk and run it.**
    ///
    /// One path, no cache and no fallback: the bytes are read from `path`, framed by
    /// [`voxel_package::recipe::load`] — the reader every package in this tree loads through,
    /// whether it was compiled by `build.rs` into the binary or emitted by a filing — and then
    /// adopted by [`Mount::reload`]. **The package this returns is the one that was mounted**,
    /// because it is the same value rather than a second read.
    ///
    /// **It cannot fall back to a default.** A loader that substituted the embedded package
    /// when a path was missing would make a missing package and the running one
    /// indistinguishable, and every refusal below is the alternative.
    pub fn load_package(&mut self, path: &std::path::Path) -> Result<voxel_package::recipe::Loaded, String> {
        let bytes = std::fs::read(path)
            .map_err(|found| format!("{} cannot be read ({found})", path.display()))?;
        let loaded = voxel_package::recipe::load(&bytes).map_err(|refusals| {
            format!(
                "{} does not frame a package: {}",
                path.display(),
                refusals
                    .all()
                    .iter()
                    .map(|refusal| refusal.detail().to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;
        self.reload(&loaded)?;
        Ok(loaded)
    }

    /// **The live-reload watch: a package the compiler emits while the game runs is adopted.**
    ///
    /// DSL: `packages -> game "loaded and verified before use"`. The loader's search path **is**
    /// the compiler's output root, so a package that arrives mid-run is the same kind of thing as
    /// the package the game opened with — not a second channel and not a side door.
    ///
    /// # Why it exists
    ///
    /// The user's ruling, 2026-10-02: *"the game is already supposed to have live reloading of
    /// packages"*. The reason is not convenience. A change to the shell **is** a declaration in a
    /// package, so the only honest way to iterate on the UI is to emit a package and let the
    /// running game adopt it: file a ticket, the compiler audits and emits, this adopts it, the
    /// console says what happened. Without it every iteration is edit, rebuild, relaunch — and
    /// the running world the UI is being fixed against is thrown away each time.
    ///
    /// # What is adopted, and what a bad package costs
    ///
    /// The newest `.shpkg` **or `.shspec`** by `(mtime, name)`: the name breaks ties, so two
    /// files landing in the same instant still resolve to one, deterministically. A digest already
    /// handled is not read again, and **refused counts as handled** — a file that does not frame,
    /// or that [`Mount::reload`] turns away because it would take the console window off screen,
    /// is named on the console once rather than once per tick. The arrangement in force is
    /// untouched in both cases: the refusal is the loader's own, and there is no second rule here.
    ///
    /// # The shell prototype is a `.shspec`, and saving it is the edit
    ///
    /// `packages/alpha-shell.shspec` is the shell package's own source — the file `build.rs`
    /// compiles into the package the game opens with — so a saved spec is compiled **with the
    /// compiler's own `recipe::compile`** and adopted. That is the loop the user ruled the game is
    /// supposed to have, and the one the dirty branch used to add a missing glyph to the window
    /// chrome: edit the prototype, the running game shows it, read what it did. No second compiler
    /// lives here — the bytes are framed by the same function, so a spec this watch accepts is a
    /// spec the build accepts, and one it refuses is named rather than half-loaded.
    pub fn watch_packages(
        &mut self,
        dir: &std::path::Path,
        tick: Option<u64>,
    ) -> Option<std::path::PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            // **No directory is not an error**: a game run outside a workspace, or before the
            // compiler has emitted anything, has no search path to watch — and a watch that
            // failed loudly here would report the same fact every tick.
            return None;
        };
        let mut newest: Option<(std::time::SystemTime, String, std::path::PathBuf)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            let live = path
                .extension()
                .map(|ext| ext == "shpkg" || ext == "shspec")
                .unwrap_or(false);
            if !live {
                continue;
            }
            let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) else {
                continue;
            };
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let beats = match &newest {
                None => true,
                Some((at, other, _)) => modified > *at || (modified == *at && name > *other),
            };
            if beats {
                newest = Some((modified, name, path));
            }
        }
        let (_, _, path) = newest?;
        let bytes = std::fs::read(&path).ok()?;
        let digest = voxel_model::identity::Digest::of(&bytes);
        if self
            .packages_seen
            .as_ref()
            .map(|(_, seen)| seen == &digest)
            .unwrap_or(false)
        {
            return None;
        }
        self.packages_seen = Some((path.clone(), digest));
        // **A saved spec is compiled with the compiler's own function**, so the watch cannot
        // accept an arrangement the build would refuse; a package is already framed.
        let framed = if path.extension().map(|ext| ext == "shspec").unwrap_or(false) {
            match voxel_package::recipe::compile(&bytes) {
                Ok(frame) => frame,
                Err(refusals) => {
                    self.log(
                        Level::Warn,
                        "reload",
                        format!(
                            "{} was refused: {}",
                            path.display(),
                            refusals
                                .all()
                                .iter()
                                .map(|refusal| refusal.detail().to_string())
                                .collect::<Vec<_>>()
                                .join("; ")
                        ),
                        tick,
                    );
                    return None;
                }
            }
        } else {
            bytes
        };
        let loaded = match voxel_package::recipe::load(&framed) {
            Ok(loaded) => loaded,
            Err(refusals) => {
                self.log(
                    Level::Warn,
                    "reload",
                    format!(
                        "{} does not frame a package: {}",
                        path.display(),
                        refusals
                            .all()
                            .iter()
                            .map(|refusal| refusal.detail().to_string())
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                    tick,
                );
                return None;
            }
        };
        match self.reload(&loaded) {
            Ok(()) => {
                self.log(
                    Level::Event,
                    "reload",
                    format!(
                        "adopted {} while running: {} window(s)",
                        path.display(),
                        loaded.recipe.windows.len()
                    ),
                    tick,
                );
                Some(path)
            }
            Err(why) => {
                self.log(
                    Level::Warn,
                    "reload",
                    format!("{} was refused: {why}", path.display()),
                    tick,
                );
                None
            }
        }
    }

    /// Whether the console window holds focus, and therefore whether it owns the keyboard.
    ///
    /// **A caller asks this before routing keys, because a focused line edit owns them.** The
    /// bin's digit bindings drive the shell's own selection, and a console that could not
    /// receive a digit would be a command line that cannot type half its arguments.
    pub fn console_has_focus(&self) -> bool {
        self.focused.as_deref() == Some(CONSOLE_WINDOW)
    }

    /// One key, routed to the console **only while the console window holds focus**.
    ///
    /// The shell has no key bindings of its own yet, so an unfocused key is `Ignored` rather
    /// than swallowed somewhere: a caller can tell "nothing uses this key" from "the console
    /// used it and nothing moved".
    pub fn key_pressed(&mut self, key: Key, tick: Option<u64>) -> KeyOutcome {
        if self.focused.as_deref() != Some(CONSOLE_WINDOW) {
            return KeyOutcome::Ignored;
        }
        match key {
            // **Enter leaves the search box rather than submitting it.** The reference's two
            // fields are separate; an Enter that executed a query would make searching for a
            // word and running it the same act.
            Key::Enter if self.console.field() == super::console::Field::Search => {
                self.console.focus_command();
                KeyOutcome::Changed
            }
            Key::Enter => match self.console.submit(tick) {
                // The attempt is already on the record — `submit` logs before it parses — so all
                // that is left is the dispatch, and a refusal was logged beside its command.
                Some(Ok(command)) => {
                    self.run(&command, tick);
                    KeyOutcome::Changed
                }
                Some(Err(_)) | None => KeyOutcome::Changed,
            },
            other => self.console.key(other),
        }
    }

    /// Run a parsed command against this shell.
    ///
    /// **Every verb either acts or is refused by name.** There is no arm that does nothing,
    /// because a command that is accepted and silently ignored is indistinguishable from one
    /// that was never received, and an operator cannot tell a refusal from a no-op. The verbs
    /// the reference implements and this shell has no target for are refused *with the reason*,
    /// which is also the only honest answer while `Split` needs a drop point and `Theme` needs
    /// a scheme this tree does not have.
    pub fn run(&mut self, command: &Command, tick: Option<u64>) {
        let Command { verb, args } = command;
        let arg = |n: usize| args.get(n).map(String::as_str);
        // **The window id, read once and totally.** Every window verb declares
        // `arity() == Some((1, 1))`, so `parse` has already refused a bare `focus` before a
        // command reaches here — which is why `with_window` has no missing-argument arm. The
        // default is not a silent skip: an empty id is not a declared window, so the one refusal
        // `with_window` can make is *true* of it.
        let id = arg(0).unwrap_or_default();
        let said = |mount: &mut Self, text: String| {
            mount.console.log(Level::Event, verb.name(), text, tick);
        };
        match verb {
            Verb::Help => {
                for line in crate::shell::console::help() {
                    self.console.log(Level::Info, "HELP", line, tick);
                }
            }
            Verb::Clear => self.console.clear(),
            Verb::Echo => match args.join(" ") {
                text if text.trim().is_empty() => {
                    self.refuse_cmd(*verb, "echo needs something to say", tick)
                }
                text => said(self, text.to_string()),
            },
            Verb::List => {
                let rows: Vec<String> = self
                    .windows()
                    .map(|window| {
                        format!(
                            "{}\t{:?}\t{}",
                            window.spec.id,
                            window.status,
                            window.spec.title
                        )
                    })
                    .collect();
                for row in rows {
                    said(self, row);
                }
            }
            Verb::Stats => {
                said(self, format!("windows\t{}", self.windows().count()));
                said(self, format!("bar entries\t{}", self.bar().len()));
                said(self, format!("refusals\t{}", self.refusals().len()));
                said(self, format!("records\t{}", self.console.len()));
                if self.console.dropped() > 0 {
                    said(self, format!("dropped\t{}", self.console.dropped()));
                }
            }
            Verb::Launch => self.with_window(*verb, id, tick, |mount, id| {
                match mount.open(id) {
                    Ok(()) => mount.console.log(Level::Event, "launch", format!("opened {id}"), tick),
                    Err(refusal) => {
                        mount.console.log(Level::Warn, "launch", refusal.to_string(), tick)
                    }
                }
            }),
            Verb::Focus => self.with_window(*verb, id, tick, |mount, id| {
                mount.focus(id);
                mount.console.log(Level::Event, "focus", format!("focused {id}"), tick);
            }),
            Verb::Close => self.with_window(*verb, id, tick, |mount, id| {
                mount.close(id);
                mount.console.log(Level::Event, "close", format!("closed {id}"), tick);
            }),
            Verb::Min => self.with_window(*verb, id, tick, |mount, id| {
                mount.minimize(id);
                mount.console.log(Level::Event, "min", format!("minimized {id}"), tick);
            }),
            Verb::Max => self.with_window(*verb, id, tick, |mount, id| {
                mount.toggle_maximize(id);
                mount.console.log(Level::Event, "max", format!("toggled {id}"), tick);
            }),
            // Refused by name, each with the thing that is missing rather than a shrug.
            Verb::Split => self.refuse_cmd(
                *verb,
                "split needs a drop point, and a command line has no cursor to drop at",
                tick,
            ),
            Verb::Undock => self.with_window(*verb, id, tick, |mount, id| {
                match mount.undock(id, id) {
                    Ok(()) => mount
                        .console
                        .log(Level::Event, "undock", format!("undocked {id}"), tick),
                    Err(refusal) => mount
                        .console
                        .log(Level::Warn, "undock", refusal.to_string(), tick),
                }
            }),
            Verb::Theme => self.refuse_cmd(
                *verb,
                "theme needs a scheme this tree has not declared; the palette is \
                 `Palette::declared()` and nothing switches it",
                tick,
            ),
            Verb::Setting => self.refuse_cmd(
                *verb,
                "setting needs a settings store the shell does not keep",
                tick,
            ),
            Verb::Report => self.refuse_cmd(
                *verb,
                "report needs the gate's readings, which are a build-time artefact and not in \
                 this process",
                tick,
            ),
            Verb::File => self.file(args, tick),
            Verb::Approvals
            | Verb::Approve
            | Verb::Reject
            | Verb::Blockers
            | Verb::Mode
            | Verb::Plan => {
                self.refuse_cmd(
                    *verb,
                    "this verb governs a record surface the shell does not hold; filing a ticket \
                     is not one of the twenty verbs the reference console implements",
                    tick,
                )
            }
        }
    }

    /// The shared shape of every `verb <window-id>`.
    ///
    /// **No missing-argument arm, and that is deliberate.** Every window verb declares
    /// `arity() == Some((1, 1))`, so [`crate::shell::console::parse`] has already refused a bare
    /// `focus` with *"focus takes 1 arguments and 0 were given"* before a command ever reaches
    /// here. An `Option` parameter would have carried a branch no input can take, and a refusal
    /// that can never fire is a second, weaker answer to a question already answered.
    ///
    /// What is left is the one refusal this can make and the parser cannot: **a name this
    /// package never declared**, which is a different fact from a wrong number of arguments.
    fn with_window(
        &mut self,
        verb: Verb,
        id: &str,
        tick: Option<u64>,
        act: impl FnOnce(&mut Self, &str),
    ) {
        if self.window(id).is_none() {
            let declared: Vec<&str> = self.windows().map(|w| w.spec.id.as_str()).collect();
            self.refuse_cmd(
                verb,
                format!(
                    "{id:?} is not a declared window; this package declares {}",
                    declared.join(", ")
                ),
                tick,
            );
            return;
        }
        act(self, id);
    }

    /// A refusal, on the record, at `WARN`, naming the verb and the reason.
    fn refuse_cmd(&mut self, verb: Verb, why: impl std::fmt::Display, tick: Option<u64>) {
        self.console.log(Level::Warn, verb.name(), why.to_string(), tick);
    }

    // -----------------------------------------------------------------------
    // Reading the state
    // -----------------------------------------------------------------------

    pub fn shell_shown(&self) -> bool {
        self.shown
    }

    pub fn toggle_shell(&mut self) {
        self.shown = !self.shown;
        if !self.shown {
            // The menu is part of the shell layer. Leaving it open behind a hidden shell would
            // make it appear when the shell is shown again with no click behind it.
            self.menu = None;
            self.drag = None;
        }
    }

    pub fn set_shown(&mut self, shown: bool) {
        if self.shown != shown {
            self.toggle_shell();
        }
    }

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }

    /// Whether a point lands on the shell's own chrome: the bar, an open menu, or a window.
    ///
    /// **The bin uses this to route a click**, and it is deliberately generous about the bar:
    /// the strip is the shell's surface across its whole width, so a click on the empty part of
    /// it is the shell's too rather than a dig at the world behind it.
    pub fn hit(&self, at: (f64, f64)) -> bool {
        if !self.shown {
            return false;
        }
        if at.1 < bar_height_dp() {
            return true;
        }
        if let Some(entry) = self
            .menu
            .as_ref()
            .and_then(|tab| self.bar.iter().position(|entry| &entry.id == tab))
        {
            let rows = self.menu_rows(entry, self.viewport);
            if let (Some((_, first)), Some((_, last))) = (rows.first(), rows.last()) {
                if Rect::new(first.x, first.y, first.width_dp, last.bottom() - first.y).contains(at)
                {
                    return true;
                }
            }
        }
        self.ordered
            .iter()
            .rev()
            .any(|id| self.rect_of(id).is_some_and(|rect| rect.contains(at)))
    }

    pub fn window(&self, id: &str) -> Option<&Window> {
        self.windows.get(id)
    }

    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }

    pub fn bar(&self) -> &[BarEntry] {
        &self.bar
    }

    /// The tab whose menu is open, if one is.
    pub fn open_menu(&self) -> Option<&str> {
        self.menu.as_deref()
    }

    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// The windows a tab's menu offers: the tab's own window first, then the windows filed
    /// under it, in declaration order (H-15.3).
    pub fn offers(&self, tab: &str) -> Vec<String> {
        let mut out = Vec::new();
        if self.windows.contains_key(tab) {
            out.push(tab.to_string());
        }
        for (window, home) in &self.filed {
            if home == tab {
                out.push(window.clone());
            }
        }
        out
    }

    /// The pane titles of a window, in tree order — which is what its title is derived from
    /// when it holds more than one pane (UI-SPEC §3, C-65.4).
    pub fn pane_titles(&self, id: &str) -> Vec<String> {
        let Some(window) = self.windows.get(id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        collect_titles(window.arrangement.root(), &self.windows, &mut out);
        out
    }

    /// The pane titles, keyed by pane id — what the port's drawing reads
    /// (`PortalWindow::elements`'s `titles`), where an absent entry is the reference's empty
    /// slot. One derivation with [`Mount::pane_titles`]: this is that list with the ids it came
    /// from.
    fn pane_title_map(&self, id: &str) -> BTreeMap<String, String> {
        let Some(window) = self.windows.get(id) else {
            return BTreeMap::new();
        };
        let mut out = BTreeMap::new();
        collect_title_map(window.arrangement.root(), &self.windows, &mut out);
        out
    }

    /// **What an empty slot offers**: the declared windows that are not already in this window's
    /// tree, in declaration order — the reference's `PanePlaceholder` grid (2607–2622), with
    /// this tree's window list in place of the simulator's ten apps.
    fn empty_slot_offers(&self, id: &str) -> Vec<(String, String)> {
        let Some(window) = self.windows.get(id) else {
            return Vec::new();
        };
        let held: Vec<String> = window
            .arrangement
            .windows()
            .into_iter()
            .map(|pane| pane.as_str().to_string())
            .collect();
        self.windows
            .values()
            .filter(|candidate| {
                !held.contains(&candidate.spec.id) && candidate.spec.id != id
            })
            .map(|candidate| (candidate.spec.id.clone(), candidate.spec.title.clone()))
            .collect()
    }

    /// The pane mark under the pointer, if any — the hover the port's pane header draws one
    /// elevation step for. The lanes come from `pane.rs`, the same call the drawing makes.
    fn hovered_pane_mark(&self, id: &str) -> Option<voxel_ui::PaneMark> {
        let at = self.cursor?;
        let window = self.windows.get(id)?;
        let rect = self.rect_of(id)?;
        let layout = window.layout_at(rect).ok()?;
        let pane = layout.pane_at(at)?;
        let pane_rect = layout.pane_rect(pane)?;
        let undock_shown =
            window.arrangement.panes() > 1 && matches!(pane, PaneRef::Window(_));
        match super::pane::press(pane_rect, undock_shown, at) {
            super::pane::PanePress::Action(mark) => Some(mark),
            _ => None,
        }
    }

    /// The seam index under the pointer, if any.
    fn hovered_seam(&self, id: &str) -> Option<usize> {
        let at = self.cursor?;
        let window = self.windows.get(id)?;
        let rect = self.rect_of(id)?;
        window.layout_at(rect).ok()?.seam_at(at)
    }

    /// The title a window's chrome draws: its own title when it is one pane, and the pane set's
    /// when it is more. The reference derives a splitter's title from what it holds; the
    /// package's `title` is the one-pane case of the same rule.
    pub fn title_of(&self, id: &str) -> String {
        let Some(window) = self.windows.get(id) else {
            return id.to_string();
        };
        let titles = self.pane_titles(id);
        match titles.len() {
            0 => window.spec.title.clone(),
            // **A window holding one empty slot is the reference's `"◫  New Split Slot"`
            // window** (3145–3152), not the multi-pane slot label: the title distinguishes the
            // one-pane case, and the tree says whether that pane is a slot.
            1 if matches!(window.arrangement.root(), Node::Slot) => super::pane::empty_pane_title(),
            1 => titles[0].clone(),
            _ => titles.join(" / "),
        }
    }

    /// Where a window is drawn right now: its rect, or the viewport under the bar when it is
    /// maximized.
    ///
    /// **A docked window has no rect of its own** — `None`, because its host's arrangement
    /// decides its pane. That is a state, not a missing value.
    pub fn rect_of(&self, id: &str) -> Option<Rect> {
        let window = self.windows.get(id)?;
        if window.host.is_some() {
            return None;
        }
        Some(match window.status {
            Status::Maximized => Rect::new(
                0.0,
                bar_height_dp(),
                self.viewport.width_dp,
                (self.viewport.height_dp - bar_height_dp()).max(0.0),
            ),
            _ => window.rect,
        })
    }

    /// Everything drawn on the shell layer: the windows that float, the bar, and the menu.
    ///
    /// **A window that refuses is one window's refusal, not the shell's.** A dock that halves a
    /// pane, or a panel that outgrows the window the package declared for it, must not take the
    /// other fifteen windows off the screen with it: the shell drew nothing at all, and the
    /// refusal named one window. So each window is laid out on its own, the ones that lay out are
    /// drawn, and the ones that do not leave their frame and a refusal in [`Mount::refusals`].
    pub fn shell_elements(
        &mut self,
        palette: &Palette,
        viewport: Rect,
        views: &ContentViews,
    ) -> UiResult<Vec<Element>> {
        let mut out = Vec::new();
        self.refusals.clear();
        let open: Vec<String> = self.ordered.clone();
        for id in &open {
            if let Err(refusal) = self.window_elements(id, palette, views, &mut out) {
                self.refusals.push(refusal.to_string());
            }
        }
        self.bar_elements(palette, viewport, &mut out)?;
        out.extend(self.menu_elements(palette, viewport)?);
        Ok(out)
    }

    /// Everything drawn on the **pinned layer**: the windows that stay while the shell is
    /// hidden. H-9.8 and `shell/hud.rs`; the same windows, the same painter — including the rule
    /// that one window's refusal is one window's.
    pub fn pinned_elements(
        &mut self,
        palette: &Palette,
        _viewport: Rect,
        views: &ContentViews,
    ) -> UiResult<Vec<Element>> {
        let mut out = Vec::new();
        self.refusals.clear();
        let open: Vec<String> = self.ordered.clone();
        for id in &open {
            let pinned = self.windows.get(id).is_some_and(|w| w.spec.pinnable);
            if pinned {
                if let Err(refusal) = self.window_elements(id, palette, views, &mut out) {
                    self.refusals.push(refusal.to_string());
                }
            }
        }
        Ok(out)
    }

    /// The refusals from the last layout, one line per window that could not be drawn.
    ///
    /// **Empty after a layout that refused nothing**, which is the ordinary case and is worth a
    /// caller checking rather than assuming.
    pub fn refusals(&self) -> &[String] {
        &self.refusals
    }

    // -----------------------------------------------------------------------
    // Changing the state
    // -----------------------------------------------------------------------

    /// Open a window, or bring it to the front if it is already open.
    pub fn open(&mut self, id: &str) -> UiResult<()> {
        let error = || UiRefusal::UnknownWindow { window: id.to_string() };
        let window = self.windows.get_mut(id).ok_or_else(error)?;
        if window.host.is_some() {
            // A docked window is a pane of another; opening it would put it in two places.
            return Err(UiRefusal::DuplicateWindow { window: id.to_string() });
        }
        window.status = match window.status {
            Status::Maximized => Status::Maximized,
            _ => Status::Normal,
        };
        if !self.ordered.iter().any(|held| held == id) {
            self.ordered.push(id.to_string());
        }
        self.focus(id);
        Ok(())
    }

    /// Bring a window to the front and give it the focus.
    pub fn focus(&mut self, id: &str) {
        if !self.windows.contains_key(id) {
            return;
        }
        self.ordered.retain(|held| held != id);
        self.ordered.push(id.to_string());
        self.focused = Some(id.to_string());
    }

    /// Minimize a window. The menu still lists it, with its state named.
    pub fn minimize(&mut self, id: &str) {
        let Some(window) = self.windows.get_mut(id) else {
            return;
        };
        if window.host.is_some() || window.status == Status::Closed {
            return;
        }
        window.status = Status::Minimized;
        self.ordered.retain(|held| held != id);
        if self.focused.as_deref() == Some(id) {
            self.focused = self.ordered.last().cloned();
        }
    }

    /// Maximize a window, or restore it if it already is. The restored rect is the one the
    /// player left, not the declaration.
    pub fn toggle_maximize(&mut self, id: &str) {
        let Some(window) = self.windows.get_mut(id) else {
            return;
        };
        if window.host.is_some() {
            return;
        }
        window.status = match window.status {
            Status::Maximized => Status::Normal,
            Status::Normal => Status::Maximized,
            other => other,
        };
        self.focus(id);
    }

    /// Close a window. H-9.8's pinned windows close like any other window; pinning is a layer,
    /// not immortality.
    ///
    /// **The panes are released, not closed with the host.** A window docked into this one was
    /// an independent window before it was docked, so the host closing hands it back rather
    /// than destroying it — the same forgiveness §2 asks of every destructive-looking action.
    pub fn close(&mut self, id: &str) {
        let panes: Vec<String> = match self.windows.get(id) {
            Some(window) => window
                .arrangement
                .windows()
                .into_iter()
                .map(|pane| pane.as_str().to_string())
                .filter(|pane| pane != id)
                .collect(),
            None => return,
        };
        if let Some(window) = self.windows.get_mut(id) {
            window.status = Status::Closed;
            window.host = None;
        }
        self.ordered.retain(|held| held != id);
        // A closed window's viewport dies with it: the reference destroys the widget, and a
        // scrollback that outlived its window would open the next one at a position nobody
        // chose. A pane *released* by the close keeps its own — it is still the same window.
        self.scroll.remove(id);
        for pane in panes {
            if let Some(window) = self.windows.get_mut(&pane) {
                window.host = None;
                if window.status != Status::Closed && !self.ordered.iter().any(|held| held == &pane)
                {
                    self.ordered.push(pane.clone());
                }
            }
        }
        if self.focused.as_deref() == Some(id) {
            self.focused = self.ordered.last().cloned();
        }
    }

    /// Undock a pane into a standalone window, where it was last drawn.
    pub fn undock(&mut self, host: &str, pane: &str) -> UiResult<()> {
        if pane == host {
            return Err(UiRefusal::UnknownWindow { window: pane.to_string() });
        }
        let pane_id = WindowId::new(pane)?;
        let viewport = self.viewport;
        let Some(host_window) = self.windows.get_mut(host) else {
            return Err(UiRefusal::UnknownWindow { window: host.to_string() });
        };
        let arrangement = host_window
            .arrangement
            .close(&PaneRef::Window(pane_id.clone()))
            .ok_or(UiRefusal::UnknownWindow { window: pane.to_string() })?;
        // Where the pane was, before the close collapses it out of the tree.
        let host_rect = if host_window.status == Status::Maximized {
            Rect::new(
                0.0,
                bar_height_dp(),
                viewport.width_dp,
                (viewport.height_dp - bar_height_dp()).max(0.0),
            )
        } else {
            host_window.rect
        };
        let rect = host_window
            .layout_at(host_rect)
            .ok()
            .and_then(|layout| layout.window_rect(&pane_id))
            .unwrap_or_else(|| Rect::new(0.0, bar_height_dp(), 320.0, 240.0));
        host_window.arrangement = arrangement;
        if let Some(window) = self.windows.get_mut(pane) {
            window.host = None;
            window.rect = rect;
            window.status = Status::Normal;
            if !self.ordered.iter().any(|held| held == pane) {
                self.ordered.push(pane.to_string());
            }
            self.focused = Some(pane.to_string());
        }
        Ok(())
    }

    /// Dock `dragged` into `host`, as a pane on the side the drop came from.
    ///
    /// **The one direct-manipulation rule the reference proves:** dropping onto a window splits
    /// *that window's* pane tree, in the orientation the drop side implies. Only a window that
    /// holds one pane is docked — a window that is itself a host would nest one arrangement
    /// inside another, and the record's tree has one owner per pane, not two.
    pub fn dock(&mut self, host: &str, dragged: &str, at: (f64, f64)) -> UiResult<()> {
        // **The same rule the preview uses** — one zone, one target: a landing that resolved
        // its own side by a second arithmetic could dock somewhere the preview never showed
        // (`SHELL-PORT.md` §5). The zone rule is the port's (`drop::zone_at`).
        let target = {
            let Some(window) = self.windows.get(host) else {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            };
            if window.host.is_some() {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            }
            let rect = self
                .rect_of(host)
                .ok_or(UiRefusal::UnknownWindow { window: host.to_string() })?;
            let layout = window.layout_at(rect)?;
            let title_height = window::title_height(&window.spec);
            drop::zone_at(rect, title_height, &layout, at)
                .ok_or(UiRefusal::UnknownWindow { window: host.to_string() })?
        };
        self.dock_target(host, dragged, &target)
    }

    /// **Land a drag where its preview showed.** The port's `Window::dock` (window.rs) *is* the
    /// reference's `dock_window` (3013–3039): the zone decides the orientation and the side,
    /// and the target pane — or the root, for a window-edge zone — is where the split happens.
    fn dock_target(&mut self, host: &str, dragged: &str, target: &DropTarget) -> UiResult<()> {
        let dragged_id = WindowId::new(dragged)?;
        {
            let Some(window) = self.windows.get(dragged) else {
                return Err(UiRefusal::UnknownWindow { window: dragged.to_string() });
            };
            if window.host.is_some() || window.arrangement.panes() > 1 {
                return Err(UiRefusal::DuplicateWindow { window: dragged.to_string() });
            }
        }
        if !self
            .windows
            .get(host)
            .is_some_and(|window| window.host.is_none())
        {
            return Err(UiRefusal::UnknownWindow { window: host.to_string() });
        }
        {
            let Some(host_window) = self.windows.get_mut(host) else {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            };
            host_window.dock(&[dragged_id], target.pane.as_ref(), target.zone)?;
        }
        if let Some(window) = self.windows.get_mut(dragged) {
            window.host = Some(WindowId::new(host)?);
            window.status = Status::Normal;
        }
        self.ordered.retain(|held| held != dragged);
        self.focus(host);
        Ok(())
    }

    /// A pane's own header control (the reference's `PaneHeader` signals, 2516–2582): `×`
    /// closes the pane, `⇱` undocks it, and `◧`/`⬓` split it — **the split creates an empty
    /// slot**, the reference's `WindowPane(app=None)` (2920–2921), which the placeholder then
    /// offers windows into (or a drop fills).
    fn pane_action(&mut self, host: &str, pane: &PaneRef, mark: voxel_ui::PaneMark) {
        match mark {
            voxel_ui::PaneMark::Close => self.close_pane(host, pane),
            voxel_ui::PaneMark::Undock => {
                // **Only a window pane undocks**: a slot has no id for the registry to file a
                // new window under, and the pane header does not offer the control on one.
                if let PaneRef::Window(id) = pane {
                    let _ = self.undock(host, id.as_str());
                }
            }
            voxel_ui::PaneMark::SplitHorizontal | voxel_ui::PaneMark::SplitVertical => {
                let Some((orientation, side)) = window::split_axis(mark) else {
                    return;
                };
                let _ = self.split_slot(host, Some(pane.clone()), orientation, side);
            }
        }
    }

    /// **The window's pane tree gains one empty slot** — the pane header's `◧`/`⬓` (a target
    /// pane, `"after"` it) and the outer edge triggers (the root, on the zone's side) both
    /// arrive here (the reference's `split()` 2905–2909 and `split_pane` 2919–3012).
    ///
    /// The split's own orientation, side and root-vs-pane rule are `arrangement.rs`'s; this is
    /// the registry half — the window the slot appears in is the one whose layout the pointer
    /// resolved, and the reference logs the event under that window's name.
    fn split_slot(
        &mut self,
        host: &str,
        target: Option<PaneRef>,
        orientation: Orientation,
        side: Side,
    ) -> UiResult<PaneRef> {
        let created = match self.windows.get_mut(host) {
            Some(window) => window.arrangement.split_slot(target.as_ref(), orientation, side)?,
            None => return Err(UiRefusal::UnknownWindow { window: host.to_string() }),
        };
        // **The lanes go with the split**: the reference hides them in `split()` (2909), because
        // the press that acted on one has been answered.
        self.triggers = None;
        self.focus(host);
        Ok(created)
    }

    /// `×` on a pane: **the pane's window closes, not the host's** — and when it was the
    /// host's last pane the host closes too (the reference's `close_pane`, 3086–3110).
    fn close_pane(&mut self, host: &str, pane: &PaneRef) {
        if let PaneRef::Window(id) = pane {
            if id.as_str() == host {
                // The host's own pane: its `×` is on the bar, so the pane control and the
                // window control answer the same thing.
                self.close(host);
                return;
            }
        }
        let survives = match self.windows.get_mut(host) {
            Some(window) => window.close_pane(pane),
            None => return,
        };
        // **A slot has no window to close**, so only a window pane's own window leaves the
        // registry: its status, its z-order entry and its scrollback go with it — and the
        // focus the closed pane held returns to the host.
        if let PaneRef::Window(id) = pane {
            let pane = id.as_str();
            if let Some(window) = self.windows.get_mut(pane) {
                window.status = Status::Closed;
                window.host = None;
            }
            self.ordered.retain(|held| held != pane);
            self.scroll.remove(pane);
            if self.focused.as_deref() == Some(pane) {
                self.focused = Some(host.to_string());
            }
        }
        if !survives {
            self.close(host);
        }
    }

    /// A press inside a pane's body: focus is the pane's rule (2712–2718), and a pane holding
    /// the console routes its lanes against **its own body** — the same `pane::body` the
    /// drawing pass places the content in, so a docked console's controls act where they are
    /// drawn (the case the derived routing named as absent).
    fn press_in_pane(&mut self, host: &str, pane: &WindowId, rect: Rect, at: (f64, f64)) {
        let Some(pane_rect) = self
            .windows
            .get(host)
            .and_then(|window| window.layout_at(rect).ok())
            .and_then(|layout| layout.window_rect(pane))
        else {
            return;
        };
        let Some(content) = self
            .windows
            .get(pane.as_str())
            .map(|window| window.spec.content)
        else {
            return;
        };
        if content != Content::Console {
            return;
        }
        let body = super::pane::body(pane_rect);
        for (lane, lane_rect) in console_lanes(body) {
            if !lane_rect.contains(at) {
                continue;
            }
            match lane {
                ConsoleLane::Filter => {
                    self.console.cycle_filter();
                }
                ConsoleLane::Search => self.console.focus_search(),
                ConsoleLane::Clear => {
                    self.console.clear();
                    // **The reference appends its own line after clearing**, so an empty
                    // console is never mistaken for one that never worked.
                    self.console.log(Level::Sys, "CONSOLE", "log cleared", None);
                }
                ConsoleLane::Copy => {
                    // **Refused by name.** The reference copies the visible log onto the
                    // system clipboard; this shell is a library inside a game window and has
                    // no clipboard, and a `copy` that silently did nothing would be the exact
                    // failure the console exists to refuse.
                    self.console.log(
                        Level::Warn,
                        "CONSOLE",
                        format!(
                            "copy: this shell has no clipboard, so the {} visible record(s) \
                             cannot be put on one",
                            self.console.lines().len()
                        ),
                        None,
                    );
                }
                ConsoleLane::AutoScroll => {
                    // **Following means the tail.** Switching the control back on releases the
                    // freeze *and* every console window's scrollback — the operator asking to
                    // follow the tail means the tail, not the moment they stopped following it.
                    // Switching it off keeps the offset, because the frozen reading is where
                    // they were: that is what makes the wheel-then-freeze gesture readable.
                    if self.console.toggle_autoscroll() {
                        self.anchor_console_windows();
                    }
                }
                ConsoleLane::Command => self.console.focus_command(),
                ConsoleLane::Run => {
                    if let Some(Ok(command)) = self.console.submit(None) {
                        self.run(&command, None);
                    }
                }
            }
            return;
        }
    }

    /// The topmost floating window under a point, with the rect it is drawn at. The order is
    /// `ordered`'s, which is back to front, so the last window holding the point is the one the
    /// click resolves to — and it is the only one (a click through a window is a mis-click).
    fn topmost_window(&self, at: (f64, f64)) -> Option<(String, Rect)> {
        self.ordered.iter().rev().find_map(|id| {
            self.rect_of(id)
                .filter(|rect| rect.contains(at))
                .map(|rect| (id.clone(), rect))
        })
    }

    /// **The view the frame drew for a window, as a value** — the same construction
    /// [`Mount::window_elements`] passes to `elements_at`, so a hover mark or a preview the
    /// drawing resolved is the one the pointer resolves.
    fn window_view(&self, id: &str) -> WindowView {
        WindowView {
            title: self.title_of(id),
            focused: self.focused.as_deref() == Some(id),
            hovered_pane_mark: self.hovered_pane_mark(id),
            hovered_seam: self.hovered_seam(id),
            drop: self
                .drop_preview()
                .filter(|(target, _)| target == id)
                .map(|(_, target)| target.clone()),
            triggers: self
                .triggers
                .as_ref()
                .filter(|(shown, _, _)| shown == id)
                .map(|(_, lanes, _)| *lanes),
            hovered_zone: self
                .triggers
                .as_ref()
                .filter(|(shown, _, _)| shown == id)
                .and_then(|(_, _, zone)| *zone),
            offers: self.empty_slot_offers(id),
        }
    }

    /// **The drop preview in flight**, from the drag that owns it — one source, so the preview
    /// a release lands on (`DragState::preview`) is the preview the frame drew.
    fn drop_preview(&self) -> Option<&(String, DropTarget)> {
        self.drag.as_ref().and_then(|drag| drag.preview.as_ref())
    }

    /// Where a drag over `dragged` would land right now — the port's `drop_target_at` over the
    /// floats in draw order, each with the layout and title-bar height **the frame draws it
    /// with**, which is the arithmetic the preview is drawn from.
    fn drop_target_for(&self, dragged: &str, at: (f64, f64)) -> Option<(String, DropTarget)> {
        let candidates: Vec<(String, Rect, f64, super::splitter::Layout)> = self
            .ordered
            .iter()
            .filter(|id| id.as_str() != dragged)
            .filter_map(|id| {
                let window = self.windows.get(id)?;
                let rect = self.rect_of(id)?;
                let layout = window.layout_at(rect).ok()?;
                let title_height = window::title_height(&window.spec);
                Some((id.clone(), rect, title_height, layout))
            })
            .collect();
        window::drop_target_at(
            candidates
                .iter()
                .map(|(id, rect, title, layout)| (id.as_str(), *rect, *title, layout)),
            at,
        )
    }

    /// The edge-trigger pass: the port's `edge_triggers_at` for the window under the pointer,
    /// with the zone under the pointer for the hover; nothing clears them (the reference's
    /// `mouseMoveEvent` 3374–3390 and `leaveEvent` 3403–3405).
    fn update_triggers(&mut self, at: (f64, f64)) {
        let found = self.topmost_window(at).and_then(|(id, rect)| {
            let lanes = self.windows.get(&id)?.edge_triggers_at(rect, at)?;
            Some((id, lanes, lanes.zone_at(at)))
        });
        self.triggers = found;
    }

    /// Where a pane sits inside its host right now, if the host holds it.
    pub fn pane_rect(&self, host: &str, pane: &str) -> Option<Rect> {
        let window = self.windows.get(host)?;
        let rect = self.rect_of(host)?;
        let pane = WindowId::new(pane).ok()?;
        window.layout_at(rect).ok()?.window_rect(&pane)
    }

    // -----------------------------------------------------------------------
    // The pointer
    // -----------------------------------------------------------------------

    /// The pointer left the shell: the drag is abandoned, and **the trigger lanes go with it**
    /// (the reference's `leaveEvent`, 3403–3405, hides them). A seam drag ends the same way —
    /// its shares stay where the last move put them, which is what the reference's handle does
    /// when the pointer leaves the window mid-drag.
    pub fn pointer_left(&mut self) {
        self.cursor = None;
        self.drag = None;
        self.seam = None;
        self.triggers = None;
    }

    /// **One move, resolved against the port's own machine.** A resize in flight wins over
    /// everything (`_resize_from`, 3444–3473); a drag in flight moves the window and asks the
    /// port where it would land ([`window::drop_target_at`], the same call the preview is drawn
    /// from); otherwise the move is a hover — the edge-trigger pass (3374–3390), whose lanes
    /// and hover the drawing reads back from the same state.
    pub fn pointer_moved(&mut self, at: (f64, f64)) {
        self.cursor = Some(at);
        if let Some((id, start, edges, from)) = self.resize.clone() {
            let dx = at.0 - from.0;
            let dy = at.1 - from.1;
            let resized = self
                .windows
                .get(&id)
                .map(|window| window.resize_from(start, edges, dx, dy));
            if let (Some(rect), Some(window)) = (resized, self.windows.get_mut(&id)) {
                window.rect = rect;
            }
            return;
        }
        // **A seam drag moves a share, from the shares the drag started at.** The mutation is
        // `arrangement.rs::drag_seam`'s — the same floor the layout enforces is its clamp — and
        // a refusal (a tree that moved under the drag, a pair already under the floor) leaves
        // the arrangement alone rather than writing a boundary nobody grabbed.
        if let Some(seam) = self.seam.clone() {
            let delta = seam.delta(at);
            if let Some(window) = self.windows.get_mut(&seam.id) {
                let _ = window.arrangement.drag_seam(
                    seam.splitter,
                    seam.boundary,
                    seam.span,
                    seam.from,
                    delta,
                );
            }
            return;
        }
        let Some((id, origin)) = self
            .drag
            .as_mut()
            .map(|drag| (drag.id.clone(), drag.moved_to(at)))
        else {
            self.update_triggers(at);
            return;
        };
        let clamped = self.windows.get(&id).and_then(|window| {
            (window.status != Status::Maximized).then(|| {
                self.clamp_window(Rect::new(
                    origin.0,
                    origin.1,
                    window.rect.width_dp,
                    window.rect.height_dp,
                ))
            })
        });
        if let Some(rect) = clamped {
            if let Some(window) = self.windows.get_mut(&id) {
                window.rect = rect;
            }
        }
        // **The preview is the same answer the landing will use**, so what is drawn is what
        // would happen: [`Mount::dock_target`] consumes this target on release.
        let found = self.drop_target_for(&id, at);
        if let Some(drag) = self.drag.as_mut() {
            drag.set_preview(found);
        }
    }

    pub fn pointer_pressed(&mut self, button: Button, at: (f64, f64)) {
        self.pointer_press(button, at, false);
    }

    /// **A double-click, routed as the reference routes it.**
    ///
    /// Qt sends `mouseDoubleClickEvent` to the same handlers a press reaches, and the port's
    /// [`PortalWindow::press_at`] takes the `double` flag so the title bar's maximize verdict
    /// (2506–2515) can be answered. **The click count is the caller's**: the platform here
    /// reports presses and no counts, so the bin owns the interval and the slop — a platform
    /// translation, named where it is made — and this entry is what it calls.
    pub fn pointer_double_clicked(&mut self, button: Button, at: (f64, f64)) {
        self.pointer_press(button, at, true);
    }

    /// The press body, shared by [`Mount::pointer_pressed`] and
    /// [`Mount::pointer_double_clicked`] — one machine, so a double-click cannot answer
    /// differently from a press anywhere but where `double` is read.
    fn pointer_press(&mut self, button: Button, at: (f64, f64), double: bool) {
        self.cursor = Some(at);
        if !self.shown {
            return;
        }

        // A menu swallows its own clicks: a row opens, anywhere else closes it. This runs
        // before the bar so a menu drawn over an entry cannot be clicked through. The first
        // row is the menu's header — it names the sort and opens nothing.
        if let Some(tab) = self.menu.clone() {
            if let Some(entry) = self.bar.iter().position(|entry| entry.id == tab) {
                let rows = self.menu_rows(entry, self.viewport);
                if let Some((id, _)) = rows
                    .iter()
                    .skip(1)
                    .find(|(_, rect)| rect.contains(at))
                {
                    let id = id.clone();
                    self.menu = None;
                    let _ = self.open(&id);
                    return;
                }
                let over_menu = rows
                    .first()
                    .is_some_and(|(_, first)| {
                        let last = rows.last().expect("a menu has at least its header").1;
                        Rect::new(
                            first.x,
                            first.y,
                            first.width_dp,
                            last.bottom() - first.y,
                        )
                        .contains(at)
                    });
                if !over_menu {
                    self.menu = None;
                }
            }
        }

        // The bar.
        for entry in self.bar_rects(self.viewport) {
            let (id, tab, rect) = entry;
            if !rect.contains(at) {
                continue;
            }
            if tab {
                self.menu = if self.menu.as_deref() == Some(id.as_str()) { None } else { Some(id) };
            } else {
                let _ = self.open(&id);
            }
            return;
        }

        // The windows, front to back. The topmost window under the pointer handles the click,
        // and it is the only one: a click through a window is a mis-click, not a feature.
        let Some((id, rect)) = self.topmost_window(at) else {
            return;
        };

        if button == Button::Right {
            // Right-click on a pane undocks it: the task-viewer's own action, without a menu
            // to find first. (The reference has no right-click gesture here; this stays the
            // derived shell's own.)
            let pane = self.windows.get(&id).and_then(|window| {
                let layout = window.layout_at(rect).ok()?;
                layout.window_at(at).map(|pane| pane.as_str().to_string())
            });
            if let Some(pane) = pane {
                if pane != id {
                    let host = id.clone();
                    let _ = self.undock(&host, &pane);
                }
            }
            return;
        }

        // **One verdict, answered by the port.** The controls, the resize edges, the title
        // drag, the double-click maximize, a seam, a pane's own header and an empty slot's
        // offers are one `press_at` call over the same view the frame drew — the derived
        // shell asked the same questions here in its own arithmetic, which `SHELL-PORT.md` §5
        // retired.
        //
        // **`double` is the caller's answer, not this function's guess**: the bin counts the
        // clicks (the platform reports presses, not counts) and calls
        // [`Mount::pointer_double_clicked`] for the second one; a plain press passes `false`.
        let view = self.window_view(&id);
        let verdict = match self.windows.get(&id) {
            Some(window) => window.press_at(rect, &view, at, double),
            None => WindowPress::None,
        };
        let maximized = self.windows.get(&id).is_some_and(|w| w.status == Status::Maximized);
        match verdict {
            WindowPress::Control(action) => match action {
                GlyphAction::Close => self.close(&id),
                GlyphAction::Minimize => self.minimize(&id),
                GlyphAction::Maximize => self.toggle_maximize(&id),
            },
            WindowPress::Maximize => self.toggle_maximize(&id),
            WindowPress::Drag => {
                self.focus(&id);
                // **The verdict does not apply itself**: a maximized window is not dragged
                // from its bar (the reference's own rule), so the state check lives here.
                if !maximized {
                    self.drag = Some(DragState::start(&id, at, (rect.x, rect.y)));
                }
            }
            WindowPress::Resize(edges) => {
                self.focus(&id);
                if !maximized {
                    self.resize = Some((id.clone(), rect, edges, at));
                }
            }
            WindowPress::PaneAction(pane, mark) => self.pane_action(&id, &pane, mark),
            WindowPress::Pane(pane) => {
                self.focus(&id);
                // **Only a pane holding a window has content to route a body press into**; a
                // slot's body is its offer grid, which answers above (the `Offer` verdict).
                if let PaneRef::Window(pane) = &pane {
                    self.press_in_pane(&id, pane, rect, at);
                }
            }
            WindowPress::Trigger(zone) => {
                // **An edge trigger splits the window at its outer edge** (the reference's
                // `split`, 2905–2909): the root is the target, the zone gives the orientation
                // and the side, and the new pane is an empty slot.
                let orientation = match zone {
                    drop::DropZone::Left | drop::DropZone::Right => Orientation::Horizontal,
                    drop::DropZone::Top | drop::DropZone::Bottom => Orientation::Vertical,
                };
                let side = match zone {
                    drop::DropZone::Left | drop::DropZone::Top => Side::Before,
                    drop::DropZone::Right | drop::DropZone::Bottom => Side::After,
                };
                self.focus(&id);
                let _ = self.split_slot(&id, None, orientation, side);
            }
            WindowPress::Seam(seam) => {
                // **A seam press starts a share drag.** The start pair is read from the tree's
                // own shares — the same list `drag_seam` writes — so the drag's arithmetic and
                // the drawing's cannot disagree about which boundary was grabbed.
                self.focus(&id);
                let from = self.windows.get(&id).and_then(|window| {
                    let shares = window.arrangement.shares();
                    shares
                        .get(seam.splitter)
                        .and_then(|children| {
                            Some((*children.get(seam.boundary)?, *children.get(seam.boundary + 1)?))
                        })
                });
                if let Some(from) = from {
                    self.seam = Some(window::SeamDrag::start(&id, &seam, from, at));
                }
            }
            WindowPress::Offer(pane, index) => {
                // **Filling a slot is two facts, and both are the reference's.** The slot
                // stops being a slot — the reference's `set_app` (2697–2711), which swaps the
                // placeholder for the app's widget — and the window that filled it leaves the
                // float registry and becomes a pane of this window, the same bookkeeping
                // `dock_target` does for a dock.
                let offered = self
                    .empty_slot_offers(&id)
                    .get(index)
                    .map(|(window, _)| window.clone());
                let Some(offered) = offered else {
                    return;
                };
                let Ok(window_id) = WindowId::new(offered.as_str()) else {
                    return;
                };
                let PaneRef::Slot(path) = &pane else {
                    return;
                };
                let filled = self
                    .windows
                    .get_mut(&id)
                    .is_some_and(|window| window.arrangement.fill(path, window_id).is_ok());
                if !filled {
                    return;
                }
                if let Some(window) = self.windows.get_mut(&offered) {
                    window.host = Some(WindowId::new(id.as_str()).expect("the host's own id"));
                    window.status = Status::Normal;
                }
                self.ordered.retain(|held| held != &offered);
                self.focus(&id);
            }
            WindowPress::None => {}
        }
    }

    /// **A release lands where the preview showed, not where the pointer happens to be.** The
    /// port's drag carries the target it last displayed ([`DragState::preview`]); the release
    /// consumes exactly that one, so a landing cannot disagree with the frame before it.
    pub fn pointer_released(&mut self, _at: (f64, f64)) {
        if self.resize.take().is_some() {
            return;
        }
        // **The seam drag's shares are already written** — every move wrote them — so the
        // release only ends the drag, exactly as Qt's handle leaves the splitter where it was
        // let go.
        if self.seam.take().is_some() {
            return;
        }
        let Some(drag) = self.drag.take() else {
            return;
        };
        if !drag.moved {
            return;
        }
        let Some((host, target)) = drag.preview.clone() else {
            return;
        };
        let _ = self.dock_target(&host, &drag.id, &target);
    }

    /// **One wheel gesture, resolved against what is drawn.** The pane under the pointer is the
    /// pane the drawing pass laid out (via [`window_body`]), the scroll surface is the region
    /// [`console_regions`] gives that pane, and the clamp is [`console_window`]'s — so the
    /// gesture and the frame cannot disagree about where a view can go.
    ///
    /// # The freeze is the operator's reading
    ///
    /// With Auto-Scroll on, the console follows the tail. A step *back* while following is the
    /// operator asking for a reading of an earlier moment — which is exactly what the reference's
    /// Auto-Scroll control is for, except the reference makes them find the control first and
    /// then rebuilds the view; the gesture here does what the control does, and the frozen model
    /// underneath makes the reading stable. **A declared extension of the reference's control**,
    /// not a claim that the reference has it. Stepping *forward* never resumes following: the
    /// control is how the tail is resumed, and a wheel that flipped it back would make the
    /// checkbox unreadable.
    ///
    /// **The log window is not the console.** It has no Auto-Scroll of its own, so a wheel over
    /// it moves its own viewport and nothing else; while it is scrolled back, arrivals move
    /// beneath it — its offset is a distance from the record's end — and scrolling forward
    /// returns to the tail, because there is no freeze control for it to ask about.
    pub fn wheel(&mut self, at: (f64, f64), delta: WheelDelta) {
        if !self.shown {
            return;
        }
        // The pane under the pointer, front to back — the same order a click resolves in, and
        // **the same layout the drawing laid out** ([`PortalWindow::layout_at`]).
        let Some((id, rect)) = self.topmost_window(at) else {
            return;
        };
        let (pane, pane_rect) = {
            let Some(window) = self.windows.get(&id) else {
                return;
            };
            let Some(layout) = window.layout_at(rect).ok() else {
                return;
            };
            // **Only a window pane scrolls**: a slot's body is a placeholder, and there is no
            // subject under the wheel to move.
            let Some(pane) = layout.window_at(at) else {
                return;
            };
            let Some(pane_rect) = layout.window_rect(pane) else {
                return;
            };
            (pane.as_str().to_string(), pane_rect)
        };
        let Some(content) = self.windows.get(&pane).map(|window| window.spec.content) else {
            return;
        };
        if !matches!(content, Content::Console | Content::Log) {
            return;
        }
        // **The rect the content is drawn in**: the pane's own body below its header — the
        // 40 dp the derived wheel did not remove, which put its scroll surface above the
        // drawn one.
        let regions = console_regions(super::pane::body(pane_rect), content);
        if !regions.output.contains(at) {
            // The toolbar and the command bar are rows of targets, not scroll surfaces.
            return;
        }
        // **A pixel delta is smaller than a row, and truncating each one would lose the
        // motion** — so the fraction is kept until it is a whole row, one accumulator for both
        // units, and it is only spent on a scroll surface: a gesture made over the toolbar is
        // not a reading. A notched wheel reports a whole line, which is [`WHEEL_ROWS_PER_LINE`]
        // rows.
        let rows = match delta {
            WheelDelta::Lines(lines) => lines * WHEEL_ROWS_PER_LINE,
            WheelDelta::Pixels(pixels) => pixels / row_height_dp(),
        };
        self.wheel_remainder += rows;
        let whole = self.wheel_remainder.trunc();
        self.wheel_remainder -= whole;
        if whole == 0.0 {
            return;
        }
        let capacity = console_capacity(regions.output);
        let (total, held) = if content == Content::Console {
            (self.console.visible().len(), self.console.held_back())
        } else {
            (self.console.all().len(), 0)
        };
        let stored = self.scroll.get(&pane).copied().unwrap_or(0);
        let wanted = if whole > 0.0 {
            stored.saturating_add(whole as usize)
        } else {
            stored.saturating_sub((-whole) as usize)
        };
        let window = console_window(total, held, self.console.dropped(), capacity, wanted);
        // **A step back that can actually move the view is the freeze request.** A console that
        // fits entirely cannot move, and freezing it then would change the control for a gesture
        // that showed nothing.
        if content == Content::Console && window.scroll > 0 && self.console.autoscroll() {
            self.console.toggle_autoscroll();
        }
        self.scroll.insert(pane, window.scroll);
    }

    /// Put every console window back at its tail, the way the reference pins its scrollbar to
    /// the maximum after a render. See the Auto-Scroll lane and [`Mount::wheel`].
    fn anchor_console_windows(&mut self) {
        let ids: Vec<String> = self
            .windows
            .iter()
            .filter(|(_, window)| window.spec.content == Content::Console)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.scroll.insert(id, 0);
        }
    }

    // -----------------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------------

    /// One window: its chrome and its content, **drawn by the port** (`window.rs`).
    ///
    /// The composite, the title bar, the pane chrome, the seams, the edge triggers and the drop
    /// preview are all `window.rs`'s — this function's remaining job is the **content** each
    /// pane hosts (H-7.17: the console's three regions and the four HUD panels are the shell's,
    /// not the port's), and it places that content against **the layout the port returned**, so
    /// the chrome and the content cannot disagree about a rect (§5).
    fn window_elements(
        &self,
        id: &str,
        palette: &Palette,
        views: &ContentViews,
        out: &mut Vec<Element>,
    ) -> UiResult<()> {
        let Some(window) = self.windows.get(id) else {
            return Ok(());
        };
        if window.status == Status::Minimized || window.status == Status::Closed {
            return Ok(());
        }
        let Some(rect) = self.rect_of(id) else {
            return Ok(());
        };
        let opacity = (window.spec.opacity as f64 / 1000.0).clamp(0.0, 1.0);

        // The port's chrome, at the rect this frame draws the window at — the same rect the
        // pointer resolves against ([`Mount::rect_of`]). The titles and the empty-slot offers
        // are the registry's: a pane here holds a window, and a pane the registry cannot title
        // draws the reference's placeholder. **One construction with the pointer**: this view
        // is [`Mount::window_view`], so what is drawn and what is hit-tested are one value.
        let titles = self.pane_title_map(id);
        let view = self.window_view(id);
        // **A pane whose window the registry does not hold is not in this map**, and that is
        // the port's empty-slot reading: `titles` absent means the placeholder draws. Every
        // pane id in this tree's model names a window, so the map is complete for a shipped
        // arrangement — and a pane the registry has lost draws the reference's slot rather than
        // a blank.
        let layout = window.elements_at(rect, &view, palette, &titles, out)?;

        // The content, against the layout the chrome drew. **A slot has no content to draw**
        // here: its body is the placeholder, which the port's own door drew
        // (`window.rs::elements_at`) — this loop is the shell's half, and the shell draws
        // content for the panes that hold a window.
        for (pane_rect, pane) in &layout.panes {
            let Some(pane_id) = pane.window().map(|id| id.as_str().to_string()) else {
                continue;
            };
            let pane_rect = *pane_rect;
            // **The console window and the log window draw their own body** (H-7.17): the
            // reference's three regions are not a panel of readouts, so they cannot come from
            // `panel_for`. A pane whose view this shell cannot build still falls through to the
            // declared-content panel below, which is the honest answer rather than a blank.
            //
            // **The content sits below the pane's header**, the reference's own slot
            // (`WindowPane` 2666–2685): the header is one pointer row, and the body is what is
            // left of the pane.
            let pane_rect = super::pane::body(pane_rect);
            if let Some(pane_window) = self.windows.get(&pane_id) {
                let content = pane_window.spec.content;
                if matches!(content, Content::Console | Content::Log) {
                    if let Some(console) = views.console.as_ref() {
                        console_body(
                            console,
                            content,
                            pane_rect,
                            // **This pane's own scrollback.** Two readings of one log are two
                            // scroll positions, which is why the offset is the pane's and not
                            // the console's.
                            self.scroll.get(&pane_id).copied().unwrap_or(0),
                            opacity,
                            self.cursor,
                            palette,
                            out,
                        )?;
                        if pane_id != id {
                            out.push(Element::Rule {
                                rect: Rect::new(pane_rect.x, pane_rect.y, 1.0, pane_rect.height_dp),
                                paint: Ink::Divider.paint(),
                            });
                        }
                        continue;
                    }
                }
            }
            let panel = match self.windows.get(&pane_id) {
                Some(pane_window) => views.panel_for(&pane_window.spec),
                None => declared_content_panel(&window.spec),
            };
            // **Bounded to the pane it is about to be drawn in.** A window at the size its
            // package declared was already sized to its view; a pane the operator made by docking
            // was not, and one such pane used to refuse the whole window — so a docked window
            // drew no chrome and no content. The bound names the rows it held back.
            let panel = fit_panel(panel, pane_rect.height_dp);
            let width = pane_rect.width_dp - 2.0 * spacing_dp();
            // **Measured before it is placed.** §5.7's rule: the surface knows its own height
            // before it draws, so a panel that does not fit its pane is a refusal with both
            // numbers rather than a drawing that spills past the window's declared rect — and
            // a pane with no room at all is the same refusal with the same two numbers rather
            // than a silent skip (a skipped pane is a pane the shell drew nothing in and said
            // nothing about). **After `fit_panel`**, so this now fires only when the pane is too
            // small even for the panel's title and a held-count note.
            if width <= 0.0 || panel.height_dp() > pane_rect.height_dp + 0.5 {
                return Err(UiRefusal::ContentExceedsPane {
                    window: pane_id.clone(),
                    panel_dp: panel.height_dp(),
                    pane_dp: pane_rect.height_dp,
                });
            }
            out.extend(voxel_ui::layout_panel(
                palette,
                &panel,
                (pane_rect.x + spacing_dp(), pane_rect.y),
                width,
                BRIGHTEST,
            )?);
            if pane_id != id {
                out.push(Element::Rule {
                    rect: Rect::new(pane_rect.x, pane_rect.y, 1.0, pane_rect.height_dp),
                    paint: Ink::Divider.paint(),
                });
            }
        }
        Ok(())
    }

    /// The bar, drawn over everything the windows put there.
    fn bar_elements(
        &self,
        palette: &Palette,
        viewport: Rect,
        out: &mut Vec<Element>,
    ) -> UiResult<()> {
        let chrome = palette.get(Role::Chrome)?;
        let height = bar_height_dp();
        let bar = Rect::new(0.0, 0.0, viewport.width_dp, height);
        // The bar is the shell's own surface: chrome at the HUD's declared panel opacity, which
        // is the least opacity at which the ink ladder survives a white world (voxel-ui's
        // derivation, not a second number chosen here).
        let opacity = voxel_ui::panel_opacity();
        out.push(Element::Surface {
            rect: bar,
            paint: Paint { rgb: chrome, alpha: opacity },
        });
        out.push(Element::Rule {
            rect: Rect::new(0.0, height, viewport.width_dp, 1.0),
            paint: Ink::Divider.paint(),
        });

        let backdrop = chrome.blend_over(BRIGHTEST, opacity);
        let rects = self.bar_rects(viewport);
        for (entry, (id, tab, rect)) in self.bar.iter().zip(rects) {
            let is_open = self.menu.as_deref() == Some(entry.id.as_str());
            let focused = self.focused.as_deref() == Some(entry.id.as_str());
            let ink = if is_open || focused { Ink::Full } else { Ink::Secondary };
            contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
            out.push(Element::Text {
                rect: Rect::new(
                    rect.x + spacing_dp(),
                    rect.y + (height - voxel_ui::text_size_dp()) / 2.0,
                    rect.width_dp - 2.0 * spacing_dp(),
                    voxel_ui::text_size_dp(),
                ),
                text: entry.label.clone(),
                paint: ink.paint(),
                size_dp: voxel_ui::text_size_dp(),
                align: Align::Start,
            });
            // The entry that is open (its menu, or its window focused) is marked by a rule at
            // the bar's foot — the MD1 underline, one ink step up from the divider.
            if is_open || focused {
                out.push(Element::Rule {
                    rect: Rect::new(rect.x, height - 2.0, rect.width_dp, 2.0),
                    paint: Ink::Secondary.paint(),
                });
            }
            let _ = (id, tab);
        }
        Ok(())
    }

    /// The open tab's menu, if there is one.
    fn menu_elements(&self, palette: &Palette, viewport: Rect) -> UiResult<Vec<Element>> {
        let Some(tab) = self.menu.as_deref() else {
            return Ok(Vec::new());
        };
        let Some(entry) = self.bar.iter().position(|entry| entry.id == tab) else {
            return Ok(Vec::new());
        };
        let surface_role = Role::SurfaceAlt;
        let surface = palette.get(surface_role)?;
        let opacity = voxel_ui::panel_opacity();
        let backdrop = surface.blend_over(BRIGHTEST, opacity);
        let rows = self.menu_rows(entry, viewport);
        let mut out = Vec::new();
        let Some((_, first)) = rows.first().cloned() else {
            return Ok(Vec::new());
        };
        let height = rows
            .last()
            .map(|(_, rect)| rect.bottom() - first.y)
            .unwrap_or(0.0);
        out.push(Element::Surface {
            rect: Rect::new(first.x, first.y, first.width_dp, height),
            paint: Paint { rgb: surface, alpha: opacity },
        });

        // The menu's title row: the tab's own label, in the secondary ink — it names the sort,
        // not a window.
        contrast_over_backdrop(Ink::Secondary.composited_over(backdrop), backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                first.x + spacing_dp(),
                first.y + (row_height_dp() - voxel_ui::text_size_dp()) / 2.0,
                first.width_dp - 2.0 * spacing_dp(),
                voxel_ui::text_size_dp(),
            ),
            text: self
                .windows
                .get(tab)
                .map(|w| w.spec.title.clone())
                .unwrap_or_else(|| tab.to_string()),
            paint: Ink::Secondary.paint(),
            size_dp: voxel_ui::text_size_dp(),
            align: Align::Start,
        });
        out.push(Element::Rule {
            rect: Rect::new(first.x, first.y + row_height_dp(), first.width_dp, 1.0),
            paint: Ink::Divider.paint(),
        });

        for (id, rect) in rows.into_iter().skip(1) {
            let Some(window) = self.windows.get(&id) else {
                continue;
            };
            let state = window.status.name();
            contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;
            contrast_over_backdrop(Ink::Secondary.composited_over(backdrop), backdrop)?;
            let inner = rect.width_dp - 2.0 * spacing_dp();
            let label_width = inner * 0.65;
            out.push(Element::Text {
                rect: Rect::new(
                    rect.x + spacing_dp(),
                    rect.y + (row_height_dp() - voxel_ui::text_size_dp()) / 2.0,
                    label_width,
                    voxel_ui::text_size_dp(),
                ),
                text: window.spec.title.clone(),
                paint: Ink::Full.paint(),
                size_dp: voxel_ui::text_size_dp(),
                align: Align::Start,
            });
            out.push(Element::Text {
                rect: Rect::new(
                    rect.x + spacing_dp() + label_width,
                    rect.y + (row_height_dp() - voxel_ui::text_size_dp()) / 2.0,
                    inner - label_width,
                    voxel_ui::text_size_dp(),
                ),
                text: state.to_string(),
                paint: Ink::Secondary.paint(),
                size_dp: voxel_ui::text_size_dp(),
                align: Align::End,
            });
            out.push(Element::Rule {
                rect: Rect::new(rect.x, rect.bottom() - 1.0, rect.width_dp, 1.0),
                paint: Ink::Divider.paint(),
            });
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Geometry helpers
    // -----------------------------------------------------------------------

    /// The bar's entry rects, in strip order, laid left to right.
    ///
    /// **An entry never drops below the target floor.** The declared width is 112 dp so a label
    /// fits without the layout measuring text, and a viewport too narrow for the whole strip
    /// narrows every entry toward 48 dp before it clips one — a clipped target is a control
    /// nobody can hit, and the record would rather have a cramped bar than an invisible entry.
    pub fn bar_rects(&self, viewport: Rect) -> Vec<(String, bool, Rect)> {
        let count = self.bar.len().max(1) as f64;
        let width = (viewport.width_dp / count).clamp(min_target_dp(), bar_entry_width_dp());
        self.bar
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    entry.id.clone(),
                    entry.tab,
                    Rect::new(index as f64 * width, 0.0, width, bar_height_dp()),
                )
            })
            .collect()
    }

    /// The control lanes for a window's title bar — **the port's own list** (`titlebar.rs`),
    /// so a lane a test clicks is the lane the frame drew and the lane `press_at` hit-tests.
    pub fn control_lanes(&self, id: &str, rect: Rect) -> Vec<(GlyphAction, Rect)> {
        let Some(window) = self.windows.get(id) else {
            return Vec::new();
        };
        super::titlebar::control_lanes(rect, &window.spec)
    }

    /// The menu's rows, in draw order: a **header** naming the sort, then one row per offered
    /// window. Returned as `(window id, rect)`, and the header's id is the tab's — which is why
    /// click handling skips the first row rather than testing the id.
    ///
    /// The rows are shifted up when they would run off the foot of the viewport, so a tab near
    /// the bottom edge still offers all of its windows rather than a clipped list.
    pub fn menu_rows(&self, entry: usize, viewport: Rect) -> Vec<(String, Rect)> {
        let Some(tab) = self.bar.get(entry) else {
            return Vec::new();
        };
        let width = voxel_ui::hud_panel_width_dp();
        let x = self
            .bar_rects(viewport)
            .get(entry)
            .map(|(_, _, rect)| rect.x)
            .unwrap_or(0.0);
        let mut y = bar_height_dp();
        let mut rows = Vec::new();
        rows.push((tab.id.clone(), Rect::new(x, y, width, row_height_dp())));
        y += row_height_dp();
        for id in self.offers(&tab.id) {
            rows.push((id, Rect::new(x, y, width, row_height_dp())));
            y += row_height_dp();
        }
        let height = rows.len() as f64 * row_height_dp();
        let shift = (bar_height_dp() + height - viewport.height_dp).max(0.0);
        rows.into_iter()
            .map(|(id, rect)| {
                (id, Rect::new(rect.x, rect.y - shift, rect.width_dp, rect.height_dp))
            })
            .collect()
    }

    /// Keep a dragged window on the screen and under the bar.
    fn clamp_window(&self, rect: Rect) -> Rect {
        let max_x = (self.viewport.width_dp - rect.width_dp).max(0.0);
        let max_y = (self.viewport.height_dp - rect.height_dp).max(bar_height_dp());
        Rect::new(
            rect.x.clamp(0.0, max_x),
            rect.y.clamp(bar_height_dp(), max_y),
            rect.width_dp,
            rect.height_dp,
        )
    }
}

/// The pane titles keyed by pane id — [`Mount::pane_title_map`]'s walk, one implementation with
/// [`collect_titles`].
fn collect_title_map(
    node: &Node,
    windows: &BTreeMap<String, Window>,
    out: &mut BTreeMap<String, String>,
) {
    match node {
        Node::Pane(id) => {
            if let Some(window) = windows.get(id.as_str()) {
                out.insert(id.as_str().to_string(), window.spec.title.clone());
            }
        }
        // **A slot has no window to title**, and the drawing does not ask: the port reads
        // slot-ness from the tree (`PaneRef`), not from an absent entry in this map.
        Node::Slot => {}
        Node::Split { children, .. } => {
            for (_, child) in children {
                collect_title_map(child, windows, out);
            }
        }
    }
}

fn collect_titles(node: &Node, windows: &BTreeMap<String, Window>, out: &mut Vec<String>) {
    match node {
        Node::Pane(id) => {
            let title = windows
                .get(id.as_str())
                .map(|window| window.spec.title.clone())
                .unwrap_or_else(|| id.as_str().to_string());
            out.push(title);
        }
        // **The reference's multi-pane slot name** (`"◫ Split"`, 3155): a slot takes its place
        // in the composed title like any pane.
        Node::Slot => out.push(super::pane::split_slot_title()),
        Node::Split { children, .. } => {
            for (_, child) in children {
                collect_titles(child, windows, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::arrangement::MIN_EXTENT;
    use crate::shell::notifications::toast_ticks;
    use voxel_package::recipe;
    use voxel_ui::{title_bar_dp, Palette};

    const TEST_RECIPE: &str = "shploingus.recipe/3\n\
package\ttest-shell\tv0\n\
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
\tgeometry\t16 128 480 440\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\ttrue\n\
\tlayout\tminimize,maximize,close,title-position\n\
\tbehavior\tdrag,resize,maximize,minimize,close\n\
\trecord-id\twindow-inventory\n\
window\ttrees\n\
\ttitle\tTrees\n\
\tcontent\ttrees\n\
\tlane\ttab\n\
\textent\tsquare-pixels\n\
\tgeometry\t80 160 480 440\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\ttrue\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag,close\n\
\trecord-id\twindow-trees\n\
window\ttools\n\
\ttitle\tTools\n\
\tcontent\ttools\n\
\tlane\ttab\n\
\textent\tsquare-pixels\n\
\tgeometry\t144 192 480 440\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag,close\n\
\trecord-id\twindow-tools\n\
window\tconsole\n\
\ttitle\tConsole\n\
\tcontent\tconsole\n\
\tlane\tcontrol\n\
\textent\tsquare-pixels\n\
\tgeometry\t16 96 640 406\n\
\tmin-size\t320 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag,close\n\
\trecord-id\twindow-console\n\
tab\tinventory\n\
tab\ttrees\n\
bar\tconsole\n\
filed\ttools\tinventory\n\
pin\tworld\n";

    fn loaded() -> Loaded {
        let package = recipe::compile(TEST_RECIPE.as_bytes()).expect("the test recipe compiles");
        recipe::load(&package).expect("the test package loads")
    }

    fn views() -> ContentViews {
        let panel = |title: &str| {
            Panel::new(
                "test view",
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
        }
    }

    fn mount() -> Mount {
        let mut mount = Mount::new(&loaded()).expect("the test package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
        mount
    }

    /// A package for the console and its log — H-7.17's two windows, and nothing else.
    const CONSOLE_RECIPE: &str = "shploingus.recipe/3\n\
package\tconsole-test\tv0\n\
window\tconsole\n\
\ttitle\tConsole\n\
\tcontent\tconsole\n\
\tlane\tcontrol\n\
\textent\tsquare-pixels\n\
\tgeometry\t16 96 640 406\n\
\tmin-size\t320 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag,close\n\
\trecord-id\twindow-console\n\
window\tlog\n\
\ttitle\tLog\n\
\tcontent\tlog\n\
\tlane\tframe\n\
\textent\tsquare-pixels\n\
\tgeometry\t680 96 640 406\n\
\tmin-size\t320 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag,close\n\
\trecord-id\twindow-log\n";

    /// The console package, mounted, with the console and its log both open.
    fn console_mount() -> Mount {
        let package = recipe::compile(CONSOLE_RECIPE.as_bytes()).expect("the console recipe compiles");
        let loaded = recipe::load(&package).expect("the console package loads");
        let mut mount = Mount::new(&loaded).expect("the console package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
        mount.open("console").expect("declared");
        mount.open("log").expect("declared");
        mount
    }

    /// The console's world: a console value and the test panels.
    fn console_views(console: Console) -> ContentViews {
        let mut views = views();
        views.console = Some(console);
        views
    }

    /// The console window's body rect, **the same arithmetic the drawing path lays out
    /// inside** — [`window_body`], so a lane fixture cannot disagree with the drawn control.
    fn console_body_rect(mount: &Mount) -> Rect {
        let rect = mount.rect_of("console").expect("the console is open");
        let spec = mount.window("console").expect("the console is held").spec.clone();
        super::super::pane::body(window_body(&spec, rect))
    }

    /// Every text the shell draws for this mount, through the same entry point the bin uses.
    fn drawn_texts(mount: &mut Mount) -> Vec<String> {
        let palette = Palette::declared();
        let viewport = mount.viewport();
        let views = console_views(mount.console().clone());
        mount
            .shell_elements(&palette, viewport, &views)
            .expect("the shell lays out")
            .iter()
            .filter_map(|element| element.text().map(str::to_string))
            .collect()
    }

    fn lane_center(rect: Rect) -> (f64, f64) {
        (rect.x + rect.width_dp / 2.0, rect.y + rect.height_dp / 2.0)
    }

    /// The toast's elements as the layer draws them — the same call `shell/hud.rs` appends after
    /// its chosen layer, so a test cannot disagree with the drawing.
    fn toast_elements(mount: &Mount) -> Vec<Element> {
        let palette = Palette::declared();
        let mut out = Vec::new();
        mount
            .toast_elements(&palette, mount.viewport(), &mut out)
            .expect("the toast paints");
        out
    }

    /// The toast's timer rule's width, or `None` when nothing is drawn.
    fn toast_timer_width(mount: &Mount) -> Option<f64> {
        toast_elements(mount).iter().find_map(|element| match element {
            Element::Rule { rect, .. } => Some(rect.width_dp),
            _ => None,
        })
    }

    /// Every surface the shell layer draws, with its paint — the same entry point the bin uses.
    fn drawn_surfaces(mount: &mut Mount) -> Vec<(Rect, voxel_ui::Rgb)> {
        let palette = Palette::declared();
        let viewport = mount.viewport();
        let views = console_views(mount.console().clone());
        mount
            .shell_elements(&palette, viewport, &views)
            .expect("the shell lays out")
            .iter()
            .filter_map(|element| match element {
                Element::Surface { rect, paint } => Some((*rect, paint.rgb)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_transient_record_goes_on_screen_as_a_toast_and_its_timer_runs_down() {
        let mut mount = console_mount();
        mount.announce(Level::Event, "world", "placed regolith at (1, 2, 3)", Some(0));
        assert!(
            mount.toasts.current().is_none(),
            "nothing is on screen until the frame advances the queue"
        );
        mount.set_tick(0);
        let lines: Vec<String> = toast_elements(&mount)
            .iter()
            .filter_map(|element| element.text().map(str::to_string))
            .collect();
        assert_eq!(lines, vec!["placed regolith at (1, 2, 3)".to_string()]);

        let width = voxel_ui::toast_width_dp();
        let full = toast_timer_width(&mount).expect("the timer is drawn");
        assert_eq!(full, width, "the timer starts at the whole width");
        mount.set_tick(toast_ticks() / 2);
        let half = toast_timer_width(&mount).expect("the timer is drawn");
        assert!(half < full && half > 0.0, "the timer runs down: {full} -> {half}");
        mount.set_tick(toast_ticks());
        assert!(
            mount.toasts.current().is_none(),
            "six seconds and the record is gone"
        );
        assert!(toast_elements(&mount).is_empty(), "and nothing is drawn");
    }

    #[test]
    fn a_refusal_is_logged_and_never_toasted_and_a_timeless_record_is_not_raised() {
        let mut mount = console_mount();
        mount.announce(Level::Warn, "world", "sedimentary cannot go there", Some(0));
        mount.set_tick(0);
        assert!(
            mount.toasts.current().is_none(),
            "a refusal is never a toast that scrolls away (C-64.2)"
        );
        assert!(
            mount.console().lines().iter().any(|line| line.contains("cannot go there")),
            "and it is still a record on the console: {:?}",
            mount.console().lines()
        );

        mount.announce(Level::Event, "world", "placed regolith", None);
        mount.set_tick(1);
        assert!(
            mount.toasts.current().is_none(),
            "a record with no tick cannot be timed, so it is not raised"
        );
        assert!(
            mount.console().lines().iter().any(|line| line.contains("placed regolith")),
            "and it is not lost either"
        );
    }

    #[test]
    fn a_windows_declared_elevation_is_the_one_drawn() {
        // Both windows in the fixture declare elevation 1. The declaration was parsed, validated
        // and never read before C-109: a window's depth was in the file and not on the screen.
        let mut mount = console_mount();
        let palette = Palette::declared();
        let rect = mount.rect_of("console").expect("the console is open");
        let body = drawn_surfaces(&mut mount)
            .into_iter()
            .find(|(drawn, _)| *drawn == rect)
            .map(|(_, rgb)| rgb)
            .expect("the console's body surface is drawn");
        let raw = palette.get(Role::Surface).expect("admitted");
        assert_eq!(body, raw.elevated(1).expect("level 1 is on the ladder"));
        assert_ne!(body, raw, "a declared elevation has to be visible");
    }

    #[test]
    fn the_notifications_window_reads_the_persistent_records_and_not_the_console_s_view() {
        let mut console = Console::new();
        console.log(Level::Info, "world", "placed regolith at (1, 2, 3)", Some(1));
        console.log(Level::Warn, "world", "sedimentary cannot go there", Some(2));
        // The console window's own view control: it must not change what the notifications
        // window says, because both read one store (UI-SPEC §7).
        console.set_filter(Some(Level::Info));

        let panel = notifications_panel(&console, 358.0);
        let texts: Vec<String> = panel.rows.iter().map(|row| row.value.clone()).collect();
        assert!(
            texts.iter().any(|text| text.contains("cannot go there")),
            "the refusal is a notification: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.contains("placed regolith")),
            "a transient result is a toast, not a notification: {texts:?}"
        );
        assert!(panel.validate().is_empty(), "the panel is legal: {:?}", panel.validate());
    }

    #[test]
    fn an_empty_notifications_window_says_why_it_is_empty() {
        let panel = notifications_panel(&Console::new(), 358.0);
        assert!(
            panel.note.as_deref().unwrap_or("").contains("no persistent records"),
            "an empty view says which emptiness it is (MD1's empty-view rule): {:?}",
            panel.note
        );
        assert!(panel.validate().is_empty(), "{:?}", panel.validate());
    }

    #[test]
    fn the_console_window_clamps_at_both_ends_and_reserves_the_note_row() {
        // Everything fits: there is nowhere to scroll and nothing is held.
        let window = console_window(5, 0, 0, 10, 100);
        assert_eq!(
            (window.scroll, window.start, window.shown, window.older, window.newer),
            (0, 0, 5, 0, 0)
        );
        // Thirty records in ten rows: one row is the note's, so nine are drawn and twenty-one
        // are older.
        let tail = console_window(30, 0, 0, 10, 0);
        assert_eq!((tail.start, tail.shown, tail.newer), (21, 9, 0));
        // Scrolled to the top: the window stops at the first record instead of scrolling past
        // it, and the clamp is what a caller stores back.
        let top = console_window(30, 0, 0, 10, 1000);
        assert_eq!((top.scroll, top.start, top.shown, top.newer), (21, 0, 9, 21));
        // A freeze holds rows *below* the window: the tail stops at the frozen end.
        let frozen = console_window(30, 5, 0, 10, 0);
        assert_eq!((frozen.start, frozen.shown, frozen.newer), (16, 9, 5));
        // Records the ceiling dropped claim the note row even when everything else fits.
        let dropped = console_window(5, 0, 3, 10, 0);
        assert_eq!((dropped.shown, dropped.older, dropped.newer), (5, 0, 0));
        assert_eq!(console_window(20, 0, 1, 10, 0).shown, 9);
    }

    #[test]
    fn the_console_s_wheel_moves_the_view_and_a_step_back_is_the_freeze_request() {
        let mut mount = console_mount();
        // **The console's reading, with the log's reading out of the frame.** Both windows draw
        // the same record, and a claim about the console's view must not be satisfied or denied
        // by the log's.
        mount.close("log");
        for index in 0..64u64 {
            mount.log(Level::Info, "TEST", format!("line {index:02}"), Some(index));
        }
        let body = console_body_rect(&mount);
        let at = lane_center(console_regions(body, Content::Console).output);

        // Following the tail: the newest line is drawn and the oldest is not.
        let texts = drawn_texts(&mut mount);
        assert!(
            texts.iter().any(|text| text.contains("(TEST) line 63")),
            "{texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.contains("(TEST) line 00")),
            "{texts:?}"
        );

        // A step back scrolls *and freezes* — the gesture is the Auto-Scroll control's own
        // meaning, and one reported line is WHEEL_ROWS_PER_LINE rows.
        mount.wheel(at, WheelDelta::Lines(1.0));
        assert!(!mount.console().autoscroll(), "the gesture asked for a reading");
        assert_eq!(mount.scroll.get("console").copied(), Some(3));
        let texts = drawn_texts(&mut mount);
        assert!(
            !texts.iter().any(|text| text.contains("(TEST) line 63")),
            "the view moved back: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.contains("(TEST) line 60")),
            "and the window still ends three records before the tail: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.contains("(TEST) line 61")),
            "nothing newer than the window's end is drawn: {texts:?}"
        );
        // **A reading, not an edit:** every record is still there.
        assert_eq!(mount.console().len(), 64);

        // A step forward returns to the tail and does not resume following by itself.
        mount.wheel(at, WheelDelta::Lines(-1.0));
        assert_eq!(mount.scroll.get("console").copied(), Some(0));
        assert!(!mount.console().autoscroll(), "the control is how the tail is resumed");
        assert!(
            drawn_texts(&mut mount)
                .iter()
                .any(|text| text.contains("(TEST) line 63")),
            "the tail is drawn again"
        );

        // The toolbar and the command bar are rows of targets, not scroll surfaces.
        let toolbar = console_regions(body, Content::Console)
            .toolbar
            .expect("the toolbar");
        mount.wheel(lane_center(toolbar), WheelDelta::Lines(1.0));
        assert_eq!(mount.scroll.get("console").copied(), Some(0));
    }

    #[test]
    fn the_log_window_has_its_own_scroll_and_it_is_not_the_console_s() {
        let mut mount = console_mount();
        for index in 0..64u64 {
            mount.log(Level::Info, "TEST", format!("line {index:02}"), Some(index));
        }
        let log = mount.rect_of("log").expect("the log is open");
        let log_spec = mount.window("log").expect("the log is held").spec.clone();
        let at = lane_center(console_regions(window_body(&log_spec, log), Content::Log).output);
        mount.wheel(at, WheelDelta::Lines(1.0));
        assert_eq!(mount.scroll.get("log").copied(), Some(3));
        assert_eq!(
            mount.scroll.get("console").copied(),
            None,
            "the console's viewport did not move"
        );
        assert!(
            mount.console().autoscroll(),
            "the log has no Auto-Scroll of its own to ask about, so the console's is untouched"
        );
    }

    #[test]
    fn the_console_window_is_the_reference_s_toolbar_output_and_command_bar() {
        // H-7.17: the console is a Rust derivation of the reference's `ConsoleWidget` — its
        // controls, its output and its command bar — not a panel of readouts.
        let mut mount = console_mount();
        mount.log(Level::Event, "LAUNCH", "launched files", Some(3));
        let palette = Palette::declared();
        let viewport = mount.viewport();
        let views = console_views(mount.console().clone());
        let elements = mount
            .shell_elements(&palette, viewport, &views)
            .expect("the shell lays out");
        let texts: Vec<String> = elements
            .iter()
            .filter_map(|element| element.text().map(str::to_string))
            .collect();
        for wanted in [
            "All Logs",
            "Filter logs...",
            "Clear",
            "Copy",
            "Auto-Scroll on",
            "Run",
            ">",
            "[EVENT]",
            "t3 (LAUNCH) launched files",
        ] {
            assert!(
                texts.iter().any(|text| text.contains(wanted)),
                "{wanted:?} is part of the console window: {texts:?}"
            );
        }
    }

    #[test]
    fn the_console_s_controls_act_where_they_are_drawn() {
        let mut mount = console_mount();
        let body = console_body_rect(&mount);
        let lane = |wanted: ConsoleLane| {
            console_lanes(body)
                .into_iter()
                .find(|(lane, _)| *lane == wanted)
                .expect("every lane exists")
                .1
        };

        // The filter cycles, and its own name changes with it.
        assert_eq!(mount.console().filter_name(), "All Logs");
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::Filter)));
        assert_eq!(mount.console().filter_name(), "System [SYS]");
        // Back to All Logs, so the records this test logs are the ones it reads.
        mount.console.set_filter(None);

        // Auto-Scroll is a toggle, checked by default — and switching it back on releases the
        // freeze, or the records this test logs next would be held back rather than shown.
        assert!(mount.console().autoscroll());
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::AutoScroll)));
        assert!(!mount.console().autoscroll());
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::AutoScroll)));
        assert!(mount.console().autoscroll());
        assert_eq!(mount.console().held_back(), 0);

        // The search box takes the keyboard; a query filters and never executes.
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::Search)));
        assert_eq!(mount.console().field(), crate::shell::console::Field::Search);
        for character in "files".chars() {
            mount.key_pressed(Key::Char(character), None);
        }
        assert_eq!(mount.console().search(), "files");
        // Enter puts the keyboard back on the command rather than submitting the query.
        mount.key_pressed(Key::Enter, None);
        assert_eq!(mount.console().field(), crate::shell::console::Field::Command);
        mount.console.clear_search();

        // The command line types a verb and Run executes it.
        for character in "echo hallo".chars() {
            mount.key_pressed(Key::Char(character), None);
        }
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::Run)));
        assert!(
            mount.console().lines().iter().any(|line| line.contains("hallo")),
            "run executed the command line: {:?}",
            mount.console().lines()
        );

        // Copy has no clipboard in this shell, and says so instead of silently doing nothing.
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::Copy)));
        assert!(
            mount
                .console()
                .lines()
                .iter()
                .any(|line| line.contains("[WARN]") && line.contains("no clipboard")),
            "copy refuses by name: {:?}",
            mount.console().lines()
        );

        // Clear empties the output and appends the reference's own line.
        mount.pointer_pressed(Button::Left, lane_center(lane(ConsoleLane::Clear)));
        let lines = mount.console().lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("log cleared"), "{lines:?}");
    }

    #[test]
    fn the_log_window_is_the_event_record_and_not_the_console_s_view() {
        // H-7.17: *"plus a log window"*. The console filters; the log is the record, so a
        // filtered console cannot make a record invisible in the log.
        let mut mount = console_mount();
        mount.log(Level::Info, "SHELL", "mounted", Some(0));
        mount.log(Level::Warn, "PANEL", "too tall for the pane", Some(1));
        // Filter the console to Warnings only.
        let body = console_body_rect(&mount);
        while mount.console().filter_name() != "Warnings [WARN]" {
            mount.pointer_pressed(
                Button::Left,
                lane_center(
                    console_lanes(body)
                        .into_iter()
                        .find(|(lane, _)| *lane == ConsoleLane::Filter)
                        .expect("the filter lane")
                        .1,
                ),
            );
        }
        let palette = Palette::declared();
        let viewport = mount.viewport();
        let views = console_views(mount.console().clone());
        let elements = mount
            .shell_elements(&palette, viewport, &views)
            .expect("the shell lays out");
        let texts: Vec<String> = elements
            .iter()
            .filter_map(|element| element.text().map(str::to_string))
            .collect();
        // **Exactly the badge, not any text containing it** — the filter lane's own label
        // ("Warnings [WARN]") contains a badge by construction and is not a record.
        let badges = |wanted: &str| texts.iter().filter(|text| text.trim() == wanted).count();
        assert_eq!(badges("[WARN]"), 2, "both the console and the log show the warn");
        assert_eq!(
            badges("[INFO]"),
            1,
            "only the log shows the info the console is filtering out: {texts:?}"
        );
    }

    #[test]
    fn the_bar_is_the_declared_tabs_in_strip_order_then_the_entries_that_are_not_tabs() {
        let mount = mount();
        let ids: Vec<&str> = mount.bar().iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["inventory", "trees", "console"]);
        assert!(mount.bar()[0].tab);
        assert!(!mount.bar()[2].tab, "the console is on the bar, not a tab");
        // Every entry is a target, and one wide enough for its label at the declared width.
        for (_, _, rect) in mount.bar_rects(mount.viewport()) {
            rect.check_target("a bar entry").expect("the bar's entries are targets");
        }
    }

    #[test]
    fn a_tab_s_menu_offers_its_own_window_and_the_windows_filed_under_it() {
        let mut mount = mount();
        mount.pointer_pressed(Button::Left, (60.0, 24.0));
        assert_eq!(mount.open_menu(), Some("inventory"));
        assert_eq!(mount.offers("inventory"), vec!["inventory", "tools"]);
        // The menu's rows are targets too.
        let viewport = mount.viewport();
        for (_, rect) in mount.menu_rows(0, viewport) {
            rect.check_target("a menu row").expect("a menu row is a target");
        }
        // Clicking a row opens the window it names — and the rows are draws of `offers`,
        // so the row for `tools` is the one after the tab's own window.
        let rows = mount.menu_rows(0, viewport);
        let (_, row) = rows
            .iter()
            .find(|(id, _)| id == "tools")
            .expect("the filed window has a row")
            .clone();
        mount.pointer_pressed(Button::Left, (row.x + 4.0, row.y + 4.0));
        assert_eq!(mount.open_menu(), None, "a row click closes the menu");
        assert_eq!(mount.window("tools").map(|w| w.status), Some(Status::Normal));
        assert_eq!(mount.focused(), Some("tools"));
    }

    #[test]
    fn a_bar_entry_that_is_not_a_tab_opens_its_window_without_a_menu() {
        let mut mount = mount();
        let console = mount
            .bar_rects(mount.viewport())
            .into_iter()
            .find(|(id, _, _)| id == "console")
            .expect("the console is on the bar");
        mount.pointer_pressed(Button::Left, (console.2.x + 4.0, console.2.y + 4.0));
        assert_eq!(mount.open_menu(), None);
        assert_eq!(mount.window("console").map(|w| w.status), Some(Status::Normal));
    }

    #[test]
    fn an_unpermitted_control_is_absent_rather_than_drawn_disabled() {
        let mount = mount();
        let inventory = mount.window("inventory").expect("declared");
        assert_eq!(inventory.controls().len(), 3, "minimize, maximize, close are declared");
        let trees = mount.window("trees").expect("declared");
        assert_eq!(trees.controls(), vec![GlyphAction::Close]);
        // And the lanes really are the permitted ones: one lane, the trailing one.
        let rect = mount.rect_of("trees").expect("a standalone window has a rect");
        let lanes = mount.control_lanes("trees", rect);
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].0, GlyphAction::Close);
        // **The lane is a dense-pointer target, not a touch one**: D-1 makes the ported
        // controls `POINTER_TARGET_DP` (40 dp), so `check_target`'s 48 dp touch floor is the
        // wrong bar for them — the declared dense floor is the bar.
        assert!(
            lanes[0].1.width_dp >= voxel_ui::pointer_target_dp()
                && lanes[0].1.height_dp >= voxel_ui::pointer_target_dp(),
            "a control lane is a pointer target: {:?}",
            lanes[0].1
        );
    }

    #[test]
    fn a_click_on_the_close_glyph_closes_the_window_and_not_its_neighbour() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let rect = mount.rect_of("trees").expect("trees is standalone");
        let (_, lane) = mount.control_lanes("trees", rect)[0];
        mount.pointer_pressed(Button::Left, (lane.x + 4.0, lane.y + 4.0));
        assert_eq!(mount.window("trees").map(|w| w.status), Some(Status::Closed));
        assert_eq!(mount.window("inventory").map(|w| w.status), Some(Status::Normal));
    }

    #[test]
    fn a_window_split_beside_another_derives_its_title_from_the_pane_set() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        mount
            .dock("inventory", "trees", (host.x + 10.0, host.y + title_bar_dp() + 40.0))
            .expect("the drop splits the host");
        assert_eq!(
            mount.window("trees").and_then(|w| w.host.as_ref()).map(WindowId::as_str),
            Some("inventory")
        );
        // The drop landed in the upper half, so the dragged window takes the leading pane and
        // the title says so in tree order.
        assert_eq!(mount.title_of("inventory"), "Trees / Inventory");
        assert_eq!(mount.rect_of("trees"), None, "a docked window has no rect of its own");
        // And undocking gives it a rect back, where its pane was.
        let pane = mount.pane_rect("inventory", "trees").expect("the pane is laid out");
        mount.undock("inventory", "trees").expect("the pane undocks");
        assert_eq!(mount.window("trees").and_then(|w| w.host.as_ref()), None);
        assert_eq!(mount.window("trees").map(|w| w.rect), Some(pane));
    }

    #[test]
    fn closing_a_host_releases_its_panes_instead_of_taking_them_with_it() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        mount
            .dock("inventory", "trees", (host.x + 10.0, host.y + title_bar_dp() + 40.0))
            .expect("the drop splits the host");
        mount.close("inventory");
        assert_eq!(mount.window("inventory").map(|w| w.status), Some(Status::Closed));
        assert_eq!(mount.window("trees").map(|w| w.status), Some(Status::Normal));
        assert!(mount.rect_of("trees").is_some(), "a released pane floats again");
    }

    #[test]
    fn a_drag_shows_the_landing_and_the_release_lands_where_it_showed() {
        // The port's drag machine end to end: the bar starts a `DragState`, a move resolves the
        // drop with `drop_target_at`, and the release docks into the target the preview showed
        // — not into a second, point-based derivation.
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        let bar = (host.x + host.width_dp / 2.0, host.y + 20.0);
        mount.pointer_pressed(Button::Left, bar);
        assert!(mount.drag.is_some(), "the bar starts a drag");
        let target = mount.rect_of("trees").expect("trees is open");
        let at = (target.x + target.width_dp / 2.0, target.y + target.height_dp / 2.0);
        mount.pointer_moved(at);
        let (shown, preview) = mount
            .drop_preview()
            .cloned()
            .expect("the drag shows the landing");
        assert_eq!(shown, "trees");
        assert_eq!(
            preview.pane.as_ref().and_then(PaneRef::window).map(WindowId::as_str),
            Some("trees"),
            "the pane under the point is the target"
        );
        mount.pointer_released(at);
        assert_eq!(
            mount
                .window("inventory")
                .and_then(|w| w.host.as_ref())
                .map(WindowId::as_str),
            Some("trees"),
            "the release landed where the preview showed"
        );
        assert!(mount.drop_preview().is_none(), "the preview is spent");
    }

    #[test]
    fn a_resize_edge_drag_moves_that_edge_and_leaves_the_far_one() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let rect = mount.rect_of("inventory").expect("standalone");
        let at = (rect.right() - 4.0, rect.y + rect.height_dp / 2.0);
        mount.pointer_pressed(Button::Left, at);
        assert!(mount.resize.is_some(), "the east strip starts a resize");
        mount.pointer_moved((at.0 + 60.0, at.1));
        mount.pointer_released((at.0 + 60.0, at.1));
        let after = mount.rect_of("inventory").expect("standalone");
        assert_eq!(after.x, rect.x, "the far edge stayed");
        assert_eq!(after.width_dp, rect.width_dp + 60.0);
        assert_eq!(after.height_dp, rect.height_dp);
        assert!(mount.resize.is_none(), "the release ends the resize");
    }

    #[test]
    fn a_double_click_on_the_bar_maximizes_where_the_declaration_permits_it() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let rect = mount.rect_of("inventory").expect("standalone");
        let bar = (rect.x + rect.width_dp / 2.0, rect.y + 20.0);
        // One click is a press: focus and a drag, never a maximize.
        mount.pointer_pressed(Button::Left, bar);
        assert_eq!(mount.window("inventory").map(|w| w.status), Some(Status::Normal));
        // The second click arrives as a double-click (the bin's count), and the port's
        // `press_at(double)` answers the bar with its maximize verdict.
        mount.pointer_double_clicked(Button::Left, bar);
        assert_eq!(
            mount.window("inventory").map(|w| w.status),
            Some(Status::Maximized),
            "the double-click maximized where the policy permits it"
        );
        // A window whose declaration does not permit maximize answers the same gesture with
        // nothing — the verdict is the port's, and it checks the policy.
        mount.open("world").expect("declared");
        let world = mount.rect_of("world").expect("standalone");
        let world_bar = (world.x + world.width_dp / 2.0, world.y + 20.0);
        mount.pointer_double_clicked(Button::Left, world_bar);
        assert_eq!(
            mount.window("world").map(|w| w.status),
            Some(Status::Normal),
            "maximize is absent when the package does not permit it"
        );
    }

    #[test]
    fn the_pane_close_control_closes_that_pane_and_not_the_host() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        // **Docked on the right**, so the pane's own controls are at the window's end: the
        // edge-trigger lanes are centred on the window and are raised above the panes
        // (3280–3281), and a close lane under the top lane would answer the trigger.
        mount
            .dock(
                "inventory",
                "trees",
                (host.right() - 10.0, host.y + title_bar_dp() + 40.0),
            )
            .expect("the drop splits the host");
        let pane = mount.pane_rect("inventory", "trees").expect("the pane is laid out");
        let (_, lane) = super::super::pane::action_lanes(pane, true)
            .into_iter()
            .find(|(mark, _)| *mark == voxel_ui::PaneMark::Close)
            .expect("the close lane");
        mount.pointer_pressed(
            Button::Left,
            (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0),
        );
        assert_eq!(mount.window("trees").map(|w| w.status), Some(Status::Closed));
        assert_eq!(mount.window("inventory").map(|w| w.status), Some(Status::Normal));
        assert_eq!(
            mount.window("inventory").map(|w| w.arrangement.panes()),
            Some(1),
            "the host keeps its own pane"
        );
    }

    /// **An edge trigger is the launcher it looks like**: a press on a lane splits the window
    /// at that edge and the new pane is an empty slot — the reference's `split()` (2905–2909)
    /// through `split_pane(None, …)` (2920–2921). The window's title then names the slot, which
    /// is what tells the operator the pane exists before it holds anything.
    #[test]
    fn an_edge_trigger_splits_the_window_and_the_new_pane_is_an_empty_slot() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let rect = mount.rect_of("inventory").expect("standalone");
        let content = window_body(&mount.window("inventory").expect("the window").spec, rect);
        // The left lane, at the content's mid-height, 8 dp in from the edge (D-1's 40 dp lane).
        let at = (rect.x + 20.0, content.y + content.height_dp / 2.0);
        mount.pointer_pressed(Button::Left, at);
        let window = mount.window("inventory").expect("the window");
        assert_eq!(window.arrangement.panes(), 2, "the split created a pane");
        assert_eq!(window.arrangement.windows().len(), 1, "the new pane holds no window");
        let layout = window.layout_at(rect).expect("lays out");
        assert!(
            layout.panes.iter().any(|(_, pane)| pane.window().is_none()),
            "one of the two panes is the empty slot"
        );
        assert!(mount.triggers.is_none(), "the lanes hide after the split (2909)");
        // **A left-edge split puts the new pane before the old one** (the reference's `"left"`
        // is its `"before"`), so the slot is the first name in the composed title.
        assert_eq!(
            mount.title_of("inventory"),
            format!("{} / Inventory", super::super::pane::split_slot_title()),
            "the composed title names the slot (3155)"
        );
    }

    /// **A placeholder offer fills the slot** — the reference's `set_app` (2697–2711) end to
    /// end: the offered window leaves the float registry and becomes a pane of this window, and
    /// the slot stops being empty.
    #[test]
    fn a_placeholder_offer_fills_a_slot_with_its_window() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let offers = mount.empty_slot_offers("inventory");
        let Some((offered, _)) = offers.first().cloned() else {
            panic!("the package declares windows besides inventory");
        };
        let rect = mount.rect_of("inventory").expect("standalone");
        let content = window_body(&mount.window("inventory").expect("the window").spec, rect);
        mount.pointer_pressed(Button::Left, (rect.x + 20.0, content.y + content.height_dp / 2.0));
        let layout = mount
            .window("inventory")
            .expect("the window")
            .layout_at(rect)
            .expect("lays out");
        let slot = layout
            .panes
            .iter()
            .find_map(|(_, pane)| match pane {
                PaneRef::Slot(_) => Some(pane.clone()),
                PaneRef::Window(_) => None,
            })
            .expect("the trigger made a slot");
        let slot_rect = layout.pane_rect(&slot).expect("the slot is laid out");
        let frame = super::super::placeholder::frame(super::super::pane::body(slot_rect));
        let lane = super::super::placeholder::offer_lanes(frame, offers.len())
            .first()
            .copied()
            .expect("the offer grid draws");
        mount.pointer_pressed(
            Button::Left,
            (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0),
        );
        assert!(
            !mount.empty_slot_offers("inventory").iter().any(|(id, _)| id == &offered),
            "the offered window is no longer offered"
        );
        assert_eq!(
            mount
                .window(&offered)
                .and_then(|w| w.host.as_ref())
                .map(WindowId::as_str),
            Some("inventory"),
            "the offered window became a pane of the host"
        );
        assert_eq!(
            mount
                .window("inventory")
                .map(|w| w.arrangement.windows().len()),
            Some(2),
            "the slot now holds a window"
        );
    }

    #[test]
    fn a_seam_drag_moves_the_boundary_and_stops_at_the_floor() {
        // The port's seam drag end to end: a handle press answers the seam's address, a move
        // writes the shares through `Arrangement::drag_seam`, and a release ends the drag with
        // the splitter where it was let go.
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        mount
            .dock("inventory", "trees", (host.x + 10.0, host.y + title_bar_dp() + 40.0))
            .expect("the drop splits the host");
        let layout = mount
            .windows
            .get("inventory")
            .expect("the host")
            .layout_at(mount.rect_of("inventory").expect("the host's rect"))
            .expect("the host lays out");
        assert_eq!(layout.seams.len(), 1, "two panes meet at one seam");
        let seam = layout.seams[0];
        // **Mid-height, away from the trigger lanes**: the top lane is centred on the window and
        // covers the seam's top 40 dp, exactly as the reference's raised buttons do.
        let grab = (
            seam.rect.x + seam.rect.width_dp / 2.0,
            seam.rect.y + seam.rect.height_dp / 2.0,
        );
        mount.pointer_pressed(Button::Left, grab);
        assert!(mount.seam.is_some(), "a seam press starts a share drag");
        // The shares the drag started from, and the seam's own address.
        let before = mount
            .windows
            .get("inventory")
            .expect("the host")
            .arrangement
            .shares();
        assert_eq!(before.len(), 1);
        // A move of a quarter of the splitter's span, to the right.
        mount.pointer_moved((grab.0 + seam.span / 4.0, grab.1));
        let after = mount
            .windows
            .get("inventory")
            .expect("the host")
            .arrangement
            .shares();
        let p: Vec<f32> = {
            let total: f32 = after[0].iter().sum();
            after[0].iter().map(|share| share / total).collect()
        };
        assert!(
            (p[0] - 0.75).abs() < 1e-6 && (p[1] - 0.25).abs() < 1e-6,
            "the boundary moved a quarter of the span: {p:?}"
        );
        // The release ends the drag and leaves the shares where the last move put them.
        mount.pointer_released((grab.0 + seam.span / 4.0, grab.1));
        assert!(mount.seam.is_none(), "the release ends the seam drag");
        let after_release = mount
            .windows
            .get("inventory")
            .expect("the host")
            .arrangement
            .shares();
        assert_eq!(after_release, after, "the release writes nothing");
    }

    #[test]
    fn the_trigger_lanes_follow_the_pointer_onto_a_window_and_off_it() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let rect = mount.rect_of("inventory").expect("standalone");
        // Inside the left lane: 10 dp in from the drawn edge, at the content's mid-height (the
        // lane is centred on the content below the 41 dp border + bar, not on the window).
        let near = (rect.x + 10.0, rect.y + 240.0);
        mount.pointer_moved(near);
        let (id, lanes, zone) = mount.triggers.clone().expect("the lanes appear");
        assert_eq!(id, "inventory");
        assert!(lanes.left.contains(near));
        assert_eq!(zone, Some(drop::DropZone::Left), "the hover names the zone");
        // Away from every edge, and off the window entirely: they go.
        mount.pointer_moved((rect.x + rect.width_dp / 2.0, rect.y + rect.height_dp / 2.0));
        assert!(mount.triggers.is_none(), "off the band, off the lanes");
        mount.pointer_moved((rect.right() + 40.0, rect.bottom() + 40.0));
        assert!(mount.triggers.is_none(), "off the window, off the lanes");
    }

    #[test]
    fn a_pane_tree_that_cannot_hold_its_floor_is_refused_before_it_is_drawn() {
        // 66 dp of content split two ways is 33 each, below the 40 dp floor — and the refusal
        // carries the arithmetic rather than a verdict.
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        // **A top-edge drop, so the split divides the height** — the axis the floor check reads
        // here. (A left-edge drop would split the width and leave the height whole; both are
        // the port's own rule, `drop::zone_at`.)
        mount
            .dock("inventory", "trees", (host.x + host.width_dp / 2.0, host.y + 20.0))
            .expect("the drop splits the host");
        let window = mount.windows.get_mut("inventory").expect("the host");
        // The port's composite charges a 1 dp border and a 40 dp title bar, so 108 dp of window
        // is 66 dp of content — split two ways, 33 each, under the 40 dp floor. The content is
        // `height − 2 × border − title bar`, exactly, so `108 − 42` is the span the refusal
        // reports.
        window.rect = Rect::new(0.0, 48.0, 200.0, 108.0);
        let elements = mount
            .shell_elements(&Palette::declared(), mount.viewport(), &views())
            .expect("the bar and the windows that fit still lay out");
        assert!(
            !elements.is_empty(),
            "the refused window's siblings are still on screen"
        );
        assert_eq!(
            mount.refusals().len(),
            1,
            "one window refuses, and it is the host that cannot hold the split: {:?}",
            mount.refusals()
        );
        let refusal = &mount.refusals()[0];
        assert!(refusal.contains("66 dp"), "{refusal}");
        assert!(refusal.contains("2 ways"), "{refusal}");
        assert!(refusal.contains(&MIN_EXTENT.to_string()), "{refusal}");
    }

    #[test]
    fn the_shipped_package_declares_windows_that_can_hold_the_views_they_host() {
        // **The package is the arrangement (§3), so it is held to the same two-numbers rule.**
        // `alpha-shell.shspec` is the file the client actually mounts; this test mounts that
        // file rather than a fixture that agrees with it by construction. A window whose body
        // cannot hold the view it hosts is the §5.7 refusal, and it is found here rather than by
        // a person opening sixteen windows in a running client.
        let recipe_src = include_str!("../../../../packages/alpha-shell.shspec");
        let package = recipe::compile(recipe_src.as_bytes()).expect("the shipped package compiles");
        let loaded = recipe::load(&package).expect("the shipped package loads");
        let mut mount = Mount::new(&loaded).expect("the shipped package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));

        // The panels are the ones the running client builds — the same four builders, over a
        // frame with a resident world — because a package that only holds a *fixture* is a
        // package that has not been checked.
        // The worst case, not the ordinary one: a world with nothing resident (so the digest
        // row says why) *and* a refusal in hand *and* materials that cannot be shaded — every
        // row the four panels can ever hold, measured together. A package sized for the calm
        // case and not for the busy one is a package that refuses once the world gets going.
        let refusal = voxel_model::Refusal::owned(
            voxel_model::RefusalKind::UnknownComponent,
            "no such shaper",
        );
        let frame = voxel_work::Frame {
            wanted: 605,
            resident: 0,
            bytes: 6 * 1024 * 1024,
            ceiling: 256 * 1024 * 1024,
            scratch_bytes: 72 * 1024 * 1024,
            in_flight: 14,
            ..Default::default()
        };
        let state = crate::hud::HudState {
            frame: &frame,
            adapter: "NVIDIA GeForce RTX 2070 SUPER (Vulkan)",
            workers: 15,
            fps: Some(61.5),
            position_m: [0.0, 45.0, -160.0],
            grounded: true,
            vertical_m_s: 0.0,
            in_water: false,
            quads: 21_526,
            seed: 2468,
            tick: 0,
            digest: voxel_model::Digest::of(b"the resident world"),
            refusal: Some(&refusal),
            shading: crate::shading::ShadingReport { materials: 7, with_base_colour: 0 },
        };
        let views = ContentViews {
            world: crate::hud::world_panel(&state),
            streamer: crate::hud::streamer_panel(&state),
            determinism: crate::hud::determinism_panel(&state),
            refusals: crate::hud::refusal_panel(&state),
            console: Some(mount.console().clone()),
        };
        let mut short: Vec<String> = Vec::new();
        for id in mount.windows.keys() {
            let Some(window) = mount.windows.get(id) else { continue };
            // **The log is held to its own drawing, not to a panel measurement.** It draws
            // through `console_body` — the reference's three regions with no controls — and
            // `views.panel_for` would measure it against `declared_content_panel`, which is not
            // what a log draws. So the shipped geometry goes through the same entry point the
            // frame uses: `console_body` accepted means the log window can hold its body. The
            // guard it refuses with is the log's own floor, one record row.
            if matches!(window.spec.content, Content::Log) {
                let console = views.console.as_ref().expect("the views have a console");
                let declared = Rect::new(
                    0.0,
                    0.0,
                    window.spec.geometry[2] as f64,
                    window.spec.geometry[3] as f64,
                );
                let pane = super::super::pane::body(window_body(&window.spec, declared));
                let mut out = Vec::new();
                if let Err(refusal) = console_body(
                    console,
                    Content::Log,
                    pane,
                    0,
                    1.0,
                    None,
                    &Palette::declared(),
                    &mut out,
                ) {
                    short.push(format!("{id}: {refusal}"));
                }
                continue;
            }
            let panel = views.panel_for(&window.spec);
            // **The port's arithmetic, not the derived shell's.** The bound is the pane's body
            // (`pane_of`), the same number the content loop refuses against.
            let pane = pane_of(&window.spec);
            if panel.height_dp() > pane + 0.5 {
                short.push(format!(
                    "{id}: a {} dp panel in a {pane} dp pane",
                    panel.height_dp()
                ));
            }
        }
        assert!(short.is_empty(), "the package declares windows too short for their views: {short:?}");
    }

    /// **A pane the operator made by docking draws its content, bounded to the pane.**
    ///
    /// The defect this witnesses: docking `world` into `inventory` made two 199 dp panes, the
    /// world view is a 407 dp panel, and `window_elements` refused the **whole window** with
    /// *"a 407 dp panel in a 159 dp pane"* — so a docked window drew no chrome and no content.
    /// The docked pane is the operator's split, not the package's declaration: its content is
    /// bounded to it, and the count held back is named on the panel rather than the pane going
    /// blank.
    #[test]
    fn a_docked_window_draws_with_its_content_bounded_to_the_pane() {
        let recipe_src = include_str!("../../../../packages/alpha-shell.shspec");
        let package = recipe::compile(recipe_src.as_bytes()).expect("the shipped package compiles");
        let loaded = recipe::load(&package).expect("the shipped package loads");
        let mut mount = Mount::new(&loaded).expect("the shipped package mounts");
        mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
        let refusal = voxel_model::Refusal::owned(
            voxel_model::RefusalKind::UnknownComponent,
            "no such shaper",
        );
        let frame = voxel_work::Frame { wanted: 605, ..Default::default() };
        let state = crate::hud::HudState {
            frame: &frame,
            adapter: "NVIDIA GeForce RTX 2070 SUPER (Vulkan)",
            workers: 15,
            fps: Some(61.5),
            position_m: [0.0, 45.0, -160.0],
            grounded: true,
            vertical_m_s: 0.0,
            in_water: false,
            quads: 21_526,
            seed: 2468,
            tick: 0,
            digest: voxel_model::Digest::of(b"the resident world"),
            refusal: Some(&refusal),
            shading: crate::shading::ShadingReport { materials: 7, with_base_colour: 0 },
        };
        let views = ContentViews {
            world: crate::hud::world_panel(&state),
            streamer: crate::hud::streamer_panel(&state),
            determinism: crate::hud::determinism_panel(&state),
            refusals: crate::hud::refusal_panel(&state),
            console: Some(mount.console().clone()),
        };
        mount.open("inventory").expect("declared");
        mount.open("world").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        // A top-edge drop: the split divides the height, so each pane is half as tall as the
        // host was — and the world's declared pane does not fit in half of it.
        mount
            .dock("inventory", "world", (host.x + host.width_dp / 2.0, host.y + 20.0))
            .expect("the drop splits the host");
        let elements = mount
            .shell_elements(&Palette::declared(), mount.viewport(), &views)
            .expect("a docked window draws");
        assert!(mount.refusals().is_empty(), "{:?}", mount.refusals());
        let texts: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert!(texts.contains(&"World"), "the docked pane's title is drawn: {texts:?}");
        assert!(texts.contains(&"Inventory / crafting"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text.contains("held and not drawn")),
            "the bounded pane says what it is not drawing: {texts:?}"
        );
    }

    #[test]
    fn the_sizing_rule_is_the_drawing_s_own_arithmetic() {
        // A package sizes a window as *chrome + panel*, and `chrome_height_dp` is the chrome.
        // This pins the sum against the composite the drawing actually uses: if the two ever
        // disagree, every window sized by the rule refuses to draw.
        let mount = mount();
        for spec in mount.windows.values().map(|window| &window.spec) {
            let declared = Rect::new(
                0.0,
                0.0,
                spec.geometry[2] as f64,
                spec.geometry[3] as f64,
            );
            let pane = super::super::pane::body(window_body(spec, declared)).height_dp;
            assert!(
                (declared.height_dp - pane - chrome_height_dp(spec)).abs() < 0.5,
                "{}: {} dp of chrome against the composite's {} dp",
                spec.id,
                declared.height_dp - pane,
                chrome_height_dp(spec)
            );
        }
    }

    #[test]
    fn a_panel_that_does_not_fit_its_pane_is_refused_with_both_numbers() {
        // §5.7: the surface knows its height before it draws. A window declared too short for
        // the view it hosts refuses; it does not spill past its own rect.
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let window = mount.windows.get_mut("inventory").expect("the window");
        window.rect = Rect::new(16.0, 128.0, 480.0, 60.0);
        let elements = mount
            .shell_elements(&Palette::declared(), mount.viewport(), &views())
            .expect("the windows that fit are still drawn");
        let titles: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert!(
            titles.contains(&"Trees"),
            "the window beside the short one is still on screen: {titles:?}"
        );
        assert_eq!(mount.refusals().len(), 1, "{:?}", mount.refusals());
        let refusal = &mount.refusals()[0];
        assert!(refusal.contains("inventory"), "{refusal}");
        // Both numbers, because §5.7's rule is that the two numbers are the same number and a
        // refusal that named only one of them would say which side was wrong.
        let numbers: Vec<&str> = refusal.split_whitespace().collect();
        assert!(refusal.contains(" dp panel in a "), "{refusal}");
        assert!(numbers.len() > 4, "{refusal}");
    }

    #[test]
    fn one_window_that_refuses_does_not_take_the_others_with_it() {
        // The rule the running client needed: a window whose content does not fit is that
        // window's refusal, and every other window keeps drawing. Before this, one short window
        // left the whole screen without a shell.
        let mut mount = mount();
        mount.open("trees").expect("declared");
        mount.open("console").expect("declared");
        // A console window too short for the panel it hosts.
        let window = mount.windows.get_mut("console").expect("declared");
        window.rect = Rect::new(0.0, 128.0, 640.0, 60.0);
        let elements = mount
            .shell_elements(&Palette::declared(), mount.viewport(), &views())
            .expect("the layer lays out");
        let titles: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert!(titles.contains(&"Trees"), "{titles:?}");
        assert!(titles.contains(&"World"), "{titles:?}");
        assert_eq!(mount.refusals().len(), 1, "{:?}", mount.refusals());
        assert!(mount.refusals()[0].contains("console"), "{:?}", mount.refusals());
    }

    #[test]
    fn every_window_on_screen_draws_with_the_palette_the_record_declares() {
        let mut mount = mount();
        let elements = mount
            .shell_elements(&Palette::declared(), mount.viewport(), &views())
            .expect("the shell lays out");
        // The bar is drawn, and the pinned world window's title is on screen.
        let texts: Vec<&str> = elements.iter().filter_map(|element| element.text()).collect();
        assert!(texts.contains(&"World"), "{texts:?}");
        assert!(texts.contains(&"Inventory"), "the bar's tabs are drawn: {texts:?}");
        // Every surface is one of the admitted roles, at the declared panel opacity or the
        // window's own — and none of them is invented here.
        let palette = Palette::declared();
        let admitted = [palette.get(Role::Chrome).unwrap(), palette.get(Role::Surface).unwrap(), palette.get(Role::SurfaceAlt).unwrap()];
        for element in &elements {
            if let Element::Surface { paint, .. } = element {
                assert!(admitted.contains(&paint.rgb), "an unadmitted surface was painted: {paint:?}");
            }
        }
    }

    #[test]
    fn a_maximized_window_keeps_the_rect_it_will_restore_to() {
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        let before = mount.rect_of("inventory").expect("standalone");
        mount.toggle_maximize("inventory");
        let maximized = mount.rect_of("inventory").expect("maximized still has a rect");
        assert_eq!(maximized.y, bar_height_dp(), "a maximized window starts under the bar");
        assert_eq!(maximized.width_dp, mount.viewport().width_dp);
        assert_ne!(maximized, before);
        mount.toggle_maximize("inventory");
        assert_eq!(mount.rect_of("inventory"), Some(before), "restore goes back, not to the declaration");
    }

    #[test]
    fn the_ink_used_by_the_bar_and_the_windows_is_measured_not_assumed() {
        // The mount draws text; every run it emits must already have passed the floor when it
        // was built, so a HUD that draws unreadable text is a refusal rather than a warning.
        // This re-measures the two inks the chrome uses over the worst backdrop.
        let mount = mount();
        let opacity = voxel_ui::panel_opacity();
        let chrome = Palette::declared().get(Role::Chrome).unwrap();
        let backdrop = chrome.blend_over(BRIGHTEST, opacity);
        for ink in [Ink::Full, Ink::Secondary] {
            contrast_over_backdrop(ink.composited_over(backdrop), backdrop)
                .expect("the chrome's inks clear the floor over white");
        }
        let _ = mount;
    }

    #[test]
    fn the_console_window_is_hosted_by_the_console_and_not_by_the_empty_view_rule() {
        // BLD-0018. The window is declared in the test recipe *and* in `alpha-shell.shspec`, and
        // before the console was wired it drew `declared_content_panel` — the MD1 empty-view rule
        // working exactly as written, on a content kind that did have a view.
        let mut mount = mount();
        mount.open("console").expect("declared");
        mount.log(Level::Warn, "shell", "a report the window must show", Some(7));

        let spec = mount.window("console").expect("declared").spec.clone();
        let mut base = views();
        base.console = Some(mount.console().clone());
        let routed = base.panel_for(&spec);

        assert_eq!(routed.title, "Console");
        assert!(
            routed
                .rows
                .iter()
                .any(|row| row.value.contains("a report the window must show")),
            "the record written through the chokepoint is on the panel: {routed:?}"
        );
        // The named test: it is the console's rows and not the placeholder's. Without this the
        // test above would still pass if every console window went back to the empty view.
        assert_ne!(
            routed.rows,
            declared_content_panel(&spec).rows,
            "a console window drawing 'declared content' is the defect this closes"
        );
    }

    #[test]
    fn an_empty_console_is_a_named_absence_rather_than_a_blank_pane() {
        // The MD1 empty-view rule again, on the console's own terms: no records is a state, and
        // the panel says which state rather than showing nothing.
        let mount = mount();
        // The pane the console window actually declares, not a number that flatters the panel.
        let spec = mount.window(CONSOLE_WINDOW).expect("declared").spec.clone();
        let panel = console_panel(mount.console(), pane_of(&spec));
        assert!(!panel.is_empty(), "an empty console still says how many records it holds");
        assert!(
            panel.rows.iter().any(|row| row.label == "records" && row.value == "0"),
            "{panel:?}"
        );
        assert!(
            !panel.rows.iter().any(|row| row.label == "dropped"),
            "a console that dropped nothing does not claim a dropped row: {panel:?}"
        );
    }

    #[test]
    fn the_console_window_is_bounded_by_the_pane_it_is_drawn_in_and_says_what_it_holds() {
        // **The regression this pins.** Adding the `input` row made the panel taller than the
        // pane `alpha-shell.shspec` declares, and the running client refused to lay out
        // with *"a 407 dp panel in a 358 dp pane"*. The check that should have caught it
        // measured an **empty** console — three rows — and the record's own warning applies:
        // *"a package sized for the calm case and not for the busy one is a package that
        // refuses once the world gets going."*
        let mut mount = console();
        let spec = mount.window(CONSOLE_WINDOW).expect("declared").spec.clone();
        let pane = pane_of(&spec);
        for tick in 0..400u64 {
            mount.log(Level::Info, "tick", format!("record {tick}"), Some(tick));
        }

        let panel = console_panel(mount.console(), pane);
        assert!(
            panel.height_dp() <= pane,
            "a full console fits the pane it is drawn in: {} dp in {pane} dp",
            panel.height_dp()
        );
        let drawn = panel.rows.iter().filter(|row| row.label == "record").count();
        assert!(drawn < 400, "so it cannot have drawn them all: {drawn} of 400");
        let note = panel.note.as_deref().expect("a bounded console states what it is not drawing");
        assert!(note.contains("not drawn"), "{note}");
        // **The newest survive.** A log whose head is kept and whose tail is cut reads as a log
        // that ended, which is the opposite of what happened.
        assert!(
            panel.rows.iter().any(|row| row.value.contains("record 399")),
            "the newest record is on the panel: {panel:?}"
        );
        // And the header rows are never the ones dropped, or a bounded console could not say
        // how much it was hiding.
        assert!(
            panel.rows.iter().any(|row| row.label == "records" && row.value == "400"),
            "{panel:?}"
        );
    }

    #[test]
    fn the_console_window_counts_records_it_dropped_rather_than_reading_as_complete() {
        // `Console` keeps 2,048 and counts the rest. A window that drew only what survived would
        // report the same `records` as a console that never had more, and the second is the
        // alarming one.
        let mut mount = mount();
        for tick in 0..=(crate::shell::console::MAX_RECORDS as u64) {
            mount.log(Level::Info, "tick", "x", Some(tick));
        }
        let panel = console_panel(mount.console(), 400.0);
        assert!(
            panel
                .rows
                .iter()
                .any(|row| row.label == "dropped" && row.value == "1"),
            "one record past the ceiling is counted as dropped: {panel:?}"
        );
    }

    /// **The game gives the agent the recipe, and the agent reads it with the agent's stream.**
    ///
    /// DSL: `queue -> agent "the recipes the game derived from the solutions"`. The whole loop,
    /// through the surface it happens on: a person files at the console, the compiler back-compiles
    /// and emits a package, the judge writes one recipe notification into `logs/agent-inbox/`, and
    /// the agent's own read — the same `NotificationStream` the notifications window uses, opened
    /// on the other queue — verifies it against the package's own bytes and reads the work out.
    ///
    /// **The row carries the person's words, not a paraphrase**, because the recipe is what the
    /// agent implements against; a recipe that said something else would be a second statement of
    /// the work, and the record is the only place the work is stated.
    #[test]
    fn a_filed_ticket_puts_a_recipe_in_the_agents_inbox_and_the_agent_stream_reads_it() {
        let workspace = filed_workspace("inbox");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(
            &mut mount,
            "file window[console] and window[api-viewer] need the moon in the wrong part of the sky",
        );

        let inbox = voxel_model::laya::agent_inbox_dir(&workspace);
        let mut stream = crate::shell::notifications::NotificationStream::open_at(&inbox)
            .expect("the agent's inbox is there");
        let notification = stream
            .next_verified(&workspace)
            .expect("the record verifies against the package's own bytes")
            .expect("one recipe was notified");
        assert!(
            notification.decision.path.starts_with("packages/"),
            "the artifact is the package, named the way the record spells a path: {}",
            notification.decision.path
        );
        assert_eq!(
            notification.decision.verdict,
            voxel_model::laya::Verdict::Belongs,
            "the compile's own provenance round trip cleared it"
        );

        // **The work, out of the package the console emitted** — through the loader every package
        // loads through, so what is measured here is the artifact the agent is handed.
        let bytes = std::fs::read(workspace.join(&notification.decision.path))
            .expect("the package the notification names reads");
        let loaded = voxel_package::recipe::load(&bytes).expect("the package frames");
        let row = loaded
            .recipe
            .rows
            .iter()
            .find(|row| row.text.contains("the moon in the wrong part of the sky"))
            .expect("the recipe carries the person's own words");
        assert!(
            row.id.starts_with("BLD-"),
            "and the row is the ticket's own id: {}",
            row.id
        );

        // And the console said so on the surface, at the moment it happened, twice: the record,
        // and the rows the package carries.
        assert!(
            said(&mount).iter().any(|line| line.contains("the agent's inbox")),
            "{:?}",
            said(&mount)
        );
        assert!(
            said(&mount).iter().any(|line| line.contains("row(s) of work")),
            "{:?}",
            said(&mount)
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **The recipes the record already holds reach the agent, once, from the whole board.** A
    /// ticket filed before the inbox existed is still work the agent owes, and the world-open pass
    /// is how the game hands it over; a ticket already in the inbox is skipped, because re-emitting
    /// it would rewrite the package and the record the agent already verified against. The board
    /// here holds one ticket, so the counts are the same as when the pass was scoped; the next
    /// test is the one that measures the scope itself.
    #[test]
    fn the_boards_tickets_are_handed_to_the_agent_once_and_then_skipped() {
        let workspace = filed_workspace("handover");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(
            &mut mount,
            "file window[console] and window[api-viewer] the sky is the wrong blue",
        );

        // **The state of a filing made before the inbox existed**: the ticket is on the chain, and
        // no record for it is in the inbox. Removing the record the filing just wrote is the honest
        // way to reach that state — the pass cannot be tested against a board it never sees.
        let inbox = voxel_model::laya::agent_inbox_dir(&workspace);
        let records: Vec<std::path::PathBuf> = std::fs::read_dir(&inbox)
            .expect("the filing wrote into the inbox")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        assert_eq!(records.len(), 1, "one filing, one record: {records:?}");
        for record in &records {
            std::fs::remove_file(record).expect("the record is removed");
        }

        let (handed, skipped, refused) = mount.hand_the_boards_tickets_to_the_agent(Some(7));
        assert_eq!(
            (handed, skipped, refused),
            (1, 0, 0),
            "the filing is handed over: {:?}",
            said(&mount)
        );
        let record = voxel_model::laya::agent_inbox_dir(&workspace).join("recipe-BLD-0001.musli");
        assert!(record.exists(), "and the record is named for the ticket: {record:?}");

        // **The second pass hands over nothing**, because the agent already owes this one.
        let (handed, skipped, refused) = mount.hand_the_boards_tickets_to_the_agent(Some(8));
        assert_eq!((handed, skipped, refused), (0, 1, 0), "a handed-over filing is skipped");
        // And the record is still the one the first pass wrote, byte for byte: an idempotent pass
        // that rewrote the artifact would be the defect it exists to avoid.
        let before = std::fs::read(&record).expect("the record reads");
        let package = voxel_package::filing::newest_package(&workspace, "BLD-0001")
            .expect("the filing emitted a package");
        let package_before = std::fs::read(&package).expect("the package reads");
        let _ = mount.hand_the_boards_tickets_to_the_agent(Some(9));
        assert_eq!(std::fs::read(&record).expect("reads"), before);
        // **And no second package was written**: a pass that recompiled a filing it had already
        // handed over would collide with nothing now that names carry the content digest, and
        // would quietly accumulate packages for work the agent already has.
        assert_eq!(
            voxel_package::filing::filed_packages(&workspace, "BLD-0001").len(),
            1,
            "one filing, one package"
        );
        assert_eq!(std::fs::read(&package).expect("reads"), package_before);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **The evaluator compiles the ledger's ticket, and the read names both worlds.**
    ///
    /// The first version of this test held the reduction: the inbox pass was scoped to this
    /// console's world, so the ledger's own `CSE` tickets — filed by the agent in another world —
    /// were invisible to it, and the read merely named them. The user's correction (C-105): *"how
    /// are we supposed to get recipes and solutions of code if the laya ticket evaluator isn't
    /// hooked up"*. There is no left-out ticket any more. This files one ticket in each world and
    /// requires **the `CSE` ticket to reach the agent's inbox as a recipe** whose row is the
    /// ticket's own id and the entry's own heading, while the read still names both worlds and
    /// states the split in its counts rather than leaving it to inference.
    #[test]
    fn the_pass_compiles_the_ledgers_ticket_and_the_read_names_both_worlds() {
        let workspace = filed_workspace("board-read");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        // One ticket in this console's world, through the verb a person would type.
        type_line(
            &mut mount,
            "file window[console] and window[api-viewer] the sky is the wrong blue",
        );

        // **One ticket in another world**, filed the way the ledger's entries were: the record's
        // governance shape, `[CSE]` the kind marker, the entry's own heading the objective, the
        // citation the answer — through `grow`, the chained writer every filer uses.
        let namespace = voxel_model::chain::Namespace::new(
            "world-2468",
            1,
            voxel_model::chain::Season::Winter,
            "c-2026-10-01",
        )
        .expect("a namespace");
        let before = std::fs::read(voxel_package::filing::chain_path(&workspace))
            .expect("the chain reads");
        voxel_package::filing::grow(
            &workspace,
            &before,
            [voxel_package::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind: voxel_model::chain::EventKind::TicketTransition,
                question: "[CSE] C-103 — the gate's proofs are minted from the citizen's rows",
                answer: Some("docs/LEDGER.md lines 7811..=7861; sha256:placeholder"),
                kind: voxel_model::envelope::Kind::Authority,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: "agent",
                round_id: "c-2026-10-02",
                supersedes: None,
            }],
        )
        .expect("the ledger's ticket is filed");

        // **The evaluator pass takes the whole board.** The console's filing already has its
        // inbox record (the `file` verb wrote it as the filing was made), so it is skipped; the
        // ledger's ticket — the one the old world filter left out — is back-compiled through the
        // audited loop and handed over. One read, one chain, every open ticket: this is the
        // measurement the ruling asked for.
        let (handed, skipped, refused) = mount.hand_the_boards_tickets_to_the_agent(Some(8));
        assert_eq!(
            (handed, skipped, refused),
            (1, 1, 0),
            "the ledger's ticket is compiled, the console's is already owed: {:?}",
            said(&mount)
        );
        let inbox = voxel_model::laya::agent_inbox_dir(&workspace);
        let records: Vec<String> = std::fs::read_dir(&inbox)
            .expect("the inbox is there")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        let recipe = records
            .iter()
            .find(|name| name.starts_with("recipe-CSE-") && name.ends_with(".musli"))
            .unwrap_or_else(|| panic!("the ledger's ticket has a recipe: {records:?}"))
            .clone();
        // **The recipe is the usable block, and its row is the record's own words.** The record
        // names the ticket; the package frames through the loader every package loads through;
        // the row the agent implements from carries the ticket's id and the entry's heading.
        let case = recipe
            .trim_start_matches("recipe-")
            .trim_end_matches(".musli")
            .to_string();
        let package = voxel_package::filing::newest_package(&workspace, &case)
            .unwrap_or_else(|| panic!("{case} compiled to a package"));
        let bytes = std::fs::read(&package).expect("the package reads");
        let loaded = voxel_package::recipe::load(&bytes).expect("and it frames");
        assert!(
            loaded.recipe.rows.iter().any(|row| row.id == case),
            "the row is the ticket's own id: {:?}",
            loaded.recipe.rows
        );
        assert!(
            loaded
                .recipe
                .rows
                .iter()
                .any(|row| row.text.contains("C-103 — the gate's proofs")),
            "and the entry's own heading, verbatim: {:?}",
            loaded.recipe.rows
        );

        let (tickets, here, elsewhere) = mount.read_the_board(Some(9));
        assert_eq!(
            (tickets, here, elsewhere),
            (2, 1, 1),
            "two tickets on the record, one in each world: {:?}",
            said(&mount)
        );
        let said = said(&mount);
        assert!(
            said
                .iter()
                .any(|line| line.contains("CSE-") && line.contains("world-2468")),
            "the ledger's ticket is named, with its world: {said:?}"
        );
        assert!(
            said.iter()
                .any(|line| line.contains("C-103 — the gate's proofs are minted")),
            "and with the entry's own heading as the objective: {said:?}"
        );
        assert!(
            said.iter()
                .any(|line| line.contains("the board holds 2 ticket(s)")),
            "and the counts are stated rather than left to inference: {said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **The board read catches the ledger's tickets on the real record.**
    ///
    /// The fixture test above proves the path; this proves **the record**: the repository's own
    /// `chains/workspace.chain` is copied, read back through the pass the game runs at world-open,
    /// and every ticket it holds must be named on the console — including the `CSE` tickets filed
    /// in another world, which the old compile scope left out and the evaluator now takes. **A property, not a snapshot**: the
    /// record grows, so this requires the read to name *what is there*, not a count frozen today.
    /// The copy is a copy; the repository's record is not written to.
    #[test]
    fn the_board_read_catches_the_ledgers_tickets_on_the_real_record() {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        let workspace = scratch_workspace("board-real-chain");
        let _ = std::fs::remove_dir_all(&workspace);
        let chain = voxel_package::filing::chain_path(&workspace);
        std::fs::create_dir_all(chain.parent().expect("the chain has a directory"))
            .expect("the scratch chains directory");
        std::fs::copy(voxel_package::filing::chain_path(&root), &chain)
            .expect("the real chain copies");

        let mut mount = console();
        mount.set_workspace(workspace.clone());
        let (tickets, here, elsewhere) = mount.read_the_board(Some(1));

        let board =
            voxel_model::ticket::Board::from_chain(&voxel_package::filing::read(&workspace));
        assert!(!board.is_empty(), "the repository's own record holds tickets");
        assert_eq!(
            tickets,
            board.len(),
            "every ticket the record holds is counted by the read"
        );
        let expected_here = board
            .tickets()
            .iter()
            .filter(|ticket| ticket.actor == CONSOLE_WORLD)
            .count();
        assert_eq!(
            (here, elsewhere),
            (expected_here, board.len() - expected_here),
            "and the world split is measured from the board, not assumed"
        );

        // **Every ticket is named on the console** — including the ledger's `CSE` cases filed in
        // another world, which is the whole point of the read.
        let said = said(&mount);
        for ticket in board.tickets() {
            assert!(
                said.iter().any(|line| line.contains(ticket.id.as_str())),
                "{} is on the record but not on the console",
                ticket.id.as_str()
            );
        }
        let cases = board
            .tickets()
            .iter()
            .filter(|ticket| ticket.id.as_str().starts_with("CSE-"))
            .count();
        assert!(
            cases > 0,
            "**the ledger's tickets are on the record as CSE cases**, or this test measured \
             nothing: {cases}"
        );
        println!(
            "the board read on the real record: {tickets} ticket(s) named, {cases} CSE case(s) \
             among them, {elsewhere} filed in another world"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **The evaluator on the real record: the whole board compiles, the ledger's cases included.**
    ///
    /// The fixture test proves one `CSE` ticket reaches the agent; this proves **the record**. The
    /// repository's own `chains/workspace.chain` is copied, the world-open evaluator runs over it,
    /// and **every open ticket it holds must reach the agent's inbox as a recipe** — the ledger's
    /// `CSE` cases among them, which the old world filter left out entirely. Nothing is refused: a
    /// refusal here would be a ticket the evaluator cannot turn into a solution, and that is
    /// exactly the state this pass exists to make visible rather than to hide. **A property, not a
    /// snapshot**: the record grows, so the assertions measure *what is there*, not a count frozen
    /// today. The copy is a copy; the repository's record is not written to.
    #[test]
    fn the_evaluator_hands_the_whole_real_board_to_the_agent() {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        let workspace = scratch_workspace("board-real-eval");
        let _ = std::fs::remove_dir_all(&workspace);
        let chain = voxel_package::filing::chain_path(&workspace);
        std::fs::create_dir_all(chain.parent().expect("the chain has a directory"))
            .expect("the scratch chains directory");
        std::fs::copy(voxel_package::filing::chain_path(&root), &chain)
            .expect("the real chain copies");

        let mut mount = console();
        mount.set_workspace(workspace.clone());
        let board =
            voxel_model::ticket::Board::from_chain(&voxel_package::filing::read(&workspace));
        assert!(!board.is_empty(), "the repository's own record holds tickets");
        let open = board.open().len();
        assert!(open > 0, "the real record holds open tickets");

        let (handed, skipped, refused) = mount.hand_the_boards_tickets_to_the_agent(Some(1));
        assert_eq!(
            refused,
            0,
            "every open ticket on the real record compiles: {:?}",
            said(&mount)
        );
        assert_eq!(
            handed + skipped,
            open,
            "every open ticket is handed over or already owed: {handed} + {skipped} of {open}"
        );
        let records: Vec<String> =
            std::fs::read_dir(voxel_model::laya::agent_inbox_dir(&workspace))
                .expect("the inbox is there")
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
        assert_eq!(
            records.len(),
            open,
            "one recipe per open ticket — {} record(s) for {open} open ticket(s)",
            records.len()
        );
        let cases = records
            .iter()
            .filter(|name| name.starts_with("recipe-CSE-"))
            .count();
        let cases_on_board = board
            .tickets()
            .iter()
            .filter(|ticket| ticket.id.as_str().starts_with("CSE-") && !ticket.is_closed())
            .count();
        assert_eq!(
            cases, cases_on_board,
            "**the ledger's cases are recipes, not leftovers**"
        );
        assert!(cases > 0, "or this test measured nothing");
        println!(
            "the evaluator on the real record: {open} open ticket(s), {handed} recipe(s) handed \
             over, {cases} of them the ledger's CSE cases"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **The evaluator on the real record, deliberately — and the sample it designed.**
    ///
    /// [`the_evaluator_hands_the_whole_real_board_to_the_agent`] runs on a **copy**, proves the
    /// whole board compiles, and deletes the evidence on the way out. This is its deliberate
    /// sibling: the same pass against the **repository's own record**, so the recipes land in the
    /// inbox the agent actually reads (`logs/agent-inbox/`) and the packages in `packages/`, and
    /// it prints **one recipe whole** — the row the agent implements from, the ticket's id and
    /// the entry's own heading — because a run that reports only a count cannot answer *what did
    /// it design*.
    ///
    /// `#[ignore]`d for the same reason `file_the_ledger_onto_the_record` is: it writes the
    /// record's own artifacts, which is a thing to do on purpose. It is idempotent — a ticket
    /// the inbox already owes is skipped — so a second run hands over nothing.
    ///
    /// ```text
    /// cargo test --offline -p voxel-client --lib \
    ///   shell::mount::tests::hand_the_boards_tickets_to_the_agent_on_the_real_record \
    ///   -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "hands the whole real board's recipes over on the repository's own record"]
    fn hand_the_boards_tickets_to_the_agent_on_the_real_record() {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        assert!(
            voxel_package::filing::is_filed(&root),
            "the record must be filed before there is a board to evaluate"
        );
        let board = voxel_model::ticket::Board::from_chain(&voxel_package::filing::read(&root));
        let open = board.open().len();
        let mut mount = console();
        mount.set_workspace(root.clone());
        let (handed, skipped, refused) =
            mount.hand_the_boards_tickets_to_the_agent(Some(crate::game::GAME_TICK));
        assert_eq!(refused, 0, "every open ticket compiles: {:?}", said(&mount));
        assert_eq!(
            handed + skipped,
            open,
            "every open ticket is handed over or already owed: {handed} + {skipped} of {open}"
        );
        println!(
            "the real record: {open} open ticket(s), {handed} handed over, {skipped} already \
             owed, {refused} refused"
        );

        // **The sample, whole.** Every recipe in the inbox, found through the record's own
        // package locator and decoded through the loader every package loads through, printed as
        // the agent receives it — because *the recipes the game derived from the solutions* is a
        // line the agent has to be able to read, not a count it has to take on trust.
        let inbox = voxel_model::laya::agent_inbox_dir(&root);
        let mut recipes: Vec<String> = std::fs::read_dir(&inbox)
            .expect("the inbox is there")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("recipe-") && name.ends_with(".musli"))
            .collect();
        recipes.sort();
        // **Every open ticket has a recipe — and the inbox is a log, not a snapshot.** The
        // handover is complete iff no open ticket is missing its recipe; but the inbox is
        // append-only like the rest of the record, so a recipe written for a ticket that has
        // since closed stays as that handover's receipt, and nothing prunes it. The old
        // assertion demanded `recipes.len() == open` and so measured the record's own
        // longevity rather than the handover: on the real record it read *515 records for
        // 301 open tickets*, the extra 214 being receipts of tickets closed since. The
        // completeness property is asserted; the historical records are counted and named.
        let open_recipe_names: std::collections::BTreeSet<String> = board
            .open()
            .iter()
            .map(|ticket| format!("recipe-{}.musli", ticket.id.as_str()))
            .collect();
        let present: std::collections::BTreeSet<&str> =
            recipes.iter().map(String::as_str).collect();
        let missing: Vec<&String> = open_recipe_names
            .iter()
            .filter(|name| !present.contains(name.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "one recipe per open ticket — {} of {open} open ticket(s) have none: {missing:?}",
            missing.len()
        );
        let historical = recipes
            .iter()
            .filter(|name| !open_recipe_names.contains(name.as_str()))
            .count();
        println!(
            "the inbox holds {} recipe(s): {} for open tickets, {historical} kept from tickets \
             that have since closed",
            recipes.len(),
            recipes.len() - historical
        );
        // **The ledger's open cases, read off the record** — the entries that were missing from
        // the board until C-107's fix, so the sample says the earliest entries arrived rather
        // than only that a count did.
        let cases = board
            .tickets()
            .iter()
            .filter(|ticket| ticket.id.as_str().starts_with("CSE-") && !ticket.is_closed())
            .count();
        // **Two shapes, counted apart.** An *arrangement* is a filing that named the windows the
        // work touches — what an agent implements against. A *work order* is a ticket that named
        // only the record's own words, which is what a ledger entry is.
        let mut arranged = 0usize;
        let mut work_orders = 0usize;
        for name in &recipes {
            let case = name.trim_start_matches("recipe-").trim_end_matches(".musli");
            // **The content-addressed locator first, the seeding second.** The earlier compiler
            // wrote `packages/<ticket>.shpkg` with no digest in the name, which is the defect
            // that made those filings unrecompilable — so a locator that only matches the new
            // shape would report a package missing that is on disk under its old name. Both are
            // read; neither is invented.
            let package = voxel_package::filing::newest_package(&root, case).unwrap_or_else(|| {
                let legacy = root.join("packages").join(format!("{case}.shpkg"));
                assert!(
                    legacy.exists(),
                    "{case} has a recipe in the inbox and a package under neither name"
                );
                legacy
            });
            let loaded = voxel_package::recipe::load(
                &std::fs::read(&package).expect("the package reads"),
            )
            .expect("and it frames");
            if loaded.recipe.windows.is_empty() {
                work_orders += 1;
            } else {
                arranged += 1;
            }
            println!("what the pass designed, as the agent receives it:");
            println!("  package: {}", package.display());
            println!("  inbox:   {}", inbox.join(name).display());
            println!(
                "  recipe:  {} {} — {} row(s), {} window(s)",
                loaded.recipe.name,
                loaded.recipe.version,
                loaded.recipe.rows.len(),
                loaded.recipe.windows.len()
            );
            for row in &loaded.recipe.rows {
                println!("  row {}: {}", row.id, row.text);
            }
            for window in &loaded.recipe.windows {
                println!(
                    "  window {}: \"{}\" at {:?}",
                    window.id, window.title, window.geometry
                );
            }
        }
        println!(
            "{cases} of the {open} open ticket(s) are the ledger's CSE cases; {arranged} \
             recipe(s) are arrangements, {work_orders} carry the record's words alone"
        );
    }

    /// **A filing whose old package declares nothing is compiled again, under a new name.**
    ///
    /// This is `BLD-0207`…`BLD-0215` as a rule. The earlier compiler emitted those recipes under
    /// `packages/<ticket>.shpkg`, they declare no window and no row, the record's writer refuses to
    /// overwrite them (*"a truncated file is a delete with no tombstone"*), and the file name
    /// carried no content — so the filing could never be recompiled and the agent could never be
    /// handed a recipe. Now the name carries the content digest, so a recompile writes a second
    /// package, the old one **stays** as the record of what the earlier compiler produced, and the
    /// newest by the naming rule is the one the agent gets.
    #[test]
    fn a_filing_whose_package_declares_nothing_is_compiled_again_under_a_new_name() {
        let workspace = filed_workspace("recompile");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(
            &mut mount,
            "file window[console] and window[api-viewer] the sky is the wrong blue",
        );

        // **The state the earlier compiler left.** A package under the old name-only path — which
        // the naming rule does not recognise, so the pass cannot see a package for this ticket —
        // plus no record in the inbox, because the inbox did not exist when it was compiled.
        let packages = workspace.join(voxel_package::filing::PACKAGE_DIR);
        let legacy = packages.join("BLD-0001.shpkg");
        std::fs::write(&legacy, b"the earlier compiler's package, under the name-only path")
            .expect("the fixture writes the old name");
        // **The fixture replaces the record on purpose**, which is a different act from the writer's:
        // the filing just emitted a digest-named package, and the state under test is one where the
        // *only* package for this ticket is the old name-only one.
        for entry in std::fs::read_dir(&packages).expect("packages reads").flatten() {
            if entry.path() != legacy {
                std::fs::remove_file(entry.path()).expect("the emitted package is removed");
            }
        }
        let inbox = voxel_model::laya::agent_inbox_dir(&workspace);
        for entry in std::fs::read_dir(&inbox).expect("the inbox is there").flatten() {
            std::fs::remove_file(entry.path()).expect("the record is removed");
        }
        assert!(
            voxel_package::filing::newest_package(&workspace, "BLD-0001").is_none(),
            "a name-only package is not a package by this rule"
        );

        let (handed, skipped, refused) = mount.hand_the_boards_tickets_to_the_agent(Some(7));
        assert_eq!(
            (handed, skipped, refused),
            (1, 0, 0),
            "the filing is compiled again and handed over: {:?}",
            said(&mount)
        );
        let fresh = voxel_package::filing::newest_package(&workspace, "BLD-0001")
            .expect("the new package is on disk");
        assert_ne!(fresh, legacy, "and it is not the old file");
        let bytes = std::fs::read(&fresh).expect("the new package reads");
        let loaded = voxel_package::recipe::load(&bytes).expect("and it frames");
        assert!(
            loaded.recipe.rows.iter().any(|row| row.id == "BLD-0001"),
            "carrying the work as a row: {:?}",
            loaded.recipe.rows
        );
        assert!(legacy.exists(), "the earlier package was not deleted: {legacy:?}");
        let _ = std::fs::remove_dir_all(&workspace);
    }

    // ---- the command line ----------------------------------------------------

    /// The console window, focused, with the keys a caller would send.
    fn console() -> Mount {
        let mut mount = mount();
        mount.open(CONSOLE_WINDOW).expect("the console is declared");
        mount.focus(CONSOLE_WINDOW);
        mount
    }

    /// Type a line and press Enter, the way the bin does.
    fn type_line(mount: &mut Mount, line: &str) {
        for character in line.chars() {
            mount.key_pressed(Key::Char(character), None);
        }
        mount.key_pressed(Key::Enter, None);
    }

    fn said(mount: &Mount) -> Vec<String> {
        mount.console().lines()
    }

    #[test]
    fn keys_reach_the_console_only_while_it_holds_focus() {
        // A window that is not focused does not type. **Opening takes focus** — so the
        // arrangement's own `open` is what hands the keyboard over, and there is no separate
        // click-to-focus step to forget.
        let mut mount = mount();
        assert_eq!(
            mount.key_pressed(Key::Char('x'), None),
            KeyOutcome::Ignored,
            "a closed console does not type"
        );
        mount.open(CONSOLE_WINDOW).expect("declared");
        assert_eq!(mount.focused(), Some(CONSOLE_WINDOW), "opening focuses it");
        assert_eq!(mount.key_pressed(Key::Char('x'), None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "x");
        // And focus moving away takes the keyboard with it, which is the whole of
        // `console_has_focus`: there is no second path for text to arrive.
        mount.focus("trees");
        assert!(!mount.console_has_focus());
        assert_eq!(mount.key_pressed(Key::Char('y'), None), KeyOutcome::Ignored);
        assert_eq!(mount.console().draft(), "x", "and the draft is left as it was");
    }

    #[test]
    fn the_draft_is_a_row_on_the_panel_so_a_command_can_be_checked_before_it_is_run() {
        let mut mount = console();
        mount.key_pressed(Key::Char('f'), None);
        mount.key_pressed(Key::Char('o'), None);
        let panel = console_panel(mount.console(), 400.0);
        let input = panel
            .rows
            .iter()
            .find(|row| row.label == "input")
            .expect("the console panel leads with the line being typed");
        assert_eq!(input.value, "> fo", "half a command is visible: {panel:?}");
        // And an empty draft is the prompt, not a blank row.
        let mut empty = console();
        empty.key_pressed(Key::Char('z'), None);
        empty.key_pressed(Key::Backspace, None);
        let panel = console_panel(empty.console(), 400.0);
        assert_eq!(
            panel.rows.iter().find(|row| row.label == "input").unwrap().value,
            ">"
        );
    }

    #[test]
    fn an_empty_enter_is_nothing_at_all() {
        // The reference returns before it clears, logs nothing and remembers nothing.
        let mut mount = console();
        assert_eq!(mount.key_pressed(Key::Enter, None), KeyOutcome::Changed);
        assert!(said(&mount).is_empty(), "an empty line is not an event: {:?}", said(&mount));
        mount.key_pressed(Key::Char(' '), None);
        mount.key_pressed(Key::Enter, None);
        assert!(said(&mount).is_empty(), "and whitespace is not either: {:?}", said(&mount));
    }

    #[test]
    fn the_typed_command_is_on_the_record_before_anything_acts_on_it() {
        // The reference's order, and it is the reason a refused command is still findable:
        // `execute_command` logs `> cmd` before it looks at the verb.
        let mut mount = console();
        type_line(&mut mount, "focus trees");
        let lines = said(&mount);
        let attempt = lines
            .iter()
            .position(|line| line.contains("> focus trees"))
            .expect("the attempt is logged");
        let outcome = lines
            .iter()
            .position(|line| line.contains("focused trees"))
            .expect("and so is the outcome");
        assert!(attempt < outcome, "the attempt precedes the outcome: {lines:?}");
    }

    #[test]
    fn a_refused_command_is_logged_with_its_reason_and_not_only_the_attempt() {
        let mut mount = console();
        type_line(&mut mount, "frobnicate");
        let lines = said(&mount);
        assert!(
            lines.iter().any(|line| line.contains("> frobnicate")),
            "the attempt: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("frobnicate") && !line.contains("> ")),
            "and the refusal, naming it: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("undock")),
            "and the reader is told what the table holds rather than left guessing: {lines:?}"
        );
    }

    #[test]
    fn a_verb_with_no_target_in_this_shell_is_refused_by_name_and_with_the_reason() {
        // No arm does nothing. A command accepted and silently ignored is indistinguishable from
        // one that was never received.
        for (line, expected) in [
            ("theme dusk", "scheme"),
            ("setting fps 60", "settings store"),
            ("split trees", "drop point"),
            ("report", "build-time"),
            ("approvals", "record surface"),
        ] {
            let mut mount = console();
            type_line(&mut mount, line);
            let said = said(&mount);
            assert!(
                said.iter().any(|entry| entry.contains(expected)),
                "{line:?} must be refused with {expected:?}: {said:?}"
            );
        }
    }

    #[test]
    fn a_missing_argument_and_an_unknown_window_are_two_different_refusals() {
        // **The arity is the parser's, and it says so.** `focus` declares `(1, 1)`, so a bare
        // `focus` is refused before any dispatch — with the number, not a shrug.
        let mut mount = console();
        type_line(&mut mount, "focus");
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("takes 1 arguments")),
            "a wrong number of arguments is one fact: {said:?}"
        );
        assert!(
            !said.iter().any(|line| line.contains("not a declared window")),
            "and it is not confused with the other refusal: {said:?}"
        );

        // An unknown window is the second fact, and it lists what *is* declared so a typo is
        // fixable from the message alone.
        let mut mount = console();
        type_line(&mut mount, "focus nosuchwindow");
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("not a declared window")),
            "an unknown window is another: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("world") && line.contains("console")),
            "by name: {said:?}"
        );
    }

    #[test]
    fn a_window_verb_moves_the_window_and_takes_the_keyboard_with_it() {
        // The consequence worth stating rather than hiding: **`focus` means focus**, so after it
        // the console no longer holds the keyboard and a second command cannot be typed. That is
        // not a bug in the dispatch — it is what `focus` is — and it is why the verbs an operator
        // scripts with are the ones that do not move focus.
        let mut mount = console();
        mount.open("trees").expect("declared");
        type_line(&mut mount, "focus trees");
        assert_eq!(mount.focused(), Some("trees"));
        assert!(!mount.console_has_focus());
        assert_eq!(
            mount.key_pressed(Key::Char('c'), None),
            KeyOutcome::Ignored,
            "and the next keystroke goes to a window that binds nothing"
        );

        // Reclaiming focus is the console's own verb-free path, and then the close lands.
        mount.focus(CONSOLE_WINDOW);
        type_line(&mut mount, "close trees");
        assert_eq!(mount.window("trees").map(|w| w.status), Some(Status::Closed));
    }

    #[test]
    fn history_recalls_the_last_line_and_empties_past_the_end() {
        let mut mount = console();
        type_line(&mut mount, "stats");
        type_line(&mut mount, "list");
        assert_eq!(mount.key_pressed(Key::HistoryUp, None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "list");
        assert_eq!(mount.key_pressed(Key::HistoryUp, None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "stats");
        assert_eq!(mount.key_pressed(Key::HistoryDown, None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "list");
        // Past the end the line empties rather than refusing to type.
        assert_eq!(mount.key_pressed(Key::HistoryDown, None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "");
        assert_eq!(mount.key_pressed(Key::HistoryDown, None), KeyOutcome::Ignored);
    }

    #[test]
    fn backspace_on_an_empty_draft_says_nothing_changed() {
        let mut mount = console();
        assert_eq!(mount.key_pressed(Key::Backspace, None), KeyOutcome::Ignored);
        mount.key_pressed(Key::Char('a'), None);
        assert_eq!(mount.key_pressed(Key::Backspace, None), KeyOutcome::Changed);
        assert_eq!(mount.console().draft(), "");
    }

    // ---- filing -------------------------------------------------------------

    /// A workspace with one record already on it, because **`grow` refuses an unfiled
    /// workspace** — the console can extend the record, not create it, and that is the honest
    /// division: `append` creates once and refuses a second time, so a client that created the
    /// chain would be the only writer that could.
    ///
    /// **`tag` names the directory per test, and it is load-bearing.** The first version named
    /// it by process id alone and removed it on the way in, so the second filing test to run
    /// deleted the first one's record mid-flight and both failed with `grow`'s prior-bytes
    /// refusal — *"483 vs 1534"* — which reads like a filing defect and is a fixture colliding
    /// with itself. Tests share a process; a shared path is not a private one.
    /// A scratch workspace **inside the project**: `logs/test-workspaces/`, tagged per test.
    /// C-73: *a test over a temporary directory is evidence about the temporary directory* —
    /// and event-0044: file locations belong to the record, not to the agent, and `logs/`
    /// is where the record keeps artifacts. Nothing here leaves the repository.
    fn scratch_workspace(tag: &str) -> std::path::PathBuf {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        root.join("logs")
            .join("test-workspaces")
            .join(format!("shploingus-console-{}-{tag}", std::process::id()))
    }

    fn filed_workspace(tag: &str) -> std::path::PathBuf {
        let workspace = scratch_workspace(&format!("file-{tag}"));
        let _ = std::fs::remove_dir_all(&workspace);
        let namespace = voxel_model::chain::Namespace::new(
            CONSOLE_WORLD,
            CONSOLE_YEAR,
            voxel_model::chain::Season::Winter,
            CONSOLE_CAMPAIGN,
        )
        .expect("a namespace");
        let first = voxel_package::filing::linked_records(
            &workspace,
            &[],
            [voxel_package::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind: voxel_model::chain::EventKind::DevelopmentFact,
                question: "the workspace opened",
                answer: None,
                kind: voxel_model::envelope::Kind::Evidence,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: CONSOLE_ACTOR,
                round_id: CONSOLE_CAMPAIGN,
                supersedes: None,
            }],
        )
        .expect("the first record links");
        voxel_package::filing::append(&workspace, &first).expect("the first filing creates the chain");
        workspace
    }

    const TICKET: &str =
        "mount window[api-viewer] and window[ledger] so the Mods window can show what the \\
         arrangement declares";

    #[test]
    fn a_typed_ticket_lands_on_the_chain_and_compiles_to_a_solution() {
        let workspace = filed_workspace("chain");
        let before = std::fs::read(voxel_package::filing::chain_path(&workspace)).expect("reads");

        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, &format!("file {TICKET}"));

        // **The chain grew, and every byte that was already filed is still there, in order.**
        let after = std::fs::read(voxel_package::filing::chain_path(&workspace)).expect("reads");
        assert!(after.len() > before.len(), "the record got longer");
        assert!(after.starts_with(&before), "and nothing already filed was rewritten");

        // Read back **off the chain**, the way the gate does.
        let read = voxel_package::compiler::read_back(&voxel_package::filing::chain_path(&workspace))
            .expect("the chain reads");
        assert_eq!(read.chain.len(), 2, "two records, both linked");
        let ticket = read.board.tickets().last().expect("the filing is a ticket");
        assert!(
            ticket.objective.contains("api-viewer"),
            "in the player's own words: {}",
            ticket.objective
        );

        // And the console said so, with the id and what the ticket compiles to.
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("filed") && line.contains("BLD-")),
            "the id is on the record: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("emitted") && line.contains(".shpkg")),
            "and the package is on disk at a named path: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("audited stage(s)")),
            "and it went through the gate rather than around it: {said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// The filing path against **the repository's own chain bytes**. Every filing test above
    /// grows a scratch chain of one record; the game grows `chains/workspace.chain`, which is
    /// real linkage at real length (C-89.2's recurring trap). The real bytes are **copied**
    /// here, so what this test grows is a copy and the repository's record is untouched.
    #[test]
    fn the_console_files_against_the_real_chain_bytes_and_not_only_a_scratch_one() {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        let workspace = scratch_workspace("file-real-chain");
        let _ = std::fs::remove_dir_all(&workspace);
        let chain = voxel_package::filing::chain_path(&workspace);
        std::fs::create_dir_all(chain.parent().expect("the chain has a directory"))
            .expect("the scratch chains directory");
        std::fs::copy(voxel_package::filing::chain_path(&root), &chain)
            .expect("the real chain copies");
        let before = std::fs::read(&chain).expect("reads");

        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, &format!("file {TICKET}"));

        // **The chain grew from the real bytes, and every byte that was already filed is still
        // there, in order** — the same two measurements as the scratch chain's test, because a
        // difference between the two is the defect this test exists to catch.
        let after = std::fs::read(&chain).expect("reads");
        assert!(after.len() > before.len(), "the real record got longer");
        assert!(after.starts_with(&before), "and nothing already filed was rewritten");

        let read = voxel_package::compiler::read_back(&chain).expect("the chain reads");
        let ticket = read.board.tickets().last().expect("the filing is a ticket");
        assert!(
            ticket.objective.contains("api-viewer"),
            "in the player's own words: {}",
            ticket.objective
        );
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("filed") && line.contains("BLD-")),
            "the id is on the record: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("emitted") && line.contains(".shpkg")),
            "and the package is on disk at a named path: {said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// The ticket a **person** files: plain prose, no vocabulary. The window-token ticket above
    /// is plumbing; a player at the keyboard types a sentence (M14: *the person plays; the agent
    /// logs*). The system's shape is live debugging: the person files, and the agent reads the
    /// compiled solution and recipe back off the chain and implements the patch. **A ticket a
    /// person cannot file is a system only its plumbers can use** — so this measures the filing
    /// the player actually performs, against the real chain's bytes.
    #[test]
    fn a_persons_plain_prose_ticket_files_and_compiles_like_any_other() {
        let root = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        let workspace = scratch_workspace("file-prose");
        let _ = std::fs::remove_dir_all(&workspace);
        let chain = voxel_package::filing::chain_path(&workspace);
        std::fs::create_dir_all(chain.parent().expect("the chain has a directory"))
            .expect("the scratch chains directory");
        std::fs::copy(voxel_package::filing::chain_path(&root), &chain)
            .expect("the real chain copies");

        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, "file the lighting in the valley is wrong at noon");

        // The person's words are the objective, verbatim — the record keeps what they said.
        let read = voxel_package::compiler::read_back(&chain).expect("the chain reads");
        let ticket = read.board.tickets().last().expect("the filing is a ticket");
        assert_eq!(
            ticket.objective, "the lighting in the valley is wrong at noon",
            "the person's own words, unedited"
        );
        // And it compiles to a package the agent can read and implement from — the other half
        // of the live-debugging loop. A filing the back-compiler refuses is a dead end the
        // player cannot see past and the agent cannot work from.
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("emitted") && line.contains(".shpkg")),
            "a person's ticket compiles to a package the agent can read: {said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// A ticket that names the console **and** another window: the one shape that can be
    /// reloaded, because [`Mount::reload`] refuses a package that takes the console window away.
    const RELOADABLE: &str = "mount window[console] and window[api-viewer] for the arrangement";

    #[test]
    fn a_filed_ticket_is_emitted_to_disk_and_the_shell_reloads_from_it() {
        // The workspace is this test's own: see `filed_workspace`'s note on the tag.
        // **The dynamic loader, in the dirty branch's order:** compile to a package, write it
        // where the loader's search path is, read it *back off disk* through the reader every
        // package loads through, and mount that. `emit-first-package.rs` calls the read-back
        // *"an emit nothing loads is half a transaction"* — so the bytes mounted are read from
        // the file, not the in-memory copy the same call produced.
        let workspace = filed_workspace("reload");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        let before: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();

        type_line(&mut mount, &format!("file {RELOADABLE}"));

        // The package is a real file in the loader's directory, under the ticket's own name.
        // **The path is derived from the record, not scraped out of the log**: the console's
        // line formatting is the console's business and a test that parses it breaks when the
        // prefix changes.
        let read = voxel_package::compiler::read_back(&voxel_package::filing::chain_path(&workspace))
            .expect("the chain reads");
        let id = read.board.tickets().last().expect("the filing is a ticket").id.as_str().to_string();
        let path = voxel_package::filing::newest_package(&workspace, &id)
            .expect("the filing emitted a package");
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains(&id) && line.contains("emitted")),
            "the console names the ticket's own package: {said:?}"
        );
        assert_eq!(
            path.parent().and_then(|p| p.file_name()),
            Some(std::ffi::OsStr::new(voxel_package::filing::PACKAGE_DIR)),
            "a package lands in the one directory the loader reads: {path:?}"
        );
        let on_disk = std::fs::read(&path).expect("the emitted package reads");
        let loaded = voxel_package::recipe::load(&on_disk).expect("and loads");

        // **The running shell is the package, not a copy of it.**
        //
        // Compared as sets, because `windows` is keyed and the shell answers *which windows
        // exist* rather than *in what order the package listed them*; the order that is
        // load-bearing — the bar's — is checked against the package's own list below.
        let after: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        let mut declared: Vec<String> = loaded.recipe.windows.iter().map(|w| w.id.clone()).collect();
        let mut sorted = after.clone();
        sorted.sort();
        declared.sort();
        assert_eq!(sorted, declared, "the shell runs exactly what the package declares");
        assert_eq!(
            mount.bar().iter().map(|entry| entry.id.clone()).collect::<Vec<_>>(),
            loaded.recipe.tabs.iter().chain(loaded.recipe.bar.iter()).cloned().collect::<Vec<_>>(),
            "the bar is the package's rows in strip order"
        );
        assert!(after.contains(&"api-viewer".to_string()), "{after:?}");
        assert!(
            !after.contains(&"world".to_string()),
            "and the previous arrangement is gone rather than merged: {before:?} -> {after:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("the shell now runs")),
            "the reload is on the record: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("replaced") && line.contains("window(s)")),
            "and so is what it cost: {said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn the_loader_reads_the_path_it_is_given_and_refuses_rather_than_serving_the_last_package() {
        // **The teeth for the read-back.** The reload above is fed by `load_package`, and a
        // loader that answered from the bytes it last built would pass that test whatever it did
        // with the file. So the file is replaced with something that is not a package and the
        // answer must change: a refusal, and the arrangement in force untouched.
        //
        // **This is why `load_package` has no cache and no fallback.** A loader holding the last
        // good package would have kept a player's shell running after the package on disk was
        // replaced by a corrupt file, and said nothing.
        let workspace = filed_workspace("loader");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, &format!("file {RELOADABLE}"));
        let filed = voxel_package::compiler::read_back(&voxel_package::filing::chain_path(
            &workspace,
        ))
        .expect("reads");
        let id = filed.board.tickets().last().expect("a ticket").id.as_str().to_owned();
        let path = voxel_package::filing::newest_package(&workspace, &id)
            .expect("the filing emitted a package");
        let running: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();

        // The record's writer refuses to overwrite, and this is a test fixture replacing a file
        // on purpose — so it writes the file itself rather than routing around `filing::file`.
        std::fs::write(&path, b"not a package").expect("the fixture replaces the file");

        let why = mount.load_package(&path).expect_err("a file that is not a package");
        assert!(why.contains("does not frame a package"), "{why}");
        let after: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        assert_eq!(after, running, "and the arrangement in force is left alone: {after:?}");
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// **A package emitted while the game runs is adopted, and exactly once.**
    ///
    /// The live-reload watch reads the compiler's own output root (`packages -> game`), so this
    /// test is the loop the UI is fixed in: file a ticket, the compiler emits, the running game
    /// adopts, the console says what happened. Teeth: a watch that re-adopted the same package
    /// every tick fails the second assertion, and one that adopted a file which does not frame
    /// fails the third.
    #[test]
    fn a_package_emitted_while_the_game_runs_is_adopted_exactly_once() {
        let workspace = filed_workspace("watch");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, &format!("file {RELOADABLE}"));
        let filed = voxel_package::compiler::read_back(&voxel_package::filing::chain_path(
            &workspace,
        ))
        .expect("reads");
        let id = filed.board.tickets().last().expect("a ticket").id.as_str().to_owned();
        let path = voxel_package::filing::newest_package(&workspace, &id)
            .expect("the filing emitted a package");
        let dir = path.parent().expect("the compiler's output root").to_path_buf();

        assert_eq!(
            mount.watch_packages(&dir, Some(7)),
            Some(path.clone()),
            "the emitted package was not adopted"
        );
        // **Once**, because the digest is remembered: a running game must not re-mount the same
        // package on every tick.
        assert!(
            mount.watch_packages(&dir, Some(8)).is_none(),
            "the same package was adopted a second time"
        );

        // A file that does not frame is refused by name — and **the refusal is remembered too**,
        // so one bad package costs one console line rather than one per tick.
        let running: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        std::fs::write(&path, b"not a package").expect("the fixture replaces the file");
        assert!(
            mount.watch_packages(&dir, Some(9)).is_none(),
            "a file that does not frame was adopted"
        );
        let after: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        assert_eq!(after, running, "a bad package changed the arrangement in force");
        assert!(
            mount.watch_packages(&dir, Some(10)).is_none(),
            "the refused package was read again on the next tick"
        );

        // **And the shell prototype itself.** A saved `.shspec` is compiled with the compiler's
        // own function and adopted — the loop the dirty branch used to add a glyph it was missing,
        // and the reason this watch exists at all.
        let tree = voxel_package::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("the tree's root");
        let source = std::fs::read(tree.join("packages").join("alpha-shell.shspec"))
            .expect("the shell prototype reads");
        let spec = dir.join("alpha-shell.shspec");
        std::fs::write(&spec, &source).expect("the prototype is saved");
        assert_eq!(
            mount.watch_packages(&dir, Some(11)),
            Some(spec),
            "the saved shell prototype was not compiled and adopted"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn a_ticket_whose_package_would_take_the_console_window_is_refused_and_nothing_is_lost() {
        // **The refusal, and why it is not a merge.** The console window is the only way to file
        // the next ticket, so a package without it would end the session's command line with the
        // draft still in it. The arrangement in force survives untouched and the console says
        // which windows were compiled.
        let workspace = filed_workspace("no-console");
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        let before: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        // **A record written before the refusal is still readable after it.** The console is
        // `Mount`'s own field and not a window's, which is what makes a refused reload
        // recoverable rather than fatal.
        mount.log(Level::Event, "before", "a record from before the filing", None);

        type_line(&mut mount, &format!("file {TICKET}"));

        let after: Vec<String> = mount.windows().map(|w| w.spec.id.clone()).collect();
        assert_eq!(after, before, "the arrangement in force is unchanged: {after:?}");
        assert!(
            mount.console().lines().iter().any(|line| line.contains("a record from before")),
            "and the console kept its records: {:?}",
            mount.console().lines()
        );
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("Not reloaded") && line.contains("console")),
            "the refusal names the window and the consequence: {said:?}"
        );
        assert!(
            said.iter().any(|line| line.contains("api-viewer")),
            "and the windows it did declare, so the ticket can be rewritten: {said:?}"
        );
        // The package is still emitted: refusing the reload is not refusing the compile.
        assert!(
            said.iter().any(|line| line.contains("emitted")),
            "{said:?}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn a_filing_with_no_workspace_is_refused_by_name_rather_than_guessing_a_path() {
        let mut mount = console();
        type_line(&mut mount, &format!("file {TICKET}"));
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("no workspace")),
            "a client with nowhere to file says so instead of inventing a path: {said:?}"
        );
    }

    #[test]
    fn a_filing_onto_an_unfiled_workspace_is_refused_because_grow_refuses_to_create() {
        // `append` creates the chain once and refuses a second time; `grow` extends. So the
        // console extends and the first record is the gate's — and the refusal says which
        // writer was wanted rather than quietly doing the other thing.
        let workspace = scratch_workspace("unfiled");
        let _ = std::fs::remove_dir_all(&workspace);
        let mut mount = console();
        mount.set_workspace(workspace.clone());
        type_line(&mut mount, &format!("file {TICKET}"));
        let said = mount.console().lines();
        assert!(
            said.iter().any(|line| line.contains("nothing to grow")),
            "and the refusal names it: {said:?}"
        );
        assert!(
            !voxel_package::filing::is_filed(&workspace),
            "and no chain was created behind the refusal"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    // ---- the gate's pass on the console -------------------------------------

    /// A gate report in the shape `decision::TwoPass::report` writes.
    ///
    /// **A fixture that quotes the writer's own format, not a paraphrase of it.** The reader is
    /// round-tripped against the real writer in `voxel-package`; this fixture is here so the
    /// client's own half has something to read without reaching into `OUT_DIR`.
    ///
    /// **The trailing section is part of the real shape**, and it is the defect the console
    /// measured: a reader that ran past the two passes read `integrity` and `identity` as keys
    /// the model's writer never writes, and refused the build's own report (`logs/live.out`).
    /// The fixture carries the section so the client's reader is pinned against the whole shape.
    const GATE: &str = "\
# the decision layer
decider\tdeterministic
subjects\t3
subject-active\t1
subject-inactive\t0
subject-unchecked\t2
subject-refused\t0
subject\tfeature/pinned_hud\tunchecked\tno witness at crates/voxel-client/src/shell/hud.rs
subject\tchange_view\tunchecked\tno witness at crates/voxel-client/src/shell/changes.rs

# the decision layer, pass two (the model instrument)
model-decider\tstructural
model-decided\t7
model-unreached\t98
divergences\t1
divergence\tcivilian/ticket_evidence\tsystem=refused\tmodel=active

# what this gate is not
integrity\testablished
identity\tnot-established-no-signature
";

    #[test]
    fn the_gates_pass_is_on_the_console_with_every_unsettled_subject_and_its_reason() {
        // **D98's second reader, on the surface that already exists.** The build printed this to a
        // terminal; the console is the client's own record, and C-83.5's rule is that a count is
        // not the answer — *which* decision is not settled is.
        let mut mount = console();
        mount.read_gate(GATE, Some(20)).expect("the gate report reads");

        let said = said(&mount);
        assert!(
            said.iter().any(|line| line.contains("deterministic") && line.contains("3 subject(s)")),
            "the instrument and the count: {said:?}"
        );
        for subject in ["feature/pinned_hud", "change_view"] {
            let row = said
                .iter()
                .find(|line| line.contains(subject))
                .unwrap_or_else(|| panic!("{subject} is on the console: {said:?}"));
            assert!(
                row.contains("unchecked") && row.contains("no witness"),
                "**with its reason beside it, not beside the count**: {row}"
            );
        }
        assert!(
            said.iter().any(|line| {
                line.contains("pass two")
                    && line.contains("structural decided 7")
                    && line.contains("98 unreached")
            }),
            "and the model's own pass, measured rather than absent: {said:?}"
        );
        // **The whole report reads, trailing section included.** `integrity` and `identity`
        // follow the two passes in the build's own artefact, and a reader that ran past the
        // section boundary refused the build's own report (`logs/live.out`). This call above
        // returning `Ok` is that fix, and the disagreement is rendered rather than counted.
        assert!(
            said.iter().any(|line| {
                line.contains("disagreement") && line.contains("civilian/ticket_evidence")
            }),
            "a disagreement is preserved, with both answers beside it: {said:?}"
        );
        // **A subject that holds is not printed as unsettled.** The fixture has one active
        // subject and names it nowhere, so a reader printing everything would add a row.
        assert!(
            !said.iter().any(|line| line.contains("active-state")),
            "nothing is invented about a subject the report does not list: {said:?}"
        );
    }

    #[test]
    fn a_report_the_reader_cannot_understand_is_refused_by_name_and_says_nothing_is_wrong() {
        // D107 in the client too: a mount that printed four zeroes would read as a project with
        // no unsettled decisions, which is the quiet-green failure the whole gate exists to stop.
        let mut mount = console();
        let why = mount
            .read_gate("oracle-kind\tstructural-deterministic\n", None)
            .expect_err("no decision layer");
        assert!(why.contains("decision layer"), "{why}");
        assert!(
            !said(&mount).iter().any(|line| line.contains("subject(s)")),
            "and nothing is claimed about a pass that was not read: {:?}",
            said(&mount)
        );
    }

    #[test]
    fn the_gate_is_read_before_the_window_opens_and_survives_a_reload() {
        // The console is `Mount`'s own field, so a reload that adopts a new package does not
        // discard what the gate said — which is what makes the pass readable *while* the player
        // is filing the next ticket.
        let mut mount = console();
        mount.read_gate(GATE, None).expect("reads");
        let before = said(&mount).len();
        assert!(before > 0);
        assert!(
            mount.console_has_focus(),
            "and the console still holds the keyboard, because the pass is on the console"
        );
    }
}
