# 外部 Agent Skill 与 CLI 合约

> **状态**：Canonical plugin、Skills、contract fixtures 与固定 eval 场景已实现
> **最后更新**：2026-08-16

## 1. 术语与边界

- **Arcana Skill**：Pack 中由 achieved Achievement 计算积分和等级的游戏化领域对象。
- **Agent Skill**：Codex、Claude Code 等外部 harness 加载的操作说明，例如 Velvet Room、Phan Site、Pack Manager。

Agent Skill 不属于用户数据，不进入用户 Git 同步仓库，也不拥有 Agent Session。它只是 typed CLI 的智能客户端。

## 2. 唯一源码与分发

当前仓库结构：

```text
plugins/arcana/
├── .codex-plugin/plugin.json
├── fixtures/contract-v1/
├── evals/scenarios.json
└── skills/
    ├── velvet-room/{SKILL.md,agents/,references/}
    ├── phan-site/{SKILL.md,agents/,references/}
    └── pack-manager/{SKILL.md,agents/,references/}

.claude/{skills,fixtures}/      # 由兼容脚本生成，不手工维护
scripts/sync_agent_skills.py    # generate / --check
```

- `plugins/arcana/skills` 是唯一人工维护的 Skill 源码。
- Codex 使用本地 plugin/marketplace 安装；其他 harness 由构建脚本复制或转换 canonical Skill。
- Windows 不依赖符号链接；运行 `python scripts/sync_agent_skills.py` 生成镜像，运行 `python scripts/sync_agent_skills.py --check` 检查漂移。
- Skill 与 CLI 合约变更必须在同一提交更新；不维护一套内置 prompt 和一套外部 Skill 业务规则。
- 用户数据 Pack 与 Agent plugin 是两种独立分发物，不能混放。

## 3. CLI 合约

`arcana-data` 是所有 Agent Skill 的唯一数据写入口。CLI 使用稳定整数 `contract_version = 1`。成功时进程以 0 退出，并把业务 JSON 直接写到 stdout：

```json
{
  "record": {
    "definition_id": "identity.nickname",
    "value": "Alice",
    "recorded_at": "2026-08-15T12:00:00Z"
  }
}
```

失败时进程以非零状态退出，stdout 为空，stderr 只包含结构化 JSON：

```json
{
  "code": "record_unresolved",
  "message": "RecordDefinition is unavailable.",
  "details": {}
}
```

- JSON 是 CLI 的数据交换格式；不增加包含 `ok`、`data` 和重复版本号的结果 Envelope。Agent 先判断退出状态，失败时只读取稳定 error code 和结构化 details，不解析人类错误文本。
- `--help` / `--version` 是面向人的普通文本，不属于机器结果；`--compact` 只改变 JSON 空白，不改变字段。
- `capabilities` 返回 CLI contract、仓库 Schema、Pack Schema 和支持命令版本；Skill 开始写操作前必须检查兼容性。
- 读命令支持精确 ID、namespace、Pack 和状态过滤，避免每次把完整用户仓库塞入上下文。
- Record、Status selection、Achievement、Mission/Suggestion 和 AssistantMemory 修改命令支持 `--dry-run`，返回将修改的实体并在验证后回滚；新仓库使用独立的 `init` 命令，不提供旧 JSON 数据迁移命令。
- 多个用户状态更新使用 `batch apply --file <json>`，按数组顺序在一个 SQLite 事务中执行，后续操作可以读取前序结果；全部成功才提交，任一失败则全部回滚。
- Pack 结构化内容与启用状态可通过 `pack.write|enable|disable|delete` 进入 batch；二进制 asset 仍使用 `asset-put|asset-delete` 专用流程。Pack mutation 支持 `--dry-run`，`init`、查询、`pack validate` 和 `json import|export` 不接受 `--dry-run`。
- CLI 不暴露任意 SQL、任意文件写入或“跳过验证”参数。

第一版命令族：

```text
init
capabilities
context summary
record get|query|set|increment|correct|create-empty-collection|create-empty-event|add-item|correct-item|remove-item|append-event|correct-event|delete-event|delete
achievement list|state-set|state-revoke
skill list
mission list|create|update|complete|archive|delete
mission suggestion-list|suggest|accept|reject|suggestion-delete
memory list|create|update|delete
pack list|show|scaffold|validate|write|asset-put|asset-delete|enable|disable
status list-dimensions|evaluate|select
batch apply
json import|export
sync status|import|export|pull|push|run
```

`json import|export` 是不接触 Git 的底层完整目录转换命令；`sync` 后续在它之上增加 managed path digest、防覆盖、恢复 journal 和显式 Git 操作。

