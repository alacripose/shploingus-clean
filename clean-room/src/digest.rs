//! **Content identity — the project's one hash.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §1.3 (*"the world is rebuilt and reproduced with digests"*)
//! and the repository's dependency discipline (`Cargo.toml`): `sha2` is the one hash function the
//! project accepts for content identity, so the clean room has **one** place that names it and one
//! `Digest` type that everything else uses. A second hash, like a second permission matrix, is
//! refused by name (Constitution P5).
//!
//! A `Digest` is deliberately **not** a `String` and not a `bool`: it is a value that can be
//! compared, printed as hex, and folded over, and it cannot be mistaken for "was this checked?".

use std::fmt;

use sha2::{Digest as _, Sha256};

/// A SHA-256 content identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The identity of a byte string.
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Digest(hasher.finalize().into())
    }

    /// The identity of several byte strings, each **framed by its length** so no boundary between
    /// parts is ambiguous: two different lists of parts are two different identities, even when
    /// their bytes would concatenate the same way. The caller still declares the order.
    pub fn of_parts(parts: &[&[u8]]) -> Self {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update((part.len() as u64).to_le_bytes());
            hasher.update(part);
        }
        Digest(hasher.finalize().into())
    }

    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The digest of exactly 32 bytes — the inverse of [`Digest::as_bytes`], for a reader that has
    /// already framed one digest's worth of bytes off a wire or out of a save
    /// (`specs/006-world-saves-and-seeds/contracts/u9-seams.md` C2).
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Digest(bytes)
    }

    /// Parse a 64-character lower-case hex digest. **Refuses** anything else — a different length,
    /// a non-hex character, or upper case — so a wire reader cannot silently accept an encoding the
    /// writer never produces.
    pub fn from_hex(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.len() != 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
        }
        Some(Digest(bytes))
    }

    /// Lower-case hexadecimal. The one representation a report prints.
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_digest_is_stable_and_input_sensitive() {
        // The canonical SHA-256 of the empty string, so a wrong hash cannot pass quietly.
        assert_eq!(
            Digest::of(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(Digest::of(b"a").to_hex().len(), 64);
        assert_ne!(Digest::of(b"a"), Digest::of(b"b"));
        // Parts are framed, so a list of parts is not the raw concatenation, and order matters.
        assert_ne!(Digest::of_parts(&[b"a", b"b"]), Digest::of(b"ab"));
        assert_ne!(
            Digest::of_parts(&[b"a", b"b"]),
            Digest::of_parts(&[b"b", b"a"])
        );
    }

    #[test]
    fn part_boundaries_are_framed_and_not_ambiguous() {
        // The audit's case: with framing absent, these two part lists were one identity.
        assert_ne!(
            Digest::of_parts(&[b"ab", b"c"]),
            Digest::of_parts(&[b"a", b"bc"]),
            "a different boundary must be a different identity"
        );
        // And a list of parts is not the raw concatenation of those parts.
        assert_ne!(Digest::of_parts(&[b"a", b"b"]), Digest::of(b"ab"));
        // Order still matters, and the framing is stable.
        assert_ne!(Digest::of_parts(&[b"a", b"b"]), Digest::of_parts(&[b"b", b"a"]));
        assert_eq!(Digest::of_parts(&[b"ab", b"c"]), Digest::of_parts(&[b"ab", b"c"]));
    }

    #[test]
    fn from_hex_refuses_upper_case_as_documented() {
        let lower = Digest::of(b"x").to_hex();
        assert!(Digest::from_hex(&lower).is_some());
        assert!(
            Digest::from_hex(&lower.to_uppercase()).is_none(),
            "the reader documents lower-case only"
        );
        let letter = lower
            .find(|c: char| c.is_ascii_alphabetic())
            .expect("a hex digest has letters");
        let mut mixed = lower.clone();
        mixed.replace_range(letter..letter + 1, &lower[letter..letter + 1].to_uppercase());
        assert!(Digest::from_hex(&mixed).is_none(), "mixed case is not the documented form");
    }
}
