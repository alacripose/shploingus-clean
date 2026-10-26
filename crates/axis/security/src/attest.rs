//! **An attestation: a named subject, the digest a build produced, and the signature over both.**
//!
//! The subject is what was built (`build:shploingus`, `package:T-0954`), the digest is the identity
//! of its bytes, and the signature covers a **domain-separated** encoding of the pair
//! ([`payload`]): the tag names the scheme, and the two fields are length-prefixed by
//! [`Identity::of_fields`], so `("ab", "c")` and `("a", "bc")` cannot sign the same bytes.
//!
//! # A signature copied onto another change is not an attestation
//!
//! The signature is over the digest, not over the subject alone. Taking a signature from one
//! attestation and putting it on another is exactly what [`Attestation::holds`] refuses with
//! `SignatureInvalid`: the fields a reader sees were never covered by the bytes that were signed.

use voxel_model::refusal::RefusalSet;
use voxel_model::Identity;
use voxel_package::signing::{Signature, Verifier};

/// The domain tag of the attestation encoding. A second consumer of [`Identity::of_fields`] with the
/// same tag would be a second interpretation of the same bytes.
const TAG: &str = "voxel-axis-security/attestation";

/// A build hash, and the signature that says who attested it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    pub(crate) subject: String,
    pub(crate) digest: Identity,
    pub(crate) signature: Signature,
}

impl Attestation {
    /// What was built.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The identity of its bytes.
    pub fn digest(&self) -> Identity {
        self.digest
    }

    /// The signature over [`payload`] of the fields above.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// **Does the verifier hold a signature over these fields?** The payload is recomputed from the
    /// fields a reader sees, so a digest or a subject edited after signing is refused rather than
    /// inherited from the signature.
    pub fn holds(&self, verifier: &dyn Verifier) -> Result<(), RefusalSet> {
        verifier.verify(&payload(&self.subject, self.digest), &self.signature)
    }
}

/// The bytes a signature over `(subject, digest)` covers: one canonical identity, written out.
pub(crate) fn payload(subject: &str, digest: Identity) -> Vec<u8> {
    Identity::of_fields(TAG, &[subject.as_bytes(), digest.to_string().as_bytes()])
        .to_string()
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Security;
    use voxel_model::refusal::RefusalKind;

    fn temp(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-security-attest-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("checkout")).expect("a scratch checkout");
        (root.join("checkout"), root.join("key"))
    }

    #[test]
    fn an_attestation_holds_for_its_own_fields_and_no_others() {
        let (checkout, keys) = temp("fields");
        let security = Security::open(&checkout, keys.join("signing.key")).expect("a key");
        let build = Identity::of(b"the build's bytes");
        let attestation = security
            .attest("build:shploingus", build)
            .expect("an attestation");
        assert_eq!(attestation.subject(), "build:shploingus");
        assert_eq!(attestation.digest(), build);
        attestation
            .holds(&security)
            .expect("the key that signed it");

        // The signature was over the old digest, and the digest a reader sees is the new one.
        let mut edited = attestation.clone();
        edited.digest = Identity::of(b"someone else's bytes");
        let refused = edited.holds(&security).expect_err("a reused signature");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid),
            "{refused:?}"
        );

        // A subject is part of the signed bytes too.
        let mut renamed = attestation.clone();
        renamed.subject = "build:something-else".to_string();
        assert!(renamed.holds(&security).is_err());
        let _ = std::fs::remove_dir_all(checkout.parent().unwrap());
    }

    #[test]
    fn a_foreign_key_does_not_hold_the_signature_and_the_refusal_says_whose_it_is() {
        let (checkout, keys) = temp("foreign");
        let mine = Security::open(&checkout, keys.join("mine.key")).expect("a key");
        let other = Security::open(&checkout, keys.join("other.key")).expect("another key");
        let attestation = mine
            .attest("build:shploingus", Identity::of(b"bytes"))
            .expect("an attestation");
        let refused = attestation
            .holds(&other)
            .expect_err("not this key's signature");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UntrustedKey
                    && refusal.detail().contains(mine.key_id().as_str())),
            "the refusal names the key it was made by: {refused:?}"
        );
        let _ = std::fs::remove_dir_all(checkout.parent().unwrap());
    }

    #[test]
    fn an_attestation_of_nothing_is_refused_rather_than_signed() {
        let (checkout, keys) = temp("empty");
        let security = Security::open(&checkout, keys.join("signing.key")).expect("a key");
        let refused = security
            .attest("   ", Identity::of(b"bytes"))
            .expect_err("an unnamed subject");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        let _ = std::fs::remove_dir_all(checkout.parent().unwrap());
    }
}
