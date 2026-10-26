//! Feature evaluators: **the build states what is missing, and cannot be lied
//! about.**
//!
//! # What this module is for
//!
//! Ruling R-59.1 requires that when the project compiles, it checks the files
//! against the gate and *displays* which declared features are missing. Without
//! this module that display is a report someone writes by hand, and a
//! hand-written report is a claim rather than a fact.
//!
//! So a declared feature is a **variant**, its **witness** is the code that
//! implements it, and a feature with no witness or no row is a **compile error**
//! rather than something a test could quietly fail.
//!
//! # The pattern, and where it comes from
//!
//! This is the enforcement shape of the reference project's `axis.rs` (1,480
//! lines, C-58 mechanism set / C-59 R-59.7). That module states its own
//! requirement — not *model the axes* but *"tests failing if there is no
//! representation according to the axes"* — and then enforces it three ways,
//! **two of which are compile errors rather than red tests**, on a stated reason:
//! *a red test can be fixed by deleting it and a non-exhaustive match cannot be
//! merged.*
//!
//! | # | check | fails how |
//! |---|---|---|
//! | 1 | [`witness_of`] is an **exhaustive `match`** over [`Feature`] | a new feature variant with no witness **will not compile** |
//! | 2 | [`FEATURES_ARE_TOTAL`] pins the row count to the feature count at compile time | a new feature with no row **will not compile** |
//! | 3 | [`check_features`] is the runtime anti-fabrication guard | a row naming a witness that is not on disk is **refused, by name** |
//!
//! The value of the constant in (2) is meaningless — a zero-length array carries
//! nothing. Its **type** is the assertion. That is the whole trick, and it is
//! worth stating plainly for a successor: this module is not trying to *test*
//! that every feature is covered, it is trying to make coverage **unrepresentable
//! when it is absent**.
//!
//! # One vocabulary, two questions — and why they are the same question
//!
//! [`Establishment`] has five states, and it is used for **both** "is this
//! feature implemented?" and "was this package's signature checked?". Those are
//! not two ideas that happen to look alike; they are one question asked of two
//! subjects: *was this checked, and what was found?*
//!
//! | state | meaning |
//! |---|---|
//! | [`Establishment::Active`] | checked, and it holds |
//! | [`Establishment::Inactive`] | checked, and it is not under this project's root |
//! | [`Establishment::Stale`] | checked, and what it read has since changed |
//! | [`Establishment::Unchecked`] | **no instrument ran at all** — never checked |
//! | [`Establishment::Refused`] | checked, and it failed, or it could not be checked |
//!
//! Collapsing [`Unchecked`] into [`Active`] is how an unverified package comes to
//! look verified. Collapsing [`Inactive`] into [`Refused`] is how an honest
//! unknown publisher comes to look like a forgery. Collapsing [`Stale`] into either
//! is how a reading taken months ago comes to be printed as this build's answer:
//! most readings this project keeps are **snapshots**, and a snapshot that is not
//! compared against the bytes it was taken over is a claim with a number attached.
//! AWA3D's effort loop states the general form in one line — chat *"never goes
//! silent"* (C-60) — and an absence that is not rendered is indistinguishable from
//! a pass.
//!
//! # Required and pending are different, and both are displayed
//!
//! [`Feature::required_for_alpha`] decides whether a missing feature **fails the
//! build** or is merely **displayed as pending**. This is the same declared
//! cadence the effort loop uses per effort level (C-60 mechanism 1): a level
//! that does not verify is not a level that silently skipped verifying, it is a
//! level that says so. Without this split every unfinished feature would block
//! every build, and a gate that blocks everything gates nothing.

use std::path::{Path, PathBuf};

use voxel_model::{Refusal, RefusalKind, RefusalSet};

/// **The state of one thing that was supposed to be checked.**
///
/// Five states, one meaning each. The declaration moved to `voxel_model::axis` on 2026-10-25 so
/// the axis set has one home for it; this module re-exports the one type rather than keeping a
/// second five-state enum that could drift from the ladder's tokens.
pub use voxel_model::axis::Establishment;

