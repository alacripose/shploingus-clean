//! **The console: the single chokepoint every window reports through.**
//!
//! # What was ported, and from what
//!
//! `docs/portal_simulator.py` (4,252 lines, byte-identical to the reference per H-7.19, left in
//! place by ruling as C-70.4 records). C-65.4 read it and recorded the shape; this module is that
//! reading made executable. **Not the file — the decisions**, which is C-67.1's rule and the
//! reason this is 300 lines rather than 4,252.
//!
//! | the reference does | §/record says | what a re-derivation misses |
//! |---|---|---|
//! | `self.portal.log("ACTION", "CMD", f"> {cmd}")` **before** dispatch | a command surface | **the typed command is itself an event.** An operator can see what they asked for even when the verb refused |
//! | five levels, each with its own badge and filter entry | *absent* | the levels are **closed and five**, and each has a colour that is a *badge*, not a tint of one row |
//! | every branch calls `append_system_message` | *absent* | **no branch is silent.** C-60's never-silent rule arriving from a PySide6 file |
//! | `log(level, category, message)` is the one chokepoint | *absent* | a window does not write to the console; it **reports**, and the console decides how that reads |
//!
//! # The two things this must not do
//!
//! * **It is not a second messaging path.** C-62.3 made `RaiseToast` `AlwaysAllowed` precisely
//!   because *"a matrix in which the only way to speak is gated would make every refusal
//!   invisible."* So [`Console::log`] is **ungated** and takes no `Grant` — a console that
//!   refused to report a refusal would be the defect.
//! * **It is not unbounded.** `MAX_RECORDS` is a declared ceiling and the console **refuses
//!   past it by dropping the oldest and saying so**, rather than growing into a second ledger.
//!   A console that grows without bound is a record nobody can read, which is `bloat`'s finding
//!   one surface over.
//!
//! # Verbs are data, never Rust
//!
//! H-7.5: *"a view is a data record — what it selects, which stages/shapers it reads, its labels
//! and its lane class — and one generic renderer draws any declared view. Adding a view is data,
//! never Rust."* [`VERBS`] is that declaration for the command surface.
//!
//! **And the count is measured, not inherited — and it was miscounted once.** C-80.5 records
//! *"eleven command verbs"* and then lists twelve. The reference implements the twelve, plus
//! `undock` and `setting`, plus a governance set — `approvals approve blockers mode plan reject
//! report`, which is **seven** names, not six. So the surface is **twenty-one**: `12 + 2 + 7`.
//! An earlier version of this header said **twenty**, `Verb::COUNT` was set to `20` to match, and
//! the table was built from the same miscount — so [`Verb::Plan`], which the enum has and
//! `arity()` declares at `(1, 1)`, was in **no row at all**: untypable, absent from the generated
//! help, and invisible to [`VERBS_ARE_TOTAL`], which compares the count to the *table* and so
//! cannot see a variant outside both. [`VERBS_COVER_EVERY_VARIANT`] is the pin that can.
//!
//! The reference's own table is the authority on the count; C-48 says a document informs and
//! never rules, and this tree's arithmetic is its own.
//!
//! **And this tree carries a twenty-second verb the reference does not have:** `file`, which takes
//! the rest of the line as a ticket in the player's own words, grows it onto the chain, and reads
//! it back as a solution. The reference's surface is the twenty-one above and stays twenty-one —
//! the extension is declared here rather than smuggled in as though the reference had it.

use std::collections::VecDeque;

