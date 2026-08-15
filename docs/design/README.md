# Arcana 目标设计文档

> **状态**：Target / 本机数据平台实现中
> **最后更新**：2026-08-15

本目录描述 Arcana 下一阶段的数据平台与领域模型。领域模型、DDL、SQLite migration runner、Repository adapter、本机运行时锁、`basic` Pack 初始化和 scalar/collection/event Record Commands 已落地，但 UI、CLI 与同步 Codec 尚未切换，因此当前程序还没有完整支持这里的结构。

尚未切换的 UI/CLI 行为仍以 [`docs/architecture.md`](../architecture.md) 与 [`docs/schema/`](../schema/README.md) 为准；新入口只实现本目录的目标模型，不导入或兼容旧 JSON。完成入口切换后删除旧实现与旧 Schema 文档。若两组文档冲突：

- 判断当前程序行为时，以当前架构文档和代码为准；
- 设计或实现新数据平台时，以本目录为准；
- 实现中若发现规范缺口，不得由实现者在代码里隐式决定；应先补充本目录并说明兼容影响。

## 阅读顺序

1. [`architecture.md`](./architecture.md)：目标架构、依赖方向和切换边界。
2. [`data_platform.md`](./data_platform.md)：SQLite、本地 JSON/Git 同步、所有权与初始化原则。
3. [`records.md`](./records.md)：统一事实层与 RecordDefinition/Record 语义。
4. [`sqlite_storage.md`](./sqlite_storage.md)：SQLite 运行时表、约束、事务和同步转换。
5. [`sync_migration.md`](./sync_migration.md)：仓库布局、锁、Git 同步、版本和数据库恢复。
6. [`status.md`](./status.md)：Pack Dimension、五项 UI 选择、子 Score 表达式、加权平均与等级。
7. [`achievements_skills_packs.md`](./achievements_skills_packs.md)：Achievement 用户状态、Skill、PackForest。
8. [`missions_memory.md`](./missions_memory.md)：Mission、MissionSuggestion 与 AssistantMemory。
9. [`agent_skills.md`](./agent_skills.md)：外部 Agent Skill、CLI 合约、分发和质量门。

## 设计状态

### 已确定

- 一个数据仓库对应一个用户；不建立 Profile、`profile_id` 或独立 UserSettings。
- SQLite 是本地运行时存储；确定性 JSON 与 Pack asset 是 Git 同步格式，其中 JSON 可供人工阅读和编辑。
- Record 是全局、相对扁平、用户所有的事实层。
- RecordDefinition 随 Pack 发放，并由已启用 Pack 派生出运行时注册表；不复制成全局持久化定义。
- RecordDefinition ID 使用 `<namespace>.<name>`；用户 Record 聚合在 `records/<namespace>.json`。namespace 与 Pack 相互独立。
- Pack 的结构化 JSON、asset 和 namespace Record 文件 Schema、基础类型、排序与 Definition 合并规则已经确定。
- Record/Pack Definition 的 SQLite 表、索引、事务和 unresolved 保留方式已经确定。
- `identity.nickname` 和 `identity.birth_date` 是 `basic` Pack 提供的普通 RecordDefinition；不存在特殊用户设置实体。
- 仓库根清单 `arcana.json` 只包含 `schema_version` 和 `enabled_pack_ids`。
- Status 固定为 Dimension + 子 Score 两层，Dimension 使用加权平均。
- Status 第一版表达式只读取数值 scalar Record，支持四则运算与 `min`/`max`/`abs`/`clamp`；Dimension Schema 和本机五项选择结构已经确定。
- Achievement 由 Agent 根据自然语言要求与 Record 判断，不建立完成规则 DSL。
- Achievement 只保留 `tracked` / `achieved` 最小用户状态；只有 `achieved` 计分。
- Pack 具有单父级层次，但父子关系只用于组织，不构成运行依赖。
- 未接受的 MissionSuggestion 只保留在本机；接受后的 Mission 才同步。
- AssistantMemory 同步长期语义记忆，不同步完整 Agent Session。
- 外部 Agent Skill 以 `plugins/arcana/skills` 为唯一源码，只通过版本化 `arcana-data` 合约修改数据，并受 fixture/eval 质量门约束。

### 实现进度与顺序

1. 已完成：Rust Domain Model、校验器、Repository interface、SQLite DDL/migration/adapter。
2. 进行中：进程间锁、运行时初始化与 Application Commands；Record 的 get/set/increment/correct、collection item、event 和 delete 命令已完成，其他领域命令待实现。
3. 待实现：确定性 Git JSON Codec、import/export 和 sync state。
4. 待实现：Status evaluator、Achievement/Skill 查询与 Mission/Memory command。
5. 待实现：Tauri UI、`arcana-data` contract 和 canonical Agent Skill；切换时直接停止使用旧 JSON。

### 暂不处理

- CRDT、多人协作和字段级自动合并。
- 永久 operation log、通用 tombstone 和服务端同步。
- Gallery/Items 数据迁入核心。
- 社区 Pack 的自动更新与依赖解析。
- Agent Session 跨设备同步。
- 跨分辨率 UI 与新用户引导；它们在数据平台稳定后处理。

## 文档变更规则

1. 先在本目录更新目标语义，再修改 Schema 和代码。
2. 每个持久化字段必须能回答：谁拥有、是否同步、如何验证、删除后怎样处理。
3. 派生数据原则上不持久化；若需要缓存，必须标明可重建和失效规则。
4. 示例 JSON、DDL 和约束属于实现合约；修改它们时必须同步更新版本、验证和兼容说明。
