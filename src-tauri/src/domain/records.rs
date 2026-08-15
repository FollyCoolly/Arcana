use super::{
    is_snake_case_id, is_sorted_unique, is_valid_date, parse_rfc3339, split_record_definition_id,
    Validate, ValidationResult, Validator,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const RESERVED_FIELD_NAMES: &[&str] = &[
    "id",
    "definition_id",
    "value",
    "effective_at",
    "recorded_at",
    "items",
    "events",
    "occurred_at",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordKind {
    Scalar,
    Collection,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    String,
    Number,
    Integer,
    Boolean,
    Date,
    Datetime,
}

impl ValueType {
    pub fn accepts(self, value: &Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Number => value.as_f64().is_some_and(f64::is_finite),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Boolean => value.is_boolean(),
            Self::Date => value.as_str().is_some_and(is_valid_date),
            Self::Datetime => value
                .as_str()
                .is_some_and(|value| parse_rfc3339(value).is_some()),
        }
    }

    pub fn supports_unit(self) -> bool {
        matches!(self, Self::Number | Self::Integer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDefinition {
    #[serde(rename = "type")]
    pub value_type: ValueType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl Validate for FieldDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        if let Some(unit) = &self.unit {
            validator.require_non_blank(unit, "unit");
            validator.require(
                self.value_type.supports_unit(),
                "unit_not_supported",
                "unit",
                "unit is only allowed for number and integer fields",
            );
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RecordDefinition {
    Scalar(ScalarRecordDefinition),
    Collection(StructuredRecordDefinition),
    Event(StructuredRecordDefinition),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarRecordDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub value_type: ValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredRecordDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: BTreeMap<String, FieldDefinition>,
}

impl RecordDefinition {
    pub fn id(&self) -> &str {
        match self {
            Self::Scalar(definition) => &definition.id,
            Self::Collection(definition) | Self::Event(definition) => &definition.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Scalar(definition) => &definition.name,
            Self::Collection(definition) | Self::Event(definition) => &definition.name,
        }
    }

    pub fn kind(&self) -> RecordKind {
        match self {
            Self::Scalar(_) => RecordKind::Scalar,
            Self::Collection(_) => RecordKind::Collection,
            Self::Event(_) => RecordKind::Event,
        }
    }

    pub fn fields(&self) -> Option<&BTreeMap<String, FieldDefinition>> {
        match self {
            Self::Scalar(_) => None,
            Self::Collection(definition) | Self::Event(definition) => Some(&definition.fields),
        }
    }

    pub fn is_numeric_scalar(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarRecordDefinition {
                value_type: ValueType::Number | ValueType::Integer,
                ..
            })
        )
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scalar(left), Self::Scalar(right)) => {
                left.id == right.id
                    && left.name == right.name
                    && compatible_description(&left.description, &right.description)
                    && left.value_type == right.value_type
                    && left.unit == right.unit
            }
            (Self::Collection(left), Self::Collection(right))
            | (Self::Event(left), Self::Event(right)) => {
                structured_definitions_compatible(left, right)
            }
            _ => false,
        }
    }

    pub fn merged_with(&self, other: &Self) -> Option<Self> {
        if !self.is_compatible_with(other) {
            return None;
        }
        match (self, other) {
            (Self::Scalar(left), Self::Scalar(right)) => {
                let mut merged = left.clone();
                if merged.description.is_none() {
                    merged.description = right.description.clone();
                }
                Some(Self::Scalar(merged))
            }
            (Self::Collection(left), Self::Collection(right)) => {
                Some(Self::Collection(merge_structured_definitions(left, right)))
            }
            (Self::Event(left), Self::Event(right)) => {
                Some(Self::Event(merge_structured_definitions(left, right)))
            }
            _ => None,
        }
    }

    pub fn validate_value(&self, value: &Value) -> bool {
        match self {
            Self::Scalar(definition) => definition.value_type.accepts(value),
            _ => false,
        }
    }
}

impl Validate for RecordDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_record_definition_id(self.id()).is_some(),
            "invalid_record_definition_id",
            "id",
            "must be <namespace>.<name> using lowercase snake_case",
        );
        validator.require_non_blank(self.name(), "name");

        match self {
            Self::Scalar(definition) => {
                validate_optional_text(&mut validator, &definition.description, "description");
                if let Some(unit) = &definition.unit {
                    validator.require_non_blank(unit, "unit");
                    validator.require(
                        definition.value_type.supports_unit(),
                        "unit_not_supported",
                        "unit",
                        "unit is only allowed for number and integer scalar definitions",
                    );
                }
            }
            Self::Collection(definition) | Self::Event(definition) => {
                validate_optional_text(&mut validator, &definition.description, "description");
                for (name, field) in &definition.fields {
                    validator.require(
                        is_snake_case_id(name),
                        "invalid_field_name",
                        &format!("fields.{name}"),
                        "field name must use lowercase snake_case",
                    );
                    validator.require(
                        !RESERVED_FIELD_NAMES.contains(&name.as_str()),
                        "reserved_field_name",
                        &format!("fields.{name}"),
                        "field name is reserved by the Record envelope",
                    );
                    validator.merge(&format!("fields.{name}"), field.validate());
                }
            }
        }

        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordDefinitionFile {
    pub definitions: Vec<RecordDefinition>,
}

impl Validate for RecordDefinitionFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.definitions.is_empty(),
            "empty_file",
            "definitions",
            "record-definitions.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.definitions, |definition| definition.id()),
            "definitions_not_sorted_unique",
            "definitions",
            "definitions must be unique and sorted by id",
        );
        for (index, definition) in self.definitions.iter().enumerate() {
            validator.merge(&format!("definitions[{index}]"), definition.validate());
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Record {
    Scalar(ScalarRecord),
    Collection(CollectionRecord),
    Event(EventRecord),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarRecord {
    pub definition_id: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_at: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionRecord {
    pub definition_id: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionItem {
    pub id: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRecord {
    pub definition_id: String,
    pub events: Vec<EventEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEntry {
    pub id: String,
    pub occurred_at: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
    pub recorded_at: String,
}

impl Record {
    pub fn definition_id(&self) -> &str {
        match self {
            Self::Scalar(record) => &record.definition_id,
            Self::Collection(record) => &record.definition_id,
            Self::Event(record) => &record.definition_id,
        }
    }

    pub fn kind(&self) -> RecordKind {
        match self {
            Self::Scalar(_) => RecordKind::Scalar,
            Self::Collection(_) => RecordKind::Collection,
            Self::Event(_) => RecordKind::Event,
        }
    }

    pub fn validate_against(&self, definition: &RecordDefinition) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            self.definition_id() == definition.id(),
            "record_definition_mismatch",
            "definition_id",
            "record definition_id does not match the supplied definition",
        );
        validator.require(
            self.kind() == definition.kind(),
            "record_kind_mismatch",
            "definition_id",
            "record payload kind does not match its definition",
        );

        match (self, definition) {
            (Self::Scalar(record), RecordDefinition::Scalar(definition)) => validator.require(
                definition.value_type.accepts(&record.value),
                "record_value_type_mismatch",
                "value",
                "scalar value does not match RecordDefinition.value_type",
            ),
            (Self::Collection(record), RecordDefinition::Collection(definition)) => {
                validate_structured_values(
                    &mut validator,
                    record.items.iter().map(|item| &item.fields),
                    &definition.fields,
                    "items",
                );
            }
            (Self::Event(record), RecordDefinition::Event(definition)) => {
                validate_structured_values(
                    &mut validator,
                    record.events.iter().map(|event| &event.fields),
                    &definition.fields,
                    "events",
                );
            }
            _ => {}
        }

        validator.finish()
    }
}

impl Validate for Record {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_record_definition_id(self.definition_id()).is_some(),
            "invalid_record_definition_id",
            "definition_id",
            "must be <namespace>.<name> using lowercase snake_case",
        );

        match self {
            Self::Scalar(record) => {
                validator.require(
                    is_leaf_value(&record.value),
                    "invalid_scalar_value",
                    "value",
                    "scalar value must be a non-null string, finite number, integer, or boolean",
                );
                if let Some(effective_at) = &record.effective_at {
                    validator.require(
                        is_valid_date(effective_at) || parse_rfc3339(effective_at).is_some(),
                        "invalid_effective_at",
                        "effective_at",
                        "must be YYYY-MM-DD or RFC 3339 with an offset",
                    );
                }
                validate_timestamp(&mut validator, &record.recorded_at, "recorded_at");
            }
            Self::Collection(record) => {
                validator.require(
                    is_sorted_unique(&record.items, |item| item.id.as_str()),
                    "items_not_sorted_unique",
                    "items",
                    "collection items must be unique and sorted by id",
                );
                for (index, item) in record.items.iter().enumerate() {
                    validator.require_non_blank(&item.id, &format!("items[{index}].id"));
                    validate_timestamp(
                        &mut validator,
                        &item.recorded_at,
                        &format!("items[{index}].recorded_at"),
                    );
                }
            }
            Self::Event(record) => {
                let mut ids = BTreeSet::new();
                let mut previous: Option<(&str, &str)> = None;
                for (index, event) in record.events.iter().enumerate() {
                    validator.require_non_blank(&event.id, &format!("events[{index}].id"));
                    validator.require(
                        ids.insert(event.id.as_str()),
                        "duplicate_event_id",
                        &format!("events[{index}].id"),
                        "event id must be unique within a Record",
                    );
                    validate_timestamp(
                        &mut validator,
                        &event.occurred_at,
                        &format!("events[{index}].occurred_at"),
                    );
                    validate_timestamp(
                        &mut validator,
                        &event.recorded_at,
                        &format!("events[{index}].recorded_at"),
                    );
                    let current = (event.occurred_at.as_str(), event.id.as_str());
                    if let Some(previous) = previous {
                        validator.require(
                            previous < current,
                            "events_not_sorted",
                            "events",
                            "events must be sorted by occurred_at and id",
                        );
                    }
                    previous = Some(current);
                }
            }
        }

        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFile {
    pub namespace: String,
    pub records: Vec<Record>,
}

impl Validate for RecordFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            is_snake_case_id(&self.namespace),
            "invalid_namespace",
            "namespace",
            "namespace must use lowercase snake_case",
        );
        validator.require(
            !self.records.is_empty(),
            "empty_file",
            "records",
            "namespace file must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.records, |record| record.definition_id()),
            "records_not_sorted_unique",
            "records",
            "records must be unique and sorted by definition_id",
        );

        for (index, record) in self.records.iter().enumerate() {
            validator.merge(&format!("records[{index}]"), record.validate());
            let record_namespace =
                split_record_definition_id(record.definition_id()).map(|(namespace, _)| namespace);
            validator.require(
                record_namespace == Some(self.namespace.as_str()),
                "record_namespace_mismatch",
                &format!("records[{index}].definition_id"),
                "definition_id namespace must match the containing file",
            );
        }

        validator.finish()
    }
}

fn compatible_description(left: &Option<String>, right: &Option<String>) -> bool {
    left == right || left.is_none() || right.is_none()
}

fn structured_definitions_compatible(
    left: &StructuredRecordDefinition,
    right: &StructuredRecordDefinition,
) -> bool {
    if left.id != right.id
        || left.name != right.name
        || !compatible_description(&left.description, &right.description)
    {
        return false;
    }

    for (name, left_field) in &left.fields {
        match right.fields.get(name) {
            Some(right_field) if left_field != right_field => return false,
            None if left_field.required => return false,
            _ => {}
        }
    }
    for (name, right_field) in &right.fields {
        if !left.fields.contains_key(name) && right_field.required {
            return false;
        }
    }
    true
}

fn merge_structured_definitions(
    left: &StructuredRecordDefinition,
    right: &StructuredRecordDefinition,
) -> StructuredRecordDefinition {
    let mut merged = left.clone();
    if merged.description.is_none() {
        merged.description = right.description.clone();
    }
    for (name, field) in &right.fields {
        merged
            .fields
            .entry(name.clone())
            .or_insert_with(|| field.clone());
    }
    merged
}

fn validate_optional_text(validator: &mut Validator, value: &Option<String>, path: &str) {
    if let Some(value) = value {
        validator.require_non_blank(value, path);
    }
}

fn validate_timestamp(validator: &mut Validator, value: &str, path: &str) {
    validator.require(
        parse_rfc3339(value).is_some(),
        "invalid_datetime",
        path,
        "must be RFC 3339 with a timezone offset",
    );
}

fn is_leaf_value(value: &Value) -> bool {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => false,
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::Bool(_) | Value::String(_) => true,
    }
}