/// The five levels, closed. **Five, because the reference has five and a sixth would be a
/// colour nobody chose.**
///
/// **A rename here is a breaking change to the record**, and `LEVEL_NAMES` exists so the wire
/// spelling is written once — `parse` refusing an undeclared word is C-58's tenth mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Sys,
    Event,
    Action,
    Warn,
    Info,
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::Sys,
        Level::Event,
        Level::Action,
        Level::Warn,
        Level::Info,
    ];

    /// The spelling on the wire. Written out here rather than derived, because a derived name is
    /// a second answer to "what is this called".
    pub const fn slug(self) -> &'static str {
        match self {
            Level::Sys => "SYS",
            Level::Event => "EVENT",
            Level::Action => "ACTION",
            Level::Warn => "WARN",
            Level::Info => "INFO",
        }
    }

    pub const fn describe(self) -> &'static str {
        match self {
            Level::Sys => "the shell's own lifecycle: mount, layout, theme, refusal",
            Level::Event => "something in the world changed, or the record did",
            Level::Action => "an operator asked for something, including the asking",
            Level::Warn => "the request was understood and is proceeding degraded",
            Level::Info => "a reading, with nothing wrong with it",
        }
    }

    /// Whether a record at this level is one an operator must not miss.
    ///
    /// **C-64.2's MD1 split, as a predicate:** refusals and anything needing a human are
    /// persistent; the rest is transient. A warn is the first thing that goes to a notification
    /// queue rather than to a scrolling row.
    pub const fn is_persistent(self) -> bool {
        matches!(self, Level::Warn)
    }

    pub fn parse(text: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|level| level.slug() == text)
    }
}

/// One line of the console. **`level`, `category`, `message`, `tick`** — the same four things
/// `voxel_instrument::Sensor` is, and no more: C-70.2 refused to add `Site::Record` to the
/// sensor because *"a byte count is not a world reading, and adding a second meaning behind an
/// existing name is D-07's vocabulary mismatch."*
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub level: Level,
    /// What produced it. **A free string, and deliberately** — the reference uses `CMD`,
    /// `OPERATOR_MODE` and others, and a closed enum here would refuse the next one.
    pub category: String,
    pub message: String,
    /// The tick, so a line can be placed in time. **`None` rather than `0`**, because a record
    /// written before the clock starts has not happened at tick zero.
    pub tick: Option<u64>,
}

impl Record {
    pub fn describe(&self) -> String {
        let at = match self.tick {
            Some(tick) => format!("t{tick}"),
            // **Absence rendered as absence** (D107): a record with no tick says so rather than
            // reading as tick zero.
            None => "t-".to_string(),
        };
        format!(
            "[{}] {at} {}: {}",
            self.level.slug(),
            self.category,
            self.message
        )
    }
}

/// **The declared capacity, and it is a number rather than a policy.**
///
/// The reference grows a `QTextEdit` without bound, which is fine for a simulator and wrong for
/// a game that runs for hours. **Dropping the oldest is the honest eviction** — the alternative
/// is refusing to log, and a console that refuses to report a refusal is the one failure this
/// module exists to prevent. `dropped` counts them so the loss is never silent.
pub const MAX_RECORDS: usize = 2048;

/// The console's own state, and the chokepoint.
///
/// **Every window reports here and nowhere else.** `log` is the whole API surface for producing
/// output; there is no `push_record` and no public field, so a window cannot write a line the
/// console did not format.
#[derive(Clone, Debug)]
pub struct Console {
    records: VecDeque<Record>,
    dropped: usize,
    /// **The filter, and it is one level or all of them** — the reference's `All Logs` entry
    /// beside five level entries. `None` is all.
    filter: Option<Level>,
    /// What the operator typed, oldest first. **Bounded by the same ceiling**, for the same
    /// reason: history is a convenience, not a record.
    history: VecDeque<String>,
    cursor: usize,
    /// **The line being typed, right now.**
    ///
    /// **Not a record, and deliberately not in `records`.** A half-typed command is not
    /// something that happened, and a console that logged every keystroke would fill its own
    /// ceiling with an operator's typing. It becomes a record only at [`Console::submit`], and
    /// only once, and only in the order the reference does it.
    draft: String,
    /// **The workspace a `file` is grown into**, when the caller has named one.
    ///
    /// **Optional, and a `file` without it is refused by name.** A client with no workspace has
    /// no record to file into, and inventing a path would be a write somewhere the operator
    /// never named.
    workspace: Option<std::path::PathBuf>,
}

