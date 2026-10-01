//! Data-only shell recipes, version 2 — **UI-SPEC §2's `WindowSpec`, in full.**
//!
//! # Why this file was rewritten
//!
//! Version 1 of this schema was written before `docs/UI-SPEC.md` was read, and it did not
//! match it. Six fields the record requires were **absent entirely** — `lane`,
//! `extent_unit`, `layout_policy`, `behavior_policy`, `record_id`, `theme_role` — and the
//! `content` vocabulary was sixteen invented values where the record names a **closed
//! twelve**. That is recorded as C-64, and it is the same defect as the oracle vocabulary
//! in C-61: a plausible schema nobody checked against the record.
//!
//! # The format is ours, and the documents pass through it
//!
//! Ruling R-64.1. The schema is **defined here, versioned, and closed.** The
//! authoritative documents supply **values and vocabulary, never fields.** A document
//! that requires a field this schema does not have is a **refusal the build gate
//! surfaces** — not a schema change and not a silent extension.
//!
//! So `shploingus.recipe/3` is the pin. Bumping it is a recorded act, and an older header is
//! **refused** rather than converted: version 1 was a schema that did not match the record, and
//! version 2 had **no way to declare the bar's tabs** — a package in that schema cannot say which
//! of its windows are the bar's, and inferring the set (from the first eight rows, or from the
//! `tab` lane) would be exactly the plausible-substitute failure this crate exists to prevent.
//!
//! # The bar's tabs are declared, not inferred
//!
//! `tab<TAB>window-id` rows name the windows the upper bar carries as **tabs**, in strip order — the
//! same shape as `pin<TAB>window-id`, for the same reason. UI-SPEC §5 names the set and H-7.15 says the
//! shell *is* the upper bar; a shell that decided membership from position or from the lane hue
//! would be a second, unchecked answer to a question the package can answer itself.
//!
//! Two more rows complete the bar's shape, because H-15.2/H-15.3 ruled that a window has exactly one
//! **home** and that a tab *sorts* the windows filed under it:
//!
//! * `bar<TAB>window-id` — a bar entry that is **not** a tab. The console is the one today: it is the
//!   command surface (H-8.2), not a tab.
//! * `filed<TAB>window-id<TAB>tab-id` — a window offered by a tab's menu (the derivation of the
//!   simulator's start menu and slot grid, H-15.3).
//!
//! A declared window with **no** row at all is the console-opened case (H-15.2: `governance` and
//! `tools` are opened by the console), and that is a state rather than a defect: the console opens
//! any declared window by id, so no window is unreachable. A package may declare **no** tabs: a recipe
//! rendered from a ticket has no bar, and an empty bar is a real state.
//!
//! # One key per row, not eighteen columns
//!
//! Each window is a **block** of `key<TAB>value` rows under a `window<TAB>id` header.
//! A single 18-column tab-separated line is unreadable to a human and unreviewable by a
//! weaker model, and C-57 makes readability an acceptance requirement. The strictness is
//! unchanged and is in fact *higher*: an unknown **key** is refused, where version 1 had
//! no keys to be wrong about.
//!
//! # The content vocabulary, and the extension bucket
//!
//! UI-SPEC §8 requires every constant to carry its bucket, and requires the extensions
//! to be **counted by a test** *"so the bucket that can grow silently is the one that
//! cannot."* [`check_extension_count`] is that test.
//!
//! The twelve record kinds are named by the record. The **four extensions** exist because
//! the tab set (UI-SPEC §5) and the bar windows (§6) need windows the record's twelve do
//! not describe: the recipe trees, the settings knobs, the streamer, and notifications.
//! They are marked as extensions everywhere they appear, and they are the honest answer
//! to *"we need to evaluate the kinds of windows we want that come with the tabs."*

use std::collections::BTreeSet;

use voxel_model::{Digest, Identity, Refusal, RefusalKind, RefusalSet};

/// The pin. Older versions are refused; see the module header.
pub const HEADER: &str = "shploingus.recipe/3";
/// Every version this build accepts. Exactly one, and that is the point.
pub const ACCEPTED_VERSIONS: &[&str] = &[HEADER];

const MAGIC: &[u8; 8] = b"SHPKG002";

/// Alpha input ceilings — declared resource limits, never silent truncation.
pub const MAX_RECIPE_BYTES: usize = 1_048_576;
pub const MAX_WINDOWS: usize = 256;
/// Longest single value, so one row cannot be a megabyte of title.
pub const MAX_VALUE_BYTES: usize = 256;

/// How many content kinds the record names. UI-SPEC §2's closed twelve.
pub const RECORD_KIND_COUNT: usize = 12;
/// How many this repository adds. Counted by [`check_extension_count`].
pub const EXTENSION_KIND_COUNT: usize = 4;

/// Which bucket a content kind belongs to — UI-SPEC §8's discipline, applied to kinds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Bucket {
    /// Named by the record. Not this repository's to change.
    RecordRule,
    /// Declared here, and counted by a test so the bucket cannot grow silently.
    ProjectExtension,
}

impl Bucket {
    pub const fn slug(self) -> &'static str {
        match self {
            Bucket::RecordRule => "record_rule",
            Bucket::ProjectExtension => "project_extension",
        }
    }

    pub const fn describe(self) -> &'static str {
        match self {
            Bucket::RecordRule => "named by the record (UI-SPEC §2)",
            Bucket::ProjectExtension => {
                "declared by this repository because the tab set needs a window the record does \
                 not describe; counted by a test"
            }
        }
    }
}

/// What a window holds.
///
/// **The record's twelve, then this repository's four extensions.** The record's list is
/// closed; the extensions are here so the windows the tab set actually needs have a kind,
/// and they are marked as extensions in every rendering.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Content {
    // ---- the record's twelve (UI-SPEC §2) ----
    WorldView,
    Readout,
    Table,
    Log,
    Console,
    Ledger,
    ApiViewer,
    ShaperPanel,
    Mods,
    MapPlan,
    Tools,
    Inventory,
    // ---- four project extensions ----
    Trees,
    Settings,
    Streamer,
    Notifications,
}

impl Content {
    pub const ALL: [Self; RECORD_KIND_COUNT + EXTENSION_KIND_COUNT] = [
        Self::WorldView,
        Self::Readout,
        Self::Table,
        Self::Log,
        Self::Console,
        Self::Ledger,
        Self::ApiViewer,
        Self::ShaperPanel,
        Self::Mods,
        Self::MapPlan,
        Self::Tools,
        Self::Inventory,
        Self::Trees,
        Self::Settings,
        Self::Streamer,
        Self::Notifications,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::WorldView => "world-view",
            Self::Readout => "readout",
            Self::Table => "table",
            Self::Log => "log",
            Self::Console => "console",
            Self::Ledger => "ledger",
            Self::ApiViewer => "api-viewer",
            Self::ShaperPanel => "shaper-panel",
            Self::Mods => "mods",
            Self::MapPlan => "map-plan",
            Self::Tools => "tools",
            Self::Inventory => "inventory",
            Self::Trees => "trees",
            Self::Settings => "settings",
            Self::Streamer => "streamer",
            Self::Notifications => "notifications",
        }
    }

    /// Which bucket this kind is in. **Exhaustive**, so a new variant must be classified.
    pub const fn bucket(self) -> Bucket {
        match self {
            Self::WorldView
            | Self::Readout
            | Self::Table
            | Self::Log
            | Self::Console
            | Self::Ledger
            | Self::ApiViewer
            | Self::ShaperPanel
            | Self::Mods
            | Self::MapPlan
            | Self::Tools
            | Self::Inventory => Bucket::RecordRule,
            Self::Trees | Self::Settings | Self::Streamer | Self::Notifications => {
                Bucket::ProjectExtension
            }
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == text)
    }
}

