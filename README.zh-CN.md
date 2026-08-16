# Arcana

[English](README.md) | 简体中文

一个 AI 辅助的 Persona 5 风格游戏化人生管理 HUD。

> [!IMPORTANT]
> Arcana 最适合配合 AI 使用：内置 AI 助手可以理解你的自然语言更新、提出任务、追踪进度，并保持本地 JSON 数据的一致性。为了获得预期的视觉效果，请从合法来源自行安装所需字体；字体文件不会随本仓库或发行包一起分发。详见[字体要求](#字体要求)。

---

## 项目概览

Arcana 是一个 AI 辅助的桌面 HUD，用来把现实生活中的进展整理成结构化的游戏式系统：状态维度、任务、成就、技能、物品库存和媒体历史。它将数据以本地 JSON 文件保存，并通过 AI 助手理解更新、提出任务、追踪进度，让整套系统长期保持连贯。

> [!NOTE]
> 上述内容描述当前实现。已经确定的下一阶段架构会把运行时数据迁入 SQLite，同时保留确定性、可读的 JSON，通过个人私有 Git 仓库同步。参见[目标数据平台设计](docs/design/README.md)。

Arcana **不是**一个靠连续打卡和复选框驱动的习惯追踪器，也不是玩具式的数值表。它借用了游戏的视觉语言和动机循环，但底层数据都是真实的：个人里程碑、当前目标、拥有的物品、消费过的媒体，以及可衡量的状态信号。目标不是假装生活是一场游戏，而是给现实生活一个更清晰、更锋利的界面。

---

## 截图

| 主菜单 |
|--------|
| ![Arcana 主菜单](docs/screenshots/main-menu.jpg) |

<table>
  <tr>
    <th width="50%">状态</th>
    <th width="50%">任务</th>
  </tr>
  <tr>
    <td><img src="docs/screenshots/status.jpg" alt="Arcana 状态界面" width="100%"></td>
    <td><img src="docs/screenshots/missions.jpg" alt="Arcana 任务界面" width="100%"></td>
  </tr>
  <tr>
    <th width="50%">成就</th>
    <th width="50%">技能</th>
  </tr>
  <tr>
    <td><img src="docs/screenshots/achievements.jpg" alt="Arcana 成就界面" width="100%"></td>
    <td><img src="docs/screenshots/skills.jpg" alt="Arcana 技能界面" width="100%"></td>
  </tr>
  <tr>
    <th width="50%">物品</th>
    <th width="50%">图鉴</th>
  </tr>
  <tr>
    <td><img src="docs/screenshots/items.jpg" alt="Arcana 物品界面" width="100%"></td>
    <td><img src="docs/screenshots/gallery.jpg" alt="Arcana 图鉴界面" width="100%"></td>
  </tr>
</table>

---

## 功能

### 状态

由真实指标计算出的多维人生雷达图。

- 状态系统采用三层模型：原始指标（**metrics**）、评分后的维度（**dimensions**），以及 Persona 风格的等级称号（**level titles**）。
- 维度分数由指标贡献、权重、目标值、区间或评分档位计算得出，不是手动填写。
- 系统指标以 `sys_` 为前缀，会从其他模块自动派生，例如图鉴数量、技能等级、成就统计、BMI 和游戏天数。
- 雷达图提供总览，也可以进入每个维度查看背后的指标贡献。

### 成就

支持内容包的里程碑追踪系统。

- 记录人生里程碑，包含解锁时间和难度等级（`beginner` 到 `legendary`）。
- 成就可以拥有前置条件，并形成经过校验的 DAG 依赖图。
- 内容包可以加载面向不同兴趣、领域和生活方向的成就集合。
- 支持内容包导航、难度筛选、解锁排序，以及 locked/unlocked 视觉状态。
- AI 助手可以追踪部分进度、追加进度备注，并标记成就完成。

### 技能

与成就紧密绑定的蜂窝状技能进度系统。

- 每个技能节点都映射到一个成就；解锁成就会点亮对应节点。
- 技能等级仅由状态为 achieved 的节点贡献积分后计算。
- 支持技能总览和蜂窝节点图，可以查看成就详情、前置条件状态和进度历史。
- 技能和成就一起由内容包加载，因此新的内容包可以同时扩展里程碑和技能成长线。

### 任务

面向当前目标和下一步行动的 AI 任务系统。

- AI 提议在用户接受前只是保留于本机的 MissionSuggestion；接受后才成为可同步的 Mission。
- 生命周期：pending/rejected Suggestion；接受后的 Mission → `active` → `completed` / `archived`。
- 支持由 AI 维护的 0-100 进度、截止日期和完成时间。
- 主菜单可展示倒计时、进度提示和轮换任务提示。
- Mission 完成可以成为后续判断 Achievement 的上下文，但不保存静态跨系统链接。

### 物品

带有时间成本意识的个人库存。

- 追踪衣物、鞋子、电子产品、家具、书籍、收藏品和其他物品。
- 从本地物品文件中读取购买日期、价格、购买渠道、分类、图片和备注。
- 可以按名称、持有天数、购买价格和日均成本排序比较。
- 分类汇总和物品详情视图让“拥有”变成更清晰、更可反思的数据表面。

### 图鉴

聚合媒体消费和游玩历史的图鉴中心。

- 统一展示动画、游戏、剧集、电影和书籍。
- 瀑布流封面墙，支持分类筛选、评分/日期/游玩时长排序和详情视图。
- 在数据可用时记录社区评分、个人评分、标签、日期、集数、游玩时长和 Steam 成就元数据。
- 外部数据导入脚本：
  - Bangumi（动画）
  - Steam（游戏）
  - Douban（电影/剧集/书籍）

---

## AI 助手

Arcana 内置 AI 助手，可作为个人生活助手运行，支持多种入口：

| 入口 | 说明 |
|------|------|
| **外部 AI harness** | 本地 Arcana plugin 已提供基于 SQLite CLI 的 Velvet Room、Phan Site 与 Pack Manager canonical Skills。 |
| **Telegram** | 可选的移动端 / 远程访问机器人（`agent-telegram`），按需编译和运行。 |
| **Data CLI** | 脚本和未来 Agent Skill 使用的机器可读 SQLite 数据工具（`arcana-data`）。 |

Status 页面已经使用新的 Application/Repository/SQLite 栈，包括五个本机 Dimension 展示槽位。其余桌面页面和内置 Rust Agent 在迁移期间仍使用旧 JSON services。

> `agent-cli` 是一个最简调试工具，用于在不启动 Tauri 的情况下测试 agent 循环，日常使用不需要编译它。

AI 助手可以：

- 读取当前状态、任务、成就和记忆上下文
- 更新任务进度和状态
- 追踪并标记成就
- 根据你的目标提出新任务
- 维护跨会话记忆，保持连续性

---

## 技术栈

- **框架**：[Tauri v2](https://v2.tauri.app/)（Rust 后端 + webview 前端）
- **前端**：Svelte 5 + SvelteKit v2 + TypeScript + Tailwind CSS v4 + Three.js
- **后端**：Rust（IPC commands、AI 助手、旧 JSON services 与新的 SQLite Repository）
- **数据**：Status 与 `arcana-data` 使用本地 SQLite；其余迁移中的 UI/Agent 仍使用本地 JSON；确定性 JSON 是后续 Git 同步格式
- **AI**：直接集成 Anthropic API，并自建工具调用循环

---

## 项目结构

```text
src/                    # SvelteKit 前端
  ├── routes/           #   单页应用（主菜单 + 子界面）
  └── lib/
      ├── screens/      #   页面组件（Status, Achievements, Skills, Items, Gallery, Missions）
      ├── components/   #   共享 UI 组件（RadarChart, SkillNebula 等）
      ├── types/        #   TypeScript 类型定义
      └── utils/        #   前端工具函数
src-tauri/src/          # Rust 后端
  ├── commands/         #   Tauri IPC commands（status, achievements, skills, missions, items, gallery, weather）
  ├── models/           #   Serde 数据结构
  ├── domain/           #   新数据平台领域模型
  ├── application/      #   新 typed commands 与运行时边界
  ├── storage/          #   SQLite/JSON Codec 和旧 JSON 存储
  ├── services/         #   UI 与内置 Agent 使用的旧业务逻辑
  ├── agent/            #   AI 助手子系统（runner, LLM, tools, prompt, config, session）
  └── bin/              #   独立二进制：agent_cli, agent_telegram, arcana_data
data/                   # 忽略的本地开发数据
data-example/           # 为尚未迁移的 UI 暂时保留的旧 JSON 模板
  ├── packs/<pack_id>/  #   内容包（manifest.json, achievements.json, skills.json）
  └── *.json            #   missions、status、achievement_progress 等
docs/                   # 架构文档、schema 规范、UI 设计指南
  └── schema/           #   JSON schema 定义
scripts/                # Python 工具脚本（数据导入、schema 校验）
static/                 # 静态资源（图标、图片）
```

---

## 快速开始

```bash
# 1. 安装前端依赖
npm install

# 2. 启动当前桌面应用
npm run tauri dev
```

桌面 UI 迁移期间，可以单独验证 SQLite 数据平台：

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
./src-tauri/target/debug/arcana-data capabilities
./src-tauri/target/debug/arcana-data init
./src-tauri/target/debug/arcana-data pack list
./src-tauri/target/debug/arcana-data status list-dimensions
./src-tauri/target/debug/arcana-data achievement list
./src-tauri/target/debug/arcana-data skill list
./src-tauri/target/debug/arcana-data mission list
./src-tauri/target/debug/arcana-data memory list
```

`arcana-data init` 只创建 SQLite runtime 和 `basic` Pack，不会填充新手任务。Record、Pack、Status、Achievement、Arcana Skill 查询、Mission、AssistantMemory、紧凑 Agent 上下文、dry-run、原子用户状态 batch、contract fixtures 与 canonical 外部 Skills 已经迁移；Tauri Status 页面也已使用该运行时，其余桌面页面仍在迁移。

> [!NOTE]
> 如果你需要使用 agent 二进制——主要是 `agent-telegram`，它会启动一个监听服务，让你通过 Telegram 远程控制本地助手——则需要额外配置 LLM provider。通过环境变量（`ANTHROPIC_API_KEY`）或配置文件（`~/.arcana/agent_config.json`）设置 API key 即可。详见 [AI 助手](#ai-助手)。

---

## 开始使用

### 前置要求

- **Rust**：stable toolchain
- **Node.js**：v18+
- **平台**：Windows / macOS / Linux

### 字体要求

Arcana 的视觉风格依赖少量系统字体。这些字体文件**不会随本仓库或发行包一起分发**；用户需要自行安装，才能获得预期的 Persona 5-inspired 视觉效果：

- `p5hatty`：菜单、标签、卡片和拼贴文字的主要展示字体
- `Source Han Sans SC`：中文 UI 和卡片标题字体
- `Bebas Neue`：按键提示徽标字体

如果缺少这些字体，应用仍然可以运行，但 UI 会回退到 `Arial`、`Microsoft YaHei` 或通用 `sans-serif` 等系统字体，部分标题和卡片布局可能看起来不同。

### 显示缩放说明

当前 UI 主要在 Windows、4K 分辨率、100% 显示缩放环境下开发。除此之外，也在 Windows 4K 125% 缩放、Windows 2K 100% 缩放，以及 MacBook Air 13 英寸约 1710x1112 的缩放桌面环境下做过简单适配。

其他分辨率、显示缩放设置，以及 macOS/Retina 缩放模式下仍可能存在布局问题。后续需要用更统一的跨分辨率布局方案继续整理。

### 开发

```bash
# 安装前端依赖
npm install

# 运行完整桌面应用开发模式
npm run tauri dev

# 或只运行前端开发服务器
npm run dev
```

### 构建

```bash
# 构建桌面发行包
npm run tauri build

# 构建 SQLite 数据工具
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data

# 按需构建 agent 二进制
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-telegram  # Telegram 机器人，需要时编译
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-cli       # 调试工具，日常不需要
```

### 检查

```bash
# TypeScript / Svelte 类型检查
npm run check

# Rust 测试
cargo test --manifest-path src-tauri/Cargo.toml

# Rust 格式检查
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

---

## 工具脚本

Arcana 提供了一组 Python 脚本，用于导入个人数据、生成内容包、处理 UI 资源，以及校验本地 JSON 文件。

部分数据导入脚本会从 `scripts/config.json` 读取凭据或用户 ID。可以使用 `scripts/config.example.json` 作为模板，并将真实值保留在本地。

| 脚本 | 用途 |
|------|------|
| `scripts/fetch_bangumi.py` | 从 Bangumi 获取已看动画，并写入图鉴数据。 |
| `scripts/fetch_steam.py` | 获取 Steam 游戏库；`--detailed` 还会抓取成就和商店元数据。 |
| `scripts/fetch_douban.py` | 获取 Douban 电影、剧集和书籍；支持 `--status all`。 |
| `scripts/dev/process_assets.py` | 调整并准备 `static/ui/` 下的 UI 资源。 |
| `scripts/dev/remove_bg.py` | 为单个图片或文件夹批量移除背景。 |
| `scripts/validate_data.py` | 仅用于仓库本地 `data/` JSON 的旧编辑后 hook；正常 CLI/Tauri 写入使用 Rust 校验。 |

```bash
python scripts/fetch_bangumi.py
python scripts/fetch_steam.py --detailed
python scripts/fetch_douban.py --status all
```

---

## 文档

- [Architecture](docs/architecture.md)：Tauri、数据层、前端和 AI 助手架构。
- [目标数据平台设计](docs/design/README.md)：已经定稿的 SQLite 运行时、Git JSON 同步、RecordDefinition/Record、Status、Achievement、PackForest、Mission 与 Memory 架构。
- [Schema Reference](docs/schema/README.md)：missions、achievements、skills、status、items、changelog、memory 和 UI events 的详细 JSON schema。
- [Visual Style Guide](docs/visual_style_guide.md)：Persona 5 风格设计原则、调色板、字体和交互规则。
- [UI Design Spec](docs/ui_design_spec.md)：主菜单和子界面的布局/交互规范。

---

## 当前实现说明

- **当前处于分层迁移状态**：Tauri Status 页面与 `arcana-data` 已使用 SQLite；其余 Tauri 页面与内置 Agent 仍使用旧 JSON。
- **内容包体系**：成就和技能通过用户可扩展的内容包加载。
- **Agent 仍处于分层迁移**：内置 CLI/Telegram Agent 仍使用旧 JSON；canonical 外部 Skills 已位于 `plugins/arcana/skills`，并提供生成式 `.claude/skills` 镜像、版本化 contract fixtures 与固定 eval 场景。
- **前置条件校验**：当前 Achievement 模型把 prerequisites 校验为 DAG；Skill 在 UI 中呈现为紧凑的蜂窝状节点图。
- **明确的迁移边界**：`services/` 只属于旧 UI/Agent；Status IPC 与 SQLite CLI 使用 `application/`、`domain/` 和 `storage/sqlite/`。

---

## 致谢

- [Mive82/Persona-5-Calendar](https://github.com/Mive82/Persona-5-Calendar)：calendar 组件参考
- [sjpiper145/MakerSkillTree](https://github.com/sjpiper145/MakerSkillTree)：网格式技能树布局灵感
- [NERvGear/SAO-Utils](https://github.com/NERvGear/SAO-Utils)：游戏风格桌面应用灵感
- [aliubo/persona-text-gen](https://github.com/aliubo/persona-text-gen)：拼贴式 calling card 文字生成参考

---

## License

MIT
