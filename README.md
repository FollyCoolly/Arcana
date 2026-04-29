# Arcana

A HUD for gamified life management, Persona 5-styled.

---

## Project Overview

Arcana is a comprehensive personal information management and visualization tool that applies game design thinking to real life, with a built-in AI agent that helps track progress, propose missions, and keep your journey moving forward.

Arcana is **not** another habit-tracking app with streaks and checkboxes. As a desktop application, it emphasizes data organization, synthesis, and meaningful presentation. While it borrows the language of games, the data it presents carries real significance — no arbitrary "+5 STR for doing push-ups" mechanics. Instead, Arcana finds the genuine intersections between real life and game systems and surfaces them without forcing reality into a game framework.

---

## Screenshots

| Main Menu |
|-----------|
| ![Arcana main menu](docs/screenshots/main-menu.jpg) |

| Status | Missions |
|--------|----------|
| ![Arcana status screen](docs/screenshots/status.jpg) | ![Arcana missions screen](docs/screenshots/missions.jpg) |

| Achievements | Skills |
|--------------|--------|
| ![Arcana achievements screen](docs/screenshots/achievements.jpg) | ![Arcana skills screen](docs/screenshots/skills.jpg) |

| Items | Gallery |
|-------|---------|
| ![Arcana items screen](docs/screenshots/items.jpg) | ![Arcana gallery screen](docs/screenshots/gallery.jpg) |

---

## Features

### Status

Multi-dimensional life radar powered by real data.

- A three-layer model: **metrics** (raw data like weight, lift PRs, run pace), **dimensions** (scored axes on a radar chart), and **level titles** (P5-style ranks per dimension).
- Dimension scores are computed from weighted metric contributions — not manually assigned.
- System metrics (`sys_` prefix) are derived automatically from other modules (gallery counts, skill levels, achievement stats).
- Radar chart visualization with drill-down into individual dimension details.

### Achievements

Milestone tracking with content pack support.

- Record life milestones with unlock timestamps and difficulty grades (`beginner` through `legendary`).
- Achievements can have prerequisites, forming a DAG of dependencies.
- Content packs allow loading achievement sets tailored to your interests (e.g., programmer, fitness).
- AI agent can track partial progress and mark completions.

### Skills

Skill tree system tightly coupled with achievements.

- Each skill tree node maps to an achievement; unlocking achievements lights up the tree.
- Skill levels are computed from accumulated node points and key achievement requirements.
- Interactive skill tree visualization with prerequisite-based layout.
- Loaded via content packs alongside achievements.

### Missions

AI-driven quest system (replaced the former Crafting module).

- Missions are proposed by the AI agent based on current goals and context, styled as Persona 5 "Phan-Site" requests.
- Lifecycle: `proposed` → `active` → `completed` / `archived` / `rejected`.
- Progress tracked as 0–100 by the AI agent.
- Main menu integration: pin a countdown mission and a progress mission for at-a-glance tracking.
- Can link to achievements for cross-system progression.

### Items

Personal inventory management.

- Track clothing, electronics, and other possessions.
- Record purchase dates, prices, and categories.
- A data-driven reminder to consume mindfully.

### Gallery

Aggregated media consumption hub.

- Unified view of books, anime, movies, and games.
- Cover wall display with filtering and sorting.
- Import scripts for external sources:
  - Bangumi (anime/books)
  - Steam (games)
  - Douban (movies/books)

---

## AI Agent

Arcana includes a built-in AI agent that acts as a personal life assistant, operating through three channels:

| Channel | Description |
|---------|-------------|
| **CLI** | Standalone terminal agent (`agent-cli`) |
| **Telegram** | Bot adapter for mobile access (`agent-telegram`) |
| **Data CLI** | Structured data operations for AI skills (`arcana-data`) |

All three share a common services layer (`src-tauri/src/services/`) and data format, so updates from any channel are immediately visible everywhere.

The agent can:
- Read current status, missions, achievements, and memory context
- Update mission progress and status
- Track and mark achievements
- Propose new missions based on your goals
- Maintain cross-session memory for continuity

---

## Tech Stack

