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
use voxel_model::identity::Digest;
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

    // **`linked_records` returns the whole chain — `prior` followed by the new records — and
    // that is its contract**, because rendering a chain in memory is what it is for. Appending
    // that result to a file that already holds `prior` writes `prior` a second time, and it does
    // so **on every grow**: the file doubles while the record gains one row.
    //
    // That is measured rather than suspected. `chains/workspace.chain` went from **842,609,979
    // to 1,685,220,970** bytes across a single filing, and `2 × 842,609,979 + 1,012 =
    // 1,685,220,970` — where the 1,012 bytes are the one record rendered. The arithmetic names
    // the defect exactly, and the doubled file is a chain whose second half repeats the first's
    // history with linkage that cannot be right.
    //
    // So the prior prefix is **sliced off after being checked for**. The check is what keeps
    // this a measurement instead of a hope: a `linked_records` that stopped beginning with its
    // prior would refuse here rather than append bytes that follow nothing.
    let linked = linked_records(workspace, &on_disk, records)?;
    if !linked.starts_with(&on_disk) {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the chain rendered for {} does not begin with the {} bytes already on disk, so \
                 what would be appended follows a predecessor that is not there. The renderer \
                 must contain the chain it extends for the slice below to be a record rather than \
                 a guess",
                path.display(),
                on_disk.len()
            ),
        );
        return Err(refusals);
    }
    let new_bytes = &linked[on_disk.len()..];

    // **`append(true)`, and the truncate flag is never set anywhere in this module.** That is
    // the whole safety argument: there is no code path through `grow` that can shorten the
    // file, so "refuses to overwrite" still holds — it is now enforced by construction rather
    // than by a check that has to run first.
    let outcome = std::fs::OpenOptions::new().append(true).open(&path).and_then(|mut file| {
        use std::io::Write;
        file.write_all(new_bytes)
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
    // **And the file grew by the records and nothing else.** A short write would leave the
    // prefix intact and lose the tail, which the check above cannot see; a doubled append
    // would leave the prefix intact and repeat the history, which it also cannot see. One
    // equality rules out both, and it is the assertion the doubling defect walked through.
    if after.len() != on_disk.len() + new_bytes.len() {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "{} grew by {} bytes where the records rendered are {} bytes, so the file does \
                 not hold what was written. Refused rather than accepted: a file that grew by \
                 the whole chain again is a record whose second half repeats the first",
                path.display(),
                after.len().saturating_sub(on_disk.len()),
                new_bytes.len()
            ),
        );
        return Err(refusals);
    }
    Ok(path)
}

/// What a repair did, in the numbers a manifest needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repair {
    pub before_bytes: usize,
    pub after_bytes: usize,
    /// Frames kept, and frames dropped because they did not link to the record before them.
    pub kept: usize,
    pub dropped: usize,
    pub before_digest: String,
    pub after_digest: String,
    pub folded_before: usize,
    pub corrupt_before: usize,
    pub folded_after: usize,
    pub corrupt_after: usize,
    /// Tickets the board read, before and after. **The repair must not move these.**
    pub tickets_before: usize,
    pub tickets_after: usize,
}

impl Repair {
    /// The manifest, as text. Every number here is read off the two folds rather than assumed.
    pub fn describe(&self) -> String {
        format!(
            "chain-repair\t{}
chain-repair-bytes\t{} -> {} ({} removed)\nchain-repair-frames\t{} kept, {} dropped\nchain-repair-before-digest\tsha256:{}\nchain-repair-after-digest\tsha256:{}\nchain-repair-before-folded\t{}\nchain-repair-before-corrupt\t{}\nchain-repair-after-folded\t{}\nchain-repair-after-corrupt\t{}\nchain-repair-tickets\t{} -> {}\n",
            "every frame that did not link to the record before it, and nothing else",
            self.before_bytes,
            self.after_bytes,
            self.before_bytes - self.after_bytes,
            self.kept,
            self.dropped,
            self.before_digest,
            self.after_digest,
            self.folded_before,
            self.corrupt_before,
            self.folded_after,
            self.corrupt_after,
            self.tickets_before,
            self.tickets_after,
        )
    }
}

