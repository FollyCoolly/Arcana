# Missions Schema

Mission 模块管理用户的长期目标和重要任务。日常追踪需求由 AI 通过 achievement progress 处理，不在此模块中显式建模。

## 设计要点

- **统一结构**：所有 mission 为同一类型，不区分 daily/long-term
- **AI 驱动进度**：进度值由 AI agent 直接写入 `progress` 字段，不依赖子任务自动计算
- **主菜单展示**：AI 决定是否在主界面展示倒计时（右上角，最多 1 个）和进度条（右下角，最多 1 个），并撰写简洁文案
- **跨模块联动**：mission 完成可更新成就进度或触发解锁，由 AI agent 判断

## 文件路径

- `data/missions.json`：任务定义、状态与主菜单展示配置（单文件）

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
| `status` | string | 是 | `"proposed"` / `"active"` / `"completed"` / `"archived"` / `"rejected"` |
| `progress` | number | 否 | 进度 0–100，由 AI 写入 |
| `deadline` | string | 否 | 截止日期，`YYYY-MM-DD` |
| `linked_achievement_id` | string | 否 | 关联的成就 ID，由 AI agent 判断是更新进度还是解锁 |
| `created_at` | string | 否 | 创建时间，ISO 8601 |
| `completed_at` | string | 否 | 完成时间，ISO 8601 |
| `ai_metadata` | object | 否 | AI Agent 元数据（预留） |

### `main_menu` — AI 控制的主菜单展示区

AI agent 决定是否展示以及文案内容。`countdown` 和 `progress` 各最多 1 个，可为 null。

| 字段 | 类型 | 说明 |
|------|------|------|
| `countdown` | object \| null | 右上角倒计时展示 |
| `countdown.mission_id` | string | 关联的 mission ID，后端从中取 deadline 计算剩余天数 |
| `countdown.label` | string | AI 撰写的简洁文案，如 "Rust精通"、"毕业答辩" |
| `progress` | object \| null | 右下角进度条展示 |
| `progress.mission_id` | string | 关联的 mission ID，后端从中取 progress 值 |
| `progress.label` | string | AI 撰写的文案，应含"进度""完成度""熟练度"等后缀，如 "Rust 熟练度"、"论文完成度" |

**设计要点：**
- AI 负责判断何时设置/清除展示项（不适合展示时设为 null）
- `label` 不是 mission 标题的复制，而是 AI 专门为主界面撰写的简洁文案
- 前端渲染：倒计时 → `距离{label}还有{days}天`；进度条 → `{label}` + 进度条

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

`rejected` 任务不在前端显示，仅供 AI 任务生成器（phan-site）参考，避免重复推荐被拒绝的任务类型。

AI 生成的 mission ID 使用 `ai_<YYYYMMDD>_<slug>` 前缀（如 `ai_20260331_rust_ch12`）。

AI 通过 `ai_metadata` 字段存储生成元数据：

```json
"ai_metadata": {
  "generation_id": "2026-03-31",
  "difficulty_tier": "easy|medium|hard",
  "generation_reason": "拆解活跃任务: learn_rust"
}
```

## 校验规则

1. `id` 唯一
2. `progress` 范围 0–100
3. `status` 只能是 `"proposed"` / `"active"` / `"completed"` / `"archived"` / `"rejected"`
4. `linked_achievement_id` 引用的成就 ID 必须存在（运行时校验）
5. `main_menu.countdown.mission_id` 和 `main_menu.progress.mission_id` 必须引用存在的 mission
