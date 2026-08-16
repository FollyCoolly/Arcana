# Achievement、Skill 与 Content Pack

> **状态**：Achievement Domain/Application/CLI 已实现；Skill 派生查询与 Tauri UI 尚未切换
> **最后更新**：2026-08-16

## 1. 依赖方向

```text
Record + AchievementDefinition + Agent judgment
                                      |
User explicit completion -------------+--> status: achieved --> Arcana Skill

status: tracked ---------------------------------------------> UI / Agent focus
```

每个 Achievement 最多保存一条最小用户状态：

- 不存在记录：尚未跟踪且尚未完成；
- `tracked`：用户正在关注或补充这个成就；
- `achieved`：用户已经完成这个成就。

`tracked` 与 `achieved` 互斥。只有 `achieved` 参与积分；`tracked` 不等于部分完成，也不参与积分。不存在记录时，界面再根据 prerequisites 显示 `available` 或 `locked`。

## 2. AchievementDefinition

Achievement 定义至少表达：

- 稳定 `id`；
- 显示名称；
- 自然语言完成要求；
- 难度、标签和可选 prerequisites；
- 可选 `related_record_definition_ids`：已经存在、可能用于判断或跟踪的 RecordDefinition；
- 可选 `tip`：给 Agent 的自然语言辅助说明。除了补充特殊或容易误解的完成要求，也可以说明值得关注、询问或收集哪些信息，尤其适用于尚未预先定义对应 Record 的情况。

`related_record_definition_ids` 用于 Record 更新后的候选路由，不表示 Record 是完成成就的唯一证据，也不要求每个 Achievement 都预先定义 Record。`tip` 中描述的是跟踪建议和信息类型，不保存某个用户的实际事实或进度；用户数据仍不能写入共享的 AchievementDefinition。

第一版不定义机器可执行的 `criterion`、`completion`、`auto_unlock_rule` 或 Achievement 查询 DSL。Agent 读取完成要求、相关事实和可选 tip 后作出判断；依据不足时询问用户。

## 3. Progress 与用户状态

### 3.1 Progress

- 不持久化统一数值进度或自由格式 `progress_detail`。
- 需要展示或讨论时，由 Agent 根据现有 Record 即时总结。
- 不要求所有成就都有可量化进度。
- 不允许为了凑进度而反向生成虚构 Record。

### 3.2 `tracked`

`tracked` 表示用户主动关注、收集或补充某个 Achievement 所需事实，可用于：

- UI 的重点/进行中列表；
- Agent 选择后续询问内容；
- 没有相关 Record 时，根据 `tip` 识别本次对话中值得询问或用于判断的信息；
- 提醒用户可能存在未补充的历史信息。

`tracked` 不产生积分。

### 3.3 `achieved`

当 Agent 根据现有信息判断已经完成，或者用户明确表示自己已经完成时，将状态设为 `achieved`。两种方式没有不同的后续行为，因此不持久化来源类型。

用户自我确认不要求先补齐全部历史明细。状态记录只包含：

- 必填的 `status`；
- `achieved` 状态下可选的 `achieved_at`，支持 `YYYY`、`YYYY-MM` 或 `YYYY-MM-DD`。

不保存独立完成记录 ID、来源、授予时间、说明或证据。相关事实属于 Record，需要长期保留的语义信息属于 AssistantMemory。

Record 后来变化不会自动撤销 `achieved`。用户可以显式撤销；撤销时删除用户状态，若仍希望继续关注则再显式设为 `tracked`。撤销操作本身不触发重新判断，也不需要永久 suppress 状态。

prerequisites 可以影响 UI 展示和 Agent 推荐，但不能阻止用户确认一个已经完成的成就。

## 4. Agent 工作流

Arcana 不运行持续扫描全部成就的后台线程。典型流程为：

1. 用户向外部 Agent 描述经历，或 Agent 在授权下读取日记/周记。
2. Agent 提取明确事实并通过 typed command 写入 Record。
3. 根据本次 RecordDefinition 找出相关 Achievement。
4. Agent 读取要求、tip 和所需事实。
5. 满足时将用户成就状态设为 `achieved`；不确定时询问。

