use super::{
    is_sorted_unique, split_record_definition_id, FormulaExpression, Validate, ValidationResult,
    Validator, MAX_EXPRESSION_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedValueDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub expression: String,
}

impl DerivedValueDefinition {
    pub fn parse_expression(&self) -> Result<FormulaExpression, super::ExpressionParseError> {
        FormulaExpression::parse(&self.expression)
    }

    fn merged_with(&self, other: &Self) -> Option<Self> {
        let compatible = self.id == other.id
            && self.name == other.name
            && self.unit == other.unit
            && self.expression == other.expression
            && descriptions_compatible(self.description.as_deref(), other.description.as_deref());
        compatible.then(|| {
            let mut merged = self.clone();
            if merged.description.is_none() {
                merged.description = other.description.clone();
            }
            merged
        })
    }
}

impl Validate for DerivedValueDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_record_definition_id(&self.id).is_some(),
            "invalid_derived_value_id",
            "id",
            "DerivedValue id must be <namespace>.<name> using lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        if let Some(description) = &self.description {
            validator.require_non_blank(description, "description");
        }
        if let Some(unit) = &self.unit {
            validator.require_non_blank(unit, "unit");
        }
        validator.require(
            !self.expression.trim().is_empty(),
            "blank_expression",
            "expression",
            "expression must not be blank",
        );
        validator.require(
            self.expression.len() <= MAX_EXPRESSION_BYTES,
            "expression_too_long",
            "expression",
            "expression must not exceed 2048 bytes",
        );
        if self.expression.len() <= MAX_EXPRESSION_BYTES {
            if let Err(error) = self.parse_expression() {
                validator.error(
                    error.code,
                    "expression",
                    format!("invalid expression: {error}"),
                );
            }
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedValueFile {
    pub values: Vec<DerivedValueDefinition>,
}

impl Validate for DerivedValueFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        for (index, definition) in self.values.iter().enumerate() {
            validator.merge(&format!("values[{index}]"), definition.validate());
        }
        validator.require(
            is_sorted_unique(&self.values, |definition| definition.id.as_str()),
            "derived_values_not_sorted_unique",
            "values",
            "DerivedValues must be unique and sorted by id",
        );
        validate_dag(&mut validator, self);
        validator.finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DerivedValueRegistry {
    definitions: BTreeMap<String, DerivedValueDefinition>,
}

impl DerivedValueRegistry {
    pub fn build<'a>(
        enabled_packs: impl IntoIterator<Item = &'a super::Pack>,
    ) -> Result<Self, super::ValidationErrors> {
        let mut validator = Validator::default();
        let mut definitions = BTreeMap::<String, DerivedValueDefinition>::new();
        for pack in enabled_packs {
            let Some(file) = &pack.derived_values else {
                continue;
            };
            for definition in &file.values {
                match definitions.get(&definition.id) {
                    None => {
                        definitions.insert(definition.id.clone(), definition.clone());
                    }
                    Some(existing) => match existing.merged_with(definition) {
                        Some(merged) => {
                            definitions.insert(definition.id.clone(), merged);
                        }
                        None => validator.error(
                            "derived_value_definition_conflict",
                            format!(
                                "packs.{}.derived-values.{}",
                                pack.manifest.id, definition.id
                            ),
                            "enabled Packs declare incompatible DerivedValues for the same id",
                        ),
                    },
                }
            }
        }
        validator.finish()?;
        Ok(Self { definitions })
    }

    pub fn get(&self, id: &str) -> Option<&DerivedValueDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &DerivedValueDefinition)> {
        self.definitions
            .iter()
            .map(|(id, definition)| (id.as_str(), definition))
    }
}

fn descriptions_compatible(left: Option<&str>, right: Option<&str>) -> bool {
    left.is_none() || right.is_none() || left == right
}

fn validate_dag(validator: &mut Validator, file: &DerivedValueFile) {
    let known: BTreeSet<&str> = file.values.iter().map(|value| value.id.as_str()).collect();
    let dependencies: BTreeMap<String, Vec<String>> = file
        .values
        .iter()
        .map(|value| {
            let references = value
                .parse_expression()
                .ok()
                .into_iter()
                .flat_map(|expression| {
                    expression
                        .derived_value_references()
                        .filter(|id| known.contains(id))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .collect();
            (value.id.clone(), references)
        })
        .collect();
    let mut states = BTreeMap::<String, u8>::new();
    for definition in &file.values {
        let mut stack = vec![(definition.id.clone(), false)];
        while let Some((id, exiting)) = stack.pop() {
            if exiting {
                states.insert(id, 2);
                continue;
            }
            match states.get(&id).copied().unwrap_or(0) {
                1 => {
                    validator.error(
                        "derived_value_cycle",
                        "values",
                        format!("DerivedValue dependency graph contains a cycle through '{id}'"),
                    );
                    return;
                }
                2 => continue,
                _ => {}
            }
            states.insert(id.clone(), 1);
            stack.push((id.clone(), true));
            if let Some(children) = dependencies.get(&id) {
                stack.extend(children.iter().rev().map(|child| (child.clone(), false)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cycles() {
        let file = DerivedValueFile {
            values: vec![
                DerivedValueDefinition {
                    id: "test.a".to_string(),
                    name: "A".to_string(),
                    description: None,
                    unit: None,
                    expression: "derived('test.b')".to_string(),
                },
                DerivedValueDefinition {
                    id: "test.b".to_string(),
                    name: "B".to_string(),
                    description: None,
                    unit: None,
                    expression: "derived('test.a')".to_string(),
                },
            ],
        };
        assert!(file.validate().is_err());
    }
}