/// **Repair a chain that holds records which do not link, or refuse — and keep the original.**
///
/// # Why this is a compaction and not a cut, and the evidence for that
///
/// The first version of this function found the outermost repeat — the chain's own beginning,
/// written again — and cut the file there. On the live record it found that boundary exactly:
/// `842,609,979`, and removing `842,614,525` bytes, 50.0% of the file. **Then it refused**, because
/// the chain that would have been written held **166,406** records that did not link: the file had
/// been doubled **more than once**, so cutting off the outermost copy left the inner ones behind.
///
/// That refusal is the whole argument for this shape. A cut assumes a file is `P || P || tail`; a
/// compaction assumes only that some frames linked, and is decided by [`crate::compiler::compact`],
/// which uses the reader's own acceptance rule. The repair keeps the frames that folded, byte for
/// byte, and drops the frames that did not.
///
/// # The order of the acts, because this is the one irreversible one
///
/// A chain is append-only and the writer refuses to overwrite, so a repair is the only operation in
/// this module that can make a file smaller. It is fenced so that no part of it can run on a guess:
///
/// 1. the file is read and **folded**, so what is being repaired is a record rather than a blob;
/// 2. **something must actually be dropped**, or this refuses — rewriting a sound record would be a
///    change with no finding behind it, and the manifest would have nothing to record;
/// 3. the compacted chain must fold with **zero** corrupt records;
/// 4. it must fold the **same number of records** as the file did, so nothing that linked was lost;
/// 5. **the tickets must be identical**, id for id and objective for objective;
/// 6. only then: the repaired bytes are written to `keep/workspace.chain.repaired` through [`file`],
///    **the original is renamed out of `chains/` to `keep/`** (a rename, not a copy — the evidence
///    costs no second write), and the repaired file is renamed into the canonical path.
///
/// Nothing is deleted. The unrepaired file and a manifest of both digests stay in `keep/`.
pub fn repair(workspace: &Path, keep: &Path) -> Result<Repair, RefusalSet> {
    let path = chain_path(workspace);
    let mut refusals = RefusalSet::new();

    let bytes = std::fs::read(&path).map_err(|found| {
        let mut set = RefusalSet::new();
        set.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} could not be read ({found}), so nothing was repaired", path.display()),
        );
        set
    })?;
    let before = crate::compiler::fold_bytes(&bytes).map_err(|found| {
        let mut set = RefusalSet::new();
        set.absorb(found);
        set
    })?;

    let compaction = crate::compiler::compact(&bytes).map_err(|found| {
        let mut set = RefusalSet::new();
        set.absorb(found);
        set
    })?;
    if compaction.dropped == 0 {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "every record in {} folds ({} frames), so there is nothing to repair. Refused \
                 rather than rewritten: rewriting a sound record is a change with no finding \
                 behind it",
                path.display(),
                compaction.kept
            ),
        );
        return Err(refusals);
    }

    let repaired = compaction.bytes;
    let after = crate::compiler::fold_bytes(&repaired).map_err(|found| {
        let mut set = RefusalSet::new();
        set.absorb(found);
        set
    })?;
    if after.corrupt != 0 {
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the repaired chain does not fold clean: {} record(s) refused. {} A repair that \
                 trades size for a broken record is not a repair",
                after.corrupt,
                after.corrupt_detail.join("; ")
            ),
        );
        return Err(refusals);
    }
    if after.folded != before.folded {
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the repair would lose records: {} folded before, {} after. Refused — the keep-set \
                 is the frames that linked, so the count may not fall",
                before.folded, after.folded
            ),
        );
        return Err(refusals);
    }

    // **The tickets, id for id.** The dropped frames fold to no records at all, so nothing should
    // move — and this is checked rather than assumed, because the count above would still hold if a
    // repair dropped one ticket-carrying repeat and kept a duplicate of another.
    let tickets_of = |read: &crate::compiler::ChainRead| -> Vec<(String, String)> {
        read.board
            .tickets()
            .iter()
            .map(|ticket| (ticket.id.as_str().to_string(), ticket.objective.clone()))
            .collect()
    };
    let tickets_before = tickets_of(&before);
    let tickets_after = tickets_of(&after);
    if tickets_before != tickets_after {
        refusals.refuse(
            RefusalKind::ChainBroken,
            format!(
                "the repair would move tickets: {} before, {} after. Refused — a repair removes \
                 records that do not link and nothing else",
                tickets_before.len(),
                tickets_after.len()
            ),
        );
        return Err(refusals);
    }

    // **The unrepaired bytes are dropped before the repaired chain is folded.** A repair reads
    // 1.68 GB and writes 0.84 GB, and keeping both images live made a run that needed five
    // gigabytes to remove one — measured on the live record, and the reason this is dropped here
    // rather than left to the end of the function.
    let before_bytes = bytes.len();
    let before_digest = voxel_model::Digest::of(&bytes).to_hex();
    let after_bytes = repaired.len();
    let after_digest = voxel_model::Digest::of(&repaired).to_hex();
    drop(bytes);

    if let Err(found) = std::fs::create_dir_all(keep) {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} could not be created ({found})", keep.display()),
        );
        return Err(refusals);
    }
    let staged = keep.join("workspace.chain.repaired");
    let kept = keep.join("workspace.chain.as-found");
    file(&staged, &repaired)?;
    // **A rename, so the original costs no second write and cannot be half-copied.** It leaves
    // `chains/` rather than being deleted, because it is the evidence of the defect.
    if let Err(found) = std::fs::rename(&path, &kept) {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!("{} could not be renamed to {} ({found})", path.display(), kept.display()),
        );
        return Err(refusals);
    }
    if let Err(found) = std::fs::rename(&staged, &path) {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!(
                "the repaired chain could not be moved into {} ({found}). The unrepaired original \
                 is at {}",
                path.display(),
                kept.display()
            ),
        );
        return Err(refusals);
    }

    Ok(Repair {
        before_bytes,
        after_bytes,
        kept: compaction.kept,
        dropped: compaction.dropped,
        before_digest,
        after_digest,
        folded_before: before.folded,
        corrupt_before: before.corrupt,
        folded_after: after.folded,
        corrupt_after: after.corrupt,
        tickets_before: tickets_before.len(),
        tickets_after: tickets_after.len(),
    })
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

