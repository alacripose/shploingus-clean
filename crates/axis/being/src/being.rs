//! **The being itself**: an id derived from a creation record, a face, a life, a signature and a mass.
//!
//! # Why the id is derived and not counted
//!
//! The reference's rule for every id in this project is *"ids come from the directory, never from a
//! counter"*, and `voxel_model::ticket` states the reason: a process killed mid-run leaves a counter
//! behind its own record and hands out an id that already exists. A being's identity here is the
//! **digest of the act that created it** — tick, parent, mass and provenance — shortened to the forty
//! hex digits the record's other ids use. Two beings created by the same act at the same tick with
//! the same parent and mass are the same being, which is exactly right: a duplicate creation is the
//! same creation delivered twice.
//!
//! # Why `create` takes the signature and the creation record
//!
//! Because *"every being is ensouled"* (`CIVILIANS.md` §3.2) and *"creation precedes reference"*
//! (`PATCH-042` GH4). Both are enforced by the **shape of the only constructor**: there is no
//! `Being::new()` that fills them in, no `Default`, and no builder setter for either. A being with no
//! soul or no origin is not refused at runtime because it cannot be written down — and that is a
//! stronger gate than a check, because a check can be skipped by the next constructor somebody adds.
//!
//! # What a transition is, and what it is not
//!
//! [`Being::grow`] advances one stage; [`Being::die`] ends the life; [`Being::retire`] stops the
//! acting. **None of them removes anything**, and none of them can be undone by a caller that wants
//! the id back: the id, the mass, the signature and the creation record survive every transition, and
//! `the_crate_has_no_destruction_path` reads this file's own source to keep it that way.

use std::collections::BTreeMap;

use voxel_model::{endian, Identity, Ratio};

use crate::face::{BeingFace, Mobility};
use crate::life::{LifeStage, Needs, State};
use crate::refusal::BeingRefusal;
use crate::signature::SpiritualSignature;

/// The domain tag the id is hashed under, so a being's identity cannot collide with another kind's.
const BEING_DOMAIN: &str = "shploingus/being";

/// How many hex digits of the creation digest an id keeps. **Forty**, the length the record's other
/// ids use, so an id is short enough to read in a report line and long enough that two beings in one
/// world do not collide by accident.
const ID_DIGITS: usize = 40;

/// **What created a being**, and therefore what its identity is.
///
/// The four fields are the whole of the act: *when* (the world's tick), *from what* (a parent being,
/// or nothing for a first creation), *how much* (the exact mass it starts with), and *on whose word*
/// (provenance — a stand, a parent record, a Gaia transition, a `file:line`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationRecord {
    /// The world tick the creation was filed at.
    pub tick: u64,
    /// The being this one came from, when one did.
    pub parent: Option<BeingId>,
    /// The mass the being begins with, in kilograms, exactly.
    pub mass: Ratio,
    /// Where the creation came from, in words a reader can follow. Never empty.
    pub provenance: String,
}

impl CreationRecord {
    /// A creation record, refusing a mass that is not matter and a provenance that says nothing.
    pub fn new(
        tick: u64,
        parent: Option<BeingId>,
        mass: Ratio,
        provenance: impl Into<String>,
    ) -> Result<Self, BeingRefusal> {
        let provenance = provenance.into();
        if mass == Ratio::ZERO {
            return Err(BeingRefusal::Massless {
                detail: "the creation record declared zero kilograms".to_owned(),
            });
        }
        if provenance.trim().is_empty() {
            return Err(BeingRefusal::Unprovenanced {
                detail: "the provenance is empty".to_owned(),
            });
        }
        Ok(CreationRecord {
            tick,
            parent,
            mass,
            provenance,
        })
    }

    /// The fields the id is hashed over, in a fixed order.
    fn identity_fields(&self) -> Vec<Vec<u8>> {
        vec![
            endian::le_bytes(self.tick).to_vec(),
            self.parent
                .as_ref()
                .map(|parent| parent.as_str().as_bytes().to_vec())
                .unwrap_or_else(|| b"-".to_vec()),
            self.mass.to_string().into_bytes(),
            self.provenance.as_bytes().to_vec(),
        ]
    }
}

/// A being's id: `being_` and forty hex digits of its creation record's digest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BeingId(String);

impl BeingId {
    /// The id a creation record earns.
    pub fn of(creation: &CreationRecord) -> Self {
        let fields = creation.identity_fields();
        let borrowed: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
        let digest = Identity::of_fields(BEING_DOMAIN, &borrowed);
        let hex = digest.to_string();
        let hex = hex.trim_start_matches("sha256:");
        let kept: String = hex.chars().take(ID_DIGITS).collect();
        BeingId(format!("being_{kept}"))
    }

