//! **What the other projects' READMEs declare, filed as tickets against this one.**
//!
//! # Why these documents, and why they are not authority
//!
//! The rows below were chosen because the user named these projects as the reference set
//! — *"read the readme files of my projects in f documents for features that are missing so
//! you can file tickets for them"*, and then the second wave by name — *"bspt, minidoc, blrp,
//! openlock-legacy"*. **They are not the design record.** `docs/HYPERGRILLING.md`
//! and `DESIGN-SPEC.md` remain the authority for this tree; a sibling project can *declare* a
//! mechanism, and a ticket can only say *"that mechanism is absent here"*. Where a sibling
//! contradicts the record, the record wins and nothing here overrides it.
//!
//! **And two rows cite the record itself, because the record had already ruled.** `BLD-0014`
//! and `BLD-0016` were first filed against `blrp` and `AWA3D` — "you must produce something
//! playable", "every failure names itself on screen" — and the user pointed out what
//! `HYPERGRILLING.md` **Round 7** already says: the dirty branch's cuboid overlay becomes a
//! **vector debug view** of change within a chunk, and **H-7.5 rules that a view is a data
//! record and one generic renderer draws any declared view**. A sibling README cannot be the
//! declaring document for a gap the record itself ruled on, so the two rows now cite the
//! record and keep the siblings as corroboration.
//!
//! # Why `document` is a field
//!
//! [`Work::because`] is prose, and prose can be wrong in a way nobody can check. The
//! `document` field is the part that *can* be checked: every row names the file it was read
//! from, and this test asks the host whether that file is there. On the machine that wrote
//! these rows it is; on a host without `F:/Documents` the check says so and continues rather
//! than reporting a pass it did not earn — the same rule `filing.rs` follows for an
//! unprivileged symlink.
//!
//! # Why the ids are not asserted
//!
//! `Board::from_chain` numbers a ticket by its **position in the chain**, so hard-coding
//! `BLD-0008` here would make inserting a row above it a test failure. The count, the kinds
//! and the markers are asserted; the ids are printed.

use std::path::Path;

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::{Board, TicketKind};
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::filing;

/// **Where a cited document is, resolved.** A row cites either a file under the reference root
/// or a document in this tree, and the two resolve differently: an absolute reference path is
/// already absolute, and an in-tree one is relative to the **workspace**, while an integration
/// test's working directory is the crate. Resolving `docs/HYPERGRILLING.md` against the crate
/// would report a document that exists as missing.
fn declaring_document(cited: &str) -> std::path::PathBuf {
    if cited.starts_with("F:/Documents/") {
        std::path::PathBuf::from(cited)
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(cited)
    }
}

/// The namespace every filed ticket belongs to.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// One piece of work, **with the document that declares it.**
struct Work {
    kind: TicketKind,
    /// The file this row was read from. Absolute, because it is outside this tree.
    document: &'static str,
    objective: &'static str,
    because: &'static str,
}

