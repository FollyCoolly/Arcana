# Record-only SQLite 存储

> **状态**：Current / Schema version 2
> **最后更新**：2026-08-16

SQLite 只负责 Records。Pack、Definition、AchievementState、Mission、AssistantMemory 和 UI 本机状态均不在 SQLite 中。选择 SQLite 而不是专用 KV 的理由是：跨进程事务成熟、无需服务、迁移可靠，并适合 scalar/collection/event 三种结构化 Record 的一致更新。

## 1. 文件与连接

- 默认数据库：`~/.arcana/runtime/arcana.sqlite3`。
- WAL/SHM、`arcana.lock` 与 migration 状态不进入 Git。
- migration runner 按版本执行内置 SQL，并记录 checksum；当前目标版本为 2。
- Repository 启用 foreign keys、busy timeout，并在迁移/替换前 checkpoint。

## 2. Schema v2

规范 DDL 位于：

- `src-tauri/src/storage/sqlite/migrations/0001_initial.sql`：历史 v1 起点；
- `src-tauri/src/storage/sqlite/migrations/0002_record_only.sql`：删除非 Record 表；
- `src-tauri/src/storage/sqlite/record_repository.rs`：当前 adapter。

迁移完成后只保留：

| 表 | 作用 |
| --- | --- |
| `records` | 每个 `definition_id` 的 Record header 与 kind |
| `scalar_records` | scalar value 与记录时间 |
| `collection_items` | collection 的稳定 `item_id` 与 fields |
| `event_entries` | event 的稳定 `event_id`、发生时间与 fields |
| `schema_migrations` | 已执行 migration/version/checksum |
| `sync_state` | 数据库内部 revision/后续同步元数据 |

Record 不对 JSON 中的 RecordDefinition 建数据库外键。Definition 可能因 Pack 停用、删除、Git 冲突或人工编辑暂时缺失；用户事实仍必须保留并在查询时标记 unresolved。

## 3. Record 结构与约束

- `definition_id` 是业务主键，不另建对外 row ID。
- `kind` 为 `scalar`、`collection` 或 `event`，创建后不能原地改变。
- scalar 恰好拥有一个 scalar payload；collection/event 的子项使用 caller-supplied 稳定 ID。
- payload JSON 只保存领域字段；类型、required、unit 和日期格式根据 live JSON Definition registry 在 Repository commit 前校验。
- 删除 collection item/event entry 与修正使用同一 transaction，不使用 append-only oplog。

## 4. 事务

单条 Record command 与多操作 Record batch 使用同一 mutation 内核：

1. 从 live JSON 构造 Definition registry；
2. 开启 SQLite transaction；
3. 按顺序执行并校验 Record operations；
4. `--dry-run` rollback，正式执行 commit；
5. 任一失败 rollback，并返回 `operation_index` 与 `operation`。

`batch apply` 不接受 Pack、Achievement、Mission、Memory、Suggestion 或本机选择操作，因为它们不属于该 SQLite transaction。`DataRepository` 也拒绝普通 transaction 同时修改 Record 与 JSON/local state。

## 5. Schema v1 升级

如果数据库仍为 v1，runtime 在执行删除表的 v2 migration 前读取旧 semantic/local 数据并写入：

- configured live JSON repository；
- `runtime/local-state.json`。

随后 v2 migration 删除 `packs`、`pack_*`、`achievement_states`、`missions`、`assistant_memories`、`mission_suggestions`、`status_dimension_selection` 和 Dashboard 等旧表。Records 原样保留。升级失败应返回 storage error，不把旧版其他 JSON 当作回退来源。

## 6. 派生查询

DerivedValue、Status、Skill 和 context summary 会在 runtime lock 下组合 JSON Definitions、SQLite Records 与 local state 后即时计算。派生值、分数、等级、进度、游戏天数和剩余天数均不写回数据库。
