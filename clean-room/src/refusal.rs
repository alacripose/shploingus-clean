//! **The refusal vocabulary — named, closed, and collecting every problem at once.**
//!
//! Derived from `docs/DESIGN-SPEC.md` (the one rule, inherited whole: **refuse, never default**) and
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §3 (a value that cannot be validated is refused and named,
//! never replaced by a plausible substitute).
//!
//! Constitution **P2** is what this module exists for. A `Refusal` carries the **kind** and the
//! **detail** — never a bare boolean and never a silent clamp — and a [`RefusalSet`] collects
//! **all** the problems in one pass rather than the first, so an author fixes one round of them
//! instead of one at a time. There is deliberately **no** `Default` that invents a value and no
//! `From<bool>`: a refusal is a fact, and a fact that can be manufactured from `false` is not one.

use std::fmt;

/// The closed vocabulary of reasons this world model refuses something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefusalKind {
    /// A voxel coordinate cannot be expressed (Euclidean chunk math overflowed `i32`).
    CoordinateOverflow,
    /// A subvoxel axis outside `0..4`.
    SubvoxelOutOfRange,
    /// A detail-lattice axis outside `0..256`.
    DetailOutOfRange,
    /// A ladder level at or past the declared 20.
    LadderOutOfRange,
    /// A chamber of a chunk (`0..32` per axis) asked for with an axis outside it.
    ChunkOffsetOutOfRange,
    /// A material id the registry has not admitted.
    UnadmittedMaterial,
    /// A material id admitted twice.
    DuplicateMaterial,
    /// A density that is not a positive exact rational.
    NonPositiveDensity,
    /// A sourced value with no source.
    NoProvenance,
    /// A reflectance channel outside `[0, 1]`.
    ReflectanceOutOfRange,
    /// A declared period of zero seconds — no world is a function of nothing.
    ZeroPeriod,
    /// A `Profile` whose parameters are not a shape (non-finite, or a radius/height that is not
    /// positive, or a `Da` the shape law is not defined for).
    MalformedProfile,
    /// A shape parameter outside its declared domain (a height fraction, an azimuth, a radius).
    ParameterOutOfRange,
    /// A shaper that names no level it acts at, or claims the addressed level and declares no field.
    ShaperDeclaresNothing,
    /// A shaper name admitted twice.
    DuplicateShaper,
    /// A **material** shaper tried to change occupancy — only a **physical** shaper may.
    MaterialShaperCannotChangeOccupancy,
    /// A recolour of an empty cell — there is no material there to recolour.
    NoCellToRecolour,
    /// The chain's links or digests do not verify: it has been rewritten or damaged.
    ChainBroken,
    /// A wire form that cannot be parsed.
    MalformedWire,
    /// A payload that cannot survive the wire form (a tab or a newline in it).
    PayloadNotWirable,
    /// A ticket id that no `TicketFiled` event names.
    UnknownTicket,
    /// A ticket that already has a resolution.
    TicketAlreadyResolved,
    // ---- U3 world generation (`specs/003-u3-shapers/contracts/u3-seams.md` C1–C4) ----
    /// A declared stage that writes nothing — a placeholder wearing a name.
    OutputUndeclared,
    /// A stage that consumes a stage declared *below* it: the graph is not a call chain.
    ReadsBelow,
    /// Two declared stages claim the same output field.
    DuplicateOutput,
    /// A cadence rung outside the closed four-rung ladder.
    CadenceOffLadder,
    /// A law's dial is required and has no declared value; there is no invented default.
    DialUnset,
    /// A tick earlier than the last driven tick: the schedule never runs backwards.
    Rewind,
    /// A stage's declared input is unavailable at this request — refused, never zeroed.
    StageInputAbsent,
    /// No stage in the roster declares this field as an output.
    FieldUndeclared,
    /// A law definable only over a region was asked without one.
    RegionRequired,
    /// A region-only law was asked at a lone site; a site alone cannot evaluate it.
    SiteAlone,
    /// The stage is the record, not a producer: it is bodyless **by design**.
    NotAGenerator,
    /// A declared stage whose law is not implemented yet. Named, so an owed body reads as owed
    /// rather than as a zero that looks like a value.
    BodyOwed,
    /// A blend whose two domains carry the same weight at a column: there is no dominant domain to
    /// pick, and picking one would invent a boundary the world does not have.
    DomainTie,
    // ---- U4 geometry (`specs/004-u4-mesh-stream-work/contracts/u4-seams.md` C1–C4) ----
    /// A render level outside the declared `0..=2` ladder. Clamping would silently serve coarser
    /// geometry than was asked for, which is a different world from the one requested.
    LevelOffLadder,
    /// The render horizon and the residency horizon contradict each other: the levels that are cheap
    /// enough to render reach past what is allowed to stay resident.
    HorizonMixed,
    /// A derivation whose measured work is past the ceiling it was given. The ceiling is checked
    /// against what the derivation **did**, so the refusal carries real numbers.
    GeometryBudgetExceeded,
    /// A chunk outside the declared residency horizon asked for geometry: not a cache miss, a
    /// request the anchor's residency policy refuses.
    GeometryOutOfResidency,
    /// A chunk with **no column under the world's own patch** asked a cell world for geometry: a cell
    /// world declares where it is, and a request with no cells of it to mesh is refused rather than
    /// answered with an empty mesh, which would read as smooth ground.
    GeometryOutsideTheWorld,
    // ---- U6 the record and the API layer (`specs/005-u6-record-api/contracts/u6-seams.md` C1–C4) ----
    /// A request names a verb the closed tool table does not declare. The table is the contract: an
    /// undeclared verb is not an operation this gate can perform.
    ToolUndeclared,
    /// A request carries an argument the verb does not declare — arity is part of the declaration.
    ArgumentUndeclared,
    /// A verb's declared argument is missing. There is no default: an absent argument is refused.
    ArgumentAbsent,
    /// An argument is present but not of its declared kind (a chunk that is not three integers, a
    /// side that is not positive, a duplicate key).
    ArgumentMalformed,
    /// A claim in a receipt does not hold against the thing it describes: the answer is not an
    /// answer this layer hands out.
    ClaimUnverified,
    /// A receipt does not match the row it names: its answer is another verb's, its claims are not the
    /// claims its row's declaration allows, or the checks that ran are not the checks its row declares.
    /// A receipt is the evidence for an answer, so one whose evidence does not match its row is refused
    /// rather than verified over nothing (`Receipt::bind_to_row`).
    ReceiptMismatch,
    // ---- U9 saves and loads (`specs/006-world-saves-and-seeds/contracts/u9-seams.md` C1–C3) ----
    /// The bytes are not a save: the magic, the version or a declared width is wrong. A format that
    /// is not this one is refused, never reinterpreted as a nearly-right world.
    NotASave,
    /// A framed part runs past the end of the bytes: the file is shorter than it claims to be.
    SaveTruncated,
    /// The trailer digest is not the digest of the bytes before it: one byte in the save changed.
    SaveTampered,
    /// A declared sub-stream's seed is missing from the save, or names a stream this build does not
    /// declare, or is declared twice — the world's seed systems are carried individually, never
    /// folded into one dial.
    SeedUndeclared,
    /// The save names an identity the loader does not hold: another seed, another period, another
    /// configuration/ruleset identity, or a sub-stream seed that is not this world's own. Loaded
    /// under a different world, a save is a different world.
    IdentityMismatch,
    /// The record the save declares (its head and its length) is not the record it carries. A save
    /// whose history was replaced by another valid one is not the same record.
    RecordDiverged,
    /// The **document form's write side**: the bytes exist and are a save, but they could not be
    /// placed at the reference they are declared at. A document that cannot be written is refused by
    /// name rather than reported as a save — the round trip's other half is the *read*, and a caller
    /// told "written" about bytes that are not there would be reading a save that does not exist
    /// (`specs/006-world-saves-and-seeds/contracts/u9-seams.md` C4's document form).
    DocumentUnwritable,
    // ---- U6's content intake (`specs/005-u6-record-api/`; the gate's first slice) ----
    /// A declared reference is not there to be consumed **in place**: no such file, or it could not be
    /// read. The gate carries no bytes — it reads a document where it lives — so a document that is
    /// gone is refused rather than reconstructed from a copy.
    SourceAbsent,
    /// The bytes at the declared reference are not the bytes the row describes: what was read in place
    /// hashes to something else. Integrity is not established, and identity is therefore unanswerable
    /// either way — recorded rather than passed.
    SourceDigestMismatch,
    /// A row names a carrier the gate does not declare. The carriers are closed (U3–U8, the systems
    /// that may carry content derived from a reference): a name outside them is **undeclared**, not
    /// merely unusual.
    CarrierUndeclared,
    /// A row requires a **stage the world does not run**: conformance is against U3's own [`Declared`]
    /// roster, not against a list of names, so a required stage the roster does not declare is refused
    /// **with that stage named** (`docs/LEDGER.md` §55.3). A required material the registry never
    /// admitted is the registry's own refusal, [`RefusalKind::UnadmittedMaterial`], passed through.
    StageUndeclared,
    /// A row's claim is a word the gate does not declare as one a request line can carry. The claims
    /// a *row of the table* can state are closed — `checksum_only` is the one whose statement is not
    /// made out of band — so a word outside them is undeclared rather than rounded to the nearest
    /// claim (a pin names a signer and a digest an operator states elsewhere, which a request line is
    /// not).
    ClaimUndeclared,
    /// A row relies on a licence verdict the gate does not declare. The verdicts are closed
    /// (`permitted`, `refused`): a word outside them is **undeclared**, exactly as an undeclared
    /// carrier or an undeclared field is — the gate carries the verdict, it does not invent one.
    LicenceUndeclared,
    /// The row relies on a licence verdict that **refuses** its source. Nothing derived may be
    /// implemented from a refused reference, so the row is refused before it can carry anything.
    LicenceRefused,
    /// The bytes are not the digest an operator pinned for that signer — a different fact from a wrong
    /// checksum, because the payload can consistently claim its own bytes and still not be the release
    /// that was pinned.
    PinMismatch,
    /// An axis the gate declares was **not checked**: nothing was consulted, so nothing was found. An
    /// unchecked axis is never a pass.
    AxisUnchecked,
    /// An axis was checked and found **not there**: a check the row declares as owed, or a declared
    /// answer that carries nothing yet. Shown by name rather than counted as a pass.
    AxisInactive,
    /// **A quantity that cannot be weighed** (S-13): nothing crossed the boundary, so there is no
    /// quantity there to weigh. Answering `0 g` would be a measurement the gate did not make.
    NotWeighable,
    /// A **mass** was handed where a count is required. S-12's band is per item, and a mass is not a
    /// count of one.
    NotACount,
    /// A band whose minimum exceeds its maximum: it has no inside, and it has never held a reading.
    BandInverted,
    /// A reading **no gauge watches** — an item at a site no gauge of the wall answers for. Silence is
    /// not a verdict, so it is refused by name.
    Unwatched,
    // ---- U7 the decision layer (`specs/009-u7-decision-layer/`, `src/{axis,laya,von}.rs`) ----
    /// Two values from **different ladders** were compared. An axis is not a scale: `A` is a grade and an
    /// origin, and reading one ladder's token on another's is how a bare word comes to mean two things
    /// while looking like a measurement (`docs/LEDGER.md` C-73 §73.1, C-97.2's inherited rule 51).
    AxisInterchangeRefused,
    /// A ladder was rendered as a **number** — a percentage or a score. A vocabulary of words is not a
    /// quantity, so there is no value of it to reduce: the number a caller wants is one this record never
    /// stated, and a later reader would sort, average and threshold it as if it had been measured.
    ScalarReductionRefused,
    /// A **bare ladder token** — `A`, `SSS`, `gold` — written with no ladder named. Two or more declared
    /// ladders claiming it is `AMBIGUOUS`; exactly one claiming it is `UNQUALIFIED`, and that is still a
    /// bug, because the reading is inferable and is still not a reading.
    BareTokenRefused,
    /// An axis the coverage table does **not** describe: a row claiming no values for a declared ladder, a
    /// row for an axis this build gives no values to, two rows for one axis, or an axis with no row at
    /// all. A ladder with no representation is a failure rather than a todo.
    UncoveredAxis,
    /// A coverage row that claims values the axis does not declare for itself, or claims values for a
    /// ladder this tree has not read. The row is well formed and simply untrue, which **looks checked** —
    /// worse than an empty row, because a reader believes it (`docs/LEDGER.md` R-59.7's anti-fabrication
    /// guard).
    FabricatedAxis,
    /// A **ballot** that cannot be judged: an empty id, claim, artifact, instruction or presenter, a
    /// question with fewer than two terms, or a `max-age` of zero — *not a window*, because an answer that
    /// expires before it can be read is not a current answer.
    BallotIncomplete,
    /// A ballot's **floor** is outside its own terms: a `passes` naming an option the question does not
    /// offer, a `floor` that is not one of the declared rungs, or a `noul` threshold outside `(0, 1]`. A
    /// floor chosen where the answer can reach it is a floor chosen to fit the answer.
    FloorUndeclared,
    /// An instrument that is **missing or unreachable** was asked to judge. A missing instrument is not a
    /// pass, which is why this refusal exists rather than an empty reading: an unanswered question and a
    /// question answered `belongs` are not the same fact.
    OracleUnreachable,
    /// The instrument's **calibration does not beat the majority class** of the known-answer set this
    /// project ships, so its answers cannot be quoted as evidence of anything. The build gate prints
    /// `model-oracle not-established` (`docs/LEDGER.md` R-59.5's standing honesty rule).
    NotEstablished,
    /// A declared option that **no reading calibrates**. A five-way question whose set never names one of
    /// its answers cannot be measured for that answer, so the calibration is refused rather than reported
    /// over the four it happens to hold.
    OptionUncalibrated,
    /// A **`not_belongs` reading with no human's reason recorded in the same act**. A negative reading
    /// filed bare is a fact a successor cannot act on, and the reason is the part of it that carries
    /// (`docs/LEDGER.md` §59.5 item 9's fourth rule).
    ReasonAbsent,
    /// **A non-reading read as a verdict.** `no_reading` is the verdict vocabulary's third word, not a
    /// synonym for either of the other two: there was nothing to read, so a floor has nothing to compare
    /// and answering `false` would report a reading that never happened.
    NoReading,
    // ---- U8 the shell (`specs/010-u8-shell/contracts/u8-seams.md` C1–C3) ----
    /// An engine constant that is **not in the admitted table**. `docs/UI-SPEC.md` §8's four-bucket
    /// rule is the whole point: a number that gets drawn is an MD1 rule, an MD1 example, a platform
    /// translation or a counted project extension, and a number in none of those buckets is refused by
    /// name rather than drawn.
    ConstantUnadmitted,
    /// A pane operation naming a window the recipe never mounted. A window is addressed by its own
    /// **stable id**, never by a list position (`docs/UI-SPEC.md` §2, C-51 §51.3), so an unknown id is
    /// a refusal and not an index that silently lands on a neighbour.
    PaneUnknown,
    /// A split that would leave a pane below the declared **40 × 40 floor** (`docs/UI-SPEC.md` §3). The
    /// floor is a refusal rather than a clamp because a silently smaller pane is a layout the person
    /// asked for and did not get.
    SplitFloorBreached,
    // ---- The policy chokepoint (`src/policy.rs`; `docs/HYPERGRILLING.md` §L-5, `docs/LEDGER.md`
    //      R-62.3). Five names, and each is one of L-5's four properties or its readable-matrix rule.
    /// **Denied by the one decision function.** The detail quotes the list: the act, the scope it
    /// requires, the scopes the caller holds and who granted each — so a denial is actionable rather
    /// than a bare *denied* (`docs/HYPERGRILLING.md` §L-5: *a refusal quotes the list*).
    PolicyDenied,
    /// A grant (or a caller) that **names no author**. Refused at **construction**, so an
    /// unauthorised grant is not a value this build can hold — *"an authorization with no author is
    /// not an authorization"*.
    GrantUnattributed,
    /// A grant whose scope is absent, undeclared or unbounded. *"'everything' is not a scope"*, and a
    /// scope that means all cannot be quoted back in a refusal.
    GrantUnscoped,
    /// **A rejection rather than a permission**: this material may be used and never persisted
    /// (`never_store`). Distinct from [`RefusalKind::PolicyDenied`] because the *storage* is refused
    /// rather than the use.
    NeverStore,
    /// A matrix entry with nothing to say. A row a reader cannot understand is refused by name rather
    /// than printed empty (`docs/LEDGER.md` §66.3).
    ActionUndescribed,
    // ---- U5's voices (`src/feeling.rs`, `src/audio.rs`; the feeling wheel's own vocabulary) — four
    //      names, one per subject: the reading that is not there, the word that is not declared, the
    //      reading off the ladder, and the reading that contradicts itself.
    /// **No feeling at all** was handed to a voice. An absent reading is not a silent bed: a bed
    /// invented for a state that was never declared is a bed a reader would take for a measured one.
    FeelingAbsent,
    /// A **word the wheel does not declare** — a tertiary state's name where a family's is required, or
    /// an invented hue. *"Do not invent a new hue simply because an emotion feels unusual"*: the
    /// refusal quotes the six words it does know rather than matching the nearest neighbour.
    FeelingUndeclared,
    /// A reading **off the ladder this build declares**: a radius past the wheel's own rim, or a signed
    /// drift that walks the peak outside the twelve declared bands. The record's own sentence is
    /// *"refused by name if it is off the ladder"*, and a band the declared table does not hold cannot
    /// be voiced — so the reading is refused rather than voiced as its nearest band.
    FeelingUncalibrated,
    /// A reading that **contradicts itself**: more than one family named at once. *"If multiple families
    /// compete, select one dominant family rather than blending hues"* — a blended hue is a colour the
    /// wheel does not declare.
    FeelingContradictory,
    // ---- U12's first mob (`src/being.rs`) — two names, one per subject: the world cannot meet the need
    //      anywhere the being may reach, and the being's own draft reads below the floor its charter
    //      declares. Neither is a policy denial: a denial is a governance fact, and these are facts about
    //      the world and the record.
    /// **The world cannot meet the need where the being may reach.** It walked its whole declared reach
    /// along the game's own flow receivers and found no cell where the world's own filled surface stands
    /// above the ground (or stood on a sink with no water). The being refuses by name and the refusal is
    /// recorded, rather than the need being met by a meter that would hide the world's own answer.
    WaterUnreachable,
    /// **An act whose own record reads below the charter's declared floor.** The stage question came back
    /// with a stage under the one the charter requires of an act that touches anything, so the act is
    /// filed rather than taken — a being executes its own designs, and a draft that is not yet one is
    /// refused by name rather than acted on.
    StageBelowFloor,
    // ---- U1's visible client (`src/surface.rs`, §36) — two names, one per subject: a material the frame
    //      cannot derive a colour from, and the platform's own refusal of an interface call. The first is
    //      U1's own rule (a photograph of a colour nobody recorded is a colour invented); the second is a
    //      fact about the machine rather than about the world, and it is named so a client that could not
    //      be shown is never reported as one that was.
    /// **A material that derives no colour.** The column's own admitted material carries no reflectance,
    /// so the frame has no colour to draw there. `LEDGER.md` C-39: a reflectance is impossible to carry
    /// without its source — and a pixel with neither is a pixel nobody recorded.
    NoReflectance,
    /// **The platform refused an interface call the client needs.** A window that could not be created, a
    /// client area that is not the frame's own size, a device that could not be drawn on, a blit that
    /// copied fewer scan lines than the frame has, or a window interface this platform does not declare.
    /// The detail names which call answered nothing; the client exits non-zero rather than presenting a
    /// frame nobody saw.
    PlatformRefused,
}

