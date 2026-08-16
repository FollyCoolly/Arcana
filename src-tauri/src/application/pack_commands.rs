use crate::domain::{
    AchievementFile, ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction,
    DimensionFile, Pack, PackManifest, RecordDefinitionFile, RepositoryError, RepositoryErrorCode,
    RepositoryResult, SkillFile, Validate, SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Structured Pack content accepted by the application layer.
///
/// Assets are deliberately excluded: callers must manage their bytes through
/// `put_asset` and `delete_asset`, while a content write preserves every
/// existing asset in the Pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackContent {
    pub manifest: PackManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_definitions: Option<RecordDefinitionFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<DimensionFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievements: Option<AchievementFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<SkillFile>,
}

impl PackContent {
    pub fn scaffold(id: String, name: String) -> RepositoryResult<Self> {
        let content = Self {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id,
                name,
                description: None,
                author: None,
                parent_pack_id: None,
                tags: Vec::new(),
            },
            record_definitions: None,
            dimensions: None,
            achievements: None,
            skills: None,
        };
        content.to_pack(BTreeMap::new()).validate()?;
        Ok(content)
    }

    fn from_pack(pack: &Pack) -> Self {
        Self {
            manifest: pack.manifest.clone(),
            record_definitions: pack.record_definitions.clone(),
            dimensions: pack.dimensions.clone(),
            achievements: pack.achievements.clone(),
            skills: pack.skills.clone(),
        }
    }

    fn to_pack(&self, assets: BTreeMap<String, Vec<u8>>) -> Pack {
        Pack {
            manifest: self.manifest.clone(),
            record_definitions: self.record_definitions.clone(),
            dimensions: self.dimensions.clone(),
            achievements: self.achievements.clone(),
            skills: self.skills.clone(),
            assets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_pack_id: Option<String>,
    pub tags: Vec<String>,
    pub record_definition_count: usize,
    pub dimension_count: usize,
    pub achievement_count: usize,
    pub skill_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackAssetSummary {
    pub path: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackDetails {
    pub enabled: bool,
    pub content: PackContent,
    pub assets: Vec<PackAssetSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_parent_pack_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackValidation {
    pub pack_id: String,
    pub valid: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_parent_pack_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackEnabledState {
    pub pack_id: String,
    pub enabled: bool,
    pub changed: bool,
}

pub struct PackCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> PackCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&self) -> RepositoryResult<Vec<PackSummary>> {
        let snapshot = self.repository.load_synced_snapshot()?;
        Ok(snapshot
            .packs
            .values()
            .map(|pack| {
                PackSummary::from_pack(
                    pack,
                    snapshot
                        .manifest
                        .enabled_pack_ids
                        .binary_search(&pack.manifest.id)
                        .is_ok(),
                )
            })
            .collect())
    }

    pub fn show(&self, pack_id: &str) -> RepositoryResult<PackDetails> {
        let snapshot = self.repository.load_synced_snapshot()?;
        let pack = snapshot
            .packs
            .get(pack_id)
            .ok_or_else(|| pack_not_found(pack_id))?;
        Ok(PackDetails::from_pack(
            pack,
            snapshot
                .manifest
                .enabled_pack_ids
                .binary_search(&pack.manifest.id)
                .is_ok(),
            &snapshot.packs,
        ))
    }

    /// Validate a candidate exactly as `write` would, while preserving current
    /// assets and without opening a write transaction.
    pub fn validate(&self, content: PackContent) -> RepositoryResult<PackValidation> {
        let mut snapshot = self.repository.load_synced_snapshot()?;
        let pack_id = content.manifest.id.clone();
        let enabled = snapshot
            .manifest
            .enabled_pack_ids
            .binary_search(&pack_id)
            .is_ok();
        let assets = snapshot
            .packs
            .get(&pack_id)
            .map(|pack| pack.assets.clone())
            .unwrap_or_default();
        let candidate = content.to_pack(assets);
        candidate.validate()?;
        snapshot.packs.insert(pack_id.clone(), candidate);
        snapshot.validate()?;
        let missing_parent_pack_id = snapshot.packs[&pack_id]
            .manifest
            .parent_pack_id
            .clone()
            .filter(|parent_id| !snapshot.packs.contains_key(parent_id));
        Ok(PackValidation {
            pack_id,
            valid: true,
            enabled,
            missing_parent_pack_id,
        })
    }

    /// Insert or replace structured Pack content while preserving enabled state
    /// and all existing asset bytes.
    pub fn write(&mut self, content: PackContent) -> RepositoryResult<PackDetails> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let pack_id = content.manifest.id.clone();
        let enabled = snapshot
            .manifest
            .enabled_pack_ids
            .binary_search(&pack_id)
            .is_ok();
        let assets = snapshot
            .packs
            .get(&pack_id)
            .map(|pack| pack.assets.clone())
            .unwrap_or_default();
        let pack = content.to_pack(assets);
        let details = PackDetails::from_pack(&pack, enabled, &snapshot.packs);
        transaction.put_pack(pack)?;
        transaction.commit()?;
        Ok(details)
    }

    pub fn put_asset(
        &mut self,
        pack_id: &str,
        asset_path: String,
        content: Vec<u8>,
    ) -> RepositoryResult<PackAssetSummary> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mut pack = snapshot
            .packs
            .get(pack_id)
            .cloned()
            .ok_or_else(|| pack_not_found(pack_id))?;
        pack.assets.insert(asset_path.clone(), content);
        let summary = pack
            .assets
            .get(&asset_path)
            .map(|content| PackAssetSummary {
                path: asset_path,
                size_bytes: content.len(),
            })
            .expect("asset was inserted");
        transaction.put_pack(pack)?;
        transaction.commit()?;
        Ok(summary)
    }

    pub fn delete_asset(&mut self, pack_id: &str, asset_path: &str) -> RepositoryResult<()> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mut pack = snapshot
            .packs
            .get(pack_id)
            .cloned()
            .ok_or_else(|| pack_not_found(pack_id))?;
        if pack.assets.remove(asset_path).is_none() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("Pack asset '{pack_id}/{asset_path}' was not found"),
            ));
        }
        transaction.put_pack(pack)?;
        transaction.commit()
    }

    pub fn set_enabled(
        &mut self,
        pack_id: &str,
        enabled: bool,
    ) -> RepositoryResult<PackEnabledState> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        if !snapshot.packs.contains_key(pack_id) {
            return Err(pack_not_found(pack_id));
        }
        let currently_enabled = snapshot
            .manifest
            .enabled_pack_ids
            .binary_search_by(|id| id.as_str().cmp(pack_id))
            .is_ok();
        if currently_enabled == enabled {
            transaction.rollback()?;
            return Ok(PackEnabledState {
                pack_id: pack_id.to_string(),
                enabled,
                changed: false,
            });
        }
        transaction.set_pack_enabled(pack_id, enabled)?;
        transaction.commit()?;
        Ok(PackEnabledState {
            pack_id: pack_id.to_string(),
            enabled,
            changed: true,
        })
    }
}

