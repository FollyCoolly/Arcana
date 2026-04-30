# Changelog

## v0.1.0 (2026-04-30)

Initial release. A Persona 5-inspired gamified life management desktop app built with Tauri v2, Svelte 5, and Rust.

### Core Screens

- **Main Menu**: P5-style radial trapezoid menu with date/weather calendar widget, player name/game day display, star graphics, mission countdown and progress bar widgets, and mission board
- **Status**: Three-layer metric system (metrics, dimensions, level titles) with P5 star radar chart, collage-style dimension labels, dimension drill-down with Q/E navigation, and system metrics (`sys_` prefix) auto-derived from other modules
- **Achievements**: Content pack-based milestone tracking with prerequisite DAG validation, difficulty grades (beginner through legendary), filtering/sorting, and two-panel locked/unlocked UI
- **Skills**: Interactive honeycomb node map with skill levels computed from accumulated node points and key achievements; custom card art support and detail info screen per node
- **Missions**: AI-driven quest system with Phan-Site proposal workflow and review UI, lifecycle management (proposed/active/completed/archived/rejected), D/C/B/A/S difficulty grades, parent mission links for subtasks, progress tracking (0-100), and carousel sort controls
- **Items**: Personal inventory with category navigation sidebar, radial fan list with per-item trapezoid clip-paths, sortable by name/days owned/price/daily cost, concentric arc divider, and tangent scroll indicator
- **Gallery**: Unified media hub (anime, games, TV, movies, books) with cover wall, category sidebar with quad selection highlight, rating/date/playtime sorting, detail views, and image proxy with disk cache

### AI Agent

- Persistent tool-calling backend with Rust-native LLM integration and JSONL session history
- Standalone CLI (`agent-cli`) for debug and Telegram bot (`agent-telegram`) for remote access
- Layered config loading (defaults, user-level `~/.arcana/agent_config.json`, project-level, env vars)
- Data write validation with automatic rollback on schema violation
- Shared services layer (`services/`) consumed by agent, `arcana-data` CLI, and Tauri commands
- Claude Code skills: `velvet-room` (progress reporting), `phan-site` (mission proposals), `pack-manager` (achievement pack management)

### Data & Tooling

- `arcana-data` CLI: unified data operations entry point with context reading, mission/status/achievement updates, changelog writes, and memory management
- Content pack system: pluggable achievement and skill packs under `data/packs/<pack_id>/`
- Onboarding tools with example data and interactive `init` command
- Python scripts for Bangumi, Steam, and Douban data import
- PostToolUse validation hook (`scripts/validate_data.py`) for Claude Code integration
- JSON-based local storage with `{"version": 1, ...}` envelope, no database dependency

### Platform

- Tauri v2 desktop app with global shortcut to toggle window visibility
- Fullscreen summon with auto-hide menu bar on macOS
- Configurable data directory
- Multi-resolution support with per-platform zoom factor handling
- Multi-LLM provider support (Anthropic, DeepSeek, and other OpenAI-compatible APIs)
