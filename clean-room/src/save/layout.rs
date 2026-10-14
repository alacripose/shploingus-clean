//! **The declared layout — how a save's fields become bytes, and how those bytes become fields.**
//!
//! One layout, one encoder, one cursor. [`MAGIC`] and [`VERSION`] say which format these bytes claim
//! to be; [`TRAILER`] and [`ENTRY`] are the two widths the layout's own arithmetic needs; [`encode`]
//! is the only writer and [`Reader`] the only reader, so a save cannot be written one way and read
//! another (P5). The refusal side of reading lives here too: a part that runs past the slice, or past
//! the trailer the layout declares, is named together with the field it belongs to, and
//! [`read_header`] stops at the first part it cannot read so a short file is refused **once** rather
//! than once per field.
//!
//! Nothing here decides whether a save is *of this world* — that is [`super::load`]'s question — and
//! nothing here decides what a save is made of, which is [`super`]'s. This file is the byte level and
//! nothing else.

use crate::digest::Digest;
use crate::refusal::{RefusalKind, RefusalSet};
use crate::world::Domain;

use super::{Ruleset, SaveIdentity, StreamSeed};

/// The magic that opens every save. The only format this build reads: bytes that are not this layout
/// are refused, never reinterpreted (`specs/006-world-saves-and-seeds/data-model.md`).
pub const MAGIC: &[u8] = b"shploingus.save/1";

/// The layout's version. One version, one layout — there is no migration path, and a save claiming
/// another version is refused rather than guessed at.
pub const VERSION: u32 = 1;

/// The trailer's width in bytes: a [`Digest`] of every byte before it.
pub(super) const TRAILER: usize = 32;

/// The width of one stream entry in the declared table: a one-byte tag and an eight-byte seed. The
/// only number a declared stream count is ever measured against, so a count can never buy work the
/// file's own length cannot pay for.
pub(super) const ENTRY: usize = 9;

/// **The one encoder.** Every field in the declared order, every variable part framed by its length,
/// and the trailer over everything before it. `Save::of` and `Save::bytes` both come through here, so
/// there is one layout rather than one per caller (P5).
pub(super) fn encode(
    identity: &SaveIdentity,
    ruleset: Ruleset,
    streams: &[StreamSeed],
    record_head: Option<Digest>,
    record_len: u64,
    record_wire: &str,
) -> Vec<u8> {
    let tagged: Vec<(u8, u64)> = streams
        .iter()
        .map(|stream| {
            let tag = Domain::ALL
                .iter()
                .position(|domain| *domain == stream.domain)
                .unwrap_or(0) as u8;
            (tag, stream.seed)
        })
        .collect();
    encode_raw(identity, ruleset, &tagged, record_head, record_len, record_wire)
}

/// The layout itself, over **tags** rather than domains. A save this build cannot write — an
/// undeclared stream, a missing one, a doctored seed — is produced through here by the tests, so the
/// fault comes from the declared layout rather than from a hand-copied second copy of it.
pub(super) fn encode_raw(
    identity: &SaveIdentity,
    ruleset: Ruleset,
    streams: &[(u8, u64)],
    record_head: Option<Digest>,
    record_len: u64,
    record_wire: &str,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(160 + record_wire.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&ruleset.ruleset.to_le_bytes());
    out.extend_from_slice(&ruleset.revision.to_le_bytes());
    out.extend_from_slice(&identity.seed.to_le_bytes());
    out.extend_from_slice(&identity.period.to_le_bytes());
    out.extend_from_slice(&identity.tick.to_le_bytes());
    out.extend_from_slice(&(streams.len() as u32).to_le_bytes());
    for (tag, seed) in streams {
        out.push(*tag);
        out.extend_from_slice(&seed.to_le_bytes());
    }
    match record_head {
        Some(head) => {
            out.push(1);
            out.extend_from_slice(head.as_bytes());
        }
        None => out.push(0),
    }
    out.extend_from_slice(&record_len.to_le_bytes());
    out.extend_from_slice(&(record_wire.len() as u64).to_le_bytes());
    out.extend_from_slice(record_wire.as_bytes());
    let trailer = Digest::of(&out);
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// A byte cursor that **refuses** rather than panicking, and that knows where the layout ends: a part
/// that runs past the slice — or past the trailer the layout declares — is named, and the field it
/// belongs to is named with it.
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    pub(super) at: usize,
    /// The first byte this cursor may not read: the trailer's own start for a structural read, and
    /// the end of the slice for the trailer itself.
    limit: usize,
}

impl<'a> Reader<'a> {
    /// A cursor over the whole slice — used for the trailer, the one part beyond the layout's edge.
    pub(super) const fn new(bytes: &'a [u8], at: usize) -> Self {
        Reader { bytes, at, limit: bytes.len() }
    }

    /// A cursor that stops at `limit`, the trailer's own start.
    pub(super) const fn bounded(bytes: &'a [u8], at: usize, limit: usize) -> Self {
        Reader { bytes, at, limit }
    }

    pub(super) fn take(&mut self, n: usize, field: &str, set: &mut RefusalSet) -> Option<&'a [u8]> {
        let Some(end) = self.at.checked_add(n) else {
            set.refuse(
                RefusalKind::NotASave,
                format!("{field} declares {n} byte(s), which this layout cannot address"),
            );
            return None;
        };
        if end > self.limit {
            set.refuse(
                RefusalKind::SaveTruncated,
                format!(
                    "{field} needs {n} byte(s) from byte {}, and the layout's own trailer begins at \
                     byte {} — the file does not hold the save it declares",
                    self.at, self.limit
                ),
            );
            return None;
        }
        // `end <= limit <= bytes.len()`, so the slice exists: this is a refusal, never a panic.
        let raw = &self.bytes[self.at..end];
        self.at = end;
        Some(raw)
    }

    pub(super) fn u8(&mut self, field: &str, set: &mut RefusalSet) -> Option<u8> {
        self.take(1, field, set).map(|raw| raw[0])
    }

    pub(super) fn u32(&mut self, field: &str, set: &mut RefusalSet) -> Option<u32> {
        self.take(4, field, set)
            .map(|raw| u32::from_le_bytesbytes(raw.try_into().expect("four bytes")))
    }

    pub(super) fn u64(&mut self, field: &str, set: &mut RefusalSet) -> Option<u64> {
        self.take(8, field, set)
            .map(|raw| u64::from_le_bytesbytes(raw.try_into().expect("eight bytes")))
    }

    pub(super) fn digest(&mut self, field: &str, set: &mut RefusalSet) -> Option<Digest> {
        self.take(TRAILER, field, set).map(|raw| {
            let mut bytes = [0u8; TRAILER];
            bytes.copy_from_slice(raw);
            Digest::from_bytes(bytes)
        })
    }
}

/// The fixed part of the layout, read only if the file holds all of it: the first field that cannot
/// be read stops the read, so a short file is refused **once** rather than once per missing field.
pub(super) fn read_header(reader: &mut Reader<'_>, set: &mut RefusalSet) -> Option<(u32, Ruleset, SaveIdentity, u32)> {
    let version = reader.u32("version", set)?;
    let ruleset = Ruleset::new(reader.u64("ruleset", set)?, reader.u64("ruleset revision", set)?);
    let seed = reader.u64("seed", set)?;
    let period = reader.u64("declared period", set)?;
    let tick = reader.u64("tick", set)?;
    let count = reader.u32("stream count", set)?;
    Some((version, ruleset, SaveIdentity { seed, period, tick }, count))
}
