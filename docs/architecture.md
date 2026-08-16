# Arcana 架构设计文档

> **版本**: v0.1.0
> **最后更新**: 2026-08-16
> **状态**: Current implementation

> [!IMPORTANT]
> 本文同时描述迁移中的两条边界。Status、Achievement、Skill、Mission 页面和 `arcana-data` 已切到 SQLite/Record 新模型；Items、Gallery 与内置 Rust Agent 仍在旧 JSON/外部数据边界。新模型与当前 CLI 合约见 [`docs/design/`](./design/README.md)。

Arcana 是一个 Persona 5 风格的游戏化人生管理桌面应用，也就是给 “Earth Online” 加一层用户界面。当前实现已经从早期的 Status MVP 演进为一个本地优先的桌面 HUD：前端负责高表现力的菜单与模块屏幕，Rust 后端同时维护新的 SQLite 数据平台和待迁移的旧 JSON/Agent 边界。

---

## 1. 架构概览

Arcana 采用 **Local-First + Tauri Shell + Shared Services** 架构。

核心原则：

- **本地优先**：新运行时位于 `~/.arcana/runtime` 的 SQLite 数据库；尚未迁移的页面和内置 Agent 仍读取旧 `~/.arcana/data` JSON。仓库中的 `data-example/` 只为旧实现保留。
- **旧共享业务层**：尚未迁移的 Tauri IPC 与独立 Rust agent 复用 `src-tauri/src/services/`；新的 `arcana-data` 只通过 Application Commands 与 Repository 访问 SQLite。
- **数据驱动 UI**：Status、Achievement、Skill 从 SQLite 中的 Record、Pack Definition 与用户状态渲染；Mission 从同步 Mission、本机 Suggestion 与 Dashboard slot 渲染；Items、Gallery 暂时保留旧/外部数据源。
- **旧 Agent 审计**：尚未迁移的内置 Agent 写 missions/status/achievement progress 后仍写 `ai_changelog.json`；新数据平台不保留 changelog。
- **Persona 5 风格表达层**：视觉风格集中在 Svelte 组件、全局 CSS、静态资源与设计文档中，后端保持数据和规则纯净。

### 1.1 当前系统图

```mermaid
flowchart TB
    subgraph Frontend["Svelte 5 / SvelteKit SPA"]
        Main["src/routes/+page.svelte\nMain menu + screen router"]
        Screens["src/lib/screens/*\nStatus / Missions / Achievements / Skills / Items / Gallery"]
        Components["src/lib/components/*\nRadarChart / SkillNebula / common UI"]
        Types["src/lib/types/*\nFrontend data contracts"]
        Main --> Screens
        Screens --> Components
        Screens --> Types
    end

    subgraph Backend["Rust / Tauri v2"]
        TauriCommands["commands/*\nTauri IPC boundary"]
        Services["services/*\nshared business/data operations"]
        Storage["storage/*\nJSON IO, settings, validation, date utils"]
        Models["models/*\nSerde data models"]
        Agent["agent/*\nLLM runner, tools, prompt, session, channels"]
        AgentBins["bin/agent-*\nlegacy agent entry points"]
        DataCli["bin/arcana_data/*\nSQLite data CLI"]
        Application["application/*\ntyped commands + runtime"]
        Repository["domain + storage/sqlite\nvalidation + repository"]
    end

    subgraph Data["Local runtime data"]
        DataFiles["<data_dir>/*.json\nmissions, status, progress, changelog, memory"]
        Packs["<data_dir>/packs/<pack_id>/\nmanifest, achievements, skills"]
        Sessions["<data_dir>/sessions/\nagent JSONL history"]
        RuntimeDb["<runtime_dir>/arcana.sqlite3\nshared UI / CLI runtime"]
    end

    Frontend -->|"invoke(...)"| TauriCommands
    TauriCommands --> Services
    TauriCommands --> Storage
    TauriCommands --> Application
    Services --> Storage
    Services --> Models
    Storage --> DataFiles
    Storage --> Packs
    Agent --> Services
    AgentBins --> Agent
    DataCli --> Application
    Application --> Repository
    Repository --> RuntimeDb
    Agent --> Sessions
```

---

## 2. 技术栈

