# Arcana Pack contract v1

Use the canonical [complete PackContent fixture](../../../fixtures/contract-v1/pack-content.json). The Rust process tests exercise the same file; prefer it over reconstructing field names from memory.

## Commands

```text
arcana-data pack list
arcana-data pack show <pack_id>
arcana-data pack scaffold <pack_id> --name <display_name>
arcana-data pack validate --file <pack-content.json>
arcana-data pack write --file <pack-content.json>
arcana-data pack enable <pack_id>
arcana-data pack disable <pack_id>
arcana-data pack delete <pack_id>
arcana-data pack asset-put <pack_id> <assets/path.webp> --file <local-file>
arcana-data pack asset-delete <pack_id> <assets/path.webp>
```

`scaffold` does not require an initialized runtime. `validate` uses current repository context and existing assets but does not write. `write` replaces structured Pack content in the configured JSON repository while preserving assets and enabled state. Mutating commands support `--dry-run`.

Pack mutations cannot be part of a multi-operation `batch apply`; that command is reserved for Record mutations in SQLite. To create and enable a Pack, validate and write it first, then dry-run and execute `pack enable` as a separate command. If the second command fails, report that the Pack exists but remains disabled. Asset bytes likewise use their dedicated commands.

## PackContent

Top-level fields:

```json
{
  "manifest": {},
  "record_definitions": { "definitions": [] },
  "dimensions": { "dimensions": [] },
  "achievements": { "achievements": [] },
  "skills": { "skills": [] }
}
```

Only `manifest` is required. Omit an unused optional section; do not provide an empty array. Unknown fields and `null` are rejected.

### Manifest

```json
{
  "schema_version": 1,
  "id": "machine_learning",
  "name": "Machine Learning",
  "description": "Optional nonblank description.",
  "author": "Optional author",
  "parent_pack_id": "programming",
  "tags": ["ai", "programming"]
}
```

Pack IDs and tags use lowercase snake_case. Tags are unique and sorted. `parent_pack_id` cannot equal the Pack ID; a missing or disabled parent does not prevent the child from running.

### RecordDefinitions

Scalar:

```json
{
  "kind": "scalar",
  "id": "health.body_weight",
  "name": "Body weight",
  "value_type": "number",
  "unit": "kg"
}
```

Collection or event:

```json
{
  "kind": "event",
  "id": "fitness.running",
  "name": "Running",
  "fields": {
    "distance_km": { "type": "number", "required": true, "unit": "km" },
    "duration_minutes": { "type": "number", "required": false, "unit": "min" }
  }
}
```

Leaf types are `string`, `number`, `integer`, `boolean`, `date`, and `datetime`. Units are allowed only on number/integer fields. Definition IDs use `<namespace>.<name>`; arrays are sorted by ID.

Two Packs may declare the same ID only with compatible definitions. Kind, name, scalar type/unit, and common structured fields must match. A structured definition may add an optional field; required-field changes, type/unit changes, kind changes, and conflicting nonempty descriptions are breaking changes requiring a new ID.

### Status Dimensions

```json
{
  "id": "fitness::physical",
  "name": "Physical",
  "level_titles": ["Awake", "Growing", "Skilled", "Excellent", "Peak"],
  "level_thresholds": [25, 50, 75, 90],
  "scores": [
    {
      "id": "endurance",
      "name": "Endurance",
      "weight": 1,
      "expression": "280 / record('cardio.run_5k_pace_sec_per_km') * 100"
    }
  ]
}
```

Dimension ID uses `<pack_id>::<local_id>`. Titles have exactly five entries. Thresholds have exactly four values satisfying `0 < t2 < t3 < t4 < t5 <= 100`. Scores are sorted by local ID and have positive finite weights.

Expressions may use finite decimals, `+ - * /`, unary signs, parentheses, `record('<static-id>')`, `min`, `max`, `abs`, and `clamp`. Referenced Records must be numeric scalars fully declared by this Pack. Missing values propagate as missing; final valid child values clamp to `[0,100]`.

### Achievements

```json
{
  "id": "cooking::ten_dishes",
  "name": "A Table of Your Own",
  "description": "Cook ten different dishes independently.",
  "difficulty": "intermediate",
  "tags": ["repertoire"],
  "prerequisites": ["cooking::first_dish"],
  "related_record_definition_ids": ["cooking.learned_dishes"],
  "tip": "Accept direct completion if the user does not want to reconstruct old history."
}
```

Difficulty is `beginner`, `intermediate`, `advanced`, `expert`, or `legendary`. IDs, prerequisites, and related Record IDs are unique and sorted. Prerequisites and Achievement IDs must belong to this Pack; prerequisite graphs must be acyclic. Related definitions must be fully declared in this Pack.

Use `tip` only for Agent guidance that clarifies unusual completion judgment or useful tracking information. Do not put user facts or an executable unlock rule in a Definition.

### Arcana Skills

```json
{
  "id": "cooking::home_cooking",
  "name": "Home Cooking",
  "description": "Practical ability to prepare varied meals independently.",
  "level_thresholds": [5, 10, 20, 25],
  "nodes": [
    { "achievement_id": "cooking::first_dish", "points": 5 },
    { "achievement_id": "cooking::ten_dishes", "points": 20 }
  ],
  "card_image": "assets/home-cooking.webp"
}
```

Skill IDs and node Achievement IDs must belong to this Pack. Nodes are unique and sorted by Achievement ID; points are positive integers. Four positive, strictly increasing thresholds define Lv.2–Lv.5. The Lv.5 threshold cannot exceed total node points. `card_image` is optional and must be a portable path below `assets/` with PNG, JPEG, or WebP bytes.

## Assets and hierarchy

Keep bytes out of PackContent. Add/delete them only through `asset-put` and `asset-delete`. Content validation fails when a referenced card image is missing or its bytes do not match the extension.

Parent/child relations organize PackForest only. Each Pack carries all definitions it needs, enabling/disabling never cascades, and no Pack may use a parent as an implicit runtime dependency.

## Deletion

Preview deletion with `arcana-data pack delete --dry-run <pack_id>`. The result reports:

- whether the Pack was enabled;
- child Packs that will retain a missing organizational parent;
- Records and Achievement states that will become unresolved;
- local Status selections whose Dimension disappears.

Deleting a Pack removes its definitions and assets but deliberately preserves user-owned Records and Achievement states. Require explicit user confirmation after showing this impact.

## Errors

Exit 0 writes business JSON to stdout. Failure leaves stdout empty and writes structured JSON to stderr. For `validation_failed`, show every `details.validation_issues[]` entry with its code, path, and message; do not bypass validation or directly edit SQLite or the live JSON repository.