/// A key the console understands. **The shell's own enum, not `winit`'s**, for the same reason
/// [`crate::shell::mount::Button`] is: `voxel-client` has no windowing dependency, and the bin
/// is the only place in this repository that may name one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A printable character, already resolved from the platform's text input.
    Char(char),
    /// The platform's backspace. Named rather than mapped to `Char('\u{8}')` because a delete
    /// that arrives as a character is a delete nobody can test.
    Backspace,
    /// Submit the draft.
    Enter,
    /// Older history entry.
    HistoryUp,
    /// Newer history entry, and past the end is an empty draft rather than a refusal.
    HistoryDown,
}

/// What a key did, so a caller can decide whether to redraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyOutcome {
    /// Nothing changed — the key is not one the console uses, or the draft was already empty.
    Ignored,
    /// The draft or the log changed, and the frame is stale.
    Changed,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        Self {
            records: VecDeque::with_capacity(64),
            dropped: 0,
            filter: None,
            history: VecDeque::with_capacity(32),
            cursor: 0,
            draft: String::new(),
            workspace: None,
        }
    }

    /// **The chokepoint.** Every window, every event, every refusal goes through this one call.
    ///
    /// **The reference logs the typed command at `ACTION` before it dispatches**, so an operator
    /// can see what they asked for even when the verb refused — and that is the whole reason a
    /// console exists rather than an error dialog.
    pub fn log(&mut self, level: Level, category: &str, message: impl Into<String>, tick: Option<u64>) {
        let record = Record {
            level,
            category: category.to_string(),
            message: message.into(),
            tick,
        };
        if self.records.len() >= MAX_RECORDS {
            self.records.pop_front();
            self.dropped += 1;
        }
        self.records.push_back(record);
    }

    /// Set the filter. **`None` is every level**, and saying so is better than a sentinel string
    /// that reads like a level.
    pub fn set_filter(&mut self, filter: Option<Level>) {
        self.filter = filter;
    }

    pub fn filter(&self) -> Option<Level> {
        self.filter
    }

    /// The lines the console would show, oldest first. **Filtered, never truncated** — a
    /// truncated console is the deferral this repository removed from `Board::describe` earlier
    /// today, and it is the same defect wearing different clothes.
    pub fn lines(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| self.filter.is_none_or(|level| record.level == level))
            .map(Record::describe)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// How many records the ceiling has discarded. **Printed beside the count, always**, because
    /// a console showing 2,048 lines and having dropped 900 is a different fact from one that has
    /// never held 2,948.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Clear the output. **The filter and the history survive**, because clearing a screen is not
    /// clearing what you typed, and a console that forgot your last command is worse than one
    /// that forgot its output.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Record a typed line for history, and return it parsed.
    ///
    /// **History is bounded by [`MAX_RECORDS`] too** and drops its own oldest, for the same
    /// reason the records do.
    /// One key, on the draft.
    ///
    /// **The console takes keys whether or not it is focused**, and the *shell* decides focus —
    /// the same division as the reference, where the line edit belongs to the console window and
    /// the window belongs to the shell. A key that changes nothing says so rather than
    /// reporting a change, so a caller can skip a redraw.
    pub fn key(&mut self, key: Key) -> KeyOutcome {
        match key {
            Key::Char(character) => {
                self.draft.push(character);
                KeyOutcome::Changed
            }
            Key::Backspace => {
                if self.draft.pop().is_none() {
                    return KeyOutcome::Ignored;
                }
                KeyOutcome::Changed
            }
            Key::HistoryUp => {
                if self.cursor == 0 {
                    return KeyOutcome::Ignored;
                }
                self.cursor -= 1;
                self.draft = self.history[self.cursor].clone();
                KeyOutcome::Changed
            }
            Key::HistoryDown => {
                if self.cursor + 1 >= self.history.len() {
                    // **Past the end the draft empties rather than refusing.** The history is a
                    // convenience and refusing to type after recalling is a worse answer than an
                    // empty line, which the submit path already treats as nothing to do.
                    if self.cursor < self.history.len() || !self.draft.is_empty() {
                        self.cursor = self.history.len();
                        self.draft.clear();
                        return KeyOutcome::Changed;
                    }
                    return KeyOutcome::Ignored;
                }
                self.cursor += 1;
                self.draft = self.history[self.cursor].clone();
                KeyOutcome::Changed
            }
            // `Enter` is [`Console::submit`], which is a separate call because a submit is not a
            // keystroke — it is the moment the line stops being a draft.
            Key::Enter => KeyOutcome::Ignored,
        }
    }

    /// The line being typed, for a caller that draws it.
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// Where a `file` from this console is filed. **Set once, by the caller that knows the root**
    /// — the bin, which loaded a package from a path, and a test, which owns a scratch directory.
    pub fn set_workspace(&mut self, root: std::path::PathBuf) {
        self.workspace = Some(root);
    }

    /// The workspace, or `None` — and the caller turns that into a refusal rather than a guess.
    pub fn workspace(&self) -> Option<&std::path::Path> {
        self.workspace.as_deref()
    }

    /// Submit the draft, **logging it before anything acts on it**.
    ///
    /// The order is the reference's and it is load-bearing: `execute_command` calls
    /// `portal.log("ACTION", "CMD", f"> {cmd}")` *before* it looks at the verb, so a command that
    /// is unknown, mis-typed or refused is still on the record. A console that logs the outcome
    /// and not the attempt reports only what worked, and an operator cannot tell a refused
    /// command from one that was never typed.
    ///
    /// **Returns the parse, refusal included** — and a refusal is logged here too, so the reason
    /// is on the record beside the command that caused it.
    pub fn submit(&mut self, tick: Option<u64>) -> Option<Result<Command, CommandRefusal>> {
        let line = std::mem::take(&mut self.draft);
        if line.trim().is_empty() {
            // **Empty input is not an event.** The reference returns before it clears, logs
            // nothing and remembers nothing, and a console that records an empty line would fill
            // its history with the times nobody pressed anything.
            self.draft = line;
            return None;
        }

        self.log(Level::Action, "CMD", format!("> {line}"), tick);
        self.remember(&line);

        let outcome = parse(&line);
        if let Err(refusal) = &outcome {
            self.log(Level::Warn, "CMD", refusal.detail(), tick);
        }
        Some(outcome)
    }

    pub fn remember(&mut self, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        if self.history.len() >= MAX_RECORDS {
            self.history.pop_front();
        }
        self.history.push_back(line.to_string());
        self.cursor = self.history.len();
    }

    pub fn history(&self) -> Vec<String> {
        self.history.iter().cloned().collect()
    }
}