| 层 | 技术 | 当前用途 |
| --- | --- | --- |
| 桌面壳 | Tauri v2 | 原生窗口、全局快捷键、IPC command、图片代理协议 |
| 后端 | Rust 2021 | Domain/Application、SQLite/JSON IO、校验、AI agent、CLI |
| 前端 | Svelte 5 + SvelteKit v2 + TypeScript | 单页 HUD、菜单、模块屏幕、交互状态 |
| 样式 | Tailwind CSS v4 + 全局 CSS | P5 风格几何 UI、动画、响应式布局 |
| 3D/可视化 | Three.js, Canvas/SVG/CSS | SkillNebula、雷达图与动态视觉组件 |
| AI | Anthropic API via Rust agent | tool-calling loop、CLI/Telegram 运行模式 |
| 数据 | SQLite + 旧本地 JSON | 已迁移领域使用 `~/.arcana/runtime/arcana.sqlite3`；待迁移页面与旧 Agent 仍使用 `~/.arcana/data` |
| 工具 | Python scripts | 数据导入、schema/数据校验 |

当前 `package.json` 中没有 D3、vis.js、Chart.js；技能和图表渲染由项目内 Svelte 组件实现。结构化 AI 数据入口是 `arcana-data` CLI 和 Rust agent tools。

---

## 3. 分层架构

### 3.1 Frontend Presentation

位置：

- `src/routes/+page.svelte`
- `src/lib/screens/`
- `src/lib/components/`
- `src/lib/types/`
- `src/lib/utils/`

职责：

- 渲染主菜单与六个主屏幕：Status、Skills、Achievements、Items、Gallery、Missions。
- 通过 `@tauri-apps/api/core` 的 `invoke` 调用后端 commands。
- 维护屏幕切换、键盘/窗口事件、模块内排序筛选与展示状态。
- 按 `docs/visual_style_guide.md` 和 `docs/ui_design_spec.md` 实现 P5 风格 UI。

当前前端更接近单页应用：`src/routes/` 只有根 layout/page，模块屏幕在 `src/lib/screens/` 中切换，而不是每个模块一个 SvelteKit route。

### 3.2 Tauri Command Boundary

位置：`src-tauri/src/commands/`

当前 command 模块：

| 模块 | 主要职责 |
| --- | --- |
| `data_platform.rs` | 从 SQLite Application 层加载 Status、Achievement、Skill dashboard，修改 Achievement 状态，读取受控 Pack 图片，并管理五个本机 Dimension 选择 |
| `items.rs` | 加载物品来源和物品列表 |
| `gallery.rs` | 加载媒体图鉴来源与条目 |
| `missions.rs` | 从 SQLite 加载 Mission/Suggestion 与主菜单投影，执行接受、拒绝、完成、归档和本机 Dashboard slot 配置 |
| `weather.rs` | 读取天气数据 |

`src-tauri/src/lib.rs` 注册这些 commands，同时配置：

- 无边框窗口和伪全屏行为
- 全局快捷键召唤/隐藏窗口
- `imgproxy://` 自定义协议，用于代理和缓存远程媒体图片
- `tauri-plugin-opener` 与 `tauri-plugin-global-shortcut`

### 3.3 Shared Services

位置：`src-tauri/src/services/`

`services/` 是旧 JSON Agent 的共享边界。Items、Gallery 命令仍直接使用旧 `models/` 与 `storage/`；Rust agent 复用 `services/`。Status、Achievement、Skill、Mission IPC 与 `arcana-data` 已退出该层，改用 `application/`、`domain/` 和 `storage/sqlite/`。新增数据平台功能不得再接入旧 services。

| 模块 | 职责 |
| --- | --- |
| `context.rs` | 汇总 missions/status/metric definitions/achievement progress/memory 给 AI |
| `file_access.rs` | 沙箱读取 `<data_dir>` 下文件 |
| `mission.rs` | 仅供旧内置 Agent 更新/创建旧 mission 和 main_menu 配置；桌面 Mission 页面不再使用 |
| `status.rs` | 仅供旧内置 Agent 更新旧 status metric values；桌面 Status 页面不再使用 |
| `achievement.rs` | 仅供旧内置 Agent 更新旧 achievement progress，追加 progress detail |
| `changelog.rs` | 写 `ai_changelog.json`，限制 200 条 |
| `memory.rs` | 更新 `mission_memory.json` |
| `ui_events.rs` | 仅供旧内置 Agent 写入/读取旧 UI event 队列；当前前端不再消费 |

设计约束：

- 写数据优先走 typed model + service，而不是在调用方直接改 JSON。
- 写入后调用共享 validator，失败则回滚。
- AI 写入除 `mission_memory.json` 外，都要伴随 changelog。

### 3.4 Storage & Validation

位置：`src-tauri/src/storage/`

| 模块 | 职责 |
| --- | --- |
| `json_store.rs` | JSON read/write、`write_and_validate`、data dir resolution |
| `validate.rs` | Rust 侧纯校验逻辑，无 I/O |
| `settings.rs` | `~/.arcana/settings.json` 与路径展开 |
| `date_utils.rs` | 日期解析、天数计算 |