impl PackSummary {
    fn from_pack(pack: &Pack, enabled: bool) -> Self {
        Self {
            id: pack.manifest.id.clone(),
            name: pack.manifest.name.clone(),
            enabled,
            parent_pack_id: pack.manifest.parent_pack_id.clone(),
            tags: pack.manifest.tags.clone(),
            record_definition_count: pack
                .record_definitions
                .as_ref()
                .map(|file| file.definitions.len())
                .unwrap_or_default(),
            dimension_count: pack
                .dimensions
                .as_ref()
                .map(|file| file.dimensions.len())
                .unwrap_or_default(),
            achievement_count: pack
                .achievements
                .as_ref()
                .map(|file| file.achievements.len())
                .unwrap_or_default(),
            skill_count: pack
                .skills
                .as_ref()
                .map(|file| file.skills.len())
                .unwrap_or_default(),
            asset_count: pack.assets.len(),
        }
    }
}

impl PackDetails {
    fn from_pack(pack: &Pack, enabled: bool, packs: &BTreeMap<String, Pack>) -> Self {
        Self {
            enabled,
            content: PackContent::from_pack(pack),
            assets: pack
                .assets
                .iter()
                .map(|(path, content)| PackAssetSummary {
                    path: path.clone(),
                    size_bytes: content.len(),
                })
                .collect(),
            missing_parent_pack_id: pack
                .manifest
                .parent_pack_id
                .clone()
                .filter(|parent_id| !packs.contains_key(parent_id)),
        }
    }
}

