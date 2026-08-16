# Phan Site mission contract v1

## Reads

```text
arcana-data context summary
arcana-data mission list --status active
arcana-data mission suggestion-list --status pending
arcana-data mission suggestion-list --status rejected
arcana-data achievement list --status tracked
arcana-data achievement list --pack <pack_id>
arcana-data record query --pack <pack_id> --has-value true
arcana-data memory list
```

Use exact ID filters when following a reference. All filters on one query are combined with AND.

## Suggestion input

```json
{
  "title": "The Mise en Place Trial",
  "description": "Prepare every ingredient before cooking one dinner.",
  "difficulty": "D",
  "deadline": "2026-08-23",
  "parent_mission_id": "existing-mission-id",
  "reason": "Build a repeatable preparation habit for the active cooking mission."
}
```

Only `title` is required. Omit unknown optional fields instead of writing `null` or guessing. Difficulty is one of `S`, `A`, `B`, `C`, and `D`; deadline is `YYYY-MM-DD`.

The CLI generates UUIDv7 `id`, `generated_at`, and `status: pending`. Do not supply them.

## Atomic generation

```json
{
  "operations": [
    {
      "operation": "mission.suggest",
      "input": {
        "title": "The Mise en Place Trial",
        "difficulty": "D",
        "reason": "A concrete next step for the active cooking goal."
      }
    },
    {
      "operation": "mission.suggest",
      "input": {
        "title": "Five-Dish Rotation",
        "difficulty": "C",
        "reason": "Expand the user's stated home-cooking focus."
      }
    }
  ]
}
```

Run:

```text
arcana-data batch apply --file <suggestions.json> --dry-run
arcana-data batch apply --file <same-suggestions.json>
```

Dry-run IDs and timestamps are previews; do not reference them. The operation array must remain identical for the committed call.

## User decisions

```text
arcana-data mission accept <suggestion_id> --dry-run
arcana-data mission accept <suggestion_id>
arcana-data mission reject <suggestion_id> --dry-run
arcana-data mission reject <suggestion_id>
arcana-data mission suggestion-delete <suggestion_id> --dry-run
arcana-data mission suggestion-delete <suggestion_id>
```

Accept atomically creates an active Mission with the same ID and removes the local Suggestion. Only accepted Missions enter JSON export and later Git sync.

## Failure contract

Exit 0 writes direct business JSON to stdout. Any failure leaves stdout empty and writes structured JSON to stderr. Use stable `code` and `details`; a batch error includes `operation_index` and `operation`, and the whole transaction is rolled back.

See the published [capabilities fixture](../../../fixtures/contract-v1/capabilities.json) and [complete mutation fixture](../../../fixtures/contract-v1/batch-all-operations.json) for the command surface and exact field names.
