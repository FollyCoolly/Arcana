# RecordSet 与 RecordData

> **状态**：Target / 领域语义已确定，物理 Schema 尚未定稿
> **最后更新**：2026-08-15

## 1. 定位

RecordData 是 Arcana 全局、相对扁平、用户所有的事实层。Status 和 Achievement 可以读取同一份事实，但不能各自保存重复副本。

```text
RecordSet Definition --defines--> RecordData
RecordData --read by--> selected Pack Dimension
RecordData --read by--> Agent evaluating Achievement
```

- RecordData 不属于 Pack。
- 关闭或删除 Pack 不删除 RecordData。
- 一个 RecordSet 可以关联多个 Achievement；一个 Achievement 也可以关联多个 RecordSet。
- Arcana Skill 不直接读取 RecordData，只读取状态为 `achieved` 的 Achievement。

## 2. RecordSet 与 RecordData

`RecordSet` 描述稳定 ID、数据形态、字段、单位和校验约束；`RecordData` 是用户明确记录的事实。

Pack 可以提供 RecordSet 模板和兼容要求，但模板采用后形成全局定义。Pack 是定义的来源之一，不是运行时数据的所有者。

## 3. 第一版数据形态

第一版只区分结构差异：

| kind | 含义 | 示例 |
| --- | --- | --- |
| `scalar` | 一个当前值 | 体重、身高、累计总数、当前偏好值 |
| `collection` | 有稳定身份、需要去重和增删的对象集合 | 学会的菜、完成的项目、读过的论文 |
| `event` | 独立发生且发生时间有意义的事件 | 跑步、训练、演出、考试 |

这不是 gauge/counter 或“当前值/累计值”的业务分类。Scalar 可以通过不同命令更新，但数据定义不固定 `update_mode`。

## 4. 写入命令与持久化语义

以下是命令语义，不是 RecordSet 类型：

- `set`：写入当前已知值；
- `increment`：基于当前 scalar 增加；
- `correct`：修正错误事实；
- `add_item` / `remove_item`：维护 collection；
- `append_event` / `correct_event` / `delete_event`：维护 event。

所有命令通过同一事务 API 执行。`increment` 必须在数据库事务中读取并更新，不能由调用方先读后写。

## 5. 标识、时间与来源

- RecordSet 使用全局、稳定、语义化 ID，例如 `health.body_weight`、`programming.projects`。
- ID 一旦被引用就不能直接改名；更名必须迁移所有引用。
- Collection/Event 条目优先使用外部稳定 ID 或领域自然 ID；没有合适 ID 时使用 UUIDv7。
- `effective_at` / `occurred_at` 表示事实成立或事件发生时间。
- `recorded_at` 表示信息进入 Arcana 的时间。
- `source` 保存轻量来源，例如 `manual`、`assistant`、`import`、`external`。
- 物理单位保存可读 display unit，并可附标准 unit code；优先采用 UCUM 语义。

## 6. 缺失与历史

- 缺失、unavailable、invalid 和真实数值 0 必须区分。
- Arcana 只保存用户明确提供或通过授权来源导入的数据。
- 不建立通用 baseline、历史估算量或虚构 collection/event 条目。
- Collection 数量默认表示“Arcana 已明确记录的条目数”，不承诺覆盖用户完整人生经历。
- 用户可以暂不补充历史数据；相关提醒属于 AssistantMemory，不参与分数。
- 用户后来提供具体历史事实时，使用真实发生时间写入，同时保留本次 `recorded_at`。

## 7. Pack 声明与兼容

每个 Pack 必须显式声明它读取的 RecordSet 以及兼容要求，不能依赖父 Pack 隐式提供定义。

启用 Pack 时：

1. 全局 RecordSet 不存在：从 Pack 模板创建。
2. 已存在且兼容：复用全局定义和用户数据。
3. 已存在但不兼容：显示结构差异并阻止启用。
4. Pack 只提供 ID、没有足够的定义或兼容要求：Pack 校验失败。

多个 Pack 重复声明同一个兼容 RecordSet 是允许的；运行时仍只有一份全局定义和一份用户数据。

第一版兼容规则至少应满足：新增可选字段可兼容；删除字段、改变类型、单位语义或既有字段含义属于破坏性修改。精确规则在物理 Schema 定稿时补充。

## 8. 概念示例

以下示例只说明领域含义，不是最终同步 JSON：

```json
{
  "id": "cooking.learned_dishes",
  "kind": "collection",
  "item_schema": {
    "name": "string",
    "learned_at": "date?"
  }
}
```

```json
{
  "record_set_id": "cooking.learned_dishes",
  "items": [
    {
      "id": "dish:tomato-and-eggs",
      "name": "番茄炒蛋",
      "learned_at": "2026-08-15",
      "recorded_at": "2026-08-15T20:30:00+08:00",
      "source": "assistant"
    }
  ]
}
```
