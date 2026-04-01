# AI Changelog Schema

记录 AI agent skill 对数据文件的所有修改，支持审计和回滚。

## 文件路径

- `data/ai_changelog.json`

## 顶层结构

```json
{
  "version": 1,
  "entries": []
}
```

## `entries[]`

**最多 200 条**，FIFO（超出时删除最旧的）。

| 字段 | 类型 | 说明 |
|------|------|------|
| `timestamp` | string | 变更时间，ISO 8601 |
| `skill` | string | 执行变更的 skill：`"phan-site"` / `"velvet-room"` |
| `summary` | string | 人类可读的变更摘要 |
| `changes` | array | 具体变更列表 |

### `changes[]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `file` | string | 被修改的文件名（如 `"missions.json"`） |
| `type` | string | `"add"` / `"update"` / `"delete"` |
| `target` | string | 被修改的对象标识（如 mission ID、achievement ID、metric name） |
| `field` | string | 被修改的字段名（update 时使用） |
| `old_value` | any | 修改前的值（用于回滚） |
| `new_value` | any | 修改后的值 |
| `detail` | string | 补充说明（add/delete 时使用） |

## 使用规则

1. 每次 AI skill 修改数据文件时，**必须**写入 changelog
2. AI 回复中**必须**展示变更摘要
3. 用户可要求回滚：AI 读取 `old_value` 恢复数据
4. `mission_memory.json` 的更新不需要记录在 changelog 中（这是 AI 内部状态）
