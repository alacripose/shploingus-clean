//! **Filing a record that outlives the process that wrote it.**
//!
//! # What was missing, measured
//!
//! **Every chain write in this repository went to a temporary directory.** `first_ticket.rs`
//! filed a ticket, read it back, judged the document it produced, and wrote the chain file
//! under [`std::env::temp_dir`] — so it proved the round trip and then **threw the record
//! away**. `round_trip.rs` did the same with six tests. The consequence was not a slow test
//! suite; it was that **the project's own record had never been filed**, which is the single
//! fact blocking every view the project promises:
//!
//! * the chain was **empty**, so `board` had no tickets and no window could declare anything;
//! * [`crate::projection`] compared a 3,974-line record against **zero** events and reported
//!   **36 disagreements** on every build, which is H-8.1's central claim — *"a disagreement
//!   between them is impossible by construction"* — **measurably false**;
//! * the API document had nothing to describe, and the shell had no windows to mount.
//!
//! So this module is the missing link and nothing else. It is deliberately the **smallest**
//! thing that makes the chain canonical: an append that goes to disk, refuses to overwrite,
//! and is read back through the same [`crate::compiler::read_back`] the gate uses.
//!
//! # Why not call it `record.rs`, and why it is not in `chain.rs`
//!
//! `chain.rs` is in `voxel-model`, whose header declares **one dependency and no I/O**. A
//! `std::fs` call there would break that, and the dependency was chosen, not inherited.
//! `crate::record` already exists in `voxel-package` and is the **receipt/grant** side of
//! filing — the thing a filing *returns*, not the act of appending bytes. And D-07 governs
//! the name: `record` already means *"a grant-bearing fact about a world action"* in this
//! tree, so a second thing called `record` that writes files would be two names, one meaning.
//! **Filing** is the word the record already uses for the act — the client's own comment says
//! *"File the first ticket the way the in-game client files one."*
//!
//! # The two rules, both borrowed
//!
//! * **L-1 §3 (ala-cities):** *"The writer **refuses to overwrite an existing record**
//!   (`AlreadyExists`)."* [`file`] returns that refusal rather than truncating, so pointing
//!   [`file`] at `docs/LEDGER.md` destroys nothing.
//! * **H-8.1:** the chain is canonical and the Markdown is output. So [`file`] writes *chain
//!   bytes* and nothing else — it has no code path that writes `docs/LEDGER.md`, because a
//!   projection that overwrote the record would be a delete with extra steps (D6's CLM:
//!   content lifecycle is *"version / promote / supersede / retire, never delete"*).

use std::path::Path;

use voxel_model::chain::Chain;
use voxel_model::{RefusalKind, RefusalSet};

/// Where the project's own chain lives, relative to the workspace root.
///
/// **A directory, not a file, and that is the reference's shape rather than this module's
/// preference.** `bloat::read_chain` reads a *directory* and **refuses to merge two chain
/// files** — *"two files are two chains and merging them would invent a linkage that neither
/// file claims."* So the layout this module writes has to be one the reader already accepts,
/// which means one `.chain` file in `chains/` and a refusal if a second appears.
pub const CHAIN_DIR: &str = "chains";

/// The one chain file the workspace holds.
///
/// **Named for what it is, not for when it is.** A rolling name (`chain-1.chain`,
/// `chain-2.chain`) would give [`bloat::read_chain`] two files and a refusal on the second
/// build, so the name is deliberately stable: appending to the record is [`append`]'s job and
/// segmenting it is not this module's invention.
pub const CHAIN_FILE: &str = "workspace.chain";