/// The six lane hues. H-7.25, and **the six is the ruling.**
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Lane {
    Frame,
    Control,
    Tab,
    World,
    Beacon,
    Motion,
}

impl Lane {
    pub const ALL: [Self; 6] = [
        Self::Frame,
        Self::Control,
        Self::Tab,
        Self::World,
        Self::Beacon,
        Self::Motion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Frame => "frame",
            Self::Control => "control",
            Self::Tab => "tab",
            Self::World => "world",
            Self::Beacon => "beacon",
            Self::Motion => "motion",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|lane| lane.name() == text)
    }
}

/// Whether a window measures in screen pixels or in world units.
///
/// **Declared, not inferred** — this is what lets a pinned window be a HUD element or a
/// world-anchored one. ⚠️ The exact layer semantics for a pinned window are UI-SPEC §10's
/// open question (H-11.1) and are **not** decided here; the field exists so the decision
/// can be declared rather than guessed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ExtentUnit {
    SquarePixels,
    CubicWorldUnits,
}

impl ExtentUnit {
    pub const ALL: [Self; 2] = [Self::SquarePixels, Self::CubicWorldUnits];

    pub const fn name(self) -> &'static str {
        match self {
            Self::SquarePixels => "square-pixels",
            Self::CubicWorldUnits => "cubic-world-units",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|unit| unit.name() == text)
    }
}

/// Which chrome a window is **allowed**. A bitmask, so "none" is a real value.
///
/// Deny by default falls out of the representation rather than being a rule: a window
/// that declares no chrome has none, because the empty mask permits nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LayoutPolicy(pub u16);

impl LayoutPolicy {
    pub const NONE: Self = Self(0);
    pub const MINIMIZE: Self = Self(1 << 0);
    pub const MAXIMIZE: Self = Self(1 << 1);
    pub const CLOSE: Self = Self(1 << 2);
    pub const TITLE_POSITION: Self = Self(1 << 3);

    /// Every permitted token, in a fixed order so the rendering is stable.
    pub const PERMITTED: [(Self, &'static str); 4] = [
        (Self::MINIMIZE, "minimize"),
        (Self::MAXIMIZE, "maximize"),
        (Self::CLOSE, "close"),
        (Self::TITLE_POSITION, "title-position"),
    ];

    pub const fn permits(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }

    /// The declared tokens, comma-joined, or `none`.
    pub fn declared(self) -> String {
        let names: Vec<&str> = Self::PERMITTED
            .iter()
            .filter(|(flag, _)| self.permits(*flag))
            .map(|(_, name)| *name)
            .collect();
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(",")
        }
    }
}

/// Which behaviours a window is **allowed**. Same representation, same consequence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BehaviorPolicy(pub u16);

impl BehaviorPolicy {
    pub const NONE: Self = Self(0);
    pub const DRAG: Self = Self(1 << 0);
    pub const RESIZE: Self = Self(1 << 1);
    pub const MAXIMIZE: Self = Self(1 << 2);
    pub const MINIMIZE: Self = Self(1 << 3);
    pub const CLOSE: Self = Self(1 << 4);

    pub const PERMITTED: [(Self, &'static str); 5] = [
        (Self::DRAG, "drag"),
        (Self::RESIZE, "resize"),
        (Self::MAXIMIZE, "maximize"),
        (Self::MINIMIZE, "minimize"),
        (Self::CLOSE, "close"),
    ];

    pub const fn permits(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }

    pub fn declared(self) -> String {
        let names: Vec<&str> = Self::PERMITTED
            .iter()
            .filter(|(flag, _)| self.permits(*flag))
            .map(|(_, name)| *name)
            .collect();
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(",")
        }
    }
}

/// One window, exactly as UI-SPEC §2 tabulates it.
///
/// **The window declares; the shell owns `WindowState`** (normal/minimized/maximized/
/// closed), geometry at runtime, focus and z-order. A window that drew its own chrome
/// would be a window the shell cannot theme.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WindowSpec {
    /// Stable identity, **not a list position** (C-51 §51.3's lesson).
    pub id: String,
    pub title: String,
    pub content: Content,
    pub lane: Lane,
    pub extent_unit: ExtentUnit,
    /// dp: x, y, width, height. Integer declaration, converted at display scale.
    pub geometry: [u32; 4],
    pub min_size: [u32; 2],
    /// Thousandths, never a floating point serialized value.
    pub opacity: u16,
    /// `ELEV_STEP` × level, levels 0..4, **refused above rather than clamped**.
    pub elevation: u8,
    pub theme_role: String,
    /// May this window live in the HUD. Whether it *does* is the arrangement's decision.
    pub pinnable: bool,
    pub layout_policy: LayoutPolicy,
    pub behavior_policy: BehaviorPolicy,
    /// The window's own line in the one record (C-49 ruling 1).
    pub record_id: String,
}

/// Every key a window block carries, **in grammar order**.
///
/// **Declared once, because it was typed three times.** The parser's block terminator, the
/// missing-key report and the test that deletes each key all need this list; writing it out
/// three times is three places to forget a field, and the failure is silent — a window the
/// parser stops reading early, or a key no test exercises. [`WINDOW_FIELDS_ARE_TOTAL`] is the
/// compile-time check that it has not drifted from the [`WindowSpec`] it describes.
pub const WINDOW_FIELDS: [&str; 13] = [
    "title",
    "content",
    "lane",
    "extent",
    "geometry",
    "min-size",
    "opacity",
    "elevation",
    "theme-role",
    "pinnable",
    "layout",
    "behavior",
    "record-id",
];

/// The count the grammar promises, checked against the count the code has.
pub const WINDOW_FIELD_COUNT: usize = 13;

/// **`WINDOW_FIELDS` has not drifted.** A compile error, not a test, because a field
/// added to the struct without a key in the grammar is a *build* defect: the parser would
/// silently never read it.
pub const WINDOW_FIELDS_ARE_TOTAL: [(); WINDOW_FIELD_COUNT] = [(); WINDOW_FIELDS.len()];