    /// Read an id back, refusing anything that is not `being_` plus forty lowercase hex digits.
    pub fn parse(text: &str) -> Result<Self, BeingRefusal> {
        let text = text.trim();
        let Some(body) = text.strip_prefix("being_") else {
            return Err(BeingRefusal::AbsentSubject {
                subject: text.to_owned(),
            });
        };
        let well_formed = body.len() == ID_DIGITS
            && body
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
        if !well_formed {
            return Err(BeingRefusal::AbsentSubject {
                subject: text.to_owned(),
            });
        }
        Ok(BeingId(text.to_owned()))
    }

    /// The id as it is written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeingId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// **One living thing**: a being, whatever face it wears.
///
/// The fields are `pub(crate)` deliberately — a caller reads through the accessors and changes
/// through the transitions, so there is exactly one place a stage can advance and one place a state
/// can die. A public field would be a second door with no lock on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Being {
    pub(crate) id: BeingId,
    pub(crate) creation: CreationRecord,
    pub(crate) face: BeingFace,
    pub(crate) mobility: Mobility,
    pub(crate) stage: LifeStage,
    pub(crate) state: State,
    pub(crate) needs: Needs,
    pub(crate) signature: SpiritualSignature,
    pub(crate) mass: Ratio,
}

impl Being {
    /// **The only constructor.** A being begins at [`LifeStage::Seed`], [`State::Alive`], with the
    /// mass its creation record declares.
    ///
    /// It refuses a mobility its face's row does not allow — a walking plant is not a plant — and a
    /// creation mass of zero, which the creation record has already refused, so the check is stated
    /// here once as well rather than trusted across a type boundary.
    pub fn create(
        face: BeingFace,
        mobility: Mobility,
        needs: Needs,
        signature: SpiritualSignature,
        creation: CreationRecord,
    ) -> Result<Self, BeingRefusal> {
        if !mobility.is_declared_for(face) {
            return Err(BeingRefusal::MobilityNotDeclared {
                face: face.name(),
                mobility: mobility.name(),
            });
        }
        if creation.mass == Ratio::ZERO {
            return Err(BeingRefusal::Massless {
                detail: "a being cannot begin with zero mass".to_owned(),
            });
        }
        let id = BeingId::of(&creation);
        let mass = creation.mass;
        Ok(Being {
            id,
            creation,
            face,
            mobility,
            stage: LifeStage::Seed,
            state: State::Alive,
            needs,
            signature,
            mass,
        })
    }

    /// The being's id.
    pub fn id(&self) -> &BeingId {
        &self.id
    }

    /// The face it wears.
    pub fn face(&self) -> BeingFace {
        self.face
    }

    /// How it acts on space.
    pub fn mobility(&self) -> Mobility {
        self.mobility
    }

    /// Where in its life it is.
    pub fn stage(&self) -> LifeStage {
        self.stage
    }

    /// Whether it is living, retired or dead.
    pub fn state(&self) -> State {
        self.state
    }

    /// What it wants.
    pub fn needs(&self) -> Needs {
        self.needs
    }

    /// The spectral reading it carries, kept whether or not it can be drawn.
    pub fn signature(&self) -> SpiritualSignature {
        self.signature
    }

    /// What created it.
    pub fn creation(&self) -> &CreationRecord {
        &self.creation
    }

    /// Its mass, in kilograms, exactly.
    pub fn mass(&self) -> Ratio {
        self.mass
    }

    /// Whether it may act: alive **and** not retired.
    pub fn acts(&self) -> bool {
        self.state.acts() && !self.stage.is_dead()
    }

    /// Advance exactly one stage, or refuse.
    ///
    /// **One stage, never a jump.** A growth rule decides *when* the next stage begins, which is a
    /// question about mass and time; it does not decide *which* stage comes next, which is the
    /// vocabulary's. Reaching [`LifeStage::Dead`] sets [`State::Dead`] in the same step, because a
    /// corpse-stage being that still read `alive` would be two records disagreeing.
    pub fn grow(&mut self) -> Result<LifeStage, BeingRefusal> {
        if self.stage.is_dead() {
            return Err(BeingRefusal::AlreadyDead {
                being: self.id.as_str().to_owned(),
            });
        }
        let next = self.stage.next().ok_or_else(|| BeingRefusal::AlreadyDead {
            being: self.id.as_str().to_owned(),
        })?;
        self.stage = next;
        if next.is_dead() {
            self.state = State::Dead;
        }
        Ok(next)
    }