/// The file name one package occupies, **derived from its content identity**.
///
/// **This is the dirty branch's rule, and the reason it is a rule** (C-58 item 7): *filenames
/// derived from the content identity, with the path excluded from content*, so that a recompile
/// cannot silently overwrite a different package and a player's mod cannot be replaced by a build.
/// Before this the name was the ticket's alone (`packages/BLD-0207.shpkg`), and that made a ticket
/// whose package declared nothing **permanent**: the writer refuses to overwrite a record, so the
/// empty package could never be replaced by a compiling one. `BLD-0207`…`BLD-0215` are exactly
/// that case, recorded as owed by C-100 §100.4.
///
/// **The digest is the package's own bytes**, so the name answers *which bytes*, and the record's
/// name answers *which piece of work*. `name` is the caller's and it must still be the record's own
/// name — a ticket id, never a prettified or timestamped one.
pub fn package_file_name(name: &str, digest: &Digest) -> String {
    format!("{name}-{}.{PACKAGE_EXTENSION}", digest.to_hex())
}

/// The path a package with this content occupies in the workspace.
pub fn package_path(workspace: &Path, name: &str, digest: &Digest) -> std::path::PathBuf {
    workspace.join(PACKAGE_DIR).join(package_file_name(name, digest))
}

/// **Every package filed for one record name, newest first by `(mtime, name)`.**
///
/// The name rule above means one piece of work can have more than one package over its life — the
/// first compile and every recompile after it — and a reader that wants *the package for this
/// ticket* must therefore be told which one it is. **`(mtime, name)` and not mtime alone**, for the
/// reason [`crate::filing`]'s sibling in the client gives: two files landing in the same instant
/// still resolve to one, deterministically; the name is the digest-derived one, so the tie is
/// broken by content rather than by chance.
///
/// A directory that cannot be listed yields an empty vector: *there is no package for this name* is
/// a state, and a caller refusing a read is a different failure the caller can make itself.
pub fn filed_packages(workspace: &Path, name: &str) -> Vec<std::path::PathBuf> {
    let prefix = format!("{name}-");
    let Ok(entries) = std::fs::read_dir(workspace.join(PACKAGE_DIR)) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, String, std::path::PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_package = path.extension().map(|ext| ext == PACKAGE_EXTENSION).unwrap_or(false);
        if !is_package {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some(content) = stem.strip_prefix(&prefix) else {
            continue;
        };
        // **The remainder must be the digest this module writes**, so `BLD-0207` cannot match a
        // package named `BLD-02070-…`: a prefix match alone would hand back another ticket's work.
        if content.len() < 32 || !content.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        found.push((modified, stem.to_string(), path));
    }
    found.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    found.reverse();
    found.into_iter().map(|(_, _, path)| path).collect()
}