impl RefusalKind {
    /// Every kind this build declares, in declaration order. A reader that meets a refusal **name**
    /// somewhere outside this process — on the API layer's wire form — needs to turn that name back
    /// into a kind, and every closed vocabulary in this crate (`Namespace`, `EventKind`, `FieldId`,
    /// `Cadence`, `CheckName`, `Level`) carries its own list for the same reason.
    ///
    /// A kind added to the enum without a row here is **refused** by such a reader rather than
    /// misread — the failure mode is an unfamiliar refusal, never a wrong one.
    pub const ALL: [RefusalKind; 95] = [
        RefusalKind::CoordinateOverflow,
        RefusalKind::SubvoxelOutOfRange,
        RefusalKind::DetailOutOfRange,
        RefusalKind::LadderOutOfRange,
        RefusalKind::ChunkOffsetOutOfRange,
        RefusalKind::UnadmittedMaterial,
        RefusalKind::DuplicateMaterial,
        RefusalKind::NonPositiveDensity,
        RefusalKind::NoProvenance,
        RefusalKind::ReflectanceOutOfRange,
        RefusalKind::ZeroPeriod,
        RefusalKind::MalformedProfile,
        RefusalKind::ParameterOutOfRange,
        RefusalKind::ShaperDeclaresNothing,
        RefusalKind::DuplicateShaper,
        RefusalKind::MaterialShaperCannotChangeOccupancy,
        RefusalKind::NoCellToRecolour,
        RefusalKind::ChainBroken,
        RefusalKind::MalformedWire,
        RefusalKind::PayloadNotWirable,
        RefusalKind::UnknownTicket,
        RefusalKind::TicketAlreadyResolved,
        RefusalKind::OutputUndeclared,
        RefusalKind::ReadsBelow,
        RefusalKind::DuplicateOutput,
        RefusalKind::CadenceOffLadder,
        RefusalKind::DialUnset,
        RefusalKind::Rewind,
        RefusalKind::StageInputAbsent,
        RefusalKind::FieldUndeclared,
        RefusalKind::RegionRequired,
        RefusalKind::SiteAlone,
        RefusalKind::NotAGenerator,
        RefusalKind::BodyOwed,
        RefusalKind::DomainTie,
        RefusalKind::LevelOffLadder,
        RefusalKind::HorizonMixed,
        RefusalKind::GeometryBudgetExceeded,
        RefusalKind::GeometryOutOfResidency,
        RefusalKind::GeometryOutsideTheWorld,
        RefusalKind::ToolUndeclared,
        RefusalKind::ArgumentUndeclared,
        RefusalKind::ArgumentAbsent,
        RefusalKind::ArgumentMalformed,
        RefusalKind::ClaimUnverified,
        RefusalKind::ReceiptMismatch,
        RefusalKind::NotASave,
        RefusalKind::SaveTruncated,
        RefusalKind::SaveTampered,
        RefusalKind::SeedUndeclared,
        RefusalKind::IdentityMismatch,
        RefusalKind::RecordDiverged,
        RefusalKind::DocumentUnwritable,
        RefusalKind::SourceAbsent,
        RefusalKind::SourceDigestMismatch,
        RefusalKind::CarrierUndeclared,
        RefusalKind::StageUndeclared,
        RefusalKind::ClaimUndeclared,
        RefusalKind::LicenceUndeclared,
        RefusalKind::LicenceRefused,
        RefusalKind::PinMismatch,
        RefusalKind::AxisUnchecked,
        RefusalKind::AxisInactive,
        RefusalKind::NotWeighable,
        RefusalKind::NotACount,
        RefusalKind::BandInverted,
        RefusalKind::Unwatched,
        RefusalKind::AxisInterchangeRefused,
        RefusalKind::ScalarReductionRefused,
        RefusalKind::BareTokenRefused,
        RefusalKind::UncoveredAxis,
        RefusalKind::FabricatedAxis,
        RefusalKind::BallotIncomplete,
        RefusalKind::FloorUndeclared,
        RefusalKind::OracleUnreachable,
        RefusalKind::NotEstablished,
        RefusalKind::OptionUncalibrated,
        RefusalKind::ReasonAbsent,
        RefusalKind::NoReading,
        RefusalKind::ConstantUnadmitted,
        RefusalKind::PaneUnknown,
        RefusalKind::SplitFloorBreached,
        RefusalKind::PolicyDenied,
        RefusalKind::GrantUnattributed,
        RefusalKind::GrantUnscoped,
        RefusalKind::NeverStore,
        RefusalKind::ActionUndescribed,
        RefusalKind::FeelingAbsent,
        RefusalKind::FeelingUndeclared,
        RefusalKind::FeelingUncalibrated,
        RefusalKind::FeelingContradictory,
        RefusalKind::WaterUnreachable,
        RefusalKind::StageBelowFloor,
        RefusalKind::NoReflectance,
        RefusalKind::PlatformRefused,
    ];