当前实现已经提供新运行时的 `capabilities`、`init`、`context summary`、用户状态 mutation 的 `--dry-run` 与 `batch apply`、完整 `record`、`pack`、`status`、`achievement`、`mission`、`memory` 命令族、只读 `skill list` 和 `json import|export`，并实现直接业务 JSON、结构化错误和稳定退出语义。`arcana-data init [--runtime <directory>]` 创建只含启用状态 `basic` Pack 的新 SQLite；领域命令统一使用 `arcana-data <record|pack|status|achievement|skill|mission|memory> [--runtime <directory>] <action>`，省略 runtime 时读取本机 settings 或默认目录。Record `get` 返回当前 Record，`query` 可按 `--definition-id`、`--namespace`、`--pack`、`--kind`、`--has-value` 组合过滤；修改复杂 payload 的命令从 stdin 或 `--file` 读取对应 Application Command JSON。全部目标领域命令和 canonical Agent Skills 已按 SQLite 合约实现；固定 fixture 直接由 Rust process test 读取，确保 capabilities、Serde payload、错误、dry-run、原子回滚与 PackContent 样例不会漂移。

`batch apply` 输入采用稳定的相邻标签格式：

```json
{
  "operations": [
    {
      "operation": "record.set",
      "input": {
        "definition_id": "fitness.running_distance_km",
        "value": 5
      }
    },
    {
      "operation": "mission.complete",
      "input": { "mission_id": "019b..." }
    },
    {
      "operation": "achievement.state-set",
      "input": {
        "achievement_id": "fitness::first_run",
        "status": "achieved",
        "achieved_at": "2026-08-16"
      }
    }
  ]
}
```

支持的 operation 名称以 `capabilities.commands.batch.operations` 为准。成功结果按原顺序返回 `{index,operation,result}`；失败结果在 `details` 中提供 `operation_index` 和 `operation`，不返回部分成功。单条用户状态写命令和 `batch apply` 都接受全局 `--dry-run`；它们执行完全相同的事务内代码，只在结尾选择 rollback 或 commit。dry-run 中由系统生成的 UUIDv7 和时间是预览值，正式执行时会重新生成，调用方不得让后续输入引用这些预览 ID；除这些系统字段外，确认后提交的 operation 数组必须与预览一致，期间数据发生变化则重新 dry-run。

`pack scaffold <id> --name <name>` 直接输出可交给 `pack validate|write` 的 `PackContent` JSON，不要求 runtime 已初始化。`PackContent` 只包含 `manifest` 以及可选的 `record_definitions`、`dimensions`、`achievements`、`skills`；它不是新的持久化 Schema，也不包含 asset bytes。`pack validate` 使用当前 Pack 已有 asset，把候选内容放入当前仓库快照做全量校验但不写入。`pack write` 在单事务中插入或替换结构化内容，并保留原 enabled 状态和全部 asset。`pack.write` 可与 `pack.enable` 在 batch 中原子执行。asset bytes 只通过 `asset-put <pack_id> <assets/...> --file <local_file>` 与 `asset-delete` 修改；`show` 只返回 asset path 和 byte size，不把二进制编码进 JSON。启用/停用不级联父子 Pack，重复操作返回 `changed: false`。`pack delete --dry-run` 返回子 Pack、将变为 unresolved 的用户 Record/Achievement 状态和失效的本机 Status selection；实际删除 Definition 与 asset，但保留用户状态。

`status list-dimensions` 返回已启用 Pack 的完整 DimensionDefinition，以及本机五个 selection 是否仍然 available。`status evaluate [dimension_id]` 从同一 SQLite 事务快照读取 Pack、Record 与 selection；省略 ID 时计算全部有效 Dimension。每个结果包含子 Score 原始值、clamp 后分数、缺失 Record ID、Dimension 加权平均、Lv.0～Lv.5 和标题，但不持久化任何派生值。`status select <position> <dimension_id>` 只接受当前有效 Dimension；`status select <position> --clear` 显式清空位置。Pack 停用后 selection 保留并标记 unavailable，不静默补位。

`achievement list` 返回已启用 Pack 的 Definition，并额外返回 definition 不可用但仍有用户状态的 unresolved 项；支持 `--achievement-id`、`--pack`、`--status` 和 `--related-record-definition-id` 组合过滤。`availability` 是即时投影的 `locked/available/tracked/achieved/unresolved`，不入库。`state-set` 从 stdin 或 `--file` 读取 `{achievement_id,status,achieved_at?}`；prerequisites 只影响投影，不阻止直接设置 achieved。只有当前有效 Definition 可以 set；`state-revoke <id>` 在 Definition 或 Pack 不可用时仍可删除状态。重复操作返回 `changed: false`。

`skill list` 只查询已启用 Pack 的 Arcana Skill，支持 `--skill-id` 和 `--pack` 精确过滤。结果包含完整 SkillDefinition、节点 `availability`、当前/最高积分、已完成节点数和 Lv.0～Lv.5；全部结果从当前 Achievement 状态即时派生，不存在 Skill 用户状态或派生缓存。

Mission `create`/`suggest` 由系统生成 UUIDv7 和当前时间；`update` 完整替换可编辑字段，省略可选字段表示清除。`complete`、`archive` 和 `reject` 幂等；删除仍有 child 的 Mission 会返回 conflict。Suggestion 仅保存在本机，`accept` 在同一事务中把它转换为同 ID 的 active Mission，随后只有正式 Mission 进入 JSON 同步。