fn validate_structured_values<'a>(
    validator: &mut Validator,
    values: impl Iterator<Item = &'a BTreeMap<String, Value>>,
    fields: &BTreeMap<String, FieldDefinition>,
    path: &str,
) {
    for (index, values) in values.enumerate() {
        for (name, definition) in fields {
            match values.get(name) {
                Some(value) => validator.require(
                    definition.value_type.accepts(value),
                    "record_field_type_mismatch",
                    &format!("{path}[{index}].{name}"),
                    "field value does not match its definition",
                ),
                None => validator.require(
                    !definition.required,
                    "required_record_field_missing",
                    &format!("{path}[{index}].{name}"),
                    "required field is missing",
                ),
            }
        }
        for name in values.keys() {
            validator.require(
                fields.contains_key(name),
                "unknown_record_field",
                &format!("{path}[{index}].{name}"),
                "field is not declared by the RecordDefinition",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn collection_definition(optional_extra: bool) -> RecordDefinition {
        let mut fields = BTreeMap::from([(
            "name".to_string(),
            FieldDefinition {
                value_type: ValueType::String,
                required: true,
                unit: None,
            },
        )]);
        if optional_extra {
            fields.insert(
                "learned_at".to_string(),
                FieldDefinition {
                    value_type: ValueType::Date,
                    required: false,
                    unit: None,
                },
            );
        }
        RecordDefinition::Collection(StructuredRecordDefinition {
            id: "cooking.learned_dishes".to_string(),
            name: "Learned dishes".to_string(),
            description: None,
            fields,
        })
    }

    #[test]
    fn compatible_definition_can_add_optional_field() {
        assert!(collection_definition(false).is_compatible_with(&collection_definition(true)));
    }

    #[test]
    fn record_is_checked_against_definition() {
        let record = Record::Collection(CollectionRecord {
            definition_id: "cooking.learned_dishes".to_string(),
            items: vec![CollectionItem {
                id: "dish:eggs".to_string(),
                fields: BTreeMap::from([("name".to_string(), json!("Eggs"))]),
                recorded_at: "2026-08-15T20:30:00+08:00".to_string(),
            }],
        });
        assert!(record.validate().is_ok());
        assert!(record
            .validate_against(&collection_definition(true))
            .is_ok());
    }

    #[test]
    fn record_file_requires_namespace_match() {
        let file = RecordFile {
            namespace: "fitness".to_string(),
            records: vec![Record::Scalar(ScalarRecord {
                definition_id: "health.weight".to_string(),
                value: json!(72.5),
                effective_at: None,
                recorded_at: "2026-08-15T20:30:00+08:00".to_string(),
            })],
        };
        assert!(file.validate().is_err());
    }

    #[test]
    fn strict_scalar_definition_rejects_unknown_fields() {
        let value = json!({
            "id": "health.weight",
            "name": "Weight",
            "kind": "scalar",
            "value_type": "number",
            "unexpected": true
        });
        assert!(serde_json::from_value::<RecordDefinition>(value).is_err());
    }
}