    /// How many kinds this build declares. Written down, and pinned to [`RefusalKind::ALL`] by a test.
    pub const COUNT: usize = 95;

    /// The kind a declared name names, or nothing — so a reader refuses an unfamiliar name instead of
    /// inventing one.
    pub fn parse(name: &str) -> Option<Self> {
        RefusalKind::ALL.iter().copied().find(|kind| kind.name() == name)
    }

    /// The recorded name of the refusal. The **name** is the contract: a caller may match on it, so
    /// it does not change to make a test pass.
    pub const fn name(self) -> &'static str {
        match self {
            RefusalKind::CoordinateOverflow => "coordinate_overflow",
            RefusalKind::SubvoxelOutOfRange => "subvoxel_out_of_range",
            RefusalKind::DetailOutOfRange => "detail_out_of_range",
            RefusalKind::LadderOutOfRange => "ladder_out_of_range",
            RefusalKind::ChunkOffsetOutOfRange => "chunk_offset_out_of_range",
            RefusalKind::UnadmittedMaterial => "unadmitted_material",
            RefusalKind::DuplicateMaterial => "duplicate_material",
            RefusalKind::NonPositiveDensity => "non_positive_density",
            RefusalKind::NoProvenance => "no_provenance",
            RefusalKind::ReflectanceOutOfRange => "reflectance_out_of_range",
            RefusalKind::ZeroPeriod => "zero_period",
            RefusalKind::MalformedProfile => "malformed_profile",
            RefusalKind::ParameterOutOfRange => "parameter_out_of_range",
            RefusalKind::ShaperDeclaresNothing => "shaper_declares_nothing",
            RefusalKind::DuplicateShaper => "duplicate_shaper",
            RefusalKind::MaterialShaperCannotChangeOccupancy => {
                "material_shaper_cannot_change_occupancy"
            }
            RefusalKind::NoCellToRecolour => "no_cell_to_recolour",
            RefusalKind::ChainBroken => "chain_broken",
            RefusalKind::MalformedWire => "malformed_wire",
            RefusalKind::PayloadNotWirable => "payload_not_wirable",
            RefusalKind::UnknownTicket => "unknown_ticket",
            RefusalKind::TicketAlreadyResolved => "ticket_already_resolved",
            RefusalKind::OutputUndeclared => "output_undeclared",
            RefusalKind::ReadsBelow => "reads_below",
            RefusalKind::DuplicateOutput => "duplicate_output",
            RefusalKind::CadenceOffLadder => "cadence_off_ladder",
            RefusalKind::DialUnset => "dial_unset",
            RefusalKind::Rewind => "rewind",
            RefusalKind::StageInputAbsent => "stage_input_absent",
            RefusalKind::FieldUndeclared => "field_undeclared",
            RefusalKind::RegionRequired => "region_required",
            RefusalKind::SiteAlone => "site_alone",
            RefusalKind::NotAGenerator => "not_a_generator",
            RefusalKind::BodyOwed => "body_owed",
            RefusalKind::DomainTie => "domain_tie",
            RefusalKind::LevelOffLadder => "level_off_ladder",
            RefusalKind::HorizonMixed => "horizon_mixed",
            RefusalKind::GeometryBudgetExceeded => "geometry_budget_exceeded",
            RefusalKind::GeometryOutOfResidency => "geometry_out_of_residency",
            RefusalKind::GeometryOutsideTheWorld => "geometry_outside_the_world",
            RefusalKind::ToolUndeclared => "tool_undeclared",
            RefusalKind::ArgumentUndeclared => "argument_undeclared",
            RefusalKind::ArgumentAbsent => "argument_absent",
            RefusalKind::ArgumentMalformed => "argument_malformed",
            RefusalKind::ClaimUnverified => "claim_unverified",
            RefusalKind::ReceiptMismatch => "receipt_mismatch",
            RefusalKind::NotASave => "not_a_save",
            RefusalKind::SaveTruncated => "save_truncated",
            RefusalKind::SaveTampered => "save_tampered",
            RefusalKind::SeedUndeclared => "seed_undeclared",
            RefusalKind::IdentityMismatch => "identity_mismatch",
            RefusalKind::RecordDiverged => "record_diverged",
            RefusalKind::DocumentUnwritable => "document_unwritable",
            RefusalKind::SourceAbsent => "source_absent",
            RefusalKind::SourceDigestMismatch => "source_digest_mismatch",
            RefusalKind::CarrierUndeclared => "carrier_undeclared",
            RefusalKind::StageUndeclared => "stage_undeclared",
            RefusalKind::ClaimUndeclared => "claim_undeclared",
            RefusalKind::LicenceUndeclared => "licence_undeclared",
            RefusalKind::LicenceRefused => "licence_refused",
            RefusalKind::PinMismatch => "pin_mismatch",
            RefusalKind::AxisUnchecked => "axis_unchecked",
            RefusalKind::AxisInactive => "axis_inactive",
            RefusalKind::NotWeighable => "not_weighable",
            RefusalKind::NotACount => "not_a_count",
            RefusalKind::BandInverted => "band_inverted",
            RefusalKind::Unwatched => "unwatched",
            RefusalKind::AxisInterchangeRefused => "axis_interchange_refused",
            RefusalKind::ScalarReductionRefused => "scalar_reduction_refused",
            RefusalKind::BareTokenRefused => "bare_token_refused",
            RefusalKind::UncoveredAxis => "uncovered_axis",
            RefusalKind::FabricatedAxis => "fabricated_axis",
            RefusalKind::BallotIncomplete => "ballot_incomplete",
            RefusalKind::FloorUndeclared => "floor_undeclared",
            RefusalKind::OracleUnreachable => "oracle_unreachable",
            RefusalKind::NotEstablished => "not_established",
            RefusalKind::OptionUncalibrated => "option_uncalibrated",
            RefusalKind::ReasonAbsent => "reason_absent",
            RefusalKind::NoReading => "no_reading",
            RefusalKind::ConstantUnadmitted => "constant_unadmitted",
            RefusalKind::PaneUnknown => "pane_unknown",
            RefusalKind::SplitFloorBreached => "split_floor_breached",
            RefusalKind::PolicyDenied => "policy_denied",
            RefusalKind::GrantUnattributed => "grant_unattributed",
            RefusalKind::GrantUnscoped => "grant_unscoped",
            RefusalKind::NeverStore => "never_store",
            RefusalKind::ActionUndescribed => "action_undescribed",
            RefusalKind::FeelingAbsent => "feeling_absent",
            RefusalKind::FeelingUndeclared => "feeling_undeclared",
            RefusalKind::FeelingUncalibrated => "feeling_uncalibrated",
            RefusalKind::FeelingContradictory => "feeling_contradictory",
            RefusalKind::WaterUnreachable => "water_unreachable",
            RefusalKind::StageBelowFloor => "stage_below_floor",
            RefusalKind::NoReflectance => "no_reflectance",
            RefusalKind::PlatformRefused => "platform_refused",
        }
    }
}

