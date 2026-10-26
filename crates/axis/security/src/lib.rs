//! **The trust boundary's key: `PATCH-042`'s security layer, without the second record.**
//!
//! The artifact's pin gate has asked a [`Verifier`] since it was written, and every shipped build
//! answered with [`voxel_package::signing::NoSigningBackend`] — so a package was always refused at
//! the pin and the format's success path existed only in a test. This crate is the backend:
//!
//! | question | answered by | refused as |
//! |---|---|---|
//! | Where is this machine's key, and is it outside the checkout? | [`Security::machine_local`], [`Security::open`] | `UntrustedKey` for a path inside the tree; `NoSigningBackend` for a key that cannot be made or read |
//! | Who signed these bytes? | [`Security::sign`] ([`Signer`]) | a key that cannot be created, named with its path |
//! | Is this signature over these bytes by this key? | [`Security::verify`] ([`Verifier`]) | `UnknownSignatureAlgorithm`, `UnsignedEnvelope`, `UntrustedKey`, `SignatureInvalid` — four facts, not one |
//! | Was this build hash attested? | [`Security::attest`], [`Attestation::holds`] | a signature copied onto another digest or another subject |
//!
//! # The key never lives in the repository
//!
//! A key inside the checkout is in every clone, so it names no one. [`Security::open`] therefore
//! **refuses a path under the root it is given** — by name, with both paths in the refusal — and
//! [`Security::machine_local`] looks in the per-user data directory (`LOCALAPPDATA`, or
//! `XDG_DATA_HOME` / `~/.local/share`) instead. The repository itself has no key, and a test asserts
//! that no key file exists inside it.
//!
//! # The signature is a keyed digest, and the seam is the interface
//!
//! [`voxel_package::signing`] owns *who signed this* and deliberately names no cipher. This crate
//! links one: **HMAC-SHA256** ([`hmac`]), symmetric, with the key held by the machine. The artifact's
//! gate asks a `Verifier`, so an asymmetric backend replaces this one without touching the package
//! format — and a verifier that does not hold the key says `UntrustedKey` rather than guessing.
//!
//! # What is deliberately not here
//!
//! No second record (the chain is canonical), no key generation inside the tree, and **no build gate
//! in `voxel-model/build.rs`**: a build script that walks 300 files for a stray `.env` is a door that
//! teaches people to delete it. That requirement is recorded in that file's own header and measured
//! by this crate's containment test instead.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod attest;
pub mod hmac;

pub use attest::Attestation;
pub use hmac::hmac_sha256;

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use voxel_model::refusal::{RefusalKind, RefusalSet};
use voxel_model::Identity;
use voxel_package::signing::{KeyId, Signature, Signer, Verifier};

/// The one signature algorithm this build implements. Written on every signature so a verifier can
/// refuse an algorithm it does not read instead of checking the bytes and reporting a forgery.
pub const HMAC_SHA256: u16 = 1;

/// A key is 32 bytes: the digest width this project already uses, and HMAC's own natural key size.
pub const KEY_BYTES: usize = 32;

/// This machine's key, the name of that key, and the file it was read from.
#[derive(Clone, Debug)]
pub struct Security {
    key: [u8; KEY_BYTES],
    key_id: KeyId,
    path: PathBuf,
}

