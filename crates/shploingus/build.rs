//! The mandatory build gate.
//!
//! **This is not a calibrated model oracle and does not claim to be one.** It is
//! deterministic schema validation plus three evaluators, and the report it writes says
//! so in those words.
//!
//! # Why the build script at all
//!
//! C-52.1: *the game cannot compile without the system.* A gate that only runs when
//! someone remembers to run it is a suggestion, so this runs on every build, and a
//! refusal here **fails the build** rather than printing a warning.
//!
//! # Two readers, one record (C-53.3)
//!
//! UI-SPEC §9 requires the build gate to be the hard stop and **the same findings** to be
//! a record the notifications panel renders, *"so a green build and a legible panel cannot
//! disagree."* So this script writes one report to `OUT_DIR`, `main.rs` embeds exactly that
//! file, and both read the same evaluator functions. There is no second code path and no
//! second opinion.

use std::{env, fs, path::PathBuf};

use voxel_package::{
    bloat, census, compendium, conformance, features, filing, oracle, patch, projection, provenance,
    record,
};

fn main() {
    let root = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"),
    );
    // The workspace root, found rather than assumed: every witness path in the feature
    // table is relative to it, and an assumed root makes every witness resolve against
    // the wrong place.
    let workspace = record::workspace_root(&root).expect("a workspace root above this crate");

    let source_path = workspace.join("packages/alpha-shell.shspec");
    println!("cargo:rerun-if-changed={}", source_path.display());
    // The patch set is read by the gate, so a record that changes must rebuild it — the same
    // reason the chain directory is watched below: a number measured from a directory nobody
    // watches is a number from before the change.
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("patches").display()
    );
    // The record is read by the gate, so a document that changes must rebuild.
    for document in record::DOCUMENTS {
        println!(
            "cargo:rerun-if-changed={}",
            record::document_path(&workspace, document).display()
        );
    }
    // **The chain is read by the gate too, so it must rebuild the gate as well** (C-107). The
    // board section is `voxel_package::board::describe` over `bloat::read_chain`, and the bloat
    // section is `measure_dir` over the same directory — so a chain that changed while this
    // script did not re-run left both printing numbers from **before** the filing. That is the
    // same defect as a reader that greps one heading level: a stale reading that reads as a
    // measurement. `filing::CHAIN_DIR` rather than a second hand-typed `"chains"`, the same
    // reason the line below uses it.
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join(filing::CHAIN_DIR).display()
    );
    // **The declared readings the gate reads back, watched like every other input** — the same defect
    // as the chain above, one directory over. The census reads `conformance/` during this script:
    // the prose ratchet, the acceptance list, and the compiler's documentation reading with its pin.
    // Cargo re-runs a build script only for the paths it names, so without this line a reading edited
    // on disk is not re-read — the gate then prints, and the window renders, the *previous* answer as
    // this build's. The count would be right about the file and wrong about the build, which is a
    // stale reading that reads as a measurement.
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("conformance").display()
    );

    // ---- the build tree and the volume, **before anything is compiled**
    //
    // Every line below this one writes into `target/`, so a volume that is filling has to be
    // answered before the gate adds to it, not after cargo has already died in it. The guard
    // trims only what cargo regenerates, reports every removal and every kept artefact with its
    // reason, and refuses below its minimum rather than letting a linker fail with an error that
    // names nothing (`voxel_package::space` carries the measurements behind its three thresholds).
    //
    // **It runs here first and is reported further down**, because the report has to exist before
    // the door can shut: a reader whose build just failed is the reader who most needs to see why.
    let tree = voxel_package::space::target_dir(&workspace);
    let space = voxel_package::space::guard(&tree);

    let source = fs::read(&source_path).expect("required alpha shell recipe must be readable");

    // ---- the package itself: compile, then load through the same codec ----
    let package = voxel_package::recipe::compile(&source)
        .unwrap_or_else(|errors| panic!("alpha shell recipe refused: {errors}"));
    let loaded = voxel_package::recipe::load(&package)
        .unwrap_or_else(|errors| panic!("alpha shell package refused: {errors}"));
    assert_eq!(
        loaded.source, source,
        "the package must preserve the exact recipe source"
    );

    // ---- the declared features: refuse what the build depends on, display the rest ----
    let states = features::evaluate(&workspace);
    let required_absent: Vec<&str> = states
        .iter()
        .filter(|state| {
            state.state != features::Establishment::Active && state.feature.required_for_alpha()
        })
        .map(|state| state.feature.name())
        .collect();
    if let Err(refusals) = features::check_required(&workspace) {
        panic!(
            "a feature the build depends on has no witness: {refusals}. Named: {required_absent:?}"
        );
    }

    // ---- the conformance oracle: measured, and reported with its set size ----
    let (oracle_state, calibration, oracle_refusals) = oracle::gate(&workspace);

    // ---- the record: every citation must resolve to a real document ----
    let read_record = record::read_record(&workspace)
        .unwrap_or_else(|errors| panic!("the record could not be read: {errors}"));
    record::check_record(&workspace)
        .unwrap_or_else(|errors| panic!("the record and the feature table disagree: {errors}"));

    // ---- the record, compressed: the derived format solutions come from ----
    //
    // R-62.2 made mechanical: the documents are the declaration, so the build
    // reduces them to the format an evaluator reads. A gate that did not do
    // this would grade the code against 400 KB of prose, which is not a
    // measurement.
    let compressed = compendium::compress(&workspace)
        .unwrap_or_else(|errors| panic!("the record could not be compressed: {errors}"));
    compendium::check(&compressed)
        .unwrap_or_else(|errors| panic!("the compression is not well formed: {errors}"));
    let shape = compressed.shape();

    // ---- every crate in the workspace, on the same terms as the new ones ----
    //
    // R-59.7: the missing functions are evaluators for the building. The
    // evaluators built so far only ever described the new work, so this is the
    // first run that looks at the eleven crates that predate any of it.
    census::check_table_shape()
        .unwrap_or_else(|errors| panic!("the crate table is not well formed: {errors}"));
    // The table against the workspace's own manifest: a member with no row is measured by nothing
    // (four were), and a row for an absent crate reports on another tree.
    census::check_against_manifest(&workspace)
        .unwrap_or_else(|errors| panic!("the crate table disagrees with the workspace: {errors}"));
    // The declared witnesses, with `CrateRow::required` deciding what a missing one costs.
    let optional_absences = census::check_witnesses(&workspace)
        .unwrap_or_else(|errors| panic!("a crate's declared witness is not on disk: {errors}"));
    // Rule 2 of the dependency discipline, measured: is it actually used? Reported, not refused.
    let orphans = census::orphan_libraries(&workspace)
        .unwrap_or_else(|errors| panic!("which crate reads which could not be measured: {errors}"));
    // The prose ratchet: a crate's share of comment may fall and may not rise.
    let prose = census::check_prose_frame(&workspace)
        .unwrap_or_else(|errors| panic!("a crate's prose budget was exceeded: {errors}"));
    let the_census = census::census(&workspace)
        .unwrap_or_else(|errors| panic!("the workspace could not be measured: {errors}"));
    census::check_coverage(&the_census).unwrap_or_else(|errors| {
        panic!("the census did not measure everything, so it is not a census: {errors}")
    });

    // ---- conformance: the census's measurements turned into one question the build must answer
    //
    // **`of`, not `verdict`.** The census has just been taken, and `conformance::verdict` would
    // walk the whole tree a second time to take it again — the same measurement twice, which is
    // how two readers of one record come to disagree. The declared acceptance list is read here
    // so the reuse stays faithful to what the door on its own would have said.
    let (accepted_keys, accepted_declared) = conformance::accepted(&workspace);
    let the_verdict = conformance::of(&the_census, &accepted_keys, accepted_declared);

    // ---- the decision layer: two passes over one subject vocabulary ----
    //
    // **H-14.1**: Laya is the *decision layer*, not one oracle behind a trait, so every decision
    // this framework makes — the axes of every crate, all nineteen feature rows, the record's
    // numbers, the civilians' ladders, and the recipe — is asked of one seam. `deterministic`
    // answers by delegating to the same census, feature table and codec the sections above used,
    // which is what makes the agreement check below meaningful rather than circular: it compares
    // one implementation read twice.
    //
    // **Two passes, one vocabulary** (H-14.2): the model pass runs through `oracle::Model` — the
    // calibrated structural judge reached through the seam (BLD-0025). It reads the record's
    // declaration of each subject against the tree's evidence, refuses every subject before
    // reading anything when the calibration is not quotable, and never convicts without a
    // human's reason recorded in the same act; where a subject carries no prose to read, the
    // absence is named rather than defaulted. The pass is computed here — before the report is
    // built — so the oracle section below can *measure* `oracle-is-laya` from the decider that
    // actually ran instead of printing a literal.
    let system = voxel_package::decision::Deterministic::new(&workspace).with_recipe(&source);
    let model = voxel_package::oracle::Model::calibrated(&workspace);
    let two = voxel_package::decision::run(&system, &model);
    assert_eq!(
        two.system
            .of(voxel_package::decision::Subject::Recipe)
            .map(|assessment| assessment.state),
        Some(voxel_package::features::Establishment::Active),
        "the decision layer and the direct compile disagree about the shipped recipe, and two \
         readers of one record may not disagree"
    );
    assert!(
        two.model_decided() > 0,
        "the model instrument decided nothing, and a second pass that only refuses is the \
         absence `model-unreached` already names rather than a second instrument"
    );

    // ---- one report, two readers ----
    let mut report = String::new();
    report.push_str(&format!("gate\t{}\n", voxel_package::recipe::HEADER));
    report.push_str(&format!("package\t{}\n", loaded.recipe.name));
    report.push_str(&format!("version\t{}\n", loaded.recipe.version));
    report.push_str(&format!("source-sha256\t{}\n", loaded.source_digest));
    report.push_str(&format!("package-sha256\t{}\n", loaded.package_digest));
    report.push_str(&format!("windows\t{}\n", loaded.recipe.windows.len()));
    report.push_str(&format!("pinned\t{}\n", loaded.recipe.pinned.len()));

    report.push_str("\n# declared features\n");
    for state in &states {
        report.push_str(&format!(
            "feature\t{}\t{}\t{}\n",
            state.feature.name(),
            state.state.slug(),
            state.reason
        ));
    }

    report.push_str("\n# conformance oracle\n");
    report.push_str(&format!("oracle\t{}\n", oracle_state.slug()));
    report.push_str(&format!(
        "known-answers\t{}\n",
        calibration.readings_used
    ));
    report.push_str(&format!("accuracy\t{:.4}\n", calibration.accuracy));
    report.push_str(&format!(
        "majority-class\t{:.4}\n",
        calibration.majority_class
    ));
    report.push_str(&format!(
        "inverted-accuracy\t{:.4}\n",
        calibration.inverted_accuracy
    ));
    report.push_str("oracle-kind\tstructural-deterministic\n");
    // **Measured, not a literal.** The line answers *is the thing that ran the seam's model
    // instrument?* from the pass's own decider. Before BLD-0025 it was the string `no`, which
    // the record's own ticket convicted: a row that could read either way and could not.
    report.push_str(&format!(
        "oracle-is-laya\t{}\n",
        if two.model.decider == voxel_package::oracle::MODEL_NAME {
            "yes"
        } else {
            "no"
        }
    ));
    // **The model oracle's establishment is measured too** (R-59.5). Either the judge beats the
    // majority class of the known-answer set and may be quoted, or the gate says
    // `not-established` rather than saying nothing.
    report.push_str(&format!(
        "model-oracle\t{}\n",
        if model.is_quotable() { "established" } else { "not-established" }
    ));
    for note in &calibration.calibration.notes {
        report.push_str(&format!("oracle-note\t{note}\n"));
    }
    for refusal in oracle_refusals.all() {
        report.push_str(&format!("oracle-refusal\t{}\n", refusal.detail()));
    }

    report.push_str("\n# the record\n");
    for (document, digest) in &read_record {
        report.push_str(&format!(
            "document\t{}\t{}\t{}\n",
            document.path,
            document.role.slug(),
            digest
        ));
    }

    // The compression, so a reader can see what the gate derived its answers
    // from rather than having to take the answers on trust.
    report.push_str("\n# the record, compressed\n");
    report.push_str(&format!("compendium-format\t{}\n", compendium::FORMAT));
    report.push_str(&format!(
        "record-sha256\t{}\n",
        compressed.record_digest
    ));
    report.push_str(&format!("decisions\t{}\n", shape.occurrences));
    report.push_str(&format!("anchors\t{}\n", shape.distinct_anchors));
    report.push_str(&format!(
        "anchors-written-twice\t{}\n",
        shape.written_more_than_once
    ));
    for series in compendium::Series::ALL {
        let count = compressed
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.anchor.series == series)
            .count();
        report.push_str(&format!(
            "series\t{}\t{count}\tauthority={}\n",
            series.name(),
            series.is_authority()
        ));
    }
    let open = compressed.open_questions();
    report.push_str(&format!("open-questions\t{}\n", open.len()));
    for question in &open {
        report.push_str(&format!(
            "question\t{}\t{}:{}\t{}\n",
            question.anchor,
            question.document,
            question.line,
            question.openness.slug()
        ));
    }

    // **What the record costs, measured in this pass rather than by a script that cannot be
    // run from here.** H-10.1's *"one clean execution pass"* carries identity, refusals,
    // conformance and provenance, and D97 ruled Python out of the project — so the bloat
    // measurement C-66 cited now lives in `voxel_package::bloat` and prints in this report.
    //
    // The shape is `measure_dir` over the chain files, and **the empty case is stated, not
    // counted**: a build with no chain file on disk prints `not-measured` with its reason,
    // which is D107's *"absence rendered as absence"* rather than a row of zeroes.
    report.push_str("\n# what the record costs\n");
    // **`filing::CHAIN_DIR`, not the literal.** The path is declared once in the module that
    // writes it; a second hand-typed `"chains"` here would be a second answer to "where does
    // the record live", and the two would drift — the exact shape D-07 names, and the one
    // `bloat::read_chain`'s two-file refusal exists to catch one layer up.
    let chain_dir = workspace.join(filing::CHAIN_DIR);
    let chains = bloat::measure_dir(&chain_dir);
    if chains.is_empty() {
        report.push_str(&format!(
            "ledger-bloat\tnot-measured\nledger-bloat-why\t{} holds no .chain file, so no record \
             was measured; a count of zero here would read as a measurement of a record that \
             costs nothing\n",
            chain_dir.display()
        ));
    }
    for (name, measurement) in &chains {
        report.push_str(&format!("ledger-bloat-file\t{name}\n"));
        report.push_str(&measurement.describe());
    }

    // **The build tree, and the volume it is written to.** The other cost this project measures:
    // the record's prose is measured above, and the tree it is built in is measured here. Both are
    // printed every build, because a guard nobody can read is a guard nobody can check.
    report.push_str("\n# the build tree and the volume\n");
    report.push_str(&space.describe());

    // **Which bytes this build read.** R-59.1's first half, and the feature row the gate was
    // already pointing at: `provenance_digest`'s witness names
    // `crates/voxel-package/src/provenance.rs`, and the file did not exist, so the row read
    // `unchecked` on every build.
    //
    // The gix half is reported as a **condition with two named facts** — is there a commit, and
    // is the object format one `Digest` can express — because the user's ruling scopes gix to
    // *when a commit exists*, and this repository has none. `provenance-gix deferred` with its
    // reason is the honest line; `provenance-gix failed` would be a permanent red on a project
    // that is simply not a repository yet, and the gate would have trained everyone to ignore it.
    report.push_str("\n# which bytes this build read\n");
    report.push_str(&provenance::of_workspace().describe());

    // **Whether the two views of the record still agree.** H-8.1 says `docs/LEDGER.md` and the
    // in-game ledger window are *"two projections of one record, so a disagreement between them
    // is impossible by construction"*. The chain is the canonical one and the Markdown is
    // output — so the question this gate asks every build is whether the output still matches.
    //
    // **It does not, and the gate says so rather than going quiet.** Every C-entry in the
    // record was typed by hand; the chain carries none. That is the measured state of the
    // project, and a gate that printed nothing here would be the one place the record's own
    // central claim goes unchecked.
    report.push_str("\n# do the two views of the record agree\n");
    // **Three refusals, kept apart on purpose.** A missing chain file, a chain that will not
    // read, and a record that will not compress are three different facts, and collapsing them
    // is how a report gets to read as agreement when nothing was compared.
    match (
        bloat::read_chain(&chain_dir),
        compendium::compress(&workspace),
    ) {
        (Err(why), _) => {
            report.push_str("projection\t\tnot-measured\n");
            report.push_str(&format!("projection-why\t{why}\n"));
        }
        (Ok(_), Err(found)) => {
            report.push_str("projection\t\tnot-measured\n");
            for refusal in found.all() {
                report.push_str(&format!("projection-refusal\t{}\n", refusal.detail()));
            }
        }
        (Ok(chain), Ok(the_record)) => {
            report.push_str(&projection::compare(&the_record, &chain).describe());
            // The board's duplicate report belongs here: it is the same question, and without
            // a right surface the report names nothing (84 repeat filings are currently on
            // this record).
            report.push_str(&voxel_package::board::describe(&chain));
        }
    }

    // The census, one line per crate. **Every gap is printed with its reason**,
    // because a row of `active` and a row of `inactive` with no explanation are
    // the same bytes to a reader who does not know the difference, and that
    // reader is the one who needs it.
    report.push_str("\n# every crate in the workspace\n");
    report.push_str(&format!("crates\t{}\n", census::CRATE_COUNT));
    report.push_str(&format!("axes\t{}\n", census::AXIS_COUNT));
    // The count above, proved against the manifest rather than trusted.
    report.push_str(&format!(
        "crates-in-manifest\t{}\n",
        match std::fs::read_to_string(workspace.join("Cargo.toml")) {
            Ok(text) => census::workspace_members(&text).len().to_string(),
            Err(error) => format!("not-measured ({error})"),
        }
    ));
    report.push_str(&format!("crate-witness-optional-absent\t{}\n", optional_absences.len()));
    for absence in &optional_absences {
        report.push_str(&format!("crate-witness-absent\t{absence}\n"));
    }
    // Rule 2, printed every build: the count and the names, because "which two" is the question.
    report.push_str(&format!("orphan-libraries\t{}\n", orphans.len()));
    for orphan in &orphans {
        report.push_str(&format!("orphan-library\t{orphan}\n"));
    }
    // The ratchet per crate: measured beside allowed, so the slack is a visible decision.
    report.push_str(&format!("prose-frame\t{}\n", census::PROSE_FRAME_PATH));
    for standing in &prose {
        report.push_str(&format!(
            "prose\t{}\tmeasured={}\tallowed={}\tslack={}\n",
            standing.crate_.name(),
            standing.measured,
            standing.allowed,
            standing.slack()
        ));
    }

    // **The spec's own declaration tables, as a section of their own.** The census's fifth axis
    // answers "does the spec declare this crate?", and a reader who cannot see which tables
    // were searched cannot tell an unclaimed crate from a mis-searched one. The count is
    // printed next to the list for the same reason.
    match census::claim_against_spec(&workspace) {
        Ok(claims) => {
            let unclaimed: Vec<&census::Claim> =
                claims.iter().filter(|claim| !claim.is_claimed()).collect();
            report.push_str(&format!(
                "census-claimed\t{}\ncensus-unclaimed\t{}\n",
                claims.len() - unclaimed.len(),
                unclaimed.len()
            ));
            for table in census::DECLARATION_TABLES {
                report.push_str(&format!("census-declaration-table\t{table}\n"));
            }
            for claim in unclaimed {
                report.push_str(&format!("census-unclaimed-by\t{}\n", claim.why_unclaimed()));
            }
        }
        Err(found) => {
            // **A claim measured against a partial spec is not a claim.** This is the axis
            // that can fail, so its own failure is printed as a refusal rather than swallowed
            // into a count that would read as zero.
            report.push_str("census-unclaimed\tnot-measured\n");
            for refusal in found.all() {
                report.push_str(&format!("census-refusal\t{}\n", refusal.detail()));
            }
        }
    }
    for reading in &the_census.readings {
        report.push_str(&format!(
            "crate\t{}\t{}\n",
            reading.crate_.name(),
            reading
                .cells
                .iter()
                .map(|cell| cell.slug())
                .collect::<Vec<_>>()
                .join("\t")
        ));
        for (axis, state, why) in reading.gaps() {
            // **`axis=` and `state=` are spelled out** because the bare pair read as a
            // contradiction: `crate-gap voxel-ui claimed inactive …` looks like the crate is
            // claimed and also inactive, when it means *the gap is on the `claimed` axis and its
            // state is `inactive`*. A report a reader has to decode is a report some readers
            // will decode wrongly.
            report.push_str(&format!(
                "crate-gap\t{}\taxis={}\tstate={}\t{}\n",
                reading.crate_.name(),
                axis.name(),
                state.slug(),
                why.unwrap_or("no reason recorded")
            ));
        }
    }

    // **Is the tree conformant?** Every axis of every crate is already measured above; this is the
    // one section that turns those measurements into a door. A refusal is rejected unless
    // `conformance/accepted-refusals.txt` has named it, and an accepted refusal is **still
    // printed** — accepted is not deleted, and an absent list accepts nothing.
    report.push_str("\n# is the tree conformant\n");
    report.push_str(&the_verdict.describe());

    // **Which parts of the tree a record names, and which it does not.** The join the patch reader
    // was missing: `get_ownership` answers *"which record names this path?"* for one path, and
    // nobody asks that about a file they have not opened — so eleven of twelve parts went unnamed,
    // and the hole was invisible because nothing looked at the whole set at once.
    //
    // **It is evidence and not a door.** A part with no record is a part whose history was never
    // written down: a hole in the record, not a broken build. Refusing here would refuse every
    // build until the backfill lands, which is how a gate teaches everyone to route around it.
    report.push_str("\n# what has a patch\n");
    let patch_set = patch::read_dir(&workspace.join("patches"));
    report.push_str(&patch_set.describe());
    report.push('\n');
    report.push_str(&patch_set.coverage(&census::Crate::ALL).describe());
    // **And every line's place, not just every part's.** A part is named by a record; a *file* is
    // the unit the work actually lands in, and the difference is where the tree's remaining debt
    // lives: 95 code files, every shader and 23 documents found by no record on the first run.
    // The list is printed whole, biggest first, because a shortened work list reads as a complete
    // one.
    report.push('\n');
    report.push_str(&patch::placement(&patch_set, &workspace).describe());

    // ---- the decision layer: both passes, side by side ----
    //
    // Computed before the report was built, so the oracle section could measure its own line;
    // written here so the report's shape is unchanged.
    report.push_str(&two.report());

    // The claims this gate must never overstate, stated in the artefact itself.
    report.push_str("\n# what this gate is not\n");
    report.push_str("integrity\testablished\n");
    report.push_str("identity\tnot-established-no-signature\n");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo build output directory"));
    fs::write(out.join("alpha-shell.shpkg"), &package).expect("write validated shell package");
    fs::write(out.join("alpha-shell.gate"), &report).expect("write shared gate evidence");

    // ---- the door, and it is fatal
    //
    // **After the report is written, on purpose.** A gate that refuses before writing its own
    // evidence makes the failure the only thing a reader can see, and the reader who most needs
    // the report is the one whose build just failed. The sections above are the measurement; this
    // is the only one that refuses, and it refuses by name.
    //
    // **Unchecked is not silenceable.** `conformance::of` never moves an unchecked cell onto the
    // accepted list, so no declaration can turn *no instrument ran* into a pass: the list says
    // which refusals the record admits, and a cell nobody measured is not one of them.
    // **The volume first, and the order is the point.** A full disk is the one refusal that makes
    // every other answer unactionable — nothing can be compiled, so the conformance list is not
    // what a reader can act on next. It is still printed either way, because this refuses after
    // the report is written.
    if let Some(refusal) = space.refusal() {
        panic!("{refusal}");
    }

    let rejected = the_verdict.rejected();
    if !the_verdict.is_active() {
        let mut named = String::new();
        for refusal in &rejected {
            named.push_str(&format!(
                "\n  rejected\t{}\t{}\t{}\t{}",
                refusal.crate_, refusal.axis, refusal.state, refusal.why
            ));
        }
        for refusal in &the_verdict.unchecked {
            named.push_str(&format!(
                "\n  unchecked\t{}\t{}\t{}\t{}",
                refusal.crate_, refusal.axis, refusal.state, refusal.why
            ));
        }
        // **An admitted key no refusal answers** is the list's own failure, and the mirror of a
        // rejection: a permission with nothing to permit, which re-admits the refusal it names if
        // that axis ever comes back. The repair is to delete the line, so the key is printed.
        for key in &the_verdict.unmatched {
            named.push_str(&format!("\n  stale-acceptance\t{key}\t(no refusal answers this key)"));
        }
        panic!(
            "the tree is not conformant: {} rejection(s), {} unchecked cell(s) and {} stale \
             acceptance(s) — and no instrument running is not a pass either. `{}` names the \
             refusals this record admits; every other one is work, and an admitted key no refusal \
             answers is a line to delete.{named}",
            rejected.len(),
            the_verdict.unchecked.len(),
            the_verdict.unmatched.len(),
            conformance::ACCEPTED_PATH
        );
    }
}
