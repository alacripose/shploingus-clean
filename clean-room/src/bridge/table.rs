use super::*;
// ---------------------------------------------------------------------------
// The declared argument kinds.
// ---------------------------------------------------------------------------

/// The **closed** set of kinds an argument may be declared as. A kind knows only whether a value is
/// *shaped* like it; whether the value is *in the world's domain* (a level on the ladder, a field a
/// stage declares) is the owner's decision and happens at dispatch, because that is where the
/// refusal can name the owner's own vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArgKind {
    /// Three comma-separated signed 32-bit integers: a chunk.
    Chunk,
    /// A non-negative integer that fits a `u8`: a render level. *On the ladder* is `Level::new`'s.
    Level,
    /// A declared field's name.
    Field,
    /// Three comma-separated signed 32-bit integers: a voxel position.
    Voxel,
    /// A positive integer: a patch side.
    Side,
    /// **A document consumed in place by reference**: a relative path, and no bytes. The path is
    /// resolved against the gate's reference root, and it carries none of the characters the request
    /// or the receipt's framing uses, because a reference that cannot ride the wire is refused rather
    /// than reshaped (`specs/005-u6-record-api/contracts/u6-seams.md` C1–C4; the plan's Lane B rule:
    /// `path + digest`, never bytes).
    Path,
    /// A 64-character lower-case hexadecimal content identity — the one reader being
    /// [`Digest::from_hex`], so the wire and the save format refuse the same encodings.
    Digest,
    /// A single wire-shaped name: the alphabet a declared feature, carrier or licence verdict is
    /// spelled in. A prose description is a document's business, not a request's.
    Name,
    /// **A declared list**: one or more wire-shaped names separated by `+` — the stages a row requires
    /// of the world's roster, or the material ids it requires of the world's registry. A list is one
    /// value rather than a repeated key, because the table declares **one value per key**; and a value
    /// naming nothing is refused by *shape*, because a requirement nobody can resolve is a bar nobody
    /// can run.
    Names,
}

impl ArgKind {
    pub const fn name(self) -> &'static str {
        match self {
            ArgKind::Chunk => "chunk",
            ArgKind::Level => "level",
            ArgKind::Field => "field",
            ArgKind::Voxel => "voxel",
            ArgKind::Side => "side",
            ArgKind::Path => "path",
            ArgKind::Digest => "digest",
            ArgKind::Name => "name",
            ArgKind::Names => "list of names",
        }
    }

    /// Why a written value is **not** of this kind's shape, or `None` when it is. Nothing here consults
    /// a world: this is the half of validation that can be decided by the line alone, and the half that
    /// makes a malformed request a refusal that writes nothing. A kind with more than one way to be
    /// wrong says which one, so its refusal can say more than *is not one*.
    pub(super) fn accepts(self, value: &str) -> Option<&'static str> {
        // The ordinary reason: the value is not one of these at all.
        const NOT_ONE: &str = "is not one";
        match self {
            ArgKind::Chunk | ArgKind::Voxel => parse_triple(value).is_none().then_some(NOT_ONE),
            ArgKind::Level => value.parse::<u8>().is_err().then_some(NOT_ONE),
            ArgKind::Field => value.is_empty().then_some(NOT_ONE),
            ArgKind::Side => (!value.parse::<u32>().is_ok_and(|side| side > 0)).then_some(NOT_ONE),
            ArgKind::Path => reference_decline(value),
            ArgKind::Digest => Digest::from_hex(value).is_none().then_some(NOT_ONE),
            ArgKind::Name => (!is_wire_name(value)).then_some(NOT_ONE),
            ArgKind::Names => names_decline(value),
        }
    }
}

/// Why a written value is not a **list of names**, or `None` when it is one: one or more wire-shaped
/// names joined by `+`, and never nothing. `+` is the list's own separator — it is not a character the
/// request or the receipt separates its parts with, so a list is one value and never a second field.
fn names_decline(value: &str) -> Option<&'static str> {
    if value.is_empty() || value.split('+').any(|part| !is_wire_name(part)) {
        return Some("is not a list of names separated by `+`");
    }
    None
}

