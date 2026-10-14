//! **U10 — the gameplay tree the mods references imply.**
//!
//! Derived from the **U10** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16 (its hard prerequisite,
//! its interface and its §5 row-10 bar), from `docs/MODS-MAP.md` §1–§5 (the disposition map: the 91
//! URLs, the four words, the refusals, the load-bearing class), from `docs/MODS-MATERIALS.md` §8.1
//! and §8.2 (the deduped rule library: *"a technology is a rule that transforms matter"*, *"rows dedupe
//! to one per technology, citing every swept mod that declares it"*, *"the roster must never grow a
//! second row for a descendant"*), from `docs/DESIGN-SPEC.md` ruling 3 (the first target), from
//! `docs/LEDGER.md` **R-64.1** and **§55.3**, and from `specs/007-u10-gameplay-tree/` (this slice's own
//! spec, plan, data model and contracts). No `.rs` under `crates/` was read (P7), and **no mod source is
//! read at all**: the harvest (`docs/MODS-MATERIALS.md`, and the `_harvest/*` artifacts it names under
//! `F:/shploingus-mod-sources/`) is the reading, already done and HEAD-pinned. The one rule is inherited
//! whole: **refuse, never default**.
//!
//! # What the tree is, and what it is not
//!
//! The tree is **two lists and their link**:
//!
//! - **one entry per reference** in `docs/MODS-MAP.md` §1–§5, in the record's own order, each carrying
//!   the record's **disposition** for it (`mapped`, `open`, `refused`, `unresolved`), and — for a
//!   `mapped` or `refused` one — the row it declares; and
//! - **one row per technology** the record's references declare, each row citing the references the
//!   record cites for it.
//!
//! The **row is U6's row**, unchanged ([`crate::intake::Row`]): feature, `Source` (`path + digest`, read
//! **in place**), the licence verdict it relies on, the system that carries it (U3–U8), its own
//! acceptance check, a claim and what it requires of the world. The tree adds **no** row type, **no**
//! axis, **no** verb and **no** dependency; it is admitted through the gate the table already declares
//! (`admit`, served by `serve` over `specs/005-u6-record-api/contracts/u6-transport.md`). A row is an
//! **admission, never a transcription** (§16 row 10's own hard prerequisite).
//!
//! Nothing here implements anything. A row that is admitted says what *was* established — the document
//! was read where it lives and hashed to the digest it declares, the verdict it relies on permits it,
//! the carrier is declared, its own bar exists at its own reference — and it says what was **not**: a
//! checksum establishes no origin, and a row that requires nothing of the world asked nothing.
//!
//! # The three readings this table takes, named rather than hidden
//!
//! 1. **A reference's disposition is the record's own word.** `mapped` where the record states a feature
//!    for the reference (the map's own "What it gives this repo" column in §1/§3/§4; the harvest's §8.1
//!    rule row in §2); `open` where the record assigns it **no** feature (the map's `*`, §3's *"N/A until
//!    multiplayer"*, and the §2 references the harvest names in no rule and describes no process for);
//!    `refused` where a licence fact forbids deriving from it (the map's own set: `BigGlobe`,
//!    `lithostitched`, the `archipelago` datapack, plus `MFR` — *"no licence file anywhere in the
//!    tree"* — and `Create-The_Factory_Must_Grow`, the highest-caution reference);
//!    `unresolved` where the record's own destination says *"Unresolved — recorded, not guessed"*.
//! 2. **A reference's technology is the one the record states for it**, read in this order: the process
//!    the harvest's own measured passage states for the reference; otherwise the first §8.1 rule row that
//!    cites it; otherwise the reference's own class row in the map's §3/§4. Where none of the three names
//!    one, the record assigned no feature and the entry is `open` — a feature is exactly what a missing
//!    rule row would have named, and `docs/MODS-MAP.md` §2 says so in its own words (*"an `open` mod is
//!    not a rejection: it means the record assigned no feature"*).
//! 3. **Two lists, kept apart on purpose.** A technology row's **`cited_by`** is the record's own list of
//!    who proposes the rule (`docs/MODS-MATERIALS.md` §8.1, which carries the double-cover proof: an older
//!    mod and its Create-addon descendant on **one** row); the references that **declare** the row — the
//!    entries whose technology names it — are what its licence verdict is read from, so a rule declared by
//!    a refused reference is refused. And because the verdicts are **per reference** (`docs/MODS-MAP.md`
//!    §1), a permitted row that cites a no-derivation reference keeps its own verdict and **prints the
//!    refused citation** ([`Technology::refused_citations`]) rather than averaging the two into silence.
//!    §8.2's own proof is measured on `cited_by`, and a test holds that every declaring entry appears in
//!    it.
//!
//! **The three references left without an entry, given one (2026-10-12, user).** `TerraFirmaCraft` and
//! `Calculator` are in `docs/references.md` and are cited all through `docs/MODS-MATERIALS.md` §8.1, but
//! they were never in the map's §2 slot table and so had no entry; `Minecraft-CameraOverhaul` had neither
//! an entry nor a mention of its own — its only trace was inside the `resquake` note — although
//! `docs/LEDGER.md` C-150 names it as part of the player controller. Each now has an entry, and each
//! entry's evidence is stated in the map's own row for it: `TerraFirmaCraft` → `smelting` (clone
//! `6e77733c`, **EUPL-1.2**), `Calculator` → `smelting` (clone `c796f1dc`, **MIT** © 2018 Ollie
//! Lansdell), and `Minecraft-CameraOverhaul` → `open` (**GPL-3.0**, adopted into the game's player
//! controller with `resquake`, not in the clean room, so the record assigns it no row). Where a
//! description and a name disagree, the **smaller reading** is taken and said: both new `mapped` entries
//! are filed under each reference's **first** §8.1 rule (furnace firing / smelting), because the harvest's
//! measured passage states a whole processing chain for a reference while the rule library states rules
//! one at a time — a feature is a rule, so the first rule that cites a reference is the one it declares.
//!
//! **The record's own corrections, carried (2026-10-12).** Three rows of `docs/MODS-MAP.md` were
//! corrected by the user, and the entries carry them rather than inheriting the old words: `tt20` is a
//! **performance mod** (a TPS fixer), not the network/tick budget; `resquake` is the **Squake movement
//! model** — with `Minecraft-CameraOverhaul` added to the game's player controller
//! (`docs/LEDGER.md` C-150), and **not in the clean room** — so its entry leaves `unresolved` for
//! `open`, the record assigning it no row; and the Archipelago row was **re-keyed**: the reference is the
//! Modrinth **datapack** `archipelago` — world generation (island/archipelago terrain) — whose own terms
//! declare **CC-BY-NC-ND-4.0** (non-commercial, **no derivatives**), so its entry is `refused` rather than
//! `open`, and `MinecraftFabricArchipelagoMod` is demoted to the replaced URL it is: the Fabric client
//! that interacts with the Archipelago randomiser.
//!
//! # The refusals this tree produces, and the two names that are owed
//!
//! The entry's own shape rules refuse by the **closest declared name** and print the gap, because the
//! record gives no word for either fact (`specs/007-u10-gameplay-tree/contracts/u10-seams.md` C1):
//!
//! | fact | refused as | why that declared name |
//! |---|---|---|
//! | a `mapped` entry carries **no** row | `argument_absent` | a declaration is missing and there is no default — the very fact that name exists for |
//! | an `open`/`unresolved` entry **carries** a row | `argument_malformed` | a value present that is not of its declared kind: the record assigned this reference no feature, so it cannot be a row |
//! | a **second row** for a technology the tree already holds | `duplicate_output` | two declarations claiming the same thing — the shape `duplicate_material`/`duplicate_shaper`/`duplicate_output` already name, taken as the closest |
//!
//! The **exact** word for a second row on a descendant technology is **owed**: `docs/MODS-MATERIALS.md`
//! §8.2 states the rule (*"never grow a second row for a descendant"*) and no reference states the word.
//! It is named in [`Tree::owed`], printed by the binary and written in the spec's Assumptions — and it is
//! not coined here.
//!
//! # What this slice does **not** land (named, not implied)
//!
//! 1. **The recipe document's row-kind key and version.** A gameplay entry is a new row kind in a format
//!    `docs/LEDGER.md` **R-64.1** makes *versioned and closed* (`packages/alpha-shell.shspec` opens
//!    `shploingus.recipe/3` and declares `window`/`tab`/`bar`/`filed`/`pin` rows). No reference chooses the
//!    key or the number, so none is invented: the tree cites the *existing* document form and prints the
//!    gap.
//! 2. **The duplicate-row refusal's name** (the paragraph above).
//! 3. **A verb that serves the whole tree.** U6's table is closed and bijective and no design declares one;
//!    the tree's **rows** reach a client today, over `admit`. What the tree reports it holds is
//!    [`Tree::render`], read in this process.
//! 4. **The per-row family assignment** and the `SUBSTANCES` table shape. `docs/MODS-MATERIALS.md` §5 and
//!    §8.3 leave it owed (*"per-row family assignment (the ten families) … on the way to the `SUBSTANCES`
//!    table shape"*), and §8.3 calls its own progression model *"a design sketch, falsifiable"*. This
//!    module names the technologies and does **not** file them under families.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::intake::{
    Acceptance, Carrier, Claim, Establishment, Licence, Requirements, Row, Source,
};
use crate::refusal::{RefusalKind, RefusalSet};

// ---------------------------------------------------------------------------
// The record's own documents, and the digests the rows declare.
// ---------------------------------------------------------------------------

/// The map: one row per reference, with the record's disposition for each and — where the record states
/// one — the feature it gives. This is the document a §1/§3/§4 row's source points at.
pub const MAP_PATH: &str = "docs/MODS-MAP.md";

/// The harvest: the materials and technology reading, deduped to one row per rule. This is the document a
/// §2 row's source points at — the tree never re-reads a mod (D10).
pub const HARVEST_PATH: &str = "docs/MODS-MATERIALS.md";

