# Status Schema

> **状态**：Current JSON v1
> **目标替代**：[`docs/design/status.md`](../design/status.md)。目标模型改为 RecordData + Pack DimensionDefinition、子 Score 表达式和固定加权平均，并由本机 UI 选择五个 Dimension；本文件中的 values、target 和 bracket 结构仅用于解释当前代码。

`Status` 是一套通用的指标展示系统，用户可自定义追踪任意领域的数值指标，并通过雷达图展示多维度的综合评价。

## 核心概念

- **指标 (Metric)**：一个可量化的数值，纯数据描述。分为用户指标和系统指标。
- **维度 (Dimension)**：雷达图上的一个轴，关联若干指标并定义评分规则，拥有 Persona 5 风格的等级称号。
- **用户指标**：用户自定义，值由用户输入，存储在 `status.json`。
- **系统指标**：应用自带，值由后端实时计算（如 Gallery 统计量），不存储在 `status.json`。

## 文件路径

| 文件 | 用途 |
|------|------|
| `<data_dir>/status_metric_definitions.json` | 用户指标定义 + 维度配置 |
| `<data_dir>/status.json` | 用户指标的当前值 |

## 缺省值约定

- 可选字段省略，**不写 `null`**。
- `status.json` 中没有的 key 视为"暂无数据"。
- 维度中无数据的指标不参与聚合，全部无数据时显示 "--"。

---

## 指标定义文件

### 顶层结构

```json
{
  "version": 1,
  "metrics": [],
  "dimensions": []
}
```

### `metrics[]` — 用户指标定义

Metric 是纯数据字典，不包含评分逻辑。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | `string` | 是 | 全局唯一，`snake_case`，不得以 `sys_` 开头（保留给系统指标） |
| `name` | `string` | 是 | 显示名称 |
| `group` | `string` | 是 | UI 分组名称，自由文本（如 `body`, `strength`, `endurance`） |
| `unit` | `string` | 是 | 单位（如 `kg`, `cm`, `bpm`, `sec`, `count`, `percent`） |
| `value_type` | `string` | 是 | 值类型，当前仅 `number` |
| `description` | `string` | 否 | 简短说明 |

### `dimensions[]` — 雷达图维度

维度拥有评分逻辑和等级称号。数组顺序 = 雷达图显示顺序。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | `string` | 是 | 全局唯一，`snake_case` |
| `name` | `string` | 是 | 显示名称 |
| `level_titles` | `string[5]` | 是 | 5 个等级称号，从 Lv.1 到 Lv.5 |
| `level_thresholds` | `number[4]` | 是 | 升级阈值，严格递增。score >= 阈值时升级（详见计算规则） |
| `enabled` | `boolean` | 否 | 是否在雷达图上显示，默认 `true` |
| `metrics` | `object` | 是 | 评分配置，key 为 metric ID（用户或系统指标） |

### `dimensions[].metrics` — 维度内的指标评分

每个 entry 的 key 是 metric ID，value 是评分配置：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `weight` | `number` | 是 | 权重，> 0 |
| `target_max` | `number` | 否 | 目标上限，> 0 |
| `target_min` | `number` | 否 | 目标下限，> 0 |
| `scoring_brackets` | `array` | 否 | 分段评分（区间最优型） |

**评分方式优先级**：`target_min` + `target_max` 同时存在时为健康范围模式；仅 `target_max` 为越高越好模式；仅 `target_min` 为越低越好模式；`scoring_brackets` 为分段评分。`scoring_brackets` 与 `target_*` 互斥。全部省略时直接使用指标原始值。

### `scoring_brackets[]` 结构

| 字段 | 类型 | 说明 |
|------|------|------|
| `min` | `number` | 区间下界（含） |
| `max` | `number` | 区间上界（不含） |
| `score` | `number` | 该区间的分数，0 ~ 1 |

值不在任何区间内时，score 为 0。

---

## 维度分数计算

### 单指标 contribution

每个指标根据评分方式计算 contribution（贡献值）：

