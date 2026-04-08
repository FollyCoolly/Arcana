# CLAUDE.md

RealityMod — 一个 Persona 5 风格的游戏化人生管理桌面应用，"adding a user interface to Earth Online"。

## 技术栈

- **后端**: Rust + Tauri v2（IPC commands、AI agent、JSON 数据层）
- **前端**: Svelte 5 + SvelteKit v2 + TypeScript + Tailwind CSS v4 + Three.js
- **数据**: 本地 JSON 文件（`data/`，gitignored），无数据库
- **AI Agent**: 独立 Rust 子系统，直接调 Anthropic API，支持 CLI / Telegram / Tauri 内嵌三种运行模式

## 项目结构

```
src/                    # SvelteKit 前端
src-tauri/src/          # Rust 后端
  ├── commands/         #   Tauri IPC commands（achievements, gallery, items, missions, skills, status, weather）
  ├── models/           #   Serde 数据结构
  ├── storage/          #   JSON 读写 & 日期工具
  ├── agent/            #   AI agent 子系统（runner, llm, tools, prompt, config, session, bus, channels/）
  └── bin/              #   独立二进制：agent_cli.rs, agent_telegram.rs
data/                   # 运行时 JSON 数据（gitignored）
  ├── packs/<pack_id>/  #   成就包（manifest.json, achievements.json, skills.json）
  ├── sessions/         #   Agent JSONL 会话历史
  └── *.json            #   missions, status, achievement_progress, mission_memory, ai_changelog 等
docs/                   # 架构文档、schema 规范、UI 设计指南
  └── schema/           #   各 JSON 文件的完整 schema 定义
scripts/                # Python 工具脚本（数据导入、schema 校验）
.claude/skills/         # Claude Code 自定义 Skill（velvet-room, phan-site, pack-manager）
static/                 # 静态资源（图标、图片）
```

## 构建 & 开发命令

```bash
npm install                                          # 安装前端依赖
npm run dev                                          # Vite dev server（仅前端）
npm run tauri dev                                    # 完整桌面应用开发模式
npm run build                                        # 构建前端
npm run tauri build                                  # 构建桌面发行包
npm run check                                        # Svelte/TypeScript 类型检查
cargo test --manifest-path src-tauri/Cargo.toml      # Rust 测试
cargo fmt --manifest-path src-tauri/Cargo.toml       # Rust 格式化
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-cli      # 构建 CLI agent
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-telegram # 构建 Telegram bot
```

PR 前必须通过：`npm run check` + `cargo test` + `cargo fmt --check`。

## 代码规范

- **TypeScript/Svelte**: 2 空格缩进，strict 模式，`PascalCase` 组件/类型名，`camelCase` 函数/变量
- **Rust**: `snake_case` 函数/模块，`PascalCase` 结构体/枚举，Tauri command 错误信息要可操作
- **Commit**: Conventional Commits 风格 — `feat:`, `fix(scope):`, `docs:`, 祈使语气，如 `feat(status): add BMI card`
- 前后端模块边界对齐：status 相关的 UI 逻辑对应 status 相关的 backend command

## UI 设计约束（Persona 5 风格）

- **调色板**: 仅 `#000000`、`#ffffff`、`#E5191C`，**不使用渐变**
- 所有交互元素必须有几何底座（不允许裸文字按钮）
- 状态变化必须是形状+颜色双重变化（不能只改透明度）
- 动画时长：fast=120ms, base=180ms, slow=260ms, 曲线=`cubic-bezier(0.2, 0.8, 0.2, 1)`
- 详见 `docs/visual_style_guide.md` 和 `docs/ui_design_spec.md`

## 数据 Schema 核心规则

所有 schema 的完整定义在 `docs/schema/` 下，以下是 AI 操作数据时必须遵守的硬性规则。

### 通用

- 可选字段省略，**不写 `null`**
- 日期格式：`YYYY-MM-DD`；时间戳：ISO 8601
- JSON 文件统一使用 `{"version": 1, ...}` 顶层结构

### 成就包 (`data/packs/<pack_id>/`)

- 所有 ID 格式：`<pack_id>::<snake_case_name>`，`manifest.id` 必须等于目录名
- `prerequisites` 仅引用同包成就，必须构成 DAG（无环）
- `difficulty` 枚举：`beginner` / `intermediate` / `advanced` / `expert` / `legendary`
- 技能树 `level_thresholds` 数量 == `max_level`，`points_required` 严格单调递增
- `required_key_achievements` 是增量式的（每级只加新要求）
- **Refine/Extend 模式下禁止修改已有的 achievement ID**（会破坏进度数据）

### 成就进度 (`data/achievement_progress.json`)