Memory `list` 支持按 ID 和 kind 精确过滤。`create` 由系统生成 UUIDv7 与 created/updated time；`update` 保留 ID/created_at，只在 kind 或 content 实际变化时刷新 updated_at；`delete` 直接移除条目，不创建软删除状态。

`context summary` 从一个 SQLite 事务快照返回本机日期、五个 Status selection（包括不可用选择）、active Mission、显式 tracked/achieved 状态和 AssistantMemory。可用 Status 同时附带即时分数与等级；有 deadline 的 Mission 附带即时 `days_remaining`。它不返回完整 Record、Pack/Definition、已归档/已完成 Mission 或尚未接受的 Suggestion；Agent 需要这些细节时使用对应的精确查询，避免每次启动都把整个仓库塞入上下文。

实际 flag 以 CLI `--help` 为准；机器先查询 `capabilities` 确认合约、Schema、命令版本和 feature。请求体 Schema 随后由 canonical Skill 的 fixture 覆盖，避免文档示例与 Serde 类型漂移。

## 4. Velvet Room

职责：把用户明确提供的经历转成 Record、Mission、Achievement 状态和必要的长期 Memory。

流程：

1. 用 targeted query 读取相关 Definition、Record、Achievement 和 active Mission。
2. 区分明确事实、合理候选和缺失信息；不把推测写成 Record。
3. dry-run 一个 batch，向用户说明需要确认的歧义。
4. 以单事务写入事实和显式状态变化。
5. 读取受影响 Achievement，按自然语言要求和 tip 判断；不运行隐藏规则 DSL。
6. 只有确有跨会话价值时更新 AssistantMemory。

它不写 changelog，不维护自由格式 progress detail，不扫描全部 Achievement，也不因为 Record 变化自动撤销 achieved。

## 5. Phan Site

职责：根据 active Mission、tracked Achievement、相关 Record 和 AssistantMemory 生成本机 MissionSuggestion。

- 生成结果只能写入 Suggestion，不能绕过用户接受直接创建 Mission。
- 拒绝结果留在本机用于去重；接受通过 CLI 原子转换成 active Mission。
- 不同步 pending/rejected Suggestion，不把完整 prompt 或会话写入 Memory。
- 任务应有明确结果和现实范围；相近 Suggestion 必须先查询去重。

## 6. Pack Manager

职责：创建、扩展和校验 Pack 内容。

- 先读取当前 Pack Schema 和同仓库相关 Definition，避免同义 RecordDefinition 重复。
- 一个 Pack 必须携带 Dimension/Achievement 实际引用的完整 RecordDefinition，即使其他 Pack 已声明相同 ID。
- 创建 Achievement 时评估是否需要 related RecordDefinition；不强迫每个 Achievement 都可量化。
- 生成后执行 Pack 全量校验、Achievement DAG、Definition 兼容、Status 表达式和 Skill 可达性检查。
- 修改已引用 ID 或破坏性 Definition 时拒绝直接覆盖，改为创建新 ID 并列出受影响引用。
- 结构化内容只通过 `pack validate` 后的 `pack write` 修改，资源只通过 `pack asset-put` / `pack asset-delete` 导入或删除；不直接写 Pack JSON 或 Git 工作区 asset。

## 7. 安全与数据原则

- 不生成用户未提供的历史 Record、日期、证据或完成状态。
- 用户明确表示已经完成 Achievement 时允许直接设为 achieved，不强迫补齐跟踪数据。
- 不把 credentials、绝对本机路径、完整聊天、模型配置或 Session 写入同步实体。
- 不自动解决 Git 冲突，不调用破坏性 Git 命令。
- CLI 返回 conflict、unresolved、schema mismatch 或 validation error 时，Skill必须把具体问题暴露给用户。
- 所有展示给用户的计划与实际 committed batch 的 operation 数组必须一致；系统生成的 UUID/时间允许变化，dry-run 后仓库数据变化则重新验证。

## 8. 质量门

每个 Agent Skill 发布前必须通过：

- CLI contract fixture：固定 capabilities 版本与命令集、结构化错误、退出状态和 dry-run feature 标记；Skill 遇到未知 contract version 时拒绝写入。
- Golden scenario：普通成功、信息不足、用户纠正、重复输入、撤销和 Pack 缺失。
- 数据真实性 eval：不得从模糊语言虚构 Record 或 achieved。
- 幂等性 eval：重复处理同一句输入不得重复 item/event/MissionSuggestion。
- 事务失败注入：batch 中任一操作失败时无部分写入。
- Schema drift：Skill 镜像、示例和 capabilities 与当前代码一致。
- Pack quality：重复 Definition、DAG 环、不可达 Skill Lv.5、无效表达式和路径穿越必须被拒绝。
- Windows/macOS fixture：路径、编码、换行和 CLI 启动方式均通过。

Rust process test 验证确定性能力；`plugins/arcana/evals/scenarios.json` 固定自然语言判断的输入、必需行为、禁用行为与 operation 边界，供人工或后续模型 eval runner 使用。当前没有把模型判断接入确定性 CI，也不把一次模型输出当作 Schema 正确性的替代品。