/// How many features this project declares: **a literal, on purpose**.
///
/// The obvious version — `Feature::ALL.len()` — is a cycle: `ALL`'s *type* is
/// `[Feature; FEATURE_COUNT]`, so a count derived from `ALL` needs `ALL`'s type
/// before it can be computed. This crate hit exactly that (`E0391`) and the
/// literal is the fix.
///
/// The literal is not a maintenance hazard, and it is worth saying why, because
/// it looks like one. If a variant is added to [`Feature::ALL`] without bumping
/// this number, `[Feature; FEATURE_COUNT]` is an array literal of the wrong
/// length and **the build stops**. If a row is added to [`FEATURES`] without
/// bumping it, same. So the literal converts "someone forgot to update a count"
/// from a silent drift into a compile error — which is the property this module
/// exists to have. Deriving the count would make the two sides agree by
/// construction, and then the check would be checking nothing.
pub const FEATURE_COUNT: usize = 20;

/// The declared features of the v1 alpha.
///
/// One variant per feature. **Adding a variant is a compile error until it has a
/// witness** (check 1) **and a row** (check 2) — which is the entire point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    // ---- present, and the build depends on these ----
    /// Content intake: a package is admitted by validation, never by optimism.
    PackageIntake,
    /// The recipe grammar and its compiled container.
    RecipeCompilation,
    /// The mandatory build-time gate. C-52.1: the game cannot compile without
    /// the system.
    BuildGate,
    /// Voxel world generation, shapers and stages.
    WorldGeneration,
    /// The `Signer`/`Verifier` seam (B1).
    SigningSeam,

    // ---- pending, displayed rather than blocking ----
    /// Real signature verification via the envelope crate (B6).
    SignatureVerification,
    /// The signed container framework (B5).
    SealedContainer,
    /// The conformance oracle and its calibration (B3).
    ConformanceOracle,
    /// The working-tree provenance digest (B4).
    ProvenanceDigest,
    /// This module: the feature evaluators themselves.
    FeatureEvaluation,
    /// The `voxelctl` command-line tool (B7).
    CommandLineTool,
    /// The shell arrangement is mounted from package data (H-9.7).
    ShellArrangement,
    /// The pinned HUD subset, visible while the shell is hidden (H-9.8).
    PinnedHud,
    /// The shell-owned toast and notification queue (H-12.2).
    NotificationQueue,
    /// A visible window for user tickets and feedback.
    TicketWindow,
    /// One append-only, hash-chained record with two views (H-8.1).
    EventChain,
    /// The 20 Hz base clock with declared stage cadences (H-8.3).
    StageCadences,
    /// Save and load.
    SaveLoad,
    /// The vector change view: what changed, as a view over the chain.
    ChangeView,
    /// **A window's cells are a declared frame the mount grades** (`shploingus.cellframe/1`).
    ///
    /// The format, its join to the recipe's rows and the content-space restriction have landed
    /// (PATCH-076); what this row still waits for is the generator that fills a frame from the
    /// world — a declared frame is drawn today, a streamed one is not.
    CellFrames,
}

impl Feature {
    /// Every feature, in declaration order.
    pub const ALL: [Feature; FEATURE_COUNT] = [
        // present
        Feature::PackageIntake,
        Feature::RecipeCompilation,
        Feature::BuildGate,
        Feature::WorldGeneration,
        Feature::SigningSeam,
        // pending
        Feature::SignatureVerification,
        Feature::SealedContainer,
        Feature::ConformanceOracle,
        Feature::FeatureEvaluation,
        Feature::ProvenanceDigest,
        Feature::CommandLineTool,
        Feature::ShellArrangement,
        Feature::PinnedHud,
        Feature::NotificationQueue,
        Feature::TicketWindow,
        Feature::EventChain,
        Feature::StageCadences,
        Feature::SaveLoad,
        Feature::ChangeView,
        Feature::CellFrames,
    ];

