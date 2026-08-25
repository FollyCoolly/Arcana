# Arcana UI 设计规范

> 状态：Current · 实现基线：2026-08-25

本文定义当前桌面 UI 的信息架构、导航状态、输入、页面职责、数据流和窗口契约。颜色、字体、图形、动效与视觉状态由 [视觉系统](./visual_style_guide.md) 定义；本文不复制逐项 CSS 数值。

## 1. 应用外壳

Arcana UI 是 SvelteKit SPA，但没有使用 SvelteKit 页面路由。`src/routes/+page.svelte` 持有 `currentScreen`，按状态条件渲染一个主菜单或一个业务 screen：

```text
main
├── status
├── skills
├── achievements
├── items
├── gallery
└── missions
```

主菜单顺序固定为 Status、Skills、Achievements、Items、Gallery、Missions，当前六项均启用。切换 screen 不改变 URL，也不保留浏览器式历史栈；返回统一回到 `main`。

页面外壳还负责：

- 监听窗口召唤事件并重置到主菜单。
- 预加载主菜单需要的 Status 和 Mission Menu Dashboard 数据。
- 在进入 Skills 前尽力预加载 Achievement 数据。
- 在主菜单展示 Calendar、Mission 倒计时/提示和 Phan-Site 进度。
- 保存跨 screen 共享的 Status、Achievement 和 Mission Menu 最新投影。

## 2. 召唤、隐藏与返回

### 2.1 窗口状态

- 应用启动时窗口隐藏。
- macOS 使用 `Cmd+Shift+R`，Windows/Linux 使用 `Ctrl+Shift+R` 全局切换窗口。
- 隐藏时，快捷键显示窗口、贴合主显示器、置于顶层、聚焦，并发送 `arcana://summoned`。
- 显示时，快捷键取消顶层状态并隐藏窗口。
- UI 内的 Hide / `Esc` 会先重置再隐藏；全局快捷键可直接隐藏，但下次召唤事件仍会重置到主菜单，因此不会恢复之前的详情页。

### 2.2 `Esc` 退栈

`Esc` 只回退当前最内层状态：

| 当前状态 | `Esc` 结果 |
| --- | --- |
| 主菜单 | 隐藏窗口 |
| 普通业务 screen | 返回主菜单 |
| Status detail / configure | 返回 Status 雷达概览 |
| Skills Achievement modal | 关闭 modal |
| Gallery detail | 返回 Gallery 封面墙 |
| Missions 建议详情 | 关闭建议详情 |
| Missions Phan-Site 列表 | 退出 Phan-Site 模式 |
| Missions 普通详情 | 关闭详情 |

业务 screen 自己监听 `Esc` 并调用 `onBack`；根页面只在 `main` 处理隐藏。新增嵌套视图必须保持这一“先关闭内层，再离开 screen”的顺序。

## 3. 输入模型

鼠标和键盘都必须能够触发主要操作。当前 screen 的快捷键如下：

| Screen | 键位 | 当前行为 |
| --- | --- | --- |
| Main | `↑` / `↓` | 在六个菜单项间循环 |
| Main | `Enter` | 打开当前项 |
| Main | `Esc` | 隐藏窗口 |
| Status detail | `Q` / `E` | 在 All 与已选择维度间循环 |
| Skills | `Q` / `E` | 在当前筛选结果的技能间循环 |
| Skills | `H` | 在 Started（等级大于 0）与 All 间切换 |
| Items | `↑` / `↓` | 在当前分类和排序结果中移动，边界处停止 |
| Missions | `↑` / `↓` | 在普通任务或建议列表中移动，边界处停止 |
| Missions | `Enter` | 打开当前详情；普通详情已打开时关闭它 |
| Missions | `Q` / `E` | 非 Phan-Site 模式下循环排序字段 |
| Missions | `P` | 切换普通任务与 Phan-Site 建议 |
| Missions | `R` | 重新加载任务和主菜单 Dashboard |

Main 菜单以及 Achievements、Items、Gallery 的侧栏使用 hover 即选中、click 确认或打开。Achievements 和 Gallery 当前没有自定义方向键导航，主要依赖原生 Tab、鼠标和点击；这属于当前实现限制，不应在文档中表述为完整键盘体验。

## 4. 主菜单

主菜单是入口和概览，不承担领域编辑：

- Calendar 显示当前日期、星期、时段与天气。
- Mission countdown 在槽位有值且 `days_remaining <= 99` 时显示；标签按 2 字或 4 字素材排版。
- Mission hints 按顺序使用一块 fat board 和后续 slim board。
- Mission progress 使用共享的 `PhanSiteProgress`。
- `↑` / `↓` 控制逻辑焦点；hover 同步焦点；click 或 `Enter` 打开 screen。
- 左下角 Hide 与 Confirm 是可点击的快捷键提示。

