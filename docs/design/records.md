# RecordDefinition 与 Record

> **状态**：Target / Record 同步 Schema 与 SQLite 物理结构已确定
> **最后更新**：2026-08-15

## 1. 定位

Record 是 Arcana 全局、相对扁平、用户所有的事实层。Status 和 Achievement 可以读取同一份事实，但不能各自保存重复副本。

```text
RecordDefinition --defines--> Record
Record --read by--> selected Pack Dimension
Record --read by--> Agent evaluating Achievement
```

- Record 不属于 Pack。
- 关闭或删除 Pack 不删除 Record。
- 一个 RecordDefinition 可以关联多个 Achievement；一个 Achievement 也可以关联多个 RecordDefinition。
- Arcana Skill 不直接读取 Record，只读取状态为 `achieved` 的 Achievement。

## 2. RecordDefinition 与 Record

`RecordDefinition` 描述稳定 ID、数据形态、字段、单位和校验约束；`Record` 是用户明确记录的事实。

RecordDefinition 由 Pack 直接发放。启用 Pack 时，系统把所有已启用 Pack 中的定义按 ID 合并成临时的运行时注册表；该注册表是派生结果，不单独持久化或同步。

Record 是用户拥有的全局事实，不属于任何 Pack。即使提供其定义的 Pack 被停用，Record 也会保留；只是当没有任何已启用 Pack 提供对应定义时，系统暂时无法解释或使用它。

一个 RecordDefinition 最多对应一个 Record。Record 按 kind 保存完整用户数据，而不是把 collection item 或 event 再抽象成独立领域实体：

```text
RecordDefinition 1 ── 0..1 Record
```

Record 必须保存 `definition_id`。它既是对 RecordDefinition 的显式引用，也是 Record 自身的唯一标识；由于二者最多一对一，Record 不再增加独立 `id`。

RecordDefinition 第一版只包含：

- 必填 `id`、`name` 和 `kind`；
- 可选 `description`；
- scalar 使用必填 `value_type` 和可选 `unit`；
- collection/event 使用 `fields` 描述业务字段的类型、必填性和可选单位。

第一版不包含独立版本、所有者 Pack、来源、`update_mode`、UI 配置或计算规则，也不实现任意 JSON Schema。

### 2.1 标准身份 Record

目标模型不建立 Profile 或 UserSettings。普通 `basic` Pack 提供以下标准 RecordDefinition：

- `identity.nickname`：`scalar` + `string`；
- `identity.birth_date`：`scalar` + `date`。

它们使用与其他事实相同的 Record、SQLite Repository 和 Git JSON 格式。应用只在消费层约定这两个 ID：昵称缺失时显示产品默认值，生日缺失时不计算游戏天数。停用 `basic` Pack 不删除已有 Record。

创建新用户仓库时应用写入并默认启用标准 `basic` Pack；它仍是仓库中的普通、可编辑 Pack，而不是应用数据库里的隐藏 Definition。新系统不从旧版 Profile JSON 生成这两个 Record；用户在新仓库中按需重新填写。

## 3. 第一版数据形态

第一版只区分结构差异：

| kind | 含义 | 示例 |
| --- | --- | --- |
| `scalar` | 一个当前值 | 体重、身高、累计总数、当前偏好值 |
| `collection` | 有稳定身份、需要去重和增删的对象集合 | 学会的菜、完成的项目、读过的论文 |
| `event` | 独立发生且发生时间有意义的事件 | 跑步、训练、演出、考试 |

这不是 gauge/counter 或“当前值/累计值”的业务分类。Scalar 可以通过不同命令更新，但数据定义不固定 `update_mode`。

## 4. 写入命令与持久化语义

以下是命令语义，不是 RecordDefinition 类型：

- `set`：写入当前已知值；
- `increment`：基于当前 scalar 增加；
- `correct`：修正错误事实；
- `create_empty_collection` / `create_empty_event`：明确记录空集合或空事件列表；
- `add_item` / `correct_item` / `remove_item`：维护 collection；
- `append_event` / `correct_event` / `delete_event`：维护 event。

