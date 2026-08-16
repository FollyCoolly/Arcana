# 外部 Agent Skill 与 CLI 合约

> **状态**：Current
> **最后更新**：2026-08-16

## 1. 边界

- **Arcana Skill** 是 Pack 内由 achieved Achievement 派生积分/等级的领域对象。
- **Agent Skill** 是 Codex 等外部 harness 使用的操作说明，例如 Velvet Room、Phan Site、Pack Manager。

Agent Skill 不属于用户数据，不拥有或同步 Agent Session。它通过 `arcana-data` 读取和修改 Arcana；运行时不得直接编辑 SQLite 或 live JSON，以免绕过领域验证。

## 2. 唯一源码

```text
plugins/arcana/
├── .codex-plugin/plugin.json
├── fixtures/contract-v1/
├── evals/scenarios.json
└── skills/{velvet-room,phan-site,pack-manager}/

.claude/{skills,fixtures}/      # generated mirror
scripts/sync_agent_skills.py
```

只人工维护 `plugins/arcana/`。CLI 合约与 Skill 变化必须同一提交更新；运行 `python scripts/sync_agent_skills.py` 生成镜像，`--check` 检查漂移。

## 3. 进程合约

- 成功：exit 0，stdout 为直接业务 JSON，stderr 为空。
- 领域错误：exit 1；调用错误：exit 2；busy/storage/runtime：exit 3。
- 失败时 stdout 为空，stderr 为 `{code,message,details}`；控制流只依赖稳定 `code` 与 `details`。
- 不增加 `{ok,data}` 结果 Envelope；`--compact` 只改变空白。
- 写入前读取 `capabilities`，要求已知 `contract_version`、命令版本、structured errors 与 dry-run。
- CLI 不提供任意 SQL、任意路径写入或跳过校验。

当前命令族：

```text
init
capabilities
context summary
record get|query|set|increment|correct|create-empty-collection|create-empty-event|add-item|correct-item|remove-item|append-event|correct-event|delete-event|delete
achievement list|state-set|state-revoke
skill list
mission list|create|update|complete|archive|delete|suggestion-list|suggest|accept|reject|suggestion-delete
memory list|create|update|delete
pack list|show|scaffold|validate|write|asset-put|asset-delete|enable|disable|delete
status list-dimensions|evaluate|select
batch apply
json import|export
```

Git `sync` 命令族尚未实现。

## 4. Dry-run 与 batch

所有领域 mutation 可用单命令 `--dry-run`。dry-run 生成的 UUIDv7 与时间只是预览；正式执行必须复用相同输入，而不能引用预览 ID。

多操作 `batch apply` 只支持 `capabilities.commands.batch.operations` 列出的 `record.*` 操作：

```json
{
  "operations": [
    {
      "operation": "record.set",
      "input": {
        "definition_id": "identity.nickname",
        "value": "Alice"
      }
    },
    {
      "operation": "record.increment",
      "input": {
        "definition_id": "fitness.run_count",
        "delta": 1
      }
    }
  ]
}
```

Record batch 按数组顺序在一个 SQLite transaction 中执行，失败返回 `operation_index` 与 `operation` 且无部分写入。Pack、Achievement、Mission、Suggestion、Memory 和本机选择不允许进入多操作 batch；它们属于 JSON store，应逐条 dry-run 和提交。一次用户输入跨存储时，Skill 必须明确执行顺序和中途失败结果，不能声称跨存储原子性。

## 5. 读取与派生

- Record query 可按 Definition ID、namespace、Pack、kind 和是否有值过滤。
- Status 从 live Dimension Definitions 与 SQLite Records 即时计算；selection 来自本机 JSON。
- Achievement list 合并 live Definitions 与 JSON AchievementState；unresolved 状态不丢失。
- Skill points/level 只从 achieved Achievement 即时派生。
- MissionSuggestion 来自本机 JSON；accept 后 Mission 写入 live JSON repository。
- AssistantMemory 直接读取 live JSON。
- `context summary` 组合三种 store，只返回启动所需摘要；完整 Record/Definition/Suggestion 应按需查询。

## 6. Canonical Skills

### Velvet Room

把用户明确陈述变为最小真实更新：

1. targeted query 相关 Definitions、Records、Achievements、Missions 与 Memory；
2. 区分明确事实、候选判断和未知信息，不虚构日期/数量/历史；
3. 把相关 Record mutations 组成一个 batch 并 dry-run；
4. Achievement/Mission/Memory/selection 各自 dry-run；
5. 先提交 Record batch，再提交已授权的 JSON mutations；任何一步失败即停止并报告；
6. 重读结果。

用户直接说明 Achievement 已完成即可设置 achieved，不强迫补齐历史 Record。Record 更新不会自动证明或撤销 Achievement。Memory 只保存跨会话有价值的精炼偏好、约束、习惯、重点和提醒。

### Phan Site

根据 active Mission、tracked Achievement、相关 Records 与 Memory 生成 MissionSuggestion。每条 Suggestion 单独 dry-run/提交；不能绕过用户接受创建 Mission。pending/rejected Suggestion 不同步，拒绝结果保留本机用于去重。

### Pack Manager

创建、扩展与校验 Pack：

- Pack 必须完整声明自身 Dimension/Achievement 使用的 RecordDefinitions；
- PackForest 只组织，不提供隐式依赖；
- Achievement 用自然语言 requirement 与可选 `tip`；
- Status 只有一层 weighted Scores；
- 先 scaffold/validate/dry-run/write，再按需单独 enable；
- asset 只通过专用命令写入；
- Pack mutations 不进入 Record batch。

## 7. 安全与质量门

- 不生成用户未提供的事实、历史、证据或完成状态。
- 不存 credentials、绝对路径、完整聊天、模型配置或 Session。
- conflict、unresolved、Schema mismatch、Git conflict 与 validation issue 必须直接暴露。
- 重复输入不得重复创建 collection item、event 或 Suggestion。
- fixture 必须覆盖 capabilities、payload、structured errors、dry-run 与 Record batch rollback。
- Skill 源与 `.claude` 镜像必须一致，三个 Skill 均通过 Skill Creator validator。

具体 payload 以 canonical Skill references 和 `plugins/arcana/fixtures/contract-v1/` 为准。