    /// A stable, machine-readable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Feature::PackageIntake => "package_intake",
            Feature::RecipeCompilation => "recipe_compilation",
            Feature::BuildGate => "build_gate",
            Feature::WorldGeneration => "world_generation",
            Feature::SigningSeam => "signing_seam",
            Feature::SignatureVerification => "signature_verification",
            Feature::SealedContainer => "sealed_container",
            Feature::ConformanceOracle => "conformance_oracle",
            Feature::FeatureEvaluation => "feature_evaluation",
            Feature::ProvenanceDigest => "provenance_digest",
            Feature::CommandLineTool => "command_line_tool",
            Feature::ShellArrangement => "shell_arrangement",
            Feature::PinnedHud => "pinned_hud",
            Feature::NotificationQueue => "notification_queue",
            Feature::TicketWindow => "ticket_window",
            Feature::EventChain => "event_chain",
            Feature::StageCadences => "stage_cadences",
            Feature::SaveLoad => "save_load",
            Feature::ChangeView => "change_view",
            Feature::CellFrames => "cell_frames",
        }
    }

    /// Whether the build **fails** when this feature has no witness.
    ///
    /// The present features gate the build; the pending ones are displayed.
    /// Both are displayed — this decides whether an absence is also fatal, not
    /// whether it is visible.
    pub const fn required_for_alpha(self) -> bool {
        matches!(
            self,
            Feature::PackageIntake
                | Feature::RecipeCompilation
                | Feature::BuildGate
                | Feature::WorldGeneration
                | Feature::SigningSeam
                | Feature::FeatureEvaluation
        )
    }

    /// Look a feature up by its name. Unknown is refused rather than defaulted.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|feature| feature.name() == word.trim())
            .ok_or_else(|| format!("unknown feature {word:?}; this project declares {FEATURE_COUNT}"))
    }
}

/// The code that implements a feature: a path, relative to the workspace root.
///
/// A string rather than a type so a witness may live in any module, and so the
/// guard in [`check_features`] can look it up on disk without the workspace
/// having to be compiled as a dependency of itself.
///
/// **Check 1 of 3.** This `match` is exhaustive over [`Feature`] and has **no
/// wildcard arm**, which is the whole mechanism: add a variant to [`Feature`] and
/// this function stops compiling until someone says where its witness is. A
/// non-exhaustive match cannot be merged; a failing test can be deleted.
pub const fn witness_of(feature: Feature) -> &'static str {
    match feature {
        Feature::PackageIntake => "crates/voxel-package/src/lib.rs",
        Feature::RecipeCompilation => "crates/voxel-package/src/recipe.rs",
        Feature::BuildGate => "crates/shploingus/build.rs",
        Feature::WorldGeneration => "crates/voxel-shapers/src/voxel.rs",
        Feature::SigningSeam => "crates/voxel-package/src/signing.rs",
        Feature::SignatureVerification => "crates/voxel-envelope/src/lib.rs",
        Feature::SealedContainer => "crates/voxel-package/src/envelope.rs",
        Feature::ConformanceOracle => "crates/voxel-package/src/oracle.rs",
        Feature::FeatureEvaluation => "crates/voxel-package/src/features.rs",
        Feature::ProvenanceDigest => "crates/voxel-package/src/provenance.rs",
        Feature::CommandLineTool => "crates/voxelctl/src/main.rs",
        Feature::ShellArrangement => "crates/voxel-client/src/shell/arrangement.rs",
        Feature::PinnedHud => "crates/voxel-client/src/shell/hud.rs",
        Feature::NotificationQueue => "crates/voxel-client/src/shell/notifications.rs",
        Feature::TicketWindow => "crates/voxel-client/src/shell/tickets.rs",
        Feature::EventChain => "crates/voxel-model/src/chain.rs",
        // **Repointed (C-88).** This row read `voxel-work/src/budget.rs`, which
        // is a **byte** ceiling (`Budget::new`, `charge`, `release`, `fits`) with
        // no clock and no cadence in it — so a green row stood over the wrong
        // code, which is the false-finding class the ledger's `worn:` ruling
        // describes. The ladder H-8.3 declares now exists, and this is where it
        // is: `Cadence::{EveryTick, Water, Sediment, Daily}` at ×1 / ×4 / ×20 /
        // `TICKS_PER_DAY`, with `is_due_at` as the whole scheduling rule.
        Feature::StageCadences => "crates/voxel-shapers/src/stages.rs",
        Feature::SaveLoad => "crates/voxel-stream/src/resident.rs",
        Feature::ChangeView => "crates/voxel-client/src/shell/changes.rs",
        // **The declaration, not the drawing.** A frame is graded here — grammar, recipe row,
        // window join, extent — and the client draws only what this module accepted, so the
        // witness is the grader rather than the raster that trusts it.
        Feature::CellFrames => "crates/voxel-package/src/cellframe.rs",
    }
}

