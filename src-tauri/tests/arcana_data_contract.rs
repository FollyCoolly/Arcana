use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn arcana_data(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arcana-data"))
        .args(args)
        .output()
        .expect("arcana-data process must start")
}

fn utf8(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("CLI output must be UTF-8")
}

fn parse_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("CLI output must be JSON")
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn capabilities_succeeds_without_runtime_and_compact_only_changes_layout() {
    let pretty = arcana_data(&["capabilities"]);
    assert!(pretty.status.success(), "{}", utf8(&pretty.stderr));
    assert!(pretty.stderr.is_empty());
    let value = parse_json(&pretty.stdout);
    assert_eq!(value["contract_version"], 1);
    assert_eq!(value["features"]["structured_errors"], true);
    assert_eq!(value["commands"]["pack"]["version"], 1);
    assert_eq!(value["commands"]["status"]["version"], 1);
    assert!(utf8(&pretty.stdout).lines().count() > 1);

    let compact = arcana_data(&["--compact", "capabilities"]);
    assert!(compact.status.success(), "{}", utf8(&compact.stderr));
    assert_eq!(parse_json(&compact.stdout), value);
    assert_eq!(utf8(&compact.stdout).lines().count(), 1);
}

#[test]
fn invalid_and_removed_commands_return_structured_json_errors() {
    for command in ["does-not-exist", "context", "changelog"] {
        let output = arcana_data(&[command]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = parse_json(&output.stderr);
        assert_eq!(error["code"], "invalid_invocation");
        assert!(error["message"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(error["details"], serde_json::json!({}));
    }
}

#[test]
fn invalid_record_json_is_structured_and_does_not_change_sqlite() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    let initialized = arcana_data(&["init", "--runtime", &runtime_arg]);
    assert!(
        initialized.status.success(),
        "{}",
        utf8(&initialized.stderr)
    );
    assert!(runtime.join("arcana.sqlite3").is_file());

    let input = directory.path().join("invalid.json");
    std::fs::write(&input, "{not json").unwrap();
    let input_arg = path_string(&input);
    let invalid = arcana_data(&[
        "record",
        "--runtime",
        &runtime_arg,
        "set",
        "--file",
        &input_arg,
    ]);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    let error = parse_json(&invalid.stderr);
    assert_eq!(error["code"], "invalid_command_input");
    assert_eq!(error["details"]["operation"], "set scalar");
    assert_eq!(error["details"]["source"], input_arg);

    let current = arcana_data(&[
        "record",
        "--runtime",
        &runtime_arg,
        "get",
        "identity.nickname",
    ]);
    assert!(current.status.success(), "{}", utf8(&current.stderr));
    assert!(parse_json(&current.stdout)["record"].is_null());
}

#[test]
fn repository_failures_use_stable_error_codes_and_nonzero_exit_status() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let duplicate = arcana_data(&["init", "--runtime", &runtime_arg]);
    assert!(!duplicate.status.success());
    assert!(duplicate.stdout.is_empty());
    let error = parse_json(&duplicate.stderr);
    assert_eq!(error["code"], "runtime_already_initialized");
    assert_eq!(error["details"]["repository_code"], "conflict");
    assert_eq!(error["details"]["validation_issues"], serde_json::json!([]));
}

#[test]
fn record_commands_distinguish_a_missing_runtime_from_a_missing_record() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("missing-runtime");
    let runtime_arg = path_string(&runtime);
    let output = arcana_data(&[
        "record",
        "--runtime",
        &runtime_arg,
        "get",
        "identity.nickname",
    ]);
    assert!(!output.status.success());
    let error = parse_json(&output.stderr);
    assert_eq!(error["code"], "runtime_not_initialized");
    assert_eq!(
        error["details"]["database"],
        path_string(&runtime.join("arcana.sqlite3"))
    );
}

#[test]
fn pack_commands_round_trip_content_assets_and_enabled_state() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let scaffold = arcana_data(&["pack", "scaffold", "cooking", "--name", "Cooking"]);
    assert!(scaffold.status.success(), "{}", utf8(&scaffold.stderr));
    let content = parse_json(&scaffold.stdout);
    assert_eq!(content["manifest"]["id"], "cooking");
    let content_file = directory.path().join("cooking.json");
    std::fs::write(&content_file, &scaffold.stdout).unwrap();
    let content_arg = path_string(&content_file);

    let write = arcana_data(&[
        "pack",
        "--runtime",
        &runtime_arg,
        "write",
        "--file",
        &content_arg,
    ]);
    assert!(write.status.success(), "{}", utf8(&write.stderr));
    assert_eq!(parse_json(&write.stdout)["pack"]["enabled"], false);

    let asset = directory.path().join("note.txt");
    std::fs::write(&asset, "hello").unwrap();
    let asset_arg = path_string(&asset);
    let put = arcana_data(&[
        "pack",
        "--runtime",
        &runtime_arg,
        "asset-put",
        "cooking",
        "assets/note.txt",
        "--file",
        &asset_arg,
    ]);
    assert!(put.status.success(), "{}", utf8(&put.stderr));
    assert_eq!(parse_json(&put.stdout)["asset"]["size_bytes"], 5);

    let enable = arcana_data(&["pack", "--runtime", &runtime_arg, "enable", "cooking"]);
    assert!(enable.status.success(), "{}", utf8(&enable.stderr));
    assert_eq!(parse_json(&enable.stdout)["pack"]["changed"], true);

    let show = arcana_data(&["pack", "--runtime", &runtime_arg, "show", "cooking"]);
    assert!(show.status.success(), "{}", utf8(&show.stderr));
    let shown = parse_json(&show.stdout);
    assert_eq!(shown["pack"]["enabled"], true);
    assert_eq!(shown["pack"]["assets"][0]["path"], "assets/note.txt");
    assert!(shown["pack"].get("asset_bytes").is_none());
}

