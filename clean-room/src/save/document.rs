//! **A save as a document — the written form crossing a boundary by reference rather than by value.**
//!
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` **§16.2** ruled this form *owed* and named exactly what it
//! waits on: *"the gate becomes a **hard** prerequisite only for the other form of a save: one that
//! crosses the API as a **document consumed in place by reference** (`path + digest`)"* — and
//! `specs/006-world-saves-and-seeds/plan.md` names it again among what the first slice does not land.
//! It was landed on 2026-10-12 (§28).
//!
//! # What the document form is, and what it is not
//!
//! A save that *stays in one process* is a byte string: write it, parse it, load it (the replay form
//! `save::mod` already owns). A save that **crosses a boundary** — a client, a mod, another session —
//! cannot carry those bytes as bytes: the API's own rule is that a reference is `path + digest`, never
//! a body (a row that could carry a body would be a second copy of the thing it declares). So the form
//! this file adds is the honest one: **the world is written where it is declared to live, and read
//! back where it lives, by name and by digest.**
//!
//! * [`Document`] is what a save *is* on a disk: the path as declared, the digest it must hash to, and
//!   its length. It carries no bytes and no world — a document is not a second save.
//! * [`Document::write`] is the **one** writer: the bytes U9's one encoder produced
//!   ([`Save::of`]/[`Save::bytes`]), placed at the declared reference under a declared root.
//! * [`Save::read_document`] is the **one** reader of this form, and it reads through **U6's gate**
//!   ([`Intake::read`]): the bytes are read where they live and handed over only when they hash to the
//!   digest the reference declares, so a document that is not there refuses `source_absent` and one
//!   whose bytes have moved refuses `source_digest_mismatch` — by name, with both values. Only then
//!   does U9's one parser ([`Save::parse`]) decide whether those bytes are a save at all
//!   (`not_a_save`, `save_truncated`, `save_tampered`).
//!
//! # Why the gate is a *prerequisite* rather than a neighbour
//!
//! There is no `fs::read` in this file. A save document is read by the content gate's own in-place
//! reader or it is not read at all, and this module has no second way to reach a path — so the two
//! facts the plan asks for (*the document is consumed in place*; *the content gate is hard*) are the
//! **shape of the code**, not a policy a caller could forget. The root is the gate's own declaration
//! ([`Intake::root`]), which is also where [`Document::write`] places the bytes when the gate's row
//! writes one, so a session that saves and loads through one gate cannot write where it will not look.

use std::path::Path;

use crate::digest::Digest;
use crate::intake::{io_reason, Intake, Source};
use crate::refusal::{RefusalKind, RefusalSet};

use super::Save;

/// **A save as a document**: the reference it is declared at, the digest its bytes hash to, and how
/// long it is. Not a save and not a copy of one — the bytes stay where they live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    path: String,
    digest: Digest,
    len: usize,
}

impl Document {
    /// **Write a save down.** The bytes are the ones U9's one encoder produced, placed at `path` under
    /// `root` — the reference as a save carries it (relative, and never a path the framing's own
    /// separators would break; see [`crate::intake`]'s reference rule). The document handed back
    /// describes what was written, so a caller holds the reference rather than a path it hopes about.
    ///
    /// A write that failed is refused [`RefusalKind::DocumentUnwritable`], in the **same stable words**
    /// a failed read uses ([`crate::intake::io_reason`]): a caller that placed a save where it could
    /// not be written must not be told a save exists.
    pub fn write(root: &Path, path: &str, bytes: &[u8]) -> Result<Self, RefusalSet> {
        if let Err(error) = std::fs::write(root.join(path), bytes) {
            let mut set = RefusalSet::new();
            set.refuse(
                RefusalKind::DocumentUnwritable,
                format!(
                    "the document `{path}` could not be written under the reference root: {}",
                    io_reason(&error)
                ),
            );
            return Err(set);
        }
        Ok(Document {
            path: path.to_string(),
            digest: Digest::of(bytes),
            len: bytes.len(),
        })
    }

    /// The path as declared — relative to the gate's root, and the string a report prints.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The digest the bytes at that path must hash to.
    pub const fn digest(&self) -> Digest {
        self.digest
    }

    /// How many bytes the document holds.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the document holds no bytes at all. Legal as a value and never a save: the layout
    /// opens with a magic and ends with a trailer, so a document that holds nothing is refused
    /// `not_a_save` by the one parser rather than read as an empty world.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Save {
    /// **Read a document back.** The declared reference is consumed **in place** through the content
    /// gate's one reader ([`Intake::read`]) — so an absent document and a document whose bytes have
    /// moved are refused by their own names, with both values — and the bytes that did come back are
    /// parsed by U9's one parser. Nothing is read twice and nothing is carried: what a caller gets is
    /// the parsed save and the document it came from.
    ///
    /// The order of the two questions is deliberate. *Is the document the one the reference declares?*
    /// is the gate's question and it is asked first, because those bytes have not been interpreted as
    /// anything yet; *is it a save of the world it claims?* is this file's, and it is asked after the
    /// bytes are known to be the declared ones. A fault is therefore named at the level it happened:
    /// `source_absent`/`source_digest_mismatch` for the document, `not_a_save`/`save_truncated`/
    /// `save_tampered`/… for the save.
    pub fn read_document(intake: &Intake<'_>, source: &Source) -> Result<(Save, Document), RefusalSet> {
        let bytes = intake.read(source)?;
        let document = Document {
            path: source.path().to_string(),
            digest: source.digest(),
            len: bytes.len(),
        };
        let save = Save::parse(&bytes)?;
        Ok((save, document))
    }
}
