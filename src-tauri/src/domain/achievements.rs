use super::{
    is_snake_case_id, is_sorted_unique, is_valid_partial_date, split_record_definition_id,
    split_scoped_id, Validate, ValidationResult, Validator,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AchievementDifficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Legendary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: AchievementDifficulty,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_record_definition_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip: Option<String>,
}

impl Validate for AchievementDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_scoped_id(&self.id).is_some(),
            "invalid_achievement_id",
            "id",
            "must be <pack_id>::<local_id> using lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        validator.require_non_blank(&self.description, "description");
        if let Some(tip) = &self.tip {
            validator.require_non_blank(tip, "tip");
        }
        validate_sorted_tags(&mut validator, &self.tags, "tags");
        validator.require(
            is_strictly_sorted(&self.prerequisites),
            "prerequisites_not_sorted_unique",
            "prerequisites",
            "prerequisites must be unique and sorted",
        );
        validator.require(
            is_strictly_sorted(&self.related_record_definition_ids),
            "related_records_not_sorted_unique",
            "related_record_definition_ids",
            "related RecordDefinition ids must be unique and sorted",
        );
        for (index, id) in self.related_record_definition_ids.iter().enumerate() {
            validator.require(
                split_record_definition_id(id).is_some(),
                "invalid_record_definition_id",
                &format!("related_record_definition_ids[{index}]"),
                "must be <namespace>.<name> using lowercase snake_case",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementFile {
    pub achievements: Vec<AchievementDefinition>,
}

impl Validate for AchievementFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.achievements.is_empty(),
            "empty_file",
            "achievements",
            "achievements.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.achievements, |achievement| achievement.id.as_str()),
            "achievements_not_sorted_unique",
            "achievements",
            "achievements must be unique and sorted by id",
        );
        for (index, achievement) in self.achievements.iter().enumerate() {
            validator.merge(&format!("achievements[{index}]"), achievement.validate());
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AchievementStatus {
    Tracked,
    Achieved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementState {
    pub status: AchievementStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achieved_at: Option<String>,
}

impl AchievementState {
    pub fn achieved_at(&self) -> Option<&str> {
        self.achieved_at.as_deref()
    }

    pub fn is_achieved(&self) -> bool {
        self.status == AchievementStatus::Achieved
    }
}

impl Validate for AchievementState {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            self.status == AchievementStatus::Achieved || self.achieved_at.is_none(),
            "achieved_at_on_tracked_state",
            "achieved_at",
            "achieved_at is only allowed when status is achieved",
        );
        if let Some(achieved_at) = self.achieved_at() {
            validator.require(
                is_valid_partial_date(achieved_at),
                "invalid_achieved_at",
                "achieved_at",
                "must be YYYY, YYYY-MM, or YYYY-MM-DD",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementStateFile {
    pub states: BTreeMap<String, AchievementState>,
}

impl Validate for AchievementStateFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.states.is_empty(),
            "empty_file",
            "states",
            "an empty AchievementState file must be omitted",
        );
        for (id, state) in &self.states {
            validator.require(
                split_scoped_id(id).is_some(),
                "invalid_achievement_id",
                &format!("states.{id}"),
                "state key must be a valid Achievement id",
            );
            validator.merge(&format!("states.{id}"), state.validate());
        }
        validator.finish()
    }
}

fn validate_sorted_tags(validator: &mut Validator, tags: &[String], path: &str) {
    validator.require(
        is_strictly_sorted(tags),
        "tags_not_sorted_unique",
        path,
        "tags must be unique and sorted",
    );
    for (index, tag) in tags.iter().enumerate() {
        validator.require(
            is_snake_case_id(tag),
            "invalid_tag",
            &format!("{path}[{index}]"),
            "tag must use lowercase snake_case",
        );
    }
}

fn is_strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_state_rejects_achieved_at_during_validation() {
        let json = serde_json::json!({"status": "tracked", "achieved_at": "2026"});
        let state = serde_json::from_value::<AchievementState>(json).unwrap();
        assert!(state.validate().is_err());
    }

    #[test]
    fn achieved_state_accepts_partial_date() {
        let state = AchievementState {
            status: AchievementStatus::Achieved,
            achieved_at: Some("2025-06".to_string()),
        };
        assert!(state.validate().is_ok());
    }
}
