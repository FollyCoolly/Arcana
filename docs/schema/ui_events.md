# UI Events Schema

UI 待消费事件队列。数据变更方写入事件，前端读取后清除。

## 文件路径

- `data/ui_events.json`

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

## 消费规则

- 前端通过 `get_pending_events` command 读取并清除事件
- 消费是原子的：读取即删除，不会重复消费
- 可按 `type` 过滤消费
