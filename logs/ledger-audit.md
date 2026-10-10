# Whole-ledger audit — first complete pass (2026-10-02)

**What this is.** A derived report. `docs/LEDGER.md` is the authority and is append-only; this
file neither extends nor rewrites it. It exists because the record's own rule is *read the
record, do not grep it* (C-45 addendum, C-88.3, C-90) and because C-107 measured that the
earliest 39 entries are exactly what a `## `-only reader loses.

**Scope.** Every entry, in all three heading shapes: C-1…C-39 (34 bold one-liners, 5 `###`
headings) and C-40…C-107 (`##` headings). Plus the non-entry preamble groups the file carries
between them. Read end to end on 2026-10-02, in order.

**Method.** (1) Read the whole file in windows, top to bottom. (2) Re-measure the record's
crisp claims against the tree where a measurement was cheap. (3) Track every "Owed, and named"
item to its current state. (4) Fix what is ruled, unblocked and small; file the rest.
Measurements taken this pass are marked **measured today**; everything else is the record's own
tracking.

---

## 0. Measured today (the state a health assessment has to start from)

| instrument | result | command / artifact |
|---|---|---|
| workspace suite | **1256 passed, 0 failed, 25 ignored** (40 binaries) | `cargo test --offline --workspace` → `logs/health-test-run.log` |
| workspace suite, after `ECO-0001` | **1260 passed, 0 failed, 25 ignored** (40 binaries) — the 1256 above plus the envelope pin and ECO-0001's three tests | `cargo test --offline --workspace` → `logs/health-test-run-eco0001.log` |
| workspace suite, after the onset wire | **1261 passed, 0 failed, 25 ignored** (40 binaries) — the 1260 above plus the sky-consumption proof | `cargo test --offline --workspace` → `logs/health-test-run-onset.log` |
| workspace suite, after the retirement work | **1266 passed, 0 failed, 26 ignored** (41 binaries) — the 1261 above plus the four retirement-section tests, the scratch-chain integration test, and its deliberate ignored real-record pass | `cargo test --offline --workspace` → `logs/health-test-run-retire2.log` |
| workspace suite, after `BLD-0025` | **1271 passed, 0 failed, 26 ignored** (41 binaries) — the 1266 above plus the model's four tests and the report section-boundary test | `cargo test --offline --workspace` → `logs/health-test-run-bld0025.log` |
| feature report | **14 of 19 witnesses present**; missing: `signature_verification`, `sealed_container`, `command_line_tool`, `ticket_window`, `change_view` | `logs/audit-features.log` |
| crate census | **48 of 72 active** — 12 crates × **6 axes**; all 12 fail `documented`; 6 unclaimed | `logs/audit-census.log` |
| judge (laya) | 104 readings, belongs 52 / not_belongs 52, **160 artifacts judged, 0 cleared** | `logs/laya-report.txt` |
| board | **515 tickets** | `logs/board-report-full.txt` |
| structurizr contract | **Valid: 0 errors, 0 warnings, 8 info; 22 relationships, 6 views**; source sha256 `728d7d2b…` **matches the manifest** → images current | `structurizr validate`; `logs/structurizr/manifest.txt` |
| the three tests C-92.6 named | `skylight` noon **ok**; `stock::the_model_has_no_exact_subtraction` **ok** (green by design — the pin is the refusal); `hud::the_shell_chrome_is_legible_over_the_worst_backdrop` **ignored** ("needs a GPU adapter") | `logs/health-test-run.log` |
| the record, after the retirement pass | **497 rows — 105 verified, 245 broken (C-104.7 named 244), 147 self-report**; **214 retired and 214 attached, 0 missing**; 31 unbacked and named; board malformed empty; chain **1,532,016 bytes** | `cargo test --offline -p voxel-package --test retire_the_broken_rows -- --ignored --nocapture` → `logs/retirement-pass.log` |
| the gate's model pass, after `BLD-0025` | **`oracle-is-laya yes` (measured from the pass's own decider), `model-oracle established`** (accuracy 1.0000 against a majority class of 0.5631, 103 known answers); **`model-decider structural`, `model-decided 7`, `model-unreached 98`, `divergences 1`** — `civilian/ticket_evidence` (system `refused` on the broken rows, model `active` on the witness) | `logs/gate-bld-0025.log` |

---

## 1. The items before the `## C-` sections (the question that prompted this file)

These are the entries a `## `-only reader — including the filer before C-107 — never sees.
All are in scope here, and their status:

