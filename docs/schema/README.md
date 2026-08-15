# 当前 JSON Schema 文档总览

本目录用于维护 Arcana 的数据结构规范文档，重点描述本地 JSON 文件的字段定义、约束和演进方式。

> [!IMPORTANT]
> 本目录当前描述已实现的 v1 JSON 文件。下一阶段的 SQLite + Git JSON 目标模型见 [`docs/design/`](../design/README.md)。目标模型的物理 Schema 尚未定稿，因此在迁移完成前不会直接覆盖这里的现行规范。

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

目标架构中该文件迁移为不带 Profile ID 的 optional `UserSettings`，只保留 `nickname?` 和 `birth_date?`。参见 [`docs/design/architecture.md`](../design/architecture.md)。

## 模块 Schema 索引

| 当前文档 | 当前实现 | 目标方向 |
| --- | --- | --- |
| [`status.md`](./status.md) | definitions + `status.json` + 旧评分 | RecordData + Pack Dimension 两层评分 |
| [`content_packs.md`](./content_packs.md) | loaded pack 列表与三文件 Pack | RecordSet 模板、Dimension 定义与 PackForest |
| [`achievements.md`](./achievements.md) | tracked/achieved progress map | 精简为最小用户状态，只让 achieved 计分 |
| [`skills.md`](./skills.md) | Achievement 节点与积分等级 | 只由 achieved Achievement 派生，按 Pack 独立计算 |
| [`missions.md`](./missions.md) | current/archive + proposed/rejected | 统一 Mission；Suggestion 只在本机 |
| [`mission_memory.md`](./mission_memory.md) | 任务生成器和对话状态混合 | 可同步的长期语义 AssistantMemory |
| [`ai_changelog.md`](./ai_changelog.md) | AI 文件级审计日志 | 退出新核心，由 Git 历史和迁移备份替代 |
| [`ui_events.md`](./ui_events.md) | JSON 待消费事件队列 | 仅本机运行时事件，不进入同步仓库 |
| [`items.md`](./items.md) | Obsidian/Markdown 外部数据源 | 第一阶段继续由外部来源拥有 |

## Current → Target 文档映射

- 数据平台与迁移：[`../design/data_platform.md`](../design/data_platform.md)
- RecordSet / RecordData：[`../design/record_data.md`](../design/record_data.md)
- Status：[`../design/status.md`](../design/status.md)
- Achievement / Skill / Pack：[`../design/achievements_skills_packs.md`](../design/achievements_skills_packs.md)
- Mission / Memory：[`../design/missions_memory.md`](../design/missions_memory.md)

## 下一步建议

1. 先在 [`docs/design/`](../design/README.md) 定稿目标物理 Schema 和迁移规则。
2. 同步在 Rust 端定义领域模型、Repository 与反序列化校验。
3. 模块完成迁移时，再用目标 Schema 替换对应的现行文档，避免文档先于代码宣称已实现。
4. 前端按 Schema 维护对应 TypeScript 类型，避免手写漂移。