- **`target_min` + `target_max`（健康范围）**：值在 `[target_min, target_max]` 范围内时 `contribution = 1.0`；低于范围 `contribution = value / target_min`；高于范围 `contribution = target_max / value`
- **仅 `target_max`**：`contribution = min(value / target_max, 1.0)`
- **仅 `target_min`**：`contribution = min(target_min / value, 1.0)`
- **`scoring_brackets`**：查找 value 所在区间，取该区间的 `score`
- **无评分方式**：`contribution = value`（直接使用原始值）

### 维度聚合

```
filled_avg = Σ(filled contribution_i) / filled_count
completeness = filled_count / total_metric_count
estimated_missing_contribution = filled_avg × (0.7 + 0.3 × completeness)

dimension_score =
    Σ(filled contribution_i × weight_i)
    + Σ(estimated_missing_contribution × missing weight_i)
```

当前实现会用已填写指标的平均 contribution 估算缺失指标，再按完整度施加 `0.7～1.0` 的系数。全部无数据时 score 为 null，显示 "--"。返回给前端的缺失指标 contribution 仍为 null，估算值只参与 Dimension 总分。

> [!WARNING]
> 这是当前代码行为，不是目标规则。目标 Status 不估算缺失数据，而是将缺失子 Score 同时从分子和分母排除。

### 等级映射

`level_thresholds` 是一个 4 元素数组 `[t1, t2, t3, t4]`，严格递增：

| 条件 | 等级 | 显示 |
|------|------|------|
| score < t1（只要 Dimension 至少有一个已填写指标） | Lv.1 | `level_titles[0]` |
| t1 ≤ score < t2 | Lv.2 | `level_titles[1]` |
| t2 ≤ score < t3 | Lv.3 | `level_titles[2]` |
| t3 ≤ score < t4 | Lv.4 | `level_titles[3]` |
| score ≥ t4 | Lv.5 | `level_titles[4]` |

阈值由维度自行定义，不同维度的 score 量纲可以完全不同。

当前实现没有 Lv.0 的 Dimension 输出：只要存在任一已填写指标，就至少得到 Lv.1；全部缺失时 level 为 null。目标模型会把 score 为 null 或 0 映射为 Lv.0。

---

## 系统指标

以 `sys_` 为前缀，由后端注册和计算，用户不可定义，但可在维度中引用。

### Gallery 统计

| ID | 说明 |
|----|------|
| `sys_anime_watched` | 看过的动画数 |
| `sys_movies_watched` | 看过的电影数 |
| `sys_books_read` | 读过的书数 |
| `sys_games_played` | 玩过的游戏数 |

### Skill 统计

| ID | 说明 |
|----|------|
| `sys_skills_lv1` | level ≥ 1 的 skill 数量 |
| `sys_skills_lv2` | level ≥ 2 的 skill 数量 |
| `sys_skills_lv3` | level ≥ 3 的 skill 数量 |
| `sys_skills_lv4` | level ≥ 4 的 skill 数量 |
| `sys_skills_lv5` | level ≥ 5 的 skill 数量 |

### 其他

| ID | 说明 |
|----|------|
| `sys_game_days` | 出生至今天数 |

系统指标的值不存储在 `status.json`，由后端加载时实时计算。后续可按需扩展。

---

## 指标数值文件

### 结构

```json
{
  "version": 1,
  "metrics": {}
}
```

### 规则

- key 必须对应 `status_metric_definitions.json` 中用户指标的 `id`
- value 类型由对应指标的 `value_type` 决定
- 不存在的 key 视为暂无数据
- 系统指标不存储在此文件中

---

## 完整示例

### 指标定义