/// What a command turned out to be, decided by the table rather than by a `match` chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Help,
    Launch,
    List,
    Focus,
    Close,
    Min,
    Max,
    Split,
    Undock,
    Theme,
    Setting,
    Stats,
    Clear,
    Echo,
    Report,
    Approvals,
    Approve,
    Reject,
    Blockers,
    Mode,
    Plan,
    /// **This tree's twenty-second verb, and the only one the reference does not have.**
    ///
    /// Filed in the player's own words and grown onto the chain, after which the compiler
    /// back-compiles it into a [`crate::shell::mount::Mount`]-able solution. The reference's
    /// surface is the twenty-one above and stays twenty-one; C-48 says a document informs and
    /// never rules, and an extension is declared rather than smuggled in as though the
    /// reference had it.
    File,
}

impl Verb {
    /// **The count as a literal, so a verb added without a row does not compile** — the same
    /// trade `FEATURE_COUNT` and `AXIS_COUNT` make, and the reason this is not `ALL.len()`.
    pub const COUNT: usize = 22;

    pub const ALL: [Verb; Verb::COUNT] = [
        Verb::Help,
        Verb::Launch,
        Verb::List,
        Verb::Focus,
        Verb::Close,
        Verb::Min,
        Verb::Max,
        Verb::Split,
        Verb::Undock,
        Verb::Theme,
        Verb::Setting,
        Verb::Stats,
        Verb::Clear,
        Verb::Echo,
        Verb::Report,
        Verb::Approvals,
        Verb::Approve,
        Verb::Reject,
        Verb::Blockers,
        Verb::Mode,
        Verb::Plan,
        Verb::File,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Verb::Help => "help",
            Verb::Launch => "launch",
            Verb::List => "list",
            Verb::Focus => "focus",
            Verb::Close => "close",
            Verb::Min => "min",
            Verb::Max => "max",
            Verb::Split => "split",
            Verb::Undock => "undock",
            Verb::Theme => "theme",
            Verb::Setting => "setting",
            Verb::Stats => "stats",
            Verb::Clear => "clear",
            Verb::Echo => "echo",
            Verb::Report => "report",
            Verb::Approvals => "approvals",
            Verb::Approve => "approve",
            Verb::Reject => "reject",
            Verb::Blockers => "blockers",
            Verb::Mode => "mode",
            Verb::Plan => "plan",
            Verb::File => "file",
        }
    }

    /// **`?` is an alias for `help` and nothing else** — the reference spells it that way and
    /// aliasing every verb would make the vocabulary unbounded.
    pub fn parse(text: &str) -> Option<Verb> {
        if text == "?" {
            return Some(Verb::Help);
        }
        Verb::ALL.into_iter().find(|verb| verb.name() == text)
    }

    /// How many arguments this verb takes. **`None` means "any number", and it is stated per
    /// verb rather than inferred** — the reference checks each branch separately, and a table
    /// that repeats the reference's checks is C-69's *declared list nobody reads* if nothing
    /// reads it.
    pub const fn arity(self) -> Option<(usize, usize)> {
        match self {
            Verb::Help | Verb::List | Verb::Stats | Verb::Clear | Verb::Report
            | Verb::Approvals | Verb::Blockers => Some((0, 0)),
            Verb::Echo | Verb::Theme => Some((1, usize::MAX)),
            Verb::Launch | Verb::Focus | Verb::Close | Verb::Min | Verb::Max | Verb::Plan
            | Verb::Approve | Verb::Reject | Verb::Mode => Some((1, 1)),
            // **One or more words, because a ticket is prose.** `file` takes the rest of the
            // line verbatim, which is why it is the only verb whose maximum is unbounded.
            Verb::File => Some((1, usize::MAX)),
            Verb::Split => Some((1, 2)),
            Verb::Undock => Some((1, 2)),
            Verb::Setting => Some((2, 2)),
        }
    }

    /// Every variant's own index. **Exhaustive by construction**: a twenty-second verb will not
    /// compile until it is given an index here, which is what makes this a witness rather than a
    /// restatement of [`Verb::COUNT`].
    pub const fn index(self) -> usize {
        match self {
            Verb::Help => 0,
            Verb::Launch => 1,
            Verb::List => 2,
            Verb::Focus => 3,
            Verb::Close => 4,
            Verb::Min => 5,
            Verb::Max => 6,
            Verb::Split => 7,
            Verb::Undock => 8,
            Verb::Theme => 9,
            Verb::Setting => 10,
            Verb::Stats => 11,
            Verb::Clear => 12,
            Verb::Echo => 13,
            Verb::Report => 14,
            Verb::Approvals => 15,
            Verb::Approve => 16,
            Verb::Reject => 17,
            Verb::Blockers => 18,
            Verb::Mode => 19,
            Verb::Plan => 20,
            Verb::File => 21,
        }
    }
}

