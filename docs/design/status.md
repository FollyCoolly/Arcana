# Status 目标模型

> **状态**：Implemented in Domain/Application/CLI/Tauri Status UI
> **最后更新**：2026-08-18

## 1. 定位

Status 不拥有独立事实或需要同步的用户状态。Pack 直接定义 Dimension；UI 从已启用 Pack 提供的 Dimension 中选择五个，并只在本机保存这项显示配置，再从 Record 计算子 Score、Dimension 总分和等级。

Dimension 是显式选择加入的长期评价视角，不是 Pack 的必备内容。大多数 Pack，尤其是狭窄的子领域 Pack，不定义 Dimension；不能为了让 Pack 显得完整而生成 Status 配置。

```text
Record [-> DerivedValue] + selected Pack Dimension
    -> Status Score expression
    -> Dimension weighted average
    -> Lv.0..Lv.5
```

Status 分数和等级始终可重新计算，不写入同步数据。

## 2. 固定两层结构

第一版不建立递归评分树，也不允许 Dimension 自定义最终聚合表达式。

- 一个 Pack 可以直接包含若干 DimensionDefinition。
- 一个 Dimension 包含若干子 Score。
- 子 Score 通过安全表达式直接读取数值 Record，或读取可复用的 DerivedValue。
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
- `expression` 是只读 Record/DerivedValue 的纯表达式；
- 表达式可以引用多个 Record 和 DerivedValue；
- 有效数值结果默认 clamp 到 `[0, 100]`。

表达式原始结果可用于诊断，但不持久化。语法错误、类型错误、NaN 和无穷大必须显示为配置错误，不能伪装成 0、null 或正常分数。

## 4. Dimension 聚合

只对有值的子 Score 聚合：

```text
dimension_score =
    Σ(child_score × child_weight)
    / Σ(available child_weight)
```

- 缺少必要 Record 时，子 Score 为 `null`。
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

表达式是确定、无副作用的数值计算，不是任意 JavaScript、Rust、shell 或通用脚本引擎 eval。第一版使用专用的小型 parser 和 AST evaluator。

### 6.1 Record 与 DerivedValue 读取

数据读取函数是：

```text
record('<definition_id>')
derived('<derived_value_id>')
```

- 参数必须是静态字符串字面量，不能动态拼接。
- 引用的 RecordDefinition 必须由当前 Pack 完整声明，并且是 `scalar` + `number`/`integer`。
- 引用的 DerivedValue 必须由当前 Pack 完整声明；其依赖与计算规则见 [derived_values.md](./derived_values.md)。
- Definition 有效但用户 Record 缺失时返回 `null`。
- Definition 不存在、kind/type 不匹配或 Record invalid/unresolved 时报告配置或数据错误，不伪装成 0。
- 第一版不读取 string、boolean、date、datetime、collection 或 event，也不提供 count/sum/filter 查询。

表达式引用的 RecordDefinition ID 直接从 AST 提取，不再保存一份容易漂移的引用列表。

### 6.2 语法与函数白名单

第一版只允许：

- 有限十进制数值字面量；
- `+`、`-`、`*`、`/`；
- 一元 `+`、`-`；
- 括号；
- `record(id)`；
- `derived(id)`；
- `min(a, b, ...)`、`max(a, b, ...)`；
- `abs(x)`；
- `clamp(x, min, max)`。

不允许变量、赋值、比较、条件、布尔运算、循环、用户函数、属性访问、数组、文件、网络、当前时间、随机数、反射或动态代码加载。

现有评分模式可以直接表达：

- 越高越好：`record('strength.bench_press_5rm_kg') / 95 * 100`
- 越低越好：`280 / record('cardio.run_5k_pace_sec_per_km') * 100`
- 18.5～24.9 为最佳范围：`min(derived('health.bmi') / 18.5, 1, 24.9 / derived('health.bmi')) * 100`

表达式不做自动单位换算或量纲推导。不同单位只有在作者显式写出换算公式时才能组合；这与旧模型隐式相加原始指标不同。

### 6.3 `null`、错误与资源限制

- 任一运算或函数参数为 `null` 时，结果传播为 `null`；第一版不提供 `coalesce` 或忽略缺失值的函数。
- 除以 0、NaN、无穷大、语法错误和函数参数数量错误都是显式 evaluation error。
- 有效数值结果最后统一 clamp 到 `[0, 100]`；中间 `clamp` 函数只用于作者明确需要的局部限制。
- 表达式最长 2048 字节，AST 最多 256 个节点、深度最多 32；超过限制的 Pack 无效。
- 表达式在 Pack 导入、启用和修改时解析并校验，运行时只执行已经验证的 AST。

