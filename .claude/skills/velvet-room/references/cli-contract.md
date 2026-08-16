# Velvet Room CLI contract v1

## Command prefix

Use `arcana-data` below as a placeholder for the resolved executable. Let Arcana choose its configured runtime unless the user explicitly supplies another runtime directory.

Before writes:

```text
arcana-data --compact capabilities
arcana-data context summary
```

Use the published [capabilities fixture](../../../fixtures/contract-v1/capabilities.json) and [Record batch fixture](../../../fixtures/contract-v1/batch-all-operations.json) instead of reconstructing field names from memory.

## Targeted reads

```text
arcana-data record get <definition_id>
arcana-data record query --definition-id <id>
arcana-data record query --namespace <namespace>
arcana-data record query --pack <pack_id>
arcana-data achievement list --achievement-id <pack::id>
arcana-data achievement list --status tracked
arcana-data achievement list --related-record-definition-id <id>
arcana-data mission list --mission-id <id>
arcana-data mission list --status active
arcana-data mission suggestion-list --status pending
arcana-data memory list --memory-id <id>
arcana-data memory list --kind preference
```

All filters on one query are combined with AND.

## Record inputs

Scalar set or correction:

```json
{
  "definition_id": "identity.nickname",
  "value": "Alice",
  "effective_at": "2026-08-16"
}
```

Use `record.set` for a newly stated current value and `record.correct` when correcting a prior entry. Both have the same payload. Omit `effective_at` when unknown; the CLI supplies `recorded_at`.

Numeric increment:

```json
{
  "definition_id": "fitness.run_count",
  "delta": 1,
  "effective_at": "2026-08-16"
}
```

Collection item:

```json
{
  "definition_id": "cooking.learned_dishes",
  "item_id": "dish:tomato-and-eggs",
  "fields": {
    "learned_at": "2026-08-16",
    "name": "Tomato and eggs"
  }
}
```

Use `record.add-item`, `record.correct-item`, or `record.remove-item`. For removal, send only `definition_id` and `item_id`.

Event:

```json
{
  "definition_id": "fitness.running",
  "event_id": "run:2026-08-16-morning",
  "occurred_at": "2026-08-16T07:30:00+08:00",
  "fields": {
    "distance_km": 5.2,
    "duration_minutes": 31
  }
}
```

Use `record.append-event`, `record.correct-event`, or `record.delete-event`. Use a stable caller-supplied item/event ID and query the Record before adding it.

## Achievement, Mission, and Memory inputs

Achievement state:

```json
{
  "achievement_id": "cooking::first_dish",
  "status": "achieved",
  "achieved_at": "2026-08"
}
```

`status` is `tracked` or `achieved`. `achieved_at` accepts `YYYY`, `YYYY-MM`, or `YYYY-MM-DD` and is allowed only for `achieved`. Revocation uses the dedicated `achievement state-revoke` command.

Mission creation:

```json
{
  "title": "Review today's notes",
  "description": "Extract three durable takeaways.",
  "progress": 0,
  "difficulty": "D",
  "deadline": "2026-08-23",
  "parent_id": "existing-mission-id"
}
```

All fields except `title` are optional. Difficulty is `S`, `A`, `B`, `C`, or `D`. `mission.update` additionally requires `mission_id` and is a full replacement of editable fields: omitted optional fields are cleared. Use lifecycle operations for complete/archive/delete.

Memory creation and update:

```json
{
  "kind": "preference",
  "content": "Prefers missions with a concrete result and one-week scope."
}
```

Update additionally requires `memory_id`. Kinds are `focus`, `preference`, `constraint`, `habit`, `summary`, `reminder`, and `observation`.

## Atomic Record batch

```json
{
  "operations": [
    {
      "operation": "record.set",
      "input": {
        "definition_id": "identity.nickname",
        "value": "Alice"
      }
    },
    {
      "operation": "record.increment",
      "input": {
        "definition_id": "fitness.run_count",
        "delta": 1
      }
    }
  ]
}
```

Write the JSON to a temporary file, then run:

```text
arcana-data batch apply --file <batch.json> --dry-run
arcana-data batch apply --file <same-batch.json>
```

A successful response contains `dry_run` and ordered `{index, operation, result}` entries. A failed batch writes no partial Record changes and includes `details.operation_index` and `details.operation`.

Multi-operation batch accepts only `record.*` operations. Pack Definitions, enabled Pack IDs, Achievement states, accepted Missions, and AssistantMemory use the synchronized JSON repository. Suggestions and UI selections use runtime-local JSON. Dry-run and execute those mutations one command at a time; they cannot be made atomic with a Record batch.

## Process contract

- Exit 0: stdout is direct business JSON; stderr is empty.
- Exit 1: domain error such as not-found, conflict, unresolved, or validation failure.
- Exit 2: invalid invocation or malformed command input.
- Exit 3: busy/storage/runtime failure.
- Failure: stdout is empty; stderr is one structured JSON object.

Treat stable `code` and `details` as the machine contract. Show the human `message`, but do not branch on its text.