/// Write chain bytes to a path, **refusing rather than overwriting**.
///
/// L-1 §3's rule, enforced at the only place bytes meet a file. `File::create` truncates, and
/// a truncating write to a record is the one irreversible act in this project — which is why
/// the existence check is here rather than left to a caller.
///
/// **The check is `symlink_metadata`, not `exists`.** `Path::exists` follows symlinks and
/// answers `false` for a **broken** one, so a dangling symlink at this path would read as
/// absent and then be followed by the create — a record written through a link the caller
/// never named. L-1's rule is about the *path*, so the path is what is asked about.
pub fn file(path: &Path, bytes: &[u8]) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            refusals.refuse(
                RefusalKind::RecordExists,
                format!(
                    "{} already exists; the writer refuses to overwrite a record, because a \
                     truncated file is a delete with no tombstone",
                    path.display()
                ),
            );
            return Err(refusals);
        }
        Err(found) if found.kind() == std::io::ErrorKind::NotFound => {}
        // **Anything other than "not there" is a refusal, not a permission to write.** A
        // permission error or a name too long is a fact about the path this module does not
        // understand, and writing anyway would be guessing.
        Err(found) => {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!("{} could not be inspected ({found}), so it was not written", path.display()),
            );
            return Err(refusals);
        }
    }

    if let Some(parent) = path.parent() {
        // `create_dir_all` is idempotent, so this is not a second "already exists" question.
        if let Err(found) = std::fs::create_dir_all(parent) {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!("{} could not be created ({found})", parent.display()),
            );
            return Err(refusals);
        }
    }

    match std::fs::write(path, bytes) {
        Ok(()) => Ok(()),
        Err(found) => {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!("{} could not be written ({found})", path.display()),
            );
            Err(refusals)
        }
    }
}

/// **Grow the record: append records to a chain that already exists, or refuse.**
///
/// # Why this is not [`append`]
///
/// [`append`] refuses any existing file, and it was written to be safe rather than complete:
/// L-1 §3's rule — *"The writer refuses to overwrite an existing record"* — is enforced by
/// [`file`], which refuses the **path**. That is correct for *creating* a record and wrong
/// for *extending* one, because for an append-only log the two are the same act. The cost
/// was measured: with [`append`] as the only writer, the 42 work tickets could not be filed
/// at all — the chain held 56 events and **zero** tickets, and every escape hatch was
/// worse. A second `.chain` file makes [`crate::bloat::read_chain`] refuse the whole
/// directory; re-filing from scratch would truncate a record that now exists; and the
/// segmenting rule is unruled (C-73.5).
///
/// # What it still refuses
///
/// **Everything [`file`] refuses, and one more.** Nothing is truncated and nothing is
/// replaced — the file is opened for **append**, never for write, so there is no code path
/// here that can shorten the file. And a chain whose existing bytes **do not read back** is
/// refused rather than extended, because appending to a record whose earlier half cannot be
/// verified extends a chain whose integrity is already broken.
///
/// **A torn write is recoverable, and that is why appending is allowed at all.** The framing
/// is length-prefixed, so a process that dies mid-append leaves a partial final record;
/// [`crate::compiler::fold_bytes`] counts it as [`RefusalKind::CorruptRecord`] and keeps
/// folding. **The file loses that record, not the file** — which is the same property
/// [`Chain`] holds and the reason append-only storage is safe to write to in place.
pub fn grow<'a>(
    workspace: &Path,
    prior: &[u8],
    records: impl IntoIterator<Item = crate::compiler::UnlinkedRecord<'a>>,
) -> Result<std::path::PathBuf, RefusalSet> {
    let path = chain_path(workspace);
    if !is_filed(workspace) {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!(
                "{} does not exist, so there is nothing to grow. `append` files the first record \
                 and `grow` extends one that is already there — conflating them is what left \
                 this repository with 56 events and no tickets",
                path.display()
            ),
        );
        return Err(refusals);
    }

    // **The bytes on disk, not the bytes handed in.** A caller can pass whatever it likes as
    // `prior`, and trusting it would let a record claim a predecessor that is not the record
    // before it. So the existing chain is read from the file, and `prior` is only used when
    // the file is absent — which it is not, because that case refused above.
    let on_disk = std::fs::read(&path).map_err(|found| {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} exists but could not be read ({found}), so it was not extended", path.display()),
        );
        refusals
    })?;
    if !prior.is_empty() && prior != on_disk {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the prior bytes handed to `grow` are not the bytes on disk at {} ({} vs {}), so \
                 the records appended after them would not follow the record that is actually \
                 there. Refused rather than reconciled: choosing one of two histories is a \
                 ruling, not a merge",
                path.display(),
                prior.len(),
                on_disk.len()
            ),
        );
        return Err(refusals);
    }

    // **Read back before extending, not after.** A chain that will not fold is a chain whose
    // linkage cannot be trusted, and appending to it makes the damage longer.
    if let Err(found) = crate::compiler::fold_bytes(&on_disk) {
        let mut refusals = RefusalSet::new();
        refusals.absorb(found);
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the existing chain at {} does not fold, so it was not extended. Appending to a \
                 record whose earlier half cannot be verified extends a chain whose integrity is \
                 already broken",
                path.display()
            ),
        );
        return Err(refusals);
    }

    let new_bytes = linked_records(workspace, &on_disk, records)?;

    // **`append(true)`, and the truncate flag is never set anywhere in this module.** That is
    // the whole safety argument: there is no code path through `grow` that can shorten the
    // file, so "refuses to overwrite" still holds — it is now enforced by construction rather
    // than by a check that has to run first.
    let outcome = std::fs::OpenOptions::new().append(true).open(&path).and_then(|mut file| {
        use std::io::Write;
        file.write_all(&new_bytes)
    });
    if let Err(found) = outcome {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} could not be appended to ({found})", path.display()),
        );
        return Err(refusals);
    }

    // **And read back after, so a short write is a refusal rather than a silent record.**
    let after = std::fs::read(&path).map_err(|found| {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} was appended to but cannot be read back ({found})", path.display()),
        );
        refusals
    })?;
    if !after.starts_with(&on_disk) {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::ChainBroken,
            "the chain no longer begins with the bytes it began with after an append, which \
             means the append overwrote something. Refused rather than accepted"
                .to_string(),
        );
        return Err(refusals);
    }
    Ok(path)
}

