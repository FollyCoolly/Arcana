# Arcana - UI Design Spec

> **最后更新**: 2026-04-30
> **状态**: 反映当前代码实现

---

## 1. 信息架构

Arcana 采用单页 SPA 架构（非 SvelteKit 路由），所有屏幕通过 `currentScreen` 状态变量条件渲染：

- `"main"` — 主菜单
- `"status"` — 状态面板
- `"achievements"` — 成就
- `"skills"` — 技能树
- `"items"` — 物品
- `"gallery"` — 画廊
- `"missions"` — 任务

主菜单固定 6 项（按顺序，全部可用）：

1. Status
2. Skills
3. Achievements
4. Items
5. Gallery
6. Missions

主菜单底部显示两个键位提示按钮：`"Esc: Hide"`（隐藏界面）和 `"↵: Confirm"`（确认选择）。

---

## 2. 交互状态机

状态转换规则（实现于 `src/routes/+page.svelte`）：

1. 全局快捷键 `Ctrl+Shift+R` 呼出 → `currentScreen = "main"`
2. 主菜单选中项 → `currentScreen = item.id`
3. 子菜单按 `Esc` → `currentScreen = "main"`（委托给各屏幕的 `onBack`）
4. 主菜单按 `Esc` → 隐藏界面
5. 主菜单点击 `Esc: Hide` → 隐藏界面
6. 全局快捷键在可见状态 → 直接隐藏
7. 隐藏时调用 `resetToMainMenu()`，下次呼出必定进入主菜单

---

## 3. 输入规则

### 3.1 通用键位

| 键 | 主菜单 | 子菜单 |
|----|--------|--------|
| `↑` / `↓` | 移动焦点 | 由各屏幕定义 |
| `Enter` | 激活当前菜单项 | 由各屏幕定义 |
| `Esc` | 隐藏界面 | 返回主菜单 |

鼠标悬停菜单项即切换焦点，单击即激活。

### 3.2 各屏幕键位

| 屏幕 | 键位 | 功能 |
|------|------|------|
| StatusDetailView | `Q` / `E` | 切换维度 |
| SkillsScreen | `Q` / `E` | 上一个/下一个技能 |
| MissionsScreen | `Q` / `E` | 切换排序方式 |
| MissionsScreen | `P` | 切换 Phan-Site 模式 |
| MissionsScreen | `R` | 刷新任务数据 |
| MissionsScreen | `Enter` | 打开任务详情 |
| ItemsScreen | `↑` / `↓` | 导航物品列表 |

---

## 4. 主菜单页

### 4.1 布局

- 左侧：8 层大五角星 + 6 层小五角星堆叠（`80vh`，左 35%），黑白交替
- 对角线：`20%` 处斜切分隔线，左右半星独立 clip
- 菜单列表：梯形 `clip-path` + 旋转（`-30deg` 至 `2deg`），从左 30% 开始
- 右上：任务倒计时板 + 玩家信息（用户名 / Day N）
- 任务提示区：活跃任务的 slim/fat 提示板
- PhanSite 进度条
- 选中指示：红色四边形覆盖层，`mix-blend-mode: difference`，各菜单项独立配置

### 4.2 菜单项样式

| 属性 | 值 |
|------|-----|
| 底色 | `var(--rm-black)` |
| hover/焦点 | `var(--rm-red)` 底 + 白色字 |
| `clip-path` | 逐项不同的梯形 polygon |
| 旋转角 | `-30deg` → `-27deg` → `-20deg` → `-8deg` → `-2deg` → `2deg` |
| 横向偏移 | `margin-left` 从 `1.5vw` 递增至 `10vw` |
| transition | `140ms ease`（背景色） |

完整 clip-path 值见 `docs/visual_style_guide.md` 第 5.1 节。

---

## 5. 子菜单页

### 5.1 通用模式

每个子菜单页独立实现，共享以下模式：
- 左上角 `KeyHint` + `PromptWord("Back")` 返回按钮
- 不提供直接 Hide 按钮（统一 Esc 回主菜单再 Hide）
- 三种状态：`loading` / `error` / `empty` / `normal`

### 5.2 Status