/// The vocabulary this grammar declares, space-joined for [`crate::oracle::terms`].
///
/// **Derived, not typed.** Every term below is read out of the declaration that already
/// exists — the field keys, and the `name()` of every variant of every closed enum — so a
/// new content kind or a new permitted token licenses itself. The alternative, a list
/// written by hand beside the enums it mirrors, is [`census::CrateRow::means`]'s error
/// again: a second source that can disagree with the first, and nobody notices until the
/// judge convicts a document about a field that exists.
///
/// `none` is here because [`LayoutPolicy::declared`] and [`BehaviorPolicy::declared`] both
/// return it for the empty mask, and a renderer printing that word must not be convicted for
/// printing a value the grammar itself produces.
pub fn declared_window_vocabulary() -> String {
    let mut words: Vec<&str> = WINDOW_FIELDS.to_vec();
    words.extend(Content::ALL.into_iter().map(|kind| kind.name()));
    words.extend(Lane::ALL.into_iter().map(|lane| lane.name()));
    words.extend(ExtentUnit::ALL.into_iter().map(|unit| unit.name()));
    words.extend(LayoutPolicy::PERMITTED.iter().map(|(_, token)| *token));
    words.extend(BehaviorPolicy::PERMITTED.iter().map(|(_, token)| *token));
    words.push("none");
    words.join(" ")
}

/// A whole arrangement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Recipe {
    pub name: String,
    pub version: String,
    pub windows: Vec<WindowSpec>,
    /// Window ids the arrangement pins into the HUD. Pinning is a **layer move**, so it
    /// is declared here and not baked into the spec — a window that is pinnable but not
    /// pinned is the normal case, not an error.
    pub pinned: Vec<String>,
    /// Window ids the **upper bar** carries, in strip order (UI-SPEC §5, H-7.15).
    ///
    /// Declared rather than inferred: the shell draws the bar from this list, so a window
    /// cannot become a tab because it happened to be declared first. A recipe may declare
    /// none — a package compiled from a ticket has no bar, and that is a state, not a fault.
    pub tabs: Vec<String>,
    /// Bar entries that are **not** tabs, in the order they sit after the tabs (H-15.2).
    ///
    /// The console is the one today. It is on the bar because it is the command surface
    /// (H-8.2), and it is not a tab because a tab sorts windows and the console opens them.
    pub bar: Vec<String>,
    /// `(window, tab)` — the windows a tab's menu offers (H-15.3).
    ///
    /// **A window has exactly one home**, and these three rows are the homes: tab, `bar`, or
    /// filed under a tab. A window with none is console-opened and reachable by id.
    pub filed: Vec<(String, String)>,
}

fn refusal(detail: impl Into<String>) -> Refusal {
    Refusal::owned(RefusalKind::MalformedIdentity, detail)
}

/// Record a refusal **as a statement**.
///
/// [`RefusalSet::push`] returns `&mut Self` so calls can chain, which is the wrong
/// type for a `match` arm — so every arm in this file would otherwise need a block
/// around it. This macro keeps the arms reading as the single line each one is.
macro_rules! note {
    ($errors:expr, $detail:expr $(,)?) => {{
        $errors.push(refusal($detail));
    }};
}

fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn number(text: &str) -> Option<u32> {
    let n: u32 = text.parse().ok()?;
    (n.to_string() == text).then_some(n)
}

/// Whether a line is a **whole-line comment**.
///
/// A line whose first non-tab character is `#`. This is part of the format rather than a
/// loosening of it: a comment line is **skipped whole**, so it can never contribute a key,
/// a value, or a duplicate. The strictness that matters — unknown key, duplicate key,
/// missing key, unknown enum value — is untouched by it, and a declaration is far more
/// reviewable when it can say *why* a value is what it is.
///
/// A `#` **inside** a value is ordinary text and is not a comment, so a title may contain
/// one.
fn is_comment(line: &str) -> bool {
    line.trim_start_matches('\t')
        .trim_start()
        .starts_with('#')
}

/// A loaded package: the recipe, and the evidence of what it was built from.
///
/// Carrying the **exact source bytes** and both digests is the point. A loader that
/// returned only the parsed recipe would leave the caller unable to say *which bytes*
/// it admitted — and a package that cannot name its own bytes cannot be pinned,
/// recorded, or compared against a build that read different ones.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Loaded {
    pub recipe: Recipe,
    /// The exact source bytes this package carries.
    pub source: Vec<u8>,
    /// Digest of `source`. **Integrity**: these are the bytes the recipe describes.
    pub source_digest: Digest,
    /// Digest of the whole frame. A different artifact from the same source.
    pub package_digest: Digest,
}