用户也可以随时直接声明完成，系统据此将状态设为 `achieved`。

## 5. Arcana Skill

- Skill 定义属于 Pack。
- Skill 节点引用同 Pack Achievement，并定义积分。
- 只有状态为 `achieved` 的 Achievement 节点计分。
- `tracked`、即时 Progress 和 Record 不直接计分。
- Skill 积分、节点状态和等级全部派生，不同步。
- 子 Pack 拥有独立 Skill；子 Pack 的已完成成就不自动向父 Pack Skill 注入积分。

第一版固定为 Lv.0～Lv.5，并保存四个进入 Lv.2～Lv.5 的积分 threshold。目标模型移除 `max_level`、`node_id` 和 `required_key_achievements`：节点由同一 Skill 内唯一的 `achievement_id` 标识，升级只由 achieved 节点积分决定。关键路径由 Achievement prerequisites 和积分设计表达，不再叠加第二套升级门槛。

当前 Achievement CLI 提供：

```text
achievement list [filters]
achievement state-set [--file <json>]
achievement state-revoke <achievement_id>
```

`list` 的 `availability` 由 Definition、prerequisites 和最小用户状态即时推导；它不是第三种持久化状态。`state-set` 不以 prerequisites 作为完成门槛，`state-revoke` 不要求 Definition 当前可用。

## 6. Pack 内容

Pack 是兴趣领域与成长定义的组织/分发单元，可以包含：

- manifest；
- 完整的 RecordDefinition 声明；
- AchievementDefinition；
- Arcana Skill 定义；
- 可选 DimensionDefinition。

Pack 只有 enabled/disabled 状态，不区分“官方”。manifest 保留 `author`。

Pack manifest 必须包含整数 `schema_version`，用于解释该 Pack 的全部内容文件；各内容文件不重复保存 Schema 版本。第一版不保存独立内容版本，内容历史由 Git commit 表达。

Pack 不拥有用户 Record 或用户成就状态。Pack 直接拥有其 RecordDefinition、DimensionDefinition、AchievementDefinition、SkillDefinition 和 asset；关闭 Pack 不删除 Pack 内容或用户 Record。

## 7. PackForest

每个 Pack 最多保存一个可选 `parent_pack_id`，形成多个根节点组成的 forest：

```text
programmer
├── computer_network
└── machine_learning
    └── llm
```

父子关系只用于导航、领域展示和 Agent 推荐：

- Pack 目录在物理上保持平铺；
- 子 Pack 可以独立启用；
- 启用子 Pack 不启用父 Pack；
- 关闭父 Pack 不级联关闭子 Pack；
- 父 Pack 缺失时子 Pack 仍可运行，但报告层级引用警告；
- 修改 `parent_pack_id` 不改变 Record、用户成就状态或 Skill 分数；
- 第一版不支持多父级 DAG 或任意 Pack 运行依赖。

第一版不支持安装或运行依赖，也不预留 `requires` 字段。若真实社区分发需求出现，再作为新的 Pack Schema 版本设计，不能复用 `parent_pack_id`。

## 8. RecordDefinition 声明与兼容

Pack 必须携带自己运行所需的完整 RecordDefinition。启用 Pack 时，系统把当前启用 Pack 的声明按 ID 合并成派生的运行时 Definition Registry：

- 第一次出现的 ID 注册为运行时定义；
- 相同 ID 且结构兼容的声明合并，共享同一份用户 Record；
- 相同 ID 但结构不兼容时，显示差异并阻止相关 Pack 同时启用；
- 只写 ID、没有完整定义的 Pack 无效。

系统不会把 Pack 定义复制成一份持久化的全局定义。父 Pack 也不向子 Pack 隐式提供 RecordDefinition；子 Pack 必须独立携带所需定义。因此 `parent_pack_id` 不承担 RecordDefinition 继承语义。

停用 Pack 会移除它对运行时注册表的贡献。如果没有其他已启用 Pack 提供某个定义，对应用户 Record 仍被保留，但在定义再次可用前不能被解释或更新。

## 9. Status Dimension

