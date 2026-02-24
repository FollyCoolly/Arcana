# Repository Guidelines

## Project Structure & Module Organization
This repository is a Tauri v2 desktop app with a SvelteKit frontend.

- `src/`: Svelte app code (`routes/+page.svelte`, global styles in `app.css`, app shell in `app.html`)
- `src-tauri/`: Rust backend (`src/lib.rs` commands + business logic, `src/main.rs` entrypoint, `tauri.conf.json` app config)
- `static/`: static assets (icons, SVGs)
- `docs/`: architecture, directory evolution, schema docs, and visual design guidelines
- `data/`: local JSON data (`user_profile.json`, `status.json`, etc.) used at runtime; ignored by Git

## Build, Test, and Development Commands
- `npm install`: install JS dependencies.
- `npm run dev`: run Vite dev server (frontend only).
- `npm run tauri dev`: run full desktop app in development.
- `npm run build`: build frontend to `build/` (used by Tauri build).
- `npm run tauri build`: build distributable desktop bundles.
- `npm run check`: run Svelte/TypeScript checks (`svelte-check`).
- `cargo test --manifest-path src-tauri/Cargo.toml`: run Rust tests (currently minimal).
- `cargo fmt --manifest-path src-tauri/Cargo.toml`: format Rust code before PRs.

## Coding Style & Naming Conventions
- TypeScript/Svelte: 2-space indentation, strict typing (`tsconfig.json` has `"strict": true`), and descriptive type names (e.g., `StatusMetric`).
- Use `PascalCase` for Svelte component/type names, `camelCase` for functions/variables.
- Rust: `snake_case` for functions/modules, `PascalCase` for structs/enums; keep Tauri commands explicit and error messages actionable.
- Keep module boundaries aligned between UI and backend (status-related frontend logic should map to status-related backend commands).

## Testing Guidelines
Automated frontend unit tests are not set up yet. For now, treat these as required pre-PR checks:

- `npm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- manual smoke test via `npm run tauri dev` (verify status data loads and global shortcut toggles window visibility)

When adding tests, use `*.test.ts` naming on frontend and Rust `#[cfg(test)]` modules near implementation.

## Commit & Pull Request Guidelines
History favors concise, prefixed commit subjects: `feat:`, `fix(scope):`, `docs:`/`doc:`. Follow that style in imperative mood, e.g., `feat(status): add BMI card`.

PRs should include:
- a clear summary and rationale
- linked issue/task (if any)
- screenshots or short recordings for UI changes
- notes on schema/config changes (especially anything affecting `data/*.json`)
