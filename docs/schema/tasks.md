# Tasks Schema

Task 模块管理每日任务和长期任务。支持两种粒度：短期（daily）重复性任务和长期（long-term）里程碑式目标。重要长期任务可在主界面右下角显示进度条。

## 设计要点

- **双粒度**：daily 任务按天重置/循环，long-term 任务跨天持续追踪
- **主界面进度条**：标记为 `pinned` 的长期任务在主界面右下角显示，类似 P5 的"信赖度"进度条
- **AI 联动预留**：任务的创建、推荐、完成验证将来由 AI Agent 驱动，数据结构预留 `ai_metadata` 字段
- **跨模块联动**：任务完成可触发成就解锁、技能加分等，通过 `linked_achievement_id` 等字段关联

## 文件路径

- `data/tasks.json`：任务定义与状态（单文件）

## `tasks.json`

### 顶层结构

```json
{
  "version": 1,
  "daily_tasks": [],
  "long_term_tasks": []
}
```

### `daily_tasks[]` 字段

每日任务，每天可标记完成，次日自动重置。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 唯一标识 |
| `title` | string | 是 | 任务名称 |
| `description` | string | 否 | 详细描述 |
| `category` | string | 否 | 分类标签（如 "health", "study", "work"） |
| `repeat_days` | string[] | 否 | 指定星期几执行，缺省为每天。值: `"mon"` `"tue"` `"wed"` `"thu"` `"fri"` `"sat"` `"sun"` |
| `completion_log` | object | 否 | 按日期记录完成状态，key 为 `YYYY-MM-DD`，value 为 `boolean` |
| `streak` | number | 否 | 当前连续完成天数，后端计算 |
| `created_at` | string | 否 | 创建时间，ISO 8601 |
| `active` | boolean | 否 | 是否启用，缺省 `true` |
| `ai_metadata` | object | 否 | AI Agent 使用的元数据（预留） |

### `long_term_tasks[]` 字段

长期目标/里程碑任务，支持进度追踪。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 唯一标识 |
| `title` | string | 是 | 任务名称 |
| `description` | string | 否 | 详细描述 |
| `category` | string | 否 | 分类标签 |
| `status` | string | 是 | `"active"` / `"completed"` / `"archived"` |
| `progress` | number | 否 | 进度百分比 0–100，缺省 0 |
| `milestones` | array | 否 | 子里程碑列表 |
| `pinned` | boolean | 否 | 是否钉在主界面显示进度条，缺省 `false` |
| `deadline` | string | 否 | 截止日期，`YYYY-MM-DD` |
| `linked_achievement_id` | string | 否 | 完成后关联解锁的成就 ID |
| `created_at` | string | 否 | 创建时间，ISO 8601 |
| `completed_at` | string | 否 | 完成时间，ISO 8601 |
| `ai_metadata` | object | 否 | AI Agent 使用的元数据（预留） |

### `milestones[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 里程碑 ID（任务内唯一） |
| `title` | string | 是 | 里程碑名称 |
| `completed` | boolean | 是 | 是否已完成 |
| `completed_at` | string | 否 | 完成时间，ISO 8601 |

## 示例

```json
{
  "version": 1,
  "daily_tasks": [
    {
      "id": "daily_exercise",
      "title": "30 分钟运动",
      "description": "跑步、力量训练、或其他运动",
      "category": "health",
      "repeat_days": ["mon", "tue", "wed", "thu", "fri"],
      "completion_log": {
        "2026-03-22": true,
        "2026-03-23": true
      },
      "streak": 2,
      "created_at": "2026-03-01T00:00:00+08:00",
      "active": true
    }
  ],
  "long_term_tasks": [
    {
      "id": "learn_rust",
      "title": "系统学习 Rust",
      "description": "完成 Rust Book + 做 3 个项目",
      "category": "study",
      "status": "active",
      "progress": 40,
      "milestones": [
        { "id": "m1", "title": "读完 Rust Book", "completed": true, "completed_at": "2026-02-15T00:00:00+08:00" },
        { "id": "m2", "title": "完成第一个 CLI 项目", "completed": false },
        { "id": "m3", "title": "完成第二个 Web 项目", "completed": false }
      ],
      "pinned": true,
      "deadline": "2026-06-30",
      "linked_achievement_id": "programmer::rust_proficient",
      "created_at": "2026-01-15T00:00:00+08:00"
    }
  ]
}
```

## 计算字段（后端返回时附加）

| 字段 | 公式 | 说明 |
|------|------|------|
| `today_completed` | daily: `completion_log[today] == true` | 今日是否已完成 |
| `streak` | 从今天往前连续完成的天数（仅计 `repeat_days` 匹配的日子） | 连续天数 |
| `milestone_progress` | `completed_milestones / total_milestones * 100` | 里程碑进度，覆盖手动 `progress`（如果有 milestones） |
| `days_remaining` | `deadline - today` | 剩余天数，无 deadline 则 null |

## 主界面进度条

`pinned: true` 的 long-term task 在主界面右下角显示：
- 显示 `title` 和 `progress` 百分比
- 进度条样式参照 P5 的对怪盗团信任度条
- 最多同时显示 3 个 pinned 任务

## 校验规则

1. `id` 在各自数组内唯一
2. `progress` 范围 0–100
3. `status` 只能是 `"active"` / `"completed"` / `"archived"`
4. `repeat_days` 值只能是 `"mon"` ~ `"sun"`
5. `milestones[].id` 在同一任务内唯一
6. `linked_achievement_id` 引用的成就 ID 必须存在（运行时校验）
7. `completion_log` 的 key 必须为合法 `YYYY-MM-DD` 格式
