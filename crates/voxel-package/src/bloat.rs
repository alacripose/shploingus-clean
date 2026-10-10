//! **How much of the record is prose.** The measurement C-66 cited, in Rust.
//!
//! # Why this is a module and not a script
//!
//! D97 ruled *"Python is out of the project entirely; the judge is Rust"*, and C-66 cites a
//! Python script's output as evidence — so the ruling was violated by the evidence, which
//! makes the violation load-bearing rather than cosmetic. The port's first honest question was
//! **where a measurement belongs**, and the answer was already on disk.
//!
//! Three places were considered and rejected **on the record's own words**, not on taste:
//!
//! * **Not a sensor.** `voxel_instrument::Sensor` is *"the one sensor concept"* (S-03) and it is
//!   `{site, material, measured, tick}` — *"a reading is four things and no more"*, with
//!   `Measured` closed to `Units | Mass`. Both are material readings of the world, and the
//!   crate's header says *"an item is a material"* so that there is exactly one answer to what
//!   a thing is made of. A byte count is not a world reading, and adding `Site::Record` to carry
//!   one would be D-07's vocabulary mismatch — a second meaning behind an existing name.
//! * **Not a bin.** A separate executable is a finding that *"can only be read in a terminal"*,
//!   which D98 names as **incomplete**. The measurement belongs in the pass that already runs.
//! * **It is a build-gate finding.** H-10.1's *"one clean execution pass"* carries *"identity,
//!   refusal collection, conformance and provenance"*, and D97's own rule is that a lint which
//!   skipped the identity would be *"a report about bytes nobody can cite."* So this is measured
//!   where the rest of the gate measures, and printed in the gate's own report.
//!
//! # The rule this module exists to enforce
//!
//! **D107's hazard, verbatim: *"absence rendered as absence."*** The reference run reported
//! *"the judge convicts nothing"* because it read the first fourteen lines of a 533-line report.
//! A measurement that cannot find its input must therefore **say it was not measured** and
//! carry the reason — never a count of zero, which is indistinguishable from a real zero.
//!
//! [`Bloat`] makes that a type: [`Bloat::measured`] is a field, so a caller that reads the
//! counts without reading the flag has made a mistake the compiler cannot catch and the type
//! makes loud.

use std::path::Path;

use voxel_model::{RefusalSet, identity::Digest};

use crate::compiler::{ChainRead, read_back};

/// The word a record writes when its linkage was never established.
///
/// **D97 and L-1 §4 in one string:** *"`previous_event_hash=not_evaluable` means never call
/// that state tamper-evident."* A chain whose records say this has **no** tamper evidence, and
/// a report that printed a link count without saying so would be claiming a guarantee the bytes
/// do not carry.
pub const NOT_EVALUABLE: &str = "not_evaluable";

/// What a chain file costs, split into the part that carries meaning and the part that carries
/// structure.
///
/// **The two numbers are the whole point and they are the reference's two numbers.** The
/// reference ledger's measurement split each JSON record into `details` and everything else; a
/// record whose prose payload is most of its bytes is a record whose cost is unbounded, and
/// that is the observation C-66 recorded and the v3 envelope acted on by **not carrying a
/// `details` map at all**. This tree's analogue is `Chain::Event::detail` — the record's own
/// prose, described in `chain.rs` as *"what happened, in prose … nothing is summarised away"*.
///
/// So: same property, this tree's format, and **the comparison is stated rather than implied**.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bloat {
    /// Whether anything was measured at all. **False means the counts below are not
    /// measurements**, and [`Self::why_unmeasured`] says why.
    pub measured: bool,
    /// Why there is no measurement. Present exactly when `measured` is false.
    pub why_unmeasured: Option<String>,
    /// Records read.
    pub records: usize,
    /// The file's size in bytes.
    pub total_bytes: usize,
    /// Bytes of prose payload — the analogue of the reference ledger's `details`.
    pub payload_bytes: usize,
    /// Everything else: magic, versions, framing, digests, and the closed fields.
    pub frame_bytes: usize,
    /// Records whose `previous` is a real hash rather than [`NOT_EVALUABLE`].
    ///
    /// **This is a count, never a claim.** A chain of `records` with `linked == records` is
    /// *linked*; whether that makes it tamper-evident is a separate question this module does
    /// not answer and [`Self::describe`] does not imply.
    pub linked: usize,
    /// The chain's digest as read, so a report can be tied to the exact bytes.
    pub digest: Option<Digest>,
}

