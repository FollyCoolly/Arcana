# 外部 Agent Skill 与 CLI 合约

> **状态**：Target / 分发、边界与质量门已确定
> **最后更新**：2026-08-15

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

`arcana-data` 是所有 Agent Skill 的唯一数据写入口。CLI 提供稳定整数 `contract_version = 1`，所有结果使用 JSON：

```json
{
  "ok": true,
  "contract_version": 1,
  "data": {}
}
```

```json
{
  "ok": false,
  "contract_version": 1,
  "error": {
    "code": "record_unresolved",
    "message": "RecordDefinition is unavailable.",
    "details": {}
  }
}
```

- Agent 只判断 `ok`、稳定 error code 和结构化 details，不解析人类错误文本。
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
record get|query|set|increment|correct|add-item|remove-item|append-event|correct-event|delete-event|delete
achievement list|state-set|state-revoke
mission list|create|update|complete|archive|delete
mission suggestion-list|suggest|accept|reject|delete
memory list|create|update|delete
pack list|show|scaffold|validate|write|asset-put|asset-delete|enable|disable
status list-dimensions|evaluate|select
batch apply
sync status|import|export|pull|push|run
```

实际 flag 和请求体由 CLI `--help --json` 生成文档；Skill 不复制完整 JSON Schema，而是在需要时查询 capabilities/schema，避免随代码漂移。

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

- CLI contract fixture：支持版本、未知版本、结构化错误和 dry-run。
- Golden scenario：普通成功、信息不足、用户纠正、重复输入、撤销和 Pack 缺失。
- 数据真实性 eval：不得从模糊语言虚构 Record 或 achieved。
- 幂等性 eval：重复处理同一句输入不得重复 item/event/MissionSuggestion。
- 事务失败注入：batch 中任一操作失败时无部分写入。
- Schema drift：Skill 镜像、示例和 capabilities 与当前代码一致。
- Pack quality：重复 Definition、DAG 环、不可达 Skill Lv.5、无效表达式和路径穿越必须被拒绝。
- Windows/macOS fixture：路径、编码、换行和 CLI 启动方式均通过。

CI 只验证确定性能力；自然语言判断质量使用固定场景的人工/模型 eval 报告。未通过质量门的 Skill 不随正式构建发布。
