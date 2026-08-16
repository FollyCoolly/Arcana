# Mission 与 AssistantMemory

> **状态**：Mission Domain、Application、CLI 与 Tauri UI 已实现；AssistantMemory Domain/Application/CLI 已实现
> **最后更新**：2026-08-16

## 1. MissionSuggestion 与 Mission

AI 生成但用户尚未接受的内容不是正式 Mission：

```text
MissionSuggestion --accept--> Mission
```

### MissionSuggestion

- 状态为 pending 或 rejected；
- 只保留在本机，不进入 Git 同步；
- 接受时可以沿用稳定 ID 创建 Mission；
- 拒绝后的长期偏好可以被精炼进 AssistantMemory，但建议实体本身不同步。

### Mission

- 只有用户接受后才创建并同步；
- active、completed、archived 是同一实体的生命周期状态；
- 不再在 current/archive 文件之间搬运成不同类型；
- parent-child 继续通过稳定 Mission ID 表达；
- `days_remaining` 等值按需计算，不持久化。

主菜单 countdown、hints、progress 等展示选择属于本机 Dashboard 配置，不属于 Mission 领域实体，也不跨设备同步。

桌面 Mission 页面分别查询同步 Mission 与本机 pending MissionSuggestion；接受、拒绝、完成和归档使用独立 typed command。Mission 详情可以设置或清除四个本机 Dashboard slot，完成或归档后失效的引用继续保留并作为 unresolved slot 返回，不静默改写用户配置。

## 2. AssistantMemory

AssistantMemory 保存跨会话仍有价值的长期语义信息，例如：

- focus areas；
- 接受/拒绝偏好；
- 稳定习惯与约束；
- 精炼的对话摘要；
- 对用户的重要长期观察；
- 用户暂时不想补充、但未来可能影响 Achievement 的历史信息提醒。

AssistantMemory 可以同步，并使用普通 Git 冲突处理方式。

## 3. 不属于 AssistantMemory

- 完整 Agent Session；
- 原始聊天记录；
- `completed_mission_log`：应直接查询 Mission；
- `last_generation`：属于本机 MissionSuggestion 生成器状态；
- API key、provider、Telegram 和其他凭证；
- UI event 和短期缓存。

## 4. `missions.json`

只有用户已经接受的 Mission 进入同步文件：

```json
{
  "missions": [
    {
      "id": "019b1234-89ab-7def-8123-456789abcdef",
      "title": "完成 Rust Book",
      "description": "阅读全部章节并完成主要练习。",
      "status": "active",
      "progress": 40,
      "difficulty": "B",
      "deadline": "2026-12-31",
      "created_at": "2026-08-15T20:30:00+08:00"
    },
    {
      "id": "019b1234-89ab-7def-8123-456789abcdf0",
      "title": "完成所有权章节",
      "status": "completed",
      "progress": 100,
      "parent_id": "019b1234-89ab-7def-8123-456789abcdef",
      "created_at": "2026-08-15T20:35:00+08:00",
      "completed_at": "2026-08-20T21:00:00+08:00"
    }
  ]
}
```

字段与校验：

- 文件缺失表示没有 Mission；文件存在时 `missions` 必填、非空并按 `id` 排序。
- 新 Mission ID 由系统生成 UUIDv7。手动编辑同步 JSON 时必须保留稳定 ID；ID 在仓库内唯一且一经引用不能原地修改。
- `title`、`status` 和 `created_at` 必填；`description` 可选。
- `status` 只能是 `active`、`completed`、`archived`。
- `progress` 可选，必须是 0～100 的整数。完成命令把已存在的 progress 设为 100；手动 JSON 中 completed Mission 若提供 progress，也必须为 100。
- `difficulty` 可选，只能是 `S`、`A`、`B`、`C`、`D`。
- `deadline` 可选，格式为有效 `YYYY-MM-DD`。
- `parent_id` 可选，必须引用同一文件中的 Mission，不得引用自身，全部 parent 关系必须无环。
- `created_at`、`completed_at` 使用带时区偏移的 RFC 3339。completed Mission 可以因旧数据时间未知而省略 `completed_at`；archived Mission 若曾完成则保留 `completed_at`，未完成即归档时省略它。active Mission 不得包含 `completed_at`。
- 不保存 `days_remaining`、`short_desc`、`linked_achievement_id`、`ai_metadata`、Dashboard 展示或更新时间。
- 未定义字段和 JSON `null` 一律拒绝。

`days_remaining` 从 deadline 与本机当前日期计算。Mission 完成可以成为 Agent 判断 Achievement 的上下文，但不会自动修改 Achievement，也不保存静态跨模块链接。

生命周期命令固定为：`complete` 将状态改为 completed、把已有 progress 设为 100 并记录完成时间；`archive` 可以归档 active 或 completed Mission，且不抹掉 progress 或已有 `completed_at`。显式删除使用 hard delete；仍被子 Mission 的 `parent_id` 引用时拒绝删除，必须先移除或修改这些引用。

当前 Mission CLI 提供：

```text
mission list [--mission-id <id>] [--status <status>] [--parent-id <id>]
mission create|update [--file <json>]
mission complete|archive|delete <mission_id>
mission suggestion-list [--suggestion-id <id>] [--status <status>]
mission suggest [--file <json>]
mission accept|reject <suggestion_id>
mission suggestion-delete <suggestion_id>
```

