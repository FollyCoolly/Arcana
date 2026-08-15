# Achievement、Skill 与 Content Pack

> **状态**：Target / 领域语义已确定，Pack 物理 Schema 尚未定稿
> **最后更新**：2026-08-15

## 1. 依赖方向

```text
RecordData + AchievementDefinition + Agent judgment
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
- 可选 `related_record_ids`：已经存在、可能用于判断或跟踪的 RecordSet；
- 可选 `tip`：给 Agent 的自然语言辅助说明。除了补充特殊或容易误解的完成要求，也可以说明值得关注、询问或收集哪些信息，尤其适用于尚未预先定义对应 RecordData 的情况。

`related_record_ids` 用于事实更新后的候选路由，不表示 RecordData 是完成成就的唯一证据，也不要求每个 Achievement 都预先定义 RecordData。`tip` 中描述的是跟踪建议和信息类型，不保存某个用户的实际事实或进度；用户数据仍不能写入共享的 AchievementDefinition。

第一版不定义机器可执行的 `criterion`、`completion`、`auto_unlock_rule` 或 Achievement 查询 DSL。Agent 读取完成要求、相关事实和可选 tip 后作出判断；依据不足时询问用户。

## 3. Progress 与用户状态

### 3.1 Progress

- 不持久化统一数值进度或自由格式 `progress_detail`。
- 需要展示或讨论时，由 Agent 根据现有 RecordData 即时总结。
- 不要求所有成就都有可量化进度。
- 不允许为了凑进度而反向生成虚构 RecordData。

### 3.2 `tracked`

`tracked` 表示用户主动关注、收集或补充某个 Achievement 所需事实，可用于：

- UI 的重点/进行中列表；
- Agent 选择后续询问内容；
- 没有相关 RecordData 时，根据 `tip` 识别本次对话中值得询问或用于判断的信息；
- 提醒用户可能存在未补充的历史信息。

`tracked` 不产生积分。

### 3.3 `achieved`

当 Agent 根据现有信息判断已经完成，或者用户明确表示自己已经完成时，将状态设为 `achieved`。两种方式没有不同的后续行为，因此不持久化来源类型。

用户自我确认不要求先补齐全部历史明细。状态记录只包含：

- 必填的 `status`；
- `achieved` 状态下可选的 `achieved_at`，支持 `YYYY`、`YYYY-MM` 或 `YYYY-MM-DD`。

不保存独立完成记录 ID、来源、授予时间、说明或证据。相关事实属于 RecordData，需要长期保留的语义信息属于 AssistantMemory。

RecordData 后来变化不会自动撤销 `achieved`。用户可以显式撤销；撤销时删除用户状态，若仍希望继续关注则再显式设为 `tracked`。撤销操作本身不触发重新判断，也不需要永久 suppress 状态。

prerequisites 可以影响 UI 展示和 Agent 推荐，但不能阻止用户确认一个已经完成的成就。

## 4. Agent 工作流

Arcana 不运行持续扫描全部成就的后台线程。典型流程为：

1. 用户向外部 Agent 描述经历，或 Agent 在授权下读取日记/周记。
2. Agent 提取明确事实并通过 typed command 写入 RecordData。
3. 根据本次 RecordSet 找出相关 Achievement。
4. Agent 读取要求、tip 和所需事实。
5. 满足时将用户成就状态设为 `achieved`；不确定时询问。

用户也可以随时直接声明完成，系统据此将状态设为 `achieved`。

## 5. Arcana Skill

- Skill 定义属于 Pack。
- Skill 节点引用同 Pack Achievement，并定义积分。
- 只有状态为 `achieved` 的 Achievement 节点计分。
- `tracked`、即时 Progress 和 RecordData 不直接计分。
- Skill 积分、节点状态和等级全部派生，不同步。
- 子 Pack 拥有独立 Skill；子 Pack 的已完成成就不自动向父 Pack Skill 注入积分。

第一版沿用 Lv.0～Lv.5 和四个升级 threshold 的总体表现。具体 key achievement 约束是否保留，在迁移 Skill Schema 时根据现有 Pack 校准。

## 6. Pack 内容

Pack 是兴趣领域与成长定义的组织/分发单元，可以包含：

- manifest；
- RecordSet 模板和兼容要求；
- AchievementDefinition；
- Arcana Skill 定义；
- 可选 DimensionDefinition。

Pack 只有 enabled/disabled 状态，不区分“官方”。manifest 保留 `author`。

Pack 不拥有用户 RecordData 或用户成就状态。Pack 直接拥有其 DimensionDefinition；关闭 Pack 不删除 Pack 内容或用户 RecordData。

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
- 修改 `parent_pack_id` 不改变 RecordData、用户成就状态或 Skill 分数；
- 第一版不支持多父级 DAG 或任意 Pack 运行依赖。

若未来确实需要安装/运行依赖，应增加单独的 `requires` 机制，不能复用 `parent_pack_id`。

## 8. RecordSet 复用

父 Pack 最初提供的 RecordSet 模板被采用后形成全局定义，不再由父 Pack 独占。子 Pack 使用相同数据时必须显式声明该 RecordSet 及兼容要求：

- 不存在：从子 Pack 自带模板创建；
- 已存在且兼容：复用；
- 已存在但不兼容：阻止启用并显示 diff；
- 只引用 ID、不声明兼容要求：Pack 无效。

因此 `parent_pack_id` 不承担 RecordSet 继承语义。

## 9. Status Dimension

Pack 可以直接定义 Dimension，不经过模板采用或复制。已启用 Pack 的 Dimension 进入本机 UI 可选列表，用户从中选择五个展示。选择只引用稳定 Dimension ID；自定义 Dimension 应直接放在用户维护的 Pack 中。

## 10. 内容演进

第一版以用户仓库中的 Pack 文件为权威，由 Git 提供历史；暂不实现社区 Pack 自动更新、三方合并或版本依赖解析。

稳定 ID 规则：

- 被用户成就状态、Skill 或 Status 引用的 ID 不直接修改；
- 仅修改名称、描述和 tip 不需要更换 ID；
- 根本改变 Achievement 语义时创建新 ID；
- RecordSet 的破坏性修改必须显式迁移。

Pack 内容版本字段是否保留为来源元数据、以及兼容迁移格式，仍需在物理 Schema 定稿时确认。
