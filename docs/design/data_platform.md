# 数据平台、同步与版本

> **状态**：Target / 同步、存储与版本协议已确定
> **最后更新**：2026-08-15

## 1. 存储职责

Arcana 使用两种互补格式：

| 层 | 格式 | 职责 |
| --- | --- | --- |
| 本地运行时 | SQLite | 事务、并发、约束、索引、迁移和可靠读写 |
| 个人同步 | 确定性 JSON + Pack asset + Git | 人工阅读、手动编辑、资源分发、版本历史和设备同步 |

SQLite 文件不进入 Git；同步仓库中的 JSON 和 Pack asset 不作为应用运行时存储。二者只能通过同一套 Rust 领域模型转换：

```text
SQLite Adapter <-> Domain Model <-> Sync Repository Codec
```

当前实现先提供不依赖 Git 的 `JsonRepositoryCodec`：SQLite 可以导出到一个全新的规范 JSON 目录，也可以从完整 JSON 目录经全量校验后创建缺失的 SQLite，或在单个事务中替换既有数据库的同步实体；既有数据库的本机专属数据继续保留。CLI 入口为 `arcana-data init [--runtime <directory>]`、`arcana-data record [--runtime <directory>] ...`、`arcana-data pack [--runtime <directory>] ...`、`arcana-data json export --output <directory> [--runtime <directory>]` 和 `arcana-data json import --input <directory> [--runtime <directory>]`；省略 `--runtime` 时读取本机 settings 或默认运行时目录。Codec 不覆盖已有目录、不生成 `.gitattributes`、不读取 Git 状态，也不执行 add/commit/pull/push。已有工作区的 digest 防覆盖、崩溃恢复 journal 和 Git 命令属于后续同步编排层。

## 2. 为什么选择 SQLite

- Tauri UI、CLI 和外部 Skill 可能从不同进程写入，需要事务和并发控制。
- 数据库版本升级、引用完整性和索引不应由各模块自行实现。
- SQLite 成熟、跨平台、无需独立服务，并且便于备份和原子替换。
- 领域层仍提供强类型 Repository / Document 风格 API，业务模块不直接拼 SQL。

本设计不依赖关系型查询作为业务 API；选择 SQLite 是为了本地可靠性，不是要求所有领域对象采用高度规范化的表结构。

## 3. 同步边界

### 3.1 同步

- Pack 中的 RecordDefinition 与按 namespace 分组的用户 Record
- 已接受的 Mission
- 用户成就状态（`tracked` / `achieved`）
- Pack 内容和 enabled 状态
- 长期语义 AssistantMemory

### 3.2 仅本机

- pending/rejected MissionSuggestion
- Dashboard、Weather、窗口、五个 Status Dimension 选择和其他设备设置
- 数据目录、Git 工作区和同步游标
- Gallery/Items 连接路径
- 模型、Telegram、外部平台等配置和凭证
- Agent Session、UI event queue 和缓存
- `last_generation` 等一次运行的生成器状态

### 3.3 外部权威

- Gallery 第一阶段由外部平台拥有；Arcana 只通过适配器读取。
- Items 第一阶段继续由 Markdown/Obsidian 等外部来源拥有。
- 第一阶段核心不把外部来源伪装成持久化 Record。后续适配器可以增加只读事实查询，但必须明确区分 unavailable 与数值 0；只有用户显式导入后，事实才进入同步 Record。

### 3.4 只计算

- Status 子 Score、Dimension 分数与等级
- Arcana Skill 积分与等级
- Achievement 的即时进度说明
- BMI、游戏天数、剩余天数和聚合统计

## 4. Git JSON 约束

同步仓库根目录包含最小清单 `arcana.json`：

```json
{
  "schema_version": 1,
  "enabled_pack_ids": [
    "basic",
    "cooking"
  ]
}
```

- `schema_version` 是整个用户仓库格式的必填整数版本。
- `enabled_pack_ids` 是必填、去重并按字典序排序的 Pack ID 列表；每个 ID 必须指向仓库内存在且有效的 Pack。
- `arcana.json` 不保存 Profile、用户业务事实、本机配置、同步游标、导出时间、文件索引、条目数量或校验和。
- Pack 是可独立分发的内容单元，其 manifest 必须包含独立的 `schema_version`；第一版不保存内容版本，也不使用通用 Envelope 抽象。

