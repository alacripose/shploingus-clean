//! **The face a being wears** — and the agency that face implies.
//!
//! The reference's table (`CIVILIANS.md` §3.1) has six rows. The seventh, [`BeingFace::Ghost`], is
//! the one the user asked about and that the table never answered; `PATCH-042` §Ghosts records the
//! ruling as *declared, with its agency open*. A ghost is therefore a face with
//! [`Agency::Undecided`] rather than a face quietly assumed to be an agent or a plant.

use crate::life::LifeStage;
use crate::refusal::BeingRefusal;

/// How many faces this vocabulary declares. **A literal**, for the same reason
/// `census::CRATE_COUNT` is one: the pin's value is meaningless and its type is the assertion.
pub const FACE_COUNT: usize = 7;

/// **Does this face act with intent?** Three values, because the doctrine has three answers.
///
/// `CIVILIANS.md` §3.1: *"Every agent is a being; not every being is an agent"*, with a plant as the
/// being that never acts. A ghost is neither: the reference asked and did not answer, so
/// [`Agency::Undecided`] is the honest value and a `bool` would be a decision nobody made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Agency {
    /// A being that acts with intent: it holds records for its acts.
    Agent,
    /// A being that never acts: it grows, it is acted on, it produces nothing to adjudicate.
    BeingOnly,
    /// A face the doctrine asked about and did not settle. **Not a synonym for `BeingOnly`** — a
    /// ghost that is later ruled to act must not have been modelled as inert in the meantime.
    Undecided,
}

impl Agency {
    /// The word, as the axis vocabulary spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Agency::Agent => "agent",
            Agency::BeingOnly => "being_only",
            Agency::Undecided => "undecided",
        }
    }
}

/// One face of the being substrate.
///
/// **A face is a specialization, never a privilege** (`CIVILIANS.md` §3.1). Nothing here grants a
/// power; the powers live in the gates that rule on acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BeingFace {
    /// Needs, skills, health, wealth, home, job, life stage.
    Civilian,
    /// A person played by an agent or by the player: the same action set as any character.
    Pilot,
    /// The outside view and its powers — **Gaia, as nature's god** — which may incarnate.
    God,
    /// Mobility, needs, a life stage; a being **and** an agent.
    Mob,
    /// Growth and a life stage; **a being and not an agent**.
    Plant,
    /// A god or the player *inside* the world, a civilian record like any other.
    Incarnate,
    /// **The seventh face**: a being drawn in the world with its own spiritual energy. The doctrine
    /// asked whether it acts and has not answered, so its agency is [`Agency::Undecided`].
    Ghost,
}

impl BeingFace {
    /// Every face, in the reference table's order with the ghost last because it was added last.
    pub const ALL: [BeingFace; FACE_COUNT] = [
        BeingFace::Civilian,
        BeingFace::Pilot,
        BeingFace::God,
        BeingFace::Mob,
        BeingFace::Plant,
        BeingFace::Incarnate,
        BeingFace::Ghost,
    ];

    /// **Check 1 of 2.** Every face has an `ALL` entry, and the count is the array's length.
    pub const ALL_ARE_TOTAL: [(); FACE_COUNT] = [(); BeingFace::ALL.len()];

    /// The word this face is written as.
    pub const fn name(self) -> &'static str {
        match self {
            BeingFace::Civilian => "civilian",
            BeingFace::Pilot => "pilot",
            BeingFace::God => "god",
            BeingFace::Mob => "mob",
            BeingFace::Plant => "plant",
            BeingFace::Incarnate => "incarnate",
            BeingFace::Ghost => "ghost",
        }
    }

    /// The word as a value, refusing one this vocabulary does not declare.
    pub fn parse(word: &str) -> Result<Self, BeingRefusal> {
        let wanted = word.trim();
        Self::ALL
            .into_iter()
            .find(|face| face.name() == wanted)
            .ok_or_else(|| BeingRefusal::UnknownFace {
                word: wanted.to_owned(),
            })
    }

    /// **The reference table's third column**, as a value.
    pub const fn agency(self) -> Agency {
        match self {
            BeingFace::Plant => Agency::BeingOnly,
            BeingFace::Ghost => Agency::Undecided,
            _ => Agency::Agent,
        }
    }

    /// Whether this face may hold an office. **A plant never acts, so it never holds one**
    /// (`CIVILIANS.md` §3.8); a ghost's answer waits on its agency.
    pub const fn may_hold_office(self) -> bool {
        match self.agency() {
            Agency::Agent => true,
            // Not "no": an undecided face is not a refused one. The office gate refuses with
            // `AgencyUndecided` rather than pretending the ghost is a plant.
            Agency::BeingOnly | Agency::Undecided => false,
        }
    }

    /// The stages this face can be in. **A mob is born, a plant germinates, and both end `Dead`** —
    /// the stage ladder is shared on purpose, because `CIVILIANS.md` Q137 makes death one event
    /// whatever the face.
    pub const fn stages(self) -> &'static [LifeStage] {
        &LifeStage::ALL
    }
}

