# Reality Mod - 目录结构演变

> **版本**: v0.1.0  
> **最后更新**: 2026-01-22

本文档记录 Reality Mod 项目在不同开发阶段的目录结构变化，帮助理解项目的渐进式演进过程。

---

## 📋 目录

- [Reality Mod - 目录结构演变](#reality-mod---目录结构演变)
  - [📋 目录](#-目录)
  - [阶段二：MVP - Status 模块](#阶段二mvp---status-模块)
    - [目录结构](#目录结构)
    - [关键文件说明](#关键文件说明)
      - [后端 (Rust)](#后端-rust)
      - [前端 (Svelte)](#前端-svelte)
      - [数据文件](#数据文件)
    - [阶段特点](#阶段特点)
  - [阶段三：Achievements \& Skills 系统](#阶段三achievements--skills-系统)
    - [新增文件](#新增文件)
    - [关键变化](#关键变化)
  - [阶段四：Items 物品系统](#阶段四items-物品系统)
    - [新增文件](#新增文件-1)
  - [阶段五：Gallery 媒体画廊](#阶段五gallery-媒体画廊)
    - [新增文件](#新增文件-2)
  - [阶段七：完整版本](#阶段七完整版本)
    - [完整目录树](#完整目录树)
  - [演变总结](#演变总结)
  - [设计原则](#设计原则)

---

## 阶段二：MVP - Status 模块

**目标**：完成最小可用原型，验证技术栈和架构设计。

### 目录结构

```
RealityMod/
│
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 应用入口
│   │   │
│   │   ├── models/               # 数据模型
│   │   │   ├── mod.rs
│   │   │   └── status.rs         # Status 数据结构
│   │   │
│   │   ├── storage/              # 数据存储层
│   │   │   ├── mod.rs
│   │   │   └── json_store.rs     # JSON 读取
│   │   │
│   │   └── commands/             # Tauri Commands
│   │       ├── mod.rs
│   │       └── status.rs         # load_status_data
│   │
│   ├── Cargo.toml
│   └── tauri.conf.json           # Tauri 配置（快捷键等）
│
├── src/                          # Svelte 前端
│   ├── lib/
│   │   ├── components/           # UI 组件
│   │   │   ├── common/           # 通用组件
│   │   │   │   └── Card.svelte
│   │   │   │
│   │   │   └── status/           # Status 模块组件
│   │   │       ├── StatusCard.svelte
│   │   │       └── TrendChart.svelte
│   │   │
│   │   ├── stores/               # 状态管理
│   │   │   └── statusStore.ts
│   │   │
│   │   ├── utils/                # 工具函数
│   │   │   └── formatters.ts     # 数据格式化
│   │   │
│   │   └── types/                # TypeScript 类型
│   │       └── status.ts
│   │
│   ├── routes/                   # 页面路由
│   │   ├── +layout.svelte        # 全局布局
│   │   └── +page.svelte          # Status 页面
│   │
│   ├── app.html
│   └── app.css                   # Tailwind CSS
│
├── data/                         # 本地数据
│   └── status.json               # Status 示例数据
│
├── docs/
│   ├── architecture.md
│   └── directory_structure.md    # (本文档)
│
├── public/
│   └── icons/
│
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── tsconfig.json
├── vite.config.ts
└── README.md
```

### 关键文件说明

#### 后端 (Rust)

| 文件                    | 职责                          |
| ----------------------- | ----------------------------- |
| `main.rs`               | 应用入口，注册 Tauri Commands |
| `models/status.rs`      | Status 数据结构定义           |
| `storage/json_store.rs` | 通用 JSON 文件读取工具        |
| `commands/status.rs`    | `load_status_data` 命令实现   |
| `tauri.conf.json`       | 配置全局快捷键、窗口属性      |

#### 前端 (Svelte)

| 文件                                  | 职责                |
| ------------------------------------- | ------------------- |
| `routes/+page.svelte`                 | Status 页面主体     |
| `components/status/StatusCard.svelte` | 单个数据卡片        |
| `components/status/TrendChart.svelte` | 趋势图表组件        |
| `stores/statusStore.ts`               | Status 数据状态管理 |
| `types/status.ts`                     | TypeScript 类型定义 |

#### 数据文件

| 文件               | 内容示例         |
| ------------------ | ---------------- |
| `data/status.json` | 体重、运动数据等 |

### 阶段特点

- ✅ **最小化**：只包含 Status 模块必需的文件
- ✅ **可验证**：能够读取 JSON 并展示数据可视化
- ✅ **可扩展**：目录结构为后续模块留有空间

---

## 阶段三：Achievements & Skills 系统

**目标**：实现核心的成就和技能树系统，引入 Content Packs。

### 新增文件

```diff
RealityMod/
├── src-tauri/
│   ├── src/
│   │   ├── models/
│   │   │   ├── status.rs
+  │   │   │   ├── achievement.rs    # Achievement 数据结构
+  │   │   │   ├── skill.rs          # Skill, SkillTree
+  │   │   │   └── pack.rs           # ContentPack
│   │   │
+  │   │   ├── services/             # 业务逻辑层 (新增)
+  │   │   │   ├── mod.rs
+  │   │   │   ├── skill_calculator.rs
+  │   │   │   ├── achievement_checker.rs
+  │   │   │   └── pack_manager.rs
│   │   │
│   │   └── commands/
│   │       ├── status.rs
+  │   │       ├── achievements.rs   # 成就相关命令
+  │   │       ├── skills.rs         # 技能树相关命令
+  │   │       └── packs.rs          # Pack 管理命令
│
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── status/
+  │   │   │   ├── achievements/     # 成就模块组件
+  │   │   │   │   ├── AchievementList.svelte
+  │   │   │   │   ├── AchievementCard.svelte
+  │   │   │   │   └── Timeline.svelte
+  │   │   │   │
+  │   │   │   └── skills/           # 技能树模块组件
+  │   │   │       ├── SkillTreeView.svelte
+  │   │   │       ├── SkillNode.svelte
+  │   │   │       └── SkillTooltip.svelte
│   │   │
│   │   ├── stores/
│   │   │   ├── statusStore.ts
+  │   │   │   ├── achievementStore.ts
+  │   │   │   ├── skillStore.ts
+  │   │   │   └── packStore.ts
│   │   │
│   │   ├── utils/
│   │   │   ├── formatters.ts
+  │   │   │   └── graph.ts          # DAG 图算法
│   │   │
│   │   └── types/
│   │       ├── status.ts
+  │   │       ├── achievement.ts
+  │   │       └── skill.ts
│   │
│   ├── routes/
│   │   ├── +page.svelte            # 现在作为 Dashboard
+  │   │   ├── achievements/
+  │   │   │   └── +page.svelte
+  │   │   └── skills/
+  │   │       └── +page.svelte
│
├── data/
│   ├── status.json
+  │   └── packs/                    # Content Packs
+  │       ├── programmer/
+  │       │   ├── manifest.json
+  │       │   ├── achievements.json
+  │       │   └── skills.json
+  │       └── fitness/
+  │           ├── manifest.json
+  │           ├── achievements.json
+  │           └── skills.json
```

### 关键变化

- ➕ **业务逻辑层**：新增 `services/` 目录，处理技能等级计算、成就解锁检查
- ➕ **Content Packs**：支持动态加载成就包和技能树
- ➕ **多页面路由**：Achievements 和 Skills 独立页面
- ➕ **DAG 渲染**：技能树使用 D3.js 渲染有向无环图

---

## 阶段四：Items 物品系统

**目标**：实现物品管理功能。

### 新增文件

```diff
RealityMod/
├── src-tauri/
│   ├── src/
│   │   ├── models/
+  │   │   │   └── item.rs           # Item 数据结构
│   │   │
│   │   └── commands/
+  │   │       └── items.rs          # 物品相关命令
│
├── src/
│   ├── lib/
│   │   ├── components/
+  │   │   │   └── items/            # 物品模块组件
+  │   │   │       ├── ItemList.svelte
+  │   │   │       ├── ItemCard.svelte
+  │   │   │       └── ItemDetail.svelte
│   │   │
│   │   ├── stores/
+  │   │   │   └── itemStore.ts
│   │   │
│   │   └── types/
+  │   │       └── item.ts
│   │
│   ├── routes/
+  │   │   └── items/
+  │   │       └── +page.svelte
│
├── data/
+  │   └── items.json                # 物品数据
```

---

## 阶段五：Gallery 媒体画廊

**目标**：聚合阅读、观影、游戏数据。

### 新增文件

```diff
RealityMod/
├── src-tauri/
│   ├── src/
│   │   ├── models/
+  │   │   │   └── media.rs          # Book, Movie, Game
│   │   │
│   │   └── commands/
+  │   │       └── gallery.rs        # 画廊相关命令
│
├── src/
│   ├── lib/
│   │   ├── components/
+  │   │   │   └── gallery/          # 画廊模块组件
+  │   │   │       ├── MediaGrid.svelte
+  │   │   │       ├── MediaCard.svelte
+  │   │   │       └── FilterBar.svelte
│   │   │
│   │   ├── stores/
+  │   │   │   └── galleryStore.ts
│   │   │
│   │   └── types/
+  │   │       └── media.ts
│   │
│   ├── routes/
+  │   │   └── gallery/
+  │   │       └── +page.svelte
│
├── data/
+  │   └── imports/                  # 外部导入数据
+  │       ├── bangumi_anime.json
+  │       ├── steam_games.json
+  │       └── goodreads_books.json
│
+├── scripts/                        # 数据导入脚本
+│   ├── import_bangumi.py
+│   ├── import_steam.py
+│   └── import_goodreads.py
```

---

## 阶段七：完整版本

所有模块开发完成后的完整目录结构。

### 完整目录树

```
RealityMod/
│
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 应用入口
│   │   │
│   │   ├── models/               # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── status.rs         # Status
│   │   │   ├── achievement.rs    # Achievement
│   │   │   ├── skill.rs          # Skill, SkillTree
│   │   │   ├── pack.rs           # ContentPack
│   │   │   ├── item.rs           # Item
│   │   │   ├── media.rs          # Book, Movie, Game
│   │   │   └── recipe.rs         # Recipe
│   │   │
│   │   ├── storage/              # 数据存储层
│   │   │   ├── mod.rs
│   │   │   ├── json_store.rs     # 通用 JSON 读写
│   │   │   └── file_manager.rs   # 文件系统操作
│   │   │
│   │   ├── services/             # 业务逻辑层
│   │   │   ├── mod.rs
│   │   │   ├── skill_calculator.rs
│   │   │   ├── achievement_checker.rs
│   │   │   ├── pack_manager.rs
│   │   │   └── data_validator.rs
│   │   │
│   │   ├── commands/             # Tauri Commands
│   │   │   ├── mod.rs
│   │   │   ├── status.rs
│   │   │   ├── achievements.rs
│   │   │   ├── skills.rs
│   │   │   ├── packs.rs
│   │   │   ├── items.rs
│   │   │   └── gallery.rs
│   │   │
│   │   └── importers/            # 外部数据导入器
│   │       ├── mod.rs
│   │       ├── github.rs
│   │       └── bangumi.rs
│   │
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── src/                          # Svelte 前端
│   ├── lib/
│   │   ├── components/
│   │   │   ├── common/           # 通用组件
│   │   │   │   ├── Button.svelte
│   │   │   │   ├── Card.svelte
│   │   │   │   └── Modal.svelte
│   │   │   │
│   │   │   ├── status/           # Status 模块
│   │   │   │   ├── StatusCard.svelte
│   │   │   │   └── TrendChart.svelte
│   │   │   │
│   │   │   ├── achievements/     # Achievements 模块
│   │   │   │   ├── AchievementList.svelte
│   │   │   │   ├── AchievementCard.svelte
│   │   │   │   └── Timeline.svelte
│   │   │   │
│   │   │   ├── skills/           # Skills 模块
│   │   │   │   ├── SkillTreeView.svelte
│   │   │   │   ├── SkillNode.svelte
│   │   │   │   └── SkillTooltip.svelte
│   │   │   │
│   │   │   ├── items/            # Items 模块
│   │   │   │   ├── ItemList.svelte
│   │   │   │   ├── ItemCard.svelte
│   │   │   │   └── ItemDetail.svelte
│   │   │   │
│   │   │   ├── gallery/          # Gallery 模块
│   │   │   │   ├── MediaGrid.svelte
│   │   │   │   ├── MediaCard.svelte
│   │   │   │   └── FilterBar.svelte
│   │   │
│   │   ├── stores/               # 状态管理
│   │   │   ├── statusStore.ts
│   │   │   ├── achievementStore.ts
│   │   │   ├── skillStore.ts
│   │   │   ├── packStore.ts
│   │   │   ├── itemStore.ts
│   │   │   └── galleryStore.ts
│   │   │
│   │   ├── utils/                # 工具函数
│   │   │   ├── formatters.ts
│   │   │   ├── validators.ts
│   │   │   └── graph.ts
│   │   │
│   │   └── types/                # TypeScript 类型
│   │       ├── status.ts
│   │       ├── achievement.ts
│   │       ├── skill.ts
│   │       ├── item.ts
│   │       ├── media.ts
│   │       └── recipe.ts
│   │
│   ├── routes/                   # 页面路由
│   │   ├── +layout.svelte        # 全局布局
│   │   ├── +page.svelte          # Dashboard
│   │   ├── status/
│   │   │   └── +page.svelte
│   │   ├── achievements/
│   │   │   └── +page.svelte
│   │   ├── skills/
│   │   │   └── +page.svelte
│   │   ├── items/
│   │   │   └── +page.svelte
│   │   └── gallery/
│   │       └── +page.svelte
│   │
│   ├── app.html
│   └── app.css
│
├── data/                         # 本地数据存储
│   ├── status.json
│   ├── items.json
│   ├── recipes.json
│   │
│   ├── packs/                    # Content Packs
│   │   ├── programmer/
│   │   │   ├── manifest.json
│   │   │   ├── achievements.json
│   │   │   └── skills.json
│   │   │
│   │   ├── fitness/
│   │   │   ├── manifest.json
│   │   │   ├── achievements.json
│   │   │   └── skills.json
│   │   │
│   │   └── meta.json             # Pack 元数据索引
│   │
│   └── imports/                  # 外部导入数据
│       ├── github_stats.json
│       ├── bangumi_anime.json
│       ├── steam_games.json
│       └── goodreads_books.json
│
├── scripts/                      # 数据导入脚本
│   ├── import_bangumi.py
│   ├── import_github.py
│   ├── import_steam.py
│   └── import_goodreads.py
│
├── docs/
│   ├── architecture.md
│   ├── directory_structure.md    # (本文档)
│   ├── data_schema.md
│   └── pack_creation_guide.md
│
├── public/
│   └── icons/
│
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── tsconfig.json
├── vite.config.ts
└── README.md
```

---

## 演变总结

| 阶段             | 新增模型                                    | 新增路由                     | 新增服务                                                               | 数据文件                    |
| ---------------- | ------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------- | --------------------------- |
| **阶段二 (MVP)** | `status.rs`                                 | `/` (Status)                 | -                                                                      | `status.json`               |
| **阶段三**       | `achievement.rs`<br>`skill.rs`<br>`pack.rs` | `/achievements`<br>`/skills` | `skill_calculator.rs`<br>`achievement_checker.rs`<br>`pack_manager.rs` | `packs/` 目录               |
| **阶段四**       | `item.rs`                                   | `/items`                     | -                                                                      | `items.json`                |
| **阶段五**       | `media.rs`                                  | `/gallery`                   | -                                                                      | `imports/` 目录<br>导入脚本 |

---

## 设计原则

1. **渐进式开发**：每个阶段只添加必要的文件，避免过早创建空文件
2. **目录一致性**：每个模块遵循相同的结构模式（model → command → component → route）
3. **可预测性**：新增模块时，开发者能够清楚知道需要创建哪些文件
4. **向后兼容**：新阶段不会破坏已有功能

---

**文档维护者**: RealityMod Team  
**相关文档**: [架构设计](./architecture.md) | [数据结构设计](./data_schema.md)
