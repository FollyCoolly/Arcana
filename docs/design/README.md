# Arcana 数据与领域设计

> **状态**：Current
> **最后更新**：2026-08-17

本目录是 Arcana 核心数据平台与领域模型的权威设计说明。当前实现采用三个明确的数据所有者：live JSON repository 保存可读、可同步的语义数据；SQLite 只保存 Records；runtime-local JSON 保存不跨设备的 UI/推荐状态。

## 阅读顺序

1. [architecture.md](./architecture.md)：分层、依赖与写入边界。
2. [data_platform.md](./data_platform.md)：数据所有权、JSON repository 和导入导出。
3. [records.md](./records.md)：Record 与 RecordDefinition。
4. [derived_values.md](./derived_values.md)：具名、可复用且不持久化的派生值。
5. [sqlite_storage.md](./sqlite_storage.md)：Record-only SQLite DDL 与事务。
6. [achievements_skills_packs.md](./achievements_skills_packs.md)：Pack、Achievement、Skill。
7. [status.md](./status.md)：Dimension、Score、表达式与本机选择。
8. [missions_memory.md](./missions_memory.md)：Mission、Suggestion 与 AssistantMemory。
9. [agent_skills.md](./agent_skills.md)：外部 Skill/CLI 合约。
10. [sync_migration.md](./sync_migration.md)：当前初始化/升级，以及尚未实现的 Git 闭环。

## 已确定的原则

- 一个数据仓库对应一个用户；不建立 Profile 或 `profile_id`。
- `identity.nickname` 与 `identity.birth_date` 是 `basic` Pack 提供的普通 RecordDefinition。
- Definitions 与 Pack 层级来自 JSON；Records 保持扁平并存入 SQLite。
- 计算链是单向的 `Record -> DerivedValue -> Status Score -> Dimension Score`；Status Score 也可直接读取数值 Record。
- PackForest 父子关系只用于组织，不级联启用，也不是隐式运行依赖。
- Status 只有一层子 Score 和一层加权平均；所有结果限制在 `0..100`。
- Achievement requirement 使用自然语言与可选 `tip`，不建立自动解锁规则 DSL。
- 已接受 Mission、AchievementState 与 AssistantMemory 同步；未接受 Suggestion 和 UI 选择仅本机保存。
- Git 提供个人同步和历史；不建立无限增长的 changelog/op log，不自动解决 Git 冲突。

## 当前实现状态

- Rust Domain Model、验证器、Application Commands 与 composite `DataRepository` 已实现。
- JSON repository 是 Pack/Definition/语义用户状态的运行时权威源。
- SQLite Schema v2 只包含 Record 表、migration 与 sync metadata。
- 多操作 batch 只支持 Record mutation。
- 确定性 JSON import/export 已实现；Git pull/commit/push 编排尚未实现。
- 旧 SQLite Schema v1 在首次打开时会把仍有用的语义/本机数据升级到新存储边界，再迁移为 v2；更早的旧版应用 JSON 不迁移。