Data dir resolution 优先级：

1. `ARCANA_DATA_DIR`
2. `~/.arcana/settings.json` 的 `data_dir`
3. 默认 `~/.arcana/data`，不存在则创建

开发仓库中的 `data-example/` 用于初始化和示例；被忽略的 `data/` 仅供本地开发使用。

### 3.5 AI Agent & Data CLI

位置：

- `src-tauri/src/agent/`
- `src-tauri/src/bin/agent_cli.rs`
- `src-tauri/src/bin/agent_telegram.rs`
- `src-tauri/src/bin/arcana_data.rs`

当前 AI 相关入口：

| 入口 | 用途 |
| --- | --- |
| `agent-cli` | 终端运行的对话 agent |
| `agent-telegram` | Telegram bot 适配器 |
| `arcana-data` | 新 SQLite 数据平台的机器可读 CLI；当前支持 capabilities、init、context summary、用户状态 dry-run/atomic batch、Record、Pack、Status、Achievement、Arcana Skill、Mission、AssistantMemory 和 JSON 转换 |

Agent 子系统：

| 模块 | 职责 |
| --- | --- |
| `runner.rs` | LLM tool-calling 主循环 |
| `llm.rs` | Anthropic API 请求/响应 |
| `tools.rs` | 工具注册与执行，代理到 `services/` |
| `prompt.rs` | 系统提示词 |
| `config.rs` | 默认值、用户级、项目级、环境变量配置 |
| `session.rs` | JSONL 会话历史 |
| `bus.rs` | agent 内部消息/事件总线 |
| `channels/` | Telegram 等外部通道 |

Agent 当前工具集：

- `get_context`
- `read_file`
- `update_mission`
- `update_status`
- `update_achievement`
- `write_changelog`

`arcana-data` 不再代理上述旧 Agent tools，也不读写 `<data_dir>` JSON。当前提供 `capabilities`、SQLite `init`、单事务 `context summary`、用户状态 mutation 的 `--dry-run` 与 `batch apply`、完整 Record/Pack/Status/Achievement/Mission/AssistantMemory Commands、只读 Arcana Skill 派生查询和不接触 Git 的 `json import|export`；旧 JSON 实现已删除，所有同名命令均按 SQLite 合约重新实现。

---

## 4. 当前目录结构

```text
src/
  routes/
    +layout.svelte
    +layout.ts
    +page.svelte              # SPA 主菜单与 screen router
  lib/
    screens/                  # Status, Achievements, Skills, Items, Gallery, Missions
    components/               # Shared UI components
      common/
      status/
    types/                    # Frontend TS data contracts
    utils/                    # format/card title helpers
    Calendar.svelte
    MenuItem.svelte
    PhanSiteProgress.svelte
    ...

src-tauri/src/
  lib.rs                      # Tauri app setup, commands, imgproxy, global shortcut
  main.rs
  commands/                   # IPC commands
  models/                     # Serde data structures
  storage/                    # JSON IO, settings, validation, date utils
  services/                   # Legacy built-in Agent operations
  agent/                      # AI agent runtime
    channels/
  bin/
    agent_cli.rs
    agent_telegram.rs
    arcana_data.rs

data/                         # ignored local development data

data-example/                 # tracked initialization templates
  packs/<pack_id>/            # Content packs: manifest, achievements, skills
  achievement_progress.json
  gallery_sources.json
  item_sources.json
  loaded_packs.json
  missions.json
  mission_archive.json
  status_metric_definitions.json
  status.json
  user_profile.json
  weather.json

docs/
  architecture.md
  design/                      # 新数据平台权威设计
  screenshots/
  schema/                      # 旧 JSON 迁移对照
  ui_design_spec.md
  visual_style_guide.md

scripts/
  validate_data.py
  ...

static/
  icons/
  images and UI assets
```

---

## 5. 功能模块