/// How wide the enum is, which is its highest index plus one — and [`Verb::index`] is exhaustive,
/// so **this number cannot drift from the enum**, only from the table beside it.
pub const VARIANT_WIDTH: usize = Verb::File.index() + 1;

/// **The witness [`VERBS_ARE_TOTAL`] is not, and the reason is the whole point of both.**
///
/// `VERBS_ARE_TOTAL` compares `COUNT` to `ALL.len()`. A variant that is in **neither** satisfies
/// it — which is exactly what `Verb::Plan` did for as long as it was missing from the table, and
/// why that verb could be named in Rust, given an arity, and yet be untypable and absent from the
/// help while the pin stayed green.
///
/// This one compares the table to the **enum's own width**, so a variant outside the table is a
/// length mismatch and will not compile.
pub const VERBS_COVER_EVERY_VARIANT: [(); VARIANT_WIDTH] = [(); Verb::ALL.len()];

/// The verb table is total, and the value is meaningless while the type is the assertion.
///
/// **Necessary and not sufficient** — [`VERBS_COVER_EVERY_VARIANT`] is the one that can see the
/// enum. Kept because it is the older pin and because a reader who finds only this one has not
/// been told the whole truth.
pub const VERBS_ARE_TOTAL: [(); Verb::COUNT] = [(); Verb::ALL.len()];