/// Why a written value is **not a reference**, or `None` when it is one: a relative path that no
/// framing can mistake for anything else. Absolute paths, drive letters, `..` segments, **every**
/// character the request and the receipt wire separate their parts with (`=`, `~`, `;`, `,`, `|`) and
/// reserved device names are refused by *shape*, which is the half of validation a line can decide
/// alone — the other half (does the document exist, and is it the one declared) is the gate's, taken at
/// dispatch. The separators are refused here because a reference lands in a check's subject and a
/// receipt's fields: a path carrying one would be answered and then be unwritable, which is a refusal
/// owed *before* any world is read.
pub(super) fn reference_decline(value: &str) -> Option<&'static str> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(':')
        || value.split('/').any(|part| part == "..")
        || value.contains(['=', WIRE_PART, WIRE_ITEM, WIRE_FIELD, WIRE_CHECK_FIELD])
    {
        return Some("is not one");
    }
    // A device name is refused by **declaration** rather than probed, so its verdict is the same on
    // every host. Measured before this rule was written: `admit path=NUL` was **admitted** on this host
    // (reading `NUL` yields no bytes, so the empty digest was what the row declared) while `path=CON`
    // was refused `source_absent` — one class of reference, two verdicts, decided by the operating
    // system rather than by the table.
    if value.split('/').any(is_reserved_device) {
        return Some("names a reserved device, not a document");
    }
    None
}

/// Whether one segment of a reference names a **reserved device** rather than a document: `CON`,
/// `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, and the same names with any extension, because
/// the hosts that reserve these names read `NUL.txt` as `NUL`.
fn is_reserved_device(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment).trim();
    if ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|name| stem.eq_ignore_ascii_case(name))
    {
        return true;
    }
    let upper = stem.to_ascii_uppercase();
    ["COM", "LPT"].iter().any(|prefix| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.len() == 1 && rest.as_bytes()[0].is_ascii_digit())
    })
}

/// Whether a written value has the shape of a **name**: one token of letters, digits, `_`, `-` and
/// `.`. A name is what a declared feature, carrier or licence verdict is spelled in.
pub(super) fn is_wire_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.')
}

/// One declared argument of one verb.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argument {
    pub name: &'static str,
    pub kind: ArgKind,
    pub required: bool,
}

const fn arg(name: &'static str, kind: ArgKind, required: bool) -> Argument {
    Argument { name, kind, required }
}

/// A `x,y,z` triple, or `None` when the value is not one — the shape the `Chunk` and `Voxel` kinds
/// accept. It lives beside the kinds it serves, not beside the request parser, because *what a kind
/// accepts* is the kind's own business; the parser only borrows it.
pub(super) fn parse_triple(value: &str) -> Option<(i32, i32, i32)> {
    let mut parts = value.split(',');
    let x = parts.next()?.parse::<i32>().ok()?;
    let y = parts.next()?.parse::<i32>().ok()?;
    let z = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y, z))
}

// ---------------------------------------------------------------------------
// The declared checks a verb runs.
// ---------------------------------------------------------------------------

/// The **closed** set of decisions the gate makes about a call. A verb declares the subset it runs;
/// the receipt records each one with its outcome and its subject, so "which checks ran" is a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckName {
    /// The line parsed against the declared table, and the canonical line is the request's identity.
    RequestParsed,
    /// Every argument was present, of its declared kind, and resolved against the owner.
    ArgumentsTyped,
    /// The owner's seam answered (or refused by name — a refusal never reaches a receipt).
    OwnerAnswered,
    /// Every claim the answer makes re-checks against the thing it describes.
    ClaimsHold,
    /// The record gained exactly the event the call's change implies.
    Recorded,
}

impl CheckName {
    pub const ALL: [CheckName; 5] = [
        CheckName::RequestParsed,
        CheckName::ArgumentsTyped,
        CheckName::OwnerAnswered,
        CheckName::ClaimsHold,
        CheckName::Recorded,
    ];

    pub const COUNT: usize = 5;

    pub const fn name(self) -> &'static str {
        match self {
            CheckName::RequestParsed => "request_parsed",
            CheckName::ArgumentsTyped => "arguments_typed",
            CheckName::OwnerAnswered => "owner_answered",
            CheckName::ClaimsHold => "claims_hold",
            CheckName::Recorded => "recorded",
        }
    }
}

/// What one check decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    /// The check did not pass: the call is refused by this name and the answer is not handed out.
    Refused(Refusal),
}

/// One decision the gate made: **named**, with what it was made about and how it came out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub name: CheckName,
    pub subject: String,
    pub outcome: CheckOutcome,
}

// ---------------------------------------------------------------------------
// The closed tool table.
// ---------------------------------------------------------------------------