fn pack_not_found(pack_id: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::NotFound,
        format!("Pack '{pack_id}' was not found"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::basic_pack;
    use crate::domain::{
        AchievementDefinition, AchievementDifficulty, AchievementFile, ArcanaRepositoryTransaction,
        RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, SkillDefinition, SkillFile,
        SkillNode, ValueType,
    };
    use crate::storage::sqlite::SqliteRepository;

    fn repository() -> SqliteRepository {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(basic_pack()).unwrap();
        transaction.set_pack_enabled("basic", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    fn content(id: &str) -> PackContent {
        PackContent {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: id.to_string(),
                name: "Test Pack".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: Vec::new(),
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "stats.count".to_string(),
                    name: "Count".to_string(),
                    description: None,
                    value_type: ValueType::Integer,
                    unit: None,
                })],
            }),
            dimensions: None,
            achievements: None,
            skills: None,
        }
    }

    #[test]
    fn validates_without_writing_then_writes_disabled_pack() {
        let mut repository = repository();
        let mut commands = PackCommands::new(&mut repository);
        let validation = commands.validate(content("stats")).unwrap();
        assert!(validation.valid);
        assert!(!validation.enabled);
        assert!(commands.show("stats").is_err());

        let details = commands.write(content("stats")).unwrap();
        assert!(!details.enabled);
        assert_eq!(commands.list().unwrap().len(), 2);
        assert!(!commands.show("stats").unwrap().enabled);
    }

    #[test]
    fn content_write_preserves_assets_and_enable_is_idempotent() {
        let mut repository = repository();
        let mut commands = PackCommands::new(&mut repository);
        commands.write(content("stats")).unwrap();
        commands
            .put_asset("stats", "assets/note.txt".to_string(), b"hello".to_vec())
            .unwrap();

        let mut renamed = content("stats");
        renamed.manifest.name = "Renamed".to_string();
        let details = commands.write(renamed).unwrap();
        assert_eq!(details.assets[0].path, "assets/note.txt");
        assert!(commands.set_enabled("stats", true).unwrap().changed);
        assert!(!commands.set_enabled("stats", true).unwrap().changed);
    }

    #[test]
    fn deleting_referenced_asset_is_rejected() {
        let mut repository = repository();
        let mut commands = PackCommands::new(&mut repository);
        let mut content = content("stats");
        content.achievements = Some(AchievementFile {
            achievements: vec![AchievementDefinition {
                id: "stats::first_count".to_string(),
                name: "First count".to_string(),
                description: "Record the first count".to_string(),
                difficulty: AchievementDifficulty::Beginner,
                tags: Vec::new(),
                prerequisites: Vec::new(),
                related_record_definition_ids: vec!["stats.count".to_string()],
                tip: None,
            }],
        });
        content.skills = Some(SkillFile {
            skills: vec![SkillDefinition {
                id: "stats::general".to_string(),
                name: "Stats".to_string(),
                description: None,
                level_thresholds: [1, 2, 3, 4],
                nodes: vec![SkillNode {
                    achievement_id: "stats::first_count".to_string(),
                    points: 4,
                }],
                card_image: None,
            }],
        });
        commands.write(content.clone()).unwrap();
        commands
            .put_asset(
                "stats",
                "assets/card.png".to_string(),
                b"\x89PNG\r\n\x1a\n".to_vec(),
            )
            .unwrap();
        content.skills.as_mut().unwrap().skills[0].card_image = Some("assets/card.png".to_string());
        commands.write(content).unwrap();

        let error = commands
            .delete_asset("stats", "assets/card.png")
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert_eq!(commands.show("stats").unwrap().assets.len(), 1);
    }

    #[test]
    fn scaffold_uses_current_pack_schema() {
        let scaffold = PackContent::scaffold("cooking".to_string(), "Cooking".to_string()).unwrap();
        assert_eq!(scaffold.manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(scaffold.manifest.id, "cooking");
    }

    #[test]
    fn validate_checks_pack_forest_against_current_repository() {
        let mut repository = repository();
        let mut commands = PackCommands::new(&mut repository);
        let mut first = content("first");
        first.manifest.parent_pack_id = Some("second".to_string());
        let written = commands.write(first).unwrap();
        assert_eq!(written.missing_parent_pack_id.as_deref(), Some("second"));

        let mut second = content("second");
        second.manifest.parent_pack_id = Some("first".to_string());
        let error = commands.validate(second).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert!(error
            .validation_issues
            .iter()
            .any(|issue| issue.code == "pack_parent_cycle"));
    }

    #[test]
    fn enabling_incompatible_definition_rolls_back() {
        let mut repository = repository();
        let mut commands = PackCommands::new(&mut repository);
        let mut conflicting = content("conflicting");
        conflicting.record_definitions = Some(RecordDefinitionFile {
            definitions: vec![RecordDefinition::Scalar(ScalarRecordDefinition {
                id: "identity.nickname".to_string(),
                name: "Numeric nickname".to_string(),
                description: None,
                value_type: ValueType::Integer,
                unit: None,
            })],
        });
        commands.write(conflicting).unwrap();

        let error = commands.set_enabled("conflicting", true).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert!(!commands.show("conflicting").unwrap().enabled);
    }
}