- `src/lib/screens/StatusScreen.svelte` + `StatusDetailView.svelte`
- 顶层概览：雷达图（`RadarChart`）+ 维度概要卡片
- 详情视图：按维度切换，显示指标贡献条，目标区间可视化
- 系统指标（`sys_` 前缀）由后端实时计算
- 右上角 `Status.png` 标题图
- 背景：`#444444` 和黑色交替的五角星装饰

### 5.3 Achievements

- `src/lib/screens/AchievementsScreen.svelte`
- 左侧：扩展包选择侧边栏，带选中四边形覆盖层
- 右侧：成就卡片列表，按包分组
- 筛选栏：排序（名称/难度/解锁）、难度过滤、全部/已解锁切换
- 卡片显示名称、难度、描述、解锁状态

### 5.4 Skills

- `src/lib/screens/SkillsScreen.svelte`
- 六边形网格技能树 + 3D 星云卡片（`SkillNebula.svelte` / Three.js）
- 节点颜色：未解锁 `var(--rm-black)`，已解锁 `#e0093b`
- 点击节点弹出详情 modal，可解锁/锁定成就
- 检测 `ui_events` 中的成就变更并自动刷新
- `Q` / `E` 导航技能

### 5.5 Items

- `src/lib/screens/ItemsScreen.svelte`
- 左侧：分类侧边栏，带四边形选中效果
- 右侧：横向滚动物品列表，梯形 `clip-path`，滚动驱动径向扇形透视
- 物品行：名称 + 数据 pill 标签
- `↑` / `↓` 导航物品

### 5.6 Gallery

- `src/lib/screens/GalleryScreen.svelte`
- 左侧：分类侧边栏
- 主区：瀑布流封面卡片，点击展开详情
- 详情：星级评分、成就关联、图片展示

### 5.7 Missions

- `src/lib/screens/MissionsScreen.svelte`
- 任务列表：多列布局，支持排序（Q/E 切换排序方式）
- 排序轮播：可配置排序字段和方向
- 详情卡片：弹出式 overlay，显示完整描述、进度条、操作按钮
- Phan-Site 模式（P 键切换）：显示 AI 提议的任务，支持 accept/reject
- 滚动指示器

---

## 6. 组件库

`src/lib/components/` 下的可复用组件：

| 组件 | 用途 |
|------|------|
| `MenuItem.svelte` | 逐字符几何化菜单标签渲染 |
| `KeyHint.svelte` | 键位提示方块（白底黑框 + 键名） |
| `PromptWord.svelte` | Canvas 倾斜文字渲染器（支持描边） |
| `CollageLabel.svelte` | 碎片化金底黑字标签（维度名等） |
| `CallingCardText.svelte` | 倾斜字母 + 黑底红边白辉光效果 |
| `CardTitle.svelte` | SVG 卡片标题（SkillNebula 用） |
| `RadarChart.svelte` | SVG 五角星雷达图（交互式） |
| `SkillNebula.svelte` | Three.js 3D 轨道卡片星云 |
| `PhanSiteProgress.svelte` | 任务进度条（"poll" 风格） |
| `Calendar.svelte` | P5 风格日期/天气组件 |

---

## 7. 数据流

- 所有状态使用 Svelte 5 `$state()` runes（无 Svelte stores）
- 屏幕间数据通过 `$state` 变量 + prop 传递
- 数据加载：`onMount` 时预加载（`preloadStatusData()`、`preloadMissionMenuData()`），各屏幕按需调用 `invoke()`
- Tauri invoke 命令：`load_status_data`、`load_achievements`、`load_skills`、`load_items`、`load_gallery`、`load_missions`、`load_main_menu_missions`、`update_mission_status`、`set_achievement_achieved`、`lock_achievement`、`get_pending_events`、`get_weather`

---

## 8. 窗口配置

`src-tauri/tauri.conf.json`：
- 默认尺寸：1200×800
- `decorations: false`、`shadow: false`、`transparent: true`
- `visible: false`（启动隐藏，快捷键呼出）
- `resizable: false`、`maximizable: false`
- `alwaysOnTop` 由运行时 toggle（呼出时置顶，隐藏时取消）
- 呼出时扩展至主显示器全屏（`fit_to_primary()`）
- `app.css` 中 `:root { font-size: calc(100vw / 240) }` 实现流体缩放

---

## 9. 与 visual_style_guide.md 的关系

本文档描述交互架构、页面结构、数据流和组件组织。色彩、字体、动效、几何形状等视觉细节见 `docs/visual_style_guide.md`。
