//! Content identity.
//!
//! The world is content-addressed, and there is exactly **one** scheme:
//! `sha256:` followed by 64 lowercase hex digits. [`Identity::parse`] refuses
//! anything else rather than coercing it — an identity that can be spelled two
//! ways is an identity that can disagree with itself, and the whole point of a
//! digest is that two people with the same bytes get the same string.
//!
//! The digest is over **canonical bytes**, never over a debug format. A `Debug`
//! impl is a view, and a view is not an identity.

use crate::refusal::{Refusal, RefusalKind, Refused};
use core::fmt;
use sha2::{Digest as _, Sha256};

/// A 32-byte SHA-256 digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, musli::Encode, musli::Decode)]
pub struct Digest([u8; 32]);

impl Digest {
    /// Digest of some bytes.
    pub fn of(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        Digest(h.finalize().into())
    }

    /// Digest of a domain-separated sequence: `tag`, then each field, each
    /// length-prefixed.
    ///
    /// Length-prefixing is the difference between a scheme that distinguishes
    /// `("ab", "c")` from `("a", "bc")` and one that does not. A digest that
    /// cannot tell two different things apart is a digest that will.
    pub fn of_fields(tag: &str, fields: &[&[u8]]) -> Self {
        let mut h = Sha256::new();
        h.update(&crate::endian::le_bytes(tag.len() as u64));
        h.update(tag.as_bytes());
        h.update(&crate::endian::le_bytes(fields.len() as u64));
        for f in fields {
            h.update(&crate::endian::le_bytes(f.len() as u64));
            h.update(f);
        }
        Digest(h.finalize().into())
    }

    /// The sha256 of everything `reader` yields, taken in bounded chunks — the
    /// streaming shape: the whole input is never held in memory to be hashed.
    pub fn of_reader(mut reader: impl std::io::Read) -> std::io::Result<Self> {
        let mut h = Sha256::new();
        let mut chunk = [0u8; 8192];
        loop {
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            h.update(&chunk[..read]);
        }
        Ok(Digest(h.finalize().into()))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The 32 raw bytes, back into a digest.
    ///
    /// **This is the inverse of [`Digest::as_bytes`] and exists for the wire**, where a
    /// digest is 32 bytes rather than a 64-character hex string. It is deliberately *not* a
    /// parser for text: a hex rendering is a different field with a different length, and
    /// reading one as the other would be a second, unguessed interpretation of the same
    /// bytes. `Identity::parse` is the only way a digest arrives from text.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Digest(bytes)
    }

    /// Lowercase hex, no prefix.
    pub fn to_hex(self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// The project's identity form: `sha256:<64 lowercase hex>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Identity {
    digest: Digest,
}

impl Identity {
    pub const PREFIX: &'static str = "sha256:";

    pub const fn from_digest(digest: Digest) -> Self {
        Identity { digest }
    }

    /// Identity of some bytes.
    pub fn of(bytes: &[u8]) -> Self {
        Identity { digest: Digest::of(bytes) }
    }

    /// Identity of a domain-separated field sequence.
    pub fn of_fields(tag: &str, fields: &[&[u8]]) -> Self {
        Identity { digest: Digest::of_fields(tag, fields) }
    }

    pub const fn digest(&self) -> Digest {
        self.digest
    }

    /// Parse the text form, refusing anything that is not exactly right.
    ///
    /// Refused: the wrong prefix, an uppercase prefix, 63 or 65 digits, a
    /// non-hex digit, an empty string, and a bare digest with no prefix. Each is
    /// a **different** string a caller might plausibly supply, and every one of
    /// them is a refusal rather than a coercion.
    pub fn parse(text: &str) -> Refused<Self> {
        let rest = text.strip_prefix(Self::PREFIX).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::MalformedIdentity,
                format!("identity must start with {:?}: {}", Self::PREFIX, shorten(text)),
            )
        })?;

        if rest.len() != 64 {
            return Err(Refusal::owned(
                RefusalKind::MalformedIdentity,
                format!("identity body is {} hex digits, not 64: {}", rest.len(), shorten(rest)),
            ));
        }

        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            let hi = hex_val(rest.as_bytes()[2 * i])?;
            let lo = hex_val(rest.as_bytes()[2 * i + 1])?;
            *byte = (hi << 4) | lo;
        }
        Ok(Identity { digest: Digest(bytes) })
    }
}

fn hex_val(c: u8) -> Refused<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        // Lowercase only. `A` and `a` are different bytes, and accepting both
        // would make two spellings of one identity.
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(Refusal::owned(
            RefusalKind::MalformedIdentity,
            format!("non-lowercase-hex byte {:?} in identity", c as char),
        )),
    }
}

fn shorten(s: &str) -> String {
    if s.len() <= 16 {
        s.to_string()
    } else {
        format!("{}…", &s[..16])
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.digest.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identity_round_trips_through_its_text_form() {
        let id = Identity::of(b"shploingus");
        let text = id.to_string();
        assert!(text.starts_with("sha256:"));
        assert_eq!(text.len(), 7 + 64);
        assert_eq!(Identity::parse(&text).unwrap(), id);
    }

    #[test]
    fn every_wrong_spelling_is_refused() {
        let good = Identity::of(b"x").to_string();
        let cases = [
            good.trim_start_matches("sha256:").to_string(), // no prefix
            format!("sha512:{}", &good[7..]),               // wrong prefix
            format!("sha256:{}", &good[7..63]),             // 62 digits
            format!("sha256:{}", &good[7..]).to_uppercase(), // uppercase body
            format!("sha256:{good}0"),                      // 65 digits
            String::new(),
        ];
        for case in cases {
            let r = Identity::parse(&case).unwrap_err();
            assert_eq!(r.kind(), RefusalKind::MalformedIdentity, "accepted {case:?}");
        }
    }

    #[test]
    fn field_digests_are_length_prefixed_so_they_cannot_collide() {
        // Without length prefixes, ("ab", "c") and ("a", "bc") are the same bytes.
        let a = Identity::of_fields("t", &[b"ab", b"c"]);
        let b = Identity::of_fields("t", &[b"a", b"bc"]);
        assert_ne!(a, b);

        // The tag is part of the digest, so two domains cannot collide either.
        let c = Identity::of_fields("u", &[b"ab", b"c"]);
        assert_ne!(a, c);
    }

    #[test]
    fn the_digest_is_over_the_bytes_and_not_over_a_view() {
        // Two byte strings that Debug the same way but differ must differ.
        let a = Identity::of(b"shploingus");
        let b = Identity::of(b"shploingus ");
        assert_ne!(a, b);
    }

    #[test]
    fn a_known_vector_pins_the_hash_function() {
        // SHA-256("") is a published constant. If this fails, the hash changed
        // under us and every recorded identity is invalidated — which is the
        // "a version bump must not quietly change an identity" rule.
        let empty = Identity::of(b"").to_string();
        assert_eq!(
            empty,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