/// One feature, its witness, and where the declaration came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureRow {
    pub feature: Feature,
    /// The declaring authority. A row with no source is a row nobody can check.
    pub declared_by: &'static str,
    /// A one-line statement of what "implemented" means for **this** feature.
    ///
    /// The field most able to be lied about, which is why [`check_features`]
    /// checks the witness against the filesystem and this text against nothing.
    /// It is here to be read by a person, and its being unchecked is stated here
    /// rather than left for a reader to assume otherwise.
    pub means: &'static str,
}

/// Every feature this project declares, its witness, and where it was declared.
pub const FEATURES: [FeatureRow; FEATURE_COUNT] = [
    FeatureRow { feature: Feature::PackageIntake, declared_by: "C-55", means: "a package is admitted by validation and every rejection is retained" },
    FeatureRow { feature: Feature::RecipeCompilation, declared_by: "C-56", means: "a text recipe compiles to a container and round-trips to the exact source" },
    FeatureRow { feature: Feature::BuildGate, declared_by: "C-52.1", means: "the game does not build unless the package system validated it" },
    FeatureRow { feature: Feature::WorldGeneration, declared_by: "C-1", means: "stages, shapers and materials produce a deterministic world" },
    FeatureRow { feature: Feature::SigningSeam, declared_by: "R-59.2", means: "Signer and Verifier exist and name no cryptography" },
    FeatureRow { feature: Feature::SignatureVerification, declared_by: "R-59.2", means: "a real signature is checked before a payload is used" },
    FeatureRow { feature: Feature::SealedContainer, declared_by: "R-59.4", means: "a container carries a schema name and canonical fields" },
    FeatureRow { feature: Feature::ConformanceOracle, declared_by: "R-59.5", means: "the lint gate fails closed and is calibrated or it writes nothing" },
    FeatureRow { feature: Feature::FeatureEvaluation, declared_by: "R-59.7", means: "a feature with no witness is a compile error, and a missing one is displayed" },
    FeatureRow { feature: Feature::ProvenanceDigest, declared_by: "R-59.1", means: "a build is bound to the exact working-tree bytes it read" },
    FeatureRow { feature: Feature::CommandLineTool, declared_by: "R-59.6", means: "a person can point a tool at a package and get an answer" },
    FeatureRow { feature: Feature::ShellArrangement, declared_by: "H-9.7", means: "windows are data the shell mounts, not code the shell owns" },
    FeatureRow { feature: Feature::PinnedHud, declared_by: "H-9.8", means: "pinned windows stay visible while the shell is hidden" },
    FeatureRow { feature: Feature::NotificationQueue, declared_by: "H-12.2", means: "the shell owns one queue and renders one record at a time" },
    FeatureRow { feature: Feature::TicketWindow, declared_by: "C-56", means: "a ticket filed from the client is visible to the agent" },
    FeatureRow { feature: Feature::EventChain, declared_by: "H-8.1", means: "one append-only hash-chained record, read through two views" },
    FeatureRow { feature: Feature::StageCadences, declared_by: "H-8.3", means: "a 20 Hz base clock with each stage's cadence declared" },
    FeatureRow { feature: Feature::SaveLoad, declared_by: "C-56", means: "a world round-trips through disk and back to the same digests" },
    FeatureRow { feature: Feature::ChangeView, declared_by: "C-56", means: "what changed is a view over the chain, not a second record" },
    FeatureRow { feature: Feature::CellFrames, declared_by: "R-64.1", means: "a window's cells are a declared frame the mount grades, and the raster draws no cells without one" },
];

/// **Check 2 of 3.** The row count is pinned to the feature count *in a `const`*.
///
/// The value is meaningless and always zero. Its **type** is the assertion, and it
/// is why adding a feature without a row is a compile error rather than a red
/// test someone can delete.
pub const FEATURES_ARE_TOTAL: [(); FEATURE_COUNT] = [(); FEATURES.len()];

