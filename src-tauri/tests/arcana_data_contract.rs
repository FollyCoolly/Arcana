use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

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
    assert_eq!(value["commands"]["achievement"]["version"], 1);
    assert_eq!(value["commands"]["skill"]["version"], 1);
    assert_eq!(value["commands"]["mission"]["version"], 1);
    assert_eq!(value["commands"]["memory"]["version"], 1);
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

#[test]
fn achievement_commands_derive_availability_and_revoke_unresolved_state() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let pack = serde_json::json!({
        "manifest": {
            "schema_version": 1,
            "id": "cooking",
            "name": "Cooking"
        },
        "record_definitions": {
            "definitions": [{
                "kind": "scalar",
                "id": "cooking.dish_count",
                "name": "Dish count",
                "value_type": "integer"
            }]
        },
        "achievements": {
            "achievements": [
                {
                    "id": "cooking::first_dish",
                    "name": "First dish",
                    "description": "Cook one dish",
                    "difficulty": "beginner",
                    "related_record_definition_ids": ["cooking.dish_count"]
                },
                {
                    "id": "cooking::host_dinner",
                    "name": "Host dinner",
                    "description": "Host a dinner",
                    "difficulty": "intermediate",
                    "prerequisites": ["cooking::first_dish"]
                }
            ]
        },
        "skills": {
            "skills": [{
                "id": "cooking::general",
                "name": "Cooking",
                "level_thresholds": [10, 20, 30, 40],
                "nodes": [
                    {
                        "achievement_id": "cooking::first_dish",
                        "points": 15
                    },
                    {
                        "achievement_id": "cooking::host_dinner",
                        "points": 25
                    }
                ]
            }]
        }
    });
    let pack_file = directory.path().join("cooking.json");
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
        arcana_data(&["pack", "--runtime", &runtime_arg, "enable", "cooking",])
            .status
            .success()
    );

    let initial = arcana_data(&[
        "achievement",
        "--runtime",
        &runtime_arg,
        "list",
        "--pack",
        "cooking",
    ]);
    assert!(initial.status.success(), "{}", utf8(&initial.stderr));
    let initial = parse_json(&initial.stdout);
    assert_eq!(initial["achievements"][0]["availability"], "available");
    assert_eq!(initial["achievements"][1]["availability"], "locked");

    let state = serde_json::json!({
        "achievement_id": "cooking::host_dinner",
        "status": "achieved",
        "achieved_at": "2026-08"
    });
    let state_file = directory.path().join("achieved.json");
    std::fs::write(&state_file, serde_json::to_vec(&state).unwrap()).unwrap();
    let state_arg = path_string(&state_file);
    let set = arcana_data(&[
        "achievement",
        "--runtime",
        &runtime_arg,
        "state-set",
        "--file",
        &state_arg,
    ]);
    assert!(set.status.success(), "{}", utf8(&set.stderr));
    assert_eq!(
        parse_json(&set.stdout)["achievement_state"]["changed"],
        true
    );

    let achieved = arcana_data(&[
        "achievement",
        "--runtime",
        &runtime_arg,
        "list",
        "--status",
        "achieved",
    ]);
    assert!(achieved.status.success(), "{}", utf8(&achieved.stderr));
    let achieved = parse_json(&achieved.stdout);
    assert_eq!(achieved["achievements"].as_array().unwrap().len(), 1);
    assert_eq!(
        achieved["achievements"][0]["achievement_id"],
        "cooking::host_dinner"
    );

    let skills = arcana_data(&[
        "skill",
        "--runtime",
        &runtime_arg,
        "list",
        "--skill-id",
        "cooking::general",
    ]);
    assert!(skills.status.success(), "{}", utf8(&skills.stderr));
    let skills = parse_json(&skills.stdout);
    assert_eq!(skills["skills"].as_array().unwrap().len(), 1);
    assert_eq!(skills["skills"][0]["points"], 25);
    assert_eq!(skills["skills"][0]["max_points"], 40);
    assert_eq!(skills["skills"][0]["level"], 3);
    assert_eq!(skills["skills"][0]["achieved_node_count"], 1);
    assert_eq!(skills["skills"][0]["nodes"][0]["availability"], "available");
    assert_eq!(skills["skills"][0]["nodes"][1]["availability"], "achieved");

    assert!(
        arcana_data(&["pack", "--runtime", &runtime_arg, "disable", "cooking",])
            .status
            .success()
    );
    let unresolved = arcana_data(&["achievement", "--runtime", &runtime_arg, "list"]);
    assert!(unresolved.status.success(), "{}", utf8(&unresolved.stderr));
    let unresolved = parse_json(&unresolved.stdout);
    assert_eq!(unresolved["achievements"].as_array().unwrap().len(), 1);
    assert_eq!(unresolved["achievements"][0]["availability"], "unresolved");
    let disabled_skills = arcana_data(&["skill", "--runtime", &runtime_arg, "list"]);
    assert!(
        disabled_skills.status.success(),
        "{}",
        utf8(&disabled_skills.stderr)
    );
    assert_eq!(
        parse_json(&disabled_skills.stdout)["skills"],
        serde_json::json!([])
    );

    let revoke = arcana_data(&[
        "achievement",
        "--runtime",
        &runtime_arg,
        "state-revoke",
        "cooking::host_dinner",
    ]);
    assert!(revoke.status.success(), "{}", utf8(&revoke.stderr));
    assert_eq!(
        parse_json(&revoke.stdout)["achievement_state"]["changed"],
        true
    );
    let repeated = arcana_data(&[
        "achievement",
        "--runtime",
        &runtime_arg,
        "state-revoke",
        "cooking::host_dinner",
    ]);
    assert!(repeated.status.success(), "{}", utf8(&repeated.stderr));
    assert_eq!(
        parse_json(&repeated.stdout)["achievement_state"]["changed"],
        false
    );
}