主菜单只消费 `MissionMenuDashboardData` 的展示投影，不自行计算任务候选，也不直接修改 Dashboard 槽位。

## 5. Screen 职责

### 5.1 Status

`StatusScreen.svelte` 有三个内部视图：

- `radar`：显示用户名、游戏天数和最多五个已选择维度。雷达节点可打开详情。
- `detail`：显示 All 或单个维度的 score 卡、缺失 Record ID、分数条、等级和称号；`Q` / `E` 循环标签。
- `configure`：五个固定 slot 使用原生 `select` 选择或清除维度，写入后重新加载 dashboard。

没有选中维度、没有 score 或数据不可用时显示明确空态。Status 只展示后端已经计算好的 score、level 和缺失项，不在前端实现评分公式。

### 5.2 Achievements

`AchievementsScreen.svelte` 展示 pack 侧栏和当前 pack 的 Achievement 卡片网格：

- 切换 pack 时重置筛选和排序。
- 名称、难度、解锁状态排序按钮按“升序 → 降序 → 默认”循环。
- 难度可多选；状态可在 All 与 Unlocked 间切换。
- 卡片显示状态、名称、难度、描述、达成日期和 prerequisites。
- `enabled` Achievement 可以 Mark achieved 或 Revoke；写入期间禁用全部状态操作，成功后重载 Achievement dashboard。

### 5.3 Skills

`SkillsScreen.svelte` 展示 Achievement 派生的技能进度：

- 默认只显示 Started 技能；`H` 切换到 All。
- 左侧展示选中技能的名称、等级、描述和 pack card image；素材读取失败时使用 `/card_examples/fool.png`。
- 右侧按 9/8 列交错六边形排列节点，并尽量让 prerequisite 与 dependent 对齐。
- 点击节点打开 Achievement 详情 modal；可用节点支持 Mark achieved / Revoke，成功后并行刷新 Achievement 与 Skill dashboard。
- prerequisites 影响布局和说明，但不阻止用户显式标记达成。

当前 screen 使用二维六边形网格，不渲染 `SkillNebula.svelte` 的 Three.js 视图。

### 5.4 Items

`ItemsScreen.svelte` 是外部物品数据的只读浏览器：

- 默认选择统计中的第一个分类；分类 hover 或 click 会切换列表。
- 支持 Name、Owned days、Price、Daily cost 排序，重复点击反转方向。
- 当前排序不是 Name 时，行尾 pill 显示对应数值；缺失值显示 `—`。
- 滚动位置驱动扇面缩放、旋转、可见性和只读滚动指示器。
- hover、click 或方向键改变当前行选择。

当前没有物品详情视图和写操作；不要把未渲染的 source stats 或 `extra` 字段写成现有功能。

### 5.5 Gallery

`GalleryScreen.svelte` 是外部媒体数据的只读浏览器：

- 固定分类为 Anime、Games、TV、Movie、Book，并按 source 的 `media_type` 过滤。
- Anime/TV/Movie/Book 支持 Rating 和 Consume date；Games 支持 Playtime 和 Rating。新分类默认使用第一个排序字段、降序。
- 封面墙展示名称，以及评分或游戏时长/成就进度；点击进入详情。
- 详情按媒体类型显示可用的评分、日期、集数、标签、时长、发行日和游戏成就信息。
- Douban 图片经本地 `imgproxy.localhost` 代理；封面失败会重试三次后显示 fallback。

`Esc` 或左下返回按钮先关闭详情，再离开 Gallery。

### 5.6 Missions

`MissionsScreen.svelte` 同时管理已接受 Mission 和本机 MissionSuggestion：

- 普通列表包含 active、completed、archived，排序字段固定循环为 Pubtime、State、Difficulty。
- Mission 详情显示状态、难度、描述、进度和 deadline 派生文案；active 可 Complete，非 archived 可 Archive。
- Active Mission 可以绑定主菜单的 countdown、progress、hint 1、hint 2。countdown 还要求 Mission 有 deadline；再次点击已选 slot 会清除它。
- `P` 进入 Phan-Site 建议列表；建议详情展示原因并支持 Accept / Reject。
- Complete、Archive、Accept、Reject 成功后重新加载 Mission dashboard 和 Mission Menu dashboard，并关闭详情；Dashboard slot 修改只刷新 Mission Menu dashboard，详情保持打开。
- `R` 提供显式刷新；滚动指示器只反映位置，不接受拖动。

## 6. 数据流与状态所有权