impl Security {
    /// **The machine's identity.** The key lives in the per-user data directory, outside every
    /// checkout, and is created on first use when it is not there.
    pub fn machine_local(root: &Path) -> Result<Security, RefusalSet> {
        let base = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA")
        } else {
            std::env::var_os("XDG_DATA_HOME").or_else(|| {
                std::env::var_os("HOME").map(|home| {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .into_os_string()
                })
            })
        };
        let Some(base) = base else {
            return Err(refusal(
                RefusalKind::NoSigningBackend,
                "no per-user data directory is set (LOCALAPPDATA, XDG_DATA_HOME or HOME), so there \
                 is nowhere to keep a signing key outside the checkout. Refusing is the point: a \
                 key kept inside the tree would be in every clone"
                    .to_string(),
            ));
        };
        Security::open(
            root,
            PathBuf::from(base).join("shploingus").join("signing.key"),
        )
    }

    /// Open the key at `path`, creating it if it is not there, and **refuse a path inside `root`**.
    ///
    /// The containment check is made on the canonical parent plus the file name, after the parent is
    /// created: a `..` in a path that does not exist yet cannot be resolved by `canonicalize`, and a
    /// key path that escapes on paper but lands inside the tree is the case worth refusing.
    pub fn open(root: &Path, path: impl AsRef<Path>) -> Result<Security, RefusalSet> {
        let root = root.canonicalize().map_err(|error| {
            refusal(
                RefusalKind::NoReading,
                format!(
                    "the checkout {} cannot be resolved ({error}), so a key path cannot be checked \
                     against it",
                    root.display()
                ),
            )
        })?;
        let path = absolute(path.as_ref())?;
        let parent = path.parent().ok_or_else(|| {
            refusal(
                RefusalKind::UntrustedKey,
                format!("{} names no directory to hold a key", path.display()),
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|error| {
            refusal(
                RefusalKind::NoSigningBackend,
                format!(
                    "the key's directory {} cannot be made ({error}), so no key can exist there",
                    parent.display()
                ),
            )
        })?;
        let parent = parent.canonicalize().map_err(|error| {
            refusal(
                RefusalKind::NoSigningBackend,
                format!(
                    "the key's directory {} cannot be resolved ({error})",
                    parent.display()
                ),
            )
        })?;
        let path = path
            .file_name()
            .map_or(parent.clone(), |name| parent.join(name));
        if path.starts_with(&root) {
            return Err(refusal(
                RefusalKind::UntrustedKey,
                format!(
                    "the key path {} is inside the checkout {}: a key in the repository is in every \
                     clone, so it names no one. Hold the key outside the tree and pass that path",
                    path.display(),
                    root.display()
                ),
            ));
        }
        let key = match std::fs::read(&path) {
            Ok(bytes) => parse(&bytes, &path)?,
            Err(error) if error.kind() == ErrorKind::NotFound => create(&path)?,
            Err(error) => {
                return Err(refusal(
                    RefusalKind::NoSigningBackend,
                    format!(
                        "the key at {} cannot be read ({error}); a signature from a key nobody can \
                         read is not a signature",
                        path.display()
                    ),
                ))
            }
        };
        Ok(Security {
            key_id: KeyId::of(&key),
            key,
            path,
        })
    }

    /// The **name** of this machine's key — safe to print, and what a refusal quotes.
    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }

    /// The file the key was read from. Never the key itself.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// **Attest a build hash**: the subject names what was built, the digest is its identity, and
    /// the signature covers both ([`attest::payload`]).
    pub fn attest(&self, subject: &str, digest: Identity) -> Result<Attestation, RefusalSet> {
        if subject.trim().is_empty() {
            return Err(refusal(
                RefusalKind::MalformedRecord,
                "an attestation of no subject attests nothing; name what was built".to_string(),
            ));
        }
        let signature = self.sign(&attest::payload(subject, digest))?;
        Ok(Attestation {
            subject: subject.to_string(),
            digest,
            signature,
        })
    }
}

impl Signer for Security {
    fn sign(&self, payload: &[u8]) -> Result<Signature, RefusalSet> {
        Ok(Signature::new(
            HMAC_SHA256,
            self.key_id.clone(),
            hmac_sha256(&self.key, payload).to_vec(),
        ))
    }
}

impl Verifier for Security {
    fn verify(&self, payload: &[u8], signature: &Signature) -> Result<(), RefusalSet> {
        if signature.algorithm() != HMAC_SHA256 {
            return Err(refusal(
                RefusalKind::UnknownSignatureAlgorithm,
                format!(
                    "the signature names algorithm {} and this build implements {HMAC_SHA256}; the \
                     bytes were not checked, so nothing is known about them",
                    signature.algorithm()
                ),
            ));
        }
        if signature.is_empty() {
            return Err(refusal(
                RefusalKind::UnsignedEnvelope,
                format!(
                    "{} is named but the signature holds no bytes; an absent signature is not a \
                     weak one",
                    signature.key_id().as_str()
                ),
            ));
        }
        if signature.key_id() != &self.key_id {
            return Err(refusal(
                RefusalKind::UntrustedKey,
                format!(
                    "the signature was made by {} and this machine holds {}. A valid signature by an \
                     unknown key is not a forgery, and it is not this verifier's to accept",
                    signature.key_id().as_str(),
                    self.key_id.as_str()
                ),
            ));
        }
        let expected = hmac_sha256(&self.key, payload);
        if signature.bytes() != expected {
            return Err(refusal(
                RefusalKind::SignatureInvalid,
                format!(
                    "the signature names {} but is not over these bytes: the key holds, the \
                     signature does not",
                    self.key_id.as_str()
                ),
            ));
        }
        Ok(())
    }
}

