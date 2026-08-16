use arcana_lib::domain::{RepositoryError, RepositoryErrorCode, SCHEMA_VERSION};
use arcana_lib::storage::sqlite::DATABASE_SCHEMA_VERSION;
use serde::Serialize;
use serde_json::{json, Value};

pub const CONTRACT_VERSION: u32 = 1;
pub const EXIT_SUCCESS: i32 = 0;
const EXIT_DOMAIN_ERROR: i32 = 1;
const EXIT_INPUT_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 3;

#[derive(Debug, Clone, Copy)]
pub enum RepositoryOperation {
    Initialize,
    Record,
    Pack,
    PackAsset,
    JsonImport,
    JsonExport,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CliError {
    pub code: String,
    pub message: String,
    pub details: Value,
    #[serde(skip)]
    exit_code: i32,
}

impl CliError {
    pub fn invalid_invocation(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_invocation".to_string(),
            message: message.into(),
            details: json!({}),
            exit_code: EXIT_INPUT_ERROR,
        }
    }

    pub fn invalid_command_input(
        operation: &str,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        let mut merged = match details {
            Value::Object(object) => object,
            _ => serde_json::Map::new(),
        };
        merged.insert("operation".to_string(), json!(operation));
        Self {
            code: "invalid_command_input".to_string(),
            message: message.into(),
            details: Value::Object(merged),
            exit_code: EXIT_INPUT_ERROR,
        }
    }

    pub fn runtime_not_initialized(database: &std::path::Path) -> Self {
        Self {
            code: "runtime_not_initialized".to_string(),
            message: format!(
                "Arcana runtime database does not exist: {}",
                database.display()
            ),
            details: json!({ "database": database }),
            exit_code: EXIT_DOMAIN_ERROR,
        }
    }

    pub fn from_repository(error: RepositoryError, operation: RepositoryOperation) -> Self {
        let code = repository_error_code(error.code, operation);
        let exit_code = match error.code {
            RepositoryErrorCode::Busy | RepositoryErrorCode::Storage => EXIT_RUNTIME_ERROR,
            _ => EXIT_DOMAIN_ERROR,
        };
        Self {
            code: code.to_string(),
            message: error.message,
            details: json!({
                "repository_code": error.code,
                "validation_issues": error.validation_issues
            }),
            exit_code,
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

fn repository_error_code(
    code: RepositoryErrorCode,
    operation: RepositoryOperation,
) -> &'static str {
    match code {
        RepositoryErrorCode::Busy => "runtime_busy",
        RepositoryErrorCode::Storage => "storage_error",
        RepositoryErrorCode::ValidationFailed => "validation_failed",
        RepositoryErrorCode::NotFound => match operation {
            RepositoryOperation::Initialize | RepositoryOperation::JsonExport => {
                "runtime_not_initialized"
            }
            RepositoryOperation::Record => "record_not_found",
            RepositoryOperation::Pack => "pack_not_found",
            RepositoryOperation::PackAsset => "pack_asset_not_found",
            RepositoryOperation::JsonImport => "json_import_not_found",
        },
        RepositoryErrorCode::Conflict => match operation {
            RepositoryOperation::Initialize => "runtime_already_initialized",
            RepositoryOperation::Record => "record_conflict",
            RepositoryOperation::Pack => "pack_conflict",
            RepositoryOperation::PackAsset => "pack_asset_conflict",
            RepositoryOperation::JsonImport => "json_import_conflict",
            RepositoryOperation::JsonExport => "json_export_conflict",
        },
        RepositoryErrorCode::Unresolved => match operation {
            RepositoryOperation::Record => "record_unresolved",
            RepositoryOperation::Pack => "pack_unresolved",
            RepositoryOperation::PackAsset => "pack_asset_unresolved",
            RepositoryOperation::Initialize
            | RepositoryOperation::JsonImport
            | RepositoryOperation::JsonExport => "unresolved_reference",
        },
    }
}

pub fn capabilities() -> Value {
    json!({
        "contract_version": CONTRACT_VERSION,
        "repository_schema_version": SCHEMA_VERSION,
        "pack_schema_version": SCHEMA_VERSION,
        "sqlite_schema_version": DATABASE_SCHEMA_VERSION,
        "commands": {
            "init": { "version": 1 },
            "record": {
                "version": 1,
                "actions": [
                    "get",
                    "query",
                    "set",
                    "increment",
                    "correct",
                    "create-empty-collection",
                    "create-empty-event",
                    "add-item",
                    "correct-item",
                    "remove-item",
                    "append-event",
                    "correct-event",
                    "delete-event",
                    "delete"
                ]
            },
            "pack": {
                "version": 1,
                "actions": [
                    "list",
                    "show",
                    "scaffold",
                    "validate",
                    "write",
                    "asset-put",
                    "asset-delete",
                    "enable",
                    "disable"
                ]
            },
            "json": {
                "version": 1,
                "actions": ["import", "export"]
            }
        },
        "features": {
            "structured_errors": true,
            "dry_run": false,
            "batch": false,
            "git_sync": false
        }
    })
}

pub fn render_json(value: &Value, compact: bool) -> String {
    if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .expect("serde_json::Value serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::domain::{RepositoryError, ValidationIssue};

    #[test]
    fn capabilities_are_available_without_a_runtime() {
        let value = capabilities();
        assert_eq!(value["contract_version"], CONTRACT_VERSION);
        assert_eq!(value["commands"]["record"]["version"], 1);
        assert_eq!(value["features"]["dry_run"], false);
    }

    #[test]
    fn repository_errors_have_stable_codes_and_structured_details() {
        let mut error =
            RepositoryError::new(RepositoryErrorCode::Unresolved, "definition is unavailable");
        error.validation_issues.push(ValidationIssue {
            code: "missing_definition".to_string(),
            path: "definition_id".to_string(),
            message: "missing".to_string(),
        });
        let error = CliError::from_repository(error, RepositoryOperation::Record);
        assert_eq!(error.code, "record_unresolved");
        assert_eq!(error.exit_code(), EXIT_DOMAIN_ERROR);
        assert_eq!(error.details["repository_code"], "unresolved");
        assert_eq!(
            error.details["validation_issues"][0]["path"],
            "definition_id"
        );
    }

    #[test]
    fn compact_only_changes_json_whitespace() {
        let value = json!({"record": {"value": 3}});
        let pretty = render_json(&value, false);
        let compact = render_json(&value, true);
        assert!(pretty.contains('\n'));
        assert!(!compact.contains('\n'));
        assert_eq!(
            serde_json::from_str::<Value>(&pretty).unwrap(),
            serde_json::from_str::<Value>(&compact).unwrap()
        );
    }
}