/// The path this module files to, under a workspace root.
pub fn chain_path(workspace: &Path) -> std::path::PathBuf {
    workspace.join(CHAIN_DIR).join(CHAIN_FILE)
}

/// Where compiled packages live, relative to the workspace root.
///
/// **The loader's search path is one directory and it is this one** — the dirty branch's
/// `PackageLoader` note is the rule: *a loader that made its own search path would have chosen
/// where the game's packages live*. It is the repository's `packages/`, the directory the
/// `.shspec` sources already live in, so a compiled package lands beside the source it came from
/// rather than in a second place nobody looks.
pub const PACKAGE_DIR: &str = "packages";

/// Where one compiled package belongs: `<workspace>/packages/<name>.shpkg`.
///
/// **The extension is declared, not inferred from the bytes.** A reader that had to sniff the
/// frame to learn what a file is would be a reader that accepts a renamed chain.
pub const PACKAGE_EXTENSION: &str = "shpkg";

/// The path a package named `name` occupies in the workspace.
///
/// **`name` is the caller's and it must be the record's own name** — a ticket id, never a
/// prettified or timestamped one. Two packages of one name would be two answers to one question,
/// and [`file`] refuses the second by refusing the path.
pub fn package_path(workspace: &Path, name: &str) -> std::path::PathBuf {
    workspace.join(PACKAGE_DIR).join(format!("{name}.{PACKAGE_EXTENSION}"))
}

/// Whether a chain has already been filed at `workspace`.
///
/// **A convenience, and it is the honest one: it answers "is there a file", not "is there a
/// record".** A `.chain` file that will not *read back* is a file, and the gate's
/// `bloat::read_chain` is what turns that into a refusal with the reason. Naming this
/// `has_chain` would let a caller read it as "the record is there", which is the question
/// D107's *"absence rendered as absence"* is about.
pub fn is_filed(workspace: &Path) -> bool {
    chain_path(workspace).exists()
}

/// Append rendered records to the workspace chain, **or refuse**.
///
/// **The one legitimate refusal is "already filed", and it is the whole design.** This tree's
/// record is append-only and one file per workspace ([`CHAIN_FILE`]'s note), so the second
/// append has nowhere legal to go until a segmenting rule is *ruled* rather than invented.
/// Refusing is the honest answer; creating `workspace-2.chain` would make
/// `bloat::read_chain` refuse the whole directory on the next build, which trades a loud
/// error here for a louder one later.
///
/// So the *first* filing is a real one, and everything after it is a question the record has
/// not answered yet. That asymmetry is the point, and it is stated rather than papered over
/// with a rollover this module has no authority to define.
pub fn append(workspace: &Path, bytes: &[u8]) -> Result<std::path::PathBuf, RefusalSet> {
    let path = chain_path(workspace);
    file(&path, bytes)?;
    Ok(path)
}

