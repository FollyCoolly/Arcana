use super::achievement_commands::{achievement_availability, AchievementAvailability};
use crate::domain::{
    compute_skill_level, AchievementDefinition, ArcanaRepository, ArcanaRepositoryReader,
    ArcanaRepositoryTransaction, RepositoryResult, SkillDefinition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuerySkills {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillNodeEvaluation {
    pub achievement_id: String,
    pub points: u64,
    pub availability: AchievementAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillEvaluation {
    pub pack_id: String,
    pub definition: SkillDefinition,
    pub points: u64,
    pub max_points: u64,
    pub level: u8,
    pub achieved_node_count: usize,
    pub node_count: usize,
    pub nodes: Vec<SkillNodeEvaluation>,
}

pub struct SkillCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> SkillCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    /// Return every matching Skill supplied by an enabled Pack together with
    /// its current node, point, and level projection. No derived value is
    /// persisted.
    pub fn list(&mut self, query: QuerySkills) -> RepositoryResult<Vec<SkillEvaluation>> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let achievement_states = snapshot
            .achievement_states
            .as_ref()
            .map(|file| &file.states);
        let achieved_ids: BTreeSet<String> = achievement_states
            .into_iter()
            .flat_map(|states| states.iter())
            .filter(|(_, state)| state.is_achieved())
            .map(|(achievement_id, _)| achievement_id.clone())
            .collect();

        let mut evaluations = Vec::new();
        for pack_id in &snapshot.manifest.enabled_pack_ids {
            if query
                .pack_id
                .as_deref()
                .is_some_and(|query_pack_id| query_pack_id != pack_id)
            {
                continue;
            }
            let Some(pack) = snapshot.packs.get(pack_id) else {
                continue;
            };
            let achievement_definitions: BTreeMap<&str, &AchievementDefinition> = pack
                .achievements
                .iter()
                .flat_map(|file| file.achievements.iter())
                .map(|definition| (definition.id.as_str(), definition))
                .collect();

            for definition in pack.skills.iter().flat_map(|file| file.skills.iter()) {
                if query
                    .skill_id
                    .as_deref()
                    .is_some_and(|skill_id| skill_id != definition.id)
                {
                    continue;
                }
                let skill_level = compute_skill_level(definition, &achieved_ids);
                let nodes = definition
                    .nodes
                    .iter()
                    .map(|node| {
                        let availability = achievement_definitions
                            .get(node.achievement_id.as_str())
                            .map(|achievement| {
                                achievement_availability(
                                    achievement,
                                    achievement_states.and_then(|states| {
                                        states.get(node.achievement_id.as_str())
                                    }),
                                    &achieved_ids,
                                )
                                .0
                            })
                            .unwrap_or(AchievementAvailability::Unresolved);
                        SkillNodeEvaluation {
                            achievement_id: node.achievement_id.clone(),
                            points: node.points,
                            availability,
                        }
                    })
                    .collect::<Vec<_>>();
                evaluations.push(SkillEvaluation {
                    pack_id: pack_id.clone(),
                    definition: definition.clone(),
                    points: skill_level.points,
                    max_points: skill_level.max_points,
                    level: skill_level.level,
                    achieved_node_count: nodes
                        .iter()
                        .filter(|node| node.availability == AchievementAvailability::Achieved)
                        .count(),
                    node_count: nodes.len(),
                    nodes,
                });
            }
        }
        evaluations.sort_by(|left, right| left.definition.id.cmp(&right.definition.id));
        transaction.rollback()?;
        Ok(evaluations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{basic_pack, AchievementCommands, SetAchievementState};
    use crate::domain::{
        AchievementDefinition, AchievementDifficulty, AchievementFile, AchievementStatus,
        ArcanaRepositoryTransaction, Pack, PackManifest, SkillFile, SkillNode, SCHEMA_VERSION,
    };
    use crate::storage::sqlite::SqliteRepository;
    use std::collections::BTreeMap;

    fn skill_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "cooking".to_string(),
                name: "Cooking".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: Vec::new(),
            },
            record_definitions: None,
            dimensions: None,
            achievements: Some(AchievementFile {
                achievements: vec![
                    AchievementDefinition {
                        id: "cooking::first_dish".to_string(),
                        name: "First dish".to_string(),
                        description: "Cook one dish".to_string(),
                        difficulty: AchievementDifficulty::Beginner,
                        tags: Vec::new(),
                        prerequisites: Vec::new(),
                        related_record_definition_ids: Vec::new(),
                        tip: None,
                    },
                    AchievementDefinition {
                        id: "cooking::host_dinner".to_string(),
                        name: "Host dinner".to_string(),
                        description: "Host a dinner".to_string(),
                        difficulty: AchievementDifficulty::Intermediate,
                        tags: Vec::new(),
                        prerequisites: vec!["cooking::first_dish".to_string()],
                        related_record_definition_ids: Vec::new(),
                        tip: None,
                    },
                ],
            }),
            skills: Some(SkillFile {
                skills: vec![SkillDefinition {
                    id: "cooking::general".to_string(),
                    name: "Cooking".to_string(),
                    description: None,
                    level_thresholds: [10, 20, 30, 40],
                    nodes: vec![
                        SkillNode {
                            achievement_id: "cooking::first_dish".to_string(),
                            points: 15,
                        },
                        SkillNode {
                            achievement_id: "cooking::host_dinner".to_string(),
                            points: 25,
                        },
                    ],
                    card_image: None,
                }],
            }),
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> SqliteRepository {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(basic_pack()).unwrap();
        transaction.set_pack_enabled("basic", true).unwrap();
        transaction.put_pack(skill_pack()).unwrap();
        transaction.set_pack_enabled("cooking", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn list_derives_node_availability_points_and_level() {
        let mut repository = repository();
        let initial = SkillCommands::new(&mut repository)
            .list(QuerySkills::default())
            .unwrap();
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].points, 0);
        assert_eq!(initial[0].level, 0);
        assert_eq!(
            initial[0].nodes[0].availability,
            AchievementAvailability::Available
        );
        assert_eq!(
            initial[0].nodes[1].availability,
            AchievementAvailability::Locked
        );

        AchievementCommands::new(&mut repository)
            .set_state(SetAchievementState {
                achievement_id: "cooking::first_dish".to_string(),
                status: AchievementStatus::Tracked,
                achieved_at: None,
            })
            .unwrap();
        let tracked = SkillCommands::new(&mut repository)
            .list(QuerySkills::default())
            .unwrap();
        assert_eq!(tracked[0].points, 0);
        assert_eq!(
            tracked[0].nodes[0].availability,
            AchievementAvailability::Tracked
        );

        AchievementCommands::new(&mut repository)
            .set_state(SetAchievementState {
                achievement_id: "cooking::first_dish".to_string(),
                status: AchievementStatus::Achieved,
                achieved_at: None,
            })
            .unwrap();
        let achieved = SkillCommands::new(&mut repository)
            .list(QuerySkills::default())
            .unwrap();
        assert_eq!(achieved[0].points, 15);
        assert_eq!(achieved[0].level, 2);
        assert_eq!(achieved[0].achieved_node_count, 1);
        assert_eq!(
            achieved[0].nodes[1].availability,
            AchievementAvailability::Available
        );
    }

    #[test]
    fn filters_enabled_skills_and_disabled_pack_disappears() {
        let mut repository = repository();
        let matching = SkillCommands::new(&mut repository)
            .list(QuerySkills {
                skill_id: Some("cooking::general".to_string()),
                pack_id: Some("cooking".to_string()),
            })
            .unwrap();
        assert_eq!(matching.len(), 1);
        let missing = SkillCommands::new(&mut repository)
            .list(QuerySkills {
                skill_id: Some("cooking::missing".to_string()),
                pack_id: None,
            })
            .unwrap();
        assert!(missing.is_empty());

        let mut transaction = repository.begin_transaction().unwrap();
        transaction.set_pack_enabled("cooking", false).unwrap();
        transaction.commit().unwrap();
        assert!(SkillCommands::new(&mut repository)
            .list(QuerySkills::default())
            .unwrap()
            .is_empty());
    }
}