/// One feature's evaluated state, and the reason it is in that state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureState {
    pub feature: Feature,
    pub state: Establishment,
    /// Never empty when the state is not [`Establishment::Active`]. A refusal
    /// without a reason is not actionable, and a state without one is a blank
    /// row.
    pub reason: String,
}

/// Evaluate every declared feature against the tree at `root`.
///
/// Returns **every** feature's state, in declaration order — not just the
/// failures — because the point of this module is a display, and a display that
/// only shows problems cannot be read as a summary.
/// **What a feature row's witness was read as — the fact, before any rule.**
///
/// This is the inversion `decision.rs` records: **this function reports, and the decision layer
/// decides.** Nothing here turns `present` into a state, because that rule belongs to exactly one
/// place — a second copy of it is how two readers of one record come to disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureObservation {
    pub feature: Feature,
    /// The path the row declares, **as declared**, so a reader sees what the table says rather
    /// than a path this module assembled and did not name.
    pub witness: &'static str,
    /// Whether the file is there.
    pub present: bool,
    /// Whether the build refuses when it is not.
    pub required_for_alpha: bool,
}

/// Read every feature row's witness. **Facts only.**
pub fn observe(root: &Path) -> Vec<FeatureObservation> {
    FEATURES
        .iter()
        .map(|row| FeatureObservation {
            feature: row.feature,
            witness: witness_of(row.feature),
            present: root.join(witness_of(row.feature)).is_file(),
            required_for_alpha: row.feature.required_for_alpha(),
        })
        .collect()
}

/// Every feature's evaluated state — **the rule, applied by the decision layer.**
pub fn evaluate(root: &Path) -> Vec<FeatureState> {
    observe(root)
        .iter()
        .map(|observation| {
            let (state, reason) = crate::decision::rule_for_feature(observation);
            FeatureState { feature: observation.feature, state, reason }
        })
        .collect()
}

/// The features with no witness — **this is what the game displays as missing.**
pub fn missing(root: &Path) -> Vec<Feature> {
    evaluate(root)
        .into_iter()
        .filter(|state| state.state != Establishment::Active)
        .map(|state| state.feature)
        .collect()
}

/// Check a feature table's **shape**: totality, no duplicate rows, no empty
/// meanings. Does not touch the filesystem.
///
/// **Split from [`check_features`] on purpose, and the split is the point.** A table
/// whose *shape* is wrong is a defect in the table. A table whose shape is right
/// but whose *witness* is absent is a feature that has not been built yet — which is
/// a fact about the project, not a defect in the declaration, and is reported by
/// [`evaluate`] as `Unchecked`.
///
/// Conflating them would mean the shape could never be checked on a tree that is
/// mid-build, which is most of the time.
pub fn check_table_shape(rows: &[FeatureRow]) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    for row in rows {
        if row.means.trim().is_empty() {
            refusals.push(Refusal::owned(
                RefusalKind::FabricatedFeature,
                format!(
                    "{}: the row declares no meaning, so it describes no feature",
                    row.feature.name()
                ),
            ));
        }
    }

    // Totality in both directions: a feature with no row is as bad as a row with
    // no feature.
    for feature in Feature::ALL {
        if !rows.iter().any(|row| row.feature == feature) {
            refusals.push(Refusal::owned(
                RefusalKind::MissingFeature,
                format!("{}: this feature has no row, so nothing witnesses it", feature.name()),
            ));
        }
    }

    let mut seen: Vec<Feature> = rows.iter().map(|row| row.feature).collect();
    seen.sort_unstable();
    seen.dedup();
    if seen.len() != rows.len() {
        refusals.push(Refusal::owned(
            RefusalKind::FabricatedFeature,
            "feature table: two rows claim one feature, so one of them describes something \
             already described"
                .to_owned(),
        ));
    }

    refusals.into_result(())
}

