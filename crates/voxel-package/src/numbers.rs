//! **The record's numbers, checked against the record's own text.**
//!
//! # The gap this closes, measured rather than asserted
//!
//! Eleven `check_*` functions exist in this crate. **Not one of them checks a number in a
//! document.** [`crate::record::check_declarations`] reads real text and looks for an
//! **anchor**; [`crate::features::check_features`] asks whether a witness file exists; the
//! census, the policy matrix and the recipe check *shape*. Three tests in the workspace open a
//! `docs/` file at all — `claimed_by_spec.rs`, and two in `compendium.rs` — and all three check
//! **structure**: is a heading defining, is a name a table row, is a line the record's.
//!
//! **So every numeric pin in this repository asserts a constant against a constant.** C-10's
//! pin is `blocks_at_level(19) == 16_777_216` and C-28's is three `DENSE_SAMPLES_*` values —
//! both correct, both unable to see the sentence they justify. Revert `docs/DESIGN-SPEC.md` to
//! *"no dense storage at all"* and **both stay green**, because no pin reads a document.
//!
//! That is how one number came to exist in four places and be corrected in two. C-28 fixed
//! `APPEARANCE.md` §2 and `voxel-model/src/subvoxel.rs`; it never mentioned `docs/LEDGER.md`
//! **C-2** or **`docs/DESIGN-SPEC.md` §1.1**, and both still carry the pre-C-28 figure —
//! 16,777,216 read as *per voxel*, which is 64× low. This module is what makes the fourth copy
//! impossible to forget.
//!
//! **And then the base itself moved.** C-163 corrected `DETAIL_SIDE` from 256 per subvoxel edge
//! to **16** — a subvoxel face carries 16×16 = 256 samples, so the base the record calls 256 is
//! 16 per axis — and every figure above it moved with it: the corrected dense counts are
//! **4,096** samples per subvoxel and **262,144** per voxel, which is 4,096× below the 256-era
//! figures the documents still carry. The rows below report *those* corrections; the documents
//! remain the user's to correct (C-38).
//!
//! # The shape, and C-63.2's rule is why it is this shape
//!
//! > *"No anchor table was written, deliberately. The obvious design is a table of anchors per
//! > document, checked against itself — which is precisely the defect `features::check_features`
//! > exists to catch."*
//!
//! So each row carries **three** things and the check reads the **fourth** from disk: which
//! document, **the literal text the document must contain**, and **the value that text is
//! required to carry**. The number in the document is parsed out of the sentence the row names.
//! **A row cannot be satisfied by agreeing with itself**, because the sentence it quotes is
//! either in the file or it is not.
//!
//! # What a row may and may not say about a stale figure
//!
//! **`expected` is optional, and that is the whole reason this module can exist without
//! touching a document the user owns.** C-34 and C-38 established the practice twice:
//!
//! > *"`APPEARANCE.md`'s S0 claim is now **stale in the evidence and untouched in the document** —
//! > the document is the user's."* (C-34)
//!
//! > *"`docs/APPEARANCE.md` is untouched and `derive` is untouched … which value belongs to a
//! > given rock is how the world looks, and that is the reserved area."* (C-38)
//!
//! So a row may declare that a document **still says the wrong thing**, with the correction
//! recorded beside it and the refusal carrying the right value. **Correcting prose in a
//! reserved-area document is the user's hand, not this crate's**, and a gate that quietly
//! rewrote it would be the exact failure C-68.1 names — an instrument grading itself with its
//! own prose. **The gate's job is to say the document disagrees, in those words, every build.**
//!
//! # What it does not claim
//!
//! **It checks declared numbers, not all numbers.** A number nobody declared is invisible here,
//! and this table cannot find it — C-64.1's defect, one level down. `unpinned` is the honest
//! word and the gate prints it, so the gap is a number a reader can see rather than a silence.

use std::path::Path;

use voxel_model::refusal::{RefusalKind, RefusalSet};

use crate::features::workspace_root;