所有命令通过同一事务 API 执行。`increment`、collection item 和 event 的读改写必须全部发生在数据库事务内，不能由调用方先读后写。向尚不存在的 collection/event 添加第一项时同时创建对应 Record header；显式创建空 Record 不得覆盖已有 Record。重复 item/event ID 必须返回冲突，只有 `correct_item` / `correct_event` 可以替换既有内容。删除最后一项后仍保留 Record header，以区分“明确为空”和“从未记录”。

## 5. 标识与时间

- RecordDefinition ID 固定为 `<namespace>.<name>`，例如 `health.body_weight`、`programming.projects`。
- `namespace` 和 `name` 都使用小写 snake_case，分别匹配 `[a-z][a-z0-9_]*`；第一版 ID 恰好包含一个 `.`。
- `namespace` 是 Record 的稳定分组，不等于 `pack_id`，也不随 PackForest 父子关系改变。一个 Pack 可以携带和使用多个 namespace 的完整定义。
- ID 在整个用户仓库内必须全局唯一；一旦被 Record、Achievement 或 Dimension 引用就不能直接改名。第一版通过创建新 ID、显式更新引用和确认删除旧数据完成演进。
- Collection/Event 条目优先使用外部稳定 ID 或领域自然 ID；没有合适 ID 时使用 UUIDv7。
- scalar 可选 `effective_at` 表示当前值实际生效的时间。
- event 必填 `occurred_at` 表示事件发生时间。
- `recorded_at` 表示信息进入 Arcana 的时间。
- `recorded_at` 由正常写入命令填写；scalar、collection item 和 event 都保存该字段。
- 正常 `set` / `correct` / `increment` 请求不接受调用方伪造 `recorded_at`；Application Command 在事务执行时填写当前时间。同步 import 按已验证 snapshot 保留原时间。
- scalar `increment` 可以显式提供新的 `effective_at`；省略时沿用当前 scalar 的 `effective_at`，只更新值和 `recorded_at`。
- 数值单位在 RecordDefinition 中保存为可读字符串；第一版不增加独立单位编码系统。

## 6. 缺失与历史

- 缺失、unavailable、invalid 和真实数值 0 必须区分。
- Arcana 只保存用户明确提供或通过授权来源导入的数据。
- 不建立通用 baseline、历史估算量或虚构 collection/event 条目。
- Collection 数量默认表示“Arcana 已明确记录的条目数”，不承诺覆盖用户完整人生经历。
- 用户可以暂不补充历史数据；相关提醒属于 AssistantMemory，不参与分数。
- 用户后来提供具体历史事实时，使用真实生效/发生时间写入，同时保留本次 `recorded_at`。

## 7. Pack 声明与兼容

每个 Pack 必须携带它运行所需的完整 RecordDefinition，不能只写 ID，也不能依赖父 Pack 隐式提供定义。

启用 Pack 时，系统构建运行时 Definition Registry：

1. 第一次遇到某个 ID：注册该完整定义。
2. 后续遇到相同 ID 且结构兼容：合并为同一个运行时定义，共享同一份用户 Record。
3. 后续遇到相同 ID 但结构不兼容：显示结构差异，并阻止相关 Pack 同时启用。
4. Pack 只提供 ID、未提供完整定义：Pack 校验失败。

Definition Registry 只由当前启用的 Pack 派生，不是新的持久化实体，也不进入 Git 同步数据。

两个 Pack 对同一 ID 的声明可以合并，当且仅当：

- `kind` 相同；
- scalar 的 `value_type` 和 `unit` 完全相同；
- collection/event 中同名字段的 `type`、`required` 和 `unit` 完全相同；
- 两侧独有的字段全部是可选字段，合并后取字段并集；
- `name` 相同；`description` 相同，或者只有一侧提供，此时采用非空描述。

字段顺序不参与比较。任何 required 字段差异、类型变化、单位变化、非空描述冲突或 kind 变化都会阻止相关 Pack 同时启用，不做隐式单位转换。