/// U3's own declared bar: the paper-per-shaper contract (`docs/SHAPERS.md` §1). A row whose work **is** one
/// of U3's declared shapers declares this as its acceptance check — the carrying system's own bar, never
/// one invented for the tree.
pub const SHAPERS_PATH: &str = "docs/SHAPERS.md";

/// U4's own declared bar, as that slice states it: the commands and the numbers it pasted. A row whose work
/// is U4's horizon/residency ladder declares this.
pub const U4_QUICKSTART_PATH: &str = "clean-room/specs/004-u4-mesh-stream-work/quickstart.md";

/// The digest `docs/MODS-MAP.md` hashes to. Read **in place** at every admission, so a document that has
/// moved refuses by name rather than being believed.
pub const MAP_DIGEST: &str = "11873cad81c2af8d6ed344097019a46f88f1810e6cf9d729d2ece462295ca77f";

/// The digest `docs/MODS-MATERIALS.md` hashes to.
pub const HARVEST_DIGEST: &str = "6ddffde0fa7be61ee8208b14d0e4794c087ac612f65ee3d801a132fd8902fcaf";

/// The digest `docs/SHAPERS.md` hashes to.
pub const SHAPERS_DIGEST: &str = "426b58d7f518aef6cd196d99b9c9659577e63bd030495fa046fd74be7309cff7";

/// The digest U4's quickstart hashes to. (§39, 2026-10-13: the document gained §39's dated note on the
/// mesh's unit — the faces between a solid cell and air — so this declared digest moved from
/// `481412a537b29391…` to the document's own new bytes; the guard below is what made the move visible.)
pub const U4_QUICKSTART_DIGEST: &str = "ca00f1bef771d7394ab82a243b34257b031144d015ce5f4d1107abfb08d3d430";

/// **The root the record's documents are read from**: the workspace this crate lives in — `clean-room`'s
/// parent — because the map, the harvest and the shapers are the workspace's own documents, cited by the
/// path they have there. A row's `path` is relative to this root, exactly as `Intake::over` declares.
pub fn record_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A digest that is declared as text, refused as anything else — a wrong constant is a bug in this file,
/// and `Digest::from_hex` is the one reader that decides what a digest is.
fn declared_digest(hex: &'static str) -> Digest {
    match Digest::from_hex(hex) {
        Some(digest) => digest,
        None => unreachable!("a declared digest in this module is a digest: {hex}"),
    }
}

// ---------------------------------------------------------------------------
// The map's own rows — one parse, read where they live.
// ---------------------------------------------------------------------------

/// The spelling folds, exactly as the map prints them: the name in the map → the name in the tree.
/// `chunky`/`tidal` are the bare spellings §2's dated block and §5's list use for `MOD-chunky` and
/// `MOD-tidal`; `C2ME` and `Argon4W/AcceleratedRendering` are the names the same dated block states.
fn fold(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "chunky" => "MOD-chunky".to_string(),
        "tidal" => "MOD-tidal".to_string(),
        "stockbridge(refinedstrotage%2bcreate)" => "stockbridge".to_string(),
        "c2me" => "MOD-c2me".to_string(),
        "argon4w/acceleratedrendering" => "AcceleratedRendering".to_string(),
        _ => name.to_string(),
    }
}

/// The `code` spans in one line of a markdown row, in order.
fn backticked(text: &str) -> Vec<String> {
    text.split('`').skip(1).step_by(2).map(|span| span.trim().to_string()).collect()
}

/// A markdown table row's cells, or nothing when the line is not a row.
fn cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    Some(line.trim_matches('|').split('|').map(|cell| cell.trim().to_string()).collect())
}

/// The plain mod names the map's **dated 2026-10-03 Tekkit paragraph** states: that paragraph names no
/// `code` spans, so its references are listed — and a test asserts each one is named **in that
/// paragraph** before it is counted, so the list is read from the document rather than remembered.
const TEKKIT: [&str; 11] = [
    "IC2",
    "BuildCraft",
    "Railcraft",
    "ComputerCraft",
    "ProjectRed",
    "BluePower",
    "EE 3",
    "ForestryMC",
    "MFR",
    "LogisticsPipes",
    "ironchest",
];

/// **The map's own rows, read where they live** — the references `docs/MODS-MAP.md`'s §1–§5 name, the
/// spelling variants the map itself prints folded onto one name each, and, for each name, **the section
/// that first named it**.
///
/// This is the module's **one** parse of the map's tables: the tree's own per-section split is read off it
/// ([`MapRows::sections`]) and SC-001's set equality is measured against it, so the digits the census
/// prints cannot be a copy of the map's prose that has drifted from the map's rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapRows {
    named: BTreeSet<String>,
    first_section: BTreeMap<String, char>,
    section_5: Vec<String>,
}

impl MapRows {
    /// **The map's rows, read in place** at [`MAP_PATH`], from [`record_root`] — the record's own
    /// document, which is what the tree and the census read.
    pub fn read() -> Result<MapRows, RefusalSet> {
        MapRows::read_at(&record_root())
    }

    /// The same read over an **explicit root** — the seam a test drives an absent document through
    /// without touching the record's own, exactly as [`crate::intake::Intake`] is built over a root. A
    /// document that is not there to be read is refused by name ([`RefusalKind::SourceAbsent`]) rather
    /// than reconstructed from a copy — the footing every in-place read in this crate stands on.
    ///
    /// **The digest is deliberately not re-checked here.** The pin that proves the bytes are the pinned
    /// document is the row's own declared source, verified by the gate at admission
    /// (`source_digest_mismatch`) and by `the_declared_digests_are_the_documents_on_the_disk`. This reader
    /// answers one question only, of whatever bytes are there: *what do the rows say?* — which is what
    /// makes a row moved across sections visible in the printed split instead of hidden behind a digest.
    pub fn read_at(root: &Path) -> Result<MapRows, RefusalSet> {
        let path = root.join(MAP_PATH);
        match std::fs::read_to_string(&path) {
            Ok(map) => Ok(MapRows::parse(&map)),
            Err(error) => {
                let mut refusals = RefusalSet::new();
                refusals.refuse(
                    RefusalKind::SourceAbsent,
                    format!(
                        "the map `{}` is not there to be read in place at `{}`: {error} — the §1–§5 rows \
                         it names are what this reader parses, and a map that cannot be read is refused \
                         rather than reconstructed from a copy",
                        MAP_PATH,
                        path.display()
                    ),
                );
                Err(refusals)
            }
        }
    }

    /// **The parse itself** — the map's text in, and the references its own rows name out: §1's table,
    /// §2's group table plus its dated 2026-10-12 block plus the dated 2026-10-03 Tekkit paragraph, §3's
    /// and §4's class rows, with §5's list folding into rows already counted (§5 is **not** a second set of
    /// references). The struck-through §4 row is not a reference either: it is the URL the 2026-10-12
    /// re-key replaced.
    pub fn parse(map: &str) -> MapRows {
        let mut rows = MapRows::default();
        let mut section = ' ';
        for line in map.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                section = rest.chars().next().unwrap_or(' ');
                continue;
            }
            let Some(row) = cells(line) else {
                if section == '5' && line.contains('→') {
                    rows.section_5.push(line.split('→').next().expect("a name").trim().to_string());
                }
                continue;
            };
            match section {
                // §1: one reference per row, named in the first cell.
                '1' if row[0].starts_with('`') => {
                    rows.remember(fold(row[0].trim_matches('`').trim()), section);
                }
                // §2: the four group rows carry their mods in the second cell; the dated 2026-10-12 block
                // names its reference in the first.
                '2' if row[0].starts_with("**") => {
                    for name in backticked(&row[1]) {
                        rows.remember(fold(name.trim_end_matches('*')), section);
                    }
                }
                '2' if row[0].starts_with('`') => {
                    rows.remember(fold(row[0].trim_matches('`').trim()), section);
                }
                // §3 and §4: one class ruling per row, the references in the first cell.
                '3' | '4' if row[0].contains('`') && !row[0].starts_with("~~") => {
                    for name in backticked(&row[0]) {
                        rows.remember(fold(name.trim_end_matches('*')), section);
                    }
                }
                _ => {}
            }
        }
        // The Tekkit paragraph's plain names are §2's: it is the dated block that leaves §2's class open.
        for name in TEKKIT {
            rows.remember(fold(name), '2');
        }
        rows
    }

    /// Add a folded reference, remembering the section that **first** named it: a later row for a
    /// reference already counted does not move it, so the split counts first mentions — which is what
    /// makes a row moved across sections change the digits.
    fn remember(&mut self, name: String, section: char) {
        if self.named.insert(name.clone()) {
            self.first_section.insert(name, section);
        }
    }

    /// The references the map's own rows name.
    pub fn named(&self) -> &BTreeSet<String> {
        &self.named
    }

    /// Whether the map's own rows name a reference — folding the spelling the map prints in the place it
    /// prints it (`Tidal` in §5's list, `MOD-tidal` in §2's row).
    pub fn names(&self, reference: &str) -> bool {
        self.named.contains(&fold(reference))
    }

    /// **The map's own per-section split** — how many references each of §1–§4 **first** names, derived
    /// from where each name appears rather than restated. It is the figure the census prints beside the
    /// entry count, and the four sum to [`Self::named`]'s length because §5's list is rows already counted
    /// above it.
    pub fn sections(&self) -> [usize; 4] {
        ['1', '2', '3', '4']
            .map(|digit| self.first_section.values().filter(|section| **section == digit).count())
    }

    /// The names §5 lists — the URLs that had no disposition row. Each is a reference the rows already
    /// count, and a test holds that.
    pub fn section_5(&self) -> &[String] {
        &self.section_5
    }
}