/// The number of declared rows, as a **literal**.
///
/// **A literal and not `CLAIMS.len()`**, for `census::CRATES_ARE_TOTAL`'s reason: deriving it
/// would make the count and the list agree by construction, so the check would check nothing.
/// The *value* is meaningless; the **type** is the assertion.
pub const CLAIM_COUNT: usize = 6;

/// A declared number, and where the record says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberClaim {
    /// A stable name for the claim. What the gate prints.
    pub name: &'static str,
    /// The document that carries the figure, relative to the workspace root.
    pub document: &'static str,
    /// The **literal text that must be present**. Read from disk, never derived from the
    /// document — this is what stops the row agreeing with itself.
    pub sentence: &'static str,
    /// The value `sentence` is required to carry, as written in the record.
    ///
    /// **Digits as they appear, not a parsed number.** The comparison is textual on purpose:
    /// a row that parsed `16_777_216` into a `u64` and compared numerically would accept a
    /// sentence written `16.777.216`, and the point of the table is that the *prose* says the
    /// number. [`Self::expected`] is the parsed form, and it is what the refusal prints.
    pub expects: Option<&'static str>,
    /// The value the record says is correct, when the document is known to be stale.
    ///
    /// **`Some` here is a finding, not a fix.** It says *this document disagrees with the
    /// record*, and the gate prints it on every build until the user corrects the prose. See
    /// the module header on reserved-area documents.
    pub correction: Option<&'static str>,
    /// Why the value is what it is. Read by a person, checked by nothing — stated rather than
    /// implied, exactly as [`crate::record::Document::purpose`] does.
    pub why: &'static str,
}

impl NumberClaim {
    /// The value the document must carry, or the correction when it is stale.
    pub const fn expected(&self) -> &'static str {
        match self.expects {
            Some(expects) => expects,
            None => match self.correction {
                Some(correction) => correction,
                // Unreachable through [`CLAIMS`], and the panic names it rather than
                // defaulting — a row that declares neither is a row with nothing to check.
                None => "<no expected value and no correction: the row declares nothing>",
            },
        }
    }

    /// Whether this row is a **known** disagreement rather than a passing check.
    pub const fn is_stale(&self) -> bool {
        self.expects.is_none()
    }
}

/// Every declared number, and the literal sentence each one is pinned to.
///
/// **Six rows, four documents, four findings already recorded rather than fixed.** All four are
/// dense-sample counts: `APPEARANCE.md`'s pair is 4,096× high since C-163 moved the base to 16
/// per axis, and `C-2`/`DESIGN-SPEC` read the pre-C-28 figure on top of that. **None is corrected
/// here** because C-38 reserves appearance prose and the record's own lines are the user's hand.
pub const CLAIMS: &[NumberClaim] = &[
    NumberClaim {
        name: "ladder-top-level-blocks",
        document: "docs/DESIGN-SPEC.md",
        sentence: "`32 · 2¹⁹ = 16,777,216` blocks",
        expects: Some("16,777,216"),
        correction: None,
        why: "C-10. 20 levels at 32 blocks wide; the record's *justification* (16.8 km) was \
               wrong by 1000× and the ruling was kept. The block count itself is right. This row \
               first named docs/LEDGER.md, which does not carry the sentence — and the check \
               caught it on its first run, which is the point of reading the document rather \
               than trusting the table.",
    },
    NumberClaim {
        name: "ladder-top-level-km",
        document: "docs/DESIGN-SPEC.md",
        sentence: "16,777 km",
        expects: Some("16,777 km"),
        correction: None,
        why: "C-10, carried into the design authority's own ⚠️ note ten lines below the figure \
               it corrects. This is the row that proves the note and the number coexist.",
    },
    NumberClaim {
        name: "dense-samples-per-subvoxel",
        document: "docs/APPEARANCE.md",
        sentence: "**16,777,216**",
        // **Superseded in turn, by C-163.** 16,777,216 was 256³; with `DETAIL_SIDE = 16` the
        // count per subvoxel is 16³ = 4,096.
        expects: None,
        correction: Some("4,096"),
        why: "C-28's correction assumed the 256-per-axis base. C-163 corrected the base to 16 per \
              subvoxel edge, so the figure this document carries is 4,096× high.",
    },
    NumberClaim {
        name: "dense-samples-per-voxel",
        document: "docs/APPEARANCE.md",
        sentence: "**1,073,741,824**",
        // **The same C-163 move, one level up.** 64 subvoxels of 4,096 is 262,144 samples.
        expects: None,
        correction: Some("262,144"),
        why: "C-28's correction assumed the 256-per-axis base. With C-163's base this is 4,096× \
              high; the corrected count is 64 × 16³ = 262,144.",
    },
    NumberClaim {
        name: "dense-samples-per-voxel-ledger",
        document: "docs/LEDGER.md",
        sentence: "dense storage is 16.7M samples/voxel",
        // **Stale, and deliberately.** C-2 states the pre-C-28 figure; C-163 then moved the
        // base, so the corrected figure is 262,144 rather than C-28's 1,073,741,824.
        expects: None,
        correction: Some("262,144"),
        why: "C-2, and the finding this module exists for. C-28 corrected APPEARANCE.md and \
               subvoxel.rs and not this line. **The prose is not edited here**: C-2 is stamped \
               OPEN TO CORRECTION, appearance is the user's area (C-27, C-38), and the \
               correction belongs in their hand.",
    },
    NumberClaim {
        name: "dense-samples-per-voxel-spec",
        document: "docs/DESIGN-SPEC.md",
        sentence: "16,777,216 samples per voxel",
        // **Stale, and in the design authority itself.**
        expects: None,
        correction: Some("262,144"),
        why: "The same drift as the row above, in the document whose first line calls it *the \
               authority for shploingus-clean*: C-28 never mentioned this file, and C-163's base \
               correction (16 per subvoxel axis) moved the figure again. Left standing so the \
               gate names it on every build rather than once.",
    },
];