| entry | what it is | status |
|---|---|---|
| C-1 | repo exists, separate; package API as a crate boundary | standing (C-90.1: not amended, and should not be) |
| C-2 | 256× is **addressed**, never stored | standing (confirmed C-27.1); **open:** the displacement channel and the C-91 pattern field (TODO §2.1) |
| C-3 | NPR = Source 2 over OpenPBR | standing (C-27.2); material half ruled C-91 |
| C-4…C-9 | one renderer architecture; crate layout by boundary; first-target contract; `sha2` only; material registry + conservation; mesher written here | standing; C-5's member count since extended (13 in `Cargo.toml`) |
| C-10…C-16 | six measurement corrections (ladder ×1000, ridged caves, boids sign, soil cap, bed correlation, orographic sign, climate belt) | standing, each pinned by a test |
| C-17 | greedy mesh, T-junction cost **declared** | standing; **open: vertex stitching not implemented** (C-88.8 narrowed the "missing pixels" report to this or the stale-chunk trade) |
| C-18…C-24 | octree canonicalisation, padding, `uv` frame, inside-raycast `None`, transparency from the registry, no-face fast path, column profile | standing |
| C-25…C-26 | instrument layer built early; band gauge refusal at construction | standing |
| C-27…C-28 | appearance derives/refuses; 64× detail figure corrected | standing; appearance's six absent parameters are the user's area |
| C-29 | memory is a type; horizon table | standing; **open: C-50.2.3 heading-vs-table 4.6 % (C-51.4 owed)** |
| C-30…C-31 | box window; F12 enforced at the engine | standing; related open: `GAME_WINDOW` radius 5 (C-94.7) |
| C-32 | F: build tree + MSVC | standing; **two corrections recorded and still owed as amendments**: kache restores by COPY (C-51.5), and "100 %" reads two ways (C-50.2.4) |
| C-33…C-34 | offscreen renderer held to the crate by a test; FMA bar measured 2/8 | standing; `APPEARANCE.md` S0 evidence is stale in the document (user's) |
| C-35 | four-bucket rule countable | standing; **open: craft-floor's unenforced halves → BLD-0004** |
| C-36…C-37 | shared constants; HUD drawn and measured | standing |
| C-38 | the black screen, owned | **superseded by C-39** (2 → 10 distinct pixels; 55.5 % lit) |
| C-39 | sourced reflectance; the world turns visible | standing (values approved C-47) |

**Non-entry content between them** (the "Corrections found by measurement" preamble, the
"crate layer" groupings, the "road to the client" framing) carries no owed items of its own;
where it states a fact the entries below restate it, the entry is the authority.

---

## 2. Entry index, C-40 … C-107

Status key: **standing** = a rule/decision nothing contradicts; **closed** = its owed work
landed (closing entry named); **filed** = its work is on the board as tickets; **open** = owed
work named and still not done.

| entry | what it is | status / open item |
|---|---|---|
| C-40 | never test the client invisibly | standing rule |
| C-41 | four shaper corrections; high agency logged for that invocation | standing |
| C-42 | shapes and the laws they come from; anti-minimalism | standing |
| C-43 | quads vs triangles answered | standing |
| C-44 | 256 reaches geometry; two bounds | standing; **open:** merge must stop at block boundaries for a displaced material; a bare amplitude has no shape of its own |
| C-45 | Ponytail out (standing); output unfiltered | standing rule |
| C-46 | the wrong-world measurement | closed (seed printed, eye derived) |
| C-47 | seven reflectances approved | standing |
| C-48 | audit before debug views; references are reference | standing; its present-tense plan paragraph superseded by C-50.1 |
| C-49 | the rulings after the audit | standing; cadence rungs' provenance recorded C-50.3 |
| C-50 | whole-ledger re-check #1 | standing; C-29/C-32 amendments owed |
| C-51 | re-check #2 (wrong slab count; voxel-package missing) | standing; **open: resolved external-crate count (C-51.4)**; the voxel-package entry was paid at C-55.3 |
| C-52 | the system is a build prerequisite | standing |
| C-53 | landing is reference; probes bind; one pass, two readers | standing; the UI spec was written (`docs/UI-SPEC.md`) |
| C-54 | one chain; arrangement first; shell API; depth evidence | standing |
| C-55 | depth is a brightness tint; the owed `voxel-package` entry | standing |
| C-56 | v1 alpha delivery invocation | standing (mission) |
| C-57 | successor readability is an alpha requirement | standing; **open: HANDOFF owes a compaction (C-100.4)** |
| C-58 | gix/Laya/envelope reference reads; six questions | standing; gix deferred on a condition (C-71) |
| C-59 | six rulings + block plan B1…B9 | standing; blocks landed through B7 per later entries |
| C-60 | AWA3D mechanisms | standing |
| C-61 | invented vocabulary corrected; set grown | closed (C-63 grew it to 53) |
| C-62 | second rulings; B3b landed | standing; **open: R-62.1 right-pane navigation** |
| C-63 | docs are the declaration; calibration at 53 | standing; **open: the conformance half — is a derived recipe a faithful reading (C-92.6 keeps it open)** |
| C-64 | UI-SPEC contradicts the recipe schema | **closed — measured today**: `recipe.rs` now carries `extent_unit`, `theme_role`, `layout_policy`, `behavior_policy`, `record_id` |
| C-65 | compendium + census | standing; **census has since grown a sixth axis and reads 48/72 (measured today)** |
| C-66 | chain + v3 envelope + ticket taxonomy | standing; the id-scheme defect it left was repaired C-67.4, then re-numbered C-105/C-107 |
| C-67 | this tree's wire format; ticket read-back | standing; **open:** in-game ticket grammar; envelope axes dropped on the chain (C-67.7) |
| C-68 | the axis that could not fail → `Claimed` | closed; **open: does prose declare? six unclaimed crates** |
| C-69 | apidoc + calibration | standing; **open: `lint` names no culprit term (BLD-0008); `apidoc.rs` still has no caller (C-92.6)** |
| C-70 | axis mutation measured; bloat ported; the record had no chain | standing; the chain now exists; **open: numbering incompatibility (INT-0007); 80 subsection headings unread as tickets** |
| C-71 | working-tree digest; gix deferred on a commit | standing; **open: first commit + SHA-1 → sha256 (C-92.0.2; conversion not run)** |
| C-72 | godplan: method, not document | standing; two `GOD_AGENTS.md` versions undetermined (informational) |
| C-73 | the first record that outlives the process | closed; **open: no segmenting rule for a second `.chain`; pre-chain history → INT-0007** |
| C-74 | the open-question list is 100 % wrong | **open: INT-0006 — `Openness::Settled` unconstructible** |
| C-75 | the 56 answers audited | closed by C-76 (typing) and C-77 (filing) |
| C-76 | the writer can no longer spell an unadmitted word | closed; mutation-tested |
| C-77 | 26 answers + 1 ambiguity filed | closed; linking landed C-79 |
| C-78 | the pane tree, from the reference | closed; **open: BLD-0000 (resize) and BLD-0001 (docking)** |
| C-79 | the chain is hash-linked; linkage was never readable | closed; **open: BLD-0002 (queue) and BLD-0003 (two-reader panel)** |
| C-80 | eight tickets; two reference features the shell lacks | filed (BLD-0000…BLD-0005); craft floor unenforced halves |
| C-81 | twelve tickets from the other projects' READMEs | filed; actor-scope rule (INT-0002) open |
| C-82 | four more tickets; the Laya scope correction | filed; **open: BLD-0013…BLD-0022, INT-0020; the gate still prints `oracle-is-laya no`** |
| C-83 | one seam for every decision | closed; **the model instrument is built (BLD-0025 — see §5); INT-0024 was superseded by H-14.2**; **open: BLD-0023 (seam fronts, does not make)** |
| C-84 | the suite runs through Laya; two passes | closed; **the second pass decides (BLD-0025 — see §5)**; **open: BLD-0026 (grade in the client), INT-0006** |
| C-85 | the shell runs in the client | closed; **today: `ticket_window` is the remaining missing shell witness** |
| C-86 | the debug lanes | closed; **open: BLD-0014/BLD-0015 (vector change view machinery); H-7.10 unruled** |
| C-87 | the vector change view's subject | closed (model); **open: nothing draws it; `shell/changes.rs` still does not exist (measured today)** |
| C-88 | deferred tick ladder; the flash fix | closed; **open: the tick does not advance (C-96.4); pointer grab unruled; `GAME_WINDOW` radius 5** |
| C-89 | the chain is not empty; the reader kept 15 % | closed; **open: numbering incompatibility; stale counts as amendments** |
| C-90 | the smallest thing worth **detecting**; read, do not grep | standing rule — and the reason this file starts at C-1 |
| C-91 | a material's surface pattern is a signed instance field | standing (ruling); **open: the field itself; the meta-ball reference** |
| C-92 | the v3 envelope audit | closed; **92.3's missing `CONTROLS_ARE_TOTAL` pin was landed in this pass**; remaining: BLD-0026, deferred tick, BLD-0014/0015 |
| C-93 | Holdridge citation vs inputs | **`ECO-0001` closed — ruled (aggregate) and landed in this pass**: `column.rs` classifies on `holdridge_inputs`' biotemperature + mean annual precipitation, sampled from the ticked shapers; C-93.2's drift re-measured at **0 of 4** (`logs/biome-drift.log`). **Still open: threshold provenance (C-93.5)** — see F-6 |
| C-94 | onset layer; stage unit; water ledger; sky pass | standing; **onset now wired to `Stage::Weather` — the sky consumes the stage's rate (C-94.7, landed this pass)**; **open: `WeighingGate::net`; rodio playback; surface-water reservoir drains; `GROUNDWATER_CAP` declared** |
| C-95 | the camera stops floating | standing; **open: `STEP_UP` cannot fire at 0.55; no ceiling test; the permissive refusal policy** |
| C-96 | the world stops being a constant | standing; **open: the world is pinned at tick 0 (C-96.4)** |
| C-97 | "simpler" is caught every time | standing rule |
| C-98 | the architecture contract | standing; **validated again today (0/0, 8 info)**; **open: `voxel-audio`'s ticket; README 12 vs Cargo.toml 13; `player → shploingus`; `oracle-is-laya no`** |
| C-99 | laya wired into the game; the agent never read the filings | closed; **the `read_gate` section-boundary defect is fixed — the console refused `integrity`/`identity` as model rows, and the section now ends where the next begins (see §5)**; **open: the agent side is a read, not a connection; BLD-207/208/209** |
| C-100 | the agent's inbox | closed; **the digest-addressed package path landed before this report (measured: `filing.rs` naming + `mount.rs` wiring; `BLD-0207`…`BLD-0215` recompiled under digest names — see §5)**; **open: nothing reads the rows; the inbox has no verb** |
| C-101 | the laya observer is the only citizen | closed; **open: `agency/action.rs` has no tests of its own** |
| C-102 | laya recalibrated to 20 | closed |
| C-103 | the gate's proofs minted from the citizen's rows | closed; **open: no production caller commits a governed act → 0 cleared** |
| C-104 | the ledger filed as tickets; the game catches the board | closed; **the broken rows were repaired this pass — 214 retired, 31 unbacked and named (F-8)**; **open: 147 self-report rows; model instrument; the five parts uneven** |
| C-105 | the board-wide ticket evaluator | closed; **open: the evaluator compiles and does not retire — the retirement event now exists (F-8), but the evaluator itself emits none; `emit`'s audit entries are in-memory only** |
| C-106 | the calibration is the tree's whole vocabulary | closed; **open: the judge's per-artifact basis — the gate's clearing gap** |
| C-107 | the earliest entries were not tickets | closed; **the old tiler's C-entry debris was retired this pass (214 as `Superseded`); 27 preamble tilings + 4 others left unbacked and named (F-8)**; **open: a ledger entry compiles to a work order, not an arrangement** |

---

## 3. The open backlog, deduplicated and blocked-vs-unblocked

**Blocked on the user (nothing may be guessed):**
1. ~~`ECO-0001` — aggregate Holdridge inputs or amend the citation (C-93.4).~~ **Ruled during this pass: aggregate. Landed — see §5.**
2. The surface-water reservoir's sink (evaporation vs re-derived runoff) (C-94.8).
3. Pointer grab (C-88.2); `STEP_UP` 0.55 (C-95.5).
4. `H-7.10` persistence marker (unruled); `INT-0007` pre-chain history; `INT-0006` `Openness::Settled`; `INT-0024` model instrument scope; `voxel-audio`'s declaring document (C-98.7).
5. The chain's numbering incompatibility (C-89.3) — three shapes were put and none taken.
6. Biome threshold provenance typing (C-93.5) and `No unauthoritative limits` (BLD-0013).
7. The **31 unbacked broken fragments** the retirement pass left and named: 27 are the old
   tiler's tiling of the pre-`C-` preamble (`docs/LEDGER.md entry 0`…`entry 8`, incl.
   "Corrections found by measurement"), 3 are CSE-0088/0132/0174 (objectives that record a
   `NAMED GAP` and name no ledger entry), and CSE-0570 (C-106 has no verified successor
   filing). Each needs a successor filing or a ruling — not a retirement, which would be an
   over-claim.