Pack 自身更新时，增加可选字段兼容；增加必填字段、删除字段、改变字段类型或 required 状态、改变 scalar 类型、kind 或单位都属于破坏性变化，必须创建新的 Definition ID。第一版不提供内容迁移 DSL。仅修改名称和描述不会改变用户 Record，但重复声明同一 ID 的其他已启用 Pack 仍必须满足上述合并规则。

## 8. 对象结构

以下结构已经确定。

### 8.1 基础类型与字段定义

第一版只支持以下叶子类型：

| type | JSON 表示 | 约束 |
| --- | --- | --- |
| `string` | string | 任意 Unicode 字符串 |
| `number` | number | 有限数值 |
| `integer` | number | 无小数部分的整数 |
| `boolean` | boolean | `true` / `false` |
| `date` | string | `YYYY-MM-DD` |
| `datetime` | string | 带时区偏移的 RFC 3339 时间 |

第一版不支持嵌套对象、业务数组、枚举或任意 JSON。持续时长、距离和计数使用 `number`/`integer` 加 `unit` 表示。

Collection/Event 的 `fields` 是以字段名为 key 的对象。每个字段定义必须包含 `type` 和 `required`；只有 `number`/`integer` 可以包含 `unit`。业务字段名使用小写 snake_case，并且不能使用系统保留名：

```text
id, definition_id, value, effective_at, recorded_at,
items, events, occurred_at
```

可选值通过省略字段表示，JSON `null` 不代表缺失且第一版一律拒绝。

### 8.2 scalar

```json
{
  "id": "health.body_weight",
  "name": "体重",
  "kind": "scalar",
  "value_type": "number",
  "unit": "kg"
}
```

```json
{
  "definition_id": "health.body_weight",
  "value": 72.5,
  "effective_at": "2026-08-15",
  "recorded_at": "2026-08-15T20:30:00+08:00"
}
```

`effective_at` 可选；真实数值 0 必须保留，不能用缺少 Record 表示。

### 8.3 collection

```json
{
  "id": "cooking.learned_dishes",
  "name": "学会的菜",
  "description": "用户已经能够独立制作的菜品",
  "kind": "collection",
  "fields": {
    "learned_at": {
      "type": "date",
      "required": false
    },
    "name": {
      "type": "string",
      "required": true
    }
  }
}
```

```json
{
  "definition_id": "cooking.learned_dishes",
  "items": [
    {
      "id": "dish:tomato-and-eggs",
      "learned_at": "2026-08-15",
      "name": "番茄炒蛋",
      "recorded_at": "2026-08-15T20:30:00+08:00"
    }
  ]
}
```

Collection item 的 `id` 和 `recorded_at` 是系统字段；其他字段由 RecordDefinition 定义。缺少 Record 表示尚无可靠数据，存在且 `items` 为空表示明确记录为空集合。

### 8.4 event

```json
{
  "id": "fitness.running",
  "name": "跑步",
  "kind": "event",
  "fields": {
    "distance_km": {
      "type": "number",
      "required": true,
      "unit": "km"
    },
    "duration_minutes": {
      "type": "number",
      "required": false,
      "unit": "min"
    }
  }
}
```

```json
{
  "definition_id": "fitness.running",
  "events": [
    {
      "id": "0198b6d4-61a0-7ad5-8d5d-a9347abf0152",
      "occurred_at": "2026-08-15T07:30:00+08:00",
      "distance_km": 5.2,
      "duration_minutes": 31,
      "recorded_at": "2026-08-15T08:10:00+08:00"
    }
  ]
}
```

Event 的 `id`、`occurred_at` 和 `recorded_at` 是系统字段；其他字段由 RecordDefinition 定义。存在且 `events` 为空表示 Arcana 中明确记录为空事件列表，不代表用户过去从未发生过该类事件。

## 9. Git 同步文件 Schema

RecordDefinition 随 Pack 存放；用户 Record 与 Pack 分离，并按稳定 namespace 聚合：

```text
packs/
└── cooking/
    └── record-definitions.json

records/
├── cooking.json
└── fitness.json
```