Pack 可以直接定义 Dimension，不经过模板采用或复制。已启用 Pack 的 Dimension 进入本机 UI 可选列表，用户从中选择五个展示。选择只引用稳定 Dimension ID；自定义 Dimension 应直接放在用户维护的 Pack 中。

## 10. 内容演进

第一版以用户仓库中的 Pack 文件为权威，由 Git 提供历史；不实现社区 Pack 自动更新、三方合并或版本依赖解析。

稳定 ID 规则：

- 被用户成就状态、Skill 或 Status 引用的 ID 不直接修改；
- 仅修改名称、描述和 tip 不需要更换 ID；
- 根本改变 Achievement 语义时创建新 ID；
- RecordDefinition 的破坏性修改必须创建新 ID。第一版不设计内容迁移 DSL；用户通过 typed command 写入新 Record、更新引用，并在确认后删除旧数据。

Pack manifest 的 `schema_version` 只表示文件格式，不表示内容发布版本。个人仓库中的内容演进直接由 Git 记录；将来若建设社区分发，再增加独立的来源和发布元数据。

## 11. Pack 目录与 manifest

Pack 目录保持平铺：

```text
packs/<pack_id>/
├── manifest.json
├── record-definitions.json   # 可选
├── dimensions.json           # 可选
├── achievements.json         # 可选
├── skills.json               # 可选
└── assets/                   # 可选
```

`manifest.json` 是唯一必需文件：

```json
{
  "schema_version": 1,
  "id": "cooking",
  "name": "烹饪",
  "description": "记录并拓展烹饪能力。",
  "author": "Alice",
  "parent_pack_id": "life_skills",
  "tags": ["creative", "life"]
}
```

规则如下：

- `schema_version` 必须是当前支持的正整数版本；高于应用支持版本时拒绝导入。
- `id` 必须等于目录名并匹配 `[a-z][a-z0-9_]*`；整个仓库内唯一。
- `name` 必填且非空。
- `description`、`author`、`parent_pack_id` 和 `tags` 可选；空值通过省略表达，不写 `null`。
- `tags` 使用小写 snake_case、去重并按字典序排序。
- `parent_pack_id` 使用 Pack ID 格式，不得等于自身。已存在的父子关系必须无环；父 Pack 缺失只产生警告。
- manifest 不保存 enabled、official、安装路径、远端地址、凭证、内容版本或更新时间。
- 未定义字段一律拒绝。

`arcana.json.enabled_pack_ids` 是唯一启用状态来源。所有 Pack 无论是否启用都必须通过自身 Schema 和内部引用校验；只有已启用 Pack 参与 RecordDefinition 兼容合并和运行时内容查询。

## 12. `achievements.json`

```json
{
  "achievements": [
    {
      "id": "cooking::first_signature_dish",
      "name": "第一道拿手菜",
      "description": "能够不依赖菜谱独立完成一道自己认可的拿手菜。",
      "difficulty": "beginner",
      "tags": ["cooking", "milestone"],
      "related_record_definition_ids": ["cooking.learned_dishes"]
    },
    {
      "id": "cooking::host_a_dinner",
      "name": "宴请朋友",
      "description": "独立规划并完成一次至少三道菜的朋友聚餐。",
      "difficulty": "intermediate",
      "prerequisites": ["cooking::first_signature_dish"],
      "tip": "如果用户只说举办了聚餐，应确认是否由用户负责规划和主要烹饪。"
    }
  ]
}
```

- 文件存在时 `achievements` 必填、非空并按 `id` 排序；没有 Achievement 时省略整个文件。
- ID 必须是 `<manifest.id>::<local_id>`，local ID 使用小写 snake_case。
- `name`、`description` 和 `difficulty` 必填且非空；`description` 就是自然语言完成要求，不另建 criterion 字段。
- `difficulty` 只能是 `beginner`、`intermediate`、`advanced`、`expert`、`legendary`。
- `tags`、`prerequisites` 和 `related_record_definition_ids` 可选；存在时必须去重并排序。
- prerequisites 只能引用同 Pack Achievement，必须构成 DAG。它影响可用性、推荐和 UI，不阻止用户确认已经完成。
- related RecordDefinition 必须由同一 Pack 的 `record-definitions.json` 完整声明。
- `tip` 可选且非空，只保存给 Agent 的判断/询问建议，不能保存用户事实。
- 未定义字段和 JSON `null` 一律拒绝。

