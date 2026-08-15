use super::{
    is_sorted_unique, is_valid_date, parse_rfc3339, Validate, ValidationResult, Validator,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionStatus {
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionDifficulty {
    S,
    A,
    B,
    C,
    D,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mission {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: MissionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<MissionDifficulty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

impl Validate for Mission {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require_non_blank(&self.id, "id");
        validator.require_non_blank(&self.title, "title");
        if let Some(description) = &self.description {
            validator.require_non_blank(description, "description");
        }
        if let Some(progress) = self.progress {
            validator.require(
                progress <= 100,
                "invalid_mission_progress",
                "progress",
                "progress must be an integer from 0 through 100",
            );
        }
        if let Some(deadline) = &self.deadline {
            validator.require(
                is_valid_date(deadline),
                "invalid_mission_deadline",
                "deadline",
                "deadline must be a valid YYYY-MM-DD date",
            );
        }
        if let Some(parent_id) = &self.parent_id {
            validator.require_non_blank(parent_id, "parent_id");
            validator.require(
                parent_id != &self.id,
                "mission_self_parent",
                "parent_id",
                "Mission cannot be its own parent",
            );
        }

        let created_at = parse_rfc3339(&self.created_at);
        validator.require(
            created_at.is_some(),
            "invalid_mission_created_at",
            "created_at",
            "created_at must be RFC 3339 with a timezone offset",
        );
        let completed_at = self.completed_at.as_deref().and_then(parse_rfc3339);
        if self.completed_at.is_some() {
            validator.require(
                completed_at.is_some(),
                "invalid_mission_completed_at",
                "completed_at",
                "completed_at must be RFC 3339 with a timezone offset",
            );
            validator.require(
                self.status != MissionStatus::Active,
                "completed_at_on_active_mission",
                "completed_at",
                "active Mission cannot contain completed_at",
            );
            validator.require(
                self.progress.is_none() || self.progress == Some(100),
                "completed_mission_progress",
                "progress",
                "Mission with completed_at must omit progress or set it to 100",
            );
        }
        if self.status == MissionStatus::Completed {
            validator.require(
                self.progress.is_none() || self.progress == Some(100),
                "completed_mission_progress",
                "progress",
                "completed Mission must omit progress or set it to 100",
            );
        }
        if let (Some(created_at), Some(completed_at)) = (created_at, completed_at) {
            validator.require(
                completed_at >= created_at,
                "mission_time_order",
                "completed_at",
                "completed_at must not be earlier than created_at",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionFile {
    pub missions: Vec<Mission>,
}

impl Validate for MissionFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.missions.is_empty(),
            "empty_file",
            "missions",
            "missions.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.missions, |mission| mission.id.as_str()),
            "missions_not_sorted_unique",
            "missions",
            "missions must be unique and sorted by id",
        );
        let ids: BTreeSet<&str> = self
            .missions
            .iter()
            .map(|mission| mission.id.as_str())
            .collect();
        for (index, mission) in self.missions.iter().enumerate() {
            validator.merge(&format!("missions[{index}]"), mission.validate());
            if let Some(parent_id) = &mission.parent_id {
                validator.require(
                    ids.contains(parent_id.as_str()),
                    "mission_parent_missing",
                    &format!("missions[{index}].parent_id"),
                    "parent_id must reference a Mission in the same file",
                );
            }
        }
        validate_parent_dag(&mut validator, &self.missions);
        validator.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionSuggestionStatus {
    Pending,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionSuggestion {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<MissionDifficulty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_mission_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub generated_at: String,
    pub status: MissionSuggestionStatus,
}

impl Validate for MissionSuggestion {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require_non_blank(&self.id, "id");
        validator.require_non_blank(&self.title, "title");
        for (value, path) in [
            (&self.description, "description"),
            (&self.parent_mission_id, "parent_mission_id"),
            (&self.reason, "reason"),
        ] {
            if let Some(value) = value {
                validator.require_non_blank(value, path);
            }
        }
        if let Some(deadline) = &self.deadline {
            validator.require(
                is_valid_date(deadline),
                "invalid_mission_deadline",
                "deadline",
                "deadline must be a valid YYYY-MM-DD date",
            );
        }
        validator.require(
            parse_rfc3339(&self.generated_at).is_some(),
            "invalid_suggestion_generated_at",
            "generated_at",
            "generated_at must be RFC 3339 with a timezone offset",
        );
        validator.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardMissionSlot {
    Countdown,
    Progress,
    #[serde(rename = "hint_1")]
    Hint1,
    #[serde(rename = "hint_2")]
    Hint2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DashboardMissionSelection {
    pub mission_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

pub type DashboardMissionSelections = BTreeMap<DashboardMissionSlot, DashboardMissionSelection>;

impl Validate for DashboardMissionSelection {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require_non_blank(&self.mission_id, "mission_id");
        if let Some(label) = &self.label {
            validator.require_non_blank(label, "label");
        }
        validator.finish()
    }
}

fn validate_parent_dag(validator: &mut Validator, missions: &[Mission]) {
    let parents: BTreeMap<&str, Option<&str>> = missions
        .iter()
        .map(|mission| (mission.id.as_str(), mission.parent_id.as_deref()))
        .collect();

    for mission in missions {
        let mut path = BTreeSet::new();
        let mut current = Some(mission.id.as_str());
        while let Some(id) = current {
            if !path.insert(id) {
                validator.error(
                    "mission_parent_cycle",
                    "missions",
                    format!("parent relationship contains a cycle through '{id}'"),
                );
                break;
            }
            current = parents.get(id).copied().flatten();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mission(id: &str, parent_id: Option<&str>) -> Mission {
        Mission {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            status: MissionStatus::Active,
            progress: None,
            difficulty: None,
            deadline: None,
            parent_id: parent_id.map(str::to_string),
            created_at: "2026-08-15T20:30:00+08:00".to_string(),
            completed_at: None,
        }
    }

    #[test]
    fn mission_file_rejects_parent_cycle() {
        let file = MissionFile {
            missions: vec![mission("a", Some("b")), mission("b", Some("a"))],
        };
        assert!(file.validate().is_err());
    }

    #[test]
    fn archived_mission_may_retain_completion_time() {
        let mut mission = mission("a", None);
        mission.status = MissionStatus::Archived;
        mission.progress = Some(100);
        mission.completed_at = Some("2026-08-16T20:30:00+08:00".to_string());
        assert!(mission.validate().is_ok());
    }

    #[test]
    fn mission_json_rejects_removed_legacy_fields() {
        let value = serde_json::json!({
            "id": "a",
            "title": "A",
            "status": "active",
            "created_at": "2026-08-15T20:30:00+08:00",
            "short_desc": "legacy"
        });
        assert!(serde_json::from_value::<Mission>(value).is_err());
    }
}
