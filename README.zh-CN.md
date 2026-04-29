# Arcana

[English](README.md) | 简体中文

一个 AI 辅助的 Persona 5 风格游戏化人生管理 HUD。

> [!IMPORTANT]
> Arcana 最适合配合 AI 使用：内置 AI 助手可以理解你的自然语言更新、提出任务、追踪进度，并保持本地 JSON 数据的一致性。为了获得预期的视觉效果，请从合法来源自行安装所需字体；字体文件不会随本仓库或发行包一起分发。详见[字体要求](#字体要求)。

---

## 项目概览

Arcana 是一个 AI 辅助的桌面 HUD，用来把现实生活中的进展整理成结构化的游戏式系统：状态维度、任务、成就、技能、物品库存和媒体历史。它将数据以本地 JSON 文件保存，并通过 AI 助手理解更新、提出任务、追踪进度，让整套系统长期保持连贯。

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
- 技能等级由已解锁节点的点数和必要关键成就共同计算。
- 支持技能总览和蜂窝节点图，可以查看成就详情、前置条件状态和进度历史。
- 技能和成就一起由内容包加载，因此新的内容包可以同时扩展里程碑和技能成长线。

### 任务

面向当前目标和下一步行动的 AI 任务系统。

- 任务由 AI 助手根据当前目标和上下文提出，风格类似 Persona 5 的 “Phan-Site” 请求。
- 生命周期：`proposed` → `active` → `completed` / `archived` / `rejected`。
- 支持由 AI 维护的 0-100 进度、截止日期和完成时间。
- 主菜单可展示倒计时、进度提示和轮换任务提示。
- 任务可以关联到成就，形成跨系统进展。

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

Arcana 内置 AI 助手，可作为个人生活助手运行，目前支持三种入口：

| 入口 | 说明 |
|------|------|
| **CLI** | 独立终端助手（`agent-cli`） |
| **Telegram** | 面向移动端访问的机器人适配器（`agent-telegram`） |
| **Data CLI** | 面向 AI 技能的结构化数据操作工具（`arcana-data`） |

这三种入口共享同一套服务层（`src-tauri/src/services/`）和数据格式，因此任何入口写入的更新都会在其他地方立即可见。

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
- **后端**：Rust（IPC commands、AI 助手、JSON 数据层）
- **数据**：本地 JSON 文件（`data/`，gitignored），无数据库
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
      ├── stores/       #   Svelte stores
      └── utils/        #   前端工具函数
src-tauri/src/          # Rust 后端
  ├── commands/         #   Tauri IPC commands（status, achievements, skills, missions, items, gallery, weather）
  ├── models/           #   Serde 数据结构
  ├── storage/          #   JSON 读写与校验
  ├── services/         #   共享业务逻辑（AI 助手、arcana-data CLI、Tauri commands 共用）
  ├── agent/            #   AI 助手子系统（runner, LLM, tools, prompt, config, session）
  └── bin/              #   独立二进制：agent_cli, agent_telegram, arcana_data
data/                   # 运行时 JSON 数据（gitignored）
  ├── packs/<pack_id>/  #   内容包（manifest.json, achievements.json, skills.json）
  ├── sessions/         #   AI 助手 JSONL 会话历史
  └── *.json            #   missions, status, achievement_progress, mission_memory 等
docs/                   # 架构文档、schema 规范、UI 设计指南
  └── schema/           #   JSON schema 定义
scripts/                # Python 工具脚本（数据导入、schema 校验）
static/                 # 静态资源（图标、图片）
```

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

# 构建独立 AI 助手二进制
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-cli
cargo build --manifest-path src-tauri/Cargo.toml --bin agent-telegram
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
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
| `scripts/extract_maker_trees.py` | 将 MakerSkillTree 源数据提取成中间 JSON。 |
| `scripts/generate_packs.py` | 从解码后的技能树数据生成 Arcana 成就包和技能包。 |
| `scripts/process_assets.py` | 调整并准备 `static/ui/` 下的 UI 资源。 |
| `scripts/remove_bg.py` | 为单个图片或文件夹批量移除背景。 |
| `scripts/validate_data.py` | 校验运行时 JSON 数据和内容包 schema 规则。 |

```bash
python scripts/fetch_bangumi.py
python scripts/fetch_steam.py --detailed
python scripts/fetch_douban.py --status all
python scripts/validate_data.py data/missions.json
```

---

## 文档

- [Architecture](docs/architecture.md)：Tauri、数据层、前端和 AI 助手架构。
- [Directory Structure](docs/directory_structure.md)：项目布局和历史结构说明。
- [Schema Reference](docs/schema/README.md)：missions、achievements、skills、status、items、gallery、changelog、memory 和 UI events 的详细 JSON schema。
- [Visual Style Guide](docs/visual_style_guide.md)：Persona 5 风格设计原则、调色板、字体和交互规则。
- [UI Design Spec](docs/ui_design_spec.md)：主菜单和子界面的布局/交互规范。
- [AI 助手集成方案](docs/ai_agent_integration.md)：MCP/Nanobot 集成方案和 AI 助手平台调研记录。

---

## 设计决策

- **Tauri + JSON，而非 Electron + SQLite**：更小体积、更好性能，同时保留可读、可版本控制的数据文件。
- **内容包体系**：成就和技能通过可插拔内容包加载，支持社区扩展。
- **AI 助手与 UI 解耦**：AI 助手可以独立于桌面 GUI 运行（CLI / Telegram），并共享同一套数据层。
- **前置条件驱动的进度系统**：成就前置条件在数据模型中保持为经过校验的 DAG，而技能在 UI 中呈现为紧凑的蜂窝状节点图，不是传统连线图。
- **共享服务层**：`services/` 集中任务、状态、成就、记忆和 changelog 等业务逻辑，供 Tauri commands、`arcana-data` 和 Rust AI 助手共同使用。

---

## v0.1 路线图

- [ ] 提供示例数据配置
- [x] 打磨主菜单，包括倒计时和进度条组件
- [x] 打磨技能界面
- [x] 打磨成就界面
- [x] 打磨物品界面
- [x] 打磨图鉴界面
- [x] 打磨任务界面
- [ ] 测试 AI 技能相关功能

---

## 未来想法

### UI 与体验

- 首次设置向导
- 全局界面音效
- 数据变更揭示动画：首次打开时展示上次会话以来发生的变化
- 接受任务和完成任务时的电影感动画

### 功能

- 技能塔罗牌生成器：为每个追踪中的技能自动生成 Persona 风格卡牌，可以考虑接入生成模型
- 图鉴增加音乐追踪，与书籍、动画、电影、游戏并列
- AI 导航伙伴：一个常驻屏幕的助手，灵感来自 P5 的 Futaba / Morgana（默认形象参考《Steins;Gate》的 Kurisu）

### 审计与透明度

- 面向用户的 changelog 查看器：在 UI 中展示 `ai_changelog.json`，让用户审查、确认和回滚 AI 驱动的数据变更
- AI 修改的 diff 视图和一键回滚

### 集成与平台

- 支持更多 IM 渠道（如 Discord、WeChat）以及 Anthropic 之外的 LLM 服务商
- 为图鉴和状态系统添加更多数据源导入器
- 与外部 AI 知识管理系统做更深集成
- 移动端只读看板：用于快速查看状态雷达图和任务进度的轻量 web view
- 健康数据自动导入：从 Apple Health / Google Fit / Garmin 同步，保持状态指标更新
- 社区内容包仓库：允许其他人发布和分享成就包

---

## 致谢

- [Mive82/Persona-5-Calendar](https://github.com/Mive82/Persona-5-Calendar)：calendar 组件参考
- [sjpiper145/MakerSkillTree](https://github.com/sjpiper145/MakerSkillTree)：网格式技能树布局灵感
- [NERvGear/SAO-Utils](https://github.com/NERvGear/SAO-Utils)：游戏风格桌面应用灵感
- [aliubo/persona-text-gen](https://github.com/aliubo/persona-text-gen)：拼贴式 calling card 文字生成参考

---

## License

MIT
