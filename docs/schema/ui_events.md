# UI Events Schema

> **状态**：Legacy JSON v1 / 仅供旧内置 Agent
> 已迁移的 Achievement / Skill 页面在 mutation 成功后直接刷新 SQLite 派生查询，不再读取本队列。旧内置 Agent 完成迁移后删除该实现；它不属于用户同步数据。参见 [`docs/design/data_platform.md`](../design/data_platform.md)。

旧 Agent 数据变更事件队列。旧实现的数据变更方写入事件；当前前端不再消费。

## 文件路径

- `<data_dir>/ui_events.json`

## 顶层结构

```json
{
  "version": 1,
  "events": []
}
```

## `events[]`

**最多 100 条**，FIFO（超出时删除最旧的）。

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 唯一 ID，格式 `evt_{unix_secs}_{random_4char}` |
| `type` | string | 事件类型（见下方枚举） |
| `timestamp` | string | 事件发生时间，ISO 8601 |
| `data` | object | 事件载荷，结构由 `type` 决定 |

## 事件类型

### `achievement_status_change`

成就状态变更时触发。

| data 字段 | 类型 | 说明 |
|-----------|------|------|
| `achievement_id` | string | 成就 ID |
| `old_status` | string \| null | 变更前状态（`null` 表示新追踪，之前不存在） |
| `new_status` | string | 变更后状态：`"tracked"` / `"achieved"` |

## 旧消费规则

- 已删除的旧 Tauri command 曾通过 `get_pending_events` 读取并清除事件
- 消费是原子的：读取即删除，不会重复消费
- 可按 `type` 过滤消费