/// Render records into bytes **with their linkage computed here**, not by the caller.
///
/// **This is the chokepoint H-12.2 names** — *"the shell owns a queue … the notification panel
/// is fed by the owner of the queue, so it cannot miss an event that some call site forgot to
/// log."* Applied to the record rather than the panel: **one writer computes the linkage**, so
/// a record cannot be filed claiming a predecessor it does not have.
///
/// # Why `prior` is a parameter, and it is not a convenience
///
/// **The head has to come from the bytes that will actually precede these records, and those
/// may not be on disk yet.** The first version read the head from the workspace, so calling
/// this twice before filing anything produced **two records both claiming the genesis
/// digest** — and the second was refused as `ChainBroken` on read-back, correctly, because it
/// named a predecessor that was not the record before it.
///
/// **That is the refusal working**, and the fix is not to weaken it. `prior` is what is
/// already accumulated, so the second call links to the first call's record. The result is
/// `prior` followed by the new records, which is what makes building a multi-record chain in
/// memory possible at all — and in memory is the only way, since [`append`] refuses a second
/// filing by design.
pub fn linked_records<'a>(
    workspace: &Path,
    prior: &[u8],
    records: impl IntoIterator<Item = crate::compiler::UnlinkedRecord<'a>>,
) -> Result<Vec<u8>, RefusalSet> {
    let mut out: Vec<u8> = prior.to_vec();
    // **An empty `prior` means the chain starts here**, so the head is the genesis link. A
    // non-empty one means the head is the last record in it — which is why this reads the
    // bytes rather than the workspace.
    let mut previous = if out.is_empty() {
        linkage_of(workspace)
    } else {
        crate::compiler::digest_of_last(&out).ok_or_else(|| {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "the records already accumulated do not fold, so the next one has no predecessor \
                 to name. Writing past an unreadable record would extend a chain whose earlier \
                 half cannot be verified",
            );
            refusals
        })?
    };
    for fields in records {
        out.extend(crate::compiler::render_event(fields.linked(&previous))?);
        // **The next record's predecessor is this one's digest, so the writer folds what it
        // just wrote.** Folding through the same function the reader uses is the only way to
        // get a digest a reader will agree with — computing one here would be a second
        // implementation of the wire format, and two implementations of a digest disagree
        // exactly when it matters.
        previous = crate::compiler::digest_of_last(&out).ok_or_else(|| {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "the record just rendered could not be read back, so the next one has no \
                 predecessor to name",
            );
            refusals
        })?;
    }
    Ok(out)
}

/// The `previous` value a record must carry, given what is already filed.
///
/// **This is the linkage, and it is the difference between a record and a chain.**
/// `Chain::append` has always linked by digest — `previous: self.head`,
/// `Linkage::Linked(digest)` — but the *writer* takes `previous` as a `&'a str` from its
/// caller, so a caller who writes `not_evaluable` on every record gets a file of records that
/// each deny knowing the one before. D97's rule is then the only thing standing between that
/// file and being called tamper-evident, and the honest report must be `AppendOnly`, not
/// `HashLinked`.
///
/// **So the first record says `not_evaluable` and every record after it says the digest of
/// the one before.** A caller cannot opt out: [`append`] computes this and the caller never
/// supplies it. That is the C-49 chokepoint in its smallest form — one writer, so the linkage
/// is a property of the file rather than of whichever call site remembered.
///
/// **The digest is the record's own, computed by [`crate::compiler::read_back`] as it folds.**
/// Computing it here would be a second implementation of the wire format, and two
/// implementations of a digest disagree exactly when it matters.
pub fn linkage_of(workspace: &Path) -> String {
    let chain = read(workspace);
    // **A `Chain::linked()` genesis, not `not_evaluable`.** `Chain::new()` starts with head
    // `NotEvaluable`, so its first record names no predecessor — and because
    // `is_tamper_evident` requires *every* event to be linked, **a chain whose first record
    // says `not_evaluable` can never be called tamper-evident, however many records follow
    // it.** That is not a bug in the check; it is the check working, and it is why
    // `Chain::linked()` exists: *"the difference from `Chain::new` is the difference between
    // append-only and tamper-evident, and it is offered explicitly so the choice is visible
    // at the call site rather than defaulted."*
    //
    // `Digest::of(&[])` is a real digest of no bytes — **not a sentinel** — so the first
    // record links to something computable and the whole chain is hash-linked.
    //
    // The first version of this function returned `not_evaluable` for an empty chain and the
    // test caught the consequence: *"2 of 3 events linked"*. **The two records after the first
    // were perfectly linked and the chain still reported itself as not tamper-evident**,
    // because the first could not be. The honest fix is the genesis link, not a rule change
    // to `is_tamper_evident` — loosening that check would have made a broken chain look sound.
    if chain.is_empty() {
        return voxel_model::chain::Linkage::Linked(voxel_model::identity::Digest::of(&[]))
            .render();
    }
    // **`Linkage::render`, not a format string.** The reader's `Linkage::parse` accepts a
    // `sha256:` identity and refuses anything else *rather than downgrading it to
    // `not_evaluable`* — so a writer that hand-assembled the text would be one prefix away
    // from producing a chain that reads as unbroken but is not. Asking the type for its own
    // text form makes that impossible.
    voxel_model::chain::Linkage::Linked(chain.digest()).render()
}