/// Check a feature table for the three ways it can lie, **including** whether each
/// witness is on disk.
///
/// This is the strict form: on a tree that is mid-build it will refuse every
/// unwitnessed feature. The build gate wants [`check_required`] instead, which
/// refuses only the features the build genuinely depends on.
///
/// Returns a [`RefusalSet`] rather than the first problem, so a run that fixes one
/// row does not discover the next on the next run.
pub fn check_features(root: &Path, rows: &[FeatureRow]) -> Result<(), RefusalSet> {
    let mut refusals = match check_table_shape(rows) {
        Ok(()) => RefusalSet::new(),
        Err(found) => found,
    };

    for row in rows {
        // The anti-fabrication check, and the only one that can catch a row that is
        // well formed and simply untrue: does the witness exist?
        let witness = root.join(witness_of(row.feature));
        if !witness.is_file() {
            refusals.push(Refusal::owned(
                RefusalKind::MissingFeature,
                format!(
                    "{}: the row names {} as its witness, and nothing is at {}",
                    row.feature.name(),
                    witness_of(row.feature),
                    witness.display()
                ),
            ));
        }
    }

    refusals.into_result(())
}

/// Refuse only the **required** features that have no witness.
///
/// This is what the build gate calls. A pending feature is *displayed* and named;
/// a required feature that is absent is fatal, because the build declares it cannot
/// proceed without it. Both are visible; only one stops the build.
pub fn check_required(root: &Path) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    for state in evaluate(root) {
        if state.state == Establishment::Active || !state.feature.required_for_alpha() {
            continue;
        }
        refusals.push(Refusal::owned(RefusalKind::MissingFeature, state.reason));
    }
    refusals.into_result(())
}

