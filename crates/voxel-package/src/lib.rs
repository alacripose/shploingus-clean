#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Content intake: **a package is admitted by validation, never by optimism.**
//!
//! # The claim this crate refuses to blur
//!
//! A content package arrives with two very different kinds of statement attached
//! to it, and the vocabulary of this crate keeps them apart on purpose:
//!
//! * **Integrity** — *these are the bytes the manifest describes.* A SHA-256
//!   digest establishes this, and this repository has exactly one hashing
//!   implementation (`voxel_model::Digest`, which is the only thing that names
//!   `sha2`).
//! * **Identity** — *these bytes came from who they say they came from.* A
//!   digest **cannot** establish this. Establishing it needs a signature, which
//!   needs asymmetric cryptography, which is a dependency this repository does
//!   not have.
//!
//! So intake returns [`Established`], which is not a boolean. A package whose
//! claim is [`Claim::ChecksumOnly`] is admitted **and reports that no identity
//! was proven** — because a `true` that means "the checksum matched" is how a
//! supply-chain guarantee gets believed and does not exist. The most this
//! repository can honestly offer is **pinning**: an operator states, out of
//! band, that a named signer's digest is the one they expect, and intake checks
//! the payload against that. That is a real guarantee, and it is a *different*
//! guarantee from a signature, so it has a different name.
//!
//! # Refuse, never default
//!
//! [`admit`] collects **every** problem rather than the first, so an author
//! fixes one round of them instead of one at a time. A package that fails
//! conformance is not partially admitted, and no requirement is silently
//! dropped: a package that names a shaper stage the world does not run, or a
//! material the registry has not admitted, is refused with that name in the
//! refusal.
//!
//! **The audit, 2026-10-10 — and this one is about the game, not the gate.** 32 189 lines: more than
//! the renderer, mesher, streamer, pool, appearance, light, audio, UI and shapers combined. That is
//! the expected size of an admissions gate. What was not expected is that **the game reads it**: the
//! client takes 88 references from this crate, and three of them are load-bearing for *play* rather
//! than for the gate — `recipe` (the shell's content format), `decision::read_report` (read in
//! `shell/mount.rs`), and `features::Establishment`. So a gate verdict showing up in the game's own
//! terminal is an **import, not a leak or an accident of wording**. Whether a game should read a
//! gate's prose has never been decided on the record, and neither has the client's dependency on this
//! crate at all: it has simply been true since the shell needed a content format. Either rule it, or
//! split `recipe` — a content format — out of the admissions gate it currently shares a crate with.

pub mod apidoc;
pub mod ballot;
pub mod bloat;
/// The ticket board's report, so the build gate needs no second dependency on `voxel-model`.
pub mod board {
    use voxel_model::chain::Chain;
    use voxel_model::ticket::Board;
    /// The board as the chain holds it, reported.
    pub fn describe(chain: &Chain) -> String {
        Board::from_chain(chain).describe()
    }
}
pub mod bridge;
pub mod census;
pub mod civilians;
pub mod compendium;
pub mod conformance;
pub mod compiler;
pub mod context;
pub mod decision;
pub mod features;
pub mod filing;
pub mod ingest;
pub mod migration;
pub mod numbers;
pub mod oracle;
pub mod patch;
pub mod policy;
pub mod projection;
pub mod provenance;
pub mod recipe;
pub mod record;
pub mod signing;
pub mod space;

use voxel_model::{Digest, MaterialId, MaterialRegistry, Refusal, RefusalKind, RefusalSet};
use voxel_shapers::{Declared, Stage};

/// What a publisher attaches to a package. Two variants, because "has a
/// checksum" and "is signed by somebody I trust" are not the same claim and
/// must not share a field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Claim {
    /// A digest over the payload. Establishes **integrity only**.
    ChecksumOnly,
    /// A named signer and the digest the signature covers. Whether that signer
    /// is trusted is [`Trust`]'s question, not the package's.
    Pinned { signer: String, digest: Digest },
}