/// Why a line is not a command. **Three refusals, and they are three different facts** — the
/// reference's `execute_command` catches everything into one `except` and prints a message, which
/// is the shape this repository refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRefusal {
    /// The word is not in the closed vocabulary. **Named, never guessed** — an operator typing
    /// `statss` gets told what the whole table holds.
    UnknownVerb { said: String },
    /// The right verb, the wrong number of arguments.
    Arity {
        verb: &'static str,
        wanted: String,
        got: usize,
    },
}

impl CommandRefusal {
    pub fn detail(&self) -> String {
        match self {
            CommandRefusal::UnknownVerb { said } => format!(
                "{said:?} is not a console verb. The vocabulary is closed and it is: {}",
                Verb::ALL.map(Verb::name).join(", ")
            ),
            CommandRefusal::Arity { verb, wanted, got } => {
                format!("{verb} takes {wanted} arguments and {got} were given")
            }
        }
    }
}

/// A parsed command line: the verb, and the arguments after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub verb: Verb,
    pub args: Vec<String>,
}

/// Parse one line. **The only parser**, so the console and the tests cannot disagree about what a
/// command is — `recipe::recipe_corpus_digest`'s "a caller reading a store must sort first"
/// discipline, applied to text.
pub fn parse(line: &str) -> Result<Command, CommandRefusal> {
    let mut parts = line.split_whitespace();
    let Some(head) = parts.next() else {
        return Err(CommandRefusal::UnknownVerb { said: String::new() });
    };
    let verb = Verb::parse(head.to_ascii_lowercase().as_str()).ok_or_else(|| {
        CommandRefusal::UnknownVerb { said: head.to_string() }
    })?;
    let args: Vec<String> = parts.map(ToOwned::to_owned).collect();
    if let Some((min, max)) = verb.arity() {
        if args.len() < min || args.len() > max {
            let wanted = if min == max {
                min.to_string()
            } else {
                format!("{min} or more")
            };
            return Err(CommandRefusal::Arity {
                verb: verb.name(),
                wanted,
                got: args.len(),
            });
        }
    }
    Ok(Command { verb, args })
}