impl Recipe {
    /// Strict source grammar, version 3.
    ///
    /// ```text
    /// shploingus.recipe/3
    /// package<TAB>id<TAB>version
    /// window<TAB>id
    ///     title           text
    ///     content         kind
    ///     lane            frame|control|tab|world|beacon|motion
    ///     extent          square-pixels|cubic-world-units
    ///     geometry        x y w h
    ///     min-size        w h
    ///     opacity         930..=1000   (thousandths)
    ///     elevation       0..=4
    ///     theme-role      identifier
    ///     pinnable        true|false
    ///     layout          none|minimize,maximize,close,title-position
    ///     behavior        none|drag,resize,maximize,minimize,close
    ///     record-id       identifier
    /// pin<TAB>window-id          (may be repeated)
    /// tab<TAB>window-id          (may be repeated; the order is the strip's)
    /// bar<TAB>window-id          (a bar entry that is not a tab; may be repeated)
    /// filed<TAB>window-id<TAB>tab-id   (may be repeated; one home per window)
    /// ```
    ///
    /// **Every listed key is required**, an unknown key is refused, and a duplicate key is
    /// refused — a plausible value that silently overwrote a real one is the failure this
    /// whole crate is built against.
    pub fn parse(source: &[u8]) -> Result<Self, RefusalSet> {
        let mut errors = RefusalSet::new();
        if source.len() > MAX_RECIPE_BYTES {
            note!(errors, format!(
                "recipe has {} bytes; ceiling {MAX_RECIPE_BYTES}",
                source.len()
            ));
            return Err(errors);
        }
        let text = match std::str::from_utf8(source) {
            Ok(text) => text,
            Err(error) => {
                note!(errors, format!("recipe is not UTF-8: {error}"));
                return Err(errors);
            }
        };
        if !text.ends_with('\n') || text.contains('\r') {
            note!(errors, "recipe requires LF lines and a final newline");
        }

        // Read every line up front and walk it **by index**. A `Lines` iterator cannot
        // push a line back, so a block reader that stops at the next block's first row
        // would silently drop that row. Indexing makes the boundary explicit.
        let all: Vec<&str> = text.lines().collect();
        let mut cursor = 0usize;

        let header = all.first().copied().unwrap_or("");
        cursor += 1;
        if !ACCEPTED_VERSIONS.contains(&header) {
            note!(errors, format!(
                "unknown recipe schema {header:?}; this build accepts {ACCEPTED_VERSIONS:?}. \
                 Version 1 is refused, not converted: it was a schema that did not match \
                 UI-SPEC §2, and reading it under version 2's rules would be a plausible \
                 substitute for a declared one"
            ));
        }

        let package: Vec<&str> = all.get(cursor).copied().unwrap_or("").split('\t').collect();
        cursor += 1;
        let (name, version) = if package.len() == 3
            && package[0] == "package"
            && identifier(package[1])
            && identifier(package[2])
        {
            (package[1].to_string(), package[2].to_string())
        } else {
            note!(errors, 
                "package row requires `package`, a canonical name and a canonical version",
            );
            (String::new(), String::new())
        };

        let mut windows: Vec<WindowSpec> = Vec::new();
        let mut pinned: Vec<String> = Vec::new();
        let mut tabs: Vec<String> = Vec::new();
        let mut bar: Vec<String> = Vec::new();
        let mut filed: Vec<(String, String)> = Vec::new();

        while cursor < all.len() {
            let line = all[cursor];
            cursor += 1;
            if line.trim().is_empty() || is_comment(line) {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[0] {
                "window" if fields.len() == 2 && identifier(fields[1]) => {
                    if windows.len() == MAX_WINDOWS {
                        note!(errors, format!(
                            "recipe declares more than {MAX_WINDOWS} windows"
                        ));
                        break;
                    }
                    match Self::read_window(&all, &mut cursor, fields[1]) {
                        Ok(spec) => windows.push(spec),
                        // The block is consumed either way, so one bad window does not
                        // cascade into every key after it.
                        Err(found) => {
                            errors.absorb(found);
                        }
                    }
                }
                "pin" if fields.len() == 2 && identifier(fields[1]) => {
                    pinned.push(fields[1].to_string());
                }
                "tab" if fields.len() == 2 && identifier(fields[1]) => {
                    tabs.push(fields[1].to_string());
                }
                "bar" if fields.len() == 2 && identifier(fields[1]) => {
                    bar.push(fields[1].to_string());
                }
                "filed" if fields.len() == 3
                    && identifier(fields[1])
                    && identifier(fields[2]) =>
                {
                    filed.push((fields[1].to_string(), fields[2].to_string()));
                }
                other => {
                    note!(errors, format!(
                        "unexpected row {other:?}: expected `window<TAB>id`, `tab<TAB>id`, \
                         `bar<TAB>id`, `filed<TAB>id<TAB>tab` or `pin<TAB>id`"
                    ));
                }
            }
        }

        // Structural checks that need the whole recipe.
        let mut ids: Vec<&str> = windows.iter().map(|w| w.id.as_str()).collect();
        ids.sort_unstable();
        let total = ids.len();
        ids.dedup();
        if ids.len() != total {
            note!(errors, 
                "two windows share an id, and a window is addressed by id rather than by \
                 list position",
            );
        }

        for pin in &pinned {
            match windows.iter().find(|w| &w.id == pin) {
                None => {
                    note!(errors, format!(
                        "pin names {pin:?}, which is not a declared window"
                    ));
                }
                Some(window) if !window.pinnable => {
                    note!(errors, format!(
                        "pin names {pin:?}, which declares pinnable=false"
                    ));
                }
                Some(_) => {}
            }
        }

        // A tab is a window on the bar, so it has to be a window. **A window may be both a tab
        // and pinned** — that is not a contradiction: pinning is where a window is *hosted*, and
        // the bar is where it is *opened from*.
        for tab in &tabs {
            if !windows.iter().any(|w| &w.id == tab) {
                note!(errors, format!(
                    "tab names {tab:?}, which is not a declared window"
                ));
            }
        }
        let mut tab_ids: Vec<&str> = tabs.iter().map(String::as_str).collect();
        tab_ids.sort_unstable();
        let tab_total = tab_ids.len();
        tab_ids.dedup();
        if tab_ids.len() != tab_total {
            note!(errors, 
                "one window is on the bar twice, and a tab is an entry rather than a position",
            );
        }

        // Every bar entry names a window, and the two kinds of entry are one set: a window
        // cannot be a tab *and* a plain bar entry, because the bar draws it once.
        for entry in bar.iter().chain(tabs.iter()) {
            if !windows.iter().any(|w| &w.id == entry) {
                note!(errors, format!(
                    "the bar names {entry:?}, which is not a declared window"
                ));
            }
        }
        let mut entries: Vec<&str> = tabs.iter().chain(bar.iter()).map(String::as_str).collect();
        entries.sort_unstable();
        let entry_total = entries.len();
        entries.dedup();
        if entries.len() != entry_total {
            note!(errors, 
                "one window is both a tab and a bar entry, or is entered twice; the bar draws \
                 a window once",
            );
        }

        // `filed` is where the tab menus come from (H-15.3). Its two refusals are the same
        // shape as `pin`'s: the window must exist and the tab must be on the bar.
        for (window, tab) in &filed {
            if !windows.iter().any(|w| &w.id == window) {
                note!(errors, format!(
                    "filed names {window:?}, which is not a declared window"
                ));
            }
            if !tabs.contains(tab) {
                note!(errors, format!(
                    "filed files {window:?} under {tab:?}, which is not a declared tab"
                ));
            }
            if tabs.contains(window) || bar.contains(window) {
                note!(errors, format!(
                    "{window:?} is a bar entry and is also filed under {tab:?}; a window has one \
                     home, and the bar already opens it"
                ));
            }
        }
        let mut filed_windows: Vec<&str> = filed.iter().map(|(window, _)| window.as_str()).collect();
        filed_windows.sort_unstable();
        let filed_total = filed_windows.len();
        filed_windows.dedup();
        if filed_windows.len() != filed_total {
            note!(errors, 
                "one window is filed under two tabs, so two menus would offer it",
            );
        }

        errors.into_result(Self { name, version, windows, pinned, tabs, bar, filed })
    }

    /// Read one window block: every required key, no unknown key, no duplicate.
    ///
    /// Takes the whole line vector and a cursor, and **leaves the cursor on the first
    /// line after the block** — so a key that is not ours ends the block and is still
    /// seen by the caller's grammar.
    fn read_window<'a>(
        all: &[&'a str],
        cursor: &mut usize,
        id: &str,
    ) -> Result<WindowSpec, RefusalSet> {
        let mut errors = RefusalSet::new();
        // Keys are slices of the caller's lines, so they borrow the input rather than
        // living forever. `seen` only has to answer "have I seen this key already".
        let mut seen: BTreeSet<&'a str> = BTreeSet::new();

        let mut title = String::new();
        let mut content = None;
        let mut lane = None;
        let mut extent = None;
        let mut geometry = None;
        let mut min_size = None;
        let mut opacity: Option<u16> = None;
        let mut elevation = None;
        let mut theme_role = String::new();
        let mut pinnable = None;
        let mut layout = None;
        let mut behavior = None;
        let mut record_id = String::new();

        while *cursor < all.len() {
            let line = all[*cursor];
            if line.trim().is_empty() || is_comment(line) {
                *cursor += 1;
                continue;
            }
            // **Indentation is presentational.** A block line is written `\tkey\tvalue`
            // so a person can read the shape, and the leading tab is not part of the key.
            // Stripping it here keeps the file readable (C-57) without weakening the
            // strictness that matters: an unknown key, a duplicate key and a missing key
            // are all still refused.
            let body = line.trim_start_matches('\t');
            let fields: Vec<&str> = body.split('\t').collect();
            let key = fields[0];
            // A key that is not ours starts the next block. The cursor is **not**
            // advanced, so the caller's grammar sees the row. **Read from
            // [`WINDOW_FIELDS`]**, so a new field is one edit rather than two.
            if !WINDOW_FIELDS.contains(&key) {
                break;
            }
            *cursor += 1;
            if !seen.insert(key) {
                note!(errors, format!("window {id}: duplicate key {key:?}"));
                continue;
            }
            if fields.len() < 2 || fields[1].len() > MAX_VALUE_BYTES {
                note!(errors, format!("window {id}: {key} needs a value"));
                continue;
            }
            let value = fields[1];
            match key {
                "title" => title = value.to_string(),
                "content" => match Content::parse(value) {
                    Some(kind) => content = Some(kind),
                    None => note!(errors, format!(
                        "window {id}: unknown content kind {value:?}; the vocabulary is closed \
                         and holds {}",
                        Content::ALL.len()
                    )),
                },
                "lane" => match Lane::parse(value) {
                    Some(found) => lane = Some(found),
                    None => note!(errors, format!(
                        "window {id}: unknown lane {value:?}; the six are the ruling"
                    )),
                },
                "extent" => match ExtentUnit::parse(value) {
                    Some(found) => extent = Some(found),
                    None => note!(errors, format!("window {id}: unknown extent {value:?}")),
                },
                "geometry" => match Self::four_numbers(value) {
                    Some(found) => geometry = Some(found),
                    None => note!(errors, format!(
                        "window {id}: geometry needs four integers"
                    )),
                },
                "min-size" => match Self::two_numbers(value) {
                    Some(found) => min_size = Some(found),
                    None => note!(errors, format!(
                        "window {id}: min-size needs two integers"
                    )),
                },
                // Parsed straight into `u16`: the range is 930..=1000 thousandths, which
                // fits exactly, so narrowing here is lossless and the constructor below
                // never has to convert a value it did not range-check.
                "opacity" => match Self::thousandths(value, 930, 1000) {
                    Some(found) => opacity = Some(found as u16),
                    None => note!(errors, format!(
                        "window {id}: opacity {value:?} is outside 930..=1000 thousandths"
                    )),
                },
                "elevation" => match Self::thousandths(value, 0, 4) {
                    Some(found) => elevation = Some(found as u8),
                    None => note!(errors, format!(
                        "window {id}: elevation {value:?} is outside 0..=4; refused above rather \
                         than clamped"
                    )),
                },
                "theme-role" => {
                    if identifier(value) {
                        theme_role = value.to_string();
                    } else {
                        note!(errors, format!(
                            "window {id}: theme-role {value:?} is not a canonical identifier"
                        ));
                    }
                }
                "pinnable" => match value {
                    "true" => pinnable = Some(true),
                    "false" => pinnable = Some(false),
                    other => note!(errors, format!(
                        "window {id}: pinnable must be true or false, not {other:?}"
                    )),
                },
                "layout" => match Self::layout_mask(value) {
                    Some(mask) => layout = Some(LayoutPolicy(mask)),
                    None => note!(errors, format!(
                        "window {id}: layout {value:?} names a chrome the window may not have"
                    )),
                },
                "behavior" => match Self::behavior_mask(value) {
                    Some(mask) => behavior = Some(BehaviorPolicy(mask)),
                    None => note!(errors, format!(
                        "window {id}: behavior {value:?} names a behaviour the window may not have"
                    )),
                },
                "record-id" => {
                    if identifier(value) {
                        record_id = value.to_string();
                    } else {
                        note!(errors, format!(
                            "window {id}: record-id {value:?} is not a canonical identifier"
                        ));
                    }
                }
                _ => {}
            }
        }

        // Every key is required, and **a missing key stops the window rather than
        // substituting a value for it.** Reporting all the missing keys at once, then
        // refusing, is both the useful behaviour and the only one that does not invent a
        // plausible field — `Default` is deliberately not derived anywhere in this file.
        let mut missing: Vec<&str> = Vec::new();
        for name in WINDOW_FIELDS {
            // **Presence is asked per key, in the key's own terms.** A parallel array of
            // booleans would be a second list to keep in step with the first, which is the
            // thing [`WINDOW_FIELDS`] exists to remove.
            let present = match name {
                "title" => !title.is_empty(),
                "content" => content.is_some(),
                "lane" => lane.is_some(),
                "extent" => extent.is_some(),
                "geometry" => geometry.is_some(),
                "min-size" => min_size.is_some(),
                "opacity" => opacity.is_some(),
                "elevation" => elevation.is_some(),
                "theme-role" => !theme_role.is_empty(),
                "pinnable" => pinnable.is_some(),
                "layout" => layout.is_some(),
                "behavior" => behavior.is_some(),
                "record-id" => !record_id.is_empty(),
                // Unreachable **because `WINDOW_FIELDS` is a closed literal and the match
                // above covers it**; a new field without a presence check would report the
                // field as missing forever, which is a loud failure rather than a wrong one.
                other => {
                    note!(errors, format!("window {id}: {other} has no presence check"));
                    false
                }
            };
            if !present {
                missing.push(name);
                note!(errors, format!("window {id}: {name} is required"));
            }
        }
        if !missing.is_empty() {
            return Err(errors);
        }

        // Unwrapping is safe **because of the check above**, and it is the only
        // `unwrap`-shaped step in this file. Every value here was just proved present.
        let spec = WindowSpec {
            id: id.to_string(),
            title,
            content: content.expect("proved present above"),
            lane: lane.expect("proved present above"),
            extent_unit: extent.expect("proved present above"),
            geometry: geometry.expect("proved present above"),
            min_size: min_size.expect("proved present above"),
            opacity: opacity.expect("proved present above"),
            elevation: elevation.expect("proved present above"),
            theme_role,
            pinnable: pinnable.expect("proved present above"),
            layout_policy: layout.expect("proved present above"),
            behavior_policy: behavior.expect("proved present above"),
            record_id,
        };

        errors.into_result(spec)
    }

    fn four_numbers(text: &str) -> Option<[u32; 4]> {
        let parts: Vec<&str> = text.split(' ').collect();
        if parts.len() != 4 {
            return None;
        }
        Some([
            number(parts[0])?,
            number(parts[1])?,
            number(parts[2])?,
            number(parts[3])?,
        ])
    }

    fn two_numbers(text: &str) -> Option<[u32; 2]> {
        let parts: Vec<&str> = text.split(' ').collect();
        if parts.len() != 2 {
            return None;
        }
        Some([number(parts[0])?, number(parts[1])?])
    }

    /// An integer in an inclusive range, refused outside it.
    fn thousandths(text: &str, low: u32, high: u32) -> Option<u32> {
        let value = number(text)?;
        (low..=high).contains(&value).then_some(value)
    }

    fn layout_mask(text: &str) -> Option<u16> {
        if text == "none" {
            return Some(0);
        }
        let mut mask = 0u16;
        for token in text.split(',') {
            let flag = match token {
                "minimize" => LayoutPolicy::MINIMIZE,
                "maximize" => LayoutPolicy::MAXIMIZE,
                "close" => LayoutPolicy::CLOSE,
                "title-position" => LayoutPolicy::TITLE_POSITION,
                _ => return None,
            };
            mask |= flag.0;
        }
        Some(mask)
    }

    fn behavior_mask(text: &str) -> Option<u16> {
        if text == "none" {
            return Some(0);
        }
        let mut mask = 0u16;
        for token in text.split(',') {
            let flag = match token {
                "drag" => BehaviorPolicy::DRAG,
                "resize" => BehaviorPolicy::RESIZE,
                "maximize" => BehaviorPolicy::MAXIMIZE,
                "minimize" => BehaviorPolicy::MINIMIZE,
                "close" => BehaviorPolicy::CLOSE,
                _ => return None,
            };
            mask |= flag.0;
        }
        Some(mask)
    }
}

/// The `.shpkg` frame: magic, payload length, payload digest, then the exact source.
///
/// The digest binds **these exact source bytes**, so a payload edited after the frame was
/// written is refused rather than mounted. This is **integrity, not identity** — see
/// `crate::signing` for why those are two different claims with two different names.
pub fn compile(source: &[u8]) -> Result<Vec<u8>, RefusalSet> {
    // Parse first, so a frame is never produced for a source this build refuses.
    Recipe::parse(source)?;
    let mut frame = Vec::with_capacity(source.len() + 80);
    frame.extend_from_slice(MAGIC);
    frame.extend_from_slice(&(source.len() as u32).to_le_bytes());
    frame.extend_from_slice(Digest::of(source).as_bytes());
    frame.extend_from_slice(source);
    Ok(frame)
}

/// Read a frame back, refusing anything that is not the frame this build writes.
pub fn load(frame: &[u8]) -> Result<Loaded, RefusalSet> {
    let mut errors = RefusalSet::new();
    let head = MAGIC.len() + 4 + 32;
    if frame.len() < head {
        note!(errors, "package frame is shorter than its own header");
        return Err(errors);
    }
    if &frame[..MAGIC.len()] != MAGIC {
        note!(errors, 
            "package frame does not carry this build's magic; a version-1 frame is refused, \
             not converted",
        );
        return Err(errors);
    }
    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&frame[MAGIC.len()..MAGIC.len() + 4]);
    let length = u32::from_le_bytes(length_bytes) as usize;
    let source = &frame[head..];
    if source.len() != length {
        note!(errors, format!(
            "package frame declares {length} payload bytes and carries {}",
            source.len()
        ));
    }
    let mut declared = [0u8; 32];
    declared.copy_from_slice(&frame[MAGIC.len() + 4..head]);
    if Digest::of(source).as_bytes() != &declared {
        note!(errors, 
            "package payload does not match the digest the frame declares; a payload edited \
             after the frame was written is refused rather than mounted",
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let recipe = Recipe::parse(source)?;
    Ok(Loaded {
        recipe,
        source: source.to_vec(),
        source_digest: Digest::of(source),
        package_digest: Digest::of(frame),
    })
}

impl Recipe {
    /// The content identity of a loaded recipe: `sha256:` over the exact source bytes.
    pub fn content_identity(&self, source: &[u8]) -> Identity {
        Identity::of(source)
    }
}

/// The extension-count test UI-SPEC §8 requires, as a function the suite can call.
///
/// *"Nothing in §8.2 is adopted by being pretty"* — so the bucket that can grow is the
/// bucket that gets counted. Adding a fifth extension makes **this** fail, which is the
/// point: the growth becomes a recorded act rather than a quiet one.
pub fn check_extension_count() -> Result<(), RefusalSet> {
    let extensions = Content::ALL
        .into_iter()
        .filter(|kind| kind.bucket() == Bucket::ProjectExtension)
        .count();
    let rules = Content::ALL.len() - extensions;

    let mut refusals = RefusalSet::new();
    if extensions != EXTENSION_KIND_COUNT {
        note!(
            refusals,
            format!(
                "content vocabulary holds {extensions} project extensions; \
                 {EXTENSION_KIND_COUNT} are declared. The bucket that can grow silently is \
                 the one a test counts, so growing it is a recorded act"
            )
        );
    }
    if rules != RECORD_KIND_COUNT {
        note!(
            refusals,
            format!(
                "content vocabulary holds {rules} record kinds; UI-SPEC §2 names \
                 {RECORD_KIND_COUNT}"
            )
        );
    }
    refusals.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_declared_vocabulary_carries_every_enum_value_the_parser_accepts() {
        // **The derivation is the claim.** If `declared_window_vocabulary` ever stopped
        // reading the enums, a document printing a real content kind would be convicted for
        // printing something the grammar itself produces. This fails the moment it does.
        let vocabulary = declared_window_vocabulary();
        for kind in Content::ALL {
            assert!(
                vocabulary.split(' ').any(|word| word == kind.name()),
                "{} is a content kind the parser accepts, so the declared vocabulary must \
                 carry {:?}",
                kind.name(),
                vocabulary
            );
        }
        for key in WINDOW_FIELDS {
            assert!(
                vocabulary.split(' ').any(|word| word == key),
                "{key} is a grammar key, so the declared vocabulary must carry it"
            );
        }
        // And it must not be a superset by accident: an invented term here would licence an
        // invented term in a document, which is the whole failure this function prevents.
        assert!(
            !vocabulary.split(' ').any(|word| word == "turbo"),
            "the declared vocabulary must not contain a term no declaration produced"
        );
    }

    const GOOD: &str = "\
shploingus.recipe/3
package\talpha-shell\tv1-alpha
window\tinventory
\ttitle\tInventory / crafting
\tcontent\tinventory
\tlane\ttab
\textent\tsquare-pixels
\tgeometry\t16 64 480 384
\tmin-size\t240 192
\topacity\t930
\televation\t1
\ttheme-role\tsurface
\tpinnable\ttrue
\tlayout\tminimize,maximize,close,title-position
\tbehavior\tdrag,resize,maximize,minimize,close
\trecord-id\tledger-window-inventory
pin\tinventory
";

    #[test]
    fn a_comment_is_part_of_the_format_and_cannot_smuggle_a_key() {
        // Comments make a declaration reviewable, which C-57 requires. They are skipped
        // **whole**, so one can never stand in for a key or a value.
        let commented = GOOD.replace(
            "window\tinventory\n",
            "# the first tab (UI-SPEC §5)\nwindow\tinventory\n\t# title is the bar's text\n",
        );
        let recipe = Recipe::parse(commented.as_bytes()).expect("comments are legal");
        assert_eq!(recipe.windows[0].title, "Inventory / crafting");

        // A commented-out key does not satisfy a required one.
        let removed = GOOD.replace("\ttitle\tInventory / crafting", "\t#\ttitle\tgone");
        let refusals = Recipe::parse(removed.as_bytes()).expect_err("a comment is not a key");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("title is required")),
            "{refusals}"
        );

        // And a `#` inside a value is ordinary text, not a comment.
        let hashed = GOOD.replace("\ttitle\tInventory / crafting", "\ttitle\tPanels #1");
        let recipe = Recipe::parse(hashed.as_bytes()).expect("a # in a value is text");
        assert_eq!(recipe.windows[0].title, "Panels #1");
    }

