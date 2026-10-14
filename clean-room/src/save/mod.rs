//! **The written form of a world — a save, and the load that must reproduce the world it came from.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §1.3 (the world is a pure function of `(seed, tick, declared
//! period)`, *"rebuilt and reproduced with digests"*), §2 **F1** (*"replay inputs recorded, not
//! derived"*) and **SPEC-3** (one canonical world authority); `LEDGER.md` **C-96.1** (the seed is a
//! property of a world, and *a save is record + seed + tick*) with its H-7.20 line; `docs/GAIA.md`
//! §5c/§11 (a save stays portable across mod changes **because each record names its ruleset
//! identity**); the **U9** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16.1 and row 9 of its §5;
//! and `specs/006-world-saves-and-seeds/` (this feature's spec, research, data model and contracts).
//!
//! # A save records inputs, and never derived state (F1)
//!
//! Identity (`seed`, declared period, tick), the five declared sub-streams' own seeds, the
//! configuration/ruleset identity, and the one record. That is all: the truth digest is **derived** at
//! load time and printed by the report, and neither the resident set nor any geometry is carried,
//! because a cache is not a world (SPEC-3).
//!
//! # Five seeds, not one dial renamed (plan §16.1)
//!
//! The mission names the seed systems **in the plural**. A save carries one seed per declared
//! [`Domain`] — that domain's own [`World::stream_seed`] — and a load checks **every** one, in both
//! directions: a save missing a domain this build declares is refused, and so is one naming a domain
//! this build does not.
//!
//! # Two questions, two names (P2)
//!
//! *Was this altered?* and *is this save of the world it claims?* are different faults. The layout's
//! own lengths are checked as they are read ([`RefusalKind::NotASave`],
//! [`RefusalKind::SaveTruncated`], [`RefusalKind::SeedUndeclared`]); the trailer digest is verified
//! once the layout accounts for every byte ([`RefusalKind::SaveTampered`]); and then the
//! correspondence — the ruleset, the five streams, and the record's declared head and length
//! ([`RefusalKind::IdentityMismatch`], [`RefusalKind::RecordDiverged`]) — is checked. A refusal never
//! yields a world: [`Loaded`] exists only when every check passed.
//!
//! # Two rules that keep a hostile file cheap (2026-10-12)
//!
//! **A declared count never decides what the reader spends.** The stream count is written in the
//! file, and a table entry is [`ENTRY`] bytes wide, so the most entries a file of this length can
//! hold follows from the bytes present. A count beyond that is a short file, named **once** — never a
//! reason to iterate two million entries that cannot be there. Work and memory are bounded by the
//! input's length, which is the only bound a reader of untrusted bytes may trust.
//!
//! **One file, one fault to report.** Every structural read is bounded by the trailer boundary, and
//! the reader stops at the first part it cannot read, so a truncated file is refused as *short*. A
//! table that could not be read must not then be reconciled: that would invent causes ("the save
//! declares no seed for the climate stream") for a file whose only fault is that it ends. Faults that
//! are genuinely independent — two domains absent, one tag undeclared — are still all collected.
//!
//! # The two forms, and the one that is no longer owed (2026-10-12, §28)
//!
//! The **replay form** is the bytes in hand: [`Save::of`] → [`Save::parse`] → [`Save::load`]. The
//! **document form** is those bytes at a declared reference, consumed in place as `path + digest`, and
//! it lives in [`document`]. `specs/006-world-saves-and-seeds/plan.md` named it owed *because* it
//! waits on U6's content gate — *"that is where U6's content gate becomes a **hard** prerequisite"*
//! (§16.2) — and it is landed now that the gate has one reader that hands a document's bytes over
//! ([`crate::intake::Intake::read`]): the save's reader is that gate, so the prerequisite is a
//! compile-time shape rather than a step a caller might skip.
//!
//! # Where each piece lives
//!
//! | [`layout`] | the byte level: `MAGIC`, `VERSION`, the one encoder, and the bounded cursor |
//! | [`load`] | the correspondence checks, [`Loaded`], [`LoadReport`] |
//! | [`document`] | the **document form**: where a save lives, and the one read that consumes it in
//!   place — through U6's content gate, which is why that gate is a *prerequisite* here |
//! | this file | what a save **is**: its identity, its seed declarations, and its writer/parser |
//! | [`tests`] | the layout, the parser, the loader and the document form driven directly |
//!
//! The split is by question rather than by size — *what are these bytes* ([`layout`]), *what is a save
//! made of* (here), *is this save of the world I am loading* ([`load`]) — so each file can be read
//! alone and the writer/parser never learn what a correspondence check is.

use crate::chain::Chain;
use crate::digest::Digest;
use crate::refusal::{RefusalKind, RefusalSet};
use crate::world::{Domain, World};

