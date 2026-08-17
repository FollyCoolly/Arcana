# Arcana

[English](README.md) | [简体中文](README.zh-CN.md)

An AI-assisted, Persona 5-inspired desktop HUD for gamified life management.

Arcana turns real-life facts and goals into Status dimensions, Achievements, Skills, and Missions. Human-readable definitions and synchronized state live in a JSON repository, while Records use local SQLite. AI support is provided by external Arcana Skills, which use the typed `arcana-data` CLI instead of a built-in model runtime.

> [!IMPORTANT]
> Required fonts are not bundled. See [Font requirements](#font-requirements).

## Screenshots

| Main Menu |
| --- |
| ![Arcana main menu](docs/screenshots/main-menu.jpg) |

<table>
  <tr><th width="50%">Status</th><th width="50%">Missions</th></tr>
  <tr><td><img src="docs/screenshots/status.jpg" alt="Status" width="100%"></td><td><img src="docs/screenshots/missions.jpg" alt="Missions" width="100%"></td></tr>
  <tr><th>Achievements</th><th>Skills</th></tr>
  <tr><td><img src="docs/screenshots/achievements.jpg" alt="Achievements" width="100%"></td><td><img src="docs/screenshots/skills.jpg" alt="Skills" width="100%"></td></tr>
  <tr><th>Items</th><th>Gallery</th></tr>
  <tr><td><img src="docs/screenshots/items.jpg" alt="Items" width="100%"></td><td><img src="docs/screenshots/gallery.jpg" alt="Gallery" width="100%"></td></tr>
</table>

## Features

- **Records**: a flat fact layer shared by Status and Achievement evaluation. Definitions come from enabled Packs; user values remain user-owned.
- **Derived Values**: Packs can name reusable calculations such as BMI or game days; values are evaluated lazily from Records and never persisted.
- **Status**: Pack-defined Dimensions calculate 0–100 child scores from numeric Records or Derived Values, combine them by weighted average, and derive Lv.0–Lv.5.
- **Achievements**: milestones with prerequisites and minimal `tracked` / `achieved` state. Only achieved milestones contribute points.
- **Skills**: Pack-defined skill maps derived from achieved Achievements; levels and node state are computed rather than stored separately.
- **Missions**: local AI suggestions become synchronized Missions only after acceptance. Missions support active, completed, and archived lifecycles.
- **Packs**: hierarchical domain content containing Record and Derived Value definitions, Dimensions, Achievements, Skills, and assets. The desktop Pack screen manages installed content, enabled state, and safe deletion impact.
- **Assistant Memory**: durable semantic context that can be synchronized with the rest of the user repository.
- **Items and Gallery**: adapters over user-selected external files; these sources remain authoritative and are not copied into the core data platform.

## AI integration

Arcana does not embed an LLM runtime or run an Agent service. The canonical plugin under `plugins/arcana/` provides three external Skills:

- **Velvet Room** records facts, progress, corrections, Achievement state, Status selections, and AssistantMemory.
- **Phan Site** generates and manages MissionSuggestions.
- **Pack Manager** creates, extends, and validates domain Packs.

All Skills call `arcana-data`, so UI and AI writes share the same validation and transaction boundaries. `.claude/skills` is a generated compatibility mirror; edit the canonical plugin instead.

## Architecture

```text
src/                              SvelteKit frontend
src-tauri/src/
  application/                    typed use cases and runtime lock
  domain/                         domain models and validation
  storage/data_repository.rs      composite storage boundary
  storage/sqlite/                 Record-only migrations and adapter
  storage/json_repository.rs      live semantic JSON and deterministic codec
  storage/local_state.rs          device-local suggestions and selections
  commands/                       Tauri IPC boundary
  models/                         Items, Gallery, Weather adapters
  bin/arcana_data.rs              machine-readable data CLI
plugins/arcana/                   canonical external Agent plugin and Skills
docs/design/                      data-platform contracts and decisions
data-example/                     Items/Gallery/Weather config examples
```

The live semantic repository defaults to `~/.arcana/repository`; Records default to `~/.arcana/runtime/arcana.sqlite3`, and device-local suggestions/selections use `~/.arcana/runtime/local-state.json`. `arcana-data json import|export` converts the combined state to and from a deterministic, human-readable directory. Git pull/commit/push orchestration is not implemented yet.

Items, Gallery, and Weather read small adapter configuration files from `~/.arcana/data` (or `ARCANA_DATA_DIR`). They are separate from synchronized core user data.

## Quick start

Prerequisites: stable Rust, Node.js 18+, and the platform-specific Tauri dependencies.

```bash
npm install
npm run tauri dev
```

Build and inspect the data CLI:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
./src-tauri/target/debug/arcana-data capabilities
./src-tauri/target/debug/arcana-data init
./src-tauri/target/debug/arcana-data context summary
```

The CLI provides Record, Derived Value, Pack, Status, Achievement, Skill, Mission, AssistantMemory, batch, dry-run, and deterministic JSON import/export commands. Multi-operation batch is intentionally Record-only; JSON-backed mutations use individual commands. Run `arcana-data help` or `<group> help` for the current contract.

## Checks

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
python scripts/sync_agent_skills.py --check
```

## External-source setup

Copy the relevant files from `data-example/` into `~/.arcana/data/` and edit them for your machine:

- `item_sources.json`: absolute paths to Obsidian/Markdown item directories.
- `gallery_sources.json`: paths to generated media files.
- `weather.json`: city or coordinates for Open-Meteo.

Import helpers are available under `scripts/fetch_bangumi.py`, `scripts/fetch_steam.py`, and `scripts/fetch_douban.py`. Credentials belong in the ignored `scripts/config.json`; use `scripts/config.example.json` as a template.

## Font requirements

For the intended visual style, install `p5hatty`, `Source Han Sans SC`, and `Bebas Neue` from legitimate sources. Without them, the application falls back to system fonts and some layouts will differ.

The UI was primarily developed on Windows at 4K/100% scaling. Other resolutions, Windows scaling modes, and macOS/Retina still need a unified adaptation pass.

## Documentation

- [Current architecture](docs/architecture.md)
- [Data-platform design and contracts](docs/design/README.md)
- [Items external-source schema](docs/schema/items.md)
- [Visual style guide](docs/visual_style_guide.md)
- [UI design specification](docs/ui_design_spec.md)

## Acknowledgements

- [Mive82/Persona-5-Calendar](https://github.com/Mive82/Persona-5-Calendar)
- [sjpiper145/MakerSkillTree](https://github.com/sjpiper145/MakerSkillTree)
- [NERvGear/SAO-Utils](https://github.com/NERvGear/SAO-Utils)
- [aliubo/persona-text-gen](https://github.com/aliubo/persona-text-gen)

## License

MIT
