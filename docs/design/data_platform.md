# 数据平台、同步与迁移

> **状态**：Target / 物理 Schema 尚未定稿
> **最后更新**：2026-08-15

## 1. 存储职责

Arcana 使用两种互补格式：

| 层 | 格式 | 职责 |
| --- | --- | --- |
| 本地运行时 | SQLite | 事务、并发、约束、索引、迁移和可靠读写 |
| 个人同步 | 确定性 JSON + Git | 人工阅读、手动编辑、版本历史和设备同步 |

SQLite 文件不进入 Git；同步 JSON 不作为应用运行时数据库。二者只能通过同一套 Rust 领域模型转换：

```text
SQLite Adapter <-> Domain Model <-> Sync JSON Codec
```

## 2. 为什么选择 SQLite

- Tauri UI、CLI 和外部 Skill 可能从不同进程写入，需要事务和并发控制。
- 数据迁移、引用完整性和索引不应由各模块自行实现。
- SQLite 成熟、跨平台、无需独立服务，并且便于备份和原子替换。
- 领域层仍提供强类型 Repository / Document 风格 API，业务模块不直接拼 SQL。

本设计不依赖关系型查询作为业务 API；选择 SQLite 是为了本地可靠性，不是要求所有领域对象采用高度规范化的表结构。

## 3. 同步边界

### 3.1 同步

- `UserSettings`
- RecordSet 与用户 RecordData
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
- 外部来源可以暴露为只读 Record 查询，但 unavailable 与数值 0 必须区分。

### 3.4 只计算

- Status 子 Score、Dimension 分数与等级
- Arcana Skill 积分与等级
- Achievement 的即时进度说明
- BMI、游戏天数、剩余天数和聚合统计

## 4. Git JSON 约束

同步 Codec 必须保证：

- 对象和列表采用稳定排序；
- 日期、时间和单位采用规范格式；
- ID 稳定，不因导出顺序改变；
- 可选空字段按 Schema 约定省略；
- 同一领域状态重复导出得到相同文本；
- JSON 中不出现 SQLite row id、缓存字段和本机路径；
- 任何 API key、token、cookie 或凭证都被拒绝导出。

允许用户手动编辑 JSON，但修改后的数据必须先经过全仓库校验和原子导入。应用应保存最近一次成功导入的 Git tree hash，避免用旧 SQLite 快照覆盖较新的人工编辑。

## 5. 导入与冲突

导入按整个仓库处理，而不是逐文件尽力而为：

1. 拒绝包含未解决 Git conflict marker 的工作区。
2. 解析全部 JSON。
3. 校验 Schema、稳定 ID、唯一性、Pack/RecordSet/Achievement 引用和表达式语法。
4. 在临时 SQLite 或单个事务中构建完整状态。
5. 执行 round-trip 导出，并比较实体数量、ID 和引用。
6. 全部成功后原子切换；任一步失败都保留原数据库。

不实现 CRDT、字段级自动合并或“选一个看起来合理的值”。用户按普通 Git 工作流解决冲突后重新导入。

## 6. 删除与历史

- 常规删除使用 SQLite hard delete 和普通 Git delete。
- 不为了个人顺序同步建立永久 operation log 或通用 tombstone。
- Git commit 提供粗粒度历史，新的核心模型不再依赖 `ai_changelog.json`。
- `achieved` 状态不会因 RecordData 变化自动消失，但允许显式撤销。
- 关闭或删除 Pack 不自动删除用户 RecordData 或用户成就状态；引用该 Pack Dimension 的本机 UI 选择应报告配置错误。

## 7. 首次迁移

首次迁移必须可回滚：

1. 只读扫描并验证旧 JSON。
2. 复制完整备份并记录文件哈希。
3. 创建 `arcana.db.new` 或等价临时数据库。
4. 在单个事务中导入所有目标实体。
5. 从新数据库导出目标 JSON。
6. 做 round-trip、数量、ID、字段和引用比较。
7. 验证成功后原子切换为活动数据库。
8. 保留旧 JSON 只读备份，直到用户明确清理。

主要映射：

| 旧数据 | 目标实体 |
| --- | --- |
| `user_profile.json` | optional UserSettings |
| status metric definitions 中的事实定义 | RecordSet |
| `status.json` | scalar RecordData |
| status dimensions/scoring | 迁移到用户维护 Pack 中的 DimensionDefinition |
| achieved achievement progress | 保留为 `achieved` 用户状态，只保留可选 `achieved_at` |
| tracked achievement progress | 保留为 `tracked` 用户状态，移除旧进度详情字段 |
| current/archive mission | 统一 Mission |
| proposed/rejected mission | 本机 MissionSuggestion |
| loaded pack files | Pack 内容与 enabled 状态 |
| mission memory | 清理后的 AssistantMemory |

旧实现把 tracked Achievement 计入 Skill；新模型只计算状态为 `achieved` 的 Achievement。迁移报告必须明确提示由此造成的积分或等级下降。

## 8. 尚未确定

- SQLite 物理表、索引和 migration table。
- Git JSON 最终目录布局与逐文件 JSON Schema。
- 数据库锁与 Git sync lock 的具体实现。
- RecordSet 破坏性迁移的声明格式。
- 完整 migration report 和 rollback CLI 的命令界面。