#[test]
fn invalid_pack_json_and_missing_pack_use_stable_errors() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let malformed = directory.path().join("malformed.json");
    std::fs::write(&malformed, "{not json").unwrap();
    let malformed_arg = path_string(&malformed);
    let invalid = arcana_data(&[
        "pack",
        "--runtime",
        &runtime_arg,
        "write",
        "--file",
        &malformed_arg,
    ]);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert_eq!(parse_json(&invalid.stderr)["code"], "invalid_command_input");

    let missing = arcana_data(&["pack", "--runtime", &runtime_arg, "show", "missing"]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert_eq!(parse_json(&missing.stderr)["code"], "pack_not_found");
}

#[test]
fn status_commands_evaluate_records_and_preserve_disabled_selection() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let pack = serde_json::json!({
        "manifest": {
            "schema_version": 1,
            "id": "fitness",
            "name": "Fitness"
        },
        "record_definitions": {
            "definitions": [
                {
                    "kind": "scalar",
                    "id": "fitness.endurance",
                    "name": "Endurance",
                    "value_type": "number"
                },
                {
                    "kind": "scalar",
                    "id": "fitness.strength",
                    "name": "Strength",
                    "value_type": "integer"
                }
            ]
        },
        "dimensions": {
            "dimensions": [{
                "id": "fitness::physical",
                "name": "Physical",
                "level_titles": ["Awake", "Growing", "Skilled", "Excellent", "Peak"],
                "level_thresholds": [25, 50, 75, 90],
                "scores": [
                    {
                        "id": "endurance",
                        "name": "Endurance",
                        "weight": 1,
                        "expression": "record('fitness.endurance') * 2"
                    },
                    {
                        "id": "strength",
                        "name": "Strength",
                        "weight": 3,
                        "expression": "record('fitness.strength')"
                    }
                ]
            }]
        }
    });
    let pack_file = directory.path().join("fitness.json");
    std::fs::write(&pack_file, serde_json::to_vec(&pack).unwrap()).unwrap();
    let pack_arg = path_string(&pack_file);
    assert!(arcana_data(&[
        "pack",
        "--runtime",
        &runtime_arg,
        "write",
        "--file",
        &pack_arg,
    ])
    .status
    .success());
    assert!(
        arcana_data(&["pack", "--runtime", &runtime_arg, "enable", "fitness",])
            .status
            .success()
    );

    for (name, value) in [("endurance", 40), ("strength", 50)] {
        let command = serde_json::json!({
            "definition_id": format!("fitness.{name}"),
            "value": value
        });
        let path = directory.path().join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_vec(&command).unwrap()).unwrap();
        let path_arg = path_string(&path);
        let output = arcana_data(&[
            "record",
            "--runtime",
            &runtime_arg,
            "set",
            "--file",
            &path_arg,
        ]);
        assert!(output.status.success(), "{}", utf8(&output.stderr));
    }

    let select = arcana_data(&[
        "status",
        "--runtime",
        &runtime_arg,
        "select",
        "1",
        "fitness::physical",
    ]);
    assert!(select.status.success(), "{}", utf8(&select.stderr));
    assert_eq!(parse_json(&select.stdout)["selection"]["changed"], true);

    let evaluated = arcana_data(&[
        "status",
        "--runtime",
        &runtime_arg,
        "evaluate",
        "fitness::physical",
    ]);
    assert!(evaluated.status.success(), "{}", utf8(&evaluated.stderr));
    let evaluation = &parse_json(&evaluated.stdout)["evaluations"][0];
    assert_eq!(evaluation["selected_position"], 1);
    assert_eq!(evaluation["score"], 57.5);
    assert_eq!(evaluation["level"], 3);
    assert_eq!(evaluation["level_title"], "Skilled");

    assert!(
        arcana_data(&["pack", "--runtime", &runtime_arg, "disable", "fitness",])
            .status
            .success()
    );
    let listed = arcana_data(&["status", "--runtime", &runtime_arg, "list-dimensions"]);
    assert!(listed.status.success(), "{}", utf8(&listed.stderr));
    let listing = parse_json(&listed.stdout);
    assert_eq!(listing["dimensions"], serde_json::json!([]));
    assert_eq!(listing["selections"][0]["available"], false);

    let unresolved = arcana_data(&[
        "status",
        "--runtime",
        &runtime_arg,
        "evaluate",
        "fitness::physical",
    ]);
    assert!(!unresolved.status.success());
    assert_eq!(
        parse_json(&unresolved.stderr)["code"],
        "status_dimension_unresolved"
    );
}