/// What a package says it is and what it needs.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    /// Shaper stages the content's behaviour depends on. Every one must be in
    /// the world's declared set, or the content would be evaluated against a
    /// world it was not written for.
    pub requires_stages: Vec<Stage>,
    /// Materials the content names. Every one must be admitted, or the content
    /// would resolve a material to nothing.
    pub requires_materials: Vec<MaterialId>,
    /// The digest the publisher declares for the payload bytes.
    pub payload: Digest,
}

/// A manifest, the bytes it describes, and the claim attached to them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Package {
    pub manifest: Manifest,
    pub payload: Vec<u8>,
    pub claim: Claim,
}

/// What the **operator** has decided to trust, out of band.
///
/// A `Vec` rather than a map because pinning is a short list an operator reads,
/// not a lookup table that grows, and because [`Digest`] makes no ordering
/// promise this crate should depend on.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Trust {
    pins: Vec<(String, Digest)>,
}

impl Trust {
    /// Nothing is trusted. The correct starting state, and the reason a package
    /// that names a signer is refused by an empty [`Trust`] rather than waved
    /// through.
    pub fn none() -> Self {
        Trust { pins: Vec::new() }
    }

    pub fn pin(mut self, signer: impl Into<String>, digest: Digest) -> Self {
        let signer = signer.into();
        self.pins.retain(|(name, _)| name != &signer);
        self.pins.push((signer, digest));
        self
    }

    pub fn pinned_digest(&self, signer: &str) -> Option<Digest> {
        self.pins.iter().find(|(name, _)| name == signer).map(|(_, d)| *d)
    }

    pub fn len(&self) -> usize {
        self.pins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }
}

/// What was **actually established** about an admitted package.
///
/// Deliberately not a `bool`. `Established::ChecksumOnly` means the bytes are
/// the bytes — and that nobody's identity was proven. Code that wants to act on
/// provenance has to say which of the two it is acting on, and a caller that
/// only handles the pinned case cannot accidentally handle both.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Established {
    ChecksumOnly,
    Pinned { signer: String },
}

impl Established {
    /// Whether a **named identity** was proven. `false` for a checksum, and
    /// that `false` is the entire reason this type exists.
    pub fn establishes_identity(&self) -> bool {
        matches!(self, Established::Pinned { .. })
    }

    pub fn describe(&self) -> String {
        match self {
            Established::ChecksumOnly => {
                "the payload matches the manifest's digest; no signer identity was established"
                    .to_string()
            }
            Established::Pinned { signer } => format!(
                "the payload matches the digest pinned to signer {signer:?} by this operator"
            ),
        }
    }
}

/// One line of the intake report — what was checked, and what happened.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Line {
    pub item: String,
    pub disposition: Disposition,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Disposition {
    Admitted,
    Refused(String),
    /// Checked, and the answer is that this check cannot be made. Recorded
    /// rather than omitted, so a report cannot look complete when it is not.
    NotEstablished(String),
}

/// The whole of what intake did, per item.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Report {
    pub name: String,
    pub version: String,
    pub established: Established,
    pub lines: Vec<Line>,
}

impl Report {
    pub fn refused_lines(&self) -> Vec<&Line> {
        self.lines
            .iter()
            .filter(|l| matches!(l.disposition, Disposition::Refused(_)))
            .collect()
    }

    pub fn was_refused(&self) -> bool {
        !self.refused_lines().is_empty()
    }
}

/// A package that passed intake.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Admitted {
    pub package: Package,
    pub established: Established,
    pub report: Report,
}

/// The findings survive rejection as well as admission. Build and client readers
/// consume this same value; a rejected package never acquires an `Admitted` handle.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Intake {
    pub report: Report,
    pub result: Result<Admitted, RefusalSet>,
}

/// Validate a package against a world, and admit it or refuse it **with every
/// reason named at once**.
///
/// The checks are independent, so none short-circuits: a payload whose digest is
/// wrong *and* which requires a stage the world does not run reports both.
pub fn admit(
    package: Package,
    registry: &MaterialRegistry,
    declared: &Declared,
    trust: &Trust,
) -> Result<Admitted, RefusalSet> {
    inspect(package, registry, declared, trust).result
}