`create` 和 `suggest` 不接受调用方提供 ID 或时间，由系统生成 UUIDv7 和当前 RFC 3339 时间。`update` 的输入完整替换 title 及全部可编辑可选字段，省略可选字段表示清除；status、ID、created/completed time 只能由生命周期命令维护。重复 complete/archive/reject 返回 `changed: false`。Suggestion 被 accept 时由一个 Application operation 在 runtime lock 下创建同 ID 的 active Mission 并删除本机 Suggestion；显式接受已 rejected 的 Suggestion也被允许，因为它代表用户改变决定。两个 JSON 文件之间当前不承诺进程崩溃时的原子切换。

## 5. 本机 MissionSuggestion

MissionSuggestion 只存在于 runtime 的 `local-state.json`：

- 字段为 `id`、`title`、可选 `description`、`difficulty`、`deadline`、`parent_mission_id`、用户可读的 `reason`、`generated_at` 和 `status`。
- status 只允许 `pending` 或 `rejected`。
- 新 ID 使用 UUIDv7。接受时创建同 ID 的 active Mission，并删除 Suggestion；拒绝时保留本机实体用于后续去重。
- parent Mission、日期和枚举在接受时再次校验；失效 Suggestion 不能直接转成 Mission。
- Suggestion 不保存模型、prompt、token、generation batch 或完整会话元数据。
- 用户可以显式删除 rejected Suggestion；不使用固定 TTL 或 FIFO。

完整 JSON import 不覆盖本机 Suggestion。若导入的 Mission 已经占用相同 ID，删除对应 Suggestion，因为它已经在其他设备被接受。

## 6. Dashboard 本机配置

Dashboard Mission 展示使用固定 slot：`countdown`、`progress`、`hint_1`、`hint_2`。每个 slot 最多引用一个 Mission ID，可选保存本机展示 label。

- 配置不进入 Git。
- 不对 Mission 建硬外键；Mission 完成、删除或同步后缺失时保留 slot 并显示配置错误，用户可以替换或清除。
- 完整 JSON import 不覆盖本机配置。
- `days_remaining` 和进度显示始终读取当前 Mission，不复制业务值。

## 7. `assistant-memory.json`

```json
{
  "memories": [
    {
      "id": "019b2234-89ab-7def-8123-456789abcdef",
      "kind": "preference",
      "content": "用户更愿意接受一周内可以完成、结果明确的任务。",
      "created_at": "2026-08-15T21:00:00+08:00",
      "updated_at": "2026-08-15T21:00:00+08:00"
    },
    {
      "id": "019b2234-89ab-7def-8123-456789abcdf0",
      "kind": "reminder",
      "content": "用户可能已经学会不少菜，但目前不想回忆完整清单；下次更新烹饪记录时可以温和提醒。",
      "created_at": "2026-08-15T21:05:00+08:00",
      "updated_at": "2026-08-15T21:05:00+08:00"
    }
  ]
}
```

- 文件缺失表示没有 Memory；文件存在时 `memories` 必填、非空并按 `id` 排序。
- 新 ID 使用 UUIDv7；手动编辑同步 JSON 时必须保留稳定 ID。
- `kind` 必须是 `focus`、`preference`、`constraint`、`habit`、`summary`、`reminder`、`observation` 之一。
- `content` 必填且非空，只保存经过精炼、预计跨会话仍有价值的自然语言语义。
- `created_at` 和 `updated_at` 必填，使用带时区偏移的 RFC 3339，且 updated 不早于 created。
- 不保存来源会话、模型、置信度、证据对象、过期标记、软删除状态或用户权威事实的副本。
- 未定义字段和 JSON `null` 一律拒绝。

当前 AssistantMemory CLI 提供：

```text
memory list [--memory-id <id>] [--kind <kind>]
memory create|update [--file <json>]
memory delete <memory_id>
```

`create` 只接收 kind/content，由系统生成 UUIDv7，并把 created_at/updated_at 设为同一当前时间。`update` 完整替换 kind/content，保留 ID 与 created_at；实际内容没有变化时返回 `changed: false` 且不刷新 updated_at。`delete` 是 hard delete，恢复依赖 JSON/Git 历史。

`arcana-data context summary` 在 runtime lock 下组合 live JSON、SQLite Records 与本机 JSON，拼装 Agent 启动上下文：本机日期、Status selection、active Mission、显式 Achievement 状态和全部 AssistantMemory。Mission deadline 会派生 `days_remaining`；停用 Pack 的 Status selection 保留并标记 `available: false`。摘要不持久化，也不内嵌完整 Record、Pack Definition、已完成/归档 Mission 或 MissionSuggestion；需要时再用对应领域查询读取。

## 8. Memory 精炼与清理

- 新信息补充或修正同一语义时更新原 entry 并保留 ID，不重复追加近义条目。
- 多条摘要可以在一次事务中合并为一条：更新保留项并删除冗余项。
- 明确过时或错误的内容直接删除；Git 历史已经提供恢复能力，不建立软删除或 tombstone。
- 不设置固定条目数量、FIFO 或自动 TTL。清理由用户或 Agent 在有上下文时显式执行。
- 临时推测、单次情绪、完整聊天摘要和未经用户支持的判断不进入长期 Memory。
- Memory 只帮助 Agent 选择询问和建议，不能替代 Record、Mission 或 Achievement 状态等权威实体。
- Git 冲突按普通文本冲突暴露，不做语义自动合并。

## 9. 存储

- `missions.json` 保存已接受 Mission；parent 引用由领域校验器验证。
- runtime `local-state.json` 保存 MissionSuggestion 与 Dashboard slots，不进入 Git。
- `assistant-memory.json` 保存可同步的长期语义条目。
- SQLite 不保存 Mission、Suggestion、Dashboard 或 AssistantMemory。
- `days_remaining` 与 Agent context view 按查询派生，不建立缓存。