/// **The gaps, in the order the chain will hold them.**
///
/// The order is deliberate: the record-integrity rows first (they decide what the other ten
/// are allowed to claim), then the ones a user can see.
const WORK: [Work; 27] = [
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/ala-system/README.md",
        objective: "[BLD] check a feature by evaluating its witness, not by asking whether the \
                    witness file exists",
        because: "ala-system README, 'Three database commands, three different questions' — \
                  db:verify confirms the predicate text is installed and 'only db:check confirms \
                  it evaluates: a validator written in the wrong dialect verifies clean and \
                  enforces nothing', with 15 probes matched by error code so a probe cannot pass \
                  for the wrong reason; features.rs:467 asks `witness.is_file()` and nothing \
                  else, so a row reads active when its witness exists and the feature does not, \
                  which is what the file's own note means by 'checked against nothing'",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/income-engine/README.md",
        objective: "[BLD] refuse a ticket whose evidence names nothing that exists",
        because: "income-engine README rule 1 — 'A dollar is not income until a receipt cites \
                  it. ledger add without --evidence is refused. Evidence naming a local file \
                  that does not exist is refused. Model output is never a receipt.'; \
                  ticket.rs::Evidence::new refuses an empty id and an empty detail and never asks \
                  whether the thing named is there, so evidence citing any path at all is \
                  accepted",
    },
    Work {
        kind: TicketKind::Intervention,
        document: "F:/Documents/income-engine/README.md",
        objective: "[INT] rule which actor may verify, so a self-report cannot be the thing that \
                    closes a ticket",
        because: "income-engine README rule 2 — 'An agent cannot verify money and cannot clear a \
                  gate. Anyone calling with --source agent:<name> is refused at both \
                  chokepoints. A model cannot certify its own output.'; ala-system README says \
                  the same ('support_level is computed from the evidence... an asserted strength \
                  is discarded, never reviewed'); envelope.rs:342 has Actor refusing only an \
                  empty name, and a grep for \"agent\" across crates/ finds no chokepoint that \
                  asks who is acting before it accepts the act",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/wayfinder-hand/README.md",
        objective: "[BLD] make every shell operation answer with what actually changed, measured",
        because: "wayfinder-hand README — action verbs 'are driven one client tick at a time, \
                  measure whether the world actually changed, and try the next strategy when it \
                  did not', answer with 'a measured effect block', and name the strategy used in \
                  effect.strategies, under the rule 'No self-reported success. ok means the \
                  client did what was asked; only the server's measured state decides whether the \
                  task happened'; Arrangement::split returns UiResult<WindowId> and close returns \
                  Option<Arrangement>, so which splitters collapsed, what the spans became, and \
                  whether the floor still holds are all unobservable to the caller",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/AWA3D/README.md",
        objective: "[BLD] name the client's own failure and its own build id where the user can \
                    see them",
        because: "AWA3D README, 'When the canvas is empty' — the pill shows the running build id \
                  and is compared against client/dist/build.json so a stale bundle says so, the \
                  #fatal bar is 'the client's own diagnosis: no renderer, no frame after 12 s, \
                  the render loop dying, or app.js never running at all', and a static watchdog \
                  in index.html catches that last one 'even when the bundle fails to parse'; \
                  main.rs:397 and main.rs:405 eprintln! and exit, so a machine with no adapter is \
                  named only in a console the player may not have open — as are the other eight \
                  refusals the binary prints — and a grep for a build id across voxel-client and \
                  shploingus finds nothing",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/ala-system/README.md",
        objective: "[BLD] make the chain writer refuse a path outside the workspace it was given",
        because: "ala-system README, 'A prompt-injected model still cannot exfiltrate' — 'The \
                  tools that would let it do not exist... Tool scarcity is the actual boundary', \
                  and its client 'refuses a non-loopback host' rather than warning about one; \
                  filing::file takes any &Path and writes it, and the module's own note says \
                  pointing it at docs/LEDGER.md destroys nothing because of the overwrite \
                  refusal — not because the path is out of the writer's reach",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/skills_directory/user-flow-specs/README.md",
        objective: "[BLD] give a finding its class — fatal, advisory, context — and let only \
                    fatal stop the build",
        because: "user-flow-specs README, 'The linter' — 'Findings are classified as: Issues \
                  (exit code 1) — structural problems to fix; Warnings — design issues to \
                  consider (not commands); Info — context, never action-forcing'; refusal.rs has \
                  one RefusalKind vocabulary in which every variant is equally fatal, so severity \
                  here is expressed by which function a caller chose (check_required vs \
                  check_features) and the finding's own record cannot say whether it stops the \
                  build",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/skills_directory/user-flow-specs/README.md",
        objective: "[BLD] make every window declare its blank, loading, partial and error states, \
                    not only its ideal one",
        because: "user-flow-specs README, 'The UI Stack' — 'Every key screen gets five states \
                  considered: Ideal, Blank/Empty, Loading, Partial, Error', with the linter \
                  catching 'happy-path-only thinking (no error/empty/loading states considered)'; \
                  voxel-ui has one five-variant State {Normal, Hover, Pressed, Disabled, Empty} \
                  that Readout shares with controls (lib.rs:1078), so a reading the instrument \
                  cannot give and a hovered control are the same word, and nothing declares what \
                  a window shows while a chunk is streaming or after the stream has failed",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/ala-system/README.md",
        objective: "[BLD] carry the convicted terms out of the lint, so a refusal names what it \
                    refused",
        because: "ala-system README — 'A withheld line names the rule that stopped it'; \
                  oracle.rs splits culprits() out of Verdict precisely so that 'a refusal names \
                  what it refused' is a promise a caller can keep rather than one only the \
                  module's own tests can keep, and culprits( is called in exactly one place in \
                  the tree — its own test at oracle.rs:1351 — so the gate reads \
                  Verdict::NotBelongs and never the term",
    },
    Work {
        kind: TicketKind::Intervention,
        document: "F:/Documents/income-engine/README.md",
        objective: "[INT] rule how an act this system will never take becomes a gate naming the \
                    human step",
        because: "income-engine README rule 3 — five actions are never granted under any policy \
                  (create_account, enter_credentials, move_money, accept_terms, sign_contract), \
                  and 'A task that needs one is not silently skipped — it becomes a gate for a \
                  human'; ticket.rs::Gate { statement, expectation } already exists for exactly \
                  this, and Board::from_chain folds only TicketTransition and Retirement \
                  (ticket.rs:900), so a gate is in no view the board can produce",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/income-engine/README.md",
        objective: "[BLD] prove the invariants in one command, naming each check it proved",
        because: "income-engine README — 'python3 income_core.py selftest # 19 invariant checks', \
                  'python3 cua_worker.py selftest # 18 policy/coordinator checks', and what \
                  carried over from AWSO includes 'a selftest that proves the invariants'; \
                  crates/shploingus/src holds only main.rs, an 834-line winit event loop with no \
                  argument parsing (grep for env::args finds none), so this tree's invariants are \
                  reachable only by running a suite that names nothing it proved",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/AWA3D/README.md",
        objective: "[BLD] collapse a repeated escalation to one ticket by fingerprint",
        because: "AWA3D README lists doom.py, 'repeated-text detector the guard uses'; \
                  income-engine README records what carried over from AWSO — 'fingerprint-deduped \
                  escalation'; envelope.rs has EventType::ChildEscalation (9 of them in the \
                  record at C-77) and nothing that collapses a repeat, and a grep for \
                  fingerprint across crates/ finds no such mechanism",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/bspt/docs/MSW — the kernel.md",
        objective: "[BLD] state a ticket's gate when it is filed, rather than letting the reader \
                    copy the objective into it",
        because: "bspt/docs/MSW — the kernel.md — 'contract ← the requested outcome + the \
                  smallest criteria that prove it, stated before any work. The sole source of \
                  necessity; a ceiling as much as a floor.'; the refusal that rule implies is \
                  already in this tree — Gate::new refuses a gate that states nothing, with the \
                  reason 'Stating the bar afterwards is how a ticket gets declared done by \
                  whoever moves last' — and ticket.rs:944 builds the gate as a struct literal, \
                  `Gate { statement: objective, expectation: Expectation::None }`, so the one \
                  place that must obey the constructor bypasses it and the world is required to \
                  show nothing",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/bspt/docs/MSW — the kernel.md",
        objective: "[BLD] make a declared limit carry the authority or derivation that admits \
                    it",
        because: "bspt/docs/MSW — the kernel.md, 'No unauthoritative limits' — 'Never invent a \
                  limit. A cap, threshold, quota, budget, timeout, retry or round count, file or \
                  line count, acceptance-criterion count, agent count, or similar constraint is \
                  admissible only when its exact value is explicitly required by the requester, \
                  imposed by an applicable technical or platform contract, defined by \
                  authoritative project policy, or derived from measured evidence... State the \
                  authority or derivation whenever proposing or applying a limit'; the rule is \
                  practised in prose in three places — envelope.rs:93 derives MAX_FIELD_BYTES \
                  from 'roughly one paragraph', arrangement.rs derives MIN_EXTENT from the \
                  reference's minimumSizeHint, pool.rs:54 derives DEFAULT_QUEUE_CAPACITY from a \
                  measured ~1.3 kB per finished mesh — and no type carries it, so the next limit \
                  is a bare number and RefusalKind has no variant that could refuse one",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] prove a frame was drawn, or have the gate say it did not",
        because: "HYPERGRILLING.md Round 7 is the record's own answer to how this game is proved \
                  — H-7.1 answers 'what a really nice cuboid overlay ... the dirty branch \
                  already [has]' by ruling a vector view of the areas that change within a \
                  chunk, and HYPERGRILLING.md:770 lists 'the vector change view over a chunk' in \
                  the shploingus-clean column of the provenance table, which is the record saying \
                  this tree does not have it; nothing here draws a proof: all four tests that \
                  reach a GPU adapter are ignored — the_gpu_shades_a_surface_exactly_as_the_crate_does \
                  and the_shipped_view_is_not_a_black_screen and \
                  a_frame_of_the_world_passes_the_bars_band_count (renderer.rs:280, :404, :564) \
                  and the_hud_draws_legible_text_over_the_worst_backdrop (hud.rs:868), each \
                  'needs a GPU adapter; run explicitly with --ignored' — so all 828 that pass can \
                  pass on a build whose frame path never ran, and the one gate (H-10.1) is a build \
                  script that never mentions wgpu, so it cannot ask for a frame either; blrp/goal.md \
                  states the same acceptance line in other words ('You must produce something \
                  playable') and AWA3D shows the shape of the check ('node tools/probe.mjs render \
                  # boot + renderer verdict', with 'no frame after 12 s' as a named on-screen \
                  diagnosis)",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] build the declared view record and the one generic renderer behind it \
                    (H-7.5), so `change_view`'s witness exists",
        because: "**Corrected: this row first claimed `features.rs` has no row for the debug \
                  views, and that was false.** The gate prints `feature change_view unchecked no \
                  witness at crates/voxel-client/src/shell/changes.rs` and the oracle repeats it \
                  as a refusal (alpha-shell.gate lines 28 and 47), so the row exists, is named, and \
                  already reports itself unchecked; **second correction, 2026-10-03: the witness \
                  file now exists — the vector change view landed in it — and the gate reads \
                  `feature change_view active witness present at crates/voxel-client/src/shell/changes.rs`, \
                  but the witness is hand-written Rust, which is exactly what H-7.5 forbids a view \
                  to be, so this ticket's machinery gap is not closed by it.** **The gap is the \
                  machinery behind it**, which \
                  HYPERGRILLING.md Round 7 rules: H-7.5 answers 'declared data or hand-written \
                  code' with **All declared** — 'a view is a data record — what it selects, which \
                  stages/shasers it reads, its labels and its lane class — and one generic renderer \
                  draws any declared view. Adding a view is data, never Rust' — on the precedent of \
                  the recipe → package route and this tree's own voxel-package intake; H-7.7 ports \
                  the lane system (six classes, the frozen vocabulary and its digest, labels at the \
                  lane's own position); and H-7.1 makes the view a vector outline of the areas that \
                  change within a chunk, marking when a chunk's changes are saved",
    },
    Work {
        kind: TicketKind::Build,
        document: "F:/Documents/simulators/portal_simulator.py",
        objective: "[BLD] check the claim that the tree's vendored reference is byte-identical \
                    to the file it was copied from",
        because: "simulators/portal_simulator.py is the file arrangement.rs:21 says is in this \
                  tree 'byte-identical to' it, which is a claim in prose that nothing evaluates; \
                  provenance.rs:66 includes the copy in the working-tree digest, so a change to \
                  the copy is detectable and drift from the original is not; and openlock-legacy \
                  holds abilities.vdata beside 'abilities - Copy.vdata' and abilities_backup.vdata \
                  — the directory a reader cannot tell the record in, which is the hazard \
                  bloat::read_chain already refuses for chains, because 'two files are two \
                  chains and merging them would invent a linkage that neither file claims'",
    },
    // **Asked for by the user, and appended rather than inserted.** The order is the chain's, so
    // putting this beside the other output row would have renumbered two tickets that are already
    // cited by id in `LEDGER.md` C-82 — and an id is a position in an append-only record, which is
    // the reason nothing here is inserted in the middle.
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] make the board's tickets appear in a default `cargo test` run, not only \
                    under a flag",
        because: "HYPERGRILLING.md H-8.1 — 'One append-only record, two views... docs/LEDGER.md \
                  and the in-game ledger window are two projections of one record'; the filer \
                  prints every ticket with its declaring document (file_reference_gaps.rs:399 \
                  onward), and libtest discards a passing test's stdout, so a default `cargo \
                  test` prints '1 passed' and the board is visible only to a reader who already \
                  knows --nocapture or --show-output — measured: the default run shows nothing, \
                  and --show-output prints '# the board, read off the chain'; the gate already \
                  writes its own findings to a report under OUT_DIR, so one projection of the \
                  record reaches a file and this one reaches nothing; no test asserts this and \
                  none can, because the print is unconditional in the source and the harness is \
                  what discards it",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] build the shell console window and the log window, so a test run is \
                    visible where the game is",
        because: "HYPERGRILLING.md H-7.17 asks how 'every cargo test actually visible' works and \
                  answers it: 'Special debug shell console window that comes from \
                  desktop_simulator.py rust deriviation for the shell view, plus a log window' — a \
                  Rust derivation of the reference's ConsoleWidget (command input, execute, output, \
                  append_system_message, render_logs, an event log fed by on_event_logged) **plus a \
                  log window**, with 'the console is where a test run becomes visible in the client \
                  and where commands are given'; H-7.13 gives the reason in the user's words — \
                  'every test ran in game using the debug lane, every cargo test actually visible, \
                  so missing features are known by the client itself' — so the client is 'where the \
                  suite's own state is readable'; a grep for console in features.rs finds no row \
                  at all — the gate checks witness rows in earnest (change_view read \
                  `unchecked no witness` at crates/voxel-client/src/shell/changes.rs until the \
                  vector change view landed on 2026-10-03, when the row flipped to `active witness \
                  present`) — and the console is \
                  also the eleven-verb surface task_viewer_and_console.md declares (help launch \
                  list focus close min max split theme clear echo stats)",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] make a test result a record the client can read, not a line in a \
                    terminal",
        because: "HYPERGRILLING.md L-4's D98 — 'a test is a record, and it has a representation in \
                  the client... A test lives in the workspace, its result is a record with a stable \
                  shape and name, and it is consumable by the sensor primitive', and 'a finding \
                  that can only be read in a terminal has no client representation and is therefore \
                  incomplete'; the gate already convicts this tree against C-17 — \
                  `crate-gap shploingus axis=tested state=inactive ... C-17 holds the whole client \
                  to 'every cargo test actually visible', and a crate with no test cannot show that \
                  it was looked at` — and 828 passing tests are readable only in the terminal that \
                  ran them, which is exactly the incompleteness D98 names",
    },
    Work {
        kind: TicketKind::Intervention,
        document: "docs/HYPERGRILLING.md",
        objective: "[INT] rule Laya's scope in this tree's own terms: the decision layer over the \
                    framework, not an oracle behind a trait",
        because: "the record reduced Laya to one seam and the user's correction says that is the \
                  whole mistake: 'the older agent made laya not the oracle, but laya is the entire \
                  decision layer across the entire framwork includin the game's systems, and the \
                  compiler itself - so it should be using the full axis swuite, all of the \
                  features, making the game compile it's recipe as a package'. HYPERGRILLING.md \
                  L-4 measures Laya's *role* as D3's conformance linter and its authority note says \
                  'Nothing here may be cited as a ruling in docs/LEDGER.md; the items that survive \
                  become this tree's decisions' — so the scope change is this tree's decision to \
                  make, and today the gate answers it the other way: `oracle-kind \
                  structural-deterministic` and `oracle-is-laya no` (alpha-shell.gate lines 36-37), \
                  with `model-oracle not-evaluated` at line 161",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] put every decision through one seam: the five axes, the nineteen feature \
                    rows, and the recipe's compile and load",
        because: "the same correction — Laya 'should be using the full axis swuite, all of the \
                  features' — and today each of those answers with its own code: census.rs decides \
                  the five axes itself (Axis::{Named, Moduled, Tested, Refusing, Claimed}, \
                  AXIS_COUNT = 5) and says of Claimed that it replaced an axis that 'could not fail' \
                  because it was graded against the grader's own prose; features.rs decides every row \
                  by `witness.is_file()` and its own note says its means text is 'checked against \
                  nothing'; recipe.rs::compile and ::load decide by parsed rules; and oracle.rs is a \
                  Rust StructuralJudge behind `Oracle`, which is the one seam the gate reports as \
                  `oracle-is-laya no` — four decision surfaces with four vocabularies, where the \
                  ruling asks for one layer, and HYPERGRILLING.md's H-10.1 is why the shape matters: \
                  'the build must not be capable of forgetting'",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] make the game compile its own recipe as a package, in the game, rather \
                    than receiving one a build script compiled",
        because: "the correction's third clause — 'making the game compile it's recipe as a \
                  package' — and today the compile happens in the build script: build.rs:47 calls \
                  voxel_package::recipe::compile on packages/alpha-shell.shspec and build.rs:342 \
                  writes alpha-shell.shpkg into OUT_DIR, which main.rs:142 then \
                  include_bytes!es, so **the game receives a package it did not compile**; \
                  HYPERGRILLING.md H-10.1 rules the opposite shape — 'the game should not compile \
                  it if it does not conform... A package that goes through the system validates \
                  its place in the game' and 'one clean execution pass, proving itself' — so the \
                  pass H-10.1 describes is not the pass this tree runs: here the refusal is a \
                  build-script panic (build.rs:47) and the shipped game's only decision about its \
                  own recipe is whether the bytes it was compiled with still parse (main.rs:148)",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] move the decisions out of the measurement functions: census and \
                    features report what they read, and the seam decides what it means",
        because: "HYPERGRILLING.md H-14.1 rules the decision layer as the thing that decides, and \
                  the first cut of `decision.rs` **fronts** the decisions instead: `Deterministic` \
                  answers by calling census::census and features::evaluate, and those functions \
                  compute the states themselves — `features::evaluate` decides every row by \
                  joining a witness path and asking `is_file()`, and census's axes decide in their \
                  own code. So the rule lives in the reader, not in the layer, and a second \
                  instrument implementing the same seam cannot be handed the same observations to \
                  decide from. The seam is one pass over 80 subjects and the gate prints it — \
                  `decider deterministic / subjects 80 / subject-active 61 / subject-inactive 12 / \
                  subject-unchecked 7 / subject-refused 0` — and what remains is the **inversion**: \
                  the readers report, the layer decides",
    },
    Work {
        kind: TicketKind::Intervention,
        document: "docs/HYPERGRILLING.md",
        objective: "[INT] rule what implements the seam's model instrument, and which subjects it \
                    may decide",
        because: "HYPERGRILLING.md H-14.1 makes the oracle one implementation of the decision \
                  seam, and the record has two answers about what that implementation is: L-4's \
                  D76 — 'the linter laya model is a requirement of the system and we display the \
                  api in the ui with an api viewer' — and L-4's D97, where 'the judge is Rust' \
                  in-project with the oracle's *role* unchanged, which is that tree's ruling and \
                  not this one's; L-4's authority note says exactly that, so this tree must decide, \
                  and the gate currently reports the question rather than answering it: \
                  `model-oracle not-evaluated` (alpha-shell.gate, 'what this gate is not'), with \
                  oracle.rs already holding the calibration machinery (calibrate, KNOWN_ANSWERS, \
                  quotable) that any answer will need",
    },
    // **H-14.2 answered this one, so it is filed as build work rather than as a ruling owed.**
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] build the Laya Rust model instrument as the seam's second decider, so \
                    pass two decides instead of refusing",
        because: "HYPERGRILLING.md H-14.2 — 'a two pass parser that runs both the rust system and \
                  the laya rust model through it', and 'the rust suite runs through laya' — so the \
                  model is an in-project Rust `Decider`, not an out-of-process service, and it may \
                  decide every subject with the calibration guard travelling with each one; the \
                  gate currently prints the absence honestly and completely — `model-decider \
                  unreachable / model-decided 0 / model-unreached 80 / divergences 0` — and \
                  oracle.rs already holds what the instrument must satisfy (calibrate, \
                  KNOWN_ANSWERS, the majority-class bar, quotable, the unreasoned_override rule) so \
                  the work is an implementation against a measured bar rather than a new design",
    },
    Work {
        kind: TicketKind::Build,
        document: "docs/HYPERGRILLING.md",
        objective: "[BLD] grade every subject by axis in the client, from the two passes, rather \
                    than in a gate report a player cannot open",
        because: "HYPERGRILLING.md H-14.2's own clause — 'so everything in the project actually has \
                  a visible grading according to axis in the ui' — and H-7.13's rule that the \
                  client is where the suite's own state is readable; the two passes now exist and \
                  are written into alpha-shell.gate, which main.rs:143 embeds as PACKAGE_GATE **and \
                  nothing reads into a window**: features.rs has no console row, the gate's \
                  witness rows — change_view reads `active witness present` since 2026-10-03 — \
                  reach only the gate file, and \
                  H-7.17's console plus log window (`BLD-0018`) is the surface that would show it",
    },
];