**Unblocked and owed (the agent's):**
1. ~~`BLD-0025` — the model instrument (pass two still refuses).~~ **Landed — see §5** (`model-decided 7`, one preserved divergence, and `oracle-is-laya` is now measured).
2. `BLD-0026` — the gate's two passes graded by axis in a window (C-92.6; apidoc.rs's missing caller).
3. `BLD-0008` — `lint` names the term it convicted.
4. ~~Digest-addressed package path (C-100.4)~~ — **landed before this report; measured and
   recorded in §5** (`BLD-0207`…`BLD-0215` each carry a digest-named package beside the legacy
   one).
5. ~~`read_gate` refusing the build's own report keys (C-99.5).~~ **Landed with BLD-0025** — the pass-two section now ends at the next `# ` header, so the build's own trailing section is not read as model rows.
6. `agency/action.rs` commit-point test (C-101.4).
7. ~~The onset wired to `Stage::Weather` (C-94.7).~~ **Landed — see §5.**
8. The vector change view drawn, merge-stop at block boundaries, shader path for displaced normals (C-44.5, C-86.3, TODO §2.1).
9. ~~Retirement of the 244 broken ticket rows (C-104.7)~~ — **landed: the event was built and
   the pass ran on the record (214 retired, 31 unbacked and named; see §5)**. The 147
   self-report rows stay out of scope by ruling: they "need citations to reach `B`", which is
   content work on each row, not a retirement.
