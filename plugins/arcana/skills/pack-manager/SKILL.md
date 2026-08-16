---
name: pack-manager
description: Create, extend, refine, validate, enable, disable, remove, and add assets to Arcana content Packs containing RecordDefinitions, Status Dimensions, Achievements, and Arcana Skills. Use when a user wants a new domain pack, wants an existing pack expanded or improved, wants Pack availability changed, or needs pack schema and quality problems diagnosed or fixed.
---

# Pack Manager

Design self-contained, high-quality Arcana Packs and write them through `arcana-data pack` commands. Pack definitions, assets, and enabled state live in the configured JSON repository; SQLite contains only Records. Do not bypass CLI validation by editing either store during a Skill run.

## Prepare

1. Resolve `arcana-data` from `PATH`; inside the repository, fall back to the current-platform debug binary or `cargo run --manifest-path src-tauri/Cargo.toml --bin arcana-data --`.
2. Run `arcana-data --compact capabilities` and require `contract_version: 1` plus Pack command version 1.
3. Read [references/pack-contract.md](references/pack-contract.md) before producing PackContent.
4. Run `pack list`. In refine/extend mode, run `pack show <id>`. Inspect related Packs when reusing a RecordDefinition ID so the full definition remains compatible.

## Choose the Mode

- Create: establish a domain Pack from the user's present focus.
- Extend: add a coherent subdomain, Achievement, Dimension, Skill, or RecordDefinition while preserving existing IDs and content.
- Refine: improve existing definitions or organization while explicitly identifying compatibility and user-data impact.

Let a Pack start small. Use `parent_pack_id` to organize broad-to-specific domains when one Pack would become unwieldy; the relationship is organizational, not an enablement or runtime dependency.

## Model the Domain

Keep Records flat and Packs hierarchical. A Pack may declare RecordDefinitions from multiple namespaces. If its Dimensions or Achievements reference a RecordDefinition, include that full definition in the same Pack even when another Pack already declares a compatible copy.

Create a RecordDefinition only for reusable, user-specific facts worth recording. Do not force every Achievement into a measurable Record. Link existing facts with `related_record_definition_ids`; use `tip` for unusual judgment or useful tracking information that does not justify a predefined Record.

Define Status as one Dimension layer containing weighted child Scores. Keep each Score in `[0,100]` through the system's default clamp and use the safe expression language only. Do not add a recursive tree or final score expression.

Define Achievement completion in natural language. Keep prerequisites local to the Pack and acyclic. Define Arcana Skill nodes as local Achievement references with points; only achieved states score.

## Write Safely

1. Start a new Pack with `pack scaffold <id> --name <name>` or the published PackContent fixture.
2. Keep IDs and all required arrays sorted. Preserve every existing stable ID unless the user explicitly chooses a breaking replacement strategy.
3. Write candidate PackContent to a temporary JSON file.
4. Run `pack validate --file <candidate.json>` and fix every issue.
5. Run `pack write --dry-run --file <same-candidate.json>` before replacing an enabled Pack whose definitions already validate user Records.
6. Run `pack write --file <same-candidate.json>` only after validation succeeds.
7. For a new Pack that should be enabled immediately, dry-run and execute `pack enable <id>` as a second command after `pack write`. For an existing Pack, enable or disable it explicitly only when requested. Enabling does not cascade to parents or children.
8. Import binary card art separately with `pack asset-put`; validate again after content references the asset.
9. Run `pack show <id>` and summarize the resulting counts, hierarchy, and enabled state.

`pack write` preserves current enabled state and existing assets. Pack mutations are single logical JSON-repository operations and cannot be placed in multi-operation `batch apply`; batch is reserved for Record mutations in SQLite. Do not embed asset bytes or local absolute paths in PackContent.

For deletion, first run `pack show`, then `pack delete --dry-run <id>`. Surface child Packs, unresolved Records or Achievement states, and orphaned local Status selections from the preview. Execute the real delete only after the user explicitly confirms the impact. Deletion preserves user Records and Achievement states as unresolved data instead of silently discarding them.

## Quality Check

Before writing, verify:

- names and descriptions are distinct, concrete, and game-readable;
- difficulty reflects real progression;
- RecordDefinitions are reusable and not near-duplicates;
- all referenced definitions are fully declared and compatible;
- Achievement prerequisites form a DAG;
- related Record IDs, Skill nodes, and scoped IDs belong to this Pack;
- every Score expression is readable and uses numeric scalar Records;
- Skill Lv.5 is reachable but does not require every possible milestone by design;
- assets use portable `assets/...` paths and supported image bytes.

Surface validation details instead of weakening the model or bypassing checks.
