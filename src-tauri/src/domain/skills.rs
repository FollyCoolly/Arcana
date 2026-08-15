use super::{
    is_portable_asset_path, is_sorted_unique, split_scoped_id, Validate, ValidationResult,
    Validator,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SKILL_THRESHOLD_COUNT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub level_thresholds: [u64; SKILL_THRESHOLD_COUNT],
    pub nodes: Vec<SkillNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillNode {
    pub achievement_id: String,
    pub points: u64,
}

impl Validate for SkillDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_scoped_id(&self.id).is_some(),
            "invalid_skill_id",
            "id",
            "must be <pack_id>::<local_id> using lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        if let Some(description) = &self.description {
            validator.require_non_blank(description, "description");
        }
        validator.require(
            self.level_thresholds[0] > 0
                && self
                    .level_thresholds
                    .windows(2)
                    .all(|pair| pair[0] < pair[1]),
            "invalid_skill_thresholds",
            "level_thresholds",
            "four thresholds must be positive and strictly increasing",
        );
        validator.require(
            !self.nodes.is_empty(),
            "empty_skill_nodes",
            "nodes",
            "Skill must contain at least one Achievement node",
        );
        validator.require(
            is_sorted_unique(&self.nodes, |node| node.achievement_id.as_str()),
            "skill_nodes_not_sorted_unique",
            "nodes",
            "nodes must be unique and sorted by achievement_id",
        );

        let mut total_points = 0_u64;
        let mut overflow = false;
        for (index, node) in self.nodes.iter().enumerate() {
            validator.require(
                split_scoped_id(&node.achievement_id).is_some(),
                "invalid_achievement_id",
                &format!("nodes[{index}].achievement_id"),
                "must be a valid Achievement id",
            );
            validator.require(
                node.points > 0,
                "invalid_skill_points",
                &format!("nodes[{index}].points"),
                "points must be greater than zero",
            );
            match total_points.checked_add(node.points) {
                Some(total) => total_points = total,
                None => overflow = true,
            }
        }
        validator.require(
            !overflow,
            "skill_points_overflow",
            "nodes",
            "sum of node points exceeds supported integer range",
        );
        if !overflow {
            validator.require(
                self.level_thresholds[SKILL_THRESHOLD_COUNT - 1] <= total_points,
                "unreachable_skill_level",
                "level_thresholds[3]",
                "Lv.5 threshold must not exceed total node points",
            );
        }
        if let Some(card_image) = &self.card_image {
            validator.require(
                is_portable_asset_path(card_image),
                "invalid_card_image_path",
                "card_image",
                "card_image must be a portable path below assets/",
            );
            let extension = card_image.rsplit('.').next().unwrap_or_default();
            validator.require(
                matches!(extension, "png" | "jpg" | "jpeg" | "webp"),
                "unsupported_card_image_type",
                "card_image",
                "card_image must be PNG, JPEG, or WebP",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    pub skills: Vec<SkillDefinition>,
}

impl Validate for SkillFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.skills.is_empty(),
            "empty_file",
            "skills",
            "skills.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.skills, |skill| skill.id.as_str()),
            "skills_not_sorted_unique",
            "skills",
            "skills must be unique and sorted by id",
        );
        for (index, skill) in self.skills.iter().enumerate() {
            validator.merge(&format!("skills[{index}]"), skill.validate());
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLevel {
    pub level: u8,
    pub points: u64,
    pub max_points: u64,
}

pub fn compute_skill_level(
    skill: &SkillDefinition,
    achieved_achievement_ids: &BTreeSet<String>,
) -> SkillLevel {
    let points = skill
        .nodes
        .iter()
        .filter(|node| achieved_achievement_ids.contains(&node.achievement_id))
        .map(|node| node.points)
        .sum();
    let max_points = skill.nodes.iter().map(|node| node.points).sum();
    let level = if points == 0 {
        0
    } else {
        1 + skill
            .level_thresholds
            .iter()
            .take_while(|threshold| points >= **threshold)
            .count() as u8
    };

    SkillLevel {
        level,
        points,
        max_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_achieved_nodes_contribute_to_skill_level() {
        let skill = SkillDefinition {
            id: "cooking::general".to_string(),
            name: "Cooking".to_string(),
            description: None,
            level_thresholds: [10, 20, 30, 40],
            nodes: vec![
                SkillNode {
                    achievement_id: "cooking::first".to_string(),
                    points: 15,
                },
                SkillNode {
                    achievement_id: "cooking::second".to_string(),
                    points: 25,
                },
            ],
            card_image: None,
        };
        let achieved = BTreeSet::from(["cooking::first".to_string()]);
        assert_eq!(compute_skill_level(&skill, &achieved).level, 2);
    }
}