10. Alphabetical tail: vertex stitching (C-17), `WeighingGate::net` pin (C-94.1), the `documented` census axis (see finding F-2).

---

## 4. Feature health — the 19 declared features (measured today)

| feature | witness | state |
|---|---|---|
| `package_intake` (C-55) | lib.rs | **active** |
| `recipe_compilation` (C-56) | recipe.rs | **active** |
| `build_gate` (C-52.1) | shploingus/build.rs | **active** |
| `world_generation` (C-1) | shapers/voxel.rs | **active** |
| `signing_seam` (R-59.2) | package/signing.rs | **active** |
| `signature_verification` (R-59.2) | voxel-envelope | **absent** — no crate |
| `sealed_container` (R-59.4) | package/envelope.rs | **absent** |
| `conformance_oracle` (R-59.5) | package/oracle.rs | active |
| `feature_evaluation` (R-59.7) | package/features.rs | active |
| `provenance_digest` (R-59.1) | package/provenance.rs | active |
| `command_line_tool` (R-59.6) | voxelctl | **absent** |
| `shell_arrangement` (H-9.7) | shell/arrangement.rs | active (witness) — **means unaudited, BLD-0005** |
| `pinned_hud` (H-9.8) | shell/hud.rs | active |
| `notification_queue` (H-12.2) | shell/notifications.rs | active |
| `ticket_window` (C-56) | shell/tickets.rs | **absent** |
| `event_chain` (H-8.1) | model/chain.rs | active |
| `stage_cadences` (H-8.3) | shapers/stages.rs | active (repointed C-88.6) |
| `save_load` (C-56) | stream/resident.rs | active (witness only — no file I/O module) |
| `change_view` (C-56) | shell/changes.rs | **absent** |

