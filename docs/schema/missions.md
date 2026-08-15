# Missions Schema

> **状态**：Current JSON v1
> **目标替代**：[`docs/design/missions_memory.md`](../design/missions_memory.md)。目标 `missions.json` 统一保存 active/completed/archived Mission；未接受或已拒绝的 MissionSuggestion 和主菜单展示槽只保存在本机 SQLite。目标 Mission 移除 `short_desc`、成就关联和 AI 元数据等非核心字段。

Mission 模块管理用户的长期目标和重要任务。日常追踪需求由 AI 通过 achievement progress 处理，不在此模块中显式建模。

## 设计要点

- **统一结构**：所有 mission 为同一类型，不区分 daily/long-term
- **AI 驱动进度**：进度值由 AI agent 直接写入 `progress` 字段，不依赖子任务自动计算
- **主菜单展示**：AI 决定是否在主界面展示倒计时（右上角，最多 1 个）和进度条（右下角，最多 1 个），并撰写简洁文案
- **跨模块联动**：mission 完成可更新成就进度或触发解锁，由 AI agent 判断

## 文件路径

- `<data_dir>/missions.json`：当前任务板，仅保存 `proposed` / `active` mission，以及主菜单展示配置
- `<data_dir>/mission_archive.json`：历史档案，仅保存 `completed` / `archived` / `rejected` mission，用于历史回顾和 AI 去重

## `missions.json`

### 顶层结构

```json
{
  "version": 1,
  "missions": [],
  "main_menu": {
    "countdown": null,
    "progress": null
  }
}
```

### `missions[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 唯一标识 |
| `title` | string | 是 | 任务名称 |
| `description` | string | 否 | 详细描述 |
| `status` | string | 是 | `missions.json` 中为 `"proposed"` / `"active"`；`mission_archive.json` 中为 `"completed"` / `"archived"` / `"rejected"` |
| `progress` | number | 否 | 进度 0–100，由 AI 写入 |
| `difficulty` | string | 否 | 难度等级，枚举 `"S"` / `"A"` / `"B"` / `"C"` / `"D"`（S 最难） |
| `deadline` | string | 否 | 截止日期，`YYYY-MM-DD` |
| `short_desc` | string | 否 | 5–15 字简洁描述，供主菜单任务提示板直接渲染 |
| `linked_achievement_id` | string | 否 | 关联的成就 ID，由 AI agent 判断是更新进度还是解锁 |
| `created_at` | string | 否 | 创建时间，ISO 8601 |
| `completed_at` | string | 否 | 完成时间，ISO 8601 |
| `parent_id` | string | 否 | 父任务 ID，用于表示子任务关系（仅关系标记，不影响进度计算） |
| `ai_metadata` | object | 否 | AI Agent 元数据（预留） |

### `main_menu` — AI 控制的主菜单展示区

AI agent 决定是否展示以及文案内容。`countdown` 和 `progress` 各最多 1 个，可为 null。

| 字段 | 类型 | 说明 |
|------|------|------|
| `countdown` | object \| null | 右上角倒计时展示 |
| `countdown.mission_id` | string | 关联的 mission ID，后端从中取 deadline 计算剩余天数 |
| `countdown.label` | string | AI 撰写的简洁文案，**恰好 2 字或 4 字**（决定背景板版型：2wc / 4wc），如 "发布"、"正式发布" |
| `hints` | array | 普通任务提示，最多 2 条（纯 AI 控制，不自动 fallback） |
| `hints[].mission_id` | string | 关联的 mission ID，后端校验其为 active 状态，`short_desc` 从 mission 本身读取 |
| `progress` | object \| null | 右下角进度条展示 |
| `progress.mission_id` | string | 关联的 mission ID，后端从中取 progress 值 |
| `progress.label` | string | AI 撰写的文案，应含"进度""完成度""熟练度"等后缀，如 "Rust 熟练度"、"论文完成度" |

**设计要点：**
- AI 负责判断何时设置/清除展示项（不适合展示时设为 null / 空数组）
- countdown `label` 是 mission 标题的精炼，不是复制
- hints 渲染 mission 自身的 `short_desc`（无则 fallback 到 title），fat 板（第 1 条）比 slim 板（第 2 条）更大
- 前端渲染：倒计时 → `距离{label}还有{days}天`；进度条 → `{label}` + 进度条

## `mission_archive.json`

### 顶层结构

```json
{
  "version": 1,
  "missions": []
}
```

`mission_archive.json` 与 `missions.json` 使用同一套 `missions[]` 字段结构，但不包含 `main_menu`。任务进入终态时移动到 archive：

- `completed`
- `archived`
- `rejected`

任务被重新激活或重新提案时，可以从 archive 移回 `missions.json`。

## 计算字段（后端返回时附加）

| 字段 | 公式 | 说明 |
|------|------|------|
| `days_remaining` | `deadline - today` | 剩余天数，无 deadline 则 null |

## 示例

```json
{
  "version": 1,
  "missions": [
    {
      "id": "learn_rust",
      "title": "系统学习 Rust",
      "description": "完成 Rust Book + 做 3 个项目",
      "status": "active",
      "progress": 40,
      "deadline": "2026-06-30",
      "linked_achievement_id": "programmer::rust_proficient",
      "created_at": "2026-01-15T00:00:00+08:00"
    }
  ],
  "main_menu": {
    "countdown": {
      "mission_id": "learn_rust",
      "label": "Rust精通"
    },
    "progress": {
      "mission_id": "learn_rust",
      "label": "Rust 熟练度"
    }
  }
}
```

## `proposed` 状态与 AI 任务生成

AI agent skill（`phan-site`）会生成 `status: "proposed"` 的任务建议。用户接受后变为 `active`，拒绝则变为 `rejected`。

`rejected` 任务不在前端显示，并存入 `mission_archive.json`，仅供 AI 任务生成器（phan-site）参考，避免重复推荐被拒绝的任务类型。

AI 生成的 mission ID 使用 `ai_<YYYYMMDD>_<slug>` 前缀（如 `ai_20260331_rust_ch12`）。

AI 通过 `ai_metadata` 字段存储生成元数据：

```json
{
  "ai_metadata": {
    "generation_id": "2026-03-31",
    "generation_reason": "拆解活跃任务: learn_rust"
  }
}
```

> 历史注：早期 AI 把难度埋在 `ai_metadata.difficulty_tier`，现已提升为顶级字段 `difficulty`。新数据不要再写 `ai_metadata.difficulty_tier`。

## 校验规则

1. `id` 唯一
2. `progress` 范围 0–100
3. `status` 只能是 `"proposed"` / `"active"` / `"completed"` / `"archived"` / `"rejected"`
4. `missions.json` 只能包含 `"proposed"` / `"active"`；`mission_archive.json` 只能包含 `"completed"` / `"archived"` / `"rejected"`
5. `difficulty` 若提供，只能是 `"S"` / `"A"` / `"B"` / `"C"` / `"D"`
6. `linked_achievement_id` 引用的成就 ID 必须存在（运行时校验）
7. `main_menu.countdown.mission_id` 和 `main_menu.progress.mission_id` 必须引用 `missions.json` 中存在的当前 mission