mod document;
mod layout;
mod load;

pub use document::Document;
pub use layout::{MAGIC, VERSION};
pub use load::{LoadReport, Loaded};

use layout::{encode, read_header, Reader, ENTRY, TRAILER};

/// The configuration/ruleset identity a save names. A load that declares a different one is refused
/// with both, because a save is portable across mod changes **only** when what it was made under is
/// written down (`docs/GAIA.md` §5c/§11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ruleset {
    /// The ruleset's own number.
    pub ruleset: u64,
    /// The revision of that ruleset.
    pub revision: u64,
}

impl Ruleset {
    pub const fn new(ruleset: u64, revision: u64) -> Self {
        Ruleset { ruleset, revision }
    }
}

/// One declared sub-stream and **its own** seed — the "seed systems" the mission names in the plural.
/// A save carries one per declared [`Domain`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamSeed {
    pub domain: Domain,
    pub seed: u64,
}

/// The identity a save writes down: the world's seed and declared period, and the tick it was written
/// at. The seed is a property of the world and the tick is not the seed (C-96.1), so both are carried
/// and neither is derived from the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveIdentity {
    pub seed: u64,
    pub period: u64,
    pub tick: u64,
}

/// A parsed save: the identity, the seed declarations, the ruleset identity, and the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    identity: SaveIdentity,
    ruleset: Ruleset,
    streams: Vec<StreamSeed>,
    record_head: Option<Digest>,
    record_len: u64,
    record_wire: String,
}

impl Save {
    /// **Write a world down.** The record must verify before it is written: a save never carries a
    /// record it could not re-adopt.
    pub fn of(world: &World, chain: &Chain, ruleset: Ruleset) -> Result<Vec<u8>, RefusalSet> {
        chain.verify()?;
        let identity = SaveIdentity {
            seed: world.identity().seed,
            period: world.identity().period.get(),
            tick: world.tick().0,
        };
        let streams: Vec<StreamSeed> = Domain::ALL
            .iter()
            .map(|domain| StreamSeed {
                domain: *domain,
                seed: world.stream_seed(*domain),
            })
            .collect();
        Ok(encode(
            &identity,
            ruleset,
            &streams,
            chain.head(),
            chain.len() as u64,
            &chain.wire(),
        ))
    }

    /// **Read the bytes back.** The layout is checked as it is read, every byte is accounted for, and
    /// the trailer digest is verified. A fault is named; `Save` is produced only when none was found.
    pub fn parse(bytes: &[u8]) -> Result<Save, RefusalSet> {
        let mut set = RefusalSet::new();

        // 1. The magic, before anything else is interpreted.
        if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
            set.refuse(
                RefusalKind::NotASave,
                format!(
                    "the bytes do not open with {:?} — {} byte(s) were read",
                    String::from_utf8_lossy(MAGIC),
                    bytes.len()
                ),
            );
            return Err(set);
        }

        // **The layout's own edge.** Everything the layout declares lies before the last [`TRAILER`]
        // bytes — they are the digest of what precedes them — so no structural read may cross into
        // them. Bounding reads here is what keeps a short file from being *interpreted*: absent
        // streams and a tampered trailer are causes the bytes cannot evidence.
        let expected_trailer = bytes.len().saturating_sub(TRAILER);
        let mut reader = Reader::bounded(bytes, MAGIC.len(), expected_trailer);

        // 2. The fixed header, then the streams, then the record. The first part that cannot be read
        //    is the file's fault, and it is the only one reported: `read_header` stops at it.
        let Some((version, ruleset, identity, count)) = read_header(&mut reader, &mut set) else {
            return Err(set);
        };

        //    **What a declared count may cost is bounded by the bytes present.** A table entry is
        //    [`ENTRY`] bytes, so a file of this length holds at most `remaining / ENTRY` of them; a
        //    count beyond that is a short file, named once, and no work follows from it.
        let remaining = expected_trailer.saturating_sub(reader.at);
        if u64::from(count) > (remaining / ENTRY) as u64 {
            set.refuse(
                RefusalKind::SaveTruncated,
                format!(
                    "the save declares {count} stream(s) and {remaining} byte(s) remain before the \
                     trailer, which hold {} of the {} byte(s) that table needs",
                    remaining / ENTRY,
                    u64::from(count) * ENTRY as u64
                ),
            );
            return Err(set);
        }

        let mut streams: Vec<StreamSeed> = Vec::new();
        for index in 0..count {
            let Some(tag) = reader.u8(&format!("stream {index} tag"), &mut set) else {
                return Err(set);
            };
            let Some(seed) = reader.u64(&format!("stream {index} seed"), &mut set) else {
                return Err(set);
            };
            // The tag is the domain's index in the closed declaration order.
            match Domain::ALL.get(tag as usize) {
                Some(domain) => streams.push(StreamSeed { domain: *domain, seed }),
                None => set.refuse(
                    RefusalKind::SeedUndeclared,
                    format!(
                        "stream {index} declares tag {tag}, and this build declares {} streams \
                         (0..{})",
                        Domain::COUNT,
                        Domain::COUNT
                    ),
                ),
            }
        }

