# CLAUDE.md

Arcana 是一个 Persona 5 风格的游戏化人生管理桌面应用。项目正在从旧 JSON 运行时迁移到 SQLite 本地运行时与确定性 JSON 同步格式。

## 当前迁移边界

- Tauri UI 与内置 Rust Agent 暂时仍使用 `models/`、`services/` 和旧 JSON storage；只修复必要问题，不在这套模型上增加新数据平台功能。
- `arcana-data` 已完全停止暴露旧 JSON 命令，只使用 `application/`、`domain/`、`storage/sqlite/` 和 JSON Repository Codec。
- 旧 `context/read/mission/status/achievement/pack/changelog/memory` JSON 实现与旧 `.claude/skills` 已删除，不得恢复兼容层；`pack`、`status`、`achievement`、`mission`、`memory` 已按 SQLite 合约重新实现。
- 新数据不迁移旧 JSON；UI/Agent 完成切换后再删除剩余旧模块和 `docs/schema/` 迁移对照文档。
- 新数据平台的权威设计位于 `docs/design/`，旧 UI 的现状说明位于 `docs/architecture.md`。

## 项目结构

```text
src/                              SvelteKit 前端
src-tauri/src/
  application/                    新 typed commands 与运行时锁
  domain/                         新领域模型和校验
  storage/sqlite/                 SQLite migrations 与 Repository adapter
  storage/json_repository.rs      SQLite ↔ 确定性 JSON Codec
  bin/arcana_data.rs              新数据 CLI 入口
  bin/arcana_data/                CLI contract、Record、Pack、Status、Achievement、Skill、Mission、Memory、runtime/json 模块
  commands/ models/ services/     尚未迁移的 UI/Agent 旧实现
  agent/                          尚未迁移的内置 Rust Agent
docs/design/                      新数据平台权威文档
docs/schema/                      旧 JSON Schema，仅作迁移对照
data-example/                     旧 UI JSON 示例，不由新 init 使用
static/                           UI 静态资源
```

## 构建与检查

```bash
npm install
npm run dev
npm run tauri dev
npm run check

cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins --tests
```

提交前至少通过 `npm run check`、完整 Rust 测试、Rust fmt check 和 `git diff --check`。

## 新数据 CLI

当前命令：

```text
arcana-data capabilities
arcana-data init [--runtime <directory>]
arcana-data context [--runtime <directory>] summary
arcana-data record [--runtime <directory>] <action>
arcana-data pack [--runtime <directory>] <action>
arcana-data status [--runtime <directory>] <action>
arcana-data achievement [--runtime <directory>] <action>
arcana-data skill [--runtime <directory>] list
arcana-data mission [--runtime <directory>] <action>
arcana-data memory [--runtime <directory>] <action>
arcana-data json import|export ...
```

协议规则：

- 成功：退出码 0，stdout 直接输出业务 JSON。
- 失败：非零退出码，stdout 为空，stderr 只输出 `{code, message, details}` JSON。
- 不增加 `ok/data` 结果 Envelope；contract 与 Schema 版本只从 `capabilities` 获取。
- `--help` 面向人类，`--compact` 只改变 JSON 空白。
- CLI 不直接执行 SQL，不绕过领域校验，不读写旧 `<data_dir>` JSON。
- `pack write` 只替换结构化 Pack 内容并保留已有 asset；asset 只能通过 `pack asset-put|asset-delete` 修改。
- Status 分数/等级只从当前 Record 即时计算；五个 `status select` 展示位只保存在本机 SQLite。
- Achievement 只保存 tracked/achieved 与可选 achieved_at；prerequisites 不阻止用户直接确认完成，显式 revoke 不要求 Definition 可用。
- Skill 没有独立用户状态；节点可用性、积分和 Lv.0～Lv.5 每次从已启用 Pack 与 Achievement 状态即时计算。
- `json import|export` 不执行 Git 操作，也不覆盖已有导出目录。

## 新数据模型硬约束

- 一个 Repository 对应一个用户，不建立 Profile 或 `profile_id`。
- `identity.nickname`、`identity.birth_date` 是 `basic` Pack 的普通 scalar RecordDefinition。
- RecordDefinition ID 为 `<namespace>.<name>`；namespace 不等于 Pack。
- Record 是全局扁平事实；Pack 有单父级 PackForest，但父子关系不产生运行依赖。
- Pack 必须完整声明自身 Dimension/Achievement 使用的 RecordDefinition；启用时合并兼容定义，拒绝不兼容定义。
- Record 只有 scalar、collection、event 三种结构，不保存 `update_mode`、baseline 或 gauge/counter 分类。
- Status 使用 Dimension + 子 Score 两层，分数 clamp 到 0–100，再按权重平均；Lv.0 表示未解锁，Lv.1 为分数大于 0，另有四个 threshold。
- Achievement 只有 `tracked/achieved` 用户状态；只有 achieved 计分。Record 变化不自动撤销 achieved，但允许显式撤销。
- 不实现 `auto_unlock_rule`；Agent 根据自然语言要求、相关 Record 与可选 `tip` 判断。
- 只同步已接受 Mission；pending/rejected MissionSuggestion 仅本机。
- Mission 的 create/suggest 由系统生成 UUIDv7 与时间；complete/archive 是幂等生命周期命令，update 完整替换可编辑字段。
- AssistantMemory 同步长期语义，不同步 Agent Session。
- Memory create 由系统生成 UUIDv7 与时间；update 保留 ID/created_at，只在 kind/content 变化时刷新 updated_at；delete 是 hard delete。
- `context summary` 从单一事务快照返回选中的 Status、active Mission、显式 Achievement 状态与 AssistantMemory；不内嵌完整 Record 或 Pack。
- 不建立 changelog、operation log、通用 tombstone 或旧 JSON 迁移。

完整 Schema 和事务语义见：

- `docs/design/records.md`
- `docs/design/sqlite_storage.md`
- `docs/design/status.md`
- `docs/design/achievements_skills_packs.md`
- `docs/design/missions_memory.md`
- `docs/design/agent_skills.md`

## 代码规范

- Rust 领域规则放在 Domain/Application 层，CLI 只解析输入、分发 typed command 和呈现结构化结果。
- 所有读改写必须在 Repository transaction 内完成；不能由调用方先读再写。
- 新 CLI 错误码属于机器合约，修改时必须同步 contract fixture 和设计文档。
- 不把 SQLite row id、本机路径、凭证、Agent Session 或缓存字段写入同步 JSON。
- TypeScript/Svelte 使用 2 空格缩进和 strict 模式；Rust 遵循 `cargo fmt`。
- Commit 使用 Conventional Commits 和祈使语气。

## UI 设计约束

- 主色为 `#000000`、`#ffffff`、`#E5191C`；不使用渐变。
- 数据可视化可使用 `#F5A623`，装饰结构可使用 `#2E2E2E`。
- 所有交互元素必须有几何底座，不使用裸文字按钮。
- 动画时长：fast 120ms、base 180ms、slow 260ms；曲线为 `cubic-bezier(0.2, 0.8, 0.2, 1)`。
- 视觉实现遵循 `docs/visual_style_guide.md` 和 `docs/ui_design_spec.md`。
