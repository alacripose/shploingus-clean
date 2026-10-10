use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::matter::Rational;
use crate::world::{coord_text, ActorId, Coord, PhysicalVerb, WorldError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillId(String);

impl SkillId {
    pub fn new(value: impl Into<String>) -> Result<Self, ActorError> {
        named_id("skill", value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Needs {
    nourishment: Rational,
    rest: Rational,
    safety: Rational,
}

impl Needs {
    pub const fn new(nourishment: Rational, rest: Rational, safety: Rational) -> Self {
        Self {
            nourishment,
            rest,
            safety,
        }
    }

    pub const fn nourishment(self) -> Rational {
        self.nourishment
    }

    pub const fn rest(self) -> Rational {
        self.rest
    }

    pub const fn safety(self) -> Rational {
        self.safety
    }

    fn validate(self) -> Result<(), ActorError> {
        for (name, value) in [
            ("nourishment", self.nourishment),
            ("rest", self.rest),
            ("safety", self.safety),
        ] {
            if value.is_negative()
                || !matches!(
                    value.try_cmp(&Rational::from_integer(1)),
                    Ok(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            {
                return Err(ActorError::NeedOutOfRange { need: name, value });
            }
        }
        Ok(())
    }
}

impl Default for Needs {
    fn default() -> Self {
        Self::new(
            Rational::from_integer(1),
            Rational::from_integer(1),
            Rational::from_integer(1),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CitizenRole {
    Player,
    Autonomous,
}

impl CitizenRole {
    pub fn capabilities(self) -> BTreeSet<PhysicalVerb> {
        match self {
            Self::Player => [
                PhysicalVerb::Break,
                PhysicalVerb::Place,
                PhysicalVerb::Gather,
                PhysicalVerb::Dig,
                PhysicalVerb::Fill,
                PhysicalVerb::Use,
                PhysicalVerb::Craft,
            ]
            .into_iter()
            .collect(),
            Self::Autonomous => [
                PhysicalVerb::Break,
                PhysicalVerb::Gather,
                PhysicalVerb::Dig,
                PhysicalVerb::Fill,
                PhysicalVerb::Use,
            ]
            .into_iter()
            .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Injured { reason: String },
    Diseased { condition: String },
    Dead { reason: String },
    Retired { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub state: LifecycleState,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citizen {
    id: ActorId,
    position: Coord,
    needs: Needs,
    health: Rational,
    skills: BTreeMap<SkillId, Rational>,
    lifecycle: LifecycleState,
    role: CitizenRole,
    history: Vec<LifecycleEvent>,
}

pub type PlayerCitizen = Citizen;

impl Citizen {
    pub fn player(id: ActorId, position: Coord) -> Result<Self, ActorError> {
        Self::new(id, position, CitizenRole::Player)
    }

    pub fn autonomous(id: ActorId, position: Coord) -> Result<Self, ActorError> {
        Self::new(id, position, CitizenRole::Autonomous)
    }

    fn new(id: ActorId, position: Coord, role: CitizenRole) -> Result<Self, ActorError> {
        validate_coord(position)?;
        let lifecycle = LifecycleState::Active;
        Ok(Self {
            id,
            position,
            needs: Needs::default(),
            health: Rational::from_integer(1),
            skills: BTreeMap::new(),
            lifecycle: lifecycle.clone(),
            role,
            history: vec![LifecycleEvent {
                state: lifecycle,
                reason: "registered".to_owned(),
            }],
        })
    }

    pub fn with_skills(mut self, skills: BTreeMap<SkillId, Rational>) -> Result<Self, ActorError> {
        for (skill, value) in &skills {
            if value.is_negative() {
                return Err(ActorError::SkillOutOfRange {
                    skill: skill.clone(),
                    value: *value,
                });
            }
        }
        self.skills = skills;
        self.validate()?;
        Ok(self)
    }

    pub fn id(&self) -> &ActorId {
        &self.id
    }

    pub fn position(&self) -> Coord {
        self.position
    }

    pub fn needs(&self) -> Needs {
        self.needs
    }

    pub fn health(&self) -> Rational {
        self.health
    }

    pub fn skills(&self) -> &BTreeMap<SkillId, Rational> {
        &self.skills
    }

    pub fn lifecycle(&self) -> &LifecycleState {
        &self.lifecycle
    }

    pub fn role(&self) -> CitizenRole {
        self.role
    }

    pub fn history(&self) -> &[LifecycleEvent] {
        &self.history
    }

    pub fn can_participate(&self) -> bool {
        matches!(self.lifecycle, LifecycleState::Active)
    }

    pub fn supports(&self, verb: PhysicalVerb) -> bool {
        self.role.capabilities().contains(&verb)
    }

    pub fn validate(&self) -> Result<(), ActorError> {
        validate_coord(self.position)?;
        self.needs.validate()?;
        validate_health(self.health)?;
        for (skill, value) in &self.skills {
            if value.is_negative() {
                return Err(ActorError::SkillOutOfRange {
                    skill: skill.clone(),
                    value: *value,
                });
            }
        }
        Ok(())
    }

    pub fn move_to(&mut self, position: Coord) -> Result<(), ActorError> {
        validate_coord(position)?;
        self.position = position;
        Ok(())
    }

    pub fn update_needs(&mut self, needs: Needs) -> Result<(), ActorError> {
        needs.validate()?;
        self.needs = needs;
        Ok(())
    }

    pub fn set_health(&mut self, health: Rational) -> Result<(), ActorError> {
        validate_health(health)?;
        self.health = health;
        Ok(())
    }

    pub fn learn(&mut self, skill: SkillId, amount: Rational) -> Result<(), ActorError> {
        if amount.is_negative() {
            return Err(ActorError::SkillOutOfRange {
                skill,
                value: amount,
            });
        }
        let current = self.skills.get(&skill).copied().unwrap_or(Rational::ZERO);
        self.skills.insert(
            skill,
            current
                .checked_add(&amount)
                .ok_or(ActorError::SkillAmountOverflow)?,
        );
        Ok(())
    }

    pub fn injure(&mut self, reason: impl Into<String>) -> Result<(), ActorError> {
        self.transition(
            LifecycleState::Injured {
                reason: reason.into(),
            },
            true,
        )
    }

    pub fn recover_injury(&mut self) -> Result<(), ActorError> {
        self.transition_active("recovered from injury".to_owned())
    }

    pub fn contract_disease(&mut self, condition: impl Into<String>) -> Result<(), ActorError> {
        self.transition(
            LifecycleState::Diseased {
                condition: condition.into(),
            },
            true,
        )
    }

    pub fn recover_disease(&mut self) -> Result<(), ActorError> {
        self.transition_active("recovered from disease".to_owned())
    }

    pub fn retire(&mut self, reason: impl Into<String>) -> Result<(), ActorError> {
        self.transition(
            LifecycleState::Retired {
                reason: reason.into(),
            },
            true,
        )
    }

    pub fn reenter(&mut self, reason: impl Into<String>) -> Result<(), ActorError> {
        if !matches!(self.lifecycle, LifecycleState::Retired { .. }) {
            return Err(ActorError::InvalidLifecycleTransition {
                from: self.lifecycle.clone(),
                to: LifecycleState::Active,
            });
        }
        self.lifecycle = LifecycleState::Active;
        self.history.push(LifecycleEvent {
            state: LifecycleState::Active,
            reason: reason.into(),
        });
        Ok(())
    }

    pub fn die(&mut self, reason: impl Into<String>) -> Result<(), ActorError> {
        self.transition(
            LifecycleState::Dead {
                reason: reason.into(),
            },
            true,
        )
    }

    fn transition_active(&mut self, reason: String) -> Result<(), ActorError> {
        self.transition_with_reason(LifecycleState::Active, false, reason)
    }

    fn transition(&mut self, next: LifecycleState, from_active: bool) -> Result<(), ActorError> {
        let reason = match &next {
            LifecycleState::Active => "recovered or re-entered".to_owned(),
            LifecycleState::Injured { reason }
            | LifecycleState::Diseased { condition: reason }
            | LifecycleState::Dead { reason }
            | LifecycleState::Retired { reason } => reason.clone(),
        };
        self.transition_with_reason(next, from_active, reason)
    }

    fn transition_with_reason(
        &mut self,
        next: LifecycleState,
        from_active: bool,
        reason: String,
    ) -> Result<(), ActorError> {
        let allowed = if from_active {
            !matches!(
                self.lifecycle,
                LifecycleState::Dead { .. } | LifecycleState::Retired { .. }
            )
        } else {
            matches!(
                self.lifecycle,
                LifecycleState::Injured { .. } | LifecycleState::Diseased { .. }
            )
        };
        if !allowed {
            return Err(ActorError::InvalidLifecycleTransition {
                from: self.lifecycle.clone(),
                to: next,
            });
        }
        self.lifecycle = next.clone();
        self.history.push(LifecycleEvent {
            state: next,
            reason,
        });
        Ok(())
    }
}

fn validate_health(health: Rational) -> Result<(), ActorError> {
    if health.is_negative()
        || !matches!(
            health.try_cmp(&Rational::from_integer(1)),
            Ok(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        )
    {
        return Err(ActorError::HealthOutOfRange { value: health });
    }
    Ok(())
}

fn named_id(kind: &str, value: impl Into<String>) -> Result<String, ActorError> {
    let value = value.into();
    let value = value.trim();
    if value.is_empty() {
        return Err(ActorError::InvalidId {
            kind: kind.to_owned(),
        });
    }
    Ok(value.to_owned())
}

fn validate_coord(coord: Coord) -> Result<(), ActorError> {
    let region = crate::world::SignedRegion::new(Coord::new(-32, -32, -16), Coord::new(31, 31, 47))
        .map_err(|error: WorldError| ActorError::InvalidPosition {
            reason: error.to_string(),
        })?;
    if region.contains(coord) {
        Ok(())
    } else {
        Err(ActorError::InvalidPosition {
            reason: format!(
                "{} is outside the signed first-band region",
                coord_text(coord)
            ),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorError {
    InvalidId {
        kind: String,
    },
    InvalidPosition {
        reason: String,
    },
    HealthOutOfRange {
        value: Rational,
    },
    NeedOutOfRange {
        need: &'static str,
        value: Rational,
    },
    SkillOutOfRange {
        skill: SkillId,
        value: Rational,
    },
    SkillAmountOverflow,
    InvalidLifecycleTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
}

impl fmt::Display for ActorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { kind } => write!(formatter, "{kind} id must not be empty"),
            Self::InvalidPosition { reason } => {
                write!(formatter, "invalid citizen position: {reason}")
            }
            Self::HealthOutOfRange { .. } => {
                formatter.write_str("health must be between zero and one")
            }
            Self::NeedOutOfRange { need, .. } => {
                write!(formatter, "{need} need must be between zero and one")
            }
            Self::SkillOutOfRange { skill, .. } => {
                write!(formatter, "skill {} cannot be negative", skill.as_str())
            }
            Self::SkillAmountOverflow => formatter.write_str("skill amount overflow"),
            Self::InvalidLifecycleTransition { from, to } => write!(
                formatter,
                "invalid lifecycle transition from {from:?} to {to:?}"
            ),
        }
    }
}

impl std::error::Error for ActorError {}
