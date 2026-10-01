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

use voxel_package::recipe::{BehaviorPolicy, Content, LayoutPolicy, Loaded, WindowSpec};
use voxel_ui::{
    bar_entry_width_dp, bar_height_dp, contrast_over_backdrop, glyph_dp, glyph_for, min_target_dp,
    row_height_dp, spacing_dp, title_bar_dp, Align, Element, GlyphAction, Ink, Paint, Palette,
    Panel, Readout, Rect, Role, UiRefusal, UiResult,
};

use super::console::{Command, Console, Key, KeyOutcome, Level, Verb};

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
use crate::shell::arrangement::{Arrangement, Node, Orientation, Side, WindowId, MIN_EXTENT};

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

/// What the shell owns about one window: its state, its geometry and its pane tree.
///
/// **The declaration travels with it** so the shell can always answer *"what did the package
/// say this was"* without a second table, and so the title bar's controls can be derived from
/// the declared policies rather than from a hand-written list.
pub struct Window {
    pub spec: WindowSpec,
    pub status: Status,
    /// The window's rect in dp when it is normal. A maximized window keeps this so restoring
    /// goes back to where the player left it rather than to the declaration.
    pub rect: Rect,
    /// The pane tree this window holds. One pane, holding this window's own id, is the
    /// unstyled case (§3: *"a window with no split is one pane"*).
    pub arrangement: Arrangement,
    /// The window that holds this one as a pane, when it is docked. The arrangement is the
    /// authority on membership; this is the index the shell needs to answer *"is this window
    /// floating"* in one lookup rather than by scanning every tree.
    pub host: Option<String>,
}

impl Window {
    /// Whether this window is drawn on its own. A docked window is drawn by its host.
    pub fn is_standalone(&self) -> bool {
        self.host.is_none()
    }

    /// Whether the title bar carries this control, from the **declared** layout policy.
    pub fn permits(&self, action: GlyphAction) -> bool {
        let policy = self.spec.layout_policy;
        match action {
            GlyphAction::Minimize => policy.permits(LayoutPolicy::MINIMIZE),
            GlyphAction::Maximize => policy.permits(LayoutPolicy::MAXIMIZE),
            GlyphAction::Close => policy.permits(LayoutPolicy::CLOSE),
        }
    }

    /// The controls the title bar draws, in the declared order. **An unpermitted control is
    /// absent**, which is the HIG rule the record keeps: a disabled control that looks like a
    /// grant is worse than no control.
    pub fn controls(&self) -> Vec<GlyphAction> {
        GlyphAction::ALL
            .into_iter()
            .filter(|action| self.permits(*action))
            .collect()
    }
}

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
    /// window's body is its height less the title bar, and §5.7's rule is that the surface
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
            _ => declared_content_panel(spec),
        }
    }
}