/// Every row's document, and whether the count is what it claims to be.
pub const CLAIMS_ARE_TOTAL: [(); CLAIM_COUNT] = [(); CLAIMS.len()];

/// **What one claim's document actually contained. A report, and nothing else.**
///
/// **This type carries no verdict, and that is the whole point of its shape.** The first
/// version of this module returned `Reading::{Agrees, Stale, Absent}` from the reader — three
/// states that *are* a decision, taken by the thing that opened the file. That is C-84.2's
/// inversion reproduced in a fourth place: *"the readers report; the layer decides."*
///
/// So the reader answers two questions and no more: **was the document readable**, and **what
/// did the line carrying the sentence say**. [`crate::decision::rule_for_number`] turns that
/// into a state, exactly as `decision::rule_for_axis` does for the census.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberObservation {
    /// The row this observation is about.
    pub claim: &'static NumberClaim,
    /// Whether the document could be read at all. **`false` is never agreement** — D107.
    pub readable: bool,
    /// Why it could not be read, when it could not.
    pub unreadable: Option<String>,
    /// The **line** of the document carrying `sentence`, verbatim, or `None` when the sentence
    /// is not there.
    ///
    /// **A line rather than a parsed number on purpose.** Deciding whether that line carries the
    /// right figure is the layer's job; extracting the figure here would be the reader deciding
    /// with its own parser, which is the defect [`crate::record`] already refuses — *"a
    /// judgement here would be a fabricated one."*
    pub line: Option<String>,
}

impl NumberObservation {
    /// Read one claim out of its document, by opening the file.
    ///
    /// **Opens the real document and looks for `sentence` in it.** There is no cache, no parsed
    /// side-table and no second copy of the prose: the sentence the row names is either in the
    /// file or it is not, which is the only reason this table is not `features::check_features`
    /// grading itself.
    pub fn read(root: &Path, claim: &'static NumberClaim) -> Self {
        let path = root.join(claim.document);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(why) => {
                return Self {
                    claim,
                    readable: false,
                    unreadable: Some(why.to_string()),
                    line: None,
                }
            }
        };
        // **The line, not the document.** A number that appears elsewhere in the file cannot
        // satisfy a row about this sentence, and searching the whole document would let it.
        let line = text
            .lines()
            .find(|line| line.contains(claim.sentence))
            .map(|line| line.to_string());
        Self {
            claim,
            readable: true,
            unreadable: None,
            line,
        }
    }
}