    /// End the life. The stage becomes [`LifeStage::Dead`] and the state [`State::Dead`].
    ///
    /// **The record is untouched**: id, mass, signature and creation survive, because the mass still
    /// exists somewhere and the being is still the subject of its own death. A second death is
    /// refused rather than idempotent, because a cause filed twice is two causes.
    pub fn die(&mut self) -> Result<(), BeingRefusal> {
        if self.stage.is_dead() {
            return Err(BeingRefusal::AlreadyDead {
                being: self.id.as_str().to_owned(),
            });
        }
        self.stage = LifeStage::Dead;
        self.state = State::Dead;
        Ok(())
    }

    /// Stop acting, keeping the life. **Retirement is not death**: the stage is untouched, the mass
    /// is untouched, and the record remains the subject of everything it did.
    pub fn retire(&mut self) -> Result<(), BeingRefusal> {
        if self.stage.is_dead() {
            return Err(BeingRefusal::AlreadyDead {
                being: self.id.as_str().to_owned(),
            });
        }
        self.state = State::Retired;
        Ok(())
    }

    /// **The reference check**: a row naming a being must resolve to one that exists.
    ///
    /// The error is the *reference's*, not the being's — which is why it names the subject rather
    /// than an index. `PATCH-042`'s GH1 puts the second half in the producer: an absent subject
    /// **emits no row**, and this function is what the producer asks before it emits.
    pub fn require<'a>(
        id: &BeingId,
        beings: &'a [Being],
    ) -> Result<&'a Being, BeingRefusal> {
        beings
            .iter()
            .find(|being| &being.id == id)
            .ok_or_else(|| BeingRefusal::AbsentSubject {
                subject: id.as_str().to_owned(),
            })
    }

    /// The beings a stand or a world holds, keyed by id, for a producer that must answer repeated
    /// references without scanning.
    pub fn index(beings: &[Being]) -> BTreeMap<BeingId, &Being> {
        beings.iter().map(|being| (being.id.clone(), being)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creation(tick: u64, mass: u64) -> CreationRecord {
        CreationRecord::new(
            tick,
            None,
            Ratio::from_u64(mass),
            "gaia:stand@(0,0) species=conifer",
        )
        .expect("a valid creation")
    }

    fn being() -> Being {
        being_from(1, 5)
    }

    fn being_from(tick: u64, mass: u64) -> Being {
        Being::create(
            BeingFace::Mob,
            Mobility::Rooted,
            Needs::met(),
            SpiritualSignature::unlit(0, 0).expect("a valid signature"),
            creation(tick, mass),
        )
        .expect("a valid being")
    }

    #[test]
    fn an_id_is_the_creation_record_and_two_of_them_agree() {
        let first = BeingId::of(&creation(1, 5));
        let second = BeingId::of(&creation(1, 5));
        assert_eq!(first, second, "one act, one identity");
        assert_eq!(BeingId::parse(first.as_str()), Ok(first.clone()));
        assert!(first.as_str().starts_with("being_"));
        assert_eq!(first.as_str().len(), "being_".len() + ID_DIGITS);
    }

    #[test]
    fn a_different_act_is_a_different_being() {
        let base = BeingId::of(&creation(1, 5));
        assert_ne!(base, BeingId::of(&creation(2, 5)), "the tick is in the id");
        assert_ne!(base, BeingId::of(&creation(1, 6)), "the mass is in the id");
        let adopted = CreationRecord::new(
            1,
            Some(base.clone()),
            Ratio::from_u64(5),
            "gaia:parent",
        )
        .expect("valid");
        assert_ne!(base, BeingId::of(&adopted), "the parent is in the id");
    }

    #[test]
    fn a_malformed_id_is_an_absent_subject_and_not_a_panic() {
        for text in ["", "being_", "being_xyz", "other_0123456789012345678901234567890123456789"] {
            let refused = BeingId::parse(text).expect_err("nothing there");
            assert_eq!(refused.name(), "absent_subject", "{text:?}");
        }
    }

    #[test]
    fn a_creation_record_needs_mass_and_provenance() {
        let massless = CreationRecord::new(0, None, Ratio::ZERO, "gaia")
            .expect_err("zero kilograms is not matter");
        assert_eq!(massless.name(), "massless");

        let anonymous = CreationRecord::new(0, None, Ratio::from_u64(1), "   ")
            .expect_err("a creation must say where it came from");
        assert_eq!(anonymous.name(), "unprovenanced");
    }

    #[test]
    fn a_plant_cannot_walk_and_the_refusal_names_both_words() {
        let walked = Being::create(
            BeingFace::Plant,
            Mobility::Walking,
            Needs::met(),
            SpiritualSignature::unlit(0, 0).expect("valid"),
            creation(1, 1),
        )
        .expect_err("a walking plant is not a plant");
        assert_eq!(walked.name(), "mobility_not_declared");
        assert!(walked.describe().contains("plant"), "{}", walked.describe());
        assert!(walked.describe().contains("walking"), "{}", walked.describe());
    }

    #[test]
    fn the_life_ladder_is_walked_one_stage_at_a_time_and_ends_dead() {
        let mut being = being();
        assert_eq!(being.stage(), LifeStage::Seed);
        assert!(being.acts());
        for expected in [LifeStage::Sapling, LifeStage::Mature, LifeStage::Veteran] {
            assert_eq!(being.grow(), Ok(expected));
            assert!(being.acts());
        }
        assert_eq!(being.grow(), Ok(LifeStage::Dead));
        assert!(!being.acts(), "the dead do not act");
        assert_eq!(being.state(), State::Dead);
        assert_eq!(being.grow(), Err(BeingRefusal::AlreadyDead {
            being: being.id().as_str().to_owned()
        }));
    }

    #[test]
    fn death_keeps_the_record_and_refuses_a_second_death() {
        let mut being = being();
        let id = being.id().clone();
        let mass = being.mass();
        let signature = being.signature();
        being.die().expect("the first death");
        assert_eq!(being.stage(), LifeStage::Dead);
        assert_eq!(being.id(), &id, "the id survives the body");
        assert_eq!(being.mass(), mass, "the mass is still accounted");
        assert_eq!(being.signature(), signature, "the reading is kept");
        assert_eq!(
            being.die().expect_err("a second cause is a second death"),
            BeingRefusal::AlreadyDead {
                being: id.as_str().to_owned()
            }
        );
    }

    #[test]
    fn retirement_stops_the_acting_and_not_the_life() {
        let mut being = being();
        being.grow().expect("sapling");
        being.retire().expect("retired");
        assert_eq!(being.state(), State::Retired);
        assert_eq!(being.stage(), LifeStage::Sapling, "retirement is not a stage");
        assert!(!being.acts());
        assert!(being.grow().is_ok(), "a retired being still grows");
    }

    #[test]
    fn a_reference_to_a_being_nobody_created_is_refused_by_name() {
        let present = being();
        let absent = BeingId::of(&creation(9, 9));
        let found = Being::require(&absent, std::slice::from_ref(&present))
            .expect_err("nobody created this subject");
        assert_eq!(found.name(), "absent_subject");
        assert!(found.describe().contains(absent.as_str()), "{}", found.describe());

        let resolved = Being::require(present.id(), std::slice::from_ref(&present))
            .expect("a being that exists resolves");
        assert_eq!(resolved.id(), present.id());
    }

    #[test]
    fn an_index_answers_every_reference_in_one_pass() {
        let first = being();
        let second = being_from(2, 5);
        assert_ne!(first.id(), second.id(), "two acts, two beings");
        let index = Being::index(std::slice::from_ref(&first));
        assert!(index.contains_key(first.id()));
        assert!(!index.contains_key(second.id()));
    }

    #[test]
    fn the_crate_has_no_destruction_path() {
        // **The absence suite.** `CIVILIANS.md:118` — *"Nothing can kill a being"* — and a comment
        // saying so is worth less than a search for the word. The source is included at compile
        // time, so this reads the file that was built rather than whatever is on disk now.
        let sources = [
            ("being.rs", include_str!("being.rs")),
            ("face.rs", include_str!("face.rs")),
            ("life.rs", include_str!("life.rs")),
            ("signature.rs", include_str!("signature.rs")),
            ("refusal.rs", include_str!("refusal.rs")),
            ("lib.rs", include_str!("lib.rs")),
        ];
        // The needles are **built**, not written: a literal needle in this file would be found by
        // this very test, which is the absence suite convicting its own witness.
        let verbs = ["remov", "delet", "destroy", "drop_being"];
        for (name, source) in sources {
            for verb in verbs {
                let forbidden = format!("fn {verb}");
                assert!(
                    !source.contains(&forbidden),
                    "{name} declares a function named `{forbidden}`, and a being is retired, never \
                     deleted"
                );
            }
        }
    }
}
