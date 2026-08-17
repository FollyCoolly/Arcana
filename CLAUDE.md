# CLAUDE.md

Arcana 是一个 Persona 5 风格的游戏化人生管理桌面应用。Pack/Definition 与可同步语义状态使用 live JSON repository，Records 使用 SQLite，本机 Suggestion/选择使用 runtime-local JSON。

## 当前架构边界

- Tauri Status、Achievement、Skill、Mission 与 `arcana-data` 共用 `application/`、`domain/` 和 composite `DataRepository`。
- 项目不包含内置 Agent、模型供应商集成或 Telegram 服务。canonical 外部 Skills 位于 `plugins/arcana/skills`，只通过 `arcana-data` 访问数据。
- `.claude/skills` 与 `.claude/fixtures` 由 `scripts/sync_agent_skills.py` 生成，不得手工维护。
- Items、Gallery、Weather 只读取 `~/.arcana/data` 中的外部来源配置，不属于核心用户数据。
- 不迁移旧 JSON，不恢复 changelog、operation log、通用 tombstone 或旧命令兼容层。
- Git 同步编排尚未实现；当前 `json import|export` 不调用 Git。

## 项目结构

```text
src/                              SvelteKit 前端
src-tauri/src/
  application/                    typed commands 与运行时锁
  domain/                         领域模型和校验
  storage/data_repository.rs      JSON/SQLite/local-state 组合 Repository
  storage/sqlite/                 Record-only migrations 与 adapter
  storage/json_repository.rs      live semantic JSON 与确定性 Codec
  storage/local_state.rs          runtime-local JSON
  commands/                       Tauri IPC
  models/                         Items/Gallery/Weather 外部适配模型
  bin/arcana_data.rs              数据 CLI 入口
  bin/arcana_data/                CLI contract 与各领域命令
docs/design/                      数据平台权威文档
plugins/arcana/                   canonical Agent plugin、Skills、fixtures、evals
.claude/skills|fixtures/          生成的兼容镜像
docs/schema/items.md              Items 外部来源合约
data-example/                     Items/Gallery/Weather 配置示例
```

## 构建与检查

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins --tests
python scripts/sync_agent_skills.py --check
git diff --check
```

## 数据 CLI 约定

- 成功：退出码 0，stdout 为业务 JSON。
- 失败：非零退出码，stdout 为空，stderr 为结构化错误 JSON。
- `capabilities` 不打开数据库；Skill 应先依据其确认 contract/schema 版本。
- `--dry-run` 必须执行完整读取和校验；SQLite mutation 在事务中回滚，JSON mutation 不落盘。
- `batch apply` 只允许 `record.*`，并在一个 SQLite transaction 中全成或全败。Pack、Achievement、Mission、Memory 与本机状态通过单命令更新 JSON。
- `json import|export` 持有运行时锁，但不执行 Git 操作。

## 数据模型约束

- 一个 live repository / SQLite runtime 对应一个用户，不建立 Profile 或 `profile_id`。
- `identity.nickname` 与 `identity.birth_date` 是 `basic` Pack 的普通 RecordDefinition。
- `identity.game_days` 是 `basic` Pack 的 DerivedValue；DerivedValue 由 JSON Definition 惰性计算，不写入 SQLite。
- RecordDefinition ID 为 `<namespace>.<name>`；Record 通过 `definition_id` 引用定义。
- Pack 父子关系只用于组织；跨 Pack 引用必须显式且不能依赖父级自动启用。
- 计算依赖单向流动：`Record -> DerivedValue -> Status Score -> Dimension Score`；Status Score 也可直接读取数值 Record。
- Status Dimension 是子 Score 的加权平均，Score 范围固定裁剪到 0～100，四个 threshold 派生 Lv.0～Lv.5。
- Achievement 仅保存 `tracked` / `achieved` 状态，不保存自动解锁规则或派生进度。
- MissionSuggestion 只在本机；接受后创建可同步 Mission。
- AssistantMemory 可同步；Agent Session、凭证与设备配置不同步。