| 模块 | 数据文件/来源 | 后端入口 | 前端入口 |
| --- | --- | --- | --- |
| Status | SQLite Record、Pack Dimension、本机五项选择 | `commands/data_platform.rs`, `application/status_commands.rs` | `StatusScreen.svelte`, `StatusDetailView.svelte`, `RadarChart.svelte` |
| Missions | SQLite 同步 Mission、本机 MissionSuggestion 与 Dashboard slot | `commands/missions.rs`, `application/mission_commands.rs`, `application/mission_dashboard_commands.rs` | `MissionsScreen.svelte`, `PhanSiteProgress.svelte` |
| Achievements | SQLite Pack AchievementDefinition 与 AchievementState | `commands/data_platform.rs`, `application/achievement_commands.rs` | `AchievementsScreen.svelte` |
| Skills | SQLite Pack SkillDefinition 与派生 Achievement 状态 | `commands/data_platform.rs`, `application/skill_commands.rs` | `SkillsScreen.svelte` |
| Items | `item_sources.json` + source files | `commands/items.rs` | `ItemsScreen.svelte` |
| Gallery | `gallery_sources.json` + source files, image cache | `commands/gallery.rs`, `imgproxy` protocol | `GalleryScreen.svelte` |
| Legacy UI Events | `ui_events.json` | `services/ui_events.rs`（仅旧 Agent） | 无；已迁移页面在 mutation 后直接刷新 |
| Weather | `weather.json` | `commands/weather.rs` | root page/weather display surfaces |

### 5.1 Status Record/Dimension 模型

Status 页面现在只消费新数据平台：

1. 已启用 Pack 提供 RecordDefinition 和 DimensionDefinition。
2. 用户事实保存为 Record；每个子 Score 使用受限表达式读取数值 scalar Record。
3. Dimension 分数是可用子 Score 的加权平均，固定裁剪到 0～100，再由四个 threshold 推导 Lv.0～Lv.5。
4. 五个展示槽位保存在 SQLite 本机表中，不进入同步 JSON；UI 允许暂时少于五项并明确显示未选择状态。

昵称和生日分别读取普通 `identity.nickname` 与 `identity.birth_date` Record；缺失时显示默认昵称并隐藏游戏天数。

### 5.2 Content Pack System

新数据平台中的 Pack Definition 与 asset 保存在 SQLite；规范 JSON 导入/导出目录使用以下结构：

```text
<data_dir>/packs/<pack_id>/
  manifest.json
  record-definitions.json
  dimensions.json
  achievements.json
  skills.json
  assets/
```

规则：

- achievement ID 使用 `<pack_id>::<snake_case_name>`。
- `manifest.id` 必须等于目录名。
- achievement prerequisites 只引用同包 achievement，并且必须构成 DAG。
- Skill 固定 Lv.0～Lv.5，四个 `level_thresholds` 严格递增；只有 achieved Achievement 节点计分。
- Pack 启用状态保存在 SQLite，并在规范 JSON 中由 `arcana.json.enabled_pack_ids` 表达。

### 5.3 Mission System

Mission 与推荐内容是两个实体：

- 未接受内容是仅本机 `MissionSuggestion`，状态为 pending/rejected；
- 接受 Suggestion 时原子创建同 ID 的 active Mission，并删除 Suggestion；
- 可同步 Mission 生命周期为 active、completed、archived；
- `progress` 可选且为 0～100，complete command 会把已有 progress 设为 100；
- countdown、progress、hint 1、hint 2 是本机 Dashboard slot，不写入同步 Mission；
- deadline 的剩余天数按当前日期即时派生；Mission 不保存静态 Achievement 链接。

---

## 6. 数据流

### 6.1 UI 加载模块数据

```mermaid
sequenceDiagram
    participant UI as Svelte Screen
    participant IPC as Tauri Command
    participant Application as StatusCommands
    participant Data as SQLite Repository

    UI->>IPC: invoke("load_status_dashboard")
    IPC->>Application: list/evaluate dimensions
    Application->>Data: read Pack, Record and local selections
    Data-->>Application: repository snapshot
    Application-->>IPC: typed evaluations
    IPC-->>UI: StatusDashboardData
    UI->>UI: render screen
```

### 6.2 AI 更新数据

```mermaid
sequenceDiagram
    participant User
    participant Agent as agent-cli / agent-telegram
    participant Tools as agent/tools.rs
    participant Services as services/*
    participant Storage as storage/json_store
    participant Data as Local JSON

    User->>Agent: natural language update
    Agent->>Tools: get_context
    Tools->>Services: context::get_context
    Services->>Data: read current data
    Agent->>Tools: update_status / update_mission / update_achievement
    Tools->>Services: shared update function
    Services->>Storage: write_and_validate
    Storage->>Data: write JSON
    Agent->>Tools: write_changelog
    Tools->>Services: changelog::write_changelog
    Services->>Data: append audit entry
```

### 6.3 `arcana-data` CLI 写入

```mermaid
flowchart LR
    Skill["Codex skill / script"] --> CLI["arcana-data command"]
    CLI --> Application["application/* typed command"]
    Application --> Repository["SQLite Repository transaction"]
    Repository --> SQLite["runtime/arcana.sqlite3"]
```

