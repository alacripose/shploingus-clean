//! **The life a being has** — a stage, a state, and the needs that drive its own choices.
//!
//! Three types, and they are deliberately three: a stage is *where in its life* a being is, a state
//! is *whether it is living, retired or dead*, and a need is *what it wants*. Collapsing any two is
//! how "retired" comes to mean "dead", which is the exact confusion `CIVILIANS.md` §3.1's record
//! discipline exists to prevent.

use voxel_model::Ratio;

use crate::refusal::BeingRefusal;

/// How many stages a being's life declares.
pub const STAGE_COUNT: usize = 5;

/// How many states a being's record can hold.
pub const STATE_COUNT: usize = 3;

/// Where in its life a being is.
///
/// **A stage is not a number and not an age.** It advances by a declared successor, so a growth rule
/// may decide *when* the next stage begins but cannot skip one; and `Dead` is a stage rather than a
/// deletion, which is what makes the record outlive the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifeStage {
    /// A seed, an egg, a spore: the being exists and is not yet itself.
    Seed,
    /// Young: growing, not yet reproductive or full-sized.
    Sapling,
    /// Full size, in its prime.
    Mature,
    /// Old: the stage before death, and the one a veteran tree's record keeps.
    Veteran,
    /// Past living. **The record stays**; the mass goes to whatever the face's law says.
    Dead,
}

impl LifeStage {
    /// Every stage, in the order they are reached.
    pub const ALL: [LifeStage; STAGE_COUNT] = [
        LifeStage::Seed,
        LifeStage::Sapling,
        LifeStage::Mature,
        LifeStage::Veteran,
        LifeStage::Dead,
    ];

    /// The word.
    pub const fn name(self) -> &'static str {
        match self {
            LifeStage::Seed => "seed",
            LifeStage::Sapling => "sapling",
            LifeStage::Mature => "mature",
            LifeStage::Veteran => "veteran",
            LifeStage::Dead => "dead",
        }
    }

    /// The word as a value, refusing one this ladder does not declare.
    pub fn parse(word: &str) -> Result<Self, BeingRefusal> {
        let wanted = word.trim();
        Self::ALL
            .into_iter()
            .find(|stage| stage.name() == wanted)
            .ok_or_else(|| BeingRefusal::StageNotDeclared {
                word: wanted.to_owned(),
            })
    }

    /// The next stage, or `None` at `Dead`.
    pub const fn next(self) -> Option<LifeStage> {
        match self {
            LifeStage::Seed => Some(LifeStage::Sapling),
            LifeStage::Sapling => Some(LifeStage::Mature),
            LifeStage::Mature => Some(LifeStage::Veteran),
            LifeStage::Veteran => Some(LifeStage::Dead),
            LifeStage::Dead => None,
        }
    }

    /// Whether this stage is past living.
    pub const fn is_dead(self) -> bool {
        matches!(self, LifeStage::Dead)
    }
}

/// Whether a being is living, retired, or dead.
///
/// **Retired is not dead.** A retired being is a record that no longer acts — an office vacated, a
/// citizen stood down — and its mass, its signature and its creation record are untouched. A dead
/// being is a stage change whose mass has gone somewhere the face's law names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
    /// Living.
    Alive,
    /// No longer acting. Still a record, still a being, still alive in the stage sense.
    Retired,
    /// Dead. Terminal, and never reached by deleting anything.
    Dead,
}

impl State {
    /// Every state.
    pub const ALL: [State; STATE_COUNT] = [State::Alive, State::Retired, State::Dead];

    /// The word.
    pub const fn name(self) -> &'static str {
        match self {
            State::Alive => "alive",
            State::Retired => "retired",
            State::Dead => "dead",
        }
    }

    /// The word as a value.
    pub fn parse(word: &str) -> Result<Self, BeingRefusal> {
        let wanted = word.trim();
        Self::ALL
            .into_iter()
            .find(|state| state.name() == wanted)
            .ok_or_else(|| BeingRefusal::StageNotDeclared {
                word: wanted.to_owned(),
            })
    }

    /// Whether a being in this state may still act.
    pub const fn acts(self) -> bool {
        matches!(self, State::Alive)
    }
}

/// **What a being wants, as exact measures** — one per need, `ONE` meaning met.
///
/// C1 line 160 replaces `CitizenState` with *needs, skills, illness, wealth, home, job, life stage*,
/// so a need is the being's **own** state and not an assignment. `Ratio` rather than `f64` for the
/// reason `voxel-model` gives everywhere: a decimal that drifts is worse than no reading, and a
/// need is read by a selection planner that will compare it.
///
/// **`ONE` is met, not full.** A need at `ONE` is satisfied; at `ZERO` it is wholly unmet. A value
/// above one is refused rather than clamped, because a clamp would turn a bug in a producer into a
/// perfect score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Needs {
    /// Light: a plant's first need, and a mob's through its eyes.
    pub light: Ratio,
    /// Water.
    pub water: Ratio,
    /// Nutrient: the soil account a tree draws from, and a civilian's food.
    pub nutrient: Ratio,
}