impl fmt::Display for RefusalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One refusal: **what** was refused and **why**, in words a reader can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    kind: RefusalKind,
    detail: String,
}

impl Refusal {
    pub fn new(kind: RefusalKind, detail: impl Into<String>) -> Self {
        Refusal { kind, detail: detail.into() }
    }

    pub const fn kind(&self) -> RefusalKind {
        self.kind
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.name(), self.detail)
    }
}

/// **Every** problem found in one pass, never just the first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefusalSet {
    items: Vec<Refusal>,
}

impl RefusalSet {
    pub const fn new() -> Self {
        RefusalSet { items: Vec::new() }
    }

    pub fn refuse(&mut self, kind: RefusalKind, detail: impl Into<String>) {
        self.items.push(Refusal::new(kind, detail));
    }

    pub fn push(&mut self, refusal: Refusal) {
        self.items.push(refusal);
    }

    pub fn extend(&mut self, other: RefusalSet) {
        self.items.extend(other.items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn all(&self) -> &[Refusal] {
        &self.items
    }

    /// The first refusal, which is what a one-line report shows — but the whole set is still here.
    pub fn first_named(&self) -> Option<&Refusal> {
        self.items.first()
    }

    /// Whether the set contains a refusal of a given kind. Lets a caller assert on a *named* reason
    /// rather than on the count.
    pub fn contains(&self, kind: RefusalKind) -> bool {
        self.items.iter().any(|r| r.kind == kind)
    }

    /// The whole set as **one refusal**: the first refusal's kind, with **every** refusal's own name
    /// and detail in the message. A channel that can carry one refusal (the transport's line, the
    /// chain's wire form) therefore carries all of them rather than silently dropping the rest — and a
    /// set of one is passed through untouched, name and words alike. `None` for an empty set.
    pub fn joined(&self) -> Option<Refusal> {
        let first = self.items.first()?;
        if self.items.len() == 1 {
            return Some(first.clone());
        }
        Some(Refusal::new(
            first.kind(),
            self.items
                .iter()
                .map(|refusal| format!("[{}] {}", refusal.kind().name(), refusal.detail()))
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }

    /// Turn a completed value into a result: `Ok` only when nothing was refused.
    pub fn into_result<T>(self, ok: T) -> Result<T, RefusalSet> {
        if self.items.is_empty() {
            Ok(ok)
        } else {
            Err(self)
        }
    }
}

impl fmt::Display for RefusalSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.items.is_empty() {
            return f.write_str("no refusals");
        }
        let joined: Vec<String> = self.items.iter().map(|r| r.to_string()).collect();
        write!(f, "{} refusal(s): {}", self.items.len(), joined.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_vocabulary_is_closed_and_every_name_round_trips() {
        assert_eq!(RefusalKind::ALL.len(), RefusalKind::COUNT);
        let mut seen = std::collections::BTreeSet::new();
        for (index, kind) in RefusalKind::ALL.iter().enumerate() {
            assert!(
                RefusalKind::ALL[..index].iter().all(|other| other != kind),
                "{} is declared twice",
                kind.name()
            );
            assert!(seen.insert(kind.name()), "{} names something else too", kind.name());
            assert_eq!(RefusalKind::parse(kind.name()), Some(*kind));
            assert!(
                kind.name().bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
                "{} is not a wire-shaped name",
                kind.name()
            );
        }
        assert!(RefusalKind::parse("no_such_refusal").is_none());
    }

    #[test]
    fn a_refusal_set_collects_every_problem_not_the_first() {
        let mut set = RefusalSet::new();
        set.refuse(RefusalKind::SubvoxelOutOfRange, "k = 9");
        set.refuse(RefusalKind::UnadmittedMaterial, "material 4242");
        set.refuse(RefusalKind::NonPositiveDensity, "0/0");
        assert_eq!(set.len(), 3, "all three must survive: {set}");
        assert!(set.contains(RefusalKind::UnadmittedMaterial));
        assert_eq!(set.first_named().unwrap().kind(), RefusalKind::SubvoxelOutOfRange);

        let err = set.into_result(()).unwrap_err();
        assert_eq!(err.len(), 3);
        assert!(err.to_string().contains("subvoxel_out_of_range"));
        assert!(err.to_string().contains("unadmitted_material"));
    }

    #[test]
    fn an_empty_set_is_a_pass_and_a_name_is_stable() {
        assert!(RefusalSet::new().into_result(7).is_ok());
        assert!(RefusalSet::new().is_empty());
        assert_eq!(
            RefusalKind::ReflectanceOutOfRange.name(),
            "reflectance_out_of_range"
        );
        // The detail is carried, not dropped.
        let r = Refusal::new(RefusalKind::ZeroPeriod, "period = 0 s");
        assert_eq!(r.detail(), "period = 0 s");
        assert_eq!(r.to_string(), "zero_period: period = 0 s");
    }
}
