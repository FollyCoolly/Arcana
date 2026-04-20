# CLAUDE.md

Arcana — 一个 Persona 5 风格的游戏化人生管理桌面应用，"adding a user interface to Earth Online"。

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
  └── bin/              #   独立二进制：agent_cli.rs, agent_telegram.rs, arcana_data.rs
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
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data   # 构建数据操作 CLI
```

PR 前必须通过：`npm run check` + `cargo test` + `cargo fmt --check`。

## 代码规范

- **TypeScript/Svelte**: 2 空格缩进，strict 模式，`PascalCase` 组件/类型名，`camelCase` 函数/变量
- **Rust**: `snake_case` 函数/模块，`PascalCase` 结构体/枚举，Tauri command 错误信息要可操作
- **Commit**: Conventional Commits 风格 — `feat:`, `fix(scope):`, `docs:`, 祈使语气，如 `feat(status): add BMI card`
- 前后端模块边界对齐：status 相关的 UI 逻辑对应 status 相关的 backend command

## UI 设计约束（Persona 5 风格）

- **调色板**: `#000000`、`#ffffff`、`#E5191C` + 数据可视化专用 `#F5A623`（`--rm-gold`） + 装饰结构专用 `#2E2E2E`（`--rm-gray`），**不使用渐变**
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

- 结构：`{"version": 1, "entries": [...]}`（Rust agent 已从旧的裸数组格式迁移）
- **每次数据变更后必须写 changelog**，包含 `old_value`
- `skill` 字段：`velvet-room` / `phan-site`（Claude Code Skill）或 `agent`（Rust agent）
- 最多 200 条，FIFO 淘汰
- `mission_memory.json` 的变更**不需要**写 changelog

### AI 记忆 (`data/mission_memory.json`)

- `conversation_context` 最多 20 条（FIFO）
- `completed_mission_log` 最多 50 条（FIFO）
- 此文件是 AI 内部状态，变更不写 changelog

### Status (`data/status.json` + `data/status_metric_definitions.json`)

- 三层模型：`metrics[]` 定义指标，`dimensions[]` 定义雷达维度，`status.json` 存当前值
- 指标 (Metric) 是纯数据字典（id, name, group, unit, value_type），不含评分逻辑
- 维度 (Dimension) 拥有评分配置（weight + target_max/target_min/scoring_brackets）和 P5 风格等级称号
- 系统指标以 `sys_` 为前缀，由后端实时计算，不存储在 `status.json`，不可通过 agent 写入
- 维度分数公式：`score = Σ(contribution × weight)`，等级由 `level_thresholds` 决定
- 写入 `status.json` 前必须校验 metric ID 存在于 definitions 中
- 详细 schema 见 `docs/schema/status.md`

## AI Agent 架构

### 共享服务层 (`services/`)

所有数据操作的业务逻辑集中在 `src-tauri/src/services/` 下，被 Rust agent、arcana-data CLI、Tauri commands 三条路径共用：

| 模块 | 功能 |
|------|------|
| `context.rs` | 读取 missions/status/achievements/memory 概览 |
| `file_access.rs` | 沙箱路径校验 + 安全文件读取 |
| `mission.rs` | update_mission + create_mission |
| `status.rs` | update_status |
| `achievement.rs` | update_achievement |
| `changelog.rs` | write_changelog（skill 字段由调用方传入） |
| `memory.rs` | update_mission_memory |

### arcana-data CLI（数据操作入口）

