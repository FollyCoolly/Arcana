# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Reality Mod is a desktop application for comprehensive personal information management and visualization using gamification concepts. It's a "user interface for Earth Online" that helps manage real-life data through game-like systems (status tracking, achievements, skill trees, items, gallery, and crafting).

**Core Philosophy**: Present data with real meaning rather than forcing reality into RPG-game mechanics. Find commonalities between reality and games, but don't artificially gamify everything.

## Technology Stack

- **Framework**: Tauri v2 (Rust backend + Svelte 5 frontend)
- **Frontend**: SvelteKit + TypeScript + Tailwind CSS
- **Data Storage**: Local JSON files (local-first architecture)
- **Build Tool**: Vite
- **Languages**: Rust (stable) for backend, TypeScript for frontend

## Common Development Commands

### Development
```bash
# Run development mode (starts both frontend and backend)
npm run tauri dev

# Run frontend dev server only
npm run dev

# Type checking
npm run check

# Type checking in watch mode
npm run check:watch
```

### Building
```bash
# Build frontend for production
npm run build

# Build Tauri application
npm run tauri build

# Preview production build
npm run preview
```

### Backend (Rust)
```bash
# From src-tauri directory
cd src-tauri

# Run Rust tests
cargo test

# Run clippy for linting
cargo clippy

# Build Rust backend only
cargo build

# Build release
cargo build --release
```

## Architecture Overview

### Four-Layer Architecture

1. **Presentation Layer** (Svelte)
   - Location: `src/routes/`, `src/lib/components/`
   - Handles UI rendering and user interactions
   - Uses Svelte stores for state management

2. **API Layer** (Tauri Commands)
   - Location: `src-tauri/src/commands/`
   - Defines frontend-backend communication interface
   - Example: `greet` command in `src-tauri/src/lib.rs:6`

3. **Business Logic Layer** (Rust)
   - Location: `src-tauri/src/services/` (to be created)
   - Handles skill calculations, achievement checking, pack management

4. **Data Access Layer** (Rust)
   - Location: `src-tauri/src/storage/` (to be created)
   - Manages JSON file I/O and file system operations

### Data Flow

```
External Sources (GitHub, Bangumi, Steam)
    ↓ (Python scripts)
JSON Files (data/)
    ↓ (Rust reads/parses)
Tauri Commands
    ↓ (IPC)
Svelte Stores
    ↓
UI Components
```

## Directory Structure

Current project is in early MVP stage. Expected evolution:

```
RealityMod/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── lib.rs          # Tauri setup and commands
│   │   ├── models/         # Data structures (to be created)
│   │   ├── storage/        # JSON I/O (to be created)
│   │   ├── services/       # Business logic (to be created)
│   │   └── commands/       # Tauri commands (to be created)
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
│
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # UI components (to be created)
│   │   ├── stores/         # State management (to be created)
│   │   ├── utils/          # Utility functions (to be created)
│   │   └── types/          # TypeScript types (to be created)
│   └── routes/
│       ├── +layout.svelte  # Global layout (to be created)
│       └── +page.svelte    # Main page
│
├── data/                   # Local JSON data storage (to be created)
│   ├── profiles/           # User profiles
│   ├── packs/              # Content packs (achievements, skills)
│   └── imports/            # External data imports
│
├── scripts/                # Data import scripts (to be created)
└── docs/                   # Documentation
    ├── architecture.md
    └── directory_structure.md
```

## Six Core Modules (Planned)

1. **Status** - Body and life data center (fitness, health metrics)
2. **Achievements** - Milestone tracking with difficulty levels and dependencies
3. **Skills** - Skill tree system linked to achievements (DAG visualization)
4. **Items** - Inventory management (clothing, digital products)
5. **Gallery** - Aggregated media consumption (books, movies, games)
6. **Crafting** - Recipe management system

Each module follows the pattern: `model → command → component → route`

## Content Pack System

A key architectural feature for modular achievement/skill definitions:

- **Structure**: `data/packs/{pack_name}/` containing `manifest.json`, `achievements.json`, `skills.json`
- **Independence**: Each pack defines its own achievements and skills
- **Composability**: Users can load multiple packs simultaneously
- **Examples**: Programmer pack, fitness pack, etc.

## Key Technical Decisions

### Why Tauri over Electron?
- Smaller bundle size (~3-5 MB vs ~100+ MB)
- Lower memory footprint
- Better security (Rust memory safety)
- Superior performance

### Why JSON over SQLite?
- Human-readable and easily editable
- Simple backup/migration (copy `data/` directory)
- Version control friendly
- Sufficient for expected data scale
- Will migrate to SQLite if data exceeds 10,000 records

### Global Shortcut
- **Current**: `Ctrl+Shift+R` (Windows/Linux) or `Cmd+Shift+R` (macOS)
- **Implementation**: `src-tauri/src/lib.rs:18-35`
- Uses `tauri-plugin-global-shortcut`
- Toggles window visibility

## Development Guidelines

### Naming Conventions

**Rust:**
- Files: `snake_case` (e.g., `skill_calculator.rs`)
- Structs/Enums: `PascalCase` (e.g., `UserProfile`, `AchievementStatus`)
- Functions/Variables: `snake_case` (e.g., `calculate_level`, `user_id`)

**TypeScript/Svelte:**
- Components: `PascalCase` (e.g., `SkillTreeView.svelte`)
- Files: `camelCase` for utilities (e.g., `formatters.ts`)
- Functions/Variables: `camelCase` (e.g., `loadProfile`, `userId`)

### SvelteKit Configuration

- Uses `adapter-static` for SPA mode (Tauri doesn't support SSR)
- Frontend builds to `build/` directory
- Dev server runs on port 1420 (configured in `vite.config.js:17`)
- HMR uses port 1421

### Tauri IPC Pattern

**Frontend (TypeScript):**
```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke("command_name", { param: value });
```

**Backend (Rust):**
```rust
#[tauri::command]
fn command_name(param: Type) -> Result<ReturnType, String> {
    // Implementation
}
```

Register in `lib.rs:39`: `.invoke_handler(tauri::generate_handler![command_name])`

### Error Handling

**Rust:** Use `Result<T, E>` with custom error types
**TypeScript:** Wrap `invoke()` calls in try-catch blocks

### Layer Interaction Rules

- ✅ **Allowed**: Upper layer calling lower layer
- ❌ **Forbidden**: Lower layer calling upper layer, cross-layer calls
- **Exception**: Svelte stores can directly call Tauri commands (avoid over-abstraction)

## Current Development Stage

**Phase 2: MVP - Status Module** (In Progress)

Next steps according to roadmap:
- Design Status data JSON schema
- Implement Rust JSON file reading
- Create `load_status_data` Tauri command
- Build Status page UI with data cards and trend charts
- Add desktop HUD features (window always-on-top, transparency)

See `README.md` for detailed development roadmap through Phase 7.

## Important Notes

- Window starts hidden (`tauri.conf.json:21`) - shown via global shortcut
- All data stored locally in `data/` directory (create as needed)
- JSON schema designed incrementally, not all at once
- Follow progressive development: create directories/files only when needed
- Each module is independent but shares common infrastructure