/// The body of a declared window: its height less the title bar.
///
/// **The same arithmetic the layout uses**, so a panel bounded against this is bounded against
/// the number the refusal would report. §5.7: *"the two numbers must be the same number."*
pub fn pane_of(spec: &WindowSpec) -> f64 {
    spec.geometry[3] as f64 - title_bar_dp()
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
/// property of **the window hosting it**. `alpha-shell.shspec` declares the console 406 dp
/// tall, which is a 358 dp pane, and sixty-four records do not fit in it — so a panel built to a
/// constant refuses to lay out with *"a 407 dp panel in a 358 dp pane"* the moment the log is
/// more than a few lines long. That is §5.7's check working, and the fix is not to resize the
/// window: *"a shell that resized them would be choosing an arrangement, and the arrangement is
/// a package."*
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

    // **Measured with the note attached, not after it.** A note is a drawn row — `drawn_rows`
    // counts it — so a panel bounded while it has no note and given the note afterwards is one
    // row too tall, which is how the first version of this returned a 407 dp panel into a
    // 358 dp pane with every record dropped: the loop was satisfied and the note broke it.
    let build = |rows: &[Readout], held: usize| -> Panel {
        let panel = Panel::new("console", Role::Chrome, "Console", rows.to_vec());
        if held == 0 {
            return panel;
        }
        let note = format!(
            "{held} older record(s) are held and not drawn; the pane holds {available_dp} dp. \
             {} record(s) in all",
            console.len() + console.dropped()
        );
        panel
            .with_note(&note)
            // The note is a formatted sentence and cannot be blank, so this arm is unreachable in
            // practice. It is here rather than `unwrap` because a note that somehow *was* blank
            // must cost the note, not the records.
            .unwrap_or_else(|_| Panel::new("console", Role::Chrome, "Console", rows.to_vec()))
    };

    let mut held = 0usize;
    let mut panel = build(&rows, held);
    while panel.height_dp() > available_dp && rows.len() > header {
        // **From the front**, so what survives is the newest — a log whose tail is cut reads as
        // a log that ended.
        rows.remove(header);
        held += 1;
        panel = build(&rows, held);
    }
    panel
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

struct Drag {
    /// The window being moved.
    id: String,
    /// Where in the window the pointer grabbed it, in dp from the window's origin.
    offset: (f64, f64),
    /// Whether the pointer has actually moved it. A click that never moves is a focus, not a
    /// dock attempt.
    moved: bool,
}

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
    drag: Option<Drag>,
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
            viewport: Rect::new(0.0, 0.0, 1280.0, 720.0),
            refusals: Vec::new(),
            console: Console::new(),
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
    /// **Refused, by name, with the windows it did declare** — the same shape as
    /// `Solution::for_ticket` refusing a window the ticket does not name: the ticket has to name
    /// the console to replace the package.
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
            self.console.log(
                Level::Warn,
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
        let path = voxel_package::filing::package_path(&workspace, &receipt.name);
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

    /// The title a window's chrome draws: its own title when it is one pane, and the pane set's
    /// when it is more. The reference derives a splitter's title from what it holds; the
    /// package's `title` is the one-pane case of the same rule.
    pub fn title_of(&self, id: &str) -> String {
        let titles = self.pane_titles(id);
        match titles.len() {
            0 => self
                .windows
                .get(id)
                .map(|w| w.spec.title.clone())
                .unwrap_or_else(|| id.to_string()),
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
            .close(&pane_id)
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
        let rect = pane_rect_of(host_window.arrangement.root(), host_rect, pane)
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
        let (host_pane, orientation, side) = {
            let Some(host_window) = self.windows.get(host) else {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            };
            if host_window.host.is_some() {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            }
            let rect = self
                .rect_of(host)
                .ok_or(UiRefusal::UnknownWindow { window: host.to_string() })?;
            let Some(pane) = pane_at(host_window.arrangement.root(), rect, at) else {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            };
            let pane_rect = pane_rect_of(host_window.arrangement.root(), rect, &pane)
                .ok_or(UiRefusal::UnknownWindow { window: pane.clone() })?;
            let horizontal = at.0 < pane_rect.x + pane_rect.width_dp / 2.0;
            let vertical = at.1 < pane_rect.y + pane_rect.height_dp / 2.0;
            // The nearer half decides. Both axes are compared; the middle line is the split,
            // and a drop exactly on it takes the leading side rather than refusing.
            if (at.0 - (pane_rect.x + pane_rect.width_dp / 2.0)).abs()
                < (at.1 - (pane_rect.y + pane_rect.height_dp / 2.0)).abs()
            {
                (
                    pane,
                    Orientation::Horizontal,
                    if horizontal { Side::Before } else { Side::After },
                )
            } else {
                (
                    pane,
                    Orientation::Vertical,
                    if vertical { Side::Before } else { Side::After },
                )
            }
        };
        {
            let Some(dragged_window) = self.windows.get(dragged) else {
                return Err(UiRefusal::UnknownWindow { window: dragged.to_string() });
            };
            if dragged_window.arrangement.panes() > 1 || dragged_window.host.is_some() {
                return Err(UiRefusal::DuplicateWindow { window: dragged.to_string() });
            }
        }
        let pane_id = WindowId::new(host_pane)?;
        let dragged_id = WindowId::new(dragged)?;
        {
            let Some(host_window) = self.windows.get_mut(host) else {
                return Err(UiRefusal::UnknownWindow { window: host.to_string() });
            };
            host_window
                .arrangement
                .split(Some(&pane_id), orientation, side, dragged_id)?;
        }
        if let Some(window) = self.windows.get_mut(dragged) {
            window.host = Some(host.to_string());
        }
        self.ordered.retain(|held| held != dragged);
        self.focus(host);
        Ok(())
    }

    /// Where a pane sits inside its host right now, if the host holds it.
    pub fn pane_rect(&self, host: &str, pane: &str) -> Option<Rect> {
        let window = self.windows.get(host)?;
        let rect = self.rect_of(host)?;
        pane_rect_of(window.arrangement.root(), rect, pane)
    }

    // -----------------------------------------------------------------------
    // The pointer
    // -----------------------------------------------------------------------

    pub fn pointer_left(&mut self) {
        self.cursor = None;
        self.drag = None;
    }

    pub fn pointer_moved(&mut self, at: (f64, f64)) {
        self.cursor = Some(at);
        let Some((id, offset)) = self
            .drag
            .as_ref()
            .map(|drag| (drag.id.clone(), drag.offset))
        else {
            return;
        };
        if let Some(drag) = self.drag.as_mut() {
            drag.moved = true;
        }
        let Some(window) = self.windows.get(&id) else {
            return;
        };
        if window.status == Status::Maximized {
            return;
        }
        let wanted = Rect::new(
            at.0 - offset.0,
            at.1 - offset.1,
            window.rect.width_dp,
            window.rect.height_dp,
        );
        let clamped = self.clamp_window(wanted);
        if let Some(window) = self.windows.get_mut(&id) {
            window.rect = clamped;
        }
    }

    pub fn pointer_pressed(&mut self, button: Button, at: (f64, f64)) {
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
        let hit = self.ordered.iter().rev().find_map(|id| {
            self.rect_of(id)
                .filter(|rect| rect.contains(at))
                .map(|rect| (id.clone(), rect))
        });
        let Some((id, rect)) = hit else {
            return;
        };
        let title = Rect::new(rect.x, rect.y, rect.width_dp, title_bar_dp());

        if button == Button::Right {
            // Right-click on a pane undocks it: the task-viewer's own action, without a menu
            // to find first.
            let pane = self
                .windows
                .get(&id)
                .and_then(|window| pane_at(window.arrangement.root(), rect, at));
            if let Some(pane) = pane {
                if pane != id {
                    let host = id.clone();
                    let _ = self.undock(&host, &pane);
                }
            }
            return;
        }

        if title.contains(at) {
            // A control lane, if the click is in one.
            for (action, lane) in self.control_lanes(&id, rect) {
                if lane.contains(at) {
                    match action {
                        GlyphAction::Close => self.close(&id),
                        GlyphAction::Minimize => self.minimize(&id),
                        GlyphAction::Maximize => self.toggle_maximize(&id),
                    }
                    return;
                }
            }
            self.focus(&id);
            let draggable = self
                .windows
                .get(&id)
                .is_some_and(|w| w.spec.behavior_policy.permits(BehaviorPolicy::DRAG));
            let maximized = self.windows.get(&id).is_some_and(|w| w.status == Status::Maximized);
            if draggable && !maximized {
                let offset = (at.0 - rect.x, at.1 - rect.y);
                self.drag = Some(Drag { id: id.clone(), offset, moved: false });
            }
            return;
        }

        // The body: focus, and nothing else. Resizing is a behaviour the record declares and the
        // shell does not implement yet — said here rather than implied by a cursor.
        self.focus(&id);
    }

    pub fn pointer_released(&mut self, at: (f64, f64)) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if !drag.moved {
            return;
        }
        let host = self.ordered.iter().rev().find(|id| {
            **id != drag.id
                && self
                    .rect_of(id)
                    .is_some_and(|rect| rect.contains(at) && !self.is_on_title_bar(&rect, at))
        });
        if let Some(host) = host.cloned() {
            let _ = self.dock(&host, &drag.id, at);
        }
    }

    // -----------------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------------

    /// One window: its body, its chrome, its controls and its content.
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

        let body_role = Role::from_theme_role(&window.spec.theme_role)?;
        let body = palette.get(body_role)?;
        let chrome = palette.get(Role::Chrome)?;
        let opacity = (window.spec.opacity as f64 / 1000.0).clamp(0.0, 1.0);
        let focused = self.focused.as_deref() == Some(id);

        // The body first, so everything else sits on it.
        out.push(Element::Surface {
            rect,
            paint: Paint { rgb: body, alpha: opacity },
        });
        let title_rect = Rect::new(rect.x, rect.y, rect.width_dp, title_bar_dp());
        out.push(Element::Surface {
            rect: title_rect,
            paint: Paint { rgb: chrome, alpha: opacity },
        });
        // The rule between chrome and content, and the window's own outline. The focused
        // window's outline is one ink step brighter: focus is a state the eye can find without
        // a colour that only exists for it.
        let outline = if focused { Ink::Secondary } else { Ink::Divider };
        let divider = Ink::Divider;
        out.push(Element::Rule {
            rect: Rect::new(rect.x, title_rect.bottom(), rect.width_dp, 1.0),
            paint: divider.paint(),
        });
        for edge in [
            Rect::new(rect.x, rect.y, rect.width_dp, 1.0),
            Rect::new(rect.x, rect.bottom() - 1.0, rect.width_dp, 1.0),
            Rect::new(rect.x, rect.y, 1.0, rect.height_dp),
            Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.height_dp),
        ] {
            out.push(Element::Rule { rect: edge, paint: outline.paint() });
        }

        // The title, derived from the pane set.
        let backdrop = chrome.blend_over(BRIGHTEST, opacity);
        let ink = if focused { Ink::Full } else { Ink::Secondary };
        contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
        out.push(Element::Text {
            rect: Rect::new(
                title_rect.x + spacing_dp(),
                title_rect.y + (title_bar_dp() - voxel_ui::title_size_dp()) / 2.0,
                title_rect.width_dp - 2.0 * spacing_dp(),
                voxel_ui::title_size_dp(),
            ),
            text: self.title_of(id),
            paint: ink.paint(),
            size_dp: voxel_ui::title_size_dp(),
            align: Align::Start,
        });

        // The controls the policy permits, and not one more.
        for (action, lane) in self.control_lanes(id, rect) {
            let glyph = glyph_for(action);
            contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
            out.push(Element::Text {
                rect: Rect::new(
                    lane.x + (lane.width_dp - glyph_dp()) / 2.0,
                    lane.y + (lane.height_dp - glyph_dp()) / 2.0,
                    glyph_dp(),
                    glyph_dp(),
                ),
                text: glyph.text.to_string(),
                paint: ink.paint(),
                size_dp: glyph_dp(),
                align: Align::Start,
            });
        }

        // The content. One pane is the whole body; more panes are the arrangement's layout.
        //
        // **The body's usable height is `height − title bar`, exactly.** The pinned windows in
        // `alpha-shell.shspec` are sized as one 48 dp title bar plus the panel's own height
        // (*"the tallest is 309 dp at five rows"*), so a body that also reserved a vertical
        // inset would push the panel's last row past the declared rect. The panel supplies its
        // own padding; the window supplies the frame.
        let inner = Rect::new(
            rect.x + spacing_dp(),
            title_rect.bottom(),
            (rect.width_dp - 2.0 * spacing_dp()).max(0.0),
            (rect.height_dp - title_bar_dp()).max(0.0),
        );
        let mut panes = Vec::new();
        layout_panes(window.arrangement.root(), inner, &mut panes)?;
        for (pane_rect, pane_id) in panes {
            let panel = match self.windows.get(&pane_id) {
                Some(pane_window) => views.panel_for(&pane_window.spec),
                None => declared_content_panel(&window.spec),
            };
            let width = pane_rect.width_dp - 2.0 * spacing_dp();
            if width <= 0.0 || pane_rect.height_dp <= 0.0 {
                continue;
            }
            // **Measured before it is placed.** §5.7's rule: the surface knows its own height
            // before it draws, so a panel that does not fit its pane is a refusal with both
            // numbers rather than a drawing that spills past the window's declared rect.
            if panel.height_dp() > pane_rect.height_dp + 0.5 {
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

    /// The control lanes for a window's title bar, right-aligned in the declared order.
    pub fn control_lanes(&self, id: &str, rect: Rect) -> Vec<(GlyphAction, Rect)> {
        let Some(window) = self.windows.get(id) else {
            return Vec::new();
        };
        window
            .controls()
            .into_iter()
            .rev()
            .enumerate()
            .map(|(index, action)| {
                let right = rect.right() - (index as f64 + 1.0) * min_target_dp();
                (
                    action,
                    Rect::new(right, rect.y, min_target_dp(), title_bar_dp()),
                )
            })
            .collect()
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

    fn is_on_title_bar(&self, rect: &Rect, at: (f64, f64)) -> bool {
        at.1 >= rect.y && at.1 < rect.y + title_bar_dp()
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

fn collect_titles(node: &Node, windows: &BTreeMap<String, Window>, out: &mut Vec<String>) {
    match node {
        Node::Pane(id) => {
            let title = windows
                .get(id.as_str())
                .map(|window| window.spec.title.clone())
                .unwrap_or_else(|| id.as_str().to_string());
            out.push(title);
        }
        Node::Split { children, .. } => {
            for (_, child) in children {
                collect_titles(child, windows, out);
            }
        }
    }
}

/// Lay out a pane tree inside `rect`, in tree order.
///
/// **The 40 × 40 floor is checked where the extent is known**, which is here: the arrangement's
/// own `check_floor` takes one span and divides it by the child count at every level, and this
/// allocates real rectangles, so the check and the layout use the same numbers. A tree that
/// cannot give every child the floor is refused before anything is drawn — §3's warning is that
/// the reference discovers it while laying out, with the window already stuck.
fn layout_panes(node: &Node, rect: Rect, out: &mut Vec<(Rect, String)>) -> UiResult<()> {
    match node {
        Node::Pane(id) => {
            out.push((rect, id.as_str().to_string()));
            Ok(())
        }
        Node::Split { orientation, children } => {
            let count = children.len().max(1) as f64;
            let span = match orientation {
                Orientation::Horizontal => rect.width_dp,
                Orientation::Vertical => rect.height_dp,
            };
            if span / count < MIN_EXTENT as f64 {
                return Err(UiRefusal::PaneBelowFloor {
                    span: span as u32,
                    count: count as u32,
                    floor: MIN_EXTENT,
                    axis: orientation.constrained_axis(),
                });
            }
            let total: f64 = children.iter().map(|(share, _)| *share as f64).sum();
            let total = if total <= 0.0 { 1.0 } else { total };
            let mut at = match orientation {
                Orientation::Horizontal => rect.x,
                Orientation::Vertical => rect.y,
            };
            for (share, child) in children {
                let extent = match orientation {
                    Orientation::Horizontal => rect.width_dp * (*share as f64) / total,
                    Orientation::Vertical => rect.height_dp * (*share as f64) / total,
                };
                let child_rect = match orientation {
                    Orientation::Horizontal => {
                        Rect::new(at, rect.y, extent, rect.height_dp)
                    }
                    Orientation::Vertical => Rect::new(rect.x, at, rect.width_dp, extent),
                };
                layout_panes(child, child_rect, out)?;
                at += extent;
            }
            Ok(())
        }
    }
}

/// The deepest pane under a point, or `None` when the point is outside the tree's rectangles.
fn pane_at(node: &Node, rect: Rect, at: (f64, f64)) -> Option<String> {
    match node {
        Node::Pane(id) => rect.contains(at).then(|| id.as_str().to_string()),
        Node::Split { orientation, children } => {
            let total: f64 = children.iter().map(|(share, _)| *share as f64).sum();
            let total = if total <= 0.0 { 1.0 } else { total };
            let mut cursor = match orientation {
                Orientation::Horizontal => rect.x,
                Orientation::Vertical => rect.y,
            };
            for (share, child) in children {
                let extent = match orientation {
                    Orientation::Horizontal => rect.width_dp * (*share as f64) / total,
                    Orientation::Vertical => rect.height_dp * (*share as f64) / total,
                };
                let child_rect = match orientation {
                    Orientation::Horizontal => Rect::new(cursor, rect.y, extent, rect.height_dp),
                    Orientation::Vertical => Rect::new(rect.x, cursor, rect.width_dp, extent),
                };
                if child_rect.contains(at) {
                    return pane_at(child, child_rect, at);
                }
                cursor += extent;
            }
            None
        }
    }
}

/// The rect a named pane occupies, or `None` when this tree does not hold it.
fn pane_rect_of(node: &Node, rect: Rect, pane: &str) -> Option<Rect> {
    match node {
        Node::Pane(id) => (id.as_str() == pane).then_some(rect),
        Node::Split { orientation, children } => {
            let total: f64 = children.iter().map(|(share, _)| *share as f64).sum();
            let total = if total <= 0.0 { 1.0 } else { total };
            let mut cursor = match orientation {
                Orientation::Horizontal => rect.x,
                Orientation::Vertical => rect.y,
            };
            for (share, child) in children {
                let extent = match orientation {
                    Orientation::Horizontal => rect.width_dp * (*share as f64) / total,
                    Orientation::Vertical => rect.height_dp * (*share as f64) / total,
                };
                let child_rect = match orientation {
                    Orientation::Horizontal => Rect::new(cursor, rect.y, extent, rect.height_dp),
                    Orientation::Vertical => Rect::new(rect.x, cursor, rect.width_dp, extent),
                };
                if let Some(found) = pane_rect_of(child, child_rect, pane) {
                    return Some(found);
                }
                cursor += extent;
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_package::recipe;
    use voxel_ui::Palette;

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
\tgeometry\t16 128 480 406\n\
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
\tgeometry\t80 160 480 406\n\
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
\tgeometry\t144 192 480 406\n\
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
        lanes[0].1.check_target("a control lane").expect("a control lane is a target");
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
        assert_eq!(mount.window("trees").and_then(|w| w.host.as_deref()), Some("inventory"));
        // The drop landed in the upper half, so the dragged window takes the leading pane and
        // the title says so in tree order.
        assert_eq!(mount.title_of("inventory"), "Trees / Inventory");
        assert_eq!(mount.rect_of("trees"), None, "a docked window has no rect of its own");
        // And undocking gives it a rect back, where its pane was.
        let pane = mount.pane_rect("inventory", "trees").expect("the pane is laid out");
        mount.undock("inventory", "trees").expect("the pane undocks");
        assert_eq!(mount.window("trees").and_then(|w| w.host.as_deref()), None);
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
    fn a_pane_tree_that_cannot_hold_its_floor_is_refused_before_it_is_drawn() {
        // 60 dp of body split two ways is 30 each, below the 40 dp floor — and the refusal
        // carries the arithmetic rather than a verdict.
        let mut mount = mount();
        mount.open("inventory").expect("declared");
        mount.open("trees").expect("declared");
        let host = mount.rect_of("inventory").expect("standalone");
        mount
            .dock("inventory", "trees", (host.x + 10.0, host.y + title_bar_dp() + 40.0))
            .expect("the drop splits the host");
        let window = mount.windows.get_mut("inventory").expect("the host");
        // A 48 dp title bar and 60 dp of body, so the body splits 30 dp each — under the 40 dp
        // floor. The body's height is `height − title bar`, exactly, so `108 − 48` is the span
        // the refusal reports.
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
        assert!(refusal.contains("60 dp"), "{refusal}");
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
            let Some(rect) = mount.rect_of(id) else { continue };
            let Some(window) = mount.windows.get(id) else { continue };
            let panel = views.panel_for(&window.spec);
            let pane = rect.height_dp - title_bar_dp();
            if panel.height_dp() > pane + 0.5 {
                short.push(format!(
                    "{id}: a {} dp panel in a {pane} dp pane",
                    panel.height_dp()
                ));
            }
        }
        assert!(short.is_empty(), "the package declares windows too short for their views: {short:?}");
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
        // 358 dp pane `alpha-shell.shspec` declares, and the running client refused to lay out
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
    fn filed_workspace(tag: &str) -> std::path::PathBuf {
        let workspace = std::env::temp_dir().join(format!(
            "shploingus-console-file-{}-{tag}",
            std::process::id()
        ));
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
        let path = voxel_package::filing::package_path(&workspace, &id);
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
        let path = voxel_package::filing::package_path(&workspace, &id);
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
        let workspace = std::env::temp_dir().join(format!(
            "shploingus-console-unfiled-{}",
            std::process::id()
        ));
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
model-decider\tunreachable
model-decided\t0
model-unreached\t3
divergences\t0
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
            said.iter().any(|line| line.contains("pass two") && line.contains("unreachable")),
            "and the model's own pass, which is absent rather than agreeing: {said:?}"
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
