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
      "tags": ["coding", "milestone"],
      "prerequisites": []
    },
    {
      "id": "programmer::first_pr_merged",
      "name": "First Pull Request",
      "description": "Have a pull request merged into an open-source project.",
      "difficulty": "intermediate",
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
  "achievements": {}
}
```

### `achievements` 字段

平坦 map，key 为成就 ID，value 为进度/达成详情对象。**只记录 tracked 和 achieved 的成就，locked 成就不在此文件中。**

**三态语义：**
- **不在 map 中** = locked（未开始）
- `status: "tracked"` = AI 正在追踪进度，**UI 展示等同 achieved**
- `status: "achieved"` = 正式达成

| 字段 | 类型 | 必填 | 适用状态 | 说明 |
|------|------|------|----------|------|
| `status` | enum | 是 | 两者 | `"tracked"` 或 `"achieved"` |
| `achieved_at` | string | 否 | achieved | 达成日期，支持 `YYYY`、`YYYY-MM`、`YYYY-MM-DD` 三种精度 |
| `tracked_at` | string | 否 | tracked | 开始追踪日期，`YYYY-MM-DD` |
| `note` | string | 否 | 两者 | 用户/AI 备注 |
| `progress_detail` | string[] | 否 | tracked | AI 管理的进度条目列表（非量化，自由格式） |
| `may_be_incomplete` | boolean | 否 | tracked | 标记用户可能有未提供的历史进度，AI 不应假设列表是完整的 |

**设计要点：**
- **并非所有成就都需要 tracked 状态**。二元成就（如"5km跑进25分钟"）由 AI 根据最新信息直接判断，locked → achieved，不经过 tracked。需要积累型进度的成就（如"学会100道菜"）才使用 tracked。AI agent 自行判断。
- `tracked` 状态仅供 AI agent 参考，UI 不区分 tracked 和 achieved
- `tracked` 和 `achieved` 都计入技能积分
- `progress_detail` 无固定格式，由 AI agent 自由管理（可以是菜名列表、里程碑描述等）
- `may_be_incomplete` 为 true 时，AI 应知道用户可能有未记录的历史进度，不能仅凭列表长度判断是否达标
- 新达成的成就，UI 应自动填入当天日期（`YYYY-MM-DD`）作为 `achieved_at` 缺省值
- 卸载包时数据保留，重新加载时恢复

### 示例

```json
{
  "version": 1,
  "achievements": {
    "programmer::hello_world": {
      "status": "achieved",
      "note": "First Python script - fizzbuzz.py"
    },
    "fitness::5k_under_25min": {
      "status": "achieved",
      "achieved_at": "2023",
      "note": "公园晨跑"
    },
    "cooking::learn_100_dishes": {
      "status": "tracked",
      "tracked_at": "2026-03-01",
      "progress_detail": ["盐焗鸡", "红烧肉", "番茄炒蛋"],
      "may_be_incomplete": true,
      "note": "用户表示之前已会做很多菜，未提供完整列表"
    }
  }
}
```

## 校验规则

1. 成就 ID 必须以 `<manifest.id>::` 开头
2. `prerequisites` 只能引用同包内的有效成就 ID
3. `prerequisites` 关系必须构成 DAG（无环）——加载时通过 DFS 环检测强制执行
4. 同包内成就 ID 不可重复
5. `difficulty` 必须是枚举值之一（由 serde 反序列化时强制校验）
6. `status` 必须是 `"tracked"` 或 `"achieved"` 之一
7. `achieved_at` 若存在，格式必须为 `YYYY`、`YYYY-MM` 或 `YYYY-MM-DD` 之一