    #[test]
    fn a_complete_window_parses_and_carries_every_field_uispec_requires() {
        let recipe = Recipe::parse(GOOD.as_bytes()).expect("parses");
        assert_eq!(recipe.name, "alpha-shell");
        assert_eq!(recipe.windows.len(), 1);
        let window = &recipe.windows[0];
        assert_eq!(window.id, "inventory");
        assert_eq!(window.content, Content::Inventory);
        assert_eq!(window.lane, Lane::Tab);
        assert_eq!(window.extent_unit, ExtentUnit::SquarePixels);
        assert_eq!(window.geometry, [16, 64, 480, 384]);
        assert_eq!(window.min_size, [240, 192]);
        assert_eq!(window.opacity, 930);
        assert_eq!(window.elevation, 1);
        assert_eq!(window.theme_role, "surface");
        assert!(window.pinnable);
        assert!(window.layout_policy.permits(LayoutPolicy::CLOSE));
        assert!(window.behavior_policy.permits(BehaviorPolicy::DRAG));
        assert_eq!(window.record_id, "ledger-window-inventory");
        assert_eq!(recipe.pinned, vec!["inventory".to_string()]);
    }

    #[test]
    fn an_older_source_is_refused_rather_than_converted() {
        // The pin. A recipe/1 source is a schema that did not match UI-SPEC §2, and a
        // recipe/2 source cannot declare the bar's tabs; reading either under version 3's
        // rules would be a plausible substitute for a declared one.
        for version in ["shploingus.recipe/1", "shploingus.recipe/2"] {
            let source = format!("{version}\npackage\talpha-shell\tv1\n");
            let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
            assert!(!refusals.is_empty());
            assert!(
                refusals.all()[0].detail().contains("refused, not converted"),
                "{version}: {refusals}"
            );
        }
    }