#[test]
fn mission_commands_cover_accepted_and_local_suggestion_lifecycles() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let create = serde_json::json!({
        "title": "Read Rust Book",
        "description": "Finish every chapter",
        "progress": 20,
        "difficulty": "B",
        "deadline": "2026-12-31"
    });
    let create_file = directory.path().join("create-mission.json");
    std::fs::write(&create_file, serde_json::to_vec(&create).unwrap()).unwrap();
    let create_arg = path_string(&create_file);
    let created = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "create",
        "--file",
        &create_arg,
    ]);
    assert!(created.status.success(), "{}", utf8(&created.stderr));
    let created = parse_json(&created.stdout);
    assert_eq!(created["changed"], true);
    let mission_id = created["mission"]["id"].as_str().unwrap().to_string();
    assert_eq!(Uuid::parse_str(&mission_id).unwrap().get_version_num(), 7);
    assert_eq!(created["mission"]["status"], "active");

    let update = serde_json::json!({
        "mission_id": mission_id,
        "title": "Read and practice Rust",
        "progress": 60,
        "difficulty": "A"
    });
    let update_file = directory.path().join("update-mission.json");
    std::fs::write(&update_file, serde_json::to_vec(&update).unwrap()).unwrap();
    let update_arg = path_string(&update_file);
    let updated = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "update",
        "--file",
        &update_arg,
    ]);
    assert!(updated.status.success(), "{}", utf8(&updated.stderr));
    let updated = parse_json(&updated.stdout);
    assert_eq!(updated["mission"]["title"], "Read and practice Rust");
    assert!(updated["mission"].get("description").is_none());
    assert!(updated["mission"].get("deadline").is_none());

    let completed = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "complete",
        &mission_id,
    ]);
    assert!(completed.status.success(), "{}", utf8(&completed.stderr));
    let completed = parse_json(&completed.stdout);
    assert_eq!(completed["mission"]["status"], "completed");
    assert_eq!(completed["mission"]["progress"], 100);
    assert!(completed["mission"]["completed_at"].is_string());
    let repeated = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "complete",
        &mission_id,
    ]);
    assert!(repeated.status.success(), "{}", utf8(&repeated.stderr));
    assert_eq!(parse_json(&repeated.stdout)["changed"], false);

    let archived = arcana_data(&["mission", "--runtime", &runtime_arg, "archive", &mission_id]);
    assert!(archived.status.success(), "{}", utf8(&archived.stderr));
    assert_eq!(
        parse_json(&archived.stdout)["mission"]["status"],
        "archived"
    );

    let suggestion = serde_json::json!({
        "title": "Try Rustlings",
        "difficulty": "C",
        "reason": "Practice the ownership model"
    });
    let suggestion_file = directory.path().join("suggest-mission.json");
    std::fs::write(&suggestion_file, serde_json::to_vec(&suggestion).unwrap()).unwrap();
    let suggestion_arg = path_string(&suggestion_file);
    let suggested = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "suggest",
        "--file",
        &suggestion_arg,
    ]);
    assert!(suggested.status.success(), "{}", utf8(&suggested.stderr));
    let suggested = parse_json(&suggested.stdout);
    let suggestion_id = suggested["suggestion"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        Uuid::parse_str(&suggestion_id).unwrap().get_version_num(),
        7
    );
    assert_eq!(suggested["suggestion"]["status"], "pending");

    let rejected = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "reject",
        &suggestion_id,
    ]);
    assert!(rejected.status.success(), "{}", utf8(&rejected.stderr));
    assert_eq!(
        parse_json(&rejected.stdout)["suggestion"]["status"],
        "rejected"
    );

    let accepted = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "accept",
        &suggestion_id,
    ]);
    assert!(accepted.status.success(), "{}", utf8(&accepted.stderr));
    let accepted = parse_json(&accepted.stdout);
    assert_eq!(accepted["mission"]["id"], suggestion_id);
    assert_eq!(accepted["mission"]["status"], "active");

    let suggestions = arcana_data(&["mission", "--runtime", &runtime_arg, "suggestion-list"]);
    assert!(
        suggestions.status.success(),
        "{}",
        utf8(&suggestions.stderr)
    );
    assert_eq!(
        parse_json(&suggestions.stdout)["suggestions"],
        serde_json::json!([])
    );

    let filtered = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "list",
        "--status",
        "active",
    ]);
    assert!(filtered.status.success(), "{}", utf8(&filtered.stderr));
    let filtered = parse_json(&filtered.stdout);
    assert_eq!(filtered["missions"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["missions"][0]["id"], suggestion_id);

    let missing = arcana_data(&[
        "mission",
        "--runtime",
        &runtime_arg,
        "reject",
        "missing-suggestion",
    ]);
    assert!(!missing.status.success());
    assert_eq!(
        parse_json(&missing.stderr)["code"],
        "mission_suggestion_not_found"
    );
}