- 每个 Pack 的 `record-definitions.json` 包含该 Pack 运行所需的完整定义，可以混合多个 namespace。
- 不建立根级、持久化的全局 RecordDefinition 文件或注册表。
- `records/<namespace>.json` 保存该 namespace 下的多份用户 Record；namespace 只用于稳定分组，不表示 Pack 所有权，也不跟随 PackForest 父子关系变化。
- 每份 Record 显式保存 `definition_id`，以引用运行时注册表中的定义；Record 不再另设 ID。
- `records/<namespace>.json` 中只能出现 `definition_id` 以同一 `<namespace>.` 开头的 Record；路径和文件顶层字段中的 namespace 必须与 ID 前缀一致。
- 一份 Record 内完整保存 scalar、collection 或 event 数据；集合条目和事件不拆成独立文件。
- Definition 可以存在而没有对应 Record；这表示用户尚未记录数据。删除 Record 只删除用户事实，不影响 Pack 中的 Definition。
- 第一版不进一步拆分大型 namespace 文件；只有实际数据量证明需要时，才按年份等稳定规则分片。

### 9.1 `packs/<pack_id>/record-definitions.json`

文件顶层只有 `definitions`：

```json
{
  "definitions": [
    {
      "id": "fitness.running",
      "name": "跑步",
      "kind": "event",
      "fields": {
        "distance_km": {
          "type": "number",
          "required": true,
          "unit": "km"
        }
      }
    },
    {
      "id": "identity.nickname",
      "name": "昵称",
      "kind": "scalar",
      "value_type": "string"
    }
  ]
}
```

- 不重复保存 `schema_version`；整个 Pack 文件组由 `manifest.json` 中的 Pack Schema 版本约束。
- `definitions` 必填、非空并按 `id` 字典序排序；同一文件内 ID 不得重复。Pack 不声明任何 RecordDefinition 时省略整个文件。
- scalar 必须包含 `value_type`，不得包含 `fields`；collection/event 必须包含 `fields`，不得包含 `value_type` 或 Definition 级 `unit`。
- 除已定义字段外不接受未知字段，避免手动编辑拼写错误被静默忽略。

### 9.2 `records/<namespace>.json`

文件顶层只有 `namespace` 和 `records`：

```json
{
  "namespace": "cooking",
  "records": [
    {
      "definition_id": "cooking.dishes_cooked_total",
      "value": 32,
      "recorded_at": "2026-08-15T10:30:00+08:00"
    },
    {
      "definition_id": "cooking.learned_dishes",
      "items": [
        {
          "id": "dish:tomato-and-eggs",
          "name": "番茄炒蛋",
          "recorded_at": "2026-08-15T10:35:00+08:00"
        }
      ]
    }
  ]
}
```

- `namespace` 必须等于文件名，并符合 namespace 命名规则。
- `records` 必填且非空，按 `definition_id` 字典序排序；同一 `definition_id` 只能出现一次。namespace 没有任何 Record 时不导出对应文件。
- 每个 Record 必须且只能包含一种 payload：scalar 使用 `value`，collection 使用 `items`，event 使用 `events`。
- scalar 的 `recorded_at` 必填，`effective_at` 可选；value 必须匹配 Definition 的 `value_type`。
- collection item 的 `id`、`recorded_at` 必填，按 `id` 排序；业务字段必须匹配 Definition。
- event 的 `id`、`occurred_at`、`recorded_at` 必填，按 `occurred_at`、`id` 排序；业务字段必须匹配 Definition。
- Definition 可用时拒绝未知业务字段。Definition 不可用时只校验公共结构并无损保留 payload，将 Record 标记为 unresolved，且不允许通过正常命令更新。

### 9.3 确定性输出

- 固定字段按文档示例的语义顺序输出，动态 `fields` 和业务字段按 key 字典序输出。
- 日期和时间按基础类型约束输出；已经记录的时区偏移在无业务修改的重复导出中保持不变。
- 不输出 `null`、SQLite row id、缓存、派生值、条目数量、校验和或导出时间。
- 手动调整数组或对象 key 顺序不改变语义；下一次成功导出会恢复规范顺序。