/// The verb a row of the table serves. A closed enum, so dispatch is a total `match` rather than a
/// lookup by string that a typo could silently miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Truth,
    Fields,
    Geometry,
    Held,
    Record,
    /// **The content gate's door**: a declared row arrives by reference — its document and its own
    /// acceptance check each as `path + digest`, never as bytes — and the answer is what the row
    /// became. It is the one verb that reaches the content intake, and it changes the world in
    /// nothing.
    Admit,
    /// **U9's document form, written**: the world this gate serves is written down at the declared
    /// reference, as the bytes one encoder produced, and the answer is what is now at that path — its
    /// digest, its length, and the identity and record the save carries. It changes the world in
    /// **nothing** (the record is not touched, and a document is not the world).
    Save,
    /// **U9's document form, read**: a declared reference is consumed **in place** — through the same
    /// content gate the `admit` row goes through — and the answer is the world those bytes are of: the
    /// identity, the record, and the truth the load reproduces. It changes the world in nothing, and a
    /// document that is absent, moved, or not a save of this ruleset is refused by its own name.
    Load,
}

/// One row of the tool table: the verb, its name, its declared arguments, what it answers, the owner
/// seam it dispatches to, whether a call may change the record, the checks it runs and the claims its
/// receipts may carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tool {
    pub verb: Verb,
    pub name: &'static str,
    pub args: &'static [Argument],
    pub answers: &'static str,
    /// The owner's seam this verb dispatches to — named, because that is where the answer comes from.
    pub owner: &'static str,
    /// **The policy act this row performs.** One row, one act: the mapping from a door to an act lives
    /// on the door's own declaration rather than in a second table, so *"every declared tool maps to
    /// exactly one action"* is the shape of the row instead of a rule a reader has to check
    /// (`docs/HYPERGRILLING.md` §L-5, the injection boundary; `docs/LEDGER.md` §67.4, where a
    /// stage→action map that sent a chain write to an ungated act was found factually wrong). The act's
    /// **class** is [`crate::policy::permission_of`]'s, from the act alone — never from this row.
    pub action: crate::policy::Action,
    /// Whether a call may change the record. `false` is a promise: the call appends nothing.
    pub changes_record: bool,
    pub checks: &'static [CheckName],
    /// **The claims a receipt for this row may carry**, by name. These are the evidence the answer is
    /// made of, so they are part of the row's declaration rather than whatever a writer chose to put
    /// in a claims field: [`Receipt::bind_to_row`] refuses a receipt carrying a claim its row does not
    /// declare, and a row that declares [`CheckName::ClaimsHold`] carries at least one of these. A row
    /// declaring none makes no claim — its receipt must carry none. The list is empty for exactly the
    /// rows that do not declare `claims_hold`, which a test pins.
    pub claims: &'static [&'static str],
}

