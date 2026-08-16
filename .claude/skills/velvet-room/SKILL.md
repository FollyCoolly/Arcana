---
name: velvet-room
description: Record user-provided activities, facts, corrections, mission progress, Achievement states, Status selections, and durable AssistantMemory in Arcana. Use when a user reports what they did, asks to update or correct Arcana progress, says an Achievement was completed or mistakenly unlocked, or wants a lasting preference or reminder saved.
---

# Velvet Room

Translate the user's explicit account into the smallest truthful Arcana update. Treat `arcana-data` as the only data access path during a Skill run; never edit SQLite or the live JSON repository directly.

## Prepare

1. Resolve `arcana-data` from `PATH`. Inside the Arcana repository, fall back to the built binary for the current platform or `cargo run --manifest-path src-tauri/Cargo.toml --bin arcana-data --`.
2. Run `arcana-data --compact capabilities` before the first write. Require `contract_version: 1`, `structured_errors`, `dry_run`, and `batch`; refuse writes on an unknown contract.
3. Read [references/cli-contract.md](references/cli-contract.md) before composing payloads.
4. Run `context summary`, then use targeted queries for the entities implicated by the user's report. Do not load every Pack or Record without a reason.

## Decide What Is True

Separate the input into:

- explicit facts that can become Record mutations;
- explicit lifecycle requests for Missions or Achievement states;
- candidate Achievement completions requiring Definition text, related Records, and optional `tip`;
- durable cross-session knowledge suitable for AssistantMemory;
- ambiguity that must remain unwritten until clarified.

Never invent dates, quantities, event fields, past history, or completion states. A user's direct statement that an Achievement is complete is sufficient even without tracking Records. Conversely, a Record update does not automatically prove every related Achievement.

Use `tracked` only when the user is actively following or gathering information for an Achievement. It is not numeric progress and contributes no Skill points.

## Build the Update

1. Query the current Record, Mission, Suggestion, Achievement, or Memory before mutating it.
2. Use correction operations for corrections. Before adding a collection item or event, check for an existing stable item/event ID or an equivalent recorded fact.
3. Put related Record mutations in one `batch apply` payload; this is the only multi-operation atomic batch and it commits inside SQLite.
4. Dry-run Mission, Suggestion, Achievement, Memory, or Status mutations individually. These entities use the live or runtime-local JSON stores and cannot share a batch with Records or with each other.
5. If material ambiguity remains, show it and wait. Otherwise apply the Record batch first, then each approved JSON mutation. Stop and report precisely if a later command fails; do not claim cross-store atomicity.
6. Re-query the affected entities and summarize what actually changed.

System-generated UUIDs and timestamps in a dry-run are provisional. Never copy a preview ID into a later operation; the committed run generates it again.

## Achievement Handling

After relevant Record changes, query Achievements by `--related-record-definition-id`. Judge completion from the natural-language Definition, available facts, and optional `tip`; Arcana has no hidden auto-unlock rule.

Do not revoke an achieved state merely because related Record data later changes. Use `achievement.state-revoke` only for an explicit correction or accidental unlock. If the Definition is unavailable, surface the unresolved state rather than guessing.

## Memory Discipline

Create or update AssistantMemory only for knowledge likely to matter across sessions: stable preferences, constraints, habits, focus, refined summaries, reminders, or observations. Query first and update an existing semantic match instead of appending a near-duplicate.

Do not store full chat logs, credentials, transient mood, generated task batches, or facts already represented by Records/Missions/Achievement states.

## Failure Rules

Use process exit status before reading output. On failure, read structured JSON from stderr and expose `code`, relevant `details`, and validation issues. Never parse `message` as a control signal, retry a conflict blindly, skip validation, or partially replay a failed Record batch.