/// The key file's bytes, or a refusal that names the path. **A file that only looks like a key is
/// refused rather than hashed**: 64 lowercase hex characters, no prefix and no whitespace beyond a
/// trailing newline.
fn parse(bytes: &[u8], path: &Path) -> Result<[u8; KEY_BYTES], RefusalSet> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        refusal(
            RefusalKind::MalformedRecord,
            format!("the key at {} is not UTF-8", path.display()),
        )
    })?;
    let text = text.trim();
    let lowercase_hex = text.len() == KEY_BYTES * 2
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
    if !lowercase_hex {
        return Err(refusal(
            RefusalKind::MalformedRecord,
            format!(
                "the key at {} is not {} lowercase hex characters, which is the only spelling this \
                 build reads",
                path.display(),
                KEY_BYTES * 2
            ),
        ));
    }
    let mut key = [0u8; KEY_BYTES];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .expect("every character was checked to be lowercase hex");
    }
    Ok(key)
}

/// A new key from the OS entropy source, written with `create_new` so two processes starting at once
/// cannot each overwrite the other's key.
fn create(path: &Path) -> Result<[u8; KEY_BYTES], RefusalSet> {
    let mut key = [0u8; KEY_BYTES];
    getrandom::getrandom(&mut key).map_err(|error| {
        refusal(
            RefusalKind::NoSigningBackend,
            format!(
                "the OS entropy source refused ({error}), so no key can be made at {}",
                path.display()
            ),
        )
    })?;
    let text = format!("{}\n", hex(&key));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(text.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|error| {
                    refusal(
                        RefusalKind::NoSigningBackend,
                        format!(
                            "the key at {} could not be written ({error}); a key nobody can read \
                             back is not a key",
                            path.display()
                        ),
                    )
                })?;
            Ok(key)
        }
        // Another process made it between the read and the create: read theirs rather than replace
        // it — an overwritten identity is indistinguishable from a forged one.
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let bytes = std::fs::read(path).map_err(|error| {
                refusal(
                    RefusalKind::NoSigningBackend,
                    format!("the key at {} cannot be read ({error})", path.display()),
                )
            })?;
            parse(&bytes, path)
        }
        Err(error) => Err(refusal(
            RefusalKind::NoSigningBackend,
            format!(
                "the key at {} could not be created ({error})",
                path.display()
            ),
        )),
    }
}

