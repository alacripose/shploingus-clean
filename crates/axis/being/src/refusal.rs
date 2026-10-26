//! **The refusals this crate makes, each naming the value that was refused.**
//!
//! The workspace's refusal vocabulary lives in `voxel_model::RefusalKind`, and this crate adds none
//! of its own to it: a new central variant is a change to every exhaustive match in the tree, and
//! these are refusals about *living things*, not about coordinates or materials. So the being's
//! refusals are their own small closed set with their own words, and a later axis crate will map
//! them onto the ladder the report speaks.
//!
//! **Two refusals that would belong here are absent, and their absence is structural:**
//!
//! * there is no `NoSignature`, because [`crate::Being::create`] takes a
//!   [`crate::SpiritualSignature`](crate::signature::SpiritualSignature) and has no default;
//! * there is no `NoCreationRecord`, because the same constructor takes one, and the id is derived
//!   from it.
//!
//! A refusal for a state the type cannot hold is a test that can never fail.

/// What this crate refuses, and why — in the crate's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BeingRefusal {
    /// A row, a reference or a selection named a being with no creation record.
    ///
    /// This is `PATCH-042`'s GH2 (`AbsentSubject`), and it exists because a clicker game's cheapest
    /// growth path is to invent a person: *"the one that would have been missed, and the one that
    /// matters most"* (`GOD_AGENTS.md:16252`).
    AbsentSubject {
        /// The id that was named.
        subject: String,
    },
    /// A being's creation record declared no mass, or zero.
    Massless {
        /// The reason, in words the producer can act on.
        detail: String,
    },
    /// A creation record cites no provenance — no source, no parent, no record it came from.
    Unprovenanced {
        /// What was empty.
        detail: String,
    },
    /// A face was declared with a mobility its own row does not allow.
    MobilityNotDeclared {
        /// The face, by its word.
        face: &'static str,
        /// The mobility, by its word.
        mobility: &'static str,
    },
    /// A need was read above `ONE`.
    NeedAboveOne {
        /// The need, by its name.
        need: &'static str,
        /// The rule being enforced.
        detail: String,
    },
    /// A signature's wheel position lay outside the wheel.
    AngleOutsideWheel {
        /// The angle that was offered.
        angle: u16,
        /// The steps the wheel actually has.
        steps: u16,
    },
    /// A life stage was asked for that is not the stage's own successor.
    NotTheNextStage {
        /// The stage the being is in.
        from: &'static str,
        /// The stage the caller asked to move to.
        to: &'static str,
    },
    /// A stage change was asked of a being that is already dead.
    AlreadyDead {
        /// The being, by id.
        being: String,
    },
    /// A word was offered that this vocabulary does not declare.
    StageNotDeclared {
        /// The word.
        word: String,
    },
    /// A face word was offered that this vocabulary does not declare.
    UnknownFace {
        /// The word.
        word: String,
    },
    /// A mobility word was offered that this vocabulary does not declare.
    UnknownMobility {
        /// The word.
        word: String,
    },
}

impl BeingRefusal {
    /// The stable word a report prints. **Never the `Debug` spelling**, which a rename would move.
    pub const fn name(&self) -> &'static str {
        match self {
            BeingRefusal::AbsentSubject { .. } => "absent_subject",
            BeingRefusal::Massless { .. } => "massless",
            BeingRefusal::Unprovenanced { .. } => "unprovenanced",
            BeingRefusal::MobilityNotDeclared { .. } => "mobility_not_declared",
            BeingRefusal::NeedAboveOne { .. } => "need_above_one",
            BeingRefusal::AngleOutsideWheel { .. } => "angle_outside_wheel",
            BeingRefusal::NotTheNextStage { .. } => "not_the_next_stage",
            BeingRefusal::AlreadyDead { .. } => "already_dead",
            BeingRefusal::StageNotDeclared { .. } => "stage_not_declared",
            BeingRefusal::UnknownFace { .. } => "unknown_face",
            BeingRefusal::UnknownMobility { .. } => "unknown_mobility",
        }
    }

    /// The refusal in one sentence, naming the value.
    pub fn describe(&self) -> String {
        match self {
            BeingRefusal::AbsentSubject { subject } => format!(
                "{subject} is named by a record but has no creation record, so it is an absent \
                 subject and emits nothing"
            ),
            BeingRefusal::Massless { detail } => {
                format!("a being must be matter before it is a character: {detail}")
            }
            BeingRefusal::Unprovenanced { detail } => format!(
                "a creation record must say where the being came from: {detail}"
            ),
            BeingRefusal::MobilityNotDeclared { face, mobility } => format!(
                "a {face} may not be declared {mobility}: the face's row does not allow it"
            ),
            BeingRefusal::NeedAboveOne { need, detail } => {
                format!("the need {need} read above ONE: {detail}")
            }
            BeingRefusal::AngleOutsideWheel { angle, steps } => format!(
                "the angle {angle} is outside the wheel: the wheel has {steps} steps"
            ),
            BeingRefusal::NotTheNextStage { from, to } => format!(
                "a being at {from} cannot move to {to}: a stage advances to its own successor"
            ),
            BeingRefusal::AlreadyDead { being } => {
                format!("{being} is already dead, and a dead being has no next stage")
            }
            BeingRefusal::StageNotDeclared { word } => {
                format!("{word:?} is not a stage this vocabulary declares")
            }
            BeingRefusal::UnknownFace { word } => {
                format!("{word:?} is not a face this vocabulary declares")
            }
            BeingRefusal::UnknownMobility { word } => {
                format!("{word:?} is not a mobility this vocabulary declares")
            }
        }
    }
}

impl std::fmt::Display for BeingRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.describe())
    }
}

impl std::error::Error for BeingRefusal {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_has_one_word_and_a_sentence_that_names_the_value() {
        let refusals = [
            BeingRefusal::AbsentSubject {
                subject: "being_aaaa".to_owned(),
            },
            BeingRefusal::Massless {
                detail: "zero".to_owned(),
            },
            BeingRefusal::Unprovenanced {
                detail: "empty".to_owned(),
            },
            BeingRefusal::MobilityNotDeclared {
                face: "plant",
                mobility: "walking",
            },
            BeingRefusal::NeedAboveOne {
                need: "water",
                detail: "2/1".to_owned(),
            },
            BeingRefusal::AngleOutsideWheel {
                angle: 500,
                steps: 432,
            },
            BeingRefusal::NotTheNextStage {
                from: "seed",
                to: "mature",
            },
            BeingRefusal::AlreadyDead {
                being: "being_bbbb".to_owned(),
            },
            BeingRefusal::StageNotDeclared {
                word: "ancient".to_owned(),
            },
            BeingRefusal::UnknownFace {
                word: "dragon".to_owned(),
            },
            BeingRefusal::UnknownMobility {
                word: "teleport".to_owned(),
            },
        ];
        for refusal in refusals {
            assert!(!refusal.name().is_empty());
            assert!(!refusal.describe().is_empty());
            assert!(
                refusal.describe().len() > 20,
                "{} is a placeholder, not a sentence",
                refusal.name()
            );
        }
    }

    #[test]
    fn a_refusal_is_an_error_and_prints_its_description() {
        let refusal = BeingRefusal::UnknownFace {
            word: "dragon".to_owned(),
        };
        let as_error: &dyn std::error::Error = &refusal;
        assert!(as_error.to_string().contains("dragon"));
    }
}
