# Arcana 目标设计文档

> **状态**：Target / 尚未全部实现
> **最后更新**：2026-08-15

本目录描述 Arcana 下一阶段的数据平台与领域模型。它用于指导后续实现，不代表当前程序已经支持其中的结构。

当前代码和现有 JSON 文件仍以 [`docs/architecture.md`](../architecture.md) 与 [`docs/schema/`](../schema/README.md) 为准；新代码迁移完成后，再用本目录的目标模型替换旧 Schema 文档。若两组文档冲突：

- 判断当前程序行为时，以当前架构文档和代码为准；
- 设计或实现新数据平台时，以本目录为准；
- 尚未确定的内容不得由实现者自行扩大范围，应先更新文档中的“未决问题”。

## 阅读顺序

1. [`architecture.md`](./architecture.md)：目标架构、依赖方向和迁移边界。
2. [`data_platform.md`](./data_platform.md)：SQLite、本地 JSON/Git 同步、所有权与迁移原则。
3. [`record_data.md`](./record_data.md)：统一事实层与 RecordSet/RecordData 语义。
4. [`status.md`](./status.md)：Pack Dimension、五项 UI 选择、子 Score 表达式、加权平均与等级。
5. [`achievements_skills_packs.md`](./achievements_skills_packs.md)：Achievement 用户状态、Skill、PackForest。
6. [`missions_memory.md`](./missions_memory.md)：Mission、MissionSuggestion 与 AssistantMemory。

## 设计状态

### 已确定

- 一个数据仓库对应一个用户；不建立 Profile 或 `profile_id`。
- SQLite 是本地运行时存储；确定性 JSON 是 Git 同步和人工编辑格式。
- RecordData 是全局、相对扁平、用户所有的事实层。
- Status 固定为 Dimension + 子 Score 两层，Dimension 使用加权平均。
- Achievement 由 Agent 根据自然语言要求与 RecordData 判断，不建立完成规则 DSL。
- Achievement 只保留 `tracked` / `achieved` 最小用户状态；只有 `achieved` 计分。
- Pack 具有单父级层次，但父子关系只用于组织，不构成运行依赖。
- 未接受的 MissionSuggestion 只保留在本机；接受后的 Mission 才同步。
- AssistantMemory 同步长期语义记忆，不同步完整 Agent Session。

### 实现前仍需定稿

- SQLite 物理表、索引、迁移版本表和事务 API。
- Git JSON 的最终目录与每个文件的精确 JSON Schema。
- RecordSet 兼容判定以及破坏性迁移的声明格式。
- Status 安全表达式第一版的最小运算符和函数白名单。
- AssistantMemory 的稳定 ID、精炼、合并和清理策略。
- 旧数据迁移工具的完整字段映射与验收测试。
- 外部 Agent Skills 的更新和插件分发方式。

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
4. 示例 JSON 只表达领域结构，不提前承诺尚未确定的 SQLite 物理表。
