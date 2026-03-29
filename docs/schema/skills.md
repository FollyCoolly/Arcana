# Skills Schema

技能树定义存在于每个内容包中，描述技能的等级计算规则和节点。积分和关键成就标记在技能树节点和等级门槛上。

## 文件路径

- `data/packs/<pack_id>/skills.json`：技能树定义（每包）

## 核心设计决策

1. **积分放在技能侧**：技能树节点包含 `points`，成就定义不含积分。看技能树就能知道每个节点值多少分。
2. **无边（edges）**：技能树只定义节点，不定义边。前端渲染时根据 `achievements.prerequisites` 推导连线。
3. **无位置（position）**：节点不存储布局坐标，前端根据成就的 `difficulty`、`tags` 等属性动态计算布局。

## `skills.json`（每包）

### 结构

```json
{
  "version": 1,
  "skills": []
}
```

### `skills[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | `<pack_id>::<name>` 格式，全局唯一 |
| `name` | string | 是 | 显示名称 |
| `description` | string | 否 | 技能描述 |
| `max_level` | number | 是 | 最大等级 |
| `level_thresholds` | array | 是 | 等级门槛，共 `max_level` 条 |
| `nodes` | array | 是 | 技能树节点列表 |

### `nodes[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `node_id` | string | 是 | 树内唯一节点 ID |
| `achievement_id` | string | 是 | 对应成就 ID |
| `points` | number | 是 | 解锁后贡献的积分 |

同一成就可在多棵技能树中被引用（不同节点、不同积分值）。

### `level_thresholds[]` 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `level` | number | 是 | 等级编号（1-indexed） |
| `points_required` | number | 是 | 累计所需积分 |
| `required_key_achievements` | string[] | 否 | 该等级**新增**的关键成就 ID（增量式，低等级的自动继承；省略视为 `[]`） |

### 示例

```json
{
  "version": 1,
  "skills": [
    {
      "id": "programmer::programming_general",
      "name": "Programming",
      "description": "General software development proficiency.",
      "max_level": 5,
      "level_thresholds": [
        { "level": 1, "points_required": 5 },
        { "level": 2, "points_required": 15 },
        {
          "level": 3,
          "points_required": 30,
          "required_key_achievements": ["programmer::shipped_side_project"]
        }
      ],
      "nodes": [
        {
          "node_id": "node_hello_world",
          "achievement_id": "programmer::hello_world",
          "points": 5
        },
        {
          "node_id": "node_first_pr",
          "achievement_id": "programmer::first_pr_merged",
          "points": 10
        }
      ]
    }
  ]
}
```

## 等级计算算法

要达到某等级需**同时满足**：
- 积分 >= `points_required`
- 该等级及所有更低等级的 `required_key_achievements` 中列出的每个成就都已解锁

`required_key_achievements` 是**增量式**的：每个等级只列出该等级**新增**的关键成就，算法自动从低等级向上累积。

```
calculate_skill_level(skill, unlocked_achievement_ids):
    total_points = 0

    for node in skill.nodes:
        if node.achievement_id in unlocked_achievement_ids:
            total_points += node.points

    current_level = 0
    accumulated_keys = []
    for threshold in skill.level_thresholds (ascending by level):
        accumulated_keys += threshold.required_key_achievements ?? []
        all_keys_unlocked = all(id in unlocked_achievement_ids for id in accumulated_keys)

        if total_points >= threshold.points_required AND all_keys_unlocked:
            current_level = threshold.level
        else:
            break

    return current_level
```

### 计算示例

以 `programmer::programming_general` 为例，假设用户已解锁 `hello_world`、`first_pr_merged`、`shipped_side_project`：

- `node_hello_world` 贡献 5 分，`node_first_pr` 贡献 10 分，`node_shipped` 贡献 20 分 → total_points = 35
- Level 1：35 >= 5，accumulated_keys = [] → 通过
- Level 2：35 >= 15，accumulated_keys = [] → 通过
- Level 3：35 >= 30，accumulated_keys = [`shipped_side_project`]，已解锁 → 通过
- Level 4：35 >= 50 → 不满足 → break
- 结果：Level 3

## 连线推导算法（前端渲染）

技能树不存储边（edges）。前端根据节点的 `achievement_id` 在 `achievements.json` 中查找 `prerequisites`，推导连线：

```
derive_edges(skill, achievements_map):
    edges = []
    for node in skill.nodes:
        achievement = achievements_map[node.achievement_id]
        for prereq_id in achievement.prerequisites:
            prereq_node = skill.nodes.find(n => n.achievement_id == prereq_id)
            if prereq_node exists:
                edges.push({ from: prereq_node.node_id, to: node.node_id })
    return edges
```

同一成就在不同技能树中出现时，仅在该树中引用了前置成就节点时才会产生连线。

## 校验规则

1. 技能 ID 必须以 `<manifest.id>::` 开头
2. `nodes[].achievement_id` 必须引用同包内有效成就
3. `level_thresholds` 的 `points_required` 必须单调递增
4. `level_thresholds` 条目数必须等于 `max_level`
5. `required_key_achievements` 引用的成就 ID 必须存在
6. 同一技能内 `node_id` 不可重复