// ---------------------------------------------------------------------------
// The disposition — the record's own four words.
// ---------------------------------------------------------------------------

/// **The record's disposition for one reference**, in the record's own four words
/// (`docs/MODS-MAP.md` §1's legend for the first three, §3/§4's *"Unresolved — recorded, not guessed"* for
/// the fourth). Closed, because a disposition the map does not give is a decision nobody made.
///
/// It is the **tree's** field and not an eighth axis (D5): an axis is a question about *a row*, and a
/// disposition is a question about *a reference* — an `open` reference has no row for an axis to judge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    /// The record maps it to a feature: the entry carries a row.
    Mapped,
    /// The record explicitly left it undecided: the entry carries **no** row and is named owed.
    Open,
    /// A licence fact forbids deriving from it: the entry carries a row whose verdict is `Refused`, and
    /// the admission refuses `licence_refused`.
    Refused,
    /// Recorded, not guessed: named owed, exactly as `open`.
    Unresolved,
}

impl Disposition {
    /// The four, in the record's own order.
    pub const ALL: [Disposition; 4] = [
        Disposition::Mapped,
        Disposition::Open,
        Disposition::Refused,
        Disposition::Unresolved,
    ];

    /// How many dispositions the record declares. Written down, and pinned to [`Disposition::ALL`] by a
    /// test — the shape `Axis`, `Licence` and `Carrier` already carry.
    pub const COUNT: usize = 4;

    /// The record's word for it.
    pub const fn name(self) -> &'static str {
        match self {
            Disposition::Mapped => "mapped",
            Disposition::Open => "open",
            Disposition::Refused => "refused",
            Disposition::Unresolved => "unresolved",
        }
    }

    /// The disposition a word names, or **nothing**: a word outside the four is not read as the nearest of
    /// them (`licence_undeclared`'s rule, one vocabulary over).
    pub fn parse(name: &str) -> Option<Self> {
        Disposition::ALL.iter().copied().find(|disposition| disposition.name() == name.trim())
    }

    /// The state this disposition is, in the gate's own vocabulary — `mapped` is the only one that is a
    /// pass, a refused one is `Refused`, and an owed one is `Inactive` (checked, and not there). A state
    /// that is not `Active` is never a pass ([`Establishment::is_established`]).
    pub const fn establishment(self) -> Establishment {
        match self {
            Disposition::Mapped => Establishment::Active,
            Disposition::Refused => Establishment::Refused,
            Disposition::Open | Disposition::Unresolved => Establishment::Inactive,
        }
    }

    /// Whether an entry with this disposition carries a row: a `mapped` one declares what it carries, and a
    /// `refused` one is refused **by name** rather than being silent — while an owed reference has no
    /// feature to declare and therefore cannot be a row.
    pub const fn carries_a_row(self) -> bool {
        matches!(self, Disposition::Mapped | Disposition::Refused)
    }

    /// Whether this disposition is **owed**: named, counted, printed, and never admitted.
    pub const fn is_owed(self) -> bool {
        matches!(self, Disposition::Open | Disposition::Unresolved)
    }
}

// ---------------------------------------------------------------------------
// The entry — one reference.
// ---------------------------------------------------------------------------

/// **One entry of the tree**: a reference of the record, the record's disposition for it, and — for a
/// `mapped` or `refused` one — the row it declares. The unit the tree counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    reference: String,
    disposition: Disposition,
    row: Option<Row>,
    note: Option<String>,
}

impl Entry {
    /// An entry, or the refusal that says why it cannot be one. **The shape is the whole rule**: a
    /// `mapped` or `refused` entry declares a row, an `open`/`unresolved` one declares none, and the two
    /// ways of getting that wrong are refused — by the closest declared name, with the word the record
    /// owes this fact named in the message.
    pub fn new(
        reference: impl Into<String>,
        disposition: Disposition,
        row: Option<Row>,
        note: Option<&str>,
    ) -> Result<Self, RefusalSet> {
        let reference = reference.into();
        match (disposition.carries_a_row(), row.is_some()) {
            (true, true) | (false, false) => Ok(Entry {
                reference,
                disposition,
                row,
                note: note.map(str::to_string),
            }),
            (true, false) => {
                let mut refusals = RefusalSet::new();
                refusals.refuse(
                    RefusalKind::ArgumentAbsent,
                    format!(
                        "the entry `{reference}` is {} and carries no row: the row is the whole \
                         declaration a mapped (or refused) reference makes, and there is no default — \
                         the closest declared name is `argument_absent`, and the word the record would \
                         give this fact is **owed** \
                         (`specs/007-u10-gameplay-tree/contracts/u10-seams.md` C1)",
                        disposition.name()
                    ),
                );
                Err(refusals)
            }
            (false, true) => {
                let mut refusals = RefusalSet::new();
                refusals.refuse(
                    RefusalKind::ArgumentMalformed,
                    format!(
                        "the entry `{reference}` is {} and carries a row: the record assigned this \
                         reference no feature, so it cannot be a row — the closest declared name is \
                         `argument_malformed`, and the word the record would give this fact is **owed** \
                         (`specs/007-u10-gameplay-tree/contracts/u10-seams.md` C1)",
                        disposition.name()
                    ),
                );
                Err(refusals)
            }
        }
    }

    /// The reference's own id, as the record writes it (`MOD-c2me`, `voxy`, `Create`, …).
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The record's disposition for this reference.
    pub const fn disposition(&self) -> Disposition {
        self.disposition
    }

    /// The row this entry declares, or `None` for an owed one.
    pub fn row(&self) -> Option<&Row> {
        self.row.as_ref()
    }

    /// The **technology** this entry declares — the feature of its row — or `None`.
    pub fn technology(&self) -> Option<&str> {
        self.row.as_ref().map(Row::feature)
    }

    /// The record's own words about this entry: what an owed one waits on, and why a refused one is
    /// refused.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Whether this entry is **owed** — named, counted and printed, and never admitted.
    pub const fn is_owed(&self) -> bool {
        self.disposition.is_owed()
    }

    /// One line per entry — the form `specs/007-u10-gameplay-tree/spec.md` US1's independent test asks
    /// for: the reference, its disposition, and (for a row) its feature, licence, carrier, source and
    /// check.
    pub fn render(&self) -> String {
        let Some(row) = &self.row else {
            return format!(
                "{} {} (owed: {})",
                self.reference,
                self.disposition.name(),
                self.note.as_deref().unwrap_or("the record names no feature")
            );
        };
        let check = match row.acceptance() {
            Some(Acceptance::Provided(source)) => source.path().to_string(),
            Some(Acceptance::Owed) => "owed".to_string(),
            None => "(none)".to_string(),
        };
        let note = match self.note.as_deref() {
            Some(note) if !note.is_empty() => format!(" ({note})"),
            _ => String::new(),
        };
        format!(
            "{} {} feature={} licence={} carrier={} source={} check={}{note}",
            self.reference,
            self.disposition.name(),
            row.feature(),
            row.licence().map(Licence::name).unwrap_or("(none)"),
            row.carrier().unwrap_or("(none)"),
            row.source().path(),
            check
        )
    }
}

// ---------------------------------------------------------------------------
// One technology, and the record's citations for it.
// ---------------------------------------------------------------------------

/// **One technology of the tree**: the row U6 admits, plus the references the record cites for it. The
/// citations are the record's own list (`docs/MODS-MATERIALS.md` §8.1's *"proposed by (deduped)"* column,
/// which is where *"one rule with multiple citations"* is visible) and they are **data**, not a decision:
/// the double-cover rule lives in them (an older mod and its Create-addon descendant on **one** row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Technology {
    row: Row,
    cited_by: &'static [&'static str],
}

impl Technology {
    /// The row — U6's own value, admitted through the one table.
    pub fn row(&self) -> &Row {
        &self.row
    }

    /// The technology's name: the row's feature.
    pub fn feature(&self) -> &str {
        self.row.feature()
    }

    /// The references the record cites for this technology.
    pub fn cited_by(&self) -> &'static [&'static str] {
        self.cited_by
    }

    /// Whether the record cites more than one reference for this technology — the double-cover rule's own
    /// shape, and what §8.2's proof is measured on.
    pub fn is_double_covered(&self) -> bool {
        self.cited_by.len() > 1
    }

    /// The references the record cites for this technology **and refuses**. A row keeps its own verdict
    /// — a verdict is per reference (`docs/MODS-MAP.md` §1) — and a refused citation is **printed**
    /// rather than hidden, so a rule read from a no-derivation reference is visible as such.
    pub fn refused_citations(&self) -> Vec<&'static str> {
        self.cited_by
            .iter()
            .copied()
            .filter(|id| {
                REFERENCES
                    .iter()
                    .any(|reference| reference.id == *id && reference.disposition == Disposition::Refused)
            })
            .collect()
    }

    /// The technology as one line: its name, the system that carries it, its check, and the record's
    /// citations for it.
    pub fn render(&self) -> String {
        let check = match self.row.acceptance() {
            Some(Acceptance::Provided(source)) => source.path().to_string(),
            Some(Acceptance::Owed) => "owed".to_string(),
            None => "(none)".to_string(),
        };
        format!(
            "{} carrier={} licence={} check={} cited_by={}{}",
            self.feature(),
            self.row.carrier().unwrap_or("(none)"),
            self.row.licence().map(Licence::name).unwrap_or("(none)"),
            check,
            self.cited_by.join("+"),
            match self.refused_citations().is_empty() {
                true => String::new(),
                false => format!(" refused_citations={}", self.refused_citations().join("+")),
            }
        )
    }
}

// ---------------------------------------------------------------------------
// The table the tree is built from.
// ---------------------------------------------------------------------------

/// One row of this module's **reference table**: a reference of `docs/MODS-MAP.md` §1–§5, in the record's
/// order, with the disposition the record gives it and — for a `mapped`/`refused` one — the technology it
/// declares.
struct Reference {
    id: &'static str,
    disposition: Disposition,
    technology: Option<&'static str>,
    note: &'static str,
}