/// One validation pass, two readers: retain every finding even when the package
/// is refused. This is intake validation, not a model-oracle conformance verdict.
pub fn inspect(
    package: Package,
    registry: &MaterialRegistry,
    declared: &Declared,
    trust: &Trust,
) -> Intake {
    let mut set = manifest_structure(&package.manifest);
    let mut lines: Vec<Line> = set.all().iter().map(|r| Line {
        item: "manifest structure".to_string(),
        disposition: Disposition::Refused(r.detail().to_string()),
    }).collect();

    // ---- integrity: do the bytes match what the manifest describes? -------
    let measured = Digest::of(&package.payload);
    let integrity_ok = measured == package.manifest.payload;
    if integrity_ok {
        lines.push(Line {
            item: "payload digest".to_string(),
            disposition: Disposition::Admitted,
        });
    } else {
        let detail = format!(
            "the payload hashes to {} and the manifest declares {}",
            measured.to_hex(),
            package.manifest.payload.to_hex()
        );
        set.refuse(RefusalKind::MalformedIdentity, detail.clone());
        lines.push(Line {
            item: "payload digest".to_string(),
            disposition: Disposition::Refused(detail),
        });
    }

    // ---- identity: what, if anything, can be established? -----------------
    // A digest that does not match means the payload is not what the claim is
    // about, so the identity question is not answerable either way. Recording
    // that as `NotEstablished` rather than as a pass is the point.
    let established = if !integrity_ok {
        lines.push(Line {
            item: "signer identity".to_string(),
            disposition: Disposition::NotEstablished(
                "the payload is not the bytes the claim covers, so no identity question can be answered"
                    .to_string(),
            ),
        });
        Established::ChecksumOnly
    } else {
        match &package.claim {
            Claim::ChecksumOnly => {
                lines.push(Line {
                    item: "signer identity".to_string(),
                    disposition: Disposition::NotEstablished(
                        "the package carries a checksum and no signer; integrity is established and identity is not"
                            .to_string(),
                    ),
                });
                Established::ChecksumOnly
            }
            Claim::Pinned { signer, digest } => {
                let mut ok = true;
                match trust.pinned_digest(signer) {
                    None => {
                        let detail =
                            format!("the package names signer {signer:?}, whom this operator has not pinned");
                        set.refuse(RefusalKind::MalformedIdentity, detail.clone());
                        lines.push(Line {
                            item: format!("signer {signer:?}"),
                            disposition: Disposition::Refused(detail),
                        });
                        ok = false;
                    }
                    Some(pinned) => {
                        if pinned != *digest {
                            let detail = format!(
                                "the pin for {signer:?} is {} and the package claims {}",
                                pinned.to_hex(),
                                digest.to_hex()
                            );
                            set.refuse(RefusalKind::MalformedIdentity, detail.clone());
                            lines.push(Line {
                                item: format!("signer {signer:?}"),
                                disposition: Disposition::Refused(detail),
                            });
                            ok = false;
                        }
                        // The pin must cover the bytes in hand, not merely the
                        // bytes the package says it signs.
                        if pinned != package.manifest.payload {
                            let detail = format!(
                                "the pin for {signer:?} covers {} and this payload is {}",
                                pinned.to_hex(),
                                package.manifest.payload.to_hex()
                            );
                            set.refuse(RefusalKind::MalformedIdentity, detail.clone());
                            lines.push(Line {
                                item: format!("signer {signer:?} covers this payload"),
                                disposition: Disposition::Refused(detail),
                            });
                            ok = false;
                        }
                    }
                }
                if ok {
                    lines.push(Line {
                        item: format!("signer {signer:?}"),
                        disposition: Disposition::Admitted,
                    });
                    Established::Pinned { signer: signer.clone() }
                } else {
                    Established::ChecksumOnly
                }
            }
        }
    };

    // ---- conformance: the stages the content needs -----------------------
    for stage in &package.manifest.requires_stages {
        let item = format!("stage {stage:?}");
        match declared.require(*stage) {
            Ok(()) => lines.push(Line { item, disposition: Disposition::Admitted }),
            Err(refusal) => {
                let detail = refusal.detail().to_string();
                set.refuse(RefusalKind::MissingStage, detail.clone());
                lines.push(Line { item, disposition: Disposition::Refused(detail) });
            }
        }
    }

    // ---- conformance: the materials the content names --------------------
    for material in &package.manifest.requires_materials {
        let item = format!("material {}", material.0);
        match registry.material(*material) {
            Ok(m) => lines.push(Line {
                item: format!("material {} ({})", material.0, m.name),
                disposition: Disposition::Admitted,
            }),
            Err(refusal) => {
                let detail = refusal.detail().to_string();
                set.refuse(RefusalKind::UnadmittedMaterial, detail.clone());
                lines.push(Line { item, disposition: Disposition::Refused(detail) });
            }
        }
    }

    // ---- nothing is admitted while anything fails ------------------------
    let report = Report {
        name: package.manifest.name.clone(),
        version: package.manifest.version.clone(),
        established: established.clone(),
        lines,
    };
    let result = if set.is_empty() {
        Ok(Admitted { package, established, report: report.clone() })
    } else {
        Err(set)
    };
    Intake { report, result }
}

