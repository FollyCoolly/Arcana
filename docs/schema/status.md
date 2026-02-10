# Status Schema

`Status` 采用两层模型：

- 指标定义层：描述每个项目是什么、单位、目标值、相关部位等。
- 指标数值层：只存用户当前有数据的项目值。

这样可以直接支持“可插拔指标”和“项目自解释”。

## 文件路径

- `data/status_metric_definitions.json`：指标定义
- `data/status.json`：指标数值

## 缺省值约定

- 指标定义中的扩展字段可缺省（例如 `target_max`、`body_parts`）。
- 指标数值中没有的 key 视为“暂无数据”。
- 推荐省略字段，不写 `null`。

## 指标定义文件

### 结构

```json
{
  "version": 1,
  "metrics": []
}
```

### `metrics[]` 字段

- `id` (`string`, 必填)：全局唯一指标 ID
- `name` (`string`, 必填)：显示名称
- `category` (`string`, 必填)：`health` 或 `performance`
- `group` (`string`, 必填)：分组名称，如 `body`, `strength`, `endurance`
- `unit` (`string`, 必填)：单位，如 `kg`, `cm`, `bpm`, `sec`
- `value_type` (`string`, 必填)：当前建议 `number`
- `target_max` (`number`, 可选)：目标上限，仅建议用于 `performance` 指标
- `target_min` (`number`, 可选)：目标下限，仅建议用于 `performance` 指标
- `body_parts` (`string[]`, 可选)：相关部位标签
- `description` (`string`, 可选)：简短说明

约定：

- `health` 指标不设置 `target_min` / `target_max`。
- `performance` 指标可按需要设置目标区间。

### 示例

```json
{
  "version": 1,
  "metrics": [
    {
      "id": "weight_kg",
      "name": "Weight",
      "category": "health",
      "group": "body",
      "unit": "kg",
      "value_type": "number",
      "body_parts": [],
      "description": "Current body weight"
    },
    {
      "id": "lat_pulldown_5rm_kg",
      "name": "Lat Pulldown 5RM",
      "category": "performance",
      "group": "strength",
      "unit": "kg",
      "value_type": "number",
      "target_max": 90,
      "body_parts": ["lats", "biceps"],
      "description": "Best 5-rep max on lat pulldown"
    }
  ]
}
```

## 指标数值文件

### 结构

```json
{
  "version": 1,
  "metrics": {}
}
```

### `metrics` 规则

- key 必须对应 `status_metric_definitions.json` 中的 `id`
- value 类型由对应指标的 `value_type` 决定
- 不存在的 key 视为缺省数据

### 示例

```json
{
  "version": 1,
  "metrics": {
    "height_cm": 175,
    "weight_kg": 72.5,
    "body_fat_pct": 16.8,
    "resting_heart_rate_bpm": 58,
    "blood_pressure_sys": 118,
    "blood_pressure_dia": 76,
    "waist_cm": 78,
    "hip_cm": 94,
    "chest_cm": 98,
    "upper_arm_cm": 32,
    "forearm_cm": 28,
    "thigh_cm": 54,
    "calf_cm": 37,
    "neck_cm": 37,
    "squat_5rm_kg": 110,
    "bench_press_5rm_kg": 85,
    "deadlift_5rm_kg": 130,
    "pull_up_max_reps": 10,
    "push_up_max_reps": 35,
    "lat_pulldown_5rm_kg": 70,
    "seated_row_5rm_kg": 65,
    "pec_fly_5rm_kg": 55,
    "hip_adduction_5rm_kg": 60,
    "hip_abduction_5rm_kg": 58,
    "leg_press_5rm_kg": 220,
    "machine_chest_press_5rm_kg": 62,
    "machine_shoulder_press_5rm_kg": 40,
    "seated_crunch_5rm_kg": 45,
    "leg_extension_5rm_kg": 58,
    "leg_curl_5rm_kg": 52,
    "dumbbell_bench_press_8rm_kg_each": 28,
    "dumbbell_shoulder_press_8rm_kg_each": 20,
    "dumbbell_fly_12rm_kg_each": 12,
    "run_1k_time_sec": 255,
    "run_5k_pace_sec_per_km": 310
  }
}
```

## 可插拔约定

- 每个模块或内容包可提供一份指标定义文件。
- 应用启动时合并所有定义，并检查 `id` 冲突。
- 用户只需在 `status.json` 填写已启用且有数据的指标值。
