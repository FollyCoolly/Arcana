# AGENTS.md

Arcana 是一个 Persona 5 风格的本地优先游戏化人生管理桌面应用，使用 Svelte 5/SvelteKit、TypeScript、Rust 和 Tauri v2。

## 权威来源

- `docs/architecture.md`：当前系统边界与代码分层。
- `docs/design/README.md`：核心数据平台与领域合约索引。
- `docs/visual_style_guide.md` 与 `docs/ui_design_spec.md`：UI 约束。
- `src-tauri/src/domain/` 与 `arcana-data help`：实现中的类型、校验和 CLI 表面。

实现与文档冲突时，先用当前代码和测试确认行为，再在同一变更中修正文档。详细 schema 和长期设计规则只维护在 `docs/design/`，不要复制到本文件。

## 工作约定

- 保持依赖方向：Svelte UI / external Skills → Tauri IPC / `arcana-data` → Application → Domain → Storage。
- 新业务行为先实现于 Domain/Application；IPC、CLI 和 Svelte 保持薄层，不复制领域规则。
- 核心写入必须经过 Application Commands；外部 Skill 或脚本必须经过 `arcana-data`。不要直接修改 SQLite 或绕过校验写 live JSON。
- 测试和开发实验使用临时 runtime/repository，不触碰用户的 `~/.arcana` 数据。
- Live JSON 保存可同步语义数据，SQLite 只保存 Records，`local-state.json` 保存本机 Suggestion 与 UI 选择。不要把实体写入错误的存储所有者。
- `batch apply` 只对 `record.*` 提供单 SQLite transaction 原子性。不要声称跨 SQLite/JSON 或跨多个 JSON 文件的操作具有 crash-atomic 保证。
- 修改领域模型时依赖现有 validators，并按 `docs/design/` 保持 ID 稳定、引用完整和派生关系无环。
- Commit message 使用 Conventional Commits 和祈使语气，例如 `fix(status): preserve local selection`。

修改 UI 时遵循 `docs/visual_style_guide.md` 与 `docs/ui_design_spec.md`；不要在这里重复视觉 token 或组件细节。

## Agent plugin 与生成文件

- `plugins/arcana/` 是外部 Agent plugin、Skills、fixtures 和 evals 的人工维护源码。
- 不要直接编辑 `.claude/skills/` 或 `.claude/fixtures/`。
- 修改 canonical Skills 或 fixtures 后运行 `python3 scripts/sync_agent_skills.py` 生成镜像。
- CLI contract 变化必须在同一变更中同步 capabilities、references、fixtures、evals 和 contract tests。

## 验证

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins --tests
python3 scripts/sync_agent_skills.py --check
git diff --check
```

- 先运行与改动最接近的测试，再运行适用的完整质量门。
- 前端或共享类型变更运行 `npm run check`。
- Rust、CLI、存储或领域变更运行 Rust format、test 和 clippy。
- Agent plugin、Skill 或 contract 变更运行同步检查及相关 contract tests。
- 跨层或发布相关变更运行全部检查，并视范围运行 `npm run build` 或 `npm run tauri build`。
- 纯文档变更至少运行 `git diff --check` 并验证引用的路径和命令。
- 无法运行的检查必须在交付时说明原因和未验证范围。

## Code Review Rules

- 标记绕过 Application/Domain 校验的写入，或实体存储所有权错误。
- 标记虚假的原子性保证，以及失败后可能留下但未报告的部分状态。
- 标记手改生成镜像，或 CLI contract 与 fixtures/evals/tests 不同步。
- 标记日志、fixtures、repository 或提交中的凭据、绝对私人路径和真实用户数据。