**5 of 19 have no witness.** All six `required_for_alpha` features are present, so the gate
passes while displaying the five.

---

## 5. Fixes landed in this pass

* **The model instrument (BLD-0025): the seam's second decider, calibrated and measured.**
  H-14.2's second pass was `Unreachable` — every subject refused, and `oracle-is-laya` a string
  literal. Landed in `oracle.rs`:
  * **`Model`** — a `Decider` over the same 105 subjects that reads **the record's own
    declaration of a subject** as the judge's recipe and **the tree's own evidence for it** as
    the line, and answers through the calibrated `StructuralJudge`. The four `lint` rules travel
    per decision: an uncalibrated pass refuses **before reading anything**; a `no_reading` is a
    named absence; a `not_belongs` without a human's reason is refused (`unreasoned_override`) —
    the gate manufactures no reasons; a `not_belongs` with one is a `Refused` assessment that
    was reached, which is the disagreement the two passes preserve.
  * **The doc stage is a first-class arm**: for each crate's `documented` cell the line is the
    crate's own comment prose (`bloat::comment_lines`, sharing `is_comment` with
    `measure_prose` — one definition of what a comment is) read against the document `census`
    found defining it; where no document defines the crate, the absence names exactly that
    rather than inventing a reading.
  * **`build.rs`**: pass two is `oracle::Model::calibrated(&workspace)`; **`oracle-is-laya` is
    measured from the pass's own decider** — the ticket's own measurable part, which was a
    string literal — and `model-oracle` is **established/not-established** from the calibration
    rather than `not-evaluated`. The build asserts the model pass decided something: a second
    instrument that only refuses is not one.
  * **The reader defect fixed with it**: `decision::read_report` ran the pass-two section to the
    end of the text, so the build's own trailing section (`integrity`, `identity`) was read as
    model rows — the console refused the build's own report (`logs/live.out`). The section now
    ends at the next `# ` header; the client fixture carries the whole shape.
  * **Measured**, `logs/gate-bld-0025.log`: `oracle-is-laya yes`; `model-oracle established`
    (accuracy 1.0000 against a majority class of 0.5631, 103 known answers); `model-decider
    structural`; **`model-decided 7`, `model-unreached 98`, `divergences 1`** — the
    ticket-evidence ladder (system `refused` on the broken rows, model `active` on the witness:
    a real disagreement, printed row by row). The 7 read are the seven civilian ladders; every
    absence is named per subject. **Evidence:** `cargo test --offline -p voxel-package --lib
    oracle::` → **27 passed**; `decision::` → **14 passed**.