/// Read every declared number. **A reader: it opens files and reports, and it rules on nothing.**
pub fn observe(root: &Path) -> Vec<NumberObservation> {
    CLAIMS.iter().map(|claim| NumberObservation::read(root, claim)).collect()
}

/// One claim, and what the seam decided about it.
#[derive(Clone, Debug)]
pub struct NumberReading {
    pub claim: &'static NumberClaim,
    /// The layer's answer. **Not the reader's.**
    pub establishment: crate::features::Establishment,
    /// The layer's reason, in the record's own words rather than this module's.
    pub why: Option<String>,
    /// The line the reader found, carried through so the report can quote it.
    pub line: Option<String>,
    pub readable: bool,
}

/// Check every declared number, **through the seam**.
///
/// **This is `observe` plus [`crate::decision::rule_for_number`] and nothing else** — C-84.2's
/// exact shape, and the reason the module has no verdict of its own to fall back on. **A second
/// implementation of the rule is a second answer, and two answers disagree exactly when it
/// matters.** The first draft of this module decided in its own reader; the rule lives in
/// `decision.rs` now.
///
/// **Every row is reported, including a known-stale one.** Staleness is a *finding* rather than
/// a build failure, because C-38 reserves appearance prose: refusing would make the build
/// permanently red over arithmetic the user has not ruled on, and a gate that is always red is a
/// gate nobody reads.
pub fn check_numbers(root: &Path) -> (Vec<NumberReading>, RefusalSet) {
    let mut readings = Vec::with_capacity(CLAIM_COUNT);
    let mut refusals = RefusalSet::new();
    for observation in observe(root) {
        let (establishment, why) = crate::decision::rule_for_number(&observation);
        // **Only a row the instrument could not answer refuses.** `Active` and `Inactive` are
        // both reported states; `Refused` is the one that says the check did not run.
        if establishment == crate::features::Establishment::Refused {
            refusals.refuse(RefusalKind::FabricatedFeature, why.clone().unwrap_or_default());
        }
        readings.push(NumberReading {
            claim: observation.claim,
            establishment,
            why,
            line: observation.line,
            readable: observation.readable,
        });
    }
    (readings, refusals)
}

/// The gate's report. **Every row, every time, stale included.**
///
/// **A count is printed and then the rows**, because `board-duplicate-filings 42` beside no
/// names is a defect a reader cannot act on — the same finding as `Board::describe`, applied
/// here.
pub fn describe(readings: &[NumberReading], refusals: &RefusalSet) -> String {
    let active = readings
        .iter()
        .filter(|r| r.establishment == crate::features::Establishment::Active)
        .count();
    let mut out = format!(
        "numbers-declared\t{}\nnumbers-agree\t{active}\nnumbers-refused\t{}\n",
        readings.len(),
        refusals.all().len()
    );
    out.push_str(
        "numbers\teach row is read out of the document that carries it and ruled on by \
         decision::rule_for_number. A number nobody declared is invisible here; that limit is \
         stated rather than implied\n",
    );
    for reading in readings {
        out.push_str(&format!(
            "number\t{}\t{}\t{}\n",
            reading.claim.name,
            reading.establishment.slug(),
            reading.why.as_deref().unwrap_or("agrees with the record")
        ));
    }
    out
}