impl Bloat {
    /// A measurement that did not happen, and why.
    ///
    /// **All counts are zero and `measured` is false**, which is the one place zeros are
    /// allowed — because the flag beside them says they are not readings. A caller that wants
    /// the counts without the flag is the defect D107 describes.
    pub fn unmeasured(why: impl Into<String>) -> Self {
        Bloat {
            measured: false,
            why_unmeasured: Some(why.into()),
            ..Bloat::default()
        }
    }

    /// Prose payload as a fraction of the file, in whole percent.
    ///
    /// `None` when unmeasured, so a caller cannot print a percentage of nothing.
    pub const fn payload_percent(&self) -> Option<usize> {
        if !self.measured || self.total_bytes == 0 {
            return None;
        }
        Some((self.payload_bytes * 100) / self.total_bytes)
    }

    /// The report the gate prints, **one line per fact and never a line that could be read as
    /// a zero**.
    ///
    /// The unmeasured branch is first and is the whole reason this returns a `String` rather
    /// than a number: an absent measurement has to be *printable*, and the only way to print
    /// one is to have decided in advance what it looks like.
    pub fn describe(&self) -> String {
        if !self.measured {
            return format!(
                "ledger-bloat\tnot-measured\nledger-bloat-why\t{}",
                self.why_unmeasured.as_deref().unwrap_or("no reason recorded")
            );
        }
        let mut out = String::new();
        out.push_str(&format!("ledger-bloat-records\t{}\n", self.records));
        out.push_str(&format!("ledger-bloat-total-bytes\t{}\n", self.total_bytes));
        out.push_str(&format!("ledger-bloat-payload-bytes\t{}\n", self.payload_bytes));
        out.push_str(&format!("ledger-bloat-frame-bytes\t{}\n", self.frame_bytes));
        if let Some(percent) = self.payload_percent() {
            out.push_str(&format!("ledger-bloat-payload-percent\t{percent}\n"));
        }
        out.push_str(&format!("ledger-bloat-linked\t{}\n", self.linked));
        // **The honesty line, and it is not optional.** A reader who sees `linked 76` and no
        // caveat will report a tamper-evident chain. That is exactly the claim
        // `not_evaluable` forbids, so the number is never printed alone.
        out.push_str(&format!(
            "ledger-bloat-linkage\t{linked} of {records} records carry a previous-event hash; \
             a hash is evidence about bytes and order, not permission\n",
            linked = self.linked,
            records = self.records
        ));
        if let Some(digest) = &self.digest {
            out.push_str(&format!("ledger-bloat-digest\t{}\n", digest.to_hex()));
        }
        out
    }
}

/// Measure one chain file.
///
/// **Reads it through the same [`read_back`] the rest of the system reads it through**, so the
/// payload counted here is the payload the chain actually holds and not a second parse that
/// could disagree with the first. A file that will not read is [`Bloat::unmeasured`], never a
/// refusal that stops the build: a measurement that cannot run must not be the thing that
/// takes the gate down, and the reason is printed instead.
pub fn measure_chain_file(path: &Path) -> Bloat {
    if !path.is_file() {
        return Bloat::unmeasured(format!(
            "{} is not a file, so there was nothing to measure",
            path.display()
        ));
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(why) => {
            return Bloat::unmeasured(format!("{} could not be read: {why}", path.display()));
        }
    };
    match read_back(path) {
        Ok(read) => measure(&bytes, &read),
        Err(refusals) => Bloat::unmeasured(format!(
            "{} did not read back: {}",
            path.display(),
            RefusalSet::to_string(&refusals)
        )),
    }
}

