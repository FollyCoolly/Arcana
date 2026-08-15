use super::{is_sorted_unique, parse_rfc3339, Validate, ValidationResult, Validator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssistantMemoryKind {
    Focus,
    Preference,
    Constraint,
    Habit,
    Summary,
    Reminder,
    Observation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistantMemory {
    pub id: String,
    pub kind: AssistantMemoryKind,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Validate for AssistantMemory {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require_non_blank(&self.id, "id");
        validator.require_non_blank(&self.content, "content");
        let created_at = parse_rfc3339(&self.created_at);
        let updated_at = parse_rfc3339(&self.updated_at);
        validator.require(
            created_at.is_some(),
            "invalid_memory_created_at",
            "created_at",
            "created_at must be RFC 3339 with a timezone offset",
        );
        validator.require(
            updated_at.is_some(),
            "invalid_memory_updated_at",
            "updated_at",
            "updated_at must be RFC 3339 with a timezone offset",
        );
        if let (Some(created_at), Some(updated_at)) = (created_at, updated_at) {
            validator.require(
                updated_at >= created_at,
                "memory_time_order",
                "updated_at",
                "updated_at must not be earlier than created_at",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistantMemoryFile {
    pub memories: Vec<AssistantMemory>,
}

impl Validate for AssistantMemoryFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.memories.is_empty(),
            "empty_file",
            "memories",
            "assistant-memory.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.memories, |memory| memory.id.as_str()),
            "memories_not_sorted_unique",
            "memories",
            "memories must be unique and sorted by id",
        );
        for (index, memory) in self.memories.iter().enumerate() {
            validator.merge(&format!("memories[{index}]"), memory.validate());
        }
        validator.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_update_cannot_precede_creation() {
        let memory = AssistantMemory {
            id: "legacy-id".to_string(),
            kind: AssistantMemoryKind::Preference,
            content: "Short missions".to_string(),
            created_at: "2026-08-16T00:00:00+08:00".to_string(),
            updated_at: "2026-08-15T00:00:00+08:00".to_string(),
        };
        assert!(memory.validate().is_err());
    }
}