/// The help text, **built from [`Verb::ALL`] rather than written out**.
///
/// The reference hand-writes the help string, so it drifts from the dispatch chain below it —
/// which is how a simulator ends up documenting a verb it no longer has. Here the help text is
/// *generated from the same table the parser uses*, and a verb nobody can discover is impossible
/// by construction.
pub fn help() -> Vec<String> {
    let mut lines = vec!["verbs (the vocabulary is closed; ? is help)".to_string()];
    for verb in Verb::ALL {
        let arity = match verb.arity() {
            Some((0, 0)) => String::new(),
            Some((min, max)) if min == max => format!(" ({min})"),
            Some((min, _)) => format!(" ({min}+)"),
            None => String::new(),
        };
        lines.push(format!("  {}{arity}", verb.name()));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_vocabulary_is_five_and_closed() {
        // The reference has five. A sixth would be a colour nobody chose, so the count is
        // asserted rather than `ALL.len()`.
        assert_eq!(Level::ALL.len(), 5);
        for level in Level::ALL {
            assert_eq!(Level::parse(level.slug()), Some(level));
        }
        assert_eq!(Level::parse("DEBUG"), None, "and an undeclared level is refused");
    }

    #[test]
    fn the_verb_table_is_total_and_covers_every_variant() {
        // **C-80.5 records "eleven" and lists twelve; the surface is twelve + `undock` +
        // `setting` + a governance set of *seven*, which is twenty-one.** The earlier "twenty"
        // was a miscount of the record's own list, and it reached the code as `COUNT = 20` with
        // `Verb::Plan` in no row. The measurement is the record's correction, and this pins the
        // corrected number so it cannot drift back into a document.
        assert_eq!(Verb::ALL.len(), Verb::COUNT);
        assert_eq!(Verb::COUNT, 22, "twelve, plus two, plus seven, plus this tree's `file`");
        assert_eq!(VERBS_ARE_TOTAL, [(); Verb::COUNT]);
        // The pin that would have caught it: the table is as wide as the enum.
        assert_eq!(VERBS_COVER_EVERY_VARIANT, [(); VARIANT_WIDTH]);
        // And the specific hole, by name, because a pin that is only ever asserted as a
        // compile-time constant has never been shown to fail.
        assert_eq!(Verb::parse("plan"), Some(Verb::Plan), "plan is typable");
        let lines = help();
        assert!(
            lines.iter().any(|line| line.contains("plan")),
            "and it is in the generated help: {lines:?}"
        );
        let names: Vec<&str> = Verb::ALL.iter().map(|v| v.name()).collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "no verb is listed twice: {names:?}");
        assert_eq!(Verb::parse("undock"), Some(Verb::Undock));
        assert_eq!(Verb::parse("setting"), Some(Verb::Setting));
        assert_eq!(Verb::parse("?"), Some(Verb::Help));
        assert_eq!(Verb::parse("statss"), None);
    }

    #[test]
    fn every_verb_in_the_table_is_reachable_by_parsing_its_own_name() {
        // **The help text is generated from this table, so a verb the parser cannot reach is a
        // row that documents nothing.** This is the check that keeps `help` honest.
        //
        // The first version of this test asserted every verb parses **bare**, and it failed on
        // `Launch` — which takes an app id. The code was right and the assertion was wrong: a
        // verb with a required argument refusing to parse bare is the arity rule working.
        for verb in Verb::ALL {
            let (min, _) = verb.arity().expect("every verb declares an arity");
            let mut line = verb.name().to_string();
            for index in 0..min {
                line.push_str(&format!(" arg{index}"));
            }
            let parsed = parse(&line);
            assert_eq!(
                parsed.map(|command| command.verb),
                Ok(verb),
                "{verb:?} must be reachable by its own name with its declared arguments"
            );
            if min > 0 {
                assert!(
                    parse(verb.name()).is_err(),
                    "{verb:?} needs {min} argument(s), so the bare name is correctly refused"
                );
            }
        }
    }

    #[test]
    fn an_unknown_verb_is_refused_by_name_and_names_the_whole_vocabulary() {
        let refusal = parse("frobnicate").expect_err("not a verb");
        assert_eq!(
            refusal,
            CommandRefusal::UnknownVerb { said: "frobnicate".to_string() }
        );
        assert!(refusal.detail().contains("frobnicate"));
        assert!(
            refusal.detail().contains("undock"),
            "and the reader is told what the whole table holds rather than left guessing: {}",
            refusal.detail()
        );
    }

    #[test]
    fn the_wrong_number_of_arguments_is_a_different_refusal_from_an_unknown_verb() {
        // Two different facts, so two different refusals — the reference catches both into one
        // `except` and prints one message.
        let refusal = parse("focus").expect_err("focus needs an id");
        assert_eq!(
            refusal,
            CommandRefusal::Arity {
                verb: "focus",
                wanted: "1".to_string(),
                got: 0
            }
        );
        assert_ne!(
            refusal,
            CommandRefusal::UnknownVerb { said: "focus".to_string() }
        );
    }

    #[test]
    fn log_is_the_only_way_in_and_the_console_is_bounded() {
        let mut console = Console::new();
        assert!(console.is_empty());
        for index in 0..(MAX_RECORDS + 10) {
            console.log(Level::Info, "TEST", format!("line {index}"), Some(index as u64));
        }
        assert_eq!(
            console.len(),
            MAX_RECORDS,
            "the ceiling holds rather than growing into a second ledger"
        );
        assert_eq!(
            console.dropped(),
            10,
            "and the eviction is counted, never silent — 2,048 lines having dropped 900 is a \
             different fact from never having held 2,948"
        );
        assert!(
            console.lines().last().unwrap().contains("line 2057"),
            "the newest line survives; the oldest ten are what the ceiling took"
        );
    }

    #[test]
    fn a_filtered_console_shows_one_level_and_says_so() {
        let mut console = Console::new();
        console.log(Level::Sys, "SHELL", "mounted", Some(0));
        console.log(Level::Warn, "PANEL", "a 358 dp panel in a 336 dp pane", Some(1));
        console.log(Level::Info, "SHADER", "bands 8 of 8", Some(2));

        assert_eq!(console.lines().len(), 3, "unfiltered shows everything");
        console.set_filter(Some(Level::Warn));
        let lines = console.lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("[WARN]"), "{lines:?}");
        assert!(
            lines[0].contains("a 358 dp panel in a 336 dp pane"),
            "and the refusal's own words survive the filter: {lines:?}"
        );
        console.set_filter(None);
        assert_eq!(console.lines().len(), 3, "and the filter is reversible");
    }

    #[test]
    fn a_record_with_no_tick_says_so_rather_than_reading_as_tick_zero() {
        // D107: absence rendered as absence. A record written before the clock starts has not
        // happened at tick zero.
        let record = Record {
            level: Level::Sys,
            category: "SHELL".to_string(),
            message: "before the clock".to_string(),
            tick: None,
        };
        assert!(record.describe().contains("t-"), "{}", record.describe());
        assert!(!record.describe().contains("t0"), "{}", record.describe());
    }

    #[test]
    fn warn_is_the_only_level_that_is_persistent() {
        // C-64.2's MD1 split as a predicate: a refusal is never a toast that scrolls away.
        for level in Level::ALL {
            assert_eq!(
                level.is_persistent(),
                level == Level::Warn,
                "{level:?}'s persistence"
            );
        }
    }

    #[test]
    fn clearing_the_output_keeps_what_you_typed() {
        let mut console = Console::new();
        console.log(Level::Info, "TEST", "output", Some(0));
        console.remember("focus 3");
        console.clear();
        assert!(console.is_empty(), "the screen is cleared");
        assert_eq!(
            console.history(),
            vec!["focus 3".to_string()],
            "and the command history survives, because a console that forgot your last command \
             is worse than one that forgot its output"
        );
    }

    #[test]
    fn the_help_text_is_generated_from_the_table_it_documents() {
        let lines = help();
        assert_eq!(lines.len(), Verb::COUNT + 1, "a header and one row per verb");
        for verb in Verb::ALL {
            assert!(
                lines.iter().any(|line| line.contains(verb.name())),
                "{verb:?} is discoverable"
            );
        }
    }

    #[test]
    fn the_parser_agrees_with_the_help_text_about_arity() {
        // **A verb the help text documents with an argument count the parser does not enforce
        // is a lie told in two places at once.** The reference hand-writes its help string; this
        // table generates it, and this test is what keeps the two in step.
        for verb in Verb::ALL {
            let (min, max) = verb.arity().expect("every verb declares an arity");
            let mut with_args = verb.name().to_string();
            for index in 0..min {
                with_args.push_str(&format!(" arg{index}"));
            }
            assert!(
                parse(&with_args).is_ok(),
                "{verb:?} must accept {min} arguments, which is what help() says"
            );
            if min > 0 {
                let mut too_few = verb.name().to_string();
                for index in 0..(min - 1) {
                    too_few.push_str(&format!(" arg{index}"));
                }
                assert!(
                    parse(&too_few).is_err(),
                    "{} must refuse {} arguments",
                    verb.name(),
                    min - 1
                );
            }
            if max < usize::MAX && max > 0 {
                let mut too_many = verb.name().to_string();
                for index in 0..(max + 1) {
                    too_many.push_str(&format!(" arg{index}"));
                }
                assert!(
                    parse(&too_many).is_err(),
                    "{} must refuse more than {max} arguments",
                    verb.name()
                );
            }
        }
    }
}
