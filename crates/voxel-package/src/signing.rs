//! The signing seam: **who** signed a payload, and does it check out.
//!
//! # What this module is, and what it is for
//!
//! Two questions, deliberately not one question:
//!
//! | Question | Answered by | Refused as |
//! |---|---|---|
//! | Is this signature over these bytes, by this key? | [`Verifier::verify`] | `SignatureInvalid` |
//! | Is this key one I hold? | the verifier, about itself | `UntrustedKey` |
//! | Which algorithm is this, and do I implement it? | the verifier, about itself | `UnknownSignatureAlgorithm` |
//!
//! Collapsing the first two is the classic failure. A container that carries its
//! own verifying key can be verified by anyone and trusted by no one, because
//! the message supplies its own authority. So a [`Signature`] **names** the key
//! that made it and never carries it. Where the trusted key comes from is a
//! policy question held elsewhere, not this module's.
//!
//! # Why there is no cryptography in this file
//!
//! This crate names **no** cryptographic implementation, and that is the design
//! rather than an omission. Ruling R-59.2 requires a **crate boundary** between
//! the seam and the backend, and gives the reason: a signature that can be
//! bypassed by a build flag is not a signature.
//!
//! A Cargo feature would fail that test — a flag that turns verification off is
//! exactly the thing being forbidden. A crate boundary satisfies it structurally:
//! a build that does not depend on `voxel-envelope` cannot verify anything, and
//! what it gets instead is [`NoSigningBackend`], which **refuses every package**.
//! There is no flag to flip and nothing to disable by accident.
//!
//! # Both traits are total
//!
//! Every implementation returns the same refusal types for the same inputs. No
//! implementation panics, unwraps, or substitutes a plausible value. A signer
//! that returned a zero-filled signature when it could not reach its key would be
//! a signer that signs for nobody while reporting that it signed — which is
//! [`NoSigningBackend`]'s entire reason for existing.
//!
//! # The absence is a value, not a silence
//!
//! Three different things can be true when a signature cannot be checked, and
//! they are three different refusals rather than one:
//!
//! * [`RefusalKind::NoSigningBackend`] — nothing is linked that could check it;
//! * [`RefusalKind::UntrustedKey`] — it **was** checked, and the key is unknown here;
//! * [`RefusalKind::OracleUnreachable`]'s sibling in the envelope world,
//!   [`RefusalKind::UnknownSignatureAlgorithm`] — the algorithm is one this build
//!   does not read.
//!
//! AWA3D's effort loop states the same principle in one line — chat "never goes
//! silent" (C-60) — and it is why each of these is rendered rather than collapsed.
//! An absence that is not rendered is indistinguishable from a pass.

use voxel_model::{Identity, Refusal, RefusalKind, RefusalSet};

/// The declared identity of a signing key: `sha256:` over the key's **public**
/// bytes.
///
/// A **name for** a key, never the key. It is what a container carries, what a
/// manifest pins, and what a refusal names — so a reader can be told *which* key
/// was refused without being handed key material, and two payloads signed by
/// different keys stay distinguishable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(String);

impl KeyId {
    /// The identity of a key's public bytes.
    ///
    /// Computed with [`Identity`], which is this repository's single identity
    /// form, so a key's name cannot drift away from every other identity the
    /// project uses. A key is content like any other content, and it is addressed
    /// the same way.
    pub fn of(public_bytes: &[u8]) -> Self {
        Self(Identity::of(public_bytes).to_string())
    }

    /// Refuse anything that is not exactly a `sha256:` identity.
    ///
    /// A foreign scheme is refused rather than coerced, because coercing it would
    /// silently weaken every digest that flows through it. The strictness is
    /// [`Identity`]'s own — a wrong prefix, a bare hex digest, an uppercase
    /// digit, 63 or 65 digits — and it is reused here rather than restated, so
    /// there is one definition of what an identity looks like.
    pub fn parse(value: &str) -> Result<Self, String> {
        Identity::parse(value)
            .map(|identity| Self(identity.to_string()))
            .map_err(|refusal| {
                format!(
                    "a key id must be a sha256 identity, and {value:?} is not one: {}",
                    refusal.detail()
                )
            })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A signature over a payload: the bytes, the algorithm, and the key that made
/// it.
///
/// The algorithm and the key travel **on the signature** rather than being asked
/// of the signer separately, so there is no way to build a signature whose
/// attribution and whose bytes came from different moments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    algorithm: u16,
    key_id: KeyId,
    bytes: Vec<u8>,
}

impl Signature {
    pub fn new(algorithm: u16, key_id: KeyId, bytes: Vec<u8>) -> Self {
        Self {
            algorithm,
            key_id,
            bytes,
        }
    }