前端使用 Svelte 5 runes，没有全局 Svelte store。状态按以下边界所有：

| 所有者 | 状态 |
| --- | --- |
| `+page.svelte` | 当前 screen、主菜单焦点、Status cache、Achievement cache、Mission Menu cache |
| 各 screen | loading/error、内部视图、筛选、排序、当前选择、modal 和 mutation busy 状态 |
| Rust Application 层 | 领域读取、计算、校验和持久化 |

数据加载规则：

1. 根页面挂载时并行发起 Status 和 Mission Menu 预加载；失败是非致命的，screen 可再次加载。
2. Skills 打开前尝试加载 Achievement，失败时仍允许进入，并使用 ID fallback。
3. Achievements、Status 和 Missions 通过 callback 把 mutation 后的新投影交还根页面。
4. Skills mutation 同时刷新 Skill 与共享 Achievement；Items 和 Gallery 每次挂载自行读取。
5. UI 只通过 Tauri commands 访问数据，不读写 repository、SQLite 或 local-state 文件。

具体实体所有权和持久化位置见 [当前架构](./architecture.md)。UI 文档只描述消费方式，不复制 command 全量清单或数据 schema。

## 7. 异步、错误与空态

每个 screen 都要区分 loading、error、empty 和 normal：

- 初次加载期间在内容区显示 loading 文案，不呈现伪数据。
- command 错误转换为可操作或至少可识别的错误文案，并保留在当前 screen。
- mutation 期间禁用会造成重复写入的操作；成功后以重新读取的后端投影为准。
- 合法的空集合显示领域空态，例如 “No missions yet” 或 “No skills available yet”，不能与加载失败混为一谈。
- 图片等非关键素材失败时使用 fallback，不阻塞其余数据。

共享 cache 是显示优化，不是权威状态。发生写入后必须刷新受影响投影，不能只在前端乐观改一个字段。

## 8. 窗口与缩放契约

Tauri 配置和运行时共同形成以下行为：

- 配置初始尺寸为 `1200 × 800`，但 setup 和每次召唤都会把无边框窗口贴合主显示器。
- `decorations: false`、`shadow: false`、`transparent: true`、`resizable: false`、`maximizable: false`、启动不可见。
- 可见期间：macOS 使用 status window level，其他平台设置 always-on-top；隐藏时恢复普通层级。
- DPI scale factor 变化时重新贴合主显示器。
- `+layout.svelte` 调用 `setZoom(1 / devicePixelRatio)`，目的是在 Windows 抵消显示缩放，并在 scale change 后重算。
- `src/app.css` 以 `calc(100vw / 240)` 设置根字号，使 rem 相对 3840px 设计宽度缩放。

`app.css` 中存在 `data-platform="macos"` 的 16px override，但当前代码没有设置这个属性，因此不能把它视为已生效的平台策略。除主菜单的窄 viewport 降级外，各业务 screen 尚未完成统一响应式和跨平台缩放适配。

## 9. 可访问性现状

当前实现大量使用原生 `button`，装饰图形多标记为 `aria-hidden`，Skills 节点 modal 具有 `role="dialog"` 和 `aria-modal="true"`。但仍有已知缺口：

- Achievement 和 Gallery 没有方向键级的自定义导航。
- modal 打开后没有统一 focus trap、初始聚焦和焦点恢复机制。
- Missions 详情是视觉 overlay，但不是语义 dialog。
- 没有 `prefers-reduced-motion` 适配。
- 部分 canvas 文字和图片式标题依赖周边控件的可访问名称。

新增功能不得扩大这些缺口：主要操作使用原生交互元素，视觉标签提供可访问名称，modal 应管理焦点，状态不能只靠颜色。

## 10. 实现约束

- `+page.svelte` 只管理 shell、screen 切换和确有跨 screen 消费者的共享投影。
- screen 自己管理局部筛选、排序、详情、loading/error 和键位，并通过 `onBack` 返回。
- 领域计算和持久化留在 Rust；Svelte 是 command 的薄适配器。
- mutation 后刷新所有受影响投影，尤其是 Achievement → Skill 和 Mission → Main Menu 的关系。
- 新增嵌套 UI 时先定义 `Esc` 退栈，再增加快捷键提示。
- 不把未挂载组件、未使用类型或后端存在但 UI 未暴露的 command 记为当前 UI 功能。
- 视觉修改遵循 [视觉系统](./visual_style_guide.md)，不要在本文维护颜色、字体、角度或 transition 清单。

主要实现入口：`src/routes/+page.svelte`、`src/lib/screens/`、`src/lib/` 共享组件、`src-tauri/src/lib.rs` 和 `src-tauri/tauri.conf.json`。
