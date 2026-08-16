# 当前 JSON Schema 文档总览

本目录用于维护 Arcana 的数据结构规范文档，重点描述本地 JSON 文件的字段定义、约束和演进方式。

> [!IMPORTANT]
> 本目录描述旧内置 Agent 以及 Items/Gallery 尚在使用的 JSON v1 文件，只用于迁移对照。Status、Achievement、Skill、Mission 桌面页面已经迁移到 [`docs/design/`](../design/README.md) 定义的 SQLite 模型；本目录不再是这些页面的 current contract。

## 目标

- 统一各模块数据格式，避免前后端理解偏差。
- 为 Rust 数据结构和前端 TypeScript 类型提供单一依据。
- 支持后续字段扩展与版本迁移。

## 文档约定

- 模块文档命名：`<module>.md`，例如 `status.md`。
- `<data_dir>` 表示 `ARCANA_DATA_DIR`、`~/.arcana/settings.json` 或默认 `~/.arcana/data` 解析出的运行时数据目录。
- 每个文档建议包含：文件路径、字段定义、最小示例、校验规则、版本说明。
- 日期格式默认使用 `YYYY-MM-DD`。
- 时间戳如需精确时间，使用 ISO 8601（例如 `2026-02-10T20:15:00+08:00`）。

## 全局基础 Schema

### `user_profile.json`

用途：存储用户基础信息（身份相关，低频变更）。

路径：`<data_dir>/user_profile.json`

最小示例：

```json
{
  "username": "User01",
  "birth_date": "1998-01-01"
}
```

字段说明：

- `username` (`string`, 必填)：显示名或用户名。
- `birth_date` (`string`, 必填)：出生日期，格式 `YYYY-MM-DD`。

目标架构不再保留 Profile 或 UserSettings 实体。迁移时，用户名和生日分别成为 `basic` Pack 定义的 `identity.nickname`、`identity.birth_date` scalar Record。参见 [`docs/design/architecture.md`](../design/architecture.md)。

## 模块 Schema 索引

| 当前文档 | 当前实现 | 目标方向 |
| --- | --- | --- |
| [`status.md`](./status.md) | definitions + `status.json` + 旧评分 | Record + Pack Dimension 两层评分 |
| [`content_packs.md`](./content_packs.md) | loaded pack 列表与三文件 Pack | RecordDefinition、Dimension 定义与 PackForest |
| [`achievements.md`](./achievements.md) | tracked/achieved progress map | 精简为最小用户状态，只让 achieved 计分 |
| [`skills.md`](./skills.md) | Achievement 节点与积分等级 | 只由 achieved Achievement 派生，按 Pack 独立计算 |
| [`missions.md`](./missions.md) | current/archive + proposed/rejected | 统一 Mission；Suggestion 只在本机 |
| [`mission_memory.md`](./mission_memory.md) | 任务生成器和对话状态混合 | 可同步的长期语义 AssistantMemory |
| [`ai_changelog.md`](./ai_changelog.md) | AI 文件级审计日志 | 退出新核心，由 Git 历史和迁移备份替代 |
| [`ui_events.md`](./ui_events.md) | JSON 待消费事件队列 | 仅本机运行时事件，不进入同步仓库 |
| [`items.md`](./items.md) | Obsidian/Markdown 外部数据源 | 第一阶段继续由外部来源拥有 |

## Current → Target 文档映射

- 数据平台与迁移：[`../design/data_platform.md`](../design/data_platform.md)
- RecordDefinition / Record：[`../design/records.md`](../design/records.md)
- Status：[`../design/status.md`](../design/status.md)
- Achievement / Skill / Pack：[`../design/achievements_skills_packs.md`](../design/achievements_skills_packs.md)
- Mission / Memory：[`../design/missions_memory.md`](../design/missions_memory.md)
- SQLite 物理 Schema：[`../design/sqlite_storage.md`](../design/sqlite_storage.md)
- Git JSON、同步与迁移：[`../design/sync_migration.md`](../design/sync_migration.md)
- Agent Skill 接入：[`../design/agent_skills.md`](../design/agent_skills.md)

## 实现顺序

1. 按目标文档实现 Rust 领域模型、Repository、校验器和迁移器。
2. 前端按同一 Schema 维护 TypeScript 类型，避免手写漂移。
3. 逐模块完成迁移和回归测试。
4. 全量迁移稳定后，将本目录改写为新的 current 文档；在此之前继续保留为旧数据说明。
