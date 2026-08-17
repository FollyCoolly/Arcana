use super::{
    is_portable_asset_path, is_snake_case_id, split_scoped_id, AchievementFile, DerivedValueFile,
    DimensionFile, FormulaValue, RecordDefinition, RecordDefinitionFile, SkillFile, Validate,
    ValidationResult, Validator, ValueType, LEGACY_PACK_SCHEMA_VERSION, PACK_SCHEMA_VERSION,
    SCHEMA_VERSION,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArcanaManifest {
    pub schema_version: u32,
    pub enabled_pack_ids: Vec<String>,
}

impl Validate for ArcanaManifest {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            self.schema_version == SCHEMA_VERSION,
            "unsupported_repository_schema",
            "schema_version",
            "repository schema_version is not supported",
        );
        validator.require(
            self.enabled_pack_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
            "enabled_packs_not_sorted_unique",
            "enabled_pack_ids",
            "enabled Pack ids must be unique and sorted",
        );
        for (index, id) in self.enabled_pack_ids.iter().enumerate() {
            validator.require(
                is_snake_case_id(id),
                "invalid_pack_id",
                &format!("enabled_pack_ids[{index}]"),
                "Pack id must use lowercase snake_case",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Validate for PackManifest {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            matches!(
                self.schema_version,
                LEGACY_PACK_SCHEMA_VERSION | PACK_SCHEMA_VERSION
            ),
            "unsupported_pack_schema",
            "schema_version",
            "Pack schema_version is not supported",
        );
        validator.require(
            is_snake_case_id(&self.id),
            "invalid_pack_id",
            "id",
            "Pack id must use lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        for (value, path) in [(&self.description, "description"), (&self.author, "author")] {
            if let Some(value) = value {
                validator.require_non_blank(value, path);
            }
        }
        if let Some(parent_id) = &self.parent_pack_id {
            validator.require(
                is_snake_case_id(parent_id),
                "invalid_parent_pack_id",
                "parent_pack_id",
                "parent Pack id must use lowercase snake_case",
            );
            validator.require(
                parent_id != &self.id,
                "pack_self_parent",
                "parent_pack_id",
                "Pack cannot be its own parent",
            );
        }
        validator.require(
            self.tags.windows(2).all(|pair| pair[0] < pair[1]),
            "tags_not_sorted_unique",
            "tags",
            "tags must be unique and sorted",
        );
        for (index, tag) in self.tags.iter().enumerate() {
            validator.require(
                is_snake_case_id(tag),
                "invalid_tag",
                &format!("tags[{index}]"),
                "tag must use lowercase snake_case",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub manifest: PackManifest,
    pub record_definitions: Option<RecordDefinitionFile>,
    pub derived_values: Option<DerivedValueFile>,
    pub dimensions: Option<DimensionFile>,
    pub achievements: Option<AchievementFile>,
    pub skills: Option<SkillFile>,
    pub assets: BTreeMap<String, Vec<u8>>,
}

impl Validate for Pack {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.merge("manifest", self.manifest.validate());
        if let Some(file) = &self.record_definitions {
            validator.merge("record-definitions.json", file.validate());
        }
        if let Some(file) = &self.derived_values {
            validator.merge("derived-values.json", file.validate());
            validator.require(
                self.manifest.schema_version >= PACK_SCHEMA_VERSION,
                "derived_values_require_pack_schema_v2",
                "manifest.schema_version",
                "derived-values.json requires Pack schema_version 2",
            );
        }
        if let Some(file) = &self.dimensions {
            validator.merge("dimensions.json", file.validate());
        }
        if let Some(file) = &self.achievements {
            validator.merge("achievements.json", file.validate());
        }
        if let Some(file) = &self.skills {
            validator.merge("skills.json", file.validate());
        }

        let definition_ids: BTreeSet<&str> = self
            .record_definitions
            .iter()
            .flat_map(|file| file.definitions.iter())
            .map(RecordDefinition::id)
            .collect();
        let achievement_ids: BTreeSet<&str> = self
            .achievements
            .iter()
            .flat_map(|file| file.achievements.iter())
            .map(|achievement| achievement.id.as_str())
            .collect();
        let derived_value_ids: BTreeSet<&str> = self
            .derived_values
            .iter()
            .flat_map(|file| file.values.iter())
            .map(|value| value.id.as_str())
            .collect();

        if let Some(file) = &self.derived_values {
            for (index, value) in file.values.iter().enumerate() {
                if let Ok(expression) = value.parse_expression() {
                    for definition_id in expression.record_references() {
                        let definition = self
                            .record_definitions
                            .iter()
                            .flat_map(|file| file.definitions.iter())
                            .find(|definition| definition.id() == definition_id);
                        match definition {
                            None => validator.error(
                                "derived_record_definition_missing",
                                format!("derived-values.json.values[{index}].expression"),
                                format!(
                                    "RecordDefinition '{definition_id}' must be fully declared by this Pack"
                                ),
                            ),
                            Some(RecordDefinition::Scalar(definition)) => validator.require(
                                matches!(
                                    definition.value_type,
                                    ValueType::Number | ValueType::Integer | ValueType::Date
                                ),
                                "derived_record_definition_type_unsupported",
                                &format!("derived-values.json.values[{index}].expression"),
                                "DerivedValue expression may reference only number, integer, or date scalar definitions",
                            ),
                            Some(_) => validator.error(
                                "derived_record_definition_not_scalar",
                                format!("derived-values.json.values[{index}].expression"),
                                "DerivedValue expression may reference only scalar definitions",
                            ),
                        }
                    }
                    for derived_id in expression.derived_value_references() {
                        validator.require(
                            derived_value_ids.contains(derived_id),
                            "derived_value_reference_missing",
                            &format!("derived-values.json.values[{index}].expression"),
                            "referenced DerivedValue must be fully declared by this Pack",
                        );
                    }
                    if let Err(error) = expression.evaluate(
                        |id| sample_formula_record_value(self, id),
                        |_| Some(1.0),
                        NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid sample date"),
                    ) {
                        validator.error(
                            "derived_expression_type_error",
                            format!("derived-values.json.values[{index}].expression"),
                            format!("DerivedValue expression is not type-safe: {error}"),
                        );
                    }
                }
            }
        }

        if let Some(file) = &self.dimensions {
            for (index, dimension) in file.dimensions.iter().enumerate() {
                require_owned_scoped_id(
                    &mut validator,
                    &self.manifest.id,
                    &dimension.id,
                    &format!("dimensions.json.dimensions[{index}].id"),
                );
                for (score_index, score) in dimension.scores.iter().enumerate() {
                    if let Ok(expression) = score.parse_expression() {
                        for definition_id in expression.record_references() {
                            let definition = self
                                .record_definitions
                                .iter()
                                .flat_map(|file| file.definitions.iter())
                                .find(|definition| definition.id() == definition_id);
                            match definition {
                                None => validator.error(
                                    "score_record_definition_missing",
                                    format!(
                                        "dimensions.json.dimensions[{index}].scores[{score_index}].expression"
                                    ),
                                    format!(
                                        "RecordDefinition '{definition_id}' must be fully declared by this Pack"
                                    ),
                                ),
                                Some(definition) => validator.require(
                                    definition.is_numeric_scalar(),
                                    "score_record_definition_not_numeric_scalar",
                                    &format!(
                                        "dimensions.json.dimensions[{index}].scores[{score_index}].expression"
                                    ),
                                    "Status expression may only reference number/integer scalar definitions",
                                ),
                            }
                        }
                        for derived_id in expression.derived_value_references() {
                            validator.require(
                                derived_value_ids.contains(derived_id),
                                "score_derived_value_missing",
                                &format!(
                                    "dimensions.json.dimensions[{index}].scores[{score_index}].expression"
                                ),
                                "referenced DerivedValue must be fully declared by this Pack",
                            );
                        }
                        if let Err(error) = expression.evaluate(
                            |id| sample_formula_record_value(self, id),
                            |_| Some(1.0),
                            NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid sample date"),
                        ) {
                            validator.error(
                                "score_expression_type_error",
                                format!(
                                    "dimensions.json.dimensions[{index}].scores[{score_index}].expression"
                                ),
                                format!("Status Score expression is not type-safe: {error}"),
                            );
                        }
                    }
                }
            }
        }

        if let Some(file) = &self.achievements {
            for (index, achievement) in file.achievements.iter().enumerate() {
                let base = format!("achievements.json.achievements[{index}]");
                require_owned_scoped_id(
                    &mut validator,
                    &self.manifest.id,
                    &achievement.id,
                    &format!("{base}.id"),
                );
                for (prerequisite_index, prerequisite) in
                    achievement.prerequisites.iter().enumerate()
                {
                    require_owned_scoped_id(
                        &mut validator,
                        &self.manifest.id,
                        prerequisite,
                        &format!("{base}.prerequisites[{prerequisite_index}]"),
                    );
                    validator.require(
                        achievement_ids.contains(prerequisite.as_str()),
                        "achievement_prerequisite_missing",
                        &format!("{base}.prerequisites[{prerequisite_index}]"),
                        "prerequisite must reference an Achievement in the same Pack",
                    );
                }
                for (related_index, definition_id) in
                    achievement.related_record_definition_ids.iter().enumerate()
                {
                    validator.require(
                        definition_ids.contains(definition_id.as_str()),
                        "related_record_definition_missing",
                        &format!("{base}.related_record_definition_ids[{related_index}]"),
                        "related RecordDefinition must be fully declared by this Pack",
                    );
                }
            }
            validate_achievement_dag(&mut validator, file);
        }

        if let Some(file) = &self.skills {
            for (index, skill) in file.skills.iter().enumerate() {
                let base = format!("skills.json.skills[{index}]");
                require_owned_scoped_id(
                    &mut validator,
                    &self.manifest.id,
                    &skill.id,
                    &format!("{base}.id"),
                );
                for (node_index, node) in skill.nodes.iter().enumerate() {
                    require_owned_scoped_id(
                        &mut validator,
                        &self.manifest.id,
                        &node.achievement_id,
                        &format!("{base}.nodes[{node_index}].achievement_id"),
                    );
                    validator.require(
                        achievement_ids.contains(node.achievement_id.as_str()),
                        "skill_achievement_missing",
                        &format!("{base}.nodes[{node_index}].achievement_id"),
                        "Skill node must reference an Achievement in the same Pack",
                    );
                }
                if let Some(card_image) = &skill.card_image {
                    match self.assets.get(card_image) {
                        Some(content) => validator.require(
                            valid_card_image_bytes(card_image, content),
                            "card_image_content_mismatch",
                            &format!("{base}.card_image"),
                            "card image bytes do not match the supported file type",
                        ),
                        None => validator.error(
                            "card_image_missing",
                            format!("{base}.card_image"),
                            "card image does not exist in Pack assets",
                        ),
                    }
                }
            }
        }

        for path in self.assets.keys() {
            validator.require(
                is_portable_asset_path(path),
                "invalid_asset_path",
                &format!("assets.{path}"),
                "asset path is not portable across Windows and macOS",
            );
        }

        validator.finish()
    }
}

fn sample_formula_record_value(pack: &Pack, id: &str) -> Option<FormulaValue> {
    let definition = pack
        .record_definitions
        .iter()
        .flat_map(|file| file.definitions.iter())
        .find(|definition| definition.id() == id)?;
    match definition {
        RecordDefinition::Scalar(definition) => match definition.value_type {
            ValueType::Number | ValueType::Integer => Some(FormulaValue::Number(1.0)),
            ValueType::Date => Some(FormulaValue::Date(
                NaiveDate::from_ymd_opt(2000, 1, 1).expect("valid sample date"),
            )),
            _ => None,
        },
        RecordDefinition::Collection(_) | RecordDefinition::Event(_) => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefinitionRegistry {
    definitions: BTreeMap<String, RecordDefinition>,
}

impl DefinitionRegistry {
    pub fn build<'a>(
        enabled_packs: impl IntoIterator<Item = &'a Pack>,
    ) -> Result<Self, super::ValidationErrors> {
        let mut validator = Validator::default();
        let mut definitions: BTreeMap<String, RecordDefinition> = BTreeMap::new();

        for pack in enabled_packs {
            let Some(file) = &pack.record_definitions else {
                continue;
            };
            for definition in &file.definitions {
                match definitions.get(definition.id()) {
                    None => {
                        definitions.insert(definition.id().to_string(), definition.clone());
                    }
                    Some(existing) => match existing.merged_with(definition) {
                        Some(merged) => {
                            definitions.insert(definition.id().to_string(), merged);
                        }
                        None => validator.error(
                            "record_definition_conflict",
                            format!(
                                "packs.{}.record-definitions.{}",
                                pack.manifest.id,
                                definition.id()
                            ),
                            "enabled Packs declare incompatible definitions for the same id",
                        ),
                    },
                }
            }
        }

        validator.finish()?;
        Ok(Self { definitions })
    }

    pub fn get(&self, definition_id: &str) -> Option<&RecordDefinition> {
        self.definitions.get(definition_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &RecordDefinition)> {
        self.definitions
            .iter()
            .map(|(id, definition)| (id.as_str(), definition))
    }
}

fn require_owned_scoped_id(validator: &mut Validator, pack_id: &str, id: &str, path: &str) {
    validator.require(
        split_scoped_id(id).is_some_and(|(owner, _)| owner == pack_id),
        "pack_id_prefix_mismatch",
        path,
        "scoped id prefix must equal manifest.id",
    );
}

fn validate_achievement_dag(validator: &mut Validator, file: &AchievementFile) {
    let prerequisites: BTreeMap<&str, &[String]> = file
        .achievements
        .iter()
        .map(|achievement| {
            (
                achievement.id.as_str(),
                achievement.prerequisites.as_slice(),
            )
        })
        .collect();
    let mut states: BTreeMap<&str, u8> = BTreeMap::new();
    for achievement in &file.achievements {
        let mut stack = vec![(achievement.id.as_str(), false)];
        while let Some((id, exiting)) = stack.pop() {
            if exiting {
                states.insert(id, 2);
                continue;
            }
            match states.get(id).copied().unwrap_or(0) {
                1 => {
                    validator.error(
                        "achievement_prerequisite_cycle",
                        "achievements.json.achievements",
                        format!("prerequisite graph contains a cycle through '{id}'"),
                    );
                    return;
                }
                2 => continue,
                _ => {}
            }
            states.insert(id, 1);
            stack.push((id, true));
            if let Some(parents) = prerequisites.get(id) {
                stack.extend(parents.iter().rev().map(|parent| (parent.as_str(), false)));
            }
        }
    }
}

fn valid_card_image_bytes(path: &str, content: &[u8]) -> bool {
    match path.rsplit('.').next().unwrap_or_default() {
        "png" => content.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => content.starts_with(&[0xff, 0xd8, 0xff]),
        "webp" => content.len() >= 12 && &content[..4] == b"RIFF" && &content[8..12] == b"WEBP",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AchievementDefinition, AchievementDifficulty, DerivedValueDefinition, FieldDefinition,
        ScalarRecordDefinition, SkillDefinition, SkillNode, StructuredRecordDefinition, ValueType,
    };

    fn base_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "cooking".to_string(),
                name: "Cooking".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: vec![],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![RecordDefinition::Collection(StructuredRecordDefinition {
                    id: "cooking.learned_dishes".to_string(),
                    name: "Dishes".to_string(),
                    description: None,
                    fields: BTreeMap::from([(
                        "name".to_string(),
                        FieldDefinition {
                            value_type: ValueType::String,
                            required: true,
                            unit: None,
                        },
                    )]),
                })],
            }),
            derived_values: None,
            dimensions: None,
            achievements: Some(AchievementFile {
                achievements: vec![AchievementDefinition {
                    id: "cooking::first_dish".to_string(),
                    name: "First dish".to_string(),
                    description: "Cook one dish".to_string(),
                    difficulty: AchievementDifficulty::Beginner,
                    tags: vec![],
                    prerequisites: vec![],
                    related_record_definition_ids: vec!["cooking.learned_dishes".to_string()],
                    tip: None,
                }],
            }),
            skills: Some(SkillFile {
                skills: vec![SkillDefinition {
                    id: "cooking::general".to_string(),
                    name: "Cooking".to_string(),
                    description: None,
                    level_thresholds: [1, 2, 3, 4],
                    nodes: vec![SkillNode {
                        achievement_id: "cooking::first_dish".to_string(),
                        points: 4,
                    }],
                    card_image: None,
                }],
            }),
            assets: BTreeMap::new(),
        }
    }

    #[test]
    fn valid_pack_checks_internal_references() {
        assert!(base_pack().validate().is_ok());
    }

    #[test]
    fn registry_merges_optional_fields() {
        let left = base_pack();
        let mut right = base_pack();
        right.manifest.id = "recipes".to_string();
        if let Some(RecordDefinition::Collection(definition)) = right
            .record_definitions
            .as_mut()
            .and_then(|file| file.definitions.first_mut())
        {
            definition.fields.insert(
                "learned_at".to_string(),
                FieldDefinition {
                    value_type: ValueType::Date,
                    required: false,
                    unit: None,
                },
            );
        }
        let registry = DefinitionRegistry::build([&left, &right]).unwrap();
        assert_eq!(
            registry
                .get("cooking.learned_dishes")
                .and_then(RecordDefinition::fields)
                .map(BTreeMap::len),
            Some(2)
        );
    }

    #[test]
    fn derived_values_require_v2_and_type_safe_formulas() {
        let mut pack = base_pack();
        pack.derived_values = Some(DerivedValueFile {
            values: vec![DerivedValueDefinition {
                id: "identity.bad_days".to_string(),
                name: "Bad days".to_string(),
                description: None,
                unit: Some("day".to_string()),
                expression: "record('identity.birth_date') + 1".to_string(),
            }],
        });
        pack.record_definitions
            .as_mut()
            .unwrap()
            .definitions
            .push(RecordDefinition::Scalar(ScalarRecordDefinition {
                id: "identity.birth_date".to_string(),
                name: "Birth date".to_string(),
                description: None,
                value_type: ValueType::Date,
                unit: None,
            }));

        let errors = pack.validate().unwrap_err().into_issues();
        assert!(errors
            .iter()
            .any(|issue| issue.code == "derived_values_require_pack_schema_v2"));
        assert!(errors
            .iter()
            .any(|issue| issue.code == "derived_expression_type_error"));
    }
}