- 不在 map 中 = locked；`status: "tracked"` = AI 追踪中；`status: "achieved"` = 已完成
- `progress_detail` 只追加，不替换
- `may_be_incomplete: true` 表示用户可能有未上报的历史进度
- `achieved_at` 支持三种精度：`YYYY`、`YYYY-MM`、`YYYY-MM-DD`

### 任务 (`data/missions.json`)

- 状态生命周期：`proposed` → `active` → `completed` / `archived` / `rejected`
- `progress`：0-100 整数，AI 写入
- AI 生成的任务 ID 格式：`ai_<YYYYMMDD>_<slug>`
- `main_menu.label` 是 AI 精心撰写的简洁展示文本，**不是标题的复制**
- 任务描述**不得包含进度预测**
- `rejected` 任务对 UI 隐藏但保留用于去重

### AI Changelog (`data/ai_changelog.json`)

- 结构：`{"version": 1, "entries": [...]}`
- **每次数据变更后必须写 changelog**，包含 `old_value`
- `skill` 字段：`velvet-room` / `phan-site`（Claude Code Skill）或 `agent`（Rust agent）
- 最多 200 条，FIFO 淘汰
- `mission_memory.json` 的变更**不需要**写 changelog

### AI 记忆 (`data/mission_memory.json`)

- `conversation_context` 最多 20 条（FIFO）
- `completed_mission_log` 最多 50 条（FIFO）
- 此文件是 AI 内部状态，变更不写 changelog

### Status (`data/status.json`)

- 双文件模型：`status_metric_definitions.json` 定义指标，`status.json` 存当前值
- 写入前必须校验 metric ID 存在于 definitions 中

## AI Agent 架构

### Rust Agent（运行时）

位于 `src-tauri/src/agent/`，是一个自建的 tool-calling 循环：

- **runner.rs**: LLM ↔ 工具调用主循环（最多 20 轮迭代）
- **llm.rs**: Anthropic Messages API 客户端（非流式）
- **tools.rs**: 6 个工具 — `get_context`、`read_file`、`update_mission`、`update_status`、`update_achievement`、`write_changelog`
- **prompt.rs**: 运行时构建系统 prompt（Velvet Room 人设）
- **config.rs**: 分层配置加载（默认值 → 用户级 → 项目级 → 环境变量）
- **session.rs**: JSONL 持久化会话（按 session_key 分文件，最多 40 条消息）
- **bus.rs**: tokio mpsc 消息总线
- **channels/telegram.rs**: Telegram bot 适配器（teloxide，ACL 控制，消息分片）

写操作共享 `Mutex<()>` 写锁，防止 agent 与 Tauri command 并发写入。

`read_file` 工具有沙箱保护：拒绝绝对路径、`..` 遍历、符号链接逃逸。

### Claude Code Skills（开发时）

| Skill | 用途 | 触发方式 |
|-------|------|----------|
| `velvet-room` | 通用进度汇报：解析自然语言 → 更新 missions/achievements/status | `/velvet-room` |
| `phan-site` | 任务提案生成：基于目标和记忆生成 3-5 个任务 | `/phan-site` |
| `pack-manager` | 成就包管理：创建 / 优化 / 扩展 achievement pack | `/pack-manager` |

### Agent 行为规范

1. **始终先调 `get_context`** 获取当前状态
2. 执行数据变更后 **必须写 changelog**（含 `old_value`）
3. 每次会话结束 **必须更新 `mission_memory.json`**
4. `phan-site` 生成的任务标题必须是游戏化任务名（如 "Borrow Checker Gauntlet"），不是字面描述
5. 回复简洁，默认中文，以 "变更摘要:" 开头

## 数据校验

`scripts/validate_data.py` 作为 PostToolUse hook 在 Claude Code 每次写 `data/*.json` 后自动运行。校验覆盖：

- missions.json: ID 唯一性、status 枚举、progress 范围、main_menu 引用
- achievement_progress.json: status 有效性
- ai_changelog.json: 结构完整性、entries 上限
- mission_memory.json: FIFO 上限
- status.json: 值必须为数字
- 成就包: ID 前缀、DAG、技能树单调性
- Changelog 新鲜度检查（非阻塞警告）

## 关键设计决策

- **Tauri + JSON 而非 Electron + SQLite**: 更小体积、更好性能、数据可读可版本控制
- **Content Pack 体系**: 成就和技能通过 pack 插拔，支持社区扩展
- **Agent 与 UI 解耦**: Agent 可独立于桌面 GUI 运行（CLI / Telegram），共享数据层
- **DAG 技能树**: 前端从 `prerequisites` 自动推导边和布局，不存冗余数据
- **MCP Server 计划中**: 目标是统一 Claude Code Skill 和 Rust Agent 的数据入口（见 `docs/ai_agent_integration.md`）