/// The newest package filed for one record name, by the rule above — or `None`, which is the state
/// of a filing that has not been compiled (nothing was emitted for it, or it was emitted before
/// this naming rule and is not under this name).
pub fn newest_package(workspace: &Path, name: &str) -> Option<std::path::PathBuf> {
    filed_packages(workspace, name).into_iter().next()
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
        // **How many records the file holds, counted by the reader rather than assumed.** The
        // loop below asserts the count moves by exactly one per grow, so it needs the number it
        // starts from — and `folded` is the reader's own count, not a tally kept here.
        let mut before_growth_records =
            crate::compiler::read_back(&path).expect("reads").folded;

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
            // **And nothing in the file was skipped.** This is the assertion the doubling
            // defect walked straight through. The old test asked whether the file grew, which
            // a doubled append satisfies perfectly — while the appended half repeats the
            // chain's own history, and every record in it names a predecessor that is not the
            // record before it. `fold_bytes` refuses those and **keeps folding** (a corrupt
            // record loses that record, not the file), so the duplication hides in `corrupt`
            // as a number nobody was checking.
            //
            // Measured on the live record: `chains/workspace.chain` went 842,609,979 ->
            // 1,685,220,970 across one filing, and `2 x 842,609,979 + 1,012 =
            // 1,685,220,970` — the 1,012 bytes being the one record asked for.
            let back = crate::compiler::read_back(&path).expect("the grown file reads");
            assert_eq!(
                back.corrupt,
                0,
                "a grow appended the chain's own bytes again: {} record(s) in {} fold to a \
                 predecessor that is not the one before them. {}",
                back.corrupt,
                path.display(),
                back.corrupt_detail.join("; ")
            );
            assert_eq!(
                back.folded,
                before_growth_records + 1,
                "and the file holds exactly one more record than it did: {} folded",
                back.folded
            );
            before_growth_records = back.folded;
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

    /// **The doubling, reproduced on a scratch chain — twice over — and repaired by compaction.**
    ///
    /// The defect is reproduced by doing exactly what the broken `grow` did: take the chain that
    /// exists, render the next record **onto it**, and append that whole value to the file.
    ///
    /// **Doing it twice is the point of this test.** The first repair attempt cut the live file at
    /// its outermost repeat, found that boundary exactly (`842,609,979`), and was then refused by
    /// the fold: the live chain had been doubled **more than once**, so cutting the outermost copy
    /// off left the inner ones behind and the result held **166,406** records that did not link. A
    /// test that doubled a chain once would have passed the broken repair. So this one doubles it
    /// twice, which is what the record actually did: 105,323,302 → 842,609,979 → 1,685,220,970 is
    /// ×8 and then ×2 of a chain that kept gaining records.
    #[test]
    fn a_chain_doubled_twice_is_compacted_to_the_frames_that_link() {
        let workspace = scratch("doubling-twice");
        let namespace = voxel_model::chain::Namespace::new(
            "world-2468",
            1,
            voxel_model::chain::Season::Winter,
            "doubling",
        )
        .expect("a namespace");
        let record = |text: &'static str, event_kind: voxel_model::chain::EventKind| {
            crate::compiler::UnlinkedRecord {
                namespace: &namespace,
                event_kind,
                question: text,
                answer: None,
                kind: voxel_model::envelope::Kind::Evidence,
                event_type: voxel_model::envelope::EventType::UserDecision,
                state: voxel_model::envelope::RecordState::Answered,
                control: voxel_model::envelope::Control::Continue,
                actor: "agent-repair",
                round_id: "doubling",
                supersedes: None,
            }
        };

        // The honest chain: one history record and two tickets, since a ticket is what the board
        // folds and a repair that moved one would be removing work rather than a repeated copy.
        let first = linked_records(
            &workspace,
            &[],
            [record("the first record", voxel_model::chain::EventKind::DevelopmentFact)],
        )
        .expect("links");
        let path = append(&workspace, &first).expect("filed");
        let one = std::fs::read(&path).expect("reads");
        grow(
            &workspace,
            &one,
            [record("the second record", voxel_model::chain::EventKind::TicketTransition)],
        )
        .expect("grows");
        let honest = std::fs::read(&path).expect("reads");

        // **The first doubling**, in the shape the defect wrote it.
        let defect = linked_records(
            &workspace,
            &honest,
            [record("the record asked for", voxel_model::chain::EventKind::TicketTransition)],
        )
        .expect("renders onto the chain");
        let mut once = honest.clone();
        once.extend_from_slice(&defect);

        // **And the second**, on the doubled file: the whole of it again, plus one more record.
        let again = linked_records(
            &workspace,
            &once,
            [record("and the one after that", voxel_model::chain::EventKind::TicketTransition)],
        )
        .expect("renders onto the doubled chain");
        let mut twice = once.clone();
        twice.extend_from_slice(&again);
        std::fs::write(&path, &twice).expect("the doubled-twice file, written as bytes");

        // What the file is, by the reader's own count: the repeats do not link, and they are
        // counted rather than lost. **This is the state the live record was in.**
        let before = crate::compiler::fold_bytes(&twice).expect("the doubled file folds");
        assert!(before.corrupt > 0, "the repeated copies do not link");
        // Two records grown honestly, then one asked for at each doubling: four records fold, and
        // the copies of them do not. The copies are what this test is about.
        assert_eq!(before.folded, 4, "two grown records and two asked for");

        // The compaction keeps exactly the frames that folded — decided by `append_event`, the
        // reader's own rule, so the repair cannot disagree with the reader about linkage.
        let compacted = crate::compiler::compact(&twice).expect("the walk completes");
        assert_eq!(
            compacted.kept, before.folded,
            "the keep-set is the frames that folded, and nothing else is kept"
        );
        assert!(compacted.dropped > 0, "and something was dropped");
        let after = crate::compiler::fold_bytes(&compacted.bytes).expect("the compacted chain folds");
        assert_eq!(after.corrupt, 0, "the compacted chain folds clean");
        assert_eq!(after.folded, before.folded, "no record that linked was lost");
        assert_eq!(
            before.board.tickets().len(),
            after.board.tickets().len(),
            "and no ticket moved"
        );

        // The repair over the real file: it refuses if the result does not fold clean, if a record
        // or a ticket moves, or if nothing needs dropping at all.
        let keep = workspace.join("keep");
        let outcome =
            repair(&workspace, &keep).expect("a chain that holds unlinked records repairs");
        assert_eq!(outcome.dropped, compacted.dropped);
        assert_eq!(outcome.kept, before.folded);
        assert_eq!(outcome.corrupt_before, before.corrupt);
        assert_eq!(outcome.corrupt_after, 0, "the repaired record folds clean");
        assert_eq!(outcome.folded_before, outcome.folded_after);
        assert_eq!(outcome.tickets_before, outcome.tickets_after);
        assert!(
            keep.join("workspace.chain.as-found").exists(),
            "the record as it was found is kept as evidence rather than deleted"
        );
        let reread = crate::compiler::read_back(&path).expect("the repaired chain reads");
        assert_eq!(reread.corrupt, 0);
        assert_eq!(reread.folded, 4);
        // Three of the four are tickets: the first record is history, the rest are work.
        assert_eq!(reread.board.tickets().len(), 3);

        // **And the repair refuses to run again.** Nothing is left to drop, so a second call is
        // refused by name rather than quietly rewriting a sound record.
        std::fs::remove_file(keep.join("workspace.chain.repaired")).ok();
        let refused = repair(&workspace, &keep).expect_err("nothing to repair");
        assert!(
            refused.all()[0].detail().contains("nothing to repair"),
            "the refusal names what it found: {}",
            refused.all()[0].detail()
        );
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
