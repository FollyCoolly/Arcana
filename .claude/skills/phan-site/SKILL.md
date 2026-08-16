---
name: phan-site
description: Generate, review, accept, reject, or remove Arcana MissionSuggestions from active Missions, tracked Achievements, Records, and durable user context. Use when a user asks what to do next, wants quest-style recommendations, or explicitly manages an existing Arcana mission suggestion.
---

# Phan Site

Generate useful MissionSuggestions without turning unaccepted AI output into synchronized Missions. Use `arcana-data` as the only data access path.

## Prepare

1. Resolve `arcana-data` from `PATH`; inside the repository, fall back to the current-platform debug binary or `cargo run --manifest-path src-tauri/Cargo.toml --bin arcana-data --`.
2. Run `arcana-data --compact capabilities`. Require contract v1 plus structured errors and dry-run before writing.
3. Read [references/mission-contract.md](references/mission-contract.md) before composing a suggestion.
4. Read `context summary`, active Missions, pending/rejected suggestions, tracked Achievements, and only the Records or Definitions needed for plausible candidates.

## Generate Candidates

Produce 3-5 candidates unless the user requests another count. Prefer this mix when the context supports it:

- concrete next steps for active Missions;
- achievable milestones suggested by tracked or available Achievements;
- one exploratory task grounded in durable focus or preferences.

Make each candidate outcome-based, realistically scoped, and distinguishable from existing or rejected suggestions. Use `parent_mission_id` only for a real active parent Mission. Use `reason` to explain the relevant context, not hidden model reasoning.

Do not fabricate user interests, deadlines, Achievement completion, Record values, or a need for urgency. Do not create filler merely to reach the requested count.

## Persist Suggestions

1. Query existing suggestions and deduplicate semantically before writing.
2. Dry-run each `mission suggest` command. Resolve validation errors or factual ambiguity before writing any candidate.
3. Apply each confirmed suggestion with the same input. Treat preview IDs and timestamps as provisional.
4. Stop and report clearly if a later suggestion fails; already-created suggestions remain valid because JSON-backed MissionSuggestions are not a multi-operation SQLite batch.
5. Re-query pending suggestions and present their actual IDs, difficulty, deadline, parent, and reason.

Never use `mission.create` for a generated recommendation. A Suggestion becomes a synchronized active Mission only through `mission.accept` after the user explicitly accepts it.

## Manage Existing Suggestions

On an explicit user choice, call `mission.accept`, `mission.reject`, or `mission.suggestion-delete`. Accepting a rejected Suggestion is allowed if the user changed their mind. Rejection is idempotent and remains local for future deduplication.

Do not automatically reject older pending suggestions, store generation batches in AssistantMemory, or write complete prompts/session history. Update Memory only when the user reveals a genuinely durable preference or constraint, and use the Velvet Room discipline for that separate mutation.

## Handle Failures

Check process status first. Read structured stderr on failure and surface stable `code` plus relevant `details`. Never blindly replay a failed command or silently replace a missing parent, invalid deadline, conflict, or unresolved reference.