成功结果以退出码 0 和 stdout 业务 JSON 返回；失败结果使用非零退出码和 stderr 结构化 JSON。`capabilities` 不打开 SQLite，可用于检查 CLI contract 与 Schema 版本。

---

## 7. 数据与校验

Schema 文档在 `docs/schema/`：

- `achievements.md`
- `ai_changelog.md`
- `content_packs.md`
- `items.md`
- `mission_memory.md`
- `missions.md`
- `skills.md`
- `status.md`
- `ui_events.md`

校验分两层：

| 层 | 位置 | 覆盖 |
| --- | --- | --- |
| Rust shared validation | `src-tauri/src/storage/validate.rs` | missions、achievement progress、ai changelog、status、mission memory |
| Legacy project hook | `scripts/validate_data.py` | 仅处理仓库内 `data/` 文件；覆盖 loaded packs、Pack 文件和 changelog freshness warning，不处理配置的用户数据目录 |

Rust 写入路径使用 `write_and_validate` 时会在校验失败后恢复旧文件。Python validator 用于 Codex/脚本写入后的快速反馈。

通用数据规则：

- 顶层 JSON 使用 `{"version": 1, ...}`。
- 可选字段尽量省略，不写 `null`。
- 日期为 `YYYY-MM-DD`；时间戳为 ISO 8601。
- `ai_changelog.json` 最多 200 条，FIFO 淘汰。
- `mission_memory.json` 是 AI 内部状态，变更不写 changelog。

---

## 8. 关键设计决策

> [!NOTE]
> 本节解释当前实现为何形成。关于 SQLite、RecordDefinition/Record、Git JSON、Status 新评分和外部 Agent Skill 的后续决定，统一以 [`docs/design/`](./design/README.md) 为准；特别是 8.1 和 8.3 不再代表下一阶段方向。

### 8.1 为什么当前实现使用 JSON

- 用户可读、可备份、可手动修复。
- content packs 天然适合文件夹结构。
- 当前数据规模较小，JSON 读写足够。
- AI 写入需要审计和回滚语义，文件级 changelog 覆盖了当前实现的需求。

这些是现状说明，不是继续沿用 JSON 运行时的目标决策；下一阶段已确定使用 SQLite 运行时与 Git 同步 JSON，见 [`docs/design/data_platform.md`](./design/data_platform.md)。

### 8.2 为什么抽出 `services/`

早期 Tauri commands 直接读写 JSON 已经不够用，当时同一份数据有三类调用方：

- 桌面 UI
- Rust agent
- 旧 `arcana-data` CLI / AI skills

共享 services 仍让旧 UI 和内置 Agent 的校验、changelog、回滚和业务规则集中在一处。新的 `arcana-data` 已迁往 Application/Repository 层；services 只作为待删除的迁移边界，不再扩展。

### 8.3 为什么 Status 使用 Record + Dimension

Status 不是一组独立于用户事实的特殊数值。它需要同时支持：

- Pack 声明的可复用 RecordDefinition；
- 用户通过 Record 写入的事实；
- 一个 Dimension 内多个 0～100 子分数的加权平均；
- 固定 Lv.0～Lv.5 的等级展示；
- 本机选择的五个展示 Dimension。

因此用户事实统一进入 Record，Dimension 只声明如何读取并组合这些事实；分数和等级即时派生，不写回 SQLite。

### 8.4 为什么 Skills 绑定 Achievements

Skill 节点映射 achievement，避免用户维护两份进度。完成 milestone 后：

1. AchievementState 更新为 `achieved`。
2. skill node 自动点亮。
3. skill points 与 Lv.0～Lv.5 根据已完成节点即时计算。

`tracked` 只代表用户正在关注该 Achievement，不点亮节点、不计分。prerequisites 影响可用性和推荐，但不阻止用户显式确认已经完成。

---

## 9. 开发与验证

常用命令：

```bash
npm install
npm run dev
npm run tauri dev
npm run build
npm run tauri build
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-cli
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-telegram
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
```

PR 前最低验证：

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

开发约定：

- TypeScript/Svelte：2 空格缩进，组件/类型 `PascalCase`，变量/函数 `camelCase`。
- Rust：模块/函数 `snake_case`，结构体/枚举 `PascalCase`。
- Tauri command 错误信息应可操作。
- Commit 使用 Conventional Commits，例如 `docs: update architecture document`。

---

## 10. 相关文档

- [README](../README.md)
- [下一阶段目标架构](./design/README.md)
- [视觉风格指南](./visual_style_guide.md)
- [UI 设计规范](./ui_design_spec.md)
- [Schema 目录](./schema/README.md)