/// The numbers of the workspace this build ran in.
pub fn of_workspace() -> String {
    match workspace_root(&std::env::current_dir().unwrap_or_default()) {
        Some(root) => {
            let (readings, refusals) = check_numbers(&root);
            describe(&readings, &refusals)
        }
        None => "numbers-declared\tnot-measured\nnumbers\tno workspace root was found above the \
                 current directory, so no document was read\n"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_root() -> std::path::PathBuf {
        workspace_root(&std::env::current_dir().expect("a current directory"))
            .expect("the workspace root")
    }

    #[test]
    fn the_table_is_total_and_handed_written() {
        // **The value is meaningless; the type is the assertion.** A row added to `CLAIMS`
        // without bumping `CLAIM_COUNT` does not compile — C-65.2's shape.
        assert_eq!(CLAIMS.len(), CLAIM_COUNT);
        assert_eq!(CLAIMS_ARE_TOTAL, [(); CLAIM_COUNT]);
    }

    #[test]
    fn every_row_names_a_document_the_record_carries() {
        // A row pointing at a file outside the authoritative set would be a number checked
        // against something the record does not govern, so the set is checked rather than
        // assumed.
        let documents = crate::record::DOCUMENTS;
        for claim in CLAIMS {
            assert!(
                documents.iter().any(|d| d.path == claim.document),
                "{} names {}, which is not one of the record's documents",
                claim.name,
                claim.document
            );
            assert!(!claim.why.is_empty(), "{} must say why it is a number", claim.name);
        }
    }

    #[test]
    fn a_row_always_declares_something_to_check() {
        // **Never a row with neither an expectation nor a correction.** `expected()` would
        // otherwise return a placeholder string, and a placeholder that reads like a value is
        // the same defect as `provenance-digest 0` for an unreadable file.
        for claim in CLAIMS {
            assert!(
                claim.expects.is_some() || claim.correction.is_some(),
                "{} declares no expected value and no correction, so it checks nothing",
                claim.name
            );
        }
    }

    #[test]
    fn the_two_known_stale_numbers_are_still_stale_and_the_gate_says_so() {
        // **The finding, pinned on the real record rather than a fixture.**
        //
        // This test fails the day the user corrects the prose — and that is correct: the
        // correction is theirs to make, and when they make it this row must be updated in the
        // same act rather than left claiming a document disagrees when it does not.
        let root = real_root();
        let ledger = crate::record::read_document(&root, &crate::record::DOCUMENTS[0])
            .expect("the ledger reads");
        let spec = crate::record::read_document(
            &root,
            crate::record::DOCUMENTS
                .iter()
                .find(|d| d.path == "docs/DESIGN-SPEC.md")
                .expect("the spec is a document"),
        )
        .expect("the spec reads");

        assert!(
            ledger.contains("dense storage is 16.7M samples/voxel"),
            "C-2 no longer carries the pre-C-28 figure, so the stale row must be updated"
        );
        assert!(
            spec.contains("16,777,216 samples per voxel"),
            "DESIGN-SPEC no longer carries the pre-C-28 figure, so the stale row must be updated"
        );

        // And the gate names both, in words, rather than printing a count of two.
        let (rows, refusals) = check_numbers(&root);
        let report = describe(&rows, &refusals);
        assert!(
            report.contains("number\tdense-samples-per-voxel-ledger\tinactive"),
            "{report}"
        );
        assert!(
            report.contains("number\tdense-samples-per-voxel-spec\tinactive"),
            "{report}"
        );
        assert!(
            report.contains("262,144"),
            "and C-163's corrected figure travels with the finding: {report}"
        );
        assert!(
            report.contains("4,096"),
            "and so does the per-subvoxel correction: {report}"
        );
    }

    #[test]
    fn the_reader_reports_and_the_layer_decides() {
        // **The split is the property, so it is the test.** A scratch document and a scratch row;
        // the reader returns only `{readable, line}` and every verdict comes from
        // `decision::rule_for_number`.
        let dir = std::env::temp_dir().join(format!("shploingus-numbers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).expect("creates");
        let document = dir.join("docs/TEST.md");
        std::fs::write(&document, b"the figure is 16,777,216 blocks\n").expect("writes");

        let row: &'static NumberClaim = Box::leak(Box::new(NumberClaim {
            name: "scratch",
            document: "docs/TEST.md",
            sentence: "the figure is 16,777,216",
            expects: Some("16,777,216"),
            correction: None,
            why: "a scratch row",
        }));

        // **The reader's whole report: readable, and the line.** No `Agrees`, no `Stale` — the
        // first draft of this module returned exactly those from here, and that was the defect.
        let observation = NumberObservation::read(&dir, row);
        assert!(observation.readable);
        assert_eq!(
            observation.line.as_deref(),
            Some("the figure is 16,777,216 blocks"),
            "and the line is carried verbatim, because parsing the figure here would be the \
             reader deciding with its own parser"
        );
        let (establishment, why) = crate::decision::rule_for_number(&observation);
        assert_eq!(establishment, crate::features::Establishment::Active, "{why:?}");
        assert_eq!(why, None, "an agreeing row carries no reason");

        // **The prose changes and the constant does not.** A table checked against itself would
        // be unaffected; this one notices — through the layer, which is where the verdict lives.
        std::fs::write(&document, b"the figure is 1,073,741,824 blocks\n").expect("rewrites");
        let drifted = NumberObservation::read(&dir, row);
        let (establishment, why) = crate::decision::rule_for_number(&drifted);
        assert_eq!(
            establishment,
            crate::features::Establishment::Refused,
            "a figure that drifted from its declaration is refused, not reported as a pass"
        );
        assert!(
            why.unwrap_or_default().contains("16,777,216"),
            "and the reason quotes both numbers, so the reader is not left guessing which"
        );

        // **And the sentence goes.** A row naming prose nobody wrote is the row being wrong.
        std::fs::write(&document, b"nothing here any more\n").expect("rewrites");
        let gone = NumberObservation::read(&dir, row);
        let (establishment, why) = crate::decision::rule_for_number(&gone);
        assert_eq!(establishment, crate::features::Establishment::Refused);
        assert!(why.unwrap_or_default().contains("nothing checks"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_row_declared_stale_is_reported_inactive_and_does_not_fail_the_build() {
        // **C-38's practice as a type.** A document the user owns keeps its stale figure, the
        // gate names it every build, and the build stays green — because refusing here would
        // make this repository permanently red over prose only the user may correct.
        let dir = std::env::temp_dir().join(format!("shploingus-numbers-stale-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("docs")).expect("creates");
        std::fs::write(dir.join("docs/TEST.md"), b"dense storage is 16.7M samples/voxel\n")
            .expect("writes");
        let row: &'static NumberClaim = Box::leak(Box::new(NumberClaim {
            name: "scratch-stale",
            document: "docs/TEST.md",
            sentence: "dense storage is 16.7M samples/voxel",
            expects: None,
            correction: Some("1,073,741,824"),
            why: "a scratch stale row",
        }));

        let observation = NumberObservation::read(&dir, row);
        let (establishment, why) = crate::decision::rule_for_number(&observation);
        assert_eq!(establishment, crate::features::Establishment::Inactive);
        assert!(why.as_deref().unwrap_or_default().contains("1,073,741,824"));

        let report = describe(
            &[NumberReading {
                claim: row,
                establishment,
                why,
                line: observation.line,
                readable: observation.readable,
            }],
            &RefusalSet::new(),
        );
        assert!(report.contains("number\tscratch-stale\tinactive"), "{report}");
        assert!(
            report.contains("numbers-refused\t0"),
            "and a stale row is not a refusal: {report}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_document_is_not_agreement() {
        // D107: absence rendered as absence. A missing file is `not-measured`, never a pass —
        // and the refusal says so rather than printing a zero.
        let dir =
            std::env::temp_dir().join(format!("shploingus-numbers-absent-{}", std::process::id()));
        let row: &'static NumberClaim = Box::leak(Box::new(NumberClaim {
            name: "absent",
            document: "docs/NOT-THERE.md",
            sentence: "anything",
            expects: Some("1"),
            correction: None,
            why: "a scratch row",
        }));
        let observation = NumberObservation::read(&dir, row);
        assert!(!observation.readable, "a missing file is not readable");
        assert!(observation.line.is_none());
        let (establishment, why) = crate::decision::rule_for_number(&observation);
        assert_eq!(
            establishment,
            crate::features::Establishment::Refused,
            "and nothing was measured, which is not agreement"
        );
        assert!(why.unwrap_or_default().contains("not measured"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