- **Framework**: [Tauri v2](https://v2.tauri.app/) (Rust backend + webview frontend)
- **Frontend**: Svelte 5 + SvelteKit v2 + TypeScript + Tailwind CSS v4 + Three.js
- **Backend**: Rust (IPC commands, AI agent, JSON data layer)
- **Data**: Local JSON files (`data/`, gitignored) — no database
- **AI**: Direct Anthropic API integration with tool-calling loop

---

## Project Structure

```
src/                    # SvelteKit frontend
  ├── routes/           #   Single-page app (main menu + sub-screens)
  └── lib/
      ├── screens/      #   Screen components (Status, Achievements, Skills, Items, Gallery, Missions)
      ├── components/   #   Shared UI components (RadarChart, SkillNebula, etc.)
      ├── types/        #   TypeScript type definitions
      ├── stores/       #   Svelte stores
      └── utils/        #   Frontend utilities
src-tauri/src/          # Rust backend
  ├── commands/         #   Tauri IPC commands (status, achievements, skills, missions, items, gallery, weather)
  ├── models/           #   Serde data structures
  ├── storage/          #   JSON read/write & validation
  ├── services/         #   Shared business logic (used by agent, arcana-data CLI, and Tauri commands)
  ├── agent/            #   AI agent subsystem (runner, LLM, tools, prompt, config, session)
  └── bin/              #   Standalone binaries: agent_cli, agent_telegram, arcana_data
data/                   # Runtime JSON data (gitignored)
  ├── packs/<pack_id>/  #   Content packs (manifest.json, achievements.json, skills.json)
  ├── sessions/         #   Agent JSONL session history
  └── *.json            #   missions, status, achievement_progress, mission_memory, etc.
docs/                   # Architecture docs, schema specs, UI design guides
  └── schema/           #   JSON schema definitions
scripts/                # Python tooling (data import, schema validation)
static/                 # Static assets (icons, images)
```

---

## Getting Started

### Prerequisites

- **Rust**: stable toolchain
- **Node.js**: v18+
- **Platform**: Windows / macOS / Linux

### Font Requirements

Arcana's visual style depends on a few system fonts. These font files are **not bundled with this repository or release builds**; users need to install them locally for the intended Persona 5-inspired look:

- `p5hatty` — primary display font for menus, labels, cards, and collage-style text
- `Source Han Sans SC` — Chinese UI and card-title text
- `Bebas Neue` — key hint badges

If these fonts are missing, the app will still run, but the UI will fall back to system fonts such as `Arial`, `Microsoft YaHei`, or generic `sans-serif`, and some title/card layouts may look different.

### Development

```bash
# Install frontend dependencies
npm install

# Run full desktop app in dev mode
npm run tauri dev

# Or run only the frontend dev server
npm run dev
```

### Build

```bash
# Build desktop release
npm run tauri build

# Build standalone agent binaries
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-cli
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-telegram
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
```

### Checks

```bash
# TypeScript / Svelte type checking
npm run check

# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Rust formatting
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

---

## Documentation

- [Architecture](docs/architecture.md) — system architecture overview
- [Visual Style Guide](docs/visual_style_guide.md) — Persona 5-inspired design tokens and visual language
- [UI Design Spec](docs/ui_design_spec.md) — main menu and sub-screen interaction spec
- [Schema Reference](docs/schema/README.md) — data structure documentation
- [AI Agent Integration](docs/ai_agent_integration.md) — agent platform survey and integration plan

---

## Design Decisions

- **Tauri + JSON over Electron + SQLite**: Smaller binary, better performance, human-readable and version-controllable data files.
- **Content Pack system**: Achievements and skills are loaded via pluggable packs, supporting community extension.
- **Agent decoupled from UI**: The AI agent runs independently of the desktop GUI (CLI / Telegram), sharing the same data layer.
- **DAG skill trees**: Frontend derives edges and layout from `prerequisites` automatically — no redundant edge data stored.
- **Shared services layer**: `services/` contains all business logic, consumed by Tauri commands, arcana-data CLI, and the Rust agent alike.

---

## Roadmap to v0.1

- [ ] Provide example data configuration
- [x] Polish main menu — countdown and progress bar widgets
- [x] Polish Skills screen
- [x] Polish Achievements screen
- [x] Polish Items screen
- [x] Polish Gallery screen
- [x] Polish Missions screen
- [ ] Test skill tree functionality end-to-end

---

## Future Ideas

### UI & Experience

- Onboarding wizard for first-time setup
- Sound effects across the interface
- Data-change reveal animations — show what changed since last session on first open
- Cinematic animations for mission acceptance and completion

### Features

- Skill tarot card generator — auto-generate a Persona-style card for each tracked skill (possibly with a generative model)
- Music tracking in Gallery (alongside books, anime, movies, games)
- AI navigator companion — a persistent on-screen assistant inspired by Futaba / Morgana from P5 (default look: Kurisu from Steins;Gate)

### Audit & Transparency

- User-facing changelog viewer — surface `ai_changelog.json` in the UI so users can review, approve, and roll back AI-driven data changes
- Diff view for AI modifications with one-click revert

### Integration & Platform

- Support more IM channels (e.g. Discord, WeChat) and LLM providers beyond Anthropic
- More data source importers for Gallery and Status
- Deeper integration with external AI knowledge management systems
- Mobile read-only dashboard — a lightweight web view for checking Status radar and Mission progress on the go
- Health data auto-import — sync from Apple Health / Google Fit / Garmin to keep Status metrics up-to-date automatically
- Community content pack repository — let others publish and share achievement packs

---

## Acknowledgements

- [Mive82/Persona-5-Calendar](https://github.com/Mive82/Persona-5-Calendar) — calendar component reference
- [sjpiper145/MakerSkillTree](https://github.com/sjpiper145/MakerSkillTree) — grid-based skill tree layout inspiration
- [NERvGear/SAO-Utils](https://github.com/NERvGear/SAO-Utils) — game-styled desktop app inspiration
- [aliubo/persona-text-gen](https://github.com/aliubo/persona-text-gen) — collage-style (calling card) text generation reference

---

## License

MIT
