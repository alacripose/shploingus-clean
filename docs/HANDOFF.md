# Alpha handoff — read this first

Updated: 2026-10-02 (the body below is the 2026-09-30 handoff and is owed a compaction; the
authority list is current).
Goal: **ship-ready v1 alpha**, not published/deployed.
This checkout on `main` is correct, confirmed by the user. Preserve existing work.

## Authority and standing constraints

- User rulings and this tree's ledger win. Outside projects are reference for
  reasons/mechanisms, **not authority** (C-48/C-49). Do not import from them.
- `docs/LEDGER.md` entries **C-56 … C-65** are the current record. **C-59** holds the user's
  six rulings, **C-62** the four more, **C-63/C-64** what they produced, and **C-65** the record
  compression and the workspace-wide crate census.
- **`docs/HANDOFF.md` is explicitly NOT authority** (`record::DocumentRole::Derived`). It points
  at the record; it does not extend it. A feature citing an anchor that exists only here is
  **refused by the build gate**.
- Latest `docs/HYPERGRILLING.md` answers supersede stale TODO/UI open-question lists.
- **The current record's tail is C-84 … C-100.** **C-100** is the agent's inbox — the recipes the
  game derives from the solutions, written by laya through the notification stream, read by
  `NotificationStream::open_at`, and handed over by `mount::hand_the_consoles_filings_to_the_agent`
  at world-open. **C-99** is the game's own judgement at world-open and the process correction at
  §99.4. Read C-99.4 and C-100.4 before acting on either.
- Don't reopen settled decisions; don't replace the requested system with a smaller demo.
- Never test the client invisibly. Offscreen measurement is not visible-client acceptance.
- Run tests **exactly as the docs write them**, no output filtering, no `| grep` (C-45).
- **Refuse, never default.** No clamping, no guessing. `RefusalSet` collects *all* refusals.
- Appearance is user-owned. depth: raised = lighter, `ELEV_STEP` 0.05, levels 0..4,
  `RADIUS_DP` 2, no adopted shadow/blur.
- Seasons mean literal winter/spring/summer/autumn, not campaigns.
- Windows are mounted data (H-9.7). HUD = pinned shell windows (H-9.8). The shell owns the
  toast queue (H-12.2). First loaded package = the shell arrangement (H-11.5/C-54.2).
- One namespaced append-only event hash chain, one writer, two views (H-8.1). 20 Hz base clock
  with declared stage cadences (H-8.3).
- **The five-part composition** is `recipe + résumé + civilian + governance + security`
  (H-7.12, `HYPERGRILLING.md:1520`) and is rendered as the **API view in the right pane of the
  Mods window** (R-62.1) — readable elements, navigation per the design rulings.
- **The documents are the declaration** (R-62.2). There is no second copy to drift. **This is now
  mechanical**: `compendium.rs` compresses the ten documents into `shploingus.compendium/1`, and
  `Compendium::derive` builds a judgeable recipe from a declared list of anchors. Nothing is
  hand-transcribed, because a transcription is a second copy.
- High agency is per-invocation; a successor must **not** assume it is inherited.

## What was built — seven blocks, all in `crates/voxel-package/src/`

Each is a **compile-time-enforced** evaluator, in the pattern taken from the reference's
`axis.rs`: a declared item with no entry is a **compile error**, not a red test.

| file | what it does | the rule it enforces |
|---|---|---|
| `signing.rs` | `Signer`/`Verifier`/`KeyId`/`Signature`/`NoSigningBackend` | names **no cryptography**; a build without `voxel-envelope` refuses every package, and there is no feature flag to flip |
| `features.rs` | 19 declared features, `witness_of` (exhaustive match), `FEATURES_ARE_TOTAL` (const pin), `check_table_shape`, `check_features`, `check_required`, `evaluate` | a feature with no witness is a **compile error**; a missing one is **displayed by name** |
| `oracle.rs` | `Verdict` (closed 3-word), `Calibration`, `Oracle`, `UnreachableOracle`, `StructuralJudge`, `calibrate`, 53 known answers | the lint gate **fails closed**; the bar is the **majority class** |
| `policy.rs` | 23 closed actions, exhaustive `permission_of`, `Grant` refused at construction, `decide`, `check_matrix` | **deny by default** — silence is not permission; a missing matrix entry is a **compile error** |
| `record.rs` | 10 documents with roles, `check_declarations`, per-document digests | every feature's `declared_by` anchor must exist **in the real text of an authoritative document** |
| `compendium.rs` | the record compressed: `Series` (closed 4), `Anchor`, `Occurrence`, `Compendium::derive`, `open_questions` | the documents **become** the recipe; a body is **verbatim**, a missing anchor is a **refusal**, `L-` research **cannot license** without a recorded override |
| `census.rs` | all 12 crates × 5 axes, `witness_of` (exhaustive match), `CRATES_ARE_TOTAL` (const pin), `check_coverage` | a crate with no witness is a **compile error**; an **unmeasured cell is refused**, so the gate cannot grade only the new work |