/// Read the chain in a directory, or say why it could not be read.
///
/// **An absent directory is an EMPTY chain, and this is the one place the two are the same
/// thing.** There is no `chains/` directory because no chain has ever been written, and
/// *"the chain holds no events"* is a **complete and true** measurement of the chain. Reporting
/// it as `not-measured` would hide the fact the reader needs: that a 35-entry record has nothing
/// behind it.
///
/// So `Err` is for the two cases where the chain **cannot be known** — a file that will not read,
/// and two files that cannot be merged — and `Ok(Chain::new())` is for a chain that is empty.
/// This is the opposite of [`measure_chain_file`], which reports `not-measured` for the same
/// absent input, and the difference is not an inconsistency:
///
/// * **cost** is a property *of the file*, so no file means no cost to report;
/// * **disagreement** is a property *of two things*, so one of them being empty is a fact, not
///   a gap in the measurement.
pub fn read_chain(dir: &Path) -> Result<voxel_model::chain::Chain, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // **Absent and unreadable are both "no chain here", and neither is a refusal.** A
        // directory that is not there and a directory that is empty both mean the same thing
        // about the record.
        Err(_) => return Ok(voxel_model::chain::Chain::new()),
    };
    let mut files: Vec<std::path::PathBuf> =
        entries.flatten().map(|entry| entry.path()).collect();
    files.sort();
    let mut chain: Option<voxel_model::chain::Chain> = None;
    for path in files {
        if path.extension().and_then(|ext| ext.to_str()) != Some("chain") {
            continue;
        }
        let read = read_back(&path)
            .map_err(|found| format!("{} did not read back: {found}", path.display()))?;
        match &mut chain {
            // **The first chain file wins, and the second is a refusal rather than a merge.**
            // Appending two chain files into one chain would invent a linkage that neither
            // file claims, which is the one thing a hash chain exists to prevent.
            None => chain = Some(read.chain),
            Some(_) => {
                return Err(format!(
                    "{} holds more than one .chain file; two files are two chains and merging \
                     them would invent a linkage neither claims",
                    dir.display()
                ));
            }
        }
    }
    Ok(chain.unwrap_or_default())
}

/// The measurement, over bytes already read and the read that parsed them.
///
/// **Two inputs rather than one, and that is the point.** `bytes` is what the file costs;
/// `read` is what the file says. Measuring from only one of them is how a report about bytes
/// and a report about records come to disagree, and the disagreement is invisible because
/// both numbers look fine.
pub fn measure(bytes: &[u8], read: &ChainRead) -> Bloat {
    // The prose, summed from the chain itself rather than re-extracted from the bytes: a
    // second extractor is a second answer to "what is in this record".
    let payload_bytes: usize = read
        .chain
        .fold(0usize, |acc, event| acc + event.detail.len());
    let linked = read
        .projections
        .iter()
        .filter(|projection| {
            match projection.get("previous") {
                Some(voxel_model::wire::Value::Text(text)) => text.trim() != NOT_EVALUABLE,
                // A missing or non-text `previous` is **not** a real hash. Counting it as one
                // would be the same overclaim in a harder-to-see place.
                _ => false,
            }
        })
        .count();
    let total_bytes = bytes.len();
    Bloat {
        measured: true,
        why_unmeasured: None,
        records: read.folded,
        total_bytes,
        payload_bytes,
        frame_bytes: total_bytes.saturating_sub(payload_bytes),
        linked,
        digest: Some(Digest::of(bytes)),
    }
}

/// Every chain file in a directory, measured, in name order.
///
/// **A `Vec<Bloat>` and not a `Bloat`**, because "the whole set" and "one of them" are
/// different questions and a total across an unknown number of files is a number nobody can
/// check. An empty directory yields an empty vector, and the caller decides what an absent set
/// means — which is the decision D107 says must not be made silently.
pub fn measure_dir(dir: &Path) -> Vec<(String, Bloat)> {
    let mut found: Vec<(String, Bloat)> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return found,
    };
    let mut names: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    for path in names {
        if path.extension().and_then(|ext| ext.to_str()) != Some("chain") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unprintable>")
            .to_string();
        let measurement = measure_chain_file(&path);
        found.push((name, measurement));
    }
    found
}