## 7. Pack 同步 Schema

DimensionDefinition 直接属于 Pack，保存在：

```text
packs/<pack_id>/dimensions.json
```

完整结构：

```json
{
  "dimensions": [
    {
      "id": "fitness::physical",
      "name": "身体状态",
      "level_titles": ["觉醒", "成长", "熟练", "卓越", "巅峰"],
      "level_thresholds": [25, 50, 75, 90],
      "scores": [
        {
          "id": "endurance",
          "name": "耐力",
          "weight": 0.8,
          "expression": "derived('cardio.endurance_index')"
        },
        {
          "id": "strength",
          "name": "力量",
          "weight": 1,
          "expression": "record('strength.bench_press_5rm_kg') / 95 * 100"
        }
      ]
    }
  ]
}
```

校验规则：

- 不重复保存 `schema_version`；由 Pack manifest 的 `schema_version` 解释。
- `dimensions` 必填、非空并按 `id` 排序；Pack 没有 Dimension 时省略整个文件。
- Dimension ID 必须是 `<manifest.id>::<local_id>`，其中 local ID 使用小写 snake_case；整个仓库内不得重复。
- `name` 必填且非空。
- `level_titles` 必须恰好包含 5 个非空字符串，对应 Lv.1～Lv.5。
- `level_thresholds` 必须恰好包含 4 个有限数值，并满足 `0 < t2 < t3 < t4 < t5 <= 100`。
- `scores` 必填、非空并按局部 `id` 排序；同一 Dimension 内 Score ID 不得重复。
- Score ID 使用小写 snake_case；`name` 和 `expression` 必填且非空；`weight` 必须是有限且大于 0 的数值。
- 每个表达式引用的完整 RecordDefinition 都必须出现在同一 Pack 的 `record-definitions.json` 中。一个 Pack 因此可以同时携带多个 namespace 的 Definition。
- 每个表达式引用的 DerivedValue 都必须出现在同一 Pack 的 `derived-values.json` 中；父 Pack 不提供隐式继承。
- 未定义字段和 JSON `null` 一律拒绝。

## 8. 本机五项选择

UI 选择只保存在 runtime 的 `local-state.json`：

- `position` 范围为 0～4，表示雷达图顺序；
- `dimension_id` 在五个位置中唯一；
- 配置过程允许暂时少于五项，但完整 Status 页面要求恰好五项；
- 选择不进入 Git，不属于用户同步数据；完整 JSON import 不覆盖它；
- 不对 Pack Dimension 建外键。Pack 被关闭、删除或缺失后保留原选择，并显示具体配置错误，不能静默补位。

正常选择命令只能选择当前已启用且有效的 Dimension。用户需要自定义 Dimension 时，编辑自己维护的 Pack 或新建个人 Pack。

## 9. 存储与计算结果

- DimensionDefinition 来自 live repository 的 `packs/<pack_id>/dimensions.json`。
- `local-state.json` 只保存本机五项顺序。
- 表达式读取的 Records 来自 SQLite。
- 子 Score、原始表达式结果、clamp 后结果、Dimension 分数和等级都即时计算，不持久化。
- Pack 启用或导入时解析表达式、校验 RecordDefinition/DerivedValue 引用与派生 DAG；读取 Status 时在一致性 Record 快照上惰性计算。

当前 CLI 提供：

```text
status list-dimensions
status evaluate [dimension_id] [--as-of YYYY-MM-DD]
status select <position> <dimension_id>
status select <position> --clear
```

`list-dimensions` 同时返回当前本机 selection 的 available 状态；`evaluate` 返回原始子值、clamp 后子分数、缺失 Record、总分、等级和等级标题。读取与计算在 runtime lock 下组合 live Definitions、SQLite Records 与本机选择，所有派生结果只返回、不入库。

## 10. 被替代的旧模型

目标模型删除：

- 独立 `status.json` 事实副本；
- `target_min` / `target_max`；
- `scoring_brackets`；
- 缺失指标估算分；
- 原始不同单位数值直接相加；
- 因指标数量变化而改变尺度的加权总和；
- 递归 Score 树和 Dimension 最终表达式。
