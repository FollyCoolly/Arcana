# DerivedValue 目标模型

> **状态**：Implemented in Domain/Application/CLI；UI 尚未展示
> **最后更新**：2026-08-17

## 1. 定位

Arcana 的计算链分为三层：

```text
Record -> DerivedValue -> Status Score -> Dimension Score
   \----------------------^             |
                                          -> Lv.0..Lv.5
```

- `Record` 是用户明确记录的事实，持久化在 SQLite。
- `DerivedValue` 是 Pack 定义的、具名且可复用的中间计算结果。
- `Status Score` 是 Dimension 内的 0～100 子分数，可以直接读取 Record，也可以读取 DerivedValue。
- `Dimension Score` 只对子 Score 做加权平均。

DerivedValue 是可选层，不是所有计算的必经之路。只在公式有稳定领域含义、会被多个消费者复用，或将来值得在 UI 单独展示时创建；一次性的短公式直接写在 Status Score 中。

## 2. 所有权与持久化

DerivedValueDefinition 随 Pack 保存在 `derived-values.json`，属于可读、可同步的内容定义。计算值不持久化、不写入 SQLite，也不导出到 `records/`。

每次查询以同一个 repository snapshot 和明确的 `as_of_date` 惰性计算。当前没有后台监听线程，也不会因 Record 变化主动写入缓存或触发 Achievement。

## 3. Definition

```json
{
  "values": [
    {
      "id": "health.bmi",
      "name": "BMI",
      "description": "由当前身高和体重计算",
      "expression": "record('health.weight_kg') / (record('health.height_m') * record('health.height_m'))"
    },
    {
      "id": "identity.game_days",
      "name": "游戏时间",
      "unit": "day",
      "expression": "days_since(record('identity.birth_date'))"
    }
  ]
}
```

- `id` 使用全局稳定的 `<namespace>.<name>`，namespace 不等于 Pack ID。
- `name` 必填；`description` 与 `unit` 可选。
- `expression` 的最终结果必须是有限数值，但不自动 clamp。BMI、天数、比率等可以保留自己的自然范围。
- 同一文件的 `values` 按 ID 排序且不得重复；没有 DerivedValue 时省略整个文件。

## 4. 公式与依赖

公式支持有限数字、四则运算、一元正负、括号、`min`、`max`、`abs`、`clamp`，以及：

```text
record('<record_definition_id>')
derived('<derived_value_id>')
days_since(<date value>)
```

第一版 `record()` 只允许读取 scalar `number`、`integer` 或 `date`。`derived()` 始终返回数值。`days_since()` 只接受 date，并使用调用方提供的 `as_of_date`，不在领域表达式中隐式读取系统时钟。

DerivedValue 可以依赖同一 Pack 完整声明的其他 DerivedValue，但依赖图必须是 DAG。禁止 DerivedValue 反向读取 Status Score 或 Dimension Score。

表达式最长 2048 字节，AST 最多 256 个节点、深度最多 32；任意缺失输入传播为缺失，除以零、类型错误、NaN 和无穷大显式报错。

## 5. Pack 自包含与兼容

每个 Pack 必须完整声明其公式引用的 RecordDefinition 与 DerivedValue，不能隐式继承父 Pack 的内容。不同 Pack 可以重复声明同一 DerivedValue ID，但名称、单位、公式和非空描述必须兼容；启用冲突的 Pack 会失败。

Pack schema v2 引入可选的 `derived-values.json`。旧 Pack schema v1 仍可读取并继续使用直接引用 Record 的 Status 公式，但不能包含 DerivedValue。

## 6. 运行时结果

查询结果包含 Definition、`as_of_date`、可选数值和传递后的 `missing_record_ids`。例如 BMI 依赖体重，另一个 DerivedValue 再依赖 BMI；体重缺失时最外层结果仍报告原始的体重 Record ID，而不是把 BMI 当作神秘缺失值。

```text
arcana-data derived list [--as-of YYYY-MM-DD]
arcana-data derived evaluate <id> [--as-of YYYY-MM-DD]
```

Status `evaluate` 同样接受 `--as-of`，以便引用日期派生值时获得可复现结果。

## 7. 第一版边界

- 不聚合 collection/event，不提供 count、sum、filter 或窗口函数；按实际需求扩展。
- 不保存缓存、来源追踪图或计算历史。
- 不把 DerivedValue 当作用户可写实体，也不提供 mutation 命令。
- UI 暂不新增 DerivedValue 页面；当前由 CLI、Status 和 Dashboard 消费。
