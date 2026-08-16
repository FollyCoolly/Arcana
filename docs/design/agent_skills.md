# 外部 Agent Skill 与 CLI 合约

> **状态**：Target / 分发、边界与质量门已确定
> **最后更新**：2026-08-16

## 1. 术语与边界

- **Arcana Skill**：Pack 中由 achieved Achievement 计算积分和等级的游戏化领域对象。
- **Agent Skill**：Codex、Claude Code 等外部 harness 加载的操作说明，例如 Velvet Room、Phan Site、Pack Manager。

Agent Skill 不属于用户数据，不进入用户 Git 同步仓库，也不拥有 Agent Session。它只是 typed CLI 的智能客户端。

## 2. 唯一源码与分发

目标仓库结构：

```text
plugins/arcana/
├── .codex-plugin/plugin.json
└── skills/
    ├── velvet-room/SKILL.md
    ├── phan-site/SKILL.md
    └── pack-manager/SKILL.md

.claude/skills/                 # 由兼容脚本生成，不手工维护
```

- `plugins/arcana/skills` 是唯一人工维护的 Skill 源码。
- Codex 使用本地 plugin/marketplace 安装；其他 harness 由构建脚本复制或转换 canonical Skill。
- Windows 不依赖符号链接；生成脚本提供 `generate` 和 `--check`，CI 检查镜像没有漂移。
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
- 所有普通领域数据修改命令支持 `--dry-run`，返回将修改的实体和验证结果但不提交；新仓库使用独立的 `init` 命令，不提供旧 JSON 数据迁移命令。
- 多实体更新使用 `batch apply --file <json>`，在一个 SQLite 事务中全部成功或全部回滚。
- CLI 不暴露任意 SQL、任意文件写入或“跳过验证”参数。

第一版命令族：

```text
init
capabilities
context summary
record get|query|set|increment|correct|create-empty-collection|create-empty-event|add-item|correct-item|remove-item|append-event|correct-event|delete-event|delete
achievement list|state-set|state-revoke
mission list|create|update|complete|archive|delete
mission suggestion-list|suggest|accept|reject|delete
memory list|create|update|delete
pack list|show|scaffold|validate|write|asset-put|asset-delete|enable|disable
status list-dimensions|evaluate|select
batch apply
json import|export
sync status|import|export|pull|push|run
```

`json import|export` 是不接触 Git 的底层完整目录转换命令；`sync` 后续在它之上增加 managed path digest、防覆盖、恢复 journal 和显式 Git 操作。

当前实现已经提供新运行时的 `capabilities`、`init`、完整 `record` 命令族和 `json import|export`，并实现直接业务 JSON、结构化错误和稳定退出语义。`arcana-data init [--runtime <directory>]` 创建只含启用状态 `basic` Pack 的新 SQLite；Record 命令统一使用 `arcana-data record [--runtime <directory>] <action>`，省略 runtime 时读取本机 settings 或默认目录。`get` 返回当前 Record，`query` 可按 `--definition-id`、`--namespace`、`--pack`、`--kind`、`--has-value` 组合过滤；修改复杂 payload 的命令从 stdin 或 `--file` 读取对应 Application Command JSON。旧 `context/read/mission/status/achievement/pack/changelog/memory` JSON CLI 和旧 `.claude/skills` 已删除，必须等对应 SQLite 命令完成后再发布 canonical Agent Skill；`--dry-run` 和 batch 尚未实现。

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
- 结构化内容只通过 `pack write`/batch transaction 修改，资源只通过 `pack asset-put` / `pack asset-delete` 导入或删除；不直接写 Pack JSON 或 Git 工作区 asset。

## 7. 安全与数据原则

- 不生成用户未提供的历史 Record、日期、证据或完成状态。
- 用户明确表示已经完成 Achievement 时允许直接设为 achieved，不强迫补齐跟踪数据。
- 不把 credentials、绝对本机路径、完整聊天、模型配置或 Session 写入同步实体。
- 不自动解决 Git 冲突，不调用破坏性 Git 命令。
- CLI 返回 conflict、unresolved、schema mismatch 或 validation error 时，Skill必须把具体问题暴露给用户。
- 所有展示给用户的计划与实际 committed batch 必须一致；dry-run 后数据变化则重新验证。

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

CI 只验证确定性能力；自然语言判断质量使用固定场景的人工/模型 eval 报告。未通过质量门的 Skill 不随正式构建发布。