    #[test]
    fn the_bar_s_tabs_are_declared_and_read_in_strip_order() {
        // The whole reason for the version bump. Two tabs, declared in order, and the order
        // is the strip's order — not declaration order of the windows, not a lane hue.
        let source = format!("{}tab\tinventory\ntab\tledger\n", good_with_ledger());
        let recipe = Recipe::parse(source.as_bytes()).expect("parses");
        assert_eq!(recipe.tabs, vec!["inventory".to_string(), "ledger".to_string()]);
        // A package with no bar is a state, not a fault.
        let barless = Recipe::parse(GOOD.as_bytes()).expect("parses");
        assert!(barless.tabs.is_empty());
    }

    #[test]
    fn a_tab_that_names_no_window_or_appears_twice_is_refused() {
        let ghost = format!("{}tab\tno-such-window\n", GOOD);
        let refusals = Recipe::parse(ghost.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("no-such-window")),
            "{refusals}"
        );

        let twice = format!("{}tab\tinventory\ntab\tinventory\n", GOOD);
        let refusals = Recipe::parse(twice.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("on the bar twice")),
            "{refusals}"
        );
    }

    #[test]
    fn a_window_may_be_a_tab_and_pinned_because_those_are_different_questions() {
        // Pinning is where a window is *hosted*; the bar is where it is *opened from*. The
        // shipped package pins a window that is not on the bar and could equally pin one
        // that is, so the two lists are allowed to overlap.
        let source = format!("{}tab\tinventory\n", GOOD); // GOOD already pins inventory
        let recipe = Recipe::parse(source.as_bytes()).expect("parses");
        assert_eq!(recipe.tabs, vec!["inventory".to_string()]);
        assert_eq!(recipe.pinned, vec!["inventory".to_string()]);
    }

    #[test]
    fn a_bar_entry_that_is_not_a_tab_is_declared_and_a_window_has_one_home() {
        // H-15.2: the console is on the bar and is not a tab. H-15.3: a tab sorts the windows
        // filed under it. Both are rows, and both are validated against the declared windows.
        let source = format!(
            "{}window\tledger\n\ttitle\tLedger\n\tcontent\tledger\n\tlane\tframe\n\textent\tsquare-pixels\n\tgeometry\t16 64 480 384\n\tmin-size\t240 192\n\topacity\t930\n\televation\t1\n\ttheme-role\tsurface\n\tpinnable\tfalse\n\tlayout\tclose\n\tbehavior\tclose\n\trecord-id\tledger-window\ntab\tinventory\nbar\tledger\nfiled\tledger\tinventory\n",
            GOOD
        );
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("one home")),
            "a bar entry cannot also be filed: {refusals}"
        );

        let ok = format!(
            "{}window\tledger\n\ttitle\tLedger\n\tcontent\tledger\n\tlane\tframe\n\textent\tsquare-pixels\n\tgeometry\t16 64 480 384\n\tmin-size\t240 192\n\topacity\t930\n\televation\t1\n\ttheme-role\tsurface\n\tpinnable\tfalse\n\tlayout\tclose\n\tbehavior\tclose\n\trecord-id\tledger-window\ntab\tinventory\nbar\tinventory\n",
            GOOD
        );
        let refusals = Recipe::parse(ok.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("both a tab and a bar entry")),
            "the bar draws a window once: {refusals}"
        );

        let filed = format!("{}tab\tinventory\nfiled\tledger\tinventory\n", good_with_ledger());
        let recipe = Recipe::parse(filed.as_bytes()).expect("a filed window parses");
        assert_eq!(
            recipe.filed,
            vec![("ledger".to_string(), "inventory".to_string())]
        );

        let ghost_tab = format!("{}tab\tinventory\nfiled\tledger\tnope\n", good_with_ledger());
        let refusals = Recipe::parse(ghost_tab.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("not a declared tab")),
            "{refusals}"
        );
    }

    /// [`GOOD`]'s window, plus one more window for the filing tests to name.
    fn good_with_ledger() -> String {
        format!(
            "{}window\tledger\n\ttitle\tLedger\n\tcontent\tledger\n\tlane\tframe\n\textent\tsquare-pixels\n\tgeometry\t16 64 480 384\n\tmin-size\t240 192\n\topacity\t930\n\televation\t1\n\ttheme-role\tsurface\n\tpinnable\tfalse\n\tlayout\tclose\n\tbehavior\tclose\n\trecord-id\tledger-window\n",
            GOOD
        )
    }

    #[test]
    fn the_content_vocabulary_is_the_records_twelve_plus_four_extensions() {
        // UI-SPEC §8: the extensions are counted so the bucket that can grow silently is
        // the one that cannot.
        check_extension_count().expect("twelve record kinds and four extensions");
        assert_eq!(Content::ALL.len(), RECORD_KIND_COUNT + EXTENSION_KIND_COUNT);
        for kind in Content::ALL {
            assert!(
                !kind.bucket().describe().is_empty(),
                "{} has an unrendered bucket",
                kind.name()
            );
        }
    }

    #[test]
    fn the_lane_vocabulary_is_exactly_six() {
        // H-7.25, and the six is the ruling.
        assert_eq!(Lane::ALL.len(), 6);
        let mut names: Vec<&str> = Lane::ALL.iter().map(|lane| lane.name()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["beacon", "control", "frame", "motion", "tab", "world"]
        );
    }

    #[test]
    fn an_unknown_content_kind_or_lane_is_refused_by_name() {
        for (key, value) in [("content", "telemetry"), ("lane", "sepia")] {
            let source = GOOD.replace(&format!("\t{key}\t"), &format!("\t{key}\t{value}\t"));
            let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
            assert!(
                refusals.all().iter().any(|r| r.detail().contains(value)),
                "{key}={value} must be named: {refusals}"
            );
        }
    }

    #[test]
    fn a_missing_required_key_is_refused_and_every_one_is_reported() {
        // **Over [`WINDOW_FIELDS`]**, so a key added to the grammar is deleted and proven
        // required here too. A test that enumerates its own list proves nothing about a
        // field it never heard of.
        for key in WINDOW_FIELDS {
            let mut kept = String::new();
            for line in GOOD.lines() {
                if line.starts_with(&format!("\t{key}\t")) {
                    continue;
                }
                kept.push_str(line);
                kept.push('\n');
            }
            let refusals = Recipe::parse(kept.as_bytes())
                .err()
                .unwrap_or_else(|| panic!("{key} must be required"));
            assert!(
                refusals.all().iter().any(|r| r.detail().contains(key)),
                "{key} must be named in a refusal: {refusals}"
            );
        }
    }

    #[test]
    fn elevation_above_four_is_refused_rather_than_clamped() {
        let source = GOOD.replace("\televation\t1", "\televation\t5");
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(refusals.all()[0].detail().contains("refused above"));
    }

    #[test]
    fn opacity_outside_its_declared_range_is_refused() {
        for value in ["929", "1001", "0"] {
            let source = GOOD.replace("\topacity\t930", &format!("\topacity\t{value}"));
            assert!(
                Recipe::parse(source.as_bytes()).is_err(),
                "opacity {value} must be refused"
            );
        }
    }

    #[test]
    fn a_policy_denies_by_default_because_the_empty_mask_permits_nothing() {
        // Not a rule bolted on: "none" is a real value and it permits nothing.
        assert!(!LayoutPolicy::NONE.permits(LayoutPolicy::CLOSE));
        assert!(!BehaviorPolicy::NONE.permits(BehaviorPolicy::DRAG));
        assert_eq!(LayoutPolicy::NONE.declared(), "none");

        let source = GOOD
            .replace(
                "\tlayout\tminimize,maximize,close,title-position",
                "\tlayout\tnone",
            )
            .replace(
                "\tbehavior\tdrag,resize,maximize,minimize,close",
                "\tbehavior\tnone",
            );
        let recipe = Recipe::parse(source.as_bytes()).expect("none is a legal declaration");
        assert!(!recipe.windows[0].layout_policy.permits(LayoutPolicy::MINIMIZE));
        assert!(!recipe.windows[0].behavior_policy.permits(BehaviorPolicy::RESIZE));
    }

    #[test]
    fn a_policy_naming_something_the_window_may_not_have_is_refused() {
        let source = GOOD.replace(
            "\tlayout\tminimize,maximize,close,title-position",
            "\tlayout\tminimize,teleport",
        );
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(refusals.all()[0].detail().contains("teleport"));
    }

    #[test]
    fn pinning_a_window_that_is_not_pinnable_is_refused() {
        let source = GOOD.replace("\tpinnable\ttrue", "\tpinnable\tfalse");
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(refusals.all().iter().any(|r| r.detail().contains("pinnable=false")));
    }

    #[test]
    fn pinning_a_window_that_does_not_exist_is_refused() {
        let source = format!("{GOOD}pin\tno-such-window\n");
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(refusals.all().iter().any(|r| r.detail().contains("no-such-window")));
    }

    #[test]
    fn a_compile_and_load_round_trip_preserves_the_exact_source() {
        let frame = compile(GOOD.as_bytes()).expect("compiles");
        let loaded = load(&frame).expect("loads");
        assert_eq!(loaded.recipe, Recipe::parse(GOOD.as_bytes()).expect("parses"));
        // The evidence a caller needs to say *which* bytes it admitted.
        assert_eq!(loaded.source, GOOD.as_bytes());
        assert_eq!(loaded.source_digest, Digest::of(GOOD.as_bytes()));
        assert_eq!(loaded.package_digest, Digest::of(&frame));
        assert_ne!(loaded.source_digest, loaded.package_digest);
    }

    #[test]
    fn a_tampered_payload_is_refused_rather_than_mounted() {
        let frame = compile(GOOD.as_bytes()).expect("compiles");
        let mut tampered = frame.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        let refusals = load(&tampered).expect_err("must refuse");
        assert!(!refusals.is_empty());
    }

    #[test]
    fn a_truncated_or_trailing_frame_is_refused() {
        let frame = compile(GOOD.as_bytes()).expect("compiles");
        assert!(load(&frame[..frame.len() - 1]).is_err(), "truncated");
        let mut extended = frame.clone();
        extended.extend_from_slice(b"extra");
        assert!(load(&extended).is_err(), "trailing bytes");
    }

    #[test]
    fn a_duplicate_key_is_refused_rather_than_silently_overwriting() {
        // A plausible value overwriting a real one is the failure this crate exists
        // against, so it is refused rather than resolved. The duplicate has to sit
        // **inside** the window block — after the `pin` row it would be a top-level row
        // and would be refused for a different reason, which is not what is under test.
        let source = GOOD.replace("\npin\tinventory", "\n\topacity\t980\npin\tinventory");
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("duplicate key")),
            "{refusals}"
        );
    }

    /// The same window block **verbatim**, so its id is duplicated on purpose.
    fn repeated_window_block() -> String {
        format!(
            "{}\n",
            GOOD.lines().skip(2).collect::<Vec<_>>().join("\n")
        )
    }

    #[test]
    fn a_window_is_addressed_by_id_and_two_may_not_share_one() {
        // C-51 §51.3's lesson: a window addressed by list position stops meaning anything
        // the moment one closes. Two windows with the same id would make the id
        // ambiguous, so the second is refused rather than shadowing the first.
        let source = format!("{GOOD}{}", repeated_window_block());
        let refusals = Recipe::parse(source.as_bytes()).expect_err("must refuse");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("share an id")),
            "{refusals}"
        );
    }

    /// The same window, given a different identity. Built from [`GOOD`] by renaming
    /// three tokens, so the two blocks cannot drift apart as the schema changes.
    fn second_window_block() -> String {
        let block: String = GOOD.lines().skip(2).collect::<Vec<_>>().join("\n");
        format!(
            "{}\n",
            block
                .replace("window\tinventory", "window\tledger")
                .replace("ledger-window-inventory", "ledger-window-ledger")
                .replace("pin\tinventory", "pin\tledger")
        )
    }

    #[test]
    fn a_second_window_block_is_read_and_the_first_is_not_swallowed() {
        // The block-boundary regression: a reader that cannot push a line back loses the
        // `window` row that ends the previous block.
        let source = format!("{GOOD}{}", second_window_block());
        let recipe = Recipe::parse(source.as_bytes()).expect("two blocks parse");
        assert_eq!(recipe.windows.len(), 2, "both windows are read");
        assert_eq!(recipe.windows[0].id, "inventory");
        assert_eq!(recipe.windows[1].id, "ledger");
        assert_eq!(recipe.pinned, vec!["inventory", "ledger"]);
    }

    #[test]
    fn non_utf8_and_oversized_sources_are_refusals_not_panics() {
        assert!(Recipe::parse(&[0xff, 0xfe, 0x00]).is_err());
        let huge = vec![b'x'; MAX_RECIPE_BYTES + 1];
        assert!(Recipe::parse(&huge).is_err());
    }
}