/// The word a chain writes when its linkage was never established.
///
/// **Re-exported from `voxel-model`, not a second copy of the string.** `chain.rs` holds the
/// literal and `RefusalKind::ChainNotEvaluable` exists to police it; a third spelling here
/// would be the same value in three places, and the wire format's rule is that a word either
/// is the declared one or is refused.
pub use voxel_model::chain::NOT_EVALUABLE;

/// The chain as it is filed, or an **empty** chain when nothing is.
///
/// **Empty and not-measured are different, and this returns empty on purpose.** The chain
/// genuinely has no events; that is a fact, not a gap in a measurement. `bloat::read_chain`
/// makes the same call for the same reason, and a caller that needs to know *why* it is empty
/// asks [`is_filed`] or reads the chain file's own refusals.
pub fn read(workspace: &Path) -> Chain {
    if !is_filed(workspace) {
        return Chain::new();
    }
    match crate::compiler::read_back(&chain_path(workspace)) {
        Ok(read) => read.chain,
        // **A chain that will not read is empty, not a panic.** The gate prints the refusal
        // with its reason; a reader that cannot read must not invent events to replace them.
        Err(_) => Chain::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory the test owns and removes, so a failed test cannot leave a chain behind
    /// for the *next* run to trip the refuse-to-overwrite rule on.
    fn scratch(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("shploingus-filing-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn a_first_filing_writes_and_a_second_refuses() {
        let workspace = scratch("first-then-second");
        assert!(!is_filed(&workspace), "nothing is filed before the first append");

        let path = append(&workspace, b"first record\n").expect("the first append is legal");
        assert!(is_filed(&workspace), "the first append filed a chain");
        assert!(path.exists());

        // **The refusal is the test.** L-1 §3: the writer refuses to overwrite an existing
        // record. A second append must leave the first one's bytes exactly as they were.
        let refused = append(&workspace, b"second record\n").expect_err("the second append refuses");
        assert_eq!(refused.all().len(), 1, "one refusal, collected rather than the first returned");
        assert_eq!(refused.all()[0].kind(), RefusalKind::RecordExists);
        let after = std::fs::read(&path).expect("still readable");
        assert_eq!(after, b"first record\n", "the refusal did not truncate the record");
    }

    #[test]
    fn a_dangling_symlink_is_not_absent() {
        // **The reason `symlink_metadata` is used, pinned.** `Path::exists` follows symlinks
        // and answers `false` for a broken one, so the old check would have reported this
        // path as absent and then written *through* the link to a target the caller never
        // named. On Windows an unprivileged `symlink_file` fails, so the test says so and
        // stops rather than reporting a pass it did not earn.
        let dir = scratch("dangling");
        std::fs::create_dir_all(&dir).expect("creates");
        let target = dir.join("elsewhere.chain");
        let link = dir.join("link.chain");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("creates a dangling symlink");
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&target, &link).is_err() {
            // **Not a skip dressed as a pass:** the branch that could not run is named.
            eprintln!("unprivileged symlink creation refused on this host; the unix case covers it");
            return;
        }

        assert!(!link.exists(), "the symlink is broken, so `exists` says false");
        let refused = file(&link, b"through the link\n").expect_err("a broken link is not absent");
        assert_eq!(refused.all()[0].kind(), RefusalKind::RecordExists);
        assert!(!target.exists(), "nothing was written through the link");
    }

    /// **The property that makes `grow` safe: the file only ever gets longer.**
    ///
    /// `append` was safe because it refused; `grow` is safe because **there is no code path
    /// that can truncate** — the file is opened `append(true)` and never for write. So the
    /// test is not "it appended" but "the earlier bytes are still there, byte for byte",
    /// which is the claim L-1 §3 makes and the one a short write would break.
    #[test]
    fn growing_a_chain_extends_it_and_never_shortens_what_was_already_filed() {
        let workspace = scratch("grow");
        let namespace =
            voxel_model::chain::Namespace::new("world-2468", 1, voxel_model::chain::Season::Winter, "grow")
                .expect("a namespace");

        let first = linked_records(
            &workspace,
            &[],
            [crate::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind: voxel_model::chain::EventKind::DevelopmentFact,
                question: "the first record",
                answer: None,
                kind: voxel_model::envelope::Kind::Evidence,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: "agent-migration",
                round_id: "grow",
                supersedes: None,
            }],
        )
        .expect("the first record links");
        let path = append(&workspace, &first).expect("filed");
        let after_first = std::fs::read(&path).expect("reads");

        for text in ["the second record", "the third record"] {
            // **Re-read before each grow, because the file has moved since the last one.**
            // The first version passed the same `after_first` bytes both times, and `grow`
            // refused the second with `ChainBroken` — 480 bytes on disk against 1,436 handed
            // in. **The refusal was right and the test was wrong**, which is the same shape as
            // the harness bug `filed_record.rs` records: a `RefusalSet` reporting `RecordExists`
            // or `ChainBroken` on a path nobody else uses is a fact about the call, and
            // reading it as "the writer is broken" sends the fix into the wrong place.
            let before_growth = std::fs::read(&path).expect("reads");
            grow(&workspace, &before_growth, [crate::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind: voxel_model::chain::EventKind::TicketTransition,
                question: text,
                answer: None,
                kind: voxel_model::envelope::Kind::Authority,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: "player-1",
                round_id: "grow",
                supersedes: None,
            }])
            .expect("grows");
            let grown = std::fs::read(&path).expect("reads");
            assert!(
                grown.len() > before_growth.len(),
                "the file grew: {} -> {}",
                before_growth.len(),
                grown.len()
            );
            assert!(
                grown.starts_with(&before_growth),
                "**and every byte that was already filed is still there, in order.** This is \
                 the whole safety argument for `grow`: nothing is truncated, so refusing to \
                 overwrite still holds"
            );
        }
        let _ = after_first;

        // **And the result is one chain, not three files.** A second `.chain` file would make
        // `bloat::read_chain` refuse the whole directory, which is why `append` refused in the
        // first place; growing is the answer that keeps one file.
        let dir_entries: Vec<_> = std::fs::read_dir(workspace.join(CHAIN_DIR))
            .expect("the chain directory reads")
            .filter_map(|entry| entry.ok())
            .collect();
        assert_eq!(dir_entries.len(), 1, "still exactly one chain file");
        assert_eq!(read(&workspace).len(), 3, "three records, all linked");
    }

    #[test]
    fn growing_a_chain_that_was_never_filed_is_refused_rather_than_creating_one() {
        // `grow` extends and `append` creates. A caller that reaches for `grow` on an unfiled
        // workspace has made a mistake, and the honest answer names it rather than quietly
        // doing the other thing.
        let workspace = scratch("grow-unfiled");
        let refused = grow(&workspace, &[], []).expect_err("there is nothing to grow");
        assert!(
            refused.all()[0].detail().contains("nothing to grow"),
            "and the refusal says which of the two writers was wanted: {}",
            refused.all()[0].detail()
        );
        assert!(!is_filed(&workspace), "and nothing was created");
    }

    #[test]
    fn reading_an_unfiled_workspace_is_empty_rather_than_a_failure() {
        let workspace = scratch("unfiled");
        assert!(!is_filed(&workspace));
        assert!(read(&workspace).is_empty(), "no file means no events, and that is a fact");
    }
}