* **The digest-addressed package path (C-100.4) — landed before this report; measured and
  recorded here because the first draft of this file still listed it owed.**
  * `filing.rs`: `package_file_name(name, digest)` and `package_path` derive a package's name
    from its **content identity**, so a recompile lands beside the earlier package rather than
    colliding with it; `filed_packages` lists every package for one record name (newest first
    by `(mtime, name)`) and `newest_package` is the one-package reader. The module's own doc
    names `BLD-0207`…`BLD-0215` as the case and C-100 §100.4 as the owed item.
  * `mount.rs`: both writers go through `package_path` — the `file` verb and the bulk
    `hand_the_boards_tickets_to_the_agent`, which reads the newest package **through the
    loader** and compiles again when the one on disk does not frame. Pinned by
    `a_filing_whose_package_declares_nothing_is_compiled_again_under_a_new_name` (the
    `BLD-0207`…`BLD-0215` rule as a test: the legacy empty package is kept, a digest-named
    replacement is written and handed over).
  * **On disk**: `packages/BLD-0207.shpkg`…`BLD-0215.shpkg` (legacy, written 02:31–02:51 by
    the earlier compiler) each sit beside a `BLD-0207-<sha256>.shpkg`… (03:55), so the
    recompiles happened and the record of what the older compiler produced was not overwritten.
  **Evidence:** the suite below (mount's recompile test green); `ls packages/BLD-0207*`.

* **`CONTROLS_ARE_TOTAL`** (`crates/voxel-model/src/envelope.rs`) — C-92.3's finding:
  `Control` was the one enum in the file with a `COUNT` and no totality pin, so `COUNT` edited
  to 5 with `ALL` untouched was silence where the other three axes are compile errors. Added
  the const, referenced it in the vocabulary test, and added
  `the_control_vocabulary_is_pinned_like_the_other_three`.
  **Evidence:** `cargo test --offline -p voxel-model --lib envelope` → **11 passed; 0 failed**.
* **`ECO-0001`** (`crates/voxel-shapers/src/ecology.rs`, `column.rs`) — C-93's defect: the
  classifier cited Holdridge and was fed one tick of a season. Ruled (**aggregate**) and
  landed:
  * `holdridge_inputs` **samples the ticked shapers** — `climate::temperature_c` and
    `weather::precipitation_mm` at four quarter-month midpoints of each of twelve months
    (`MONTH_SAMPLES = 4`; the sinusoid is under-resolved by `sinc(π/24) ≈ 0.991`), clamps each
    **monthly mean** to Holdridge's `[0, 30]`, averages the twelve for **biotemperature**, and
    takes the twelve monthly means for **mean annual precipitation**. No number is re-derived
    analytically; every sample is a tick.
  * `biome_for`'s parameters and `biome_at`'s contract are renamed to what they are; `derive`
    feeds the aggregates; the instantaneous `clim`/`wx` stay in `Column` for consumers that
    want *this tick*.
  * **Three tests added**: the clamp-before-averaging pin, the aggregate's tick-invariance
    across years, and `column::tests::the_biome_is_the_same_at_every_tick_of_the_year`
    (C-93.2's four probes, all 96 days + a later year — the drift is pinned at zero).
  * **C-93.2 re-measured**: `examples/biome-drift.rs` was stale — it still fed instantaneous
    samples into `biome_at`'s now-biotemperature parameter — and is updated to the ruled
    semantics; **0 of 4** columns change biome within a year, against **4 of 4** before
    (`logs/biome-drift.log`).
  * **The coverage test's probe was corrected, not its guard**: after the aggregate,
    `the_pipeline_covers_biomes_and_lithologies_across_a_world`'s diagonal saw only
    `{Tundra, BorealForest}`. A grid sweep — the same remedy the water test in that file
    already records — measures `{Tundra, BorealForest, Alpine}` (Alpine from the declared
    altitude override), so `>= 3` stands and the probe now covers the world. Evidence:
    `examples/biome-census.rs` → `logs/biome-census.log`.
  **Evidence:** `cargo test --offline -p voxel-shapers` → **205 passed; 0 failed; 1 ignored**;
  workspace suite → **1260 passed; 0 failed; 25 ignored** (`logs/health-test-run-eco0001.log`).
* **The onset wired to `Stage::Weather` (C-94.7)** — the gap C-94.7 names: the stage's
  consumer read `precipitation_mm`, the long-run mean, so the onset layer built at 94.4 had
  no consumer on the stage it was built for, and a column whose rain never varied had a sky
  that never varied either.
  * `sky_inputs_at` now reads `weather::precipitation_now(...)` — the stage's own answer,
    carrying the onset, the rate and the two flags — and `turbidity_of` uses
    **`rate_mm_per_year`**. `WET_TURBIDITY_GAIN`'s own doc already claimed *"a world in
    which it is **raining**"*; the mean cannot honour that and a rate can. Because the rate
    is `mean · onset / ONSET_DUTY`, the derived turbidity **conserves the annual mean** —
    the haze is redistributed in time, not added.
  * `main.rs`'s ×4 weather gate now reads `voxel_shapers::weather::is_due_at` — the stage's
    own seam, whose doc says it exists for exactly this — instead of restating the ladder
    call; the seam had **no production caller** before this (the F-5 shape, in miniature).
  * **New test**: `skylight::tests::the_onset_reaches_the_sky_even_where_the_mean_never_moves`
    searches out one of the measured **49.7 %** flat downslope columns, pins its mean as
    constant, and asserts the onset is what moves the turbidity above it. The sweep test
    `the_sky_is_driven_by_the_worlds_weather_and_not_by_a_default` now samples
    `precipitation_now`.
  **Evidence:** `cargo test --offline -p voxel-client --lib skylight` → **23 passed;
  0 failed**; workspace suite → **1261 passed; 0 failed; 25 ignored**
  (`logs/health-test-run-onset.log`).
* **The retirement event, and the old tiler's broken rows filed (`EventKind::Retirement`)** —
  F-8's gap: the event kind existed and the fold had an arm for it, but the arm flipped the
  status to `Complete` with `retirement: None` (*closed-but-unretired*) and **nothing in
  `crates/` emitted one**, so C-104.7's repair had no filing path. Built, then used:
  * **The section.** `ticket.rs` carries the record as tagged text: `retirement_section`
    writes it and `parse_retirement_section` is its **strict only reader**. Grammar: `[RET]`,
    `[RET-REASON]` (one of the fourteen slugs), `[RET-CLAIM]`, `[RET-GROUNDING]
    verified|unverified: <note>`, `[RET-VERIFICATION] passed|failed, regressions|no-regressions`,
    `[RET-ACCEPTANCE]` (0+), `[RET-CHECK]` (0+), `[RET-READ-BACK] held|not_held|not_checked`.
    An unknown tag is **refused, not skipped**; a duplicate scalar, a missing field and a
    malformed identity are refused; coherence runs through `Retirement::new`, so the one
    validated token still cannot be manufactured on an unchecked read-back.
  * **The fold.** `Board::from_chain`'s `EventKind::Retirement` arm parses the section and
    calls `retire(ticket, retirement)`; a malformed retirement, or one naming a ticket no
    event holds, goes to `board.malformed()` instead of flipping a status. A duplicate
    `ticket_transition` at a new position is a new ticket, not a re-close.
  * **Tests.** Four in `ticket.rs` (`retirement_section_tests`): round trip; the validated
    token cannot be manufactured on an unchecked read-back; unknown field refused; duplicate
    scalar refused. One integration in `crates/voxel-package/tests/retire_the_broken_rows.rs`:
    a retirement filed on a scratch chain attaches its own record on read-back.
  * **The pass, on the real record** (`logs/retirement-pass.log`): **497 rows — 105 verified,
    245 broken (C-104.7 named 244), 147 self-report**. Selection: a broken row (document
    present, `holds != Some(true)`) is retired only when a later board ticket whose objective
    starts `C-<entry> —` **verifies** — reason `Superseded`, `ReadBack::NotChecked`, grounding
    `verified` by re-reading the successor's cited digest. **214 filed and 214 attached;
    0 missing**; board malformed empty; the chain grew to **1,532,016 bytes** (from
    1,325,503).
  * **31 left and named**: 27 are the old tiler's tiling of the pre-`C-` preamble
    (`docs/LEDGER.md entry 0`…`entry 8`), 3 record a `NAMED GAP` and name no ledger entry
    (CSE-0088/0132/0174), and CSE-0570's entry C-106 has no verified successor filing. The pass
    refuses to call these retired; §3 carries them as blocked.
  **Evidence:** `cargo test --offline -p voxel-package --test retire_the_broken_rows` →
  **1 passed** (the scratch-chain shape); the same test with `-- --ignored --nocapture` →
  the pass above.

---

## 6. Findings this pass (not in the ledger as such)

* **F-1 — the census's own header contradicts its axes.** `census.rs`'s module doc says
  "these five" and names `Axis::Declared`; the code runs **six** axes and the fifth is
  `Claimed` (renamed at C-68.1). The stale doc is the exact defect the project convicts:
  a comment that describes a shape the code no longer has. Measured today: report columns are
  `named moduled tested refusing claimed documented`.
* **F-2 — the `documented` axis refuses all twelve crates**, and no entry read in this pass
  introduces it. 48 of 72 cells active; 12 of the 24 gaps are `documented`, each with the same
  refusal: *"no document under docs/ defines it — no heading there begins with its name…
  prose in a code artifact is documentation written where a reader does not look for it."*
  Either the axis is a newly declared, never-satisfied gate — in which case its ruling is not
  on the record — or it is drift. Both readings are findings. **The ledger's C-65.2 census
  claim ("54 of 60 cells active") is superseded by this measurement.**
* **F-3 — one known-failing client test parked as `ignored`.** C-94.7 recorded
  `hud::tests::the_shell_chrome_is_legible_over_the_worst_backdrop` failing on `shaped_runs`;
  today it is in the ignored set ("needs a GPU adapter"). A red promoted to "ignored" is a
  red that stops being read — the C-92.5 shape of the console test failing silently.
* **F-4 — the two red tests of C-92.6 are genuinely green** (measured): the `skylight` noon
  test passes, and the `stock` pin passes because the refusal it pins is present.
* **F-5 — `apidoc.rs`'s missing caller is still the largest gap in the "two readers" rule**
  (C-53.3, C-92.6): the gate's findings reach `OUT_DIR` and the console, and no window reads
  the two passes. `BLD-0026` stands.
* **F-6 — after `ECO-0001`, the default world's realized biotemperature never reaches the
  classifier's third arm.** Measured over a 121×121 grid at 100 m (finer than the climate
  belt's ~400 m structure, so the sampling cannot be aliasing this): biotemperature spans
  **0.000 … 2.810 °C**, MAP 11.6 … 29 510.2 mm, biomes `{Tundra 8360, BorealForest 6158,
  Alpine 123}`. Two consequences: (a) no arm at or above the `temperature_c < 3.0` boundary is
  reachable in the default world — the C-16 climate belt (±9 °C) was calibrated against the
  pre-aggregate classifier; (b) the `temperature_c < -5.0` arm is unreachable **by
  construction**, because biotemperature is clamped at 0, so `PolarDesert` can never appear in
  a world. Both are declaration-level (world identity; C-93.5's owed threshold provenance), so
  this pass measured and filed them rather than turning a knob.
* **F-7 — line-shaped probes under-cover the world.** The coverage test's diagonal missed the
  Alpine columns (F-6's grid finds 10 at 300 m, 123 at 100 m) and was corrected to a grid; the
  water test in the same file already carries this lesson ("an earlier revision walked `z = 0`
  alone and found no water anywhere"). Two instances are now on the record inside one file;
  the remaining single-line or single-point probes are unmeasured as coverage claims, not
  refuted.
* **F-9 — the model's second reading is 7 of 105 subjects, and the absences are two honest
  classes.** Measured with the landed instrument: the 7 are the civilian ladders, whose witness
  line carries the token the ladder declares; the 98 absences are (a) *the declaration does not
  license the evidence* — the 11 feature witnesses whose comment prose uses words the row's
  `means` never declared, all six numbers (the document line carrying the sentence is wider
  than the sentence), and the recipe (its declaration and its evidence are the same bytes) —
  and (b) *no text was read* — 60 axis cells read as facts, and the 12 `documented` cells whose
  crates have no docs heading for the prose to belong to. None of these is a pass; every one is
  named. **The numbers reading an absence is worth a ruling**: pass one decides them from the
  sentence's presence, while the model judges the whole carrying line.
* **F-8 — a retirement had no writer and the fold dropped its contents (resolved in this
  pass).** Measured when found: `EventKind::Retirement` existed (`chain.rs:301`) and
  `filing::grow` could carry any event kind, but **nothing in `crates/` emitted one**, and the
  fold flipped the status to `Complete` with `retirement: None` — *closed-but-unretired*, the
  exact defect `RefusalKind::UnretiredTicket` and `Board::retire` exist to refuse, so a mass
  retirement pass would have been status flips with no record behind them. **Fixed**: the
  section writer/parser landed in `ticket.rs` (strict — unknown tags refused, duplicates
  refused, `Retirement::new`'s coherence still the only way to the validated token), the
  fold's arm now parses and attaches, malformed retirements go to `board.malformed()`, and the
  pass ran on the real record — **214 retired and attached, 0 missing; 31 unbacked and
  named** (§5; `logs/retirement-pass.log`). **Residue**: the 31 named fragments need a
  successor filing or a ruling (§3, blocked item 7), and the 147 self-report rows remain out
  of scope by ruling — they "need citations to reach `B`", which is content work on each row,
  not a retirement.

---

## 7. What this pass did not do

* It did not re-measure every numeric claim in C-1…C-39; those with tests are pinned, and the
  ones with known drift (C-29, C-32, the stale counts of C-70.3/C-73.4) are named above.
* It did not edit `docs/` — the record is append-only and this is a derived report in `logs/`.
  The landing of `ECO-0001` supersedes C-93.5's "Owed" line, the onset wire supersedes
  94.7's "not wired" line, the retirement pass supersedes C-104.7's "244 broken rows" line
  (214 retired, 31 left and named), the digest-addressed package path landed before this
  report (C-100.4's "next landing") and is likewise unrecorded, the model instrument (BLD-0025)
  is built with `oracle-is-laya` now measured and `model-oracle` established, and F-6…F-9 are
  new findings; the record needs a **new entry** rather than edits, which is the record owner's
  call. The derived state lives here until then.
* It did not close any remaining blocked item; the blocked list in §3 is the user's to rule.