Also added: 15 new `RefusalKind` variants in `crates/voxel-model/src/refusal.rs` — deliberately
**several kinds and not one**, because integrity vs authenticity vs "cannot check" vs "checked,
key unknown" are four different facts.

### The one shared idea

`features::Establishment` has four states — `Active` / `Inactive` / `Unchecked` / `Refused` —
and is used for **both** "is this feature implemented?" and "was this signature checked?". They are
not two ideas that look alike; they are one question asked of two subjects: *was this checked, and
what was found?* Collapsing `Unchecked` into `Active` is how an unverified package looks verified.

`policy::PermissionClass` and `features::Feature::required_for_alpha` are the same idea applied
to cadence: a pending feature is **displayed**, a required one **fails the build**. Both visible;
only one is fatal. Without that split every unfinished feature would block every build, and a gate
that blocks everything gates nothing.

## Evidence — observed, not inferred

```text
cargo test --offline -p voxel-package                   → 148 passed; 0 failed
cargo clippy --offline -p voxel-package -p shploingus --all-targets -- -D warnings → clean
cargo check --offline --workspace                       → clean
cargo build --offline -p shploingus                     → clean, and the gate runs
```

Two printed reports you should actually read rather than trust a summary of:

* the **feature report** — `features::tests::the_missing_feature_report_is_printed_...` prints all
  19 features with witness state. Currently **10 of 19 have no witness**; all 6 *required* ones
  are present, so the build does not break.
* the **oracle calibration** — 53 known answers, **accuracy 1.0000, majority class 0.6226,
  inverted 0.0566**, `establishment: active`.

Earlier in the session: `cargo test` was 21 passed, then 30, 46, 69, 83, 84, 94, 109, now 148.

Two more printed reports, added this session and worth reading rather than trusting a summary of:

* **the compression** — `compendium::tests::the_compression_reports_what_it_found` prints the whole
  derived format: **107 occurrences, 104 distinct anchors, 3 written twice**, per-series counts, and
  a `record-sha256`. The gate prints the same plus all **27 open questions** with document and line.
* **the crate census** — `census::tests::the_census_prints_its_own_measurements` prints a 12×5 grid
  and every gap with its reason. **54 of 60 cells active.**

### Not verified — do not claim these

- **No signature has ever been produced or verified.** There is no trust root, no `voxel-envelope`
  crate, and no `.shpkg` envelope. `NoSigningBackend` is what every build currently gets.
- **The structural judge is not a Laya verdict.** It is deterministic and calibrated against
  **53** answers drawn from this project's own record. 53 is better than 13 and still a narrow set.
- **The recipe is compiled from the documents, but nothing yet checks that the compilation is a
  _faithful_ reading of them.** `Compendium::derive` builds a recipe from declared anchors and
  refuses an anchor the record does not contain — that is the *citation* half of R-62.2, done at the
  point of use. What is **absent** is the conformance half: no check yet asks whether a derived
  recipe omits a decision that bears on its subject. That is the honest remaining gap, and it is
  `oracle`'s question.
- **The shell is not mounted.** `main.rs` prints "declarations only — the window manager is not
  implemented, so nothing is drawn yet" and means it. The reference semantics are read and recorded
  in LEDGER C-65.4; **no window manager code exists.**
- **The census measures; it does not judge.** `census.rs` reports six gaps (`voxel-noise` and
  `voxel-ui` name no `RefusalKind`; `voxel-ui` and `shploingus` declare no `pub mod`; `shploingus`
  has no `#[test]`). It asserts none of them should be fixed — that is a question for the user.
- No visible client run, no alpha artifact, no release claim, no fresh independent review.

## Findings that will bite the next person

1. **`reading_within` can only widen the declared set, never narrow it.** A recipe that says
   `constant` has licensed `constant`. **To exclude a term, the recipe must not mention it.**
   Found by measurement; pinned by
   `a_recipe_can_widen_the_declared_set_but_never_narrow_it`.
2. **A numeric literal is a value, not vocabulary.** The judge once read `window.opacity(930);`
   as non-conformant and named `930`. Fixed by `is_literal`, and the test also asserts `opacity`
   is *still* held when unlicensed so the fix cannot widen into "the judge stopped convicting".
3. **`H-11.5`, `H-11.6` and `H-13.1` are each written TWICE in `docs/HYPERGRILLING.md`** — asked
   in one place, answered in another, both under the same number. **Any format keyed on the anchor
   string alone will collide.** This is why `Compendium::decrees_of` returns a `Vec` and not a
   slice. Measured before the format was written; see LEDGER C-65.1.
4. **Anchors appear in headings that CITE them, not just headings that DEFINE them.** 17 anchors
   are written in more than one heading, and in every case the extra heading *cites* the anchor in
   its own title (`### 51.3 … contradicted this ledger's own C-33/C-37`). **Only a heading whose
   first token is the anchor is a decision.** Reading the other 82 as decisions inflates the record
   by 82 and attributes rulings to the wrong entries.