// ---------------------------------------------------------------------------
// The doc stage: prose that lives in a code artifact, and records that live
// beside the chain
// ---------------------------------------------------------------------------

/// Where a crate's rationale is allowed to live, as a measurement.
///
/// **The same two-number shape as [`Bloat`]** — what carries meaning against what carries the
/// frame — applied to source instead of to a record: a crate whose comment bytes outweigh its
/// code is documentation wearing a `.rs` extension. `measured`/`why_unmeasured` are here for
/// [`Bloat`]'s reason (D107: absence renders as absence, never as a zero).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prose {
    /// False means the counts below are not measurements. [`Self::why_unmeasured`] says why.
    pub measured: bool,
    /// Why there is no measurement. Present exactly when `measured` is false.
    pub why_unmeasured: Option<String>,
    /// Source files read.
    pub files: usize,
    /// Bytes of source, newlines included.
    pub total_bytes: usize,
    /// Bytes on lines whose first non-space characters are `//`.
    pub comment_bytes: usize,
    /// Everything else.
    pub code_bytes: usize,
}

impl Prose {
    /// A measurement that did not happen, and why.
    pub fn unmeasured(why: impl Into<String>) -> Self {
        Prose {
            measured: false,
            why_unmeasured: Some(why.into()),
            ..Prose::default()
        }
    }

    /// Comment bytes as whole percent of the source. `None` when unmeasured or empty, so a
    /// caller cannot print a share of nothing.
    pub const fn comment_percent(&self) -> Option<usize> {
        if !self.measured || self.total_bytes == 0 {
            return None;
        }
        Some((self.comment_bytes * 100) / self.total_bytes)
    }

    /// The gate's line for this measurement. The unmeasured branch prints no counts at all —
    /// a zero reads as a real zero, which is the defect this type exists to prevent.
    pub fn describe(&self) -> String {
        if !self.measured {
            return format!(
                "code-prose\tnot-measured\ncode-prose-why\t{}",
                self.why_unmeasured.as_deref().unwrap_or("no reason recorded")
            );
        }
        let mut out = String::new();
        out.push_str(&format!("code-prose-files\t{}\n", self.files));
        out.push_str(&format!("code-prose-total-bytes\t{}\n", self.total_bytes));
        out.push_str(&format!("code-prose-comment-bytes\t{}\n", self.comment_bytes));
        if let Some(percent) = self.comment_percent() {
            out.push_str(&format!("code-prose-comment-percent\t{percent}\n"));
        }
        out
    }
}