    /// The signature algorithm's identifier.
    pub const fn algorithm(&self) -> u16 {
        self.algorithm
    }

    /// The **name** of the key that produced this. Safe to record and to display.
    pub const fn key_id(&self) -> &KeyId {
        &self.key_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// An empty signature is not a weak signature. It is **no** signature, and
    /// the distinction is the reason [`RefusalKind::UnsignedEnvelope`] exists.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Produce a signature.
pub trait Signer {
    /// Sign `payload`, or refuse.
    ///
    /// A refusal here is not "the signature is bad" — nothing was signed. It is
    /// the signer saying it could not sign, which is why
    /// [`NoSigningBackend`] is a refusal and not an empty signature.
    fn sign(&self, payload: &[u8]) -> Result<Signature, RefusalSet>;
}

/// Check a signature.
///
/// The verifier is the authority on **which key it holds** and **which algorithm
/// it implements**, so both of those refusals are made here rather than by the
/// envelope. A container that decided whether a verifier was the right one would
/// be a container that knows about trust, which is exactly what it must not do.
///
/// **Must be called before anything in `payload` is used.** A constraint enforced
/// by a return type cannot be skipped by a caller in a hurry.
pub trait Verifier {
    /// Verify `signature` over `payload`, or refuse.
    fn verify(&self, payload: &[u8], signature: &Signature) -> Result<(), RefusalSet>;
}

/// No signing backend is present. **Refuses everything, on purpose.**
///
/// This is the correct binding for any build that does not depend on
/// `voxel-envelope`. Its purpose is to make *"the signature was never checked"*
/// impossible to mistake for *"the signature checked out"* — the same shape as
/// the conformance oracle's unreachable default, and for the same reason.
pub struct NoSigningBackend {
    /// Why there is none, carried rather than logged, so the reason survives all
    /// the way to the refusal a reader actually sees.
    pub why: String,
}

impl NoSigningBackend {
    fn refusal(&self, subject: &str) -> RefusalSet {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::NoSigningBackend,
            format!(
                "{subject}: no signing backend is present ({}), so nothing can be signed and no \
                 signature can be checked. Refusing is the point: a missing instrument is not a \
                 pass",
                self.why
            ),
        ));
        refusals
    }
}

impl Signer for NoSigningBackend {
    fn sign(&self, _payload: &[u8]) -> Result<Signature, RefusalSet> {
        Err(self.refusal("sign"))
    }
}

impl Verifier for NoSigningBackend {
    fn verify(&self, _payload: &[u8], signature: &Signature) -> Result<(), RefusalSet> {
        Err(self.refusal(signature.key_id().as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A verifier that decides, so the seam can be tested without a backend.
    struct Decides(bool);

    impl Verifier for Decides {
        fn verify(&self, _payload: &[u8], signature: &Signature) -> Result<(), RefusalSet> {
            if self.0 {
                return Ok(());
            }
            let mut refusals = RefusalSet::new();
            refusals.push(Refusal::owned(
                RefusalKind::SignatureInvalid,
                format!(
                    "{}: the scripted verifier was told to refuse",
                    signature.key_id().as_str()
                ),
            ));
            Err(refusals)
        }
    }

    fn a_signature() -> Signature {
        Signature::new(0x0801, KeyId::of(b"a public key"), vec![0xAA; 16])
    }

    #[test]
    fn a_key_id_is_a_name_for_a_key_and_is_hashed_like_everything_else() {
        // The key's identity is this repository's one identity form, not a second
        // hashing scheme living beside it.
        assert_eq!(
            KeyId::of(b"a public key").as_str(),
            Identity::of(b"a public key").to_string()
        );
        assert!(KeyId::of(b"a public key").as_str().starts_with("sha256:"));
    }

    #[test]
    fn a_key_id_in_a_foreign_scheme_is_refused_rather_than_coerced() {
        assert!(KeyId::parse(&Identity::of(b"k").to_string()).is_ok());
        for foreign in [
            "md5:00ff",
            "00ff",
            "",
            "blake3:00ff",
            "sha256:NOTHEX",
            // A bare digest with no prefix is the shape a caller is most likely
            // to pass by mistake, so it is refused explicitly.
            &Identity::of(b"k").digest().to_hex(),
        ] {
            assert!(
                KeyId::parse(foreign).is_err(),
                "{foreign:?} names a scheme this project does not accept"
            );
        }
    }

    #[test]
    fn an_empty_signature_is_no_signature_and_says_so() {
        let empty = Signature::new(0x0801, KeyId::of(b"k"), Vec::new());
        assert!(empty.is_empty(), "an empty signature is not a weak one");
        assert!(!a_signature().is_empty());
    }

    #[test]
    fn with_no_backend_signing_refuses_rather_than_producing_an_empty_signature() {
        // The distinction the kind exists for. Returning an empty Signature would
        // look to every caller like a signature that failed to verify, which is a
        // far more reassuring story than "nothing was ever signed".
        let nowhere = NoSigningBackend {
            why: "no backend is linked".to_owned(),
        };
        let refusals = nowhere.sign(b"payload").expect_err("must refuse");
        assert_eq!(refusals.len(), 1, "one reason, named: {refusals}");
        assert!(refusals.all()[0].kind() == RefusalKind::NoSigningBackend);
    }

    #[test]
    fn with_no_backend_every_verification_is_refused_and_the_reason_survives() {
        let nowhere = NoSigningBackend {
            why: "voxel-envelope is not a dependency of this build".to_owned(),
        };
        let refusals = nowhere.verify(b"payload", &a_signature()).expect_err("must refuse");
        assert_eq!(refusals.len(), 1);
        let refusal = &refusals.all()[0];
        assert!(refusal.kind() == RefusalKind::NoSigningBackend);
        assert!(
            refusal.detail().contains("voxel-envelope"),
            "the carried reason must reach the reader: {}",
            refusal.detail()
        );
    }

    #[test]
    fn a_missing_backend_never_reports_the_signature_as_wrong() {
        // The dangerous direction. A reader told "signature invalid" would go and
        // inspect the signature and find it perfectly good.
        let nowhere = NoSigningBackend {
            why: "none".to_owned(),
        };
        let refusals = nowhere.verify(b"payload", &a_signature()).expect_err("must refuse");
        assert!(
            refusals.all()[0].kind() != RefusalKind::SignatureInvalid,
            "a missing instrument must not accuse the signature"
        );
        assert!(
            refusals.all()[0].kind() != RefusalKind::UntrustedKey,
            "nor may it claim the key is unknown, which would be a different fact"
        );
    }

    #[test]
    fn the_three_absences_are_three_different_refusals() {
        // "Cannot check" and "checked, key unknown" and "cannot read the
        // algorithm" are three facts. Collapsing any two is how an unsigned
        // package comes to look signed, or an honest publisher comes to look
        // like a forgery.
        let nowhere = NoSigningBackend {
            why: "none".to_owned(),
        };
        let missing = nowhere.verify(b"p", &a_signature()).expect_err("refuses");

        let mut untrusted = RefusalSet::new();
        untrusted.push(Refusal::owned(
            RefusalKind::UntrustedKey,
            "a stranger's key".to_owned(),
        ));

        let mut unknown_algorithm = RefusalSet::new();
        unknown_algorithm.push(Refusal::owned(
            RefusalKind::UnknownSignatureAlgorithm,
            "algorithm 0x9999".to_owned(),
        ));

        let kinds = [
            missing.all()[0].kind(),
            untrusted.all()[0].kind(),
            unknown_algorithm.all()[0].kind(),
        ];
        assert_ne!(kinds[0], kinds[1]);
        assert_ne!(kinds[1], kinds[2]);
        assert_ne!(kinds[0], kinds[2]);
    }

    #[test]
    fn a_verifier_is_the_authority_on_whether_it_can_check_a_signature() {
        // The container is not allowed to decide this, so it is testable with no
        // container involved: one verifier refuses and one does not, on the same
        // signature.
        assert!(Decides(true).verify(b"payload", &a_signature()).is_ok());
        let refusals = Decides(false).verify(b"payload", &a_signature()).expect_err("refuses");
        assert!(refusals.all()[0].kind() == RefusalKind::SignatureInvalid);
    }

    #[test]
    fn the_signature_carries_the_attribution_it_was_given() {
        let signature = a_signature();
        assert_eq!(signature.algorithm(), 0x0801);
        assert_eq!(signature.key_id(), &KeyId::of(b"a public key"));
        assert_eq!(signature.bytes().len(), 16);
    }
}