impl Needs {
    /// The three measures, refusing any above `ONE`.
    pub fn new(light: Ratio, water: Ratio, nutrient: Ratio) -> Result<Self, BeingRefusal> {
        for (name, value) in [
            ("light", light),
            ("water", water),
            ("nutrient", nutrient),
        ] {
            if value > Ratio::ONE {
                return Err(BeingRefusal::NeedAboveOne {
                    need: name,
                    detail: "a need above ONE is a producer bug, and clamping it would hide that"
                        .to_owned(),
                });
            }
        }
        Ok(Needs {
            light,
            water,
            nutrient,
        })
    }

    /// Every need met.
    pub fn met() -> Self {
        Needs {
            light: Ratio::ONE,
            water: Ratio::ONE,
            nutrient: Ratio::ONE,
        }
    }

    /// Every need unmet.
    pub fn unmet() -> Self {
        Needs {
            light: Ratio::ZERO,
            water: Ratio::ZERO,
            nutrient: Ratio::ZERO,
        }
    }

    /// The needs, in declared order, so a report can print them without naming each field.
    pub fn cells(&self) -> [(&'static str, Ratio); 3] {
        [
            ("light", self.light),
            ("water", self.water),
            ("nutrient", self.nutrient),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{Refusal, RefusalKind};

    #[test]
    fn the_stage_ladder_reaches_every_stage_and_stops_at_dead() {
        let mut stage = LifeStage::Seed;
        let mut walked = vec![stage];
        while let Some(next) = stage.next() {
            stage = next;
            walked.push(stage);
        }
        assert_eq!(walked, LifeStage::ALL.to_vec());
        assert_eq!(LifeStage::Dead.next(), None);
        assert_eq!(LifeStage::ALL.len(), STAGE_COUNT);
    }

    #[test]
    fn every_stage_and_state_round_trips_through_its_word() {
        for stage in LifeStage::ALL {
            assert_eq!(LifeStage::parse(stage.name()), Ok(stage));
        }
        for state in State::ALL {
            assert_eq!(State::parse(state.name()), Ok(state));
        }
        assert_eq!(State::ALL.len(), STATE_COUNT);
        assert_eq!(
            LifeStage::parse("mature"),
            Ok(LifeStage::Mature),
            "the word is the one the ladder spells"
        );
        assert!(LifeStage::parse("ancient").is_err());
    }

    #[test]
    fn a_retired_being_still_exists_and_only_alive_acts() {
        assert!(State::Alive.acts());
        assert!(!State::Retired.acts());
        assert!(!State::Dead.acts());
        assert_ne!(State::Retired, State::Dead, "two states, two facts");
    }

    #[test]
    fn a_need_above_one_is_refused_rather_than_clamped() {
        let too_much = Ratio::from_u64(2);
        let refused = Needs::new(too_much, Ratio::ZERO, Ratio::ZERO)
            .expect_err("a need above one is a producer bug");
        assert_eq!(refused.name(), "need_above_one");
        assert!(refused.describe().contains("light"), "{}", refused.describe());

        let fine = Needs::new(Ratio::ONE, Ratio::ZERO, Ratio::ONE).expect("within range");
        assert_eq!(fine.water, Ratio::ZERO);
        assert_eq!(Needs::met().nutrient, Ratio::ONE);
        assert_eq!(Needs::unmet().light, Ratio::ZERO);
    }

    #[test]
    fn the_needs_print_in_a_declared_order() {
        let needs = Needs::new(Ratio::ZERO, Ratio::ONE, Ratio::ZERO).expect("valid");
        let words: Vec<&str> = needs.cells().iter().map(|(name, _)| *name).collect();
        assert_eq!(words, vec!["light", "water", "nutrient"]);
    }

    #[test]
    fn the_model_still_refuses_a_zero_denominator_at_the_boundary() {
        // `Ratio::ONE` is the ceiling, and the ratio type stays the refusal boundary: this test is
        // here so a future change to `Needs` cannot quietly introduce an `f64` that drifts.
        assert!(Ratio::new(1, 0).is_err());
        let refused: Refusal = Ratio::new(1, 0).expect_err("zero denominator");
        assert_eq!(refused.kind(), RefusalKind::RatiosDoNotSumToOne);
    }
}