#[test]
fn memory_commands_preserve_identity_and_export_synchronized_entries() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = directory.path().join("runtime");
    let runtime_arg = path_string(&runtime);
    assert!(arcana_data(&["init", "--runtime", &runtime_arg])
        .status
        .success());

    let create = serde_json::json!({
        "kind": "reminder",
        "content": "Ask about cooking history next time"
    });
    let create_file = directory.path().join("create-memory.json");
    std::fs::write(&create_file, serde_json::to_vec(&create).unwrap()).unwrap();
    let create_arg = path_string(&create_file);
    let created = arcana_data(&[
        "memory",
        "--runtime",
        &runtime_arg,
        "create",
        "--file",
        &create_arg,
    ]);
    assert!(created.status.success(), "{}", utf8(&created.stderr));
    let created = parse_json(&created.stdout);
    let memory_id = created["memory"]["id"].as_str().unwrap().to_string();
    assert_eq!(Uuid::parse_str(&memory_id).unwrap().get_version_num(), 7);
    assert_eq!(created["changed"], true);
    assert_eq!(
        created["memory"]["created_at"],
        created["memory"]["updated_at"]
    );
    let created_at = created["memory"]["created_at"]
        .as_str()
        .unwrap()
        .to_string();

    let listed = arcana_data(&[
        "memory",
        "--runtime",
        &runtime_arg,
        "list",
        "--kind",
        "reminder",
    ]);
    assert!(listed.status.success(), "{}", utf8(&listed.stderr));
    assert_eq!(
        parse_json(&listed.stdout)["memories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let update = serde_json::json!({
        "memory_id": memory_id,
        "kind": "observation",
        "content": "Cooking history may unlock achievements"
    });
    let update_file = directory.path().join("update-memory.json");
    std::fs::write(&update_file, serde_json::to_vec(&update).unwrap()).unwrap();
    let update_arg = path_string(&update_file);
    let updated = arcana_data(&[
        "memory",
        "--runtime",
        &runtime_arg,
        "update",
        "--file",
        &update_arg,
    ]);
    assert!(updated.status.success(), "{}", utf8(&updated.stderr));
    let updated = parse_json(&updated.stdout);
    assert_eq!(updated["memory"]["created_at"], created_at);
    assert_eq!(updated["memory"]["kind"], "observation");
    let updated_at = updated["memory"]["updated_at"]
        .as_str()
        .unwrap()
        .to_string();

    let repeated = arcana_data(&[
        "memory",
        "--runtime",
        &runtime_arg,
        "update",
        "--file",
        &update_arg,
    ]);
    assert!(repeated.status.success(), "{}", utf8(&repeated.stderr));
    let repeated = parse_json(&repeated.stdout);
    assert_eq!(repeated["changed"], false);
    assert_eq!(repeated["memory"]["updated_at"], updated_at);

    let export = directory.path().join("export");
    let export_arg = path_string(&export);
    let exported = arcana_data(&[
        "json",
        "export",
        "--runtime",
        &runtime_arg,
        "--output",
        &export_arg,
    ]);
    assert!(exported.status.success(), "{}", utf8(&exported.stderr));
    let exported_memory = serde_json::from_slice::<Value>(
        &std::fs::read(export.join("assistant-memory.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(exported_memory["memories"][0]["id"], memory_id);
    assert_eq!(exported_memory["memories"][0]["kind"], "observation");

    let deleted = arcana_data(&["memory", "--runtime", &runtime_arg, "delete", &memory_id]);
    assert!(deleted.status.success(), "{}", utf8(&deleted.stderr));
    assert_eq!(parse_json(&deleted.stdout)["deleted"], true);
    let missing = arcana_data(&["memory", "--runtime", &runtime_arg, "delete", &memory_id]);
    assert!(!missing.status.success());
    assert_eq!(parse_json(&missing.stderr)["code"], "memory_not_found");
}
