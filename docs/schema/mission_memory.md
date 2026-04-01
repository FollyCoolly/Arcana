# Mission Memory Schema

AI agent 的持久记忆，跨会话保留用户偏好和上下文，供 `phan-site`（任务生成）和 `velvet-room`（进度汇报）共同使用。

## 文件路径

- `data/mission_memory.json`

## 顶层结构

```json
{
  "version": 1,
  "last_generation": { ... } | null,
  "focus_areas": [],
  "patterns": { ... },
  "conversation_context": [],
  "completed_mission_log": []
}
```

## 字段说明

### `last_generation`

上次 `phan-site` 生成任务的信息。首次使用前为 `null`。

| 字段 | 类型 | 说明 |
|------|------|------|
| `date` | string | 生成日期，`YYYY-MM-DD` |
| `generation_id` | string | 批次 ID，通常与 date 相同 |
| `proposed_count` | number | 本次生成的任务数量 |
| `schedule` | string | 生成频率：`"daily"` / `"weekly"` 等 |

### `focus_areas[]`

用户当前的关注领域，由 AI 根据对话内容维护。

| 字段 | 类型 | 说明 |
|------|------|------|
| `area` | string | 领域名称（如 "Rust 编程"、"健身"） |
| `priority` | string | `"high"` / `"medium"` / `"low"` |
| `notes` | string | 补充说明 |
| `updated_at` | string | 最后更新日期，`YYYY-MM-DD` |

### `patterns`

用户的任务偏好模式，从接受/拒绝行为中学习。

| 字段 | 类型 | 说明 |
|------|------|------|
| `accepted_tags` | string[] | 用户倾向接受的任务类型标签 |
| `rejected_tags` | string[] | 用户倾向拒绝的任务类型标签 |
| `notes` | string | AI 对用户偏好的观察笔记 |

### `conversation_context[]`

与用户的对话摘要记录。**最多 20 条**，FIFO。

| 字段 | 类型 | 说明 |
|------|------|------|
| `date` | string | 对话日期，`YYYY-MM-DD` |
| `summary` | string | 一句话摘要 |
| `source` | string | 来源 skill：`"phan-site"` / `"velvet-room"` |

### `completed_mission_log[]`

已完成任务的简要记录。**最多 50 条**，FIFO。

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | Mission ID |
| `title` | string | 任务标题 |
| `completed_at` | string | 完成时间，ISO 8601 |
| `linked_achievement_id` | string \| null | 关联的成就 ID |