impl Reference {
    const fn mapped(id: &'static str, technology: &'static str) -> Self {
        Reference { id, disposition: Disposition::Mapped, technology: Some(technology), note: "" }
    }

    const fn open(id: &'static str, note: &'static str) -> Self {
        Reference { id, disposition: Disposition::Open, technology: None, note }
    }

    const fn unresolved(id: &'static str, note: &'static str) -> Self {
        Reference { id, disposition: Disposition::Unresolved, technology: None, note }
    }

    const fn refused(id: &'static str, technology: &'static str, note: &'static str) -> Self {
        Reference {
            id,
            disposition: Disposition::Refused,
            technology: Some(technology),
            note,
        }
    }
}

/// One row of this module's **technology table**: the rule's name, the references the record cites for it,
/// the system that would carry it, the document its own bar lives in, and the document its source is read
/// from.
struct DeclaredTechnology {
    feature: &'static str,
    cited_by: &'static [&'static str],
    carrier: Carrier,
    check: CheckDocument,
    document: SourceDocument,
}

/// Where a row's own acceptance check is declared — the carrying system's own bar, or **owed**.
#[derive(Clone, Copy)]
enum CheckDocument {
    /// `docs/SHAPERS.md` §1's paper-per-shaper contract: the bar U3 declares for its shapers.
    Shapers,
    /// U4's own quickstart: the bar that slice declares and pasted.
    U4,
    /// The row's own bar does not exist yet. Named, never counted as met.
    Owed,
}

/// The record document a row's source is read from — `path + digest`, never bytes.
#[derive(Clone, Copy)]
enum SourceDocument {
    /// `docs/MODS-MAP.md`: the record's own column for §1/§3/§4's classes.
    Map,
    /// `docs/MODS-MATERIALS.md`: the harvest, which is where §2's rules were read.
    Harvest,
}

impl SourceDocument {
    fn source(self) -> Source {
        match self {
            SourceDocument::Map => Source::new(MAP_PATH, declared_digest(MAP_DIGEST)),
            SourceDocument::Harvest => Source::new(HARVEST_PATH, declared_digest(HARVEST_DIGEST)),
        }
    }
}

impl CheckDocument {
    fn acceptance(self) -> Acceptance {
        match self {
            CheckDocument::Shapers => {
                Acceptance::Provided(Source::new(SHAPERS_PATH, declared_digest(SHAPERS_DIGEST)))
            }
            CheckDocument::U4 => Acceptance::Provided(Source::new(
                U4_QUICKSTART_PATH,
                declared_digest(U4_QUICKSTART_DIGEST),
            )),
            CheckDocument::Owed => Acceptance::Owed,
        }
    }
}

/// **The record's references**, in the record's own order: `docs/MODS-MAP.md` §1 first — the load-bearing
/// class the first target needs — then §2, §3 and §4. A reference the map writes twice (its §5 list, or
/// the same mod named under two sections) appears **once** here, and a test holds that.
const REFERENCES: [Reference; ENTRY_COUNT] = [
    // §1 — world shape and streaming, the load-bearing class. The first entry is the class the
    // repository's first target ("a fully playable basic world") directly needs (FR-008).
    Reference::mapped("MOD-c2me", "chunk-infrastructure"),
    Reference::mapped("MOD-cubicchunks", "vertical-extent"),
    Reference::mapped("distant-horizons", "lod-horizon"),
    Reference::mapped("voxy", "lod-horizon"),
    Reference::mapped("MOD-chunky", "pre-generation"),
    Reference::mapped("DynamicTrees", "flora-placement"),
    Reference::mapped("DynamicTreesPlus", "flora-placement"),
    Reference::open("MOD-tidal", "unreviewed; candidate for `voxel-shapers::hydrology` (tides)"),
    Reference::refused(
        "BigGlobe",
        "world-shape",
        "ARR + CC-BY-NC on earlier versions, explicit no-AI clause: readable to understand a problem, \
         and nothing derived may be implemented",
    ),
    Reference::refused("lithostitched", "world-structure", "no licence declared (verified 2026-09-25)"),
    // §2 — industrial and content: the mods are the credits and all of it is content scheduled after the
    // playable world. A `*` in the record's own table means the record assigned no feature.
    Reference::mapped("Create", "crushing"),
    Reference::mapped("Create-Diesel-Generators", "distillation"),
    Reference::open("CreateExtraGauges", "the record's own `*`: it assigned this mod no feature"),
    Reference::mapped("create-factory-controller", "gauges-and-control"),
    Reference::refused(
        "Create-The_Factory_Must_Grow",
        "factory-automation",
        "explicit no-AI clause — the highest-caution reference: not cloned, not read; any use needs the \
         user's explicit sign-off first",
    ),
    Reference::mapped("extra-disks", "storage-and-logistics"),
    Reference::open("cable-tiers", "the record's own `*`: it assigned this mod no feature"),
    Reference::mapped("stockbridge", "storage-and-logistics"),
    Reference::open(
        "refined-fluid-substitution",
        "the record's own `*`: it assigned this mod no feature",
    ),
    Reference::mapped("create-storage-neo-forge", "storage-and-logistics"),
    Reference::mapped("refined-storage", "silicon-refining"),
    Reference::open("Create-Threaded-Trains", "the record's own `*`: it assigned this mod no feature"),
    Reference::mapped("createsifter", "sifting"),
    Reference::open(
        "create-more-vertical-additions",
        "the record's own `*`: it assigned this mod no feature",
    ),
    Reference::open(
        "optical",
        "the clone is empty (only `.git`): nothing was harvested, so the record assigned it no feature",
    ),
    Reference::mapped("Create-Ez-Stock-Ticker", "gauges-and-control"),
    Reference::mapped("create-metalwork", "casting"),
    Reference::mapped("Create-Mobile-Packages", "storage-and-logistics"),
    Reference::open(
        "create-new-age",
        "scope = energy (corium + radiation); the harvest recorded nothing further and §8.1 declares no \
         rule for it",
    ),
    Reference::open(
        "Create-Pantographs-and-Wires",
        "the record's own `*`: it assigned this mod no feature — its citations survive on the coking, \
         oxidation and wire-drawing rows",
    ),
    Reference::open("create-pressure-gauges", "the record's own `*`: it assigned this mod no feature"),
    Reference::open("Repackaged", "the record's own `*`: it assigned this mod no feature"),
    Reference::mapped("Rubberworks", "sapping"),
    Reference::mapped("SteamPowered", "steam-generation"),
    Reference::mapped("GregTech-Modern", "smelting"),
    Reference::mapped("Immersive-Weathering", "oxidation"),
    Reference::mapped("TinkersConstruct", "casting"),
    Reference::mapped("Step-Crafter", "assembly"),
    Reference::open(
        "BuildCraft",
        "the record's own `*`: it assigned this mod no feature — its refining citations survive on the \
         distillation row",
    ),
    Reference::mapped("effortless", "parametric-primitives"),
    // §2's Tekkit Classic set: the 1.2.5 pack joins the content class, readable for ideas, no code
    // derived.
    Reference::mapped("IC2", "crushing"),
    Reference::mapped("Railcraft", "coking"),
    Reference::mapped("ComputerCraft", "storage-and-logistics"),
    Reference::mapped("ProjectRed", "silicon-refining"),
    Reference::mapped("BluePower", "wafer-doping"),
    Reference::open(
        "EE 3",
        "minium dust/shard/stone only — magic, deferred to the Deadlock-integrated model (2026-10-03)",
    ),
    Reference::mapped("ForestryMC", "woodworking"),
    Reference::refused(
        "MFR",
        "factory-automation",
        "no licence file anywhere in the tree — read-only ideas, no derivation, on the \
         BigGlobe/lithostitched footing",
    ),
    Reference::mapped("LogisticsPipes", "storage-and-logistics"),
    Reference::mapped("ironchest", "storage-and-logistics"),
    // The two content references §2 names but its slot table never listed, given their own entries
    // (2026-10-12, user): the record's platinum earth-matter reference and the mod its circuits come
    // from. Each is filed under the **first** `docs/MODS-MATERIALS.md` §8.1 rule that cites it —
    // furnace firing / smelting — which is the smaller reading: a feature is one rule.
    Reference::mapped("TerraFirmaCraft", "smelting"),
    Reference::mapped("Calculator", "smelting"),
    // §3 — presentation, performance and appearance: the record left the class open expecting one batch
    // ruling per class, and this repo took it. Never a per-mod port.
    Reference::mapped("sodium", "instanced-batch-submission"),
    Reference::mapped("sodiumextras", "instanced-batch-submission"),
    Reference::mapped("sodiumleafculling", "instanced-batch-submission"),
    Reference::mapped("AcceleratedRendering", "instanced-batch-submission"),
    Reference::mapped("Flywheel", "instanced-batch-submission"),
    Reference::mapped("ImmediatelyFast", "instanced-batch-submission"),
    Reference::mapped("pc", "instanced-batch-submission"),
    Reference::mapped("SodiumOptionsAPI", "option-dirty-tracking"),
    Reference::mapped("lithium", "allocation-free-hot-paths"),
    Reference::mapped("FerriteCore", "allocation-free-hot-paths"),
    Reference::mapped("ModernFix", "allocation-free-hot-paths"),
    Reference::open("krypton", "network/tick budget: `N/A until multiplayer`"),
    Reference::open("Redirected", "network/tick budget: `N/A until multiplayer`"),
    Reference::open(
        "tt20",
        "performance: a TPS fixer — how ticks behave when TPS is low (the network/tick reading is \
         retracted; the record assigns it no rule)",
    ),
    Reference::mapped("MixinTraceReforged", "refusal-naming"),
    Reference::mapped("SodiumDynamicLights", "dynamic-block-light"),
    Reference::unresolved("shuttfup", "open, and R32 pointed from `shuttfup` to `Stfu`: recorded, not guessed"),
    Reference::unresolved("Stfu", "open, and R32 pointed from `shuttfup` to `Stfu`: recorded, not guessed"),
    Reference::unresolved("AcceleratedRecoiling", "has no record hit: recorded, not guessed"),
    // §4 — UI, audio, ecology and worldgen content.
    Reference::mapped("Jade", "read-the-world-overlay"),
    Reference::mapped("JustEnoughItems", "one-index"),
    Reference::mapped("searchables", "one-index"),
    Reference::mapped("MouseTweaks", "bulk-inventory-gestures"),
    Reference::mapped("pick-up-notifier", "pickup-merge"),
    Reference::mapped("reeses-sodium-options", "option-dirty-tracking"),
    Reference::open("enhancedtooltips", "rides the UI/design round: the record assigned it no feature"),
    Reference::open(
        "Minecraft-Smooth-Scrolling",
        "rides the UI/design round: the record assigned it no feature",
    ),
    Reference::open(
        "lazy-language-loader",
        "no i18n row exists yet; recorded as an owed decision",
    ),
    Reference::mapped("AmbientSounds", "scanned-local-soundscape"),
    Reference::mapped("Cool-Rain", "rain-by-material"),
    Reference::mapped("Presence-Footsteps", "footsteps-by-material"),
    Reference::mapped("matmos-ambient-sound", "spatialised-emitters"),
    Reference::mapped("sound-physics-remastered", "absorption-and-reverb"),
    Reference::open(
        "immersivethunder",
        "event audio scaled by distance — but it names a dependency this world does not have: there is no \
         lightning, no storm cell and no electrical field anywhere in `voxel-shapers`",
    ),
    Reference::open("biomemusic", "`rodio` dynamic audio for all audio — verbatim R33; unscoped"),
    Reference::open("countereds-smooth-steps", "might feed FC-10.2: the record assigned it no feature"),
    Reference::mapped("SereneSeasons", "seasonal-clock"),
    Reference::mapped("Ecologics", "biome-triple"),
    Reference::mapped("boids-reforged", "flocking"),
    Reference::mapped("Boids-Birds", "flocking"),
    Reference::mapped("DenseOres", "dense-ore-dials"),
    Reference::unresolved("curious-mobs", "ecology/food rows: recorded, not guessed"),
    Reference::unresolved("personality", "ecology/food rows: recorded, not guessed"),
    Reference::unresolved("Ratatouille", "ecology/food rows: recorded, not guessed"),
    Reference::unresolved("water-your-crops", "ecology/food rows: recorded, not guessed"),
    Reference::unresolved("SeedDelight", "ecology/food rows: recorded, not guessed"),
    Reference::unresolved("consistent-block-drops", "block drops: recorded, not guessed"),
    Reference::unresolved("no-more-knockback-latency", "combat feel: recorded, not guessed"),
    Reference::open(
        "resquake",
        "the Squake movement model — added to the game's player controller with \
         `Minecraft-CameraOverhaul` (`docs/LEDGER.md` C-150); not in the clean room, assigned no row",
    ),
    Reference::open(
        "Minecraft-CameraOverhaul",
        "camera tilting and roll for the player controller (`github.com/Mirsario/Minecraft-CameraOverhaul`), \
         added to it with `resquake` (`docs/LEDGER.md` C-150); **GPL-3.0**, and not in the clean room, so \
         the record assigns it no row (2026-10-12, user)",
    ),
    Reference::unresolved("neruina", "purpose unclear from the slug: recorded, not guessed"),
    Reference::unresolved("mercuryium-mod", "purpose unclear from the slug: recorded, not guessed"),
    Reference::open("ReplayMod", "replay/camera recording — candidate for the debug views"),
    Reference::refused(
        "archipelago",
        "world-shape",
        "the Modrinth datapack the reference points at (`modrinth.com/datapack/archipelago`): its own \
         terms declare **CC-BY-NC-ND-4.0** — attribution, non-commercial, **no derivatives** — so a \
         licence fact forbids deriving (re-keyed 2026-10-12, user; the name this row used to carry, \
         `MinecraftFabricArchipelagoMod`, is the Archipelago randomiser client — a different artifact)",
    ),
];