```json
{
  "version": 1,
  "metrics": [
    {
      "id": "weight_kg",
      "name": "Weight",
      "group": "body",
      "unit": "kg",
      "value_type": "number"
    },
    {
      "id": "bench_press_5rm_kg",
      "name": "Bench Press 5RM",
      "group": "strength",
      "unit": "kg",
      "value_type": "number"
    },
    {
      "id": "pec_fly_5rm_kg",
      "name": "Pec Fly 5RM",
      "group": "strength",
      "unit": "kg",
      "value_type": "number"
    },
    {
      "id": "run_5k_pace_sec_per_km",
      "name": "Run 5K Pace",
      "group": "endurance",
      "unit": "sec_per_km",
      "value_type": "number"
    },
    {
      "id": "bmi",
      "name": "BMI",
      "group": "body",
      "unit": "",
      "value_type": "number",
      "description": "Body Mass Index (derived from height and weight)"
    }
  ],
  "dimensions": [
    {
      "id": "strength",
      "name": "Strength",
      "level_titles": ["手无缚鸡", "初具力量", "小有所成", "力大无穷", "人形兵器"],
      "level_thresholds": [0.36, 0.72, 1.08, 1.44],
      "metrics": {
        "bench_press_5rm_kg": { "weight": 1.0, "target_max": 95 },
        "pec_fly_5rm_kg":     { "weight": 0.8, "target_max": 65 }
      }
    },
    {
      "id": "endurance",
      "name": "Endurance",
      "level_titles": ["气喘吁吁", "能跑能跳", "持久作战", "铁打的肺", "永动机"],
      "level_thresholds": [0.2, 0.4, 0.6, 0.8],
      "metrics": {
        "run_5k_pace_sec_per_km": { "weight": 1.0, "target_min": 280 }
      }
    },
    {
      "id": "health",
      "name": "Health",
      "level_titles": ["亚健康", "马马虎虎", "身体不错", "健康达人", "生命巅峰"],
      "level_thresholds": [0.2, 0.4, 0.6, 0.8],
      "metrics": {
        "bmi": {
          "weight": 1.0,
          "target_min": 18.5,
          "target_max": 24.9
        }
      }
    },
    {
      "id": "culture",
      "name": "Culture",
      "level_titles": ["孤陋寡闻", "略有涉猎", "鉴赏有道", "博闻强识", "行走百科"],
      "level_thresholds": [0.6, 1.2, 1.8, 2.4],
      "metrics": {
        "sys_anime_watched": { "weight": 1.0, "target_max": 200 },
        "sys_movies_watched": { "weight": 1.0, "target_max": 100 },
        "sys_books_read":     { "weight": 1.0, "target_max": 50 }
      }
    },
    {
      "id": "mastery",
      "name": "Mastery",
      "level_titles": ["初出茅庐", "学有所成", "融会贯通", "出类拔萃", "一代宗师"],
      "level_thresholds": [3, 15, 40, 80],
      "metrics": {
        "sys_skills_lv1": { "weight": 1 },
        "sys_skills_lv2": { "weight": 2 },
        "sys_skills_lv3": { "weight": 4 },
        "sys_skills_lv4": { "weight": 8 },
        "sys_skills_lv5": { "weight": 16 }
      }
    }
  ]
}
```

### 指标数值

```json
{
  "version": 1,
  "metrics": {
    "weight_kg": 72.5,
    "bench_press_5rm_kg": 85,
    "pec_fly_5rm_kg": 55,
    "run_5k_pace_sec_per_km": 310,
    "bmi": 22.3
  }
}
```

---

## 当前加载与校验行为

- 两个文件首先按 Rust 类型反序列化；必填字段缺失或字段类型错误会导致加载失败。
- `status_metric_definitions.json` 中重复的 metric ID 会导致加载失败。
- `status.json` 的 key 若不在 metric definitions 中会导致加载失败；value 必须为数字。
- 只有 `value_type: "number"` 的 metric 会返回给 UI；其他值目前被过滤，不会作为配置错误报告。
- `dimensions` 可以为空，`enabled` 缺省为 `true`。
- Dimension 只有在 `level_titles` 长度为 5、`level_thresholds` 长度为 4 且至少一个 metric 有值时才产生分数和等级；否则返回未评分。
- 未找到的 metric 引用按缺失数据处理，不会导致加载失败。

当前实现没有完整校验 Dimension ID 唯一性、threshold 递增、正权重、target 合法范围、bracket 合法范围或多种评分配置互斥。目标架构会替换这套评分模型，因此不再为旧结构补充新的模板或校验规则。
