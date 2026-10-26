//! **HMAC-SHA256, because the seam needs a signature over bytes and this tree names no cipher.**
//!
//! The construction is RFC 2104's and the test vectors are RFC 4231's, kept here rather than trusted:
//! a hand-written MAC that is only exercised by its own round trip can be wrong in the same direction
//! on both ends. The two vectors below are the standard's, so a broken inner/outer pad order or a
//! digest that is not reset between them fails here and not in somebody's package.
//!
//! # Why a keyed digest and not a public-key signature
//!
//! The seam ([`voxel_package::signing`]) declares *who signed this*; this module is the one backend
//! the shipped build links. It is **symmetric**: the same key signs and checks, the key never leaves
//! the machine, and a verifier that does not hold the key refuses the signature as
//! [`voxel_model::RefusalKind::UntrustedKey`] rather than guessing. An asymmetric backend can replace
//! this module without touching the artifact: the artifact's gate asks a `Verifier`, not a key.

use sha2::{Digest as _, Sha256};

/// The block size of SHA-256, in bytes.
const BLOCK: usize = 64;

/// The signature over `message` with `key`.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // A key longer than the block is hashed first; a shorter one is zero-padded. Both are the RFC's
    // rule, and neither is optional: a long key used raw would change the pad length.
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for index in 0..BLOCK {
        inner_pad[index] ^= block[index];
        outer_pad[index] ^= block[index];
    }
    let inner = Sha256::new()
        .chain_update(inner_pad)
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(outer_pad)
        .chain_update(inner)
        .finalize()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_mac_matches_the_rfcs_vectors_and_not_only_its_own_round_trip() {
        // RFC 4231 test case 1.
        assert_eq!(
            hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // RFC 4231 test case 2 — the one that catches a swapped pad or an unreset digest.
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn a_key_longer_than_the_block_is_hashed_first_and_still_agrees_with_the_rfc() {
        // RFC 4231 test case 6: a 131-byte key, which must be hashed before padding.
        assert_eq!(
            hex(&hmac_sha256(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }
}