fn absolute(path: &Path) -> Result<PathBuf, RefusalSet> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| {
            refusal(
                RefusalKind::NoReading,
                format!(
                    "a relative key path {} cannot be resolved because the working directory cannot \
                     be read ({error})",
                    path.display()
                ),
            )
        })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn refusal(kind: RefusalKind, detail: String) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(kind, detail);
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("voxel-axis-security-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("checkout")).expect("a scratch checkout");
        root
    }

    #[test]
    fn a_key_is_made_outside_the_checkout_and_reused_rather_than_replaced() {
        let root = scratch("reuse");
        let checkout = root.join("checkout");
        let path = root.join("keys/signing.key");
        let first = Security::open(&checkout, &path).expect("a new key");
        assert!(path.is_file(), "the key file is on disk");
        assert!(
            !path.starts_with(checkout.canonicalize().unwrap()),
            "the key is outside the checkout"
        );
        let written = std::fs::read(&path).expect("the key file");
        assert_eq!(
            written.len(),
            KEY_BYTES * 2 + 1,
            "64 hex characters and a newline"
        );
        let second = Security::open(&checkout, &path).expect("the same key");
        assert_eq!(first.key_id(), second.key_id());
        assert_eq!(
            std::fs::read(&path).expect("the key file"),
            written,
            "reopening a key does not rewrite it"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_key_inside_the_checkout_is_refused_and_the_refusal_names_both_paths() {
        let root = scratch("inside");
        let checkout = root.join("checkout");
        let refused = Security::open(&checkout, checkout.join("signing.key"))
            .expect_err("a key in the repository is in every clone");
        let detail = refused.all()[0].detail();
        assert_eq!(refused.all()[0].kind(), RefusalKind::UntrustedKey);
        assert!(detail.contains("signing.key"), "{detail}");
        assert!(detail.contains("inside the checkout"), "{detail}");
        assert!(
            !checkout.join("signing.key").exists(),
            "the refused path was not created either"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_that_is_not_a_key_is_refused_by_its_path() {
        let root = scratch("malformed");
        let path = root.join("keys/signing.key");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "this is not a key\n").unwrap();
        let refused = Security::open(&root.join("checkout"), &path).expect_err("not a key");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        assert!(refused.all()[0].detail().contains("signing.key"));
        // Uppercase hex is a different spelling of the same bytes, and is refused rather than
        // coerced: two files that spell one key differently would be two key names.
        std::fs::write(&path, "AA".repeat(KEY_BYTES)).unwrap();
        assert!(Security::open(&root.join("checkout"), &path).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_key_that_cannot_be_made_or_read_is_a_named_refusal_not_a_zero_key() {
        let root = scratch("missing");
        let checkout = root.join("checkout");
        // A file where the key's directory should be: nothing can be created under it.
        std::fs::write(root.join("blocked"), b"not a directory").unwrap();
        let refused =
            Security::open(&checkout, root.join("blocked/signing.key")).expect_err("no such dir");
        assert_eq!(refused.all()[0].kind(), RefusalKind::NoSigningBackend);
        assert!(
            refused.all()[0].detail().contains("blocked"),
            "{}",
            refused.all()[0].detail()
        );
        // A directory where the key file should be: it exists and cannot be read as a key.
        std::fs::create_dir_all(root.join("keys/signing.key")).unwrap();
        let refused =
            Security::open(&checkout, root.join("keys/signing.key")).expect_err("not readable");
        assert_eq!(refused.all()[0].kind(), RefusalKind::NoSigningBackend);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_signature_holds_only_for_these_bytes_this_key_and_this_algorithm() {
        let root = scratch("verify");
        let checkout = root.join("checkout");
        let mine = Security::open(&checkout, root.join("keys/mine.key")).expect("a key");
        let other = Security::open(&checkout, root.join("keys/other.key")).expect("another key");
        let signature = mine.sign(b"the package's bytes").expect("signed");

        mine.verify(b"the package's bytes", &signature)
            .expect("its own bytes");
        // Edited bytes after signing.
        let refused = mine
            .verify(b"the package's bytez", &signature)
            .expect_err("the bytes moved");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid));
        // A different key.
        assert!(other
            .verify(b"the package's bytes", &signature)
            .expect_err("not this key")
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::UntrustedKey));
        // An algorithm this build does not read — and it says so instead of calling it a forgery.
        let foreign = Signature::new(99, mine.key_id().clone(), signature.bytes().to_vec());
        assert!(mine
            .verify(b"the package's bytes", &foreign)
            .expect_err("unknown algorithm")
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::UnknownSignatureAlgorithm));
        // A signature that names the key and holds no bytes.
        let empty = Signature::new(HMAC_SHA256, mine.key_id().clone(), Vec::new());
        assert!(mine
            .verify(b"the package's bytes", &empty)
            .expect_err("no signature")
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::UnsignedEnvelope));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_key_file_is_written_inside_the_checkout_this_crate_is_built_from() {
        // **The repository's own state, as a test.** The key lives in a per-user directory outside
        // every checkout; this assertion fails if one is ever committed beside the sources.
        let workspace =
            voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
                .expect("this crate sits inside the workspace");
        for candidate in [
            "signing.key",
            "keys/signing.key",
            "crates/axis/security/signing.key",
        ] {
            assert!(
                !workspace.join(candidate).exists(),
                "{candidate} is inside the checkout"
            );
        }
    }
}