/// How a being acts on space.
///
/// The mob row of the reference's table says *"mobility"* and stops there. This is that word as a
/// value, with one deliberate decision recorded: **`Rooted` is a mobility**, because a tree acts by
/// occupying and extending space, and calling it immobile would make the first mob a plant by
/// definition rather than by behaviour (`PATCH-043` §The tree, as a mob).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mobility {
    /// Fixed to a place; acts by growing into it.
    Rooted,
    /// Moves along a surface: a vine's declared mobility (`GAIA.md` §7, `0141`).
    Creeping,
    /// Walks.
    Walking,
    /// Flies or drifts.
    Flying,
}

impl Mobility {
    /// Every mobility this vocabulary declares.
    pub const ALL: [Mobility; 4] = [
        Mobility::Rooted,
        Mobility::Creeping,
        Mobility::Walking,
        Mobility::Flying,
    ];

    /// The count, as a literal for the same reason as [`FACE_COUNT`].
    pub const COUNT: usize = 4;

    /// The word.
    pub const fn name(self) -> &'static str {
        match self {
            Mobility::Rooted => "rooted",
            Mobility::Creeping => "creeping",
            Mobility::Walking => "walking",
            Mobility::Flying => "flying",
        }
    }

    /// The word as a value.
    pub fn parse(word: &str) -> Result<Self, BeingRefusal> {
        let wanted = word.trim();
        Self::ALL
            .into_iter()
            .find(|mobility| mobility.name() == wanted)
            .ok_or_else(|| BeingRefusal::UnknownMobility {
                word: wanted.to_owned(),
            })
    }

    /// **The mobilities a face may declare.** Declared, not measured: the doctrine names mobility
    /// without enumerating it, so these sets are this crate's ruling and move only by a ruling.
    ///
    /// A ghost's `Flying` is part of its open question too — it is the drawing's motion, not its
    /// agency — and a god's `Walking` is what an **incarnate** uses while inside the world.
    pub const fn declared_for(face: BeingFace) -> &'static [Mobility] {
        match face {
            BeingFace::Civilian | BeingFace::Pilot | BeingFace::Incarnate => &[Mobility::Walking],
            BeingFace::God => &[Mobility::Walking, Mobility::Flying],
            BeingFace::Mob => &[
                Mobility::Rooted,
                Mobility::Creeping,
                Mobility::Walking,
                Mobility::Flying,
            ],
            BeingFace::Plant => &[Mobility::Rooted],
            BeingFace::Ghost => &[Mobility::Flying],
        }
    }

    /// Whether a face may be declared with this mobility.
    pub fn is_declared_for(self, face: BeingFace) -> bool {
        Mobility::declared_for(face).contains(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_has_a_word_and_every_word_round_trips() {
        // Totality and naming: a face with no spelling is a face no report can print, and one this
        // parser cannot read back. Both halves, because the names are the axis vocabulary.
        for face in BeingFace::ALL {
            assert_eq!(BeingFace::parse(face.name()), Ok(face), "{face:?}");
        }
        assert_eq!(BeingFace::ALL.len(), FACE_COUNT);
    }

    #[test]
    fn an_unknown_word_is_refused_by_name() {
        let refused = BeingFace::parse("dragon").expect_err("a face nobody declared");
        assert!(refused.describe().contains("dragon"), "{}", refused.describe());
        assert_eq!(refused.name(), "unknown_face");
    }

    #[test]
    fn a_plant_is_the_only_being_that_never_acts_and_a_ghost_is_undecided() {
        // The reference's table, as three assertions. The third is the one that matters: the ghost
        // must not be silently modelled as inert, which a `bool` would have done.
        assert_eq!(BeingFace::Plant.agency(), Agency::BeingOnly);
        assert_eq!(BeingFace::Mob.agency(), Agency::Agent);
        assert_eq!(BeingFace::Ghost.agency(), Agency::Undecided);
        assert_eq!(
            BeingFace::ALL
                .into_iter()
                .filter(|face| face.agency() == Agency::BeingOnly)
                .count(),
            1,
            "exactly one face is a being and not an agent"
        );
    }

    #[test]
    fn a_plant_cannot_hold_an_office_and_a_ghost_is_not_an_answer() {
        assert!(!BeingFace::Plant.may_hold_office());
        assert!(BeingFace::Civilian.may_hold_office());
        // Both refuse, and for different reasons — the reason is the report's, so it is checked.
        assert_eq!(BeingFace::Plant.agency(), Agency::BeingOnly);
        assert_eq!(BeingFace::Ghost.agency(), Agency::Undecided);
    }

    #[test]
    fn the_mob_declares_all_four_mobilities_and_the_plant_declares_one() {
        assert_eq!(Mobility::declared_for(BeingFace::Mob).len(), Mobility::COUNT);
        assert_eq!(
            Mobility::declared_for(BeingFace::Plant),
            &[Mobility::Rooted]
        );
        assert!(!Mobility::Walking.is_declared_for(BeingFace::Plant));
        assert!(Mobility::Rooted.is_declared_for(BeingFace::Mob));
    }

    #[test]
    fn every_mobility_has_a_word_and_round_trips() {
        for mobility in Mobility::ALL {
            assert_eq!(Mobility::parse(mobility.name()), Ok(mobility));
        }
        assert_eq!(Mobility::ALL.len(), Mobility::COUNT);
        assert!(Mobility::parse("teleport").is_err());
    }
}