/// The **closed** table, and the outward door: eight verbs over eight owner seams. The list is closed,
/// so an undeclared verb is refused by name rather than answered by the nearest match — and the two
/// save rows below arrived with U9's document form (2026-10-12, §28), each dispatching to the seam
/// that owns its question rather than to a new module of their own.
pub static TOOLS: [Tool; TOOL_COUNT] = [
    Tool {
        verb: Verb::Truth,
        name: "truth",
        args: &[],
        answers: "the world's identity and its truth digest",
        owner: "world.truth_digest",
        action: crate::policy::Action::ReadWorld,
        changes_record: false,
        checks: &[CheckName::RequestParsed, CheckName::OwnerAnswered, CheckName::ClaimsHold],
        claims: &["truth_is"],
    },
    Tool {
        verb: Verb::Fields,
        name: "fields",
        args: &[
            arg("field", ArgKind::Field, true),
            arg("at", ArgKind::Voxel, true),
            arg("side", ArgKind::Side, true),
        ],
        answers: "one declared field's values over a patch, with the values' digest",
        owner: "field_at",
        action: crate::policy::Action::ReadWorld,
        changes_record: false,
        checks: &[
            CheckName::RequestParsed,
            CheckName::ArgumentsTyped,
            CheckName::OwnerAnswered,
            CheckName::ClaimsHold,
        ],
        claims: &["field_is"],
    },
    Tool {
        verb: Verb::Geometry,
        name: "geometry",
        args: &[arg("chunk", ArgKind::Chunk, true), arg("level", ArgKind::Level, true)],
        answers: "a chunk's geometry at a level, derived or served, and the record of the derivation",
        owner: "mesher.derive",
        // The one row that changes the record: its act is the append, which is gated.
        action: crate::policy::Action::AppendEvent,
        changes_record: true,
        checks: &[
            CheckName::RequestParsed,
            CheckName::ArgumentsTyped,
            CheckName::OwnerAnswered,
            CheckName::ClaimsHold,
            CheckName::Recorded,
        ],
        // A derivation says three things: the geometry is held, it is the geometry of **this** world's
        // truth (the answer carries the truth digest it was derived over), and the event this call
        // appended is still the event at its position — the last only when there was an append, since a
        // cache hit appends nothing.
        claims: &["truth_is", "mesh_is_held", "event_is"],
    },
    Tool {
        verb: Verb::Held,
        name: "held",
        args: &[],
        answers: "the geometry the gate holds, how much of it is stale, and its digest",
        owner: "resident.digest",
        action: crate::policy::Action::ReadWorld,
        changes_record: false,
        checks: &[CheckName::RequestParsed, CheckName::OwnerAnswered, CheckName::ClaimsHold],
        claims: &["held_is"],
    },
    Tool {
        verb: Verb::Record,
        name: "record",
        args: &[],
        answers: "the chain's length and its head",
        owner: "chain.head",
        action: crate::policy::Action::ReadWorld,
        changes_record: false,
        checks: &[CheckName::RequestParsed, CheckName::OwnerAnswered, CheckName::ClaimsHold],
        claims: &["record_head_is"],
    },
    Tool {
        verb: Verb::Admit,
        name: "admit",
        args: &[
            arg("path", ArgKind::Path, true),
            arg("digest", ArgKind::Digest, true),
            arg("feature", ArgKind::Name, true),
            arg("licence", ArgKind::Name, true),
            arg("claim", ArgKind::Name, true),
            arg("carrier", ArgKind::Name, true),
            arg("check", ArgKind::Path, true),
            arg("check_digest", ArgKind::Digest, true),
            // **The world's own declarations a row may require**, and the table's first **optional**
            // arguments: absent means *this row requires nothing of that world*, which is a declaration
            // rather than a silence. Conformance is against U3's roster and U2's registry, so a required
            // stage is a name and a required material is an id — the id the registry itself refuses by
            // when it never admitted it.
            arg("requires_stages", ArgKind::Names, false),
            arg("requires_materials", ArgKind::Names, false),
        ],
        answers: "what the declared row became: the digest its document hashes to, read in place, \
                   the claim it was admitted under, and every axis's state",
        owner: "intake.admit",
        action: crate::policy::Action::AdmitContent,
        changes_record: false,
        // **No `ClaimsHold`, and that is a declaration rather than an omission**: the evidence an
        // admission carries is the digest the seam read in place, and the owners a claim of this gate
        // is re-checked against do not include a file — so this row makes **no claim** rather than one
        // nothing could re-check. `Recorded` stays: the row promises it may not change the record, and
        // that promise is checked against the record.
        checks: &[
            CheckName::RequestParsed,
            CheckName::ArgumentsTyped,
            CheckName::OwnerAnswered,
            CheckName::Recorded,
        ],
        // A row that makes no claim declares none: its evidence is the digest the intake read in place.
        claims: &[],
    },
    Tool {
        verb: Verb::Save,
        name: "save",
        args: &[arg("path", ArgKind::Path, true)],
        answers: "a world written down as a document at the declared reference: the digest the bytes \
                   hash to, their length, and the identity, ruleset and record the save carries",
        owner: "save.write_document",
        action: crate::policy::Action::SaveWorld,
        changes_record: false,
        // **No `ClaimsHold`, for the same reason the `admit` row declares none**: what this answer is
        // about is a file, and a claim's owners are the world's own seams. `Recorded` stays: the row
        // promises it changes no record, and that promise is checked against the record.
        checks: &[
            CheckName::RequestParsed,
            CheckName::ArgumentsTyped,
            CheckName::OwnerAnswered,
            CheckName::Recorded,
        ],
        claims: &[],
    },
    Tool {
        verb: Verb::Load,
        name: "load",
        args: &[arg("path", ArgKind::Path, true), arg("digest", ArgKind::Digest, true)],
        answers: "the world a declared document is of, consumed in place by reference: the digest the \
                   bytes hash to, their length, and the identity, record and truth the load reproduces",
        owner: "save.load_document",
        action: crate::policy::Action::LoadWorld,
        changes_record: false,
        // The counterpart of `save`: an answer about a document, re-established from its request rather
        // than believed, with the record's promise checked.
        checks: &[
            CheckName::RequestParsed,
            CheckName::ArgumentsTyped,
            CheckName::OwnerAnswered,
            CheckName::Recorded,
        ],
        claims: &[],
    },
];

/// How many verbs the table declares. Written down, and pinned to [`TOOLS`]' length by a test, so a
/// row silently added or dropped is a failing check rather than a new surface.
pub const TOOL_COUNT: usize = 8;

/// The row a verb name names, or nothing.
pub fn tool(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

