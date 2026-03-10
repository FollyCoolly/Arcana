# Achievements Schema

成就定义存在于每个内容包中，描述用户可解锁的里程碑。成就定义**不包含**积分信息——积分由技能树节点定义。

## 文件路径

- `data/packs/<pack_id>/achievements.json`：成就定义（每包）
- `data/achievement_progress.json`：用户成就解锁状态（全局）

## `achievements.json`（每包）

### 结构

```json
{
  "version": 1,
  "achievements": []
}
```

### `achievements[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | `<pack_id>::<name>` 格式，全局唯一 |
| `name` | string | 是 | 显示名称 |
| `description` | string | 是 | 成就描述 |
| `difficulty` | enum | 是 | 难度等级 |
| `category` | string | 是 | 包内分组名称 |
| `tags` | string[] | 否 | 标签，用于筛选 |
| `prerequisites` | string[] | 否 | 前置成就 ID 列表（同包内），AND 逻辑 |

### `difficulty` 枚举值

- `beginner`
- `intermediate`
- `advanced`
- `expert`
- `legendary`

### 前置成就（prerequisites）

- 引用同包内的其他成就 ID
- 多个前置条件之间为 AND 关系（全部满足才可解锁）
- 前置关系必须构成有向无环图（DAG）
- 省略或空数组表示无前置条件

### 示例

```json
{
  "version": 1,
  "achievements": [
    {
      "id": "programmer::hello_world",
      "name": "Hello, World!",
      "description": "Write your first program in any language.",
      "difficulty": "beginner",
      "category": "fundamentals",
      "tags": ["coding", "milestone"],
      "prerequisites": []
    },
    {
      "id": "programmer::first_pr_merged",
      "name": "First Pull Request",
      "description": "Have a pull request merged into an open-source project.",
      "difficulty": "intermediate",
      "category": "open_source",
      "tags": ["git", "collaboration"],
      "prerequisites": ["programmer::hello_world"]
    }
  ]
}
```

## `achievement_progress.json`（全局）

### 结构

```json
{
  "version": 1,
  "unlocked": {}
}
```

### `unlocked` 字段

平坦 map，key 为成就 ID，value 为解锁详情对象：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `achieved_at` | string | 否 | 实际达成日期，支持 `YYYY`、`YYYY-MM`、`YYYY-MM-DD` 三种精度 |
| `note` | string | 否 | 用户备注 |

- **在 map 中 = 已解锁，不在 = 未解锁**
- `achieved_at` 省略表示已达成但不记得具体时间
- 新达成的成就，UI 应自动填入当天日期（`YYYY-MM-DD`）作为缺省值，用户可清除或修改
- 卸载包时数据保留，重新加载时恢复

### 示例

```json
{
  "version": 1,
  "unlocked": {
    "programmer::hello_world": {
      "note": "First Python script - fizzbuzz.py"
    },
    "fitness::5k_under_25min": {
      "achieved_at": "2023",
      "note": "公园晨跑"
    },
    "programmer::first_pr_merged": {
      "achieved_at": "2025-09-22"
    }
  }
}
```

## 校验规则

1. 成就 ID 必须以 `<manifest.id>::` 开头
2. `prerequisites` 只能引用同包内的有效成就 ID
3. `prerequisites` 关系必须构成 DAG（无环）
4. 同包内成就 ID 不可重复
5. `difficulty` 必须是枚举值之一
6. `achieved_at` 若存在，格式必须为 `YYYY`、`YYYY-MM` 或 `YYYY-MM-DD` 之一