/// How many references the tree declares. Written down and pinned to [`REFERENCES`] by a test.
pub const ENTRY_COUNT: usize = 106;

/// **The technologies the record's references declare**, in the order the entries first name them — §1's
/// class first, §2's rules next, then §3's and §4's. One row per technology, and a test holds that no two
/// rows share a feature.
const TECHNOLOGIES: [DeclaredTechnology; TECHNOLOGY_COUNT] = [
    // §1 — world shape and streaming. The horizon rows carry U4's own bar; the ecology one carries U3's.
    DeclaredTechnology {
        feature: "chunk-infrastructure",
        cited_by: &["MOD-c2me"],
        carrier: Carrier::U4,
        check: CheckDocument::U4,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "vertical-extent",
        cited_by: &["MOD-cubicchunks"],
        carrier: Carrier::U4,
        check: CheckDocument::U4,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "lod-horizon",
        cited_by: &["distant-horizons", "voxy"],
        carrier: Carrier::U4,
        check: CheckDocument::U4,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "pre-generation",
        cited_by: &["MOD-chunky"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "flora-placement",
        cited_by: &["DynamicTrees", "DynamicTreesPlus"],
        carrier: Carrier::U3,
        check: CheckDocument::Shapers,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "world-shape",
        cited_by: &["BigGlobe", "archipelago"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "world-structure",
        cited_by: &["lithostitched"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    // §2 — the harvest's deduped rule library. Every row's own bar is **owed**: the record's own
    // `docs/MODS-MATERIALS.md` §8.4 lists the values that still want their own real-world source, so none
    // of these rows has a bar this slice may point at.
    DeclaredTechnology {
        feature: "smelting",
        cited_by: &["TerraFirmaCraft", "Create", "Calculator", "GregTech-Modern", "refined-storage"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "coking",
        cited_by: &["Railcraft", "Create-Pantographs-and-Wires"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "blast-conversion",
        cited_by: &["Railcraft", "IC2", "TerraFirmaCraft"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "steam-generation",
        cited_by: &["SteamPowered", "Railcraft", "MFR", "GregTech-Modern"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "crushing",
        cited_by: &["Create", "IC2", "Railcraft", "create-metalwork", "GregTech-Modern"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "sifting",
        cited_by: &["createsifter"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "distillation",
        cited_by: &["BuildCraft", "Create-Diesel-Generators", "GregTech-Modern", "ForestryMC"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "casting",
        cited_by: &["TerraFirmaCraft", "create-metalwork", "Create-Diesel-Generators", "TinkersConstruct"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "sapping",
        cited_by: &["Rubberworks", "IC2"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "oxidation",
        cited_by: &["Immersive-Weathering", "Create", "Create-Pantographs-and-Wires"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "woodworking",
        cited_by: &["ForestryMC", "Create"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "silicon-refining",
        cited_by: &["refined-storage", "BluePower", "ProjectRed"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "wafer-doping",
        cited_by: &["BluePower"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "assembly",
        cited_by: &[
            "Create",
            "refined-storage",
            "Calculator",
            "Create-Pantographs-and-Wires",
            "Step-Crafter",
        ],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "gauges-and-control",
        cited_by: &[
            "Create",
            "create-factory-controller",
            "Create-Ez-Stock-Ticker",
            "Repackaged",
            "CreateExtraGauges",
            "create-pressure-gauges",
        ],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "storage-and-logistics",
        cited_by: &[
            "refined-storage",
            "ironchest",
            "MFR",
            "stockbridge",
            "LogisticsPipes",
            "ComputerCraft",
            "extra-disks",
            "create-storage-neo-forge",
            "Create-Mobile-Packages",
        ],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Harvest,
    },
    DeclaredTechnology {
        feature: "factory-automation",
        cited_by: &["MFR", "Create-The_Factory_Must_Grow"],
        carrier: Carrier::U3,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "parametric-primitives",
        cited_by: &["effortless"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    // §3 — one architecture per class, never a per-mod port.
    DeclaredTechnology {
        feature: "instanced-batch-submission",
        cited_by: &[
            "sodium",
            "sodiumextras",
            "sodiumleafculling",
            "AcceleratedRendering",
            "Flywheel",
            "ImmediatelyFast",
            "pc",
        ],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "allocation-free-hot-paths",
        cited_by: &["lithium", "FerriteCore", "ModernFix", "MOD-c2me"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "option-dirty-tracking",
        cited_by: &["reeses-sodium-options", "SodiumOptionsAPI"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "refusal-naming",
        cited_by: &["MixinTraceReforged"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "dynamic-block-light",
        cited_by: &["SodiumDynamicLights"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    // §4 — UI, audio, ecology and worldgen content.
    DeclaredTechnology {
        feature: "read-the-world-overlay",
        cited_by: &["Jade"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "one-index",
        cited_by: &["JustEnoughItems", "searchables"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "bulk-inventory-gestures",
        cited_by: &["MouseTweaks"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "pickup-merge",
        cited_by: &["pick-up-notifier"],
        carrier: Carrier::U8,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "scanned-local-soundscape",
        cited_by: &["AmbientSounds"],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "rain-by-material",
        cited_by: &["Cool-Rain"],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "footsteps-by-material",
        cited_by: &["Presence-Footsteps"],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "spatialised-emitters",
        cited_by: &["matmos-ambient-sound"],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "absorption-and-reverb",
        cited_by: &["sound-physics-remastered"],
        carrier: Carrier::U5,
        check: CheckDocument::Owed,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "seasonal-clock",
        cited_by: &["SereneSeasons"],
        carrier: Carrier::U3,
        check: CheckDocument::Shapers,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "biome-triple",
        cited_by: &["Ecologics"],
        carrier: Carrier::U3,
        check: CheckDocument::Shapers,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "flocking",
        cited_by: &["boids-reforged", "Boids-Birds"],
        carrier: Carrier::U3,
        check: CheckDocument::Shapers,
        document: SourceDocument::Map,
    },
    DeclaredTechnology {
        feature: "dense-ore-dials",
        cited_by: &["DenseOres"],
        carrier: Carrier::U3,
        check: CheckDocument::Shapers,
        document: SourceDocument::Map,
    },
];

/// How many technologies the tree declares. Written down and pinned to [`TECHNOLOGIES`] by a test.
pub const TECHNOLOGY_COUNT: usize = 43;

/// **What the record has not decided**, named where a caller and the binary both read it: the recipe
/// document's row-kind key and version, the duplicate-row refusal's name, a verb that serves the whole
/// tree, and the per-row family assignment. Each is printed as `owed` and each is written in
/// `specs/007-u10-gameplay-tree/` — never decided here (FR-014, SC-006).
pub const OWED: [&str; 4] = [
    "the recipe document's row-kind key and version (`docs/LEDGER.md` R-64.1 closes the format, and \
     `packages/alpha-shell.shspec` declares no gameplay row kind)",
    "the name of the refusal for a second row on a descendant technology (`docs/MODS-MATERIALS.md` §8.2 \
     states the rule and no reference states the word)",
    "a verb that serves the whole tree (U6's table is closed and bijective; the rows cross over `admit` \
     today, the list does not)",
    "the per-row family assignment and the `SUBSTANCES` table shape (`docs/MODS-MATERIALS.md` §5, §8.3)",
];

/// **The map's own reference count** — how many references its §1–§5 rows name, and the figure SC-001
/// compares the tree's entry count against. The header states it (`**The map's own reference count: 106
/// references**`), the arithmetic behind it is written out there and in §23 of the plan, and it is
/// pinned to [`ENTRY_COUNT`] by a test **and** re-measured from the document in place by
/// `the_maps_own_reference_count_is_the_trees_entry_count`, which also re-derives the per-section split
/// the header states — so prose that drifts from the rows fails
/// by name rather than being believed.
pub const MAP_NAMED_REFERENCES: usize = 106;

/// The count of URLs in the file this map was written from — `references/shploingus_dirty/references.md`,
/// in the sibling `shploingus-game` tree. **The "91" this map's header has always quoted is that file's
/// count**, and it is still correct for it: it is *not* a count of this map's rows (106) and not a count
/// of this repo's copy (97). The header names the three, and `U10`'s census prints them un-averaged.
pub const MAP_STATED_URLS: usize = 91;

/// The count of URLs in **this repo's** `docs/references.md` — a different document: the dirty file minus
/// three (`C2ME`, `CubicChunks`, and the replaced `MinecraftFabricArchipelagoMod`) and plus nine (six of
/// which this map gives rows).
pub const MAP_COPY_URLS: usize = 97;

/// **The §2 rows that carry the record's own `*`** — nine, corrected 2026-10-12 from an unsupported 24
/// (the map's §2 legend states the nine marks and names the three further `open` entries that carry none:
/// `optical`, `create-new-age` and `EE 3`).
pub const MAP_SECTION_2_OPEN: usize = 9;

/// The record's header count of `open` rows in the prior disposition file.
pub const MAP_HEADER_OPEN: usize = 43;

// ---------------------------------------------------------------------------
// The census of dispositions.
// ---------------------------------------------------------------------------

/// **The disposition census**: how many entries the tree holds and how many of each disposition — the
/// count a reader compares against the record's own figures, so an `open` entry can never be read as a
/// `mapped` one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dispositions {
    entries: usize,
    mapped: usize,
    open: usize,
    refused: usize,
    unresolved: usize,
}

impl Dispositions {
    /// How many entries the tree declares.
    pub const fn entries(&self) -> usize {
        self.entries
    }

    /// How many of them the record maps.
    pub const fn mapped(&self) -> usize {
        self.mapped
    }

    /// How many the record left undecided.
    pub const fn open(&self) -> usize {
        self.open
    }

    /// How many a licence fact forbids deriving from.
    pub const fn refused(&self) -> usize {
        self.refused
    }

    /// How many the record recorded rather than guessed.
    pub const fn unresolved(&self) -> usize {
        self.unresolved
    }

    /// How many entries carry a given disposition.
    pub const fn count(&self, disposition: Disposition) -> usize {
        match disposition {
            Disposition::Mapped => self.mapped,
            Disposition::Open => self.open,
            Disposition::Refused => self.refused,
            Disposition::Unresolved => self.unresolved,
        }
    }

    /// Whether the four counts are the entry count — the arithmetic a census exists to hold.
    pub const fn sums_to_entries(&self) -> bool {
        self.mapped + self.open + self.refused + self.unresolved == self.entries
    }

    /// The census as one line, with the record's own figures beside the derived ones — and with the
    /// record's own **reference** count (`{}` §1–§5) set against the entry count above, split as the map's
    /// own rows split it (`sections`), so SC-001 is read off the line rather than inferred from it.
    ///
    /// **The split is passed in, never written down here.** It is derived from the map's own rows at print
    /// time ([`MapRows::sections`], carried by the tree), so no literal on this line can disagree with the
    /// rows it is printed beside: a row moved across sections moves these digits. The two file-level counts
    /// are a different question and are printed as such: the dirty source's 91 URL lines and this repo's
    /// copy's 97 are counts of *other documents*, never of this map's rows.
    pub fn render(&self, sections: [usize; 4]) -> String {
        format!(
            "entries={} mapped={} open={} refused={} unresolved={} (mapped+open+refused+unresolved = \
             {}) — the record's own figures, un-averaged: {} references named in `{}` §1–§5 ({} §1 + \
             {} §2 + {} §3 + {} §4) = the entry count above; the file the map was written from \
             (`references/shploingus_dirty/references.md`) {} URLs; this repo's copy (`docs/references.md`) \
             {} URLs; {} §2 rows carry the record's own `*`; {} `open` rows in the prior disposition file",
            self.entries,
            self.mapped,
            self.open,
            self.refused,
            self.unresolved,
            self.mapped + self.open + self.refused + self.unresolved,
            MAP_NAMED_REFERENCES,
            MAP_PATH,
            sections[0],
            sections[1],
            sections[2],
            sections[3],
            MAP_STATED_URLS,
            MAP_COPY_URLS,
            MAP_SECTION_2_OPEN,
            MAP_HEADER_OPEN
        )
    }
}

// ---------------------------------------------------------------------------
// The tree.
// ---------------------------------------------------------------------------

/// **The tree**: the record's entries in the record's order, and one row per technology the record's
/// references declare — with the record's own **per-section split** read from the map's rows where they
/// live, so the census prints the map's own partition rather than a copy of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    entries: Vec<Entry>,
    rows: Vec<Technology>,
    sections: [usize; 4],
}

impl Tree {
    /// **The tree the record implies** — built from [`REFERENCES`] and [`TECHNOLOGIES`], one entry per
    /// reference in the record's order and one row per technology. Returns the refusals when this build's
    /// own table is not a tree (a mapped entry with no row, an owed one with a row, two rows for one
    /// technology, a feature no row declares): a table that cannot be a tree is refused by name rather
    /// than halved.
    ///
    /// The record's own document is read **in place** here too, for the one figure that is the map's own:
    /// its per-section split ([`MapRows::read`]), which the census prints beside the entry count. A map
    /// that is not there refuses the tree by name ([`RefusalKind::SourceAbsent`]) rather than leaving that
    /// figure blank.
    pub fn of_record() -> Result<Tree, RefusalSet> {
        let sections = MapRows::read()?.sections();
        let mut refusals = RefusalSet::new();

        // The rows first: one per technology, and one only. A technology's licence verdict is read from
        // the entries that declare it — a rule declared by a refused reference is refused — because the
        // verdict is **per reference** (`docs/MODS-MAP.md` §1) and nothing derived may be implemented from
        // a refused reference.
        let mut rows: Vec<Technology> = Vec::with_capacity(TECHNOLOGY_COUNT);
        for declared in &TECHNOLOGIES {
            if rows.iter().any(|technology| technology.feature() == declared.feature) {
                refusals.refuse(
                    RefusalKind::DuplicateOutput,
                    format!(
                        "the table declares `{}` twice: a technology is one row, and the record's double-\
                         cover rule keeps every citation on it rather than growing a second \
                         (`docs/MODS-MATERIALS.md` §8.2)",
                        declared.feature
                    ),
                );
                continue;
            }
            let declaring = REFERENCES
                .iter()
                .filter(|reference| reference.technology == Some(declared.feature));
            let refused = declaring
                .clone()
                .any(|reference| reference.disposition == Disposition::Refused);
            let licence = match refused {
                true => Licence::Refused,
                false => Licence::Permitted,
            };
            if declared.cited_by.is_empty() {
                refusals.refuse(
                    RefusalKind::NoProvenance,
                    format!(
                        "no reference is cited for the technology `{}`: a rule nobody proposes is a bar \
                         nobody asked for, and the row would carry no provenance",
                        declared.feature
                    ),
                );
                continue;
            }
            for reference in declaring {
                if !declared.cited_by.contains(&reference.id) {
                    refusals.refuse(
                        RefusalKind::NoProvenance,
                        format!(
                            "`{}` declares the technology `{}` and the row does not cite it: the \
                             record's own list of who proposes a rule is what the row carries",
                            reference.id, declared.feature
                        ),
                    );
                }
            }
            rows.push(Technology {
                row: Row::new(
                    declared.feature,
                    declared.document.source(),
                    Some(licence),
                    Some(declared.carrier.name()),
                    Some(declared.check.acceptance()),
                    Claim::ChecksumOnly,
                    Requirements::nothing(),
                ),
                cited_by: declared.cited_by,
            });
        }

        // Then the entries, in the record's order — each one's row is the row its technology declares.
        let mut tree = Tree { entries: Vec::with_capacity(ENTRY_COUNT), rows, sections };
        for reference in &REFERENCES {
            if tree.entries.iter().any(|entry| entry.reference() == reference.id) {
                refusals.refuse(
                    RefusalKind::ArgumentMalformed,
                    format!(
                        "the reference `{}` appears twice: one entry per reference, or the census counts \
                         one of them twice",
                        reference.id
                    ),
                );
                continue;
            }
            let row = reference.technology.and_then(|feature| {
                tree.rows.iter().find(|technology| technology.feature() == feature).map(|technology| {
                    technology.row.clone()
                })
            });
            let note = match reference.note.is_empty() {
                true => None,
                false => Some(reference.note),
            };
            match Entry::new(reference.id, reference.disposition, row, note) {
                Ok(entry) => {
                    if let Err(shape) = tree.declare(entry) {
                        refusals.extend(shape);
                    }
                }
                Err(shape) => refusals.extend(shape),
            }
        }

        refusals.into_result(tree)
    }

    /// The entries, in the record's order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The rows: one per technology.
    pub fn rows(&self) -> &[Technology] {
        &self.rows
    }

    /// **The record's own per-section split** — how many references each of §1–§4 first names, read from
    /// [`MAP_PATH`]'s rows **in place** at [`Tree::of_record`]. The census prints these digits instead of a
    /// literal, so the line cannot restate a figure the map's own rows do not support.
    pub fn sections(&self) -> [usize; 4] {
        self.sections
    }

    /// The record's first entry — the load-bearing class the repository's first target needs, which is
    /// why the tree is ordered rather than sorted (FR-008).
    pub fn first(&self) -> Option<&Entry> {
        self.entries.first()
    }

    /// The **census of dispositions**: the four counts and the entry count, with the record's own figures
    /// beside them.
    pub fn declared(&self) -> Dispositions {
        let mut census =
            Dispositions { entries: self.entries.len(), ..Dispositions::default() };
        for entry in &self.entries {
            match entry.disposition {
                Disposition::Mapped => census.mapped += 1,
                Disposition::Open => census.open += 1,
                Disposition::Refused => census.refused += 1,
                Disposition::Unresolved => census.unresolved += 1,
            }
        }
        census
    }

    /// The technology a name names, or nothing.
    pub fn technology(&self, feature: &str) -> Option<&Technology> {
        self.rows.iter().find(|technology| technology.feature() == feature)
    }

    /// The rows that carry more than one citation — the record's own double-cover proof, measured.
    pub fn double_covered(&self) -> Vec<&Technology> {
        self.rows.iter().filter(|technology| technology.is_double_covered()).collect()
    }

    /// The rows whose own acceptance check is **owed**: named, printed, and never counted as met.
    pub fn rows_owed(&self) -> Vec<&Technology> {
        self.rows
            .iter()
            .filter(|technology| matches!(technology.row.acceptance(), Some(Acceptance::Owed)))
            .collect()
    }

    /// **Add one entry.** A `mapped`/`refused` entry whose technology the tree already holds is **one
    /// citation on the existing row**, not a second row: the entry is added and the row count does not
    /// move. An entry that brings a **different** row for a technology the tree already holds is refused —
    /// the record's double-cover rule, by the closest declared name, with the word the record has not
    /// given printed as the gap (`docs/MODS-MATERIALS.md` §8.2).
    pub fn declare(&mut self, entry: Entry) -> Result<(), RefusalSet> {
        if let Some(row) = entry.row() {
            if let Some(held) = self.technology(row.feature()) {
                if held.row() != row {
                    let mut refusals = RefusalSet::new();
                    refusals.refuse(
                        RefusalKind::DuplicateOutput,
                        format!(
                            "the entry `{}` declares a second row for the technology `{}`, which the tree \
                             already holds (cited by {}): the record states the rule — one rule with every \
                             citation on it, `docs/MODS-MATERIALS.md` §8.2, *never grow a second row for a \
                             descendant* — and names no word for the refusal, so the closest declared name \
                             is `duplicate_output` and the **exact name is owed**",
                            entry.reference(),
                            row.feature(),
                            held.cited_by().join("+")
                        ),
                    );
                    return Err(refusals);
                }
                self.entries.push(entry);
                return Ok(());
            }
            self.rows.push(Technology { row: row.clone(), cited_by: &[] });
        }
        self.entries.push(entry);
        Ok(())
    }

    /// What the record has not decided, named for whoever asks (FR-014, SC-006).
    pub const fn owed() -> [&'static str; 4] {
        OWED
    }

    /// The tree as printed text: one line per entry, one per row, the census of dispositions and the owed
    /// line. Nothing is summarised away — an owed reference is **named**, never silently absent.
    pub fn render(&self) -> String {
        let mut out = format!(
            "tree: {} entries (one per reference, in `{}`'s own order) and {} rows (one per technology)\n\
             census: {}\n",
            self.entries.len(),
            MAP_PATH,
            self.rows.len(),
            self.declared().render(self.sections)
        );
        for entry in &self.entries {
            out.push_str(&format!("entry: {}\n", entry.render()));
        }
        for technology in &self.rows {
            out.push_str(&format!("row: {}\n", technology.render()));
        }
        for part in OWED {
            out.push_str(&format!("owed: {part}\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Tree {
        Tree::of_record().expect("the record's own table is a tree")
    }

    #[test]
    fn the_disposition_vocabulary_is_the_records_own_four() {
        assert_eq!(Disposition::ALL.len(), Disposition::COUNT);
        for (index, disposition) in Disposition::ALL.iter().enumerate() {
            assert!(
                Disposition::ALL[..index].iter().all(|other| other != disposition),
                "{} is declared twice",
                disposition.name()
            );
            assert_eq!(Disposition::parse(disposition.name()), Some(*disposition));
            assert_eq!(Disposition::parse(&format!(" {} ", disposition.name())), Some(*disposition));
            assert!(
                ["mapped", "open", "refused", "unresolved"].contains(&disposition.name()),
                "{} is not one of the record's own words",
                disposition.name()
            );
        }
        // A word outside the four is **nothing**, not the nearest of them.
        for word in ["nearly-mapped", "MAPPED", "closed", ""] {
            assert!(Disposition::parse(word).is_none(), "{word} is not a disposition");
        }
        // The states: only `mapped` is a pass, and only `mapped`/`refused` carry a row.
        for disposition in Disposition::ALL {
            let established = disposition.establishment().is_established();
            assert_eq!(established, disposition == Disposition::Mapped);
            assert_eq!(disposition.carries_a_row(), !disposition.is_owed());
            assert_eq!(disposition.is_owed(), matches!(disposition, Disposition::Open | Disposition::Unresolved));
        }
    }

    #[test]
    fn the_tree_declares_one_entry_per_reference_in_the_records_order() {
        let tree = tree();
        assert_eq!(tree.entries().len(), ENTRY_COUNT, "the count is written down and pinned");
        for (index, entry) in tree.entries().iter().enumerate() {
            assert!(
                tree.entries()[..index].iter().all(|other| other.reference() != entry.reference()),
                "{} appears twice",
                entry.reference()
            );
        }
        // §1 first: the head is the load-bearing class the first target needs, and the refusals and the
        // owed references come after it rather than ahead of it.
        let head = tree.first().expect("the tree is not empty");
        assert_eq!(head.reference(), "MOD-c2me");
        assert_eq!(head.disposition(), Disposition::Mapped);
        assert_eq!(head.technology(), Some("chunk-infrastructure"));
        assert!(
            !head.is_owed(),
            "the head is a mapped row, not an owed reference"
        );
    }

    #[test]
    fn every_entry_names_its_document_its_verdict_and_the_system_that_carries_it() {
        let tree = tree();
        for entry in tree.entries() {
            match entry.row() {
                Some(row) => {
                    assert!(!row.feature().is_empty(), "{} names no feature", entry.reference());
                    assert!(
                        [MAP_PATH, HARVEST_PATH].contains(&row.source().path()),
                        "{} cites {} rather than the record's own document",
                        entry.reference(),
                        row.source().path()
                    );
                    assert!(
                        row.licence().is_some(),
                        "{} relies on no licence verdict",
                        entry.reference()
                    );
                    let carrier = row.carrier().expect("a row names the system that carries it");
                    assert!(Carrier::parse(carrier).is_some(), "{carrier} is not a declared carrier");
                    match row.acceptance() {
                        Some(Acceptance::Provided(source)) => assert!(
                            [SHAPERS_PATH, U4_QUICKSTART_PATH].contains(&source.path()),
                            "{} cites a check the carrying system does not declare: {}",
                            entry.reference(),
                            source.path()
                        ),
                        Some(Acceptance::Owed) => {}
                        None => panic!("{} declares no acceptance check at all", entry.reference()),
                    }
                }
                None => assert!(
                    entry.is_owed(),
                    "{} carries no row and is not owed",
                    entry.reference()
                ),
            }
            // A refused or owed entry says, in the record's own words, why.
            if entry.disposition() != Disposition::Mapped {
                assert!(
                    entry.note().is_some_and(|note| !note.is_empty()),
                    "{} is {} and the record's own words are not carried",
                    entry.reference(),
                    entry.disposition().name()
                );
            }
        }
    }

    #[test]
    fn a_mapped_entry_without_a_row_and_an_owed_entry_with_a_row_are_refused_by_name() {
        let row = tree()
            .technology("chunk-infrastructure")
            .expect("the head's technology is declared")
            .row()
            .clone();

        // A mapped entry with no row: a declaration is missing, and there is no default.
        let absent = Entry::new("a-mapped-reference", Disposition::Mapped, None, None)
            .expect_err("a mapped entry carries a row");
        assert_eq!(absent.first_named().expect("one refusal").kind(), RefusalKind::ArgumentAbsent);
        assert!(
            absent.to_string().contains("argument_absent") && absent.to_string().contains("owed"),
            "the closest declared name and the owed word are both in the message: {absent}"
        );

        // An owed entry carrying a row: a value present that is not of its declared kind.
        for disposition in [Disposition::Open, Disposition::Unresolved] {
            let malformed = Entry::new("an-owed-reference", disposition, Some(row.clone()), Some("owed"))
                .expect_err("an owed reference cannot be a row");
            assert_eq!(
                malformed.first_named().expect("one refusal").kind(),
                RefusalKind::ArgumentMalformed
            );
            assert!(malformed.to_string().contains("owed"), "{malformed}");
        }

        // And the two shapes that are entries.
        assert!(Entry::new("m", Disposition::Mapped, Some(row), None).is_ok());
        assert!(Entry::new("o", Disposition::Open, None, Some("owed")).is_ok());
    }

    #[test]
    fn one_row_per_technology_and_every_citation_survives_on_it() {
        let tree = tree();
        assert_eq!(tree.rows().len(), TECHNOLOGY_COUNT);
        for (index, technology) in tree.rows().iter().enumerate() {
            assert!(
                tree.rows()[..index].iter().all(|other| other.feature() != technology.feature()),
                "{} has two rows",
                technology.feature()
            );
            assert!(
                !technology.cited_by().is_empty(),
                "{} is cited by nobody",
                technology.feature()
            );
        }

        // Every entry's technology is exactly one row, and every declaring entry is in that row's
        // citations — so the row and the entry cannot drift into two declarations.
        for entry in tree.entries() {
            let Some(technology) = entry.technology() else {
                continue;
            };
            let row = tree
                .technology(technology)
                .unwrap_or_else(|| panic!("{technology} is no row of the tree"));
            assert!(
                row.cited_by().contains(&entry.reference()),
                "{} declares {} and the row does not cite it",
                entry.reference(),
                technology
            );
        }

        // The double cover: an older mod and its Create-addon descendant on **one** row.
        let coking = tree.technology("coking").expect("coking is a row");
        assert_eq!(coking.cited_by(), &["Railcraft", "Create-Pantographs-and-Wires"]);
        assert_eq!(tree.rows().iter().filter(|t| t.feature() == "coking").count(), 1);
        assert!(
            tree.double_covered().len() >= 2,
            "the record's own double-cover proof is measured, not asserted"
        );

        // And a **second** row for a technology the tree already holds is refused, by the closest
        // declared name, with the owed word in the message.
        let mut grown = tree.clone();
        let row = grown
            .technology("coking")
            .expect("coking is a row")
            .row()
            .clone();
        // The same technology, declaring its **own** row (a different bar) rather than the one the tree
        // holds: exactly what "never grow a second row for a descendant" forbids.
        let other = Row::new(
            row.feature(),
            row.source().clone(),
            Some(Licence::Permitted),
            Some(Carrier::U3.name()),
            Some(Acceptance::Provided(Source::new(
                SHAPERS_PATH,
                declared_digest(SHAPERS_DIGEST),
            ))),
            Claim::ChecksumOnly,
            Requirements::nothing(),
        );
        assert_ne!(other, row, "the descendant declares a different row for one technology");
        let descendant = Entry::new("a-descendant", Disposition::Mapped, Some(other), None)
            .expect("a mapped entry with a row");
        let refused = grown
            .declare(descendant)
            .expect_err("a second row for a held technology is refused");
        assert_eq!(
            refused.first_named().expect("one refusal").kind(),
            RefusalKind::DuplicateOutput
        );
        let message = refused.to_string();
        assert!(message.contains("duplicate_output"), "{message}");
        assert!(message.contains("owed"), "the owed word is printed: {message}");
        assert!(message.contains("§8.2"), "the rule's own document is named: {message}");
        assert_eq!(
            grown.rows().iter().filter(|t| t.feature() == "coking").count(),
            1,
            "and the refusal grew no second row"
        );
    }

    #[test]
    fn the_documents_the_rows_cite_are_the_ones_on_the_disk() {
        let root = record_root();
        for (path, hex) in [
            (MAP_PATH, MAP_DIGEST),
            (HARVEST_PATH, HARVEST_DIGEST),
            (SHAPERS_PATH, SHAPERS_DIGEST),
            (U4_QUICKSTART_PATH, U4_QUICKSTART_DIGEST),
        ] {
            let bytes = std::fs::read(root.join(path))
                .unwrap_or_else(|error| panic!("{path} is the record's own document: {error}"));
            assert_eq!(
                Digest::of(&bytes),
                declared_digest(hex),
                "{path} is no longer the document the rows declare: the tree must refuse it by name"
            );
        }
    }

    #[test]
    fn a_map_that_is_not_there_is_refused_by_name() {
        let root = std::env::temp_dir().join("shploingus-u10-map-absent");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root with no map in it");
        // The root holds no `docs/MODS-MAP.md`: the read refuses **by name**, and says where it looked,
        // rather than deriving a split from nothing.
        match MapRows::read_at(&root) {
            Ok(rows) => panic!("an absent map was read: {} rows", rows.named().len()),
            Err(refusals) => {
                assert!(refusals.contains(RefusalKind::SourceAbsent), "{refusals}");
                let detail = refusals.first_named().map(|refusal| refusal.detail().to_string()).unwrap_or_default();
                assert!(detail.contains(MAP_PATH), "the refusal must name the reference: {detail}");
                assert!(detail.contains("not there to be read"), "{detail}");
            }
        }
        let _ = std::fs::remove_dir_all(&root);
        // And the record's own map *is* there: the same read, over the record's root, derives the split.
        let rows = MapRows::read().expect("the record's own map is there to be read");
        assert_eq!(rows.sections().iter().sum::<usize>(), MAP_NAMED_REFERENCES);
    }

    #[test]
    fn the_census_prints_the_maps_own_split_rather_than_a_literal() {
        let tree = tree();
        let census = tree.declared();
        // The digits on the line are the **argument's**: a census handed a different split prints that
        // one, so the line cannot be a copy of the map's prose that has drifted from the map's rows.
        let other = census.render([7, 8, 9, 10]);
        assert!(other.contains("(7 §1 + 8 §2 + 9 §3 + 10 §4)"), "{other}");
        // And the split the tree carries is the map's own, read from the document **in place** — the four
        // are the record's own reference count, because §5 lists rows already counted above it.
        let sections = tree.sections();
        assert_eq!(sections, [10, 42, 19, 35], "the map's own per-section split");
        assert_eq!(
            sections.iter().sum::<usize>(),
            MAP_NAMED_REFERENCES,
            "§1–§4 are §5's rows too: the split sums to the reference count"
        );
        let rendered = census.render(sections);
        assert!(rendered.contains("(10 §1 + 42 §2 + 19 §3 + 35 §4)"), "{rendered}");
    }

    #[test]
    fn the_census_counts_every_disposition_and_sums_to_the_entry_count() {
        let tree = tree();
        let census = tree.declared();
        assert_eq!(census.entries(), tree.entries().len());
        assert!(census.sums_to_entries(), "the four counts are the entry count: {}", census.render(tree.sections()));
        for disposition in Disposition::ALL {
            let counted = tree
                .entries()
                .iter()
                .filter(|entry| entry.disposition() == disposition)
                .count();
            assert_eq!(census.count(disposition), counted, "{}", disposition.name());
        }
        // The record's own figures are printed beside the derived ones, un-averaged — and the record's own
        // **reference** count is the entry count, which is SC-001 stated as a value rather than as prose.
        assert_eq!(
            MAP_NAMED_REFERENCES, ENTRY_COUNT,
            "the map names one reference per entry: its own count is written down and pinned"
        );
        let rendered = census.render(tree.sections());
        for figure in [
            MAP_NAMED_REFERENCES.to_string(),
            MAP_STATED_URLS.to_string(),
            MAP_COPY_URLS.to_string(),
            MAP_SECTION_2_OPEN.to_string(),
            MAP_HEADER_OPEN.to_string(),
        ] {
            assert!(rendered.contains(&figure), "the record's own {figure} is not printed");
        }
    }

    #[test]
    fn the_head_is_a_world_shape_row_whose_check_is_the_carrying_systems_own() {
        let tree = tree();
        let head = tree.first().expect("the tree is not empty");
        let row = head.row().expect("the head declares a row");
        assert_eq!(row.carrier(), Some(Carrier::U4.name()));
        assert_eq!(
            row.acceptance(),
            Some(&Acceptance::Provided(Source::new(
                U4_QUICKSTART_PATH,
                declared_digest(U4_QUICKSTART_DIGEST)
            ))),
            "the head's check is the bar U4 already declares, not one invented for the tree"
        );
        // And the shapers' own bar is where U3's shaper rows say it is.
        let flora = tree.technology("flora-placement").expect("flora placement is a row");
        assert_eq!(flora.row().carrier(), Some(Carrier::U3.name()));
        assert_eq!(
            flora.row().acceptance(),
            Some(&Acceptance::Provided(Source::new(
                SHAPERS_PATH,
                declared_digest(SHAPERS_DIGEST)
            )))
        );
    }

    #[test]
    fn the_owed_parts_are_named_and_the_render_prints_them() {
        let tree = tree();
        assert_eq!(Tree::owed().len(), 4);
        let rendered = tree.render();
        for part in Tree::owed() {
            assert!(rendered.contains(part), "the owed line omits: {part}");
        }
        assert!(rendered.contains("census:"), "the census is printed");
        assert!(
            rendered.lines().filter(|line| line.starts_with("entry: ")).count() == tree.entries().len(),
            "one line per entry"
        );
        assert!(
            rendered.lines().filter(|line| line.starts_with("row: ")).count() == tree.rows().len(),
            "one line per row"
        );
    }
}