        // 3. Which domains the save declares: every declared one, once, and no other. Both directions,
        //    because a roster that changes must not silently drop or absorb a stream's identity.
        for domain in Domain::ALL {
            let declared = streams.iter().filter(|stream| stream.domain == domain).count();
            if declared == 0 {
                set.refuse(
                    RefusalKind::SeedUndeclared,
                    format!(
                        "the save declares no seed for the {} stream, and this build declares it",
                        domain.name()
                    ),
                );
            } else if declared > 1 {
                set.refuse(
                    RefusalKind::SeedUndeclared,
                    format!(
                        "the save declares the {} stream {declared} times, and a stream has one seed",
                        domain.name()
                    ),
                );
            }
        }

        let Some(record_present) = reader.u8("record head flag", &mut set) else {
            return Err(set);
        };
        let record_head = match record_present {
            0 => None,
            1 => match reader.digest("record head", &mut set) {
                Some(head) => Some(head),
                None => return Err(set),
            },
            other => {
                set.refuse(
                    RefusalKind::NotASave,
                    format!("the record head flag is {other}, and the layout declares 0 or 1"),
                );
                return Err(set);
            }
        };
        let Some(record_len) = reader.u64("record length", &mut set) else {
            return Err(set);
        };
        let Some(wire_len) = reader.u64("record wire length", &mut set) else {
            return Err(set);
        };
        let wire_len = usize::try_from(wire_len).unwrap_or(usize::MAX);
        let Some(raw) = reader.take(wire_len, "record wire", &mut set) else {
            return Err(set);
        };
        let record_wire = match std::str::from_utf8(raw) {
            Ok(text) => text.to_string(),
            Err(_) => {
                set.refuse(
                    RefusalKind::NotASave,
                    "the record wire is not UTF-8, and the chain's wire form is text",
                );
                return Err(set);
            }
        };

        // 4. Every byte is accounted for: the record ends exactly where the trailer begins. A part
        //    that ran past that boundary was already refused where it was read, so this asks only
        //    whether the layout's own region holds anything it does not declare.
        if reader.at < expected_trailer {
            set.refuse(
                RefusalKind::NotASave,
                format!(
                    "{} byte(s) follow the record that the layout does not declare",
                    expected_trailer - reader.at
                ),
            );
            return Err(set);
        }

        // 5. Was this altered? The trailer is the digest of everything before it — compared only
        //    now, once the layout accounts for every byte, so a short file is never *also* called
        //    tampered (the trailer would otherwise be read out of the file's own header).
        let mut trailer_reader = Reader::new(bytes, expected_trailer);
        let mut trailer_probe = RefusalSet::new();
        if let Some(recorded) = trailer_reader.digest("trailer", &mut trailer_probe) {
            let recomputed = Digest::of(&bytes[..expected_trailer]);
            if recorded != recomputed {
                set.refuse(
                    RefusalKind::SaveTampered,
                    format!(
                        "the trailer says {} and the bytes before it say {}",
                        recorded.to_hex(),
                        recomputed.to_hex()
                    ),
                );
                return Err(set);
            }
        }

        // 6. A version the layout does not declare is refused rather than guessed at. Checked last on
        //    purpose: one flipped version byte is an alteration, and it is reported as the
        //    alteration it is.
        if version != VERSION {
            set.refuse(
                RefusalKind::NotASave,
                format!("the save declares version {version}, and this build reads {VERSION}"),
            );
            return Err(set);
        }

        set.into_result(Save {
            identity,
            ruleset,
            streams,
            record_head,
            record_len,
            record_wire,
        })
    }

    /// Re-encode a parsed save. Byte-identical to the bytes it was parsed from, for every save this
    /// writer produced — the layout is a value, not a convention.
    pub fn bytes(&self) -> Vec<u8> {
        encode(
            &self.identity,
            self.ruleset,
            &self.streams,
            self.record_head,
            self.record_len,
            &self.record_wire,
        )
    }

    pub const fn identity(&self) -> SaveIdentity {
        self.identity
    }

    pub const fn ruleset(&self) -> Ruleset {
        self.ruleset
    }

    pub fn streams(&self) -> &[StreamSeed] {
        &self.streams
    }

    /// The record the save **declares**: its head and its length.
    pub const fn record(&self) -> (Option<Digest>, u64) {
        (self.record_head, self.record_len)
    }
}

#[cfg(test)]
mod tests;