#[test]
fn the_reference_gaps_are_filed_as_tickets_and_read_back_off_the_board() {
    let mut workspace = std::env::temp_dir();
    workspace.push(format!("shploingus-file-reference-gaps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&workspace);

    let namespace = namespace();
    let records: Vec<UnlinkedRecord<'_>> = WORK
        .iter()
        .map(|entry| UnlinkedRecord {
            namespace: &namespace,
            event_kind: EventKind::TicketTransition,
            question: entry.objective,
            answer: None,
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "user",
            round_id: "c-2026-10-01",
            supersedes: None,
        })
        .collect();

    // **Through the chokepoint**, so the tickets form a hash-linked chain rather than a run of
    // records that each deny knowing the one before.
    let bytes = filing::linked_records(&workspace, &[], records).expect("links");
    filing::append(&workspace, &bytes).expect("files");

    let chain = filing::read(&workspace);
    let board = Board::from_chain(&chain);

    assert_eq!(chain.len(), WORK.len(), "one event per piece of work");
    assert_eq!(board.len(), WORK.len(), "and one ticket per event on the board");
    assert!(
        chain.tamper_evident_description().starts_with("hash-linked"),
        "**the tickets are a tamper-evident record**, not a list that can be edited: {}",
        chain.tamper_evident_description()
    );

    // **Every kind used is a real kind**, so a typo in `[BLD]` cannot invent a prefix — the
    // board would fall back to `Build` and the objective would silently lose its marker.
    for entry in &WORK {
        let marker = format!("[{}]", entry.kind.prefix());
        assert!(
            entry.objective.starts_with(&marker),
            "{} does not open with {marker:?}, so `split_kind` would file the kind as Build and \
             leave the marker in the objective",
            entry.objective
        );
        assert!(
            entry.objective.trim_start_matches(&format!("[{}] ", entry.kind.prefix())).len() > 20,
            "{} has no substance after its marker",
            entry.objective
        );
        // **The declaring document has to be findable from the board alone.** A row cites either
        // a file under the reference root or a document in this tree, and `because` must name it:
        // the project directory for a reference file — "README.md" names nothing on a machine with
        // eleven of them — and the file name for one of this tree's own, because "docs/" names
        // nothing on a machine with one of those in every project.
        let named = if let Some(after_root) = entry.document.strip_prefix("F:/Documents/") {
            let project = after_root.split('/').next().expect("a first component");
            let holder = entry
                .document
                .rsplit_once('/')
                .and_then(|(dir, _)| dir.rsplit('/').next())
                .expect("a document path with a directory in it");
            entry.because.contains(project) || entry.because.contains(holder)
        } else {
            entry
                .because
                .contains(entry.document.rsplit('/').next().expect("a file name"))
        };
        assert!(
            named,
            "{} cites {} but `because` never names it, so a reader of the board cannot find the \
             document that declares the gap",
            entry.objective,
            entry.document
        );
    }

    // **The declaring documents, asked about rather than assumed.** `F:/Documents` is where the
    // sibling projects live; on a host without them the check says which part it could not do.
    let host_has_documents = Path::new("F:/Documents").is_dir();
    println!("\n# the board, read off the chain\n");
    let mut by_kind: Vec<(TicketKind, usize)> = Vec::new();
    for ticket in board.tickets() {
        if !by_kind.iter().any(|(kind, _)| *kind == ticket.kind) {
            by_kind.push((ticket.kind, 0));
        }
        by_kind.iter_mut().find(|(kind, _)| *kind == ticket.kind).expect("just added").1 += 1;
    }
    for (kind, count) in &by_kind {
        println!("{} filed\t{}", kind.name(), count);
    }
    println!();
    for (index, ticket) in board.tickets().iter().enumerate() {
        let work = &WORK[index];
        let resolved = declaring_document(work.document);
        let declared = if resolved.is_file() {
            "found".to_string()
        } else if work.document.starts_with("F:/Documents/") && !host_has_documents {
            "not checkable on this host".to_string()
        } else {
            format!("**MISSING at {}**", resolved.display())
        };
        println!("{}\t{}", ticket.id.as_str(), ticket.objective);
        println!("\tdeclared by: {} ({declared})", work.document);
        println!("\tbecause: {}\n", work.because);
    }
    for entry in &WORK {
        // **A reference document is asked about only where the reference root is**, and a
        // document in this tree is always asked about — the asymmetry is the point, because
        // only one of the two can be missing for a reason this host is not responsible for.
        if entry.document.starts_with("F:/Documents/") && !host_has_documents {
            continue;
        }
        let resolved = declaring_document(entry.document);
        assert!(
            resolved.is_file(),
            "{} cites {} as its declaring document and there is nothing at {}",
            entry.objective,
            entry.document,
            resolved.display()
        );
    }
    if !host_has_documents {
        println!(
            "F:/Documents is not on this host, so its {} declaring documents were not checked and \
             this run does not claim they exist\n",
            WORK.iter().filter(|w| w.document.starts_with("F:/Documents/")).count()
        );
    }

    let _ = std::fs::remove_dir_all(&workspace);
}
