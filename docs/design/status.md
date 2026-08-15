# Status 目标模型

> **状态**：Target / 计算语义已确定，表达式白名单尚未定稿
> **最后更新**：2026-08-15

## 1. 定位

Status 不拥有独立事实或用户配置实体。Pack 直接定义 Dimension；UI 从已启用 Pack 提供的 Dimension 中选择五个，从 RecordData 计算子 Score、Dimension 总分和等级。

```text
RecordData + selected Pack Dimension
    -> child Score expression
    -> Dimension weighted average
    -> Lv.0..Lv.5
```

Status 分数和等级始终可重新计算，不写入同步数据。

## 2. 固定两层结构

第一版不建立递归评分树，也不允许 Dimension 自定义最终聚合表达式。

- 一个 Pack 可以直接包含若干 DimensionDefinition。
- 一个 Dimension 包含若干子 Score。
- 子 Score 通过安全表达式读取一个或多个 RecordData。
- Dimension 只对子 Score 做加权平均。
- 本机 UI 配置保存五个互不重复的 `selected_dimension_ids`，不创建额外的 Status 领域实体。

概念结构：

```text
Pack
└── DimensionDefinition[]
    ├── id / name
    ├── level_titles[5]
    ├── level_thresholds[4]
    └── scores[]
        ├── id / name
        ├── weight
        └── expression
```

## 3. 子 Score

每个子 Score：

- `id` 在所属 Dimension 中稳定且唯一；
- `name` 用于 UI 和诊断；
- `weight` 必须大于 0，不要求所有权重之和为 1；
- `expression` 是只读 RecordData 的纯表达式；
- 表达式可以引用多个 RecordData；
- 有效数值结果默认 clamp 到 `[0, 100]`。

表达式原始结果可用于诊断，但不持久化。语法错误、类型错误、NaN 和无穷大必须显示为配置错误，不能伪装成 0、null 或正常分数。

## 4. Dimension 聚合

只对有值的子 Score 聚合：

```text
dimension_score =
    Σ(child_score × child_weight)
    / Σ(available child_weight)
```

- 缺少必要 RecordData 时，子 Score 为 `null`。
- `null` 不进入分子或分母，不按 0 计算，也不从其他指标估算。
- 全部子 Score 为 `null` 时，Dimension 分数为 `null`。
- Dimension 结果理论上已在 `[0, 100]`；实现仍应防御非法浮点数。

## 5. 等级

Status 维持 `Lv.0～Lv.5`：

| 条件 | 等级 |
| --- | --- |
| score 为 `null` 或 `0` | Lv.0（未解锁） |
| `0 < score < t2` | Lv.1 |
| `t2 <= score < t3` | Lv.2 |
| `t3 <= score < t4` | Lv.3 |
| `t4 <= score < t5` | Lv.4 |
| `score >= t5` | Lv.5 |

每个 Dimension 保存 4 个严格递增且大于 0 的 threshold，分别表示进入 Lv.2、Lv.3、Lv.4、Lv.5 的最低分。它还保存 5 个 Lv.1～Lv.5 的显示标题；Lv.0 使用统一的未解锁表现。

## 6. 安全表达式

表达式是确定、无副作用的计算，不是任意 JavaScript、Rust 或 shell eval。

第一版遵循“按实际需要扩展”：

- 只实现当前迁移数据所需的基础算术和白名单函数；
- 可以读取 RecordData；
- 禁止赋值、循环、文件、网络、时间、随机数、反射和动态代码加载；
- 表达式在保存/导入时解析和校验；
- 执行时限制 AST 深度和计算量；
- 缺失值传播为 `null`，除非表达式显式使用忽略缺失的函数。

Scalar、Collection、Event 的具体读取函数在实现表达式引擎前根据首批 Status 配置定稿，不预先建立通用查询 DSL。

## 7. 概念示例

以下示例不是最终 JSON Schema：

```json
{
  "id": "default",
  "name": "Default Status",
  "dimensions": [
    {
      "id": "physical",
      "name": "身体状态",
      "level_titles": ["觉醒", "成长", "熟练", "卓越", "巅峰"],
      "level_thresholds": [25, 50, 75, 90],
      "scores": [
        {
          "id": "strength",
          "name": "力量",
          "weight": 1,
          "expression": "record('fitness.bench_press_5rm_kg') / 95 * 100"
        },
        {
          "id": "endurance",
          "name": "耐力",
          "weight": 0.8,
          "expression": "280 / record('fitness.run_5k_pace_sec_per_km') * 100"
        }
      ]
    }
  ]
}
```

## 8. Pack Dimension 与 UI 选择

DimensionDefinition 直接属于 Pack，不是需要采用或复制的模板：

1. Pack 启用时，系统校验 Dimension 引用的 RecordSet 和表达式。
2. 已启用 Pack 中的 Dimension 进入可选列表。
3. 用户在本机 UI 中选择五个 Dimension；选择只保存稳定 ID，不复制定义。
4. 用户需要自定义名称、子 Score、权重、表达式或阈值时，直接编辑自己维护的 Pack，或新建个人 Pack。
5. Pack 被关闭或缺失后，不删除其中的定义或 RecordData；若它仍被 UI 选择，则显示配置错误并要求用户替换，不能静默选择其他 Dimension。

Dimension ID 必须全局稳定，建议采用 `<pack_id>::<dimension_id>`。五个 UI 选择属于本机显示配置，不进入 Git 同步；DimensionDefinition 作为 Pack 内容正常同步。

## 9. 被替代的旧模型

目标模型删除：

- 独立 `status.json` 事实副本；
- `target_min` / `target_max`；
- `scoring_brackets`；
- 缺失指标估算分；
- 原始不同单位数值直接相加；
- 因指标数量变化而改变尺度的加权总和；
- 递归 Score 树和 Dimension 最终表达式。