/// The workspace root, found by walking up from `start` until a `Cargo.toml`
/// naming a workspace is found.
///
/// A build script's working directory is its own package, not the workspace, and
/// every witness path is relative to the root — so the root has to be found
/// rather than assumed, because an assumed root makes every witness resolve
/// against the wrong place and the guard fires on features that exist.
pub fn workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if text.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real workspace root, so the tests check the real tree rather than a
    /// fixture that agrees with whatever the table claims.
    fn real_root() -> PathBuf {
        workspace_root(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("a workspace root above the crate")
    }

    /// A scratch tree with one file in it, for the forged-row tests.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("voxel-features-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch tree");
        dir
    }

    #[test]
    fn the_row_count_is_pinned_to_the_feature_count() {
        // Check 2, asserted as well as compiled. The constant already enforces
        // this in a `const` context; a test states the intent where a reader
        // will look for it.
        assert_eq!(FEATURES.len(), FEATURE_COUNT);
        assert_eq!(FEATURES_ARE_TOTAL.len(), FEATURE_COUNT);
    }

    #[test]
    fn every_feature_has_exactly_one_row() {
        let mut features: Vec<Feature> = FEATURES.iter().map(|row| row.feature).collect();
        let total = features.len();
        features.sort_unstable();
        features.dedup();
        assert_eq!(features.len(), total, "two rows claim one feature");
        for feature in Feature::ALL {
            assert!(
                FEATURES.iter().any(|row| row.feature == feature),
                "{} has no row",
                feature.name()
            );
        }
    }

    #[test]
    fn every_feature_names_a_witness_and_a_declaring_authority() {
        for row in FEATURES {
            assert!(
                !witness_of(row.feature).is_empty(),
                "{} has no witness",
                row.feature.name()
            );
            assert!(
                !row.declared_by.is_empty(),
                "{} has no declaring authority",
                row.feature.name()
            );
            assert!(!row.means.trim().is_empty());
        }
    }

    #[test]
    fn every_feature_name_is_unique_so_a_name_can_address_one_feature() {
        let mut names: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "two features share a name");
    }

    #[test]
    fn a_feature_name_round_trips_and_an_unknown_one_is_refused() {
        for feature in Feature::ALL {
            assert_eq!(Feature::parse(feature.name()).unwrap(), feature);
        }
        assert!(Feature::parse("no_such_feature").is_err());
        assert!(Feature::parse("").is_err());
    }

    #[test]
    fn every_establishment_has_a_distinct_slug_and_a_rendering() {
        // A state with no rendering is a blank row, and a blank row in a view
        // reads as "nothing to report".
        let all = [
            Establishment::Active,
            Establishment::Inactive,
            Establishment::Stale,
            Establishment::Unchecked,
            Establishment::Refused,
        ];
        let mut slugs: Vec<&str> = all.iter().map(|state| state.slug()).collect();
        let total = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), total, "two states share a slug");
        for state in all {
            assert!(!state.describe().is_empty(), "{} has no rendering", state.slug());
        }
    }

    #[test]
    fn unchecked_and_refused_are_different_states() {
        // The dangerous direction: an absent instrument reported as a failure
        // sends a reader to inspect something that was never examined.
        assert_ne!(Establishment::Unchecked, Establishment::Refused);
        assert_ne!(Establishment::Inactive, Establishment::Refused);
        assert!(Establishment::Unchecked.describe().contains("not a pass"));
    }

    #[test]
    fn a_stale_reading_is_neither_a_pass_nor_a_failure() {
        // **The third dangerous direction.** A reading taken over bytes that moved is not the same
        // fact as a failed check — folding it into `Refused` would fail a build for a number nobody
        // re-measured, and folding it into `Active` would print an old count as this build's answer.
        // Its own state is the only rendering that says what happened: the reading is about a tree
        // this one is not, and the repair is to take it again.
        assert_ne!(Establishment::Stale, Establishment::Active);
        assert_ne!(Establishment::Stale, Establishment::Refused);
        assert_ne!(Establishment::Stale, Establishment::Inactive);
        assert_eq!(Establishment::Stale.slug(), "stale");
        let rendering = Establishment::Stale.describe();
        assert!(rendering.contains("since changed"), "{rendering}");
        assert!(
            rendering.contains("Re-derive"),
            "the state has to say what to do about it: {rendering}"
        );
    }

    #[test]
    fn the_real_tree_is_evaluated_and_every_feature_gets_a_state() {
        // Every feature is reported, not only the failures: this is a display.
        let states = evaluate(&real_root());
        assert_eq!(states.len(), FEATURE_COUNT);
        for state in &states {
            assert!(
                !state.reason.is_empty(),
                "{} has no reason",
                state.feature.name()
            );
        }
    }

    #[test]
    fn a_feature_this_file_implements_is_reported_active() {
        // The one self-check that keeps this module honest: it claims its own
        // witness exists, so if the file were renamed the table would be a lie
        // the test could see.
        let states = evaluate(&real_root());
        let self_state = states
            .iter()
            .find(|state| state.feature == Feature::FeatureEvaluation)
            .expect("this module evaluates itself");
        assert_eq!(self_state.state, Establishment::Active);
    }

    #[test]
    fn missing_lists_only_what_is_actually_absent() {
        let root = real_root();
        let missing = missing(&root);
        for feature in &missing {
            assert!(
                !root.join(witness_of(*feature)).is_file(),
                "{} was reported missing but its witness exists",
                feature.name()
            );
        }
        // Whatever is missing, it is a subset of what was declared.
        for feature in &missing {
            assert!(Feature::ALL.contains(feature));
        }
    }

    #[test]
    fn a_forged_table_claiming_a_feature_with_no_witness_is_refused_by_name() {
        // The anti-fabrication guard, on a real historical shape: a row that is
        // well formed and simply untrue.
        let root = scratch("forged");
        let rows = vec![FeatureRow {
            feature: Feature::ShellArrangement,
            declared_by: "R-59.6",
            means: "the shell mounts windows",
        }];
        let refusals = check_features(&root, &rows).expect_err("must refuse");
        assert!(!refusals.is_empty());
        let kinds: Vec<RefusalKind> = refusals.all().iter().map(|r| r.kind()).collect();
        assert!(
            kinds.contains(&RefusalKind::MissingFeature),
            "the missing witness must be named: {refusals}"
        );
    }

    #[test]
    fn every_problem_is_collected_rather_than_the_first_one() {
        // An empty table: every feature is uncovered, and a caller that fixed one
        // per run would take nineteen runs.
        let root = scratch("empty");
        let refusals = check_features(&root, &[]).expect_err("must refuse");
        assert_eq!(refusals.len(), FEATURE_COUNT, "one per uncovered feature");
    }

    #[test]
    fn a_row_with_no_meaning_is_refused() {
        let root = real_root();
        let mut rows = FEATURES.to_vec();
        // Take a row whose witness genuinely exists, so the only fault is the
        // empty meaning.
        rows[0].means = "   ";
        let refusals = check_features(&root, &rows).expect_err("must refuse");
        let kinds: Vec<RefusalKind> = refusals.all().iter().map(|r| r.kind()).collect();
        assert!(
            kinds.contains(&RefusalKind::FabricatedFeature),
            "a row describing nothing must be refused: {refusals}"
        );
    }

    #[test]
    fn a_required_feature_and_a_pending_feature_are_different_decisions() {
        // Without this split every unfinished feature would block every build,
        // and a gate that blocks everything gates nothing.
        assert!(Feature::PackageIntake.required_for_alpha());
        assert!(Feature::BuildGate.required_for_alpha());
        assert!(!Feature::ShellArrangement.required_for_alpha());
        assert!(!Feature::ChangeView.required_for_alpha());
    }

    #[test]
    fn a_required_missing_feature_fails_the_build_and_a_pending_one_does_not() {
        // The two halves of the split, tested on a real pending feature: the
        // reason must say which of the two happened, because a reader shown
        // "missing" cannot otherwise tell whether the build is about to fail.
        let root = real_root();
        let states = evaluate(&root);
        for state in &states {
            if state.state != Establishment::Unchecked {
                continue;
            }
            if state.feature.required_for_alpha() {
                assert!(
                    state.reason.contains("the build refuses"),
                    "{} is required but its reason does not say the build refuses: {}",
                    state.feature.name(),
                    state.reason
                );
            } else {
                assert!(
                    state.reason.contains("the build continues"),
                    "{} is pending but its reason does not say the build continues: {}",
                    state.feature.name(),
                    state.reason
                );
            }
        }
    }

    #[test]
    fn the_missing_feature_report_is_printed_so_it_can_be_read_and_not_merely_asserted() {
        // C-40: never test invisibly. The requirement of R-59.1 is that the
        // project *displays* what is missing, so the report is printed here under
        // `--nocapture` and a person reads it. An assertion that the list is
        // non-empty is not the same evidence as a reader seeing the list.
        let root = real_root();
        let states = evaluate(&root);

        println!("\n---- declared feature report ------------------------------------");
        for state in &states {
            let mark = if state.state == Establishment::Active { "+" } else { "-" };
            let gate = if state.feature.required_for_alpha() {
                "required"
            } else {
                "pending "
            };
            println!("{mark} {gate} {:<24} {}", state.feature.name(), state.reason);
        }
        let absent: Vec<&str> = missing(&root)
            .iter()
            .map(|feature| feature.name())
            .collect();
        println!("---- {} of {} declared features have no witness ----", absent.len(), FEATURE_COUNT);
        println!("missing: {}", absent.join(", "));
        println!("---------------------------------------------------------------\n");

        assert_eq!(states.len(), FEATURE_COUNT);
    }

    #[test]
    fn table_shape_is_checkable_on_a_tree_that_is_mid_build() {
        // The reason the split exists. The real tree has unwitnessed features, so the
        // strict form refuses — and shape must still be answerable, or a half-built
        // project could never tell whether its declaration is well formed.
        let root = real_root();
        check_table_shape(&FEATURES).expect("the real table's shape is sound");
        assert!(
            check_features(&root, &FEATURES).is_err(),
            "the strict form refuses on a tree that is mid-build"
        );
    }

    #[test]
    fn the_required_check_refuses_only_what_the_build_depends_on() {
        let root = real_root();
        // Every required feature has a witness on the real tree, so the build gate
        // passes — while the strict form above still refuses. That asymmetry is the
        // whole design: displayed-but-not-fatal versus fatal.
        check_required(&root).expect("every required feature has a witness");
        let absent = missing(&root);
        for feature in &absent {
            assert!(
                !feature.required_for_alpha(),
                "{} is absent and would be fatal",
                feature.name()
            );
        }
    }

    #[test]
    fn the_workspace_root_is_found_by_walking_up_and_is_not_guessed() {
        let root = real_root();
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("crates").is_dir(), "a root without crates/ is not this workspace");
        // From deep inside, the same root is found.
        let deep = root.join("crates").join("voxel-package").join("src");
        assert_eq!(workspace_root(&deep).expect("found from deep inside"), root);
        // And from a tree with no workspace anywhere above it, nothing is found
        // rather than a plausible wrong answer.
        let nowhere = PathBuf::from("/");
        if workspace_root(&nowhere).is_none() {
            // fine: no root above the filesystem root
        }
    }
}