/// The refusal a package's own manifest can earn on inspection alone, without a
/// payload. Kept separate so a manifest can be checked before its bytes are
/// fetched — the cheap check first.
pub fn check_manifest(
    manifest: &Manifest,
    registry: &MaterialRegistry,
    declared: &Declared,
) -> Result<(), RefusalSet> {
    let mut set = manifest_structure(manifest);
    for stage in &manifest.requires_stages {
        if let Err(r) = declared.require(*stage) {
            set.push(r);
        }
    }
    for material in &manifest.requires_materials {
        if let Err(r) = registry.material(*material) {
            set.push(Refusal::owned(RefusalKind::UnadmittedMaterial, r.detail()));
        }
    }
    set.into_result(())
}

fn manifest_structure(manifest: &Manifest) -> RefusalSet {
    let mut set = RefusalSet::new();
    if manifest.name.trim().is_empty() {
        set.refuse(RefusalKind::MalformedIdentity, "the package has no name");
    }
    if manifest.version.trim().is_empty() {
        set.refuse(
            RefusalKind::MalformedIdentity,
            format!("package {:?} has no version", manifest.name),
        );
    }
    let mut stages: Vec<Stage> = manifest.requires_stages.clone();
    stages.sort_unstable();
    stages.dedup();
    if stages.len() != manifest.requires_stages.len() {
        set.refuse(
            RefusalKind::MalformedIdentity,
            format!("package {:?} lists a stage twice", manifest.name),
        );
    }
    let mut materials = manifest.requires_materials.clone();
    materials.sort_unstable_by_key(|material| material.0);
    materials.dedup();
    if materials.len() != manifest.requires_materials.len() {
        set.refuse(
            RefusalKind::MalformedIdentity,
            format!("package {:?} lists a material twice", manifest.name),
        );
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::Stage;

    fn registry() -> MaterialRegistry {
        voxel_shapers::materials::registry().expect("the admitted registry builds")
    }

    fn manifest_for(payload: &[u8], name: &str) -> Manifest {
        Manifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            requires_stages: vec![Stage::Terrain, Stage::Geology],
            requires_materials: vec![MaterialId(3), MaterialId(7)],
            payload: Digest::of(payload),
        }
    }

    fn package(name: &str) -> Package {
        let payload = b"a content package".to_vec();
        Package {
            manifest: manifest_for(&payload, name),
            payload,
            claim: Claim::ChecksumOnly,
        }
    }

    #[test]
    fn rejected_intake_preserves_findings_for_the_client_without_admitting_bytes() {
        let mut p = package("broken");
        p.payload.push(b'!');
        p.manifest.requires_materials.push(MaterialId(9999));
        let intake = inspect(p, &registry(), &Declared::full(), &Trust::none());
        assert_eq!(intake.result.unwrap_err().len(), 2);
        assert_eq!(intake.report.refused_lines().len(), 2);
        assert!(intake.report.lines.iter().any(|line| matches!(
            line.disposition, Disposition::NotEstablished(_)
        )));
        assert!(intake.report.lines.iter().any(|line| matches!(
            line.disposition, Disposition::Admitted
        )));
    }

    #[test]
    fn payload_admission_cannot_bypass_manifest_structure_validation() {
        let mut p = package(" ");
        p.manifest.version.clear();
        p.manifest.requires_stages.push(Stage::Terrain);
        p.manifest.requires_materials.push(MaterialId(3));
        let before = check_manifest(&p.manifest, &registry(), &Declared::full()).unwrap_err();
        let intake = inspect(p, &registry(), &Declared::full(), &Trust::none());
        assert_eq!(before.len(), 4);
        assert_eq!(intake.result.unwrap_err(), before);
        assert_eq!(intake.report.refused_lines().len(), 4);
    }

    #[test]
    fn build_and_client_readers_share_the_exact_admitted_report() {
        let intake = inspect(package("arrangement"), &registry(), &Declared::full(), &Trust::none());
        assert_eq!(intake.report, intake.result.unwrap().report);
    }

    #[test]
    fn a_sound_package_is_admitted_and_every_line_says_so() {
        let p = package("rock-pack");
        let admitted = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap();
        assert!(admitted.report.refused_lines().is_empty());
        assert!(!admitted.report.was_refused());
        // digest + identity + 2 stages + 2 materials
        assert_eq!(admitted.report.lines.len(), 6, "{:?}", admitted.report.lines);
    }

    #[test]
    fn a_checksum_is_admitted_and_does_not_claim_to_prove_identity() {
        // The whole reason `Established` is not a bool.
        let p = package("rock-pack");
        let admitted = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap();
        assert_eq!(admitted.established, Established::ChecksumOnly);
        assert!(!admitted.established.establishes_identity());
        assert!(admitted.established.describe().contains("no signer identity"));
        // And the report records the check as *not established*, not as absent.
        let identity = admitted
            .report
            .lines
            .iter()
            .find(|l| l.item == "signer identity")
            .expect("identity is a line in the report");
        assert!(matches!(identity.disposition, Disposition::NotEstablished(_)));
    }

    #[test]
    fn a_payload_that_changed_after_the_manifest_was_written_is_refused() {
        let mut p = package("rock-pack");
        p.payload.push(b'!');
        let err = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 1, "{err}");
        assert_eq!(err.all()[0].kind(), RefusalKind::MalformedIdentity);
    }

    #[test]
    fn a_named_signer_nobody_pinned_is_refused_rather_than_assumed() {
        let mut p = package("rock-pack");
        p.claim = Claim::Pinned {
            signer: "someone".to_string(),
            digest: p.manifest.payload,
        };
        let err = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err.all()[0].detail().contains("has not pinned"));
    }

    #[test]
    fn a_correctly_pinned_signer_is_the_one_case_that_proves_identity() {
        let mut p = package("rock-pack");
        let digest = p.manifest.payload;
        p.claim = Claim::Pinned { signer: "studio".to_string(), digest };
        let trust = Trust::none().pin("studio", digest);
        let admitted = admit(p, &registry(), &Declared::full(), &trust).unwrap();
        assert_eq!(admitted.established, Established::Pinned { signer: "studio".into() });
        assert!(admitted.established.establishes_identity());
    }

    #[test]
    fn a_pin_for_a_different_payload_is_refused_even_when_the_claim_matches_itself() {
        // A package can consistently claim its own bytes and still not be the
        // release the operator pinned. Both checks are needed.
        let mut p = package("rock-pack");
        let self_consistent = p.manifest.payload;
        p.claim = Claim::Pinned { signer: "studio".to_string(), digest: self_consistent };
        let other = Digest::of(b"a different release entirely");
        let trust = Trust::none().pin("studio", other);
        let err = admit(p, &registry(), &Declared::full(), &trust).unwrap_err();
        assert_eq!(err.len(), 2, "both the pin and the coverage must complain: {err}");
    }

    #[test]
    fn a_stage_the_world_does_not_run_is_refused_by_name() {
        let mut p = package("rock-pack");
        p.manifest.requires_stages = vec![Stage::Terrain, Stage::Weather];
        // A world that runs terrain but not weather.
        let declared = Declared::closure_of([Stage::Terrain]);
        let err = admit(p, &registry(), &declared, &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 1, "{err}");
        assert_eq!(err.all()[0].kind(), RefusalKind::MissingStage);
        assert!(err.all()[0].detail().contains("Weather"), "{}", err.all()[0].detail());
    }

    #[test]
    fn a_material_the_registry_has_not_admitted_is_refused() {
        let mut p = package("rock-pack");
        p.manifest.requires_materials = vec![MaterialId(3), MaterialId(9999)];
        let err = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 1, "{err}");
        assert_eq!(err.all()[0].kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn every_problem_is_collected_rather_than_the_first_one() {
        // Three independent failures, in three different checks.
        let mut p = package("rock-pack");
        p.payload.push(b'?');
        p.manifest.requires_stages = vec![Stage::Weather];
        p.manifest.requires_materials = vec![MaterialId(9999)];
        let declared = Declared::closure_of([Stage::Terrain]);
        let err = admit(p, &registry(), &declared, &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 3, "expected three named refusals, got: {err}");
        let kinds: Vec<RefusalKind> = err.all().iter().map(|r| r.kind()).collect();
        assert!(kinds.contains(&RefusalKind::MalformedIdentity));
        assert!(kinds.contains(&RefusalKind::MissingStage));
        assert!(kinds.contains(&RefusalKind::UnadmittedMaterial));
    }

    #[test]
    fn a_refused_package_is_not_partially_admitted() {
        // One bad material among otherwise sound requirements refuses the whole
        // package. There is no `Admitted` to reach a report through, so a
        // caller cannot mistake a rejected package for an accepted one with
        // warnings.
        let mut p = package("rock-pack");
        let sound_items = p.manifest.requires_stages.len() + 1; // stages + the good material
        p.manifest.requires_materials = vec![MaterialId(3), MaterialId(9999)];
        let err = admit(p, &registry(), &Declared::full(), &Trust::none()).unwrap_err();
        assert_eq!(err.len(), 1, "only the bad material is refused: {err}");
        assert_eq!(err.all()[0].kind(), RefusalKind::UnadmittedMaterial);
        assert!(sound_items > 0, "the sound items that were still not enough to admit it");
    }

    #[test]
    fn intake_is_deterministic() {
        let a = admit(package("rock-pack"), &registry(), &Declared::full(), &Trust::none()).unwrap();
        let b = admit(package("rock-pack"), &registry(), &Declared::full(), &Trust::none()).unwrap();
        assert_eq!(a.report, b.report);
        assert_eq!(a.established, b.established);
    }

    #[test]
    fn an_empty_trust_list_refuses_every_named_signer() {
        assert!(Trust::none().is_empty());
        assert_eq!(Trust::none().pinned_digest("anyone"), None);
        let trust = Trust::none().pin("a", Digest::of(b"a"));
        assert_eq!(trust.len(), 1);
        // Pinning the same name again replaces rather than accumulates.
        let trust = trust.pin("a", Digest::of(b"b"));
        assert_eq!(trust.len(), 1);
        assert_eq!(trust.pinned_digest("a"), Some(Digest::of(b"b")));
    }

    #[test]
    fn a_manifest_can_be_checked_before_its_bytes_are_fetched() {
        let reg = registry();
        let declared = Declared::full();
        let good = manifest_for(b"anything", "rock-pack");
        assert!(check_manifest(&good, &reg, &declared).is_ok());

        let mut nameless = manifest_for(b"anything", "rock-pack");
        nameless.name = "  ".to_string();
        assert!(check_manifest(&nameless, &reg, &declared).is_err());

        let mut twice = manifest_for(b"anything", "rock-pack");
        twice.requires_stages = vec![Stage::Terrain, Stage::Terrain];
        let err = check_manifest(&twice, &reg, &declared).unwrap_err();
        assert!(err.all()[0].detail().contains("twice"));

        let mut unknown_material = manifest_for(b"anything", "rock-pack");
        unknown_material.requires_materials = vec![MaterialId(4242)];
        let err = check_manifest(&unknown_material, &reg, &declared).unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::UnadmittedMaterial);
    }
}