同步 Repository Codec 必须保证：

- 对象和列表采用稳定排序；
- 日期、时间和单位采用规范格式；
- ID 稳定，不因导出顺序改变；
- 可选空字段按 Schema 约定省略；
- 同一领域状态重复导出得到相同文本；
- JSON 中不出现 SQLite row id、缓存字段和本机路径；
- Pack asset 原始 bytes 不转码，并与结构化 Pack 内容一起参与原子 import/export；
- 任何 API key、token、cookie 或凭证都被拒绝导出。

允许用户手动编辑 JSON，但修改后的数据必须先经过全仓库校验和原子导入。应用保存最近一次成功 import/export 时全部 managed paths 的内容 digest，避免用旧 SQLite 快照覆盖较新的人工编辑；该 digest 不是 Git commit ID，也不进入同步仓库。

同步 JSON 是便于阅读的明文，Arcana 不对仓库内容额外加密。包含个人 Record、Mission、Achievement 状态和 Memory 的远端仓库应使用 private repository，并由用户自行管理访问权限。HTTPS token、SSH key 和 Git credential 只交给系统 Git/credential manager，绝不写入 `arcana.json`、Pack、SQLite 同步实体或 commit message。

## 5. 导入与冲突

导入按整个仓库处理，而不是逐文件尽力而为：

1. 拒绝包含未解决 Git conflict marker 的工作区。
2. 解析全部 JSON。
3. 校验 Schema、稳定 ID、唯一性、Pack/RecordDefinition/Achievement 引用和表达式语法。找不到 Definition 的用户 Record/Achievement 状态标记为 unresolved 并保留，不允许普通更新，但不能因此被静默删除。
4. 在同一运行时目录创建临时 SQLite，执行全部 migration 并构建完整状态。
5. 执行 round-trip 导出，并比较实体数量、ID、引用和 Pack asset digest。
6. 全部成功后原子切换；任一步失败都保留原数据库。

不实现 CRDT、字段级自动合并或“选一个看起来合理的值”。用户按普通 Git 工作流解决冲突后重新导入。

## 6. 删除与历史

- 常规删除使用 SQLite hard delete 和普通 Git delete。
- 不为了个人顺序同步建立永久 operation log 或通用 tombstone。
- Git commit 提供粗粒度历史，新的核心模型不再依赖 `ai_changelog.json`。
- `achieved` 状态不会因 Record 变化自动消失，但允许显式撤销。
- 关闭或删除 Pack 不自动删除用户 Record 或用户成就状态；引用该 Pack Dimension 的本机 UI 选择应报告配置错误。

## 7. Git 同步与首次初始化

仓库 managed paths、OS file lock、防覆盖 revision/digest、import/export、fast-forward Git 命令、SQLite migration runner 和失败恢复完整定义在 [`sync_migration.md`](./sync_migration.md)。

核心约束：

- SQLite 与 Git managed paths 同时变化时停止并暴露 `both_changed`，不能选择一侧覆盖另一侧。
- 自动 pull 只允许 fast-forward；不自动 merge、rebase 或解决冲突。
- 新数据平台只支持创建全新的 v1 用户仓库与 SQLite；不读取、不转换旧版应用 JSON，也不提供旧数据回滚入口。
- UI/CLI 切换前旧实现可以继续存在于代码树中，但新旧数据层不得同时作为权威写入源；切换完成后旧 JSON 只是不受 Arcana 管理的普通文件。

## 8. SQLite 物理结构

Record、Pack Definition、连接设置、事务和 Git JSON 转换见 [`sqlite_storage.md`](./sqlite_storage.md)。该结构采用按 kind 拆表与动态 JSON payload 的混合方案，不采用 EAV。

## 9. 实现依据

- [`records.md`](./records.md)：Record 同步 Schema 与 Definition 兼容。
- [`status.md`](./status.md)：Dimension、表达式与本机选择。
- [`achievements_skills_packs.md`](./achievements_skills_packs.md)：Pack、Achievement 和 Skill。
- [`missions_memory.md`](./missions_memory.md)：Mission、Suggestion、Dashboard 和 Memory。
- [`sqlite_storage.md`](./sqlite_storage.md)：完整 SQLite DDL 与事务语义。
- [`sync_migration.md`](./sync_migration.md)：锁、同步、版本、初始化和失败恢复。