位于 `src-tauri/src/bin/arcana_data.rs`，基于 `clap` v4。Claude Code Skills 通过 Bash 调用。

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data   # 构建
```

**子命令：**

| 命令 | 类型 | 用途 |
|------|------|------|
| `context [--missions] [--status] [--achievements] [--memory] [--active-only] [--pack ID]` | 读 | 读取当前状态概览（支持过滤） |
| `read <path>` | 读 | 沙箱读取 data/ 下任意文件 |
| `mission update <id> [flags]` | 写 | 更新任务字段 |
| `mission update-menu [--countdown JSON] [--progress JSON]` | 写 | 更新 main_menu 配置 |
| `mission create < stdin` | 写 | 插入新任务（phan-site 用，JSON 从 stdin 读） |
| `status update <key=value>...` | 写 | 更新状态指标 |
| `achievement update <id> --status <s> [flags]` | 写 | 追踪/达成成就 |
| `changelog write --skill <s> --summary "..." < stdin` | 写 | 写审计日志（**MANDATORY**，changes JSON 从 stdin 读） |
| `memory update < stdin` | 写 | 更新 AI 记忆（JSON 从 stdin 读，不需 changelog） |

全局选项：`--compact` 输出紧凑 JSON（减少 token 消耗）

### Rust Agent（运行时 — CLI / Telegram）

位于 `src-tauri/src/agent/`，自建 tool-calling 循环，代理到 `services/`：

- **runner.rs**: LLM ↔ 工具调用主循环（最多 20 轮迭代）
- **tools.rs**: 6 个工具定义，execute 代理到 services 层
- **config.rs**: 分层配置（默认值 → 用户级 → 项目级 → 环境变量）
- **session.rs**: JSONL 持久化会话（最多 40 条消息）
- **channels/telegram.rs**: Telegram bot 适配器

### Claude Code Skills（开发时）

Skills 通过 `arcana-data` CLI 操作数据，不直接读写 JSON 文件：

| Skill | 用途 | 触发方式 |
|-------|------|----------|
| `velvet-room` | 通用进度汇报：理解意图 → 调 CLI 更新数据 | `/velvet-room` |
| `phan-site` | 任务提案生成：基于上下文生成 3-5 个任务 | `/phan-site` |
| `pack-manager` | 成就包管理：创建 / 优化 / 扩展 achievement pack（直接读写 pack 文件） | `/pack-manager` |

### Agent 行为规范

1. **始终先调 `arcana-data context`** 获取当前状态
2. 执行数据变更后 **必须写 changelog**（含 `old_value`）
3. 每次会话结束 **必须更新 `mission_memory.json`**
4. `phan-site` 生成的任务标题必须是游戏化任务名（如 "Borrow Checker Gauntlet"），不是字面描述
5. 回复简洁，默认中文，以 "变更摘要:" 开头

## 数据校验

校验分两层，共享同一套规则：

### Rust 共享校验层 (`storage/validate.rs`)

位于 `storage/` 下，被 agent tools、arcana-data CLI、Tauri commands 共用。写入后自动校验，失败则回滚文件并返回错误。覆盖 agent 写的 4 类文件：

- missions.json: version、ID 唯一性、status 枚举、progress 0-100、main_menu 引用
- achievement_progress.json: version、status ∈ {tracked, achieved}
- ai_changelog.json: version、entries 上限 200、skill ∈ {velvet-room, phan-site, agent}、change type、update 必须有 old_value
- status.json: version、metrics 值必须为数字
- mission_memory.json: version、conversation_context ≤ 20、completed_mission_log ≤ 50

### Python PostToolUse Hook (`scripts/validate_data.py`)

Claude Code 每次写 `data/*.json` 后自动运行。覆盖 Rust 校验的全部规则，额外包含：

- 成就包: ID 前缀、DAG、技能树单调性
- Changelog 新鲜度检查（非阻塞警告）

## 关键设计决策

- **Tauri + JSON 而非 Electron + SQLite**: 更小体积、更好性能、数据可读可版本控制
- **Content Pack 体系**: 成就和技能通过 pack 插拔，支持社区扩展
- **Agent 与 UI 解耦**: Agent 可独立于桌面 GUI 运行（CLI / Telegram），共享数据层
- **DAG 技能树**: 前端从 `prerequisites` 自动推导边和布局，不存冗余数据
- **arcana-data CLI 统一数据入口**: `services/` 层被 agent、arcana-data CLI、Tauri commands 共用。Claude Code Skills 通过 CLI 操作数据，保留 Rust 校验和原子写入