5. **Four spaces of indent makes a `#` line a code block, not a heading** (CommonMark). The
   `docs/` tree has fenced blocks containing `#` comments; a parser that trims all leading
   whitespace compresses a shell transcript into a ruling.
6. **A one-line shell count measured FILES, not tests.** `voxel-ui` is a single 1,808-line `lib.rs`
   with 33 `#[test]`s, and a `for f in src/*.rs` loop reported it as untested. `census.rs` reads the
   tree properly and disagreed with a note written from the bad count. **Measure with a tool that
   reads the tree, never with a shell one-liner written while editing prose.**

## What the system is FOR, in one paragraph

The record is ~400 KB of prose across ten documents. It cannot be what an evaluator reads. So it is
**compressed into a derived format** (`compendium.rs`, `shploingus.compendium/1`): every numbered
decision becomes an `Occurrence` carrying its verbatim text, its document, its line and its digest.
That is `shploingus.compendium/1`: **104 anchors, 107 occurrences, 3 written twice, in ~2 KB of
structure.** `Compendium::derive` then assembles a **recipe** from a caller-declared list of anchors,
and that recipe is what `StructuralJudge` judges a line against. So the chain is: documents →
decisions → recipe → verdict, and nothing in it is hand-transcribed.

## Next task — do this, not another design questionnaire

1. **The window manager — built, and running (C-85).** `docs/portal_simulator.py`'s semantics from
   C-65.4 are implemented in `crates/voxel-client/src/shell/mount.rs`: the recursive pane tree,
   split-insert vs split-wrap, splitter cleanup with size restoration, the 40×40 floor, undock
   re-parenting, single focus, minimize-without-close, derived titles, and layout vs behaviour policy
   as separate types. `crates/voxel-client/src/shell/hud.rs` is the pinned layer over the same
   painter. `main.rs` no longer prints "declarations only"; it mounts `alpha-shell.shspec` and
   toggles the two layers on `Escape`/`F1`. **What is still missing is named in TODO §7.1**: the
   notification queue, the ticket window, the change view, and the debug lanes. The five console
   levels `SYS`/`EVENT`/`ACTION`/`WARN`/`INFO` are **not** implemented — the console window opens and
   says that its view does not exist yet.
2. **The debug lanes** (TODO §7.1 rows 1, 3, 7): six classes, a frozen vocabulary, labels drawn at
   the lane's own position, and **refuse rather than clamp**. The dirty tree's
   `crates/voxel-client/src/debug.rs` is a **reference, not authority** (C-48); the mechanism worth
   carrying is its refusal rule — *a debug overlay that silently drew off-screen, or a selection
   that silently shrank, would both be lies about what was selected.*
3. **The `voxelctl` CLI** (R-59.6): `seal`, `verify`, `provenance`, `features`, `lint`, `list`.
   A product caller, not a test and not a build script.
4. **The right-pane API view** from `docs/UI-SPEC.md` (R-62.1). Not yet designed.
5. **`voxel-envelope`** (B6, R-59.2): real ML-DSA-65, keypair generated once, public half pinned by
   digest, **no key material in the repository**.

Then the sealed container, and the mounted shell.

## Remaining alpha gates — none complete

- **Mounted shell — running; not finished.** The six tabs of H-15.2 (H-7.25's eight are superseded),
  split/dock/undock/min/max/close/focus, and the pinned layer are implemented and drawn over the
  running world (C-85, accepted by a visible run). **Still owed on this gate:** the notification
  queue, the ticket window, the change view, the console's five levels, and **keyboard/input
  isolation** — the shell takes the pointer while it is shown, and `Escape`/`F1` are the only keys
  it reads.
- **The crates that predate this work, on their own terms.** `census.rs` now measures all twelve, and
  the gaps are printed by the gate on every build: `voxel-noise` and `voxel-ui` name no
  `RefusalKind`; `voxel-ui` and `shploingus` declare no `pub mod`; **`shploingus` has no `#[test]`
  at all**, against C-17's *"every cargo test actually visible"*.
- Mods/API surfaces, with the honest `Establishment` state per package.
- One canonical durable event chain and derived views; save/load, fixed clock/cadences.
- Playable world: generation/shapers → mesh → renderer, vector change view.
- Cryptographic signatures: **absent**. Checksum/pinning is not a signature. `pqc-binary-format`
  2.0.2, `ml-dsa` 0.1.1 and `gix` 0.88.0 are all in the local cargo cache, so `--offline` can
  resolve them — they are **unapproved, not unavailable**.
- Package capability sandbox / plugin ABI: **unruled** in the record, and called *"the largest
  unruled thing in the project"*. Do not guess it; executable content makes every refusal
  guarantee conditional on a mod behaving.
- R-62.1 (right-pane API navigation) and H-13.2 (which surface takes which elevation level) are
  **unruled**. Nothing is guessed about them.
- Fresh independent review unavailable in this host; **do not fabricate it**.

## Finish criteria

Separate source inspection, tests, build proof, visible observation and release artifact evidence.
Update this file at checkpoints. Append decisions/corrections; never erase history.
Do not mark alpha shipped while any required gate is missing.