## 13. 用户 Achievement 状态

同步仓库根目录使用 `achievement-states.json`：

```json
{
  "states": {
    "cooking::first_signature_dish": {
      "status": "achieved",
      "achieved_at": "2025-06"
    },
    "cooking::host_a_dinner": {
      "status": "tracked"
    }
  }
}
```

- 文件缺失表示没有任何用户状态；文件存在时 `states` 必填且非空，key 按 Achievement ID 排序。
- 每个 Achievement 最多一条状态，且只允许 `tracked` 或 `achieved`。
- `achieved_at` 只允许出现在 achieved 状态，且必须是有效的 `YYYY`、`YYYY-MM` 或 `YYYY-MM-DD`；它仍然可省略。
- tracked 不保存开始时间、note、progress detail 或 may-be-incomplete；相应事实属于 Record 或 AssistantMemory。
- Pack 被停用或删除时保留状态。找不到 Definition 的状态标记为 unresolved，仍可原样导出，但不能通过普通 UI 改成 tracked/achieved；显式撤销始终允许。
- 显式撤销删除整个 entry。Record 变化和 prerequisites 不会自动撤销 achieved。
- 未定义字段、空对象和 JSON `null` 一律拒绝。

## 14. `skills.json`

```json
{
  "skills": [
    {
      "id": "cooking::general",
      "name": "烹饪",
      "description": "从基础料理到独立宴客。",
      "level_thresholds": [10, 20, 30, 40],
      "nodes": [
        {
          "achievement_id": "cooking::first_signature_dish",
          "points": 15
        },
        {
          "achievement_id": "cooking::host_a_dinner",
          "points": 25
        }
      ],
      "card_image": "assets/cooking-card.png"
    }
  ]
}
```

- 文件存在时 `skills` 必填、非空并按 `id` 排序；没有 Skill 时省略整个文件。
- Skill ID 必须是 `<manifest.id>::<local_id>`；`name` 必填，`description` 可选。
- `level_thresholds` 必须恰好包含四个严格递增的正整数，分别表示进入 Lv.2～Lv.5 的最低总积分。
- `nodes` 必填、非空并按 `achievement_id` 排序；同一 Skill 不得重复引用 Achievement。
- 节点只能引用同 Pack Achievement；`points` 必须是正整数。
- Lv.5 threshold 不得超过全部节点的积分总和，避免创建永远无法达到的等级。
- `card_image` 可选，必须是 Pack 目录内以 `assets/` 开头、使用 `/` 的相对路径；禁止绝对路径、空 segment、`.`、`..` 和反斜杠，且目标必须是内容与扩展名一致的 PNG、JPEG 或 WebP 普通文件。第一版不接受 SVG 作为 `card_image`。
- 未定义字段和 JSON `null` 一律拒绝。

等级算法固定为：总分为 0 时 Lv.0；总分大于 0 时至少 Lv.1；依次跨过四个 threshold 后进入 Lv.2～Lv.5。只有 achieved Achievement 节点贡献积分。

## 15. SQLite

- `pack_achievements` 保存每个 Pack 的完整 AchievementDefinition JSON，并对 `achievement_id` 建全局唯一约束。
- `achievement_states` 保存最小用户状态，不对 Pack Achievement 建外键，以保留 unresolved 状态。
- `pack_skills` 保存完整 SkillDefinition JSON，并对 `skill_id` 建全局唯一约束。
- `pack_assets` 保存 Pack asset 的规范相对路径和原始 bytes，使一次 import 可以原子切换完整 Pack，而不在运行时混读 Git 工作区文件。
- Skill 积分、等级和节点解锁状态不建表、不缓存，每次从 SkillDefinition 与 achieved 状态计算。
- 删除 Pack 只级联删除该 Pack 的 Definition；用户 Achievement 状态继续保留。