/// Whether a line is comment prose: its first non-space characters are `//`.
///
/// **One predicate, two readers.** [`measure_prose`] counts these bytes and [`comment_lines`]
/// hands the text to the model instrument's judge, so *what a comment is* has one definition —
/// a second definition would let the doc stage's measurement and its reading disagree about
/// the same file.
fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// **The text of every comment line, markers stripped — the prose the doc stage reads.**
///
/// The markers go because a line still opening `//` is shape, and the judge reads terms:
/// `//! The pinned layer.` and `The pinned layer.` are one declaration written for two
/// readers, and the instrument is owed the prose rather than the punctuation. Separator
/// lines (`////`) carry no sentences and are not handed over.
///
/// This is the second reader of [`is_comment`], and it exists so the model instrument reads
/// the same lines [`measure_prose`] counts — *"no heading there begins with its name"* is a
/// fact about these lines, and a reading of them should not re-decide which ones are comments.
pub fn comment_lines(files: &[(std::path::PathBuf, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (_, text) in files {
        for line in text.lines() {
            if !is_comment(line) {
                continue;
            }
            let prose = line
                .trim_start()
                .strip_prefix("//")
                .unwrap_or(line.trim_start())
                .trim_start_matches(['!', '/', '*'])
                .trim();
            if !prose.is_empty() {
                out.push(prose.to_string());
            }
        }
    }
    out
}

/// Measure the prose share of source files that have already been read.
///
/// "Already read" is load-bearing: a crate's sources are read once by the census, and a second
/// read here would be a second answer to what the crate contains.
pub fn measure_prose(files: &[(std::path::PathBuf, String)]) -> Prose {
    if files.is_empty() {
        return Prose::unmeasured(
            "no source files were read, so no prose share was measured",
        );
    }
    let mut total_bytes = 0usize;
    let mut comment_bytes = 0usize;
    for (_, text) in files {
        for line in text.lines() {
            // +1 for the newline the line consumed; counted in both totals so the share is a
            // share of the file rather than of some normalised form of it.
            let bytes = line.len() + 1;
            total_bytes += bytes;
            if is_comment(line) {
                comment_bytes += bytes;
            }
        }
    }
    Prose {
        measured: true,
        why_unmeasured: None,
        files: files.len(),
        total_bytes,
        comment_bytes,
        code_bytes: total_bytes.saturating_sub(comment_bytes),
    }
}

/// Extensions that are a **second record** rather than a declared store.
///
/// `forbidden.second-record` (`architecture/workspace.dsl`): the chain is the one append-only
/// record, and a scratch ledger beside it drifts. `.jsonl` is the form that keeps being reached
/// for — the record holds it as an infraction that jsonl *"was never decided upon for anything
/// in the project"* — so it is enumerated here as a refusal rather than a style preference.
/// The declared shape for a queue record is `laya::queue_dir`'s `musli` records, which are
/// decided.
pub const INVENTED_RECORD_EXTENSIONS: [&str; 2] = ["jsonl", "ndjson"];

/// One file that is a second record beside the chain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InventedStore {
    /// Path relative to the workspace root, forward slashes.
    pub path: String,
    pub bytes: u64,
}

/// Every invented-record file under `root`, in path order.
///
/// **An empty result is a measurement here, not an absence** — the walk ran and found none —
/// which is why this returns a `Vec` and not a `Bloat`-style flag. The caller prints the count
/// and the list, so a future agent that adds one is caught by the same gate that found none.
/// Directories are walked without reading any file's contents; only the extension and the size
/// are read.
pub fn invented_stores(root: &Path) -> Vec<InventedStore> {
    let mut found: Vec<InventedStore> = Vec::new();
    collect_invented(root, root, &mut found);
    found.sort();
    found
}

fn collect_invented(root: &Path, dir: &Path, into: &mut Vec<InventedStore>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    for path in names {
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            // Recorded, never followed: following a directory symlink is an unbounded walk.
            continue;
        }
        if metadata.is_dir() {
            collect_invented(root, &path, into);
            continue;
        }
        let invented = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| INVENTED_RECORD_EXTENSIONS.contains(&extension))
            .unwrap_or(false);
        if invented {
            into.push(InventedStore {
                path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                bytes: metadata.len(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unmeasured_report_prints_no_counts_at_all() {
        // **D107's hazard, pinned.** A report of zeroes reads as a measurement of nothing; a
        // report that says `not-measured` and says why reads as a report that did not run.
        // The distinction is the whole reason `measured` is a field.
        let report = Bloat::unmeasured("there is no chain file here").describe();
        assert!(
            report.contains("ledger-bloat\tnot-measured"),
            "an unmeasured run must announce itself: {report}"
        );
        assert!(
            !report.contains("ledger-bloat-records"),
            "and must print no record count, because a zero would read as a real zero: {report}"
        );
        assert!(
            report.contains("there is no chain file here"),
            "and must carry the reason, or a reader cannot fix the input: {report}"
        );
    }

    #[test]
    fn a_measured_report_states_the_linkage_caveat_beside_the_number() {
        // **The overclaim this refuses.** A reader who sees `linked 3` and no caveat reports a
        // tamper-evident chain. The caveat is not decoration; it is what makes the number
        // quotable, and it is printed unconditionally so it cannot be forgotten.
        let measured = Bloat {
            measured: true,
            records: 3,
            linked: 3,
            total_bytes: 100,
            payload_bytes: 40,
            frame_bytes: 60,
            ..Bloat::default()
        };
        let report = measured.describe();
        assert!(report.contains("ledger-bloat-linked\t3"), "{report}");
        assert!(
            report.contains("not permission"),
            "a link count printed without its caveat is the claim `not_evaluable` forbids: {report}"
        );
        assert_eq!(measured.payload_percent(), Some(40), "and the share is computed");
    }

    #[test]
    fn a_zero_byte_file_is_a_measurement_and_not_a_missing_one() {
        // **The two are different and collapsing them is the defect.** An empty file measured
        // to zero is a fact; a file that was never found is not a fact. Only the second is
        // `not-measured`, which is why `payload_percent` returns `None` on a zero total rather
        // than dividing by zero or claiming 0 %.
        let empty = Bloat {
            measured: true,
            total_bytes: 0,
            ..Bloat::default()
        };
        assert_eq!(empty.payload_percent(), None, "no percentage of nothing");
        assert!(
            empty.describe().contains("ledger-bloat-records\t0"),
            "a measured zero is still a measurement: {}",
            empty.describe()
        );
    }

    #[test]
    fn measuring_a_file_that_is_not_there_says_so_rather_than_reporting_zero() {
        let missing = measure_chain_file(Path::new("no-such-directory/no-such.chain"));
        assert!(!missing.measured, "nothing was measured");
        assert!(
            missing.why_unmeasured.is_some(),
            "and the reason is carried, so the gate can print something a reader can act on"
        );
        assert!(!missing.describe().contains("ledger-bloat-total-bytes"));
    }

    // ---- the doc stage ----------------------------------------------------

    #[test]
    fn prose_is_counted_by_line_and_the_share_is_a_percentage_of_the_whole() {
        let files = vec![(
            std::path::PathBuf::from("crates/x/src/lib.rs"),
            "// a comment\npub fn f() {}\n// another\n".to_string(),
        )];
        let prose = measure_prose(&files);
        assert!(prose.measured);
        assert_eq!(prose.files, 1);
        assert_eq!(prose.comment_bytes, 13 + 11, "both comment lines, newline included");
        assert_eq!(prose.comment_bytes + prose.code_bytes, prose.total_bytes);
        // 13 + 14 + 11 bytes: two comment lines and one code line, each with its newline.
        assert_eq!(prose.total_bytes, 38);
        assert_eq!(prose.comment_percent(), Some((24 * 100) / 38));
    }

    #[test]
    fn a_crate_with_no_sources_has_no_prose_share_rather_than_a_share_of_zero() {
        let prose = measure_prose(&[]);
        assert!(!prose.measured, "nothing was read");
        assert_eq!(prose.comment_percent(), None, "no percentage of nothing");
        let report = prose.describe();
        assert!(report.contains("code-prose\tnot-measured"), "{report}");
        assert!(
            !report.contains("code-prose-comment-percent"),
            "an unmeasured share must not print as 0 %: {report}"
        );
    }

    #[test]
    fn an_invented_record_is_found_by_extension_and_an_empty_result_is_a_measurement() {
        // **The catch is prospective, and that is why it is tested by making one.** This tree
        // holds no invented record today; a check that has never fired is a check nobody has
        // seen work, so the test constructs the file it refuses.
        let mut workspace = std::env::temp_dir();
        workspace.push(format!("shploingus-bloat-doc-stage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(workspace.join("crates/deep/src")).expect("the scratch tree");

        assert!(
            invented_stores(&workspace).is_empty(),
            "an empty tree has no invented record, and that is a measurement"
        );

        std::fs::write(workspace.join("crates/deep/src/notes.jsonl"), b"{\n}\n")
            .expect("the invented record");
        let found = invented_stores(&workspace);
        assert_eq!(found.len(), 1, "the walk reaches into nested directories: {found:?}");
        assert_eq!(found[0].path, "crates/deep/src/notes.jsonl", "and reports a project-relative path");
        assert_eq!(found[0].bytes, 4);

        let _ = std::fs::remove_dir_all(&workspace);
    }
}
