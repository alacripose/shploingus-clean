use crate::matter::Rational;
use crate::world::{ActorId, TaskId};

use super::{ActorError, SkillId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(value: impl Into<String>) -> Result<Self, ActorError> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            return Err(ActorError::InvalidId {
                kind: "tool".to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkTask {
    pub id: TaskId,
    pub actor: ActorId,
    pub required_skill: Option<SkillId>,
    pub minimum_skill: Rational,
    pub required_tool: Option<ToolId>,
    pub status: TaskStatus,
}

impl WorkTask {
    pub fn new(
        actor: ActorId,
        required_skill: Option<SkillId>,
        minimum_skill: Rational,
        required_tool: Option<ToolId>,
    ) -> Self {
        Self {
            id: TaskId::new("work").expect("static task id is valid"),
            actor,
            required_skill,
            minimum_skill,
            required_tool,
            status: TaskStatus::Ready,
        }
    }

    pub fn with_id(mut self, id: TaskId) -> Self {
        self.id = id;
        self
    }
}
