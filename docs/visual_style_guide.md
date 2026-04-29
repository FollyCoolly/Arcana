# Arcana - 视觉风格指南（P5-Inspired）

> **最后更新**: 2026-04-30
> **状态**: 反映当前代码实现

---

## 1. 核心视觉语法

- 主色基调严格限定为 `红 / 黑 / 白`
- 窗口透明背景，界面主体以不规则图形层叠呈现
- 倾斜与切角是核心语法，避免通篇圆角卡片化
- 通过层叠、遮挡、错位制造节奏和速度感
- 文本层级优先，标题与数据应比装饰图形更强势
- UI 元素必须有强形状：裸文字不构成 UI 组件，任何按钮、标签、导航项都需要有几何底座
- 状态变化必须是形状+颜色双重变化，不允许仅改变透明度或颜色来表达交互状态

---

## 2. 色彩规范

### 2.1 Design Tokens

定义于 `src/routes/+page.svelte` 的 `.rm-overlay` 内：

```css
--rm-black: #000000;
--rm-white: #ffffff;
--rm-red: #e5191c;
--rm-gold: #f5a623;
--rm-gray: #2e2e2e;
```

### 2.2 使用规则

- **禁止使用渐变**（linear-gradient / radial-gradient / conic-gradient）
- 核心 UI 配色为红 / 黑 / 白三色体系
- 金色专用于数据可视化层：维度标签、雷达图填充、分数指示等 Status 相关元素。不得用于通用 UI 组件
- 灰色专用于装饰性结构元素：背景星形、分隔线、次级几何图形等

### 2.3 实际代码中的颜色变体

以下为代码中实际使用的颜色变体（非 token，硬编码在各组件中）：

| 颜色 | 用途 | 位置 |
|------|------|------|
| `#FCCC2C` | 雷达图数据星右半三角 | `RadarChart.svelte` |
| `#E7AE16` | 雷达图数据星左半三角 | `RadarChart.svelte` |
| `#1a1a1a` | 雷达图网格星描边 / 辐线 | `RadarChart.svelte` |
| `#2d2d2d` | 雷达图网格星交替填充 | `RadarChart.svelte` |
| `#555555` | 雷达图网格点 | `RadarChart.svelte` |
| `#444444` | Status 页装饰星形填充 | `StatusScreen.svelte` |
| `#e0093b` | 技能树已解锁节点 | `SkillsScreen.svelte` |
| `rgba(30, 0, 0, 0.8)` | 全局蒙版层 | `+page.svelte` |
| `rgba(255,255,255,0.xx)` | 多处白色半透明变体 | 各组件 |

---

## 3. 字体规范

### 3.1 英数标题字体栈

```css
font-family: "p5hatty", "Orbitron", Arial, sans-serif;
```

- `p5hatty` 为项目首选字体（由系统安装或外部加载）
- `Orbitron` 为几何感等宽回退
- `Arial`、`sans-serif` 为系统回退

### 3.2 中文正文字体栈

```css
font-family: "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", "Microsoft YaHei", sans-serif;
```

用于 `MissionsScreen`、`ItemsScreen`、`PhanSiteProgress`、`+page.svelte` 等页面的中文正文区域。

### 3.3 特殊字体

- `KeyHint.svelte` 使用 `"Bebas Neue", Arial, sans-serif` 用于按键提示

### 3.4 字号层级

- `Display`：菜单主项，`clamp(3rem, 7vw, 4.8rem)`（约 48–77px）
- `H1`: 32-40px
- `H2`: 24-28px
- `Body`: 14-16px
- `Meta`: 11-12px（标签、注释）

---

## 4. 分层结构

界面呼出时为 4 层结构（从底到顶）：

| 层级 | 名称 | 实现方式 | 说明 |
|------|------|----------|------|
| 0 | OS 模糊层 | Tauri `Effect::Acrylic` | 将桌面及其他窗口磨砂模糊 |
| 1 | 蒙版层 | `background: rgba(30, 0, 0, 0.8)` | 全屏深暗红半透明覆盖 |
| 2 | 结构层 | 几何背景图形 | 五角星叠层，居于左侧 |
| 3 | 交互层 | 菜单项、信息面板 | 可点击的 UI 组件 |

### 4.1 背景星形

主页面左侧星形堆叠（`+page.svelte`）：

- **8 颗大星**：`width: 80vh`，`scale` 从 `0.92` 递减至 `0.08`，黑白交替
- **6 颗小星**：`scale` 从 `0.5` 递减至 `0.08`，黑白交替
- **分割线**：对角线 `clip-path` 分割左右两半，中间白色细线
- 星形使用 10 顶点标准五角星 `clip-path` 多边形

各屏幕页（`StatusScreen`、`SkillsScreen` 等）有独立的星形配置。

---

## 5. 几何与布局规范

### 5.1 菜单项

主菜单项使用梯形 `clip-path` + 旋转，模拟从左侧星形向外辐射的扇形：

| 子项 | 旋转角 | clip-path |
|------|--------|-----------|
| 1 | `-30deg` | `polygon(0% 10%, 100% 0%, 90% 88%, 10% 96%)` |
| 2 | `-27deg` | `polygon(0% 5%, 99% 10%, 96% 94%, 2% 100%)` |
| 3 | `-20deg` | `polygon(2% 0%, 100% 8%, 98% 100%, 0% 90%)` |
| 4 | `-8deg` | `polygon(0% 6%, 98% 0%, 100% 92%, 1% 100%)` |
| 5 | `-2deg` | `polygon(1% 0%, 100% 4%, 97% 96%, 0% 100%)` |
| 6 | `2deg` | `polygon(0% 8%, 99% 0%, 100% 100%, 3% 92%)` |

### 5.2 选中四边形

菜单选中时出现四边形覆盖层，配合 `mix-blend-mode: difference`，各菜单项有独立配置（旋转角 `-35deg` 至 `2deg`，各异的 `polygon` clip-path）。

### 5.3 侧边导航按钮

侧边导航按钮（`rm-ach-pack-btn`、`rm-gallery-pack-btn`、`rm-items-cat-btn` 等模式）：
- 斜切多边形 `clip-path` 底板
- hover 时配色反转（黑转红或红转黑）
- 选中指示使用独立的四边形覆盖层（quad），与主菜单的选中逻辑一致

### 5.4 返回按钮

返回按钮（`rm-back-btn`）：
- `position: fixed`，固定在左下角
- `background: none; border: none`（透明底，无几何色块）
- `transform: rotate(2deg)`
- hover: `scale(1.06)`
- 子元素 `KeyHint` + `PromptWord` 组合构成视觉表现

### 5.5 几何形状

- 斜切角范围：约 `2deg` 至 `35deg`（含菜单项、选中四边形、各页侧边栏按钮）
- 圆角极少使用，核心区使用硬边或切角
- 广泛使用 `clip-path: polygon()` 实现非矩形形状

---

## 6. 动效规范

### 6.1 实际使用的时长与曲线

代码中实际使用的 transition 组合（按频率排序）：

| 时长 | 曲线 | 典型用途 |
|------|------|----------|
| `120ms ease` | 选择四边形、按钮 hover、返回按钮、侧边按钮 |
| `120ms cubic-bezier(0.2, 0.8, 0.2, 1)` | Tab 按钮、排序按钮 |
| `140ms ease` | 菜单项 / 侧边导航按钮背景色切换 |
| `180ms cubic-bezier(0.2, 0.8, 0.2, 1)` | Modal 弹出动画 |
| `260ms cubic-bezier(0.2, 0.8, 0.2, 1)` | Modal 弹出、进度条填充 |
| `100ms ease` | 任务行快速交互 |
| `150ms ease` | 技能节点状态变化 |
| `180ms ease-out` | 详情卡片弹出 |
| `200ms ease` | 百分比文字变色 |
| `300ms ease` | 进度条填充 |
| `400ms cubic-bezier(0.2, 0.8, 0.2, 1)` | PhanSite 任务进度条 |

### 6.2 动效类型

- 进场：位移 + 透明度 / 缩放弹出
- 切换：背景色反转 + 形状位移
- 选择指示：四边形的 position/size/transform/clip 同步过渡

### 6.3 注意

- 当前代码**未实现 `prefers-reduced-motion` 支持**
- 无全局 keyframe 动画定义于 `app.css`，动画均由各组件内联定义

---

## 7. 组件风格参考

### 7.1 按钮

- 主导航按钮：几何斜切底板 + 色彩反转 hover
- Tab 按钮：`clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%)` + `::before` 内填充
- 裸文字按钮仅用于 `rm-back-btn` 等特殊场合，子元素提供几何底座

### 7.2 卡片

- 卡片有底板阴影层
- 标题区与数据区明确分区
- 状态条/角标仅使用红或白

---

## 附录：文件索引

| 文件 | 主要内容 |
|------|----------|
| `src/routes/+page.svelte` | Design tokens、分层结构、菜单项、返回按钮、字体栈 |
| `src/app.css` | Tailwind v4 导入、全局 font-size |
| `src/lib/components/RadarChart.svelte` | 雷达图色彩（金、灰变体） |
| `src/lib/components/MenuItem.svelte` | 菜单项字体 |
| `src/lib/components/CollageLabel.svelte` | 标签字体 |
| `src/lib/components/PromptWord.svelte` | Canvas 文字渲染 |
| `src/lib/components/KeyHint.svelte` | 按键提示字体 |
| `src/lib/screens/StatusScreen.svelte` | Status 页装饰星形色彩 |
| `src/lib/screens/SkillsScreen.svelte` | 技能节点色彩、`--rm-gold` 局部覆写 |
| `src/lib/screens/MissionsScreen.svelte` | 任务行动效 |
| `src/lib/screens/AchievementsScreen.svelte` | 成就页动效 |
| `src/lib/screens/ItemsScreen.svelte` | 物品页动效 |
| `src/lib/screens/GalleryScreen.svelte` | 画廊页动效 |
| `src/lib/screens/StatusDetailView.svelte` | 状态详情动效 |
| `src/lib/components/PhanSiteProgress.svelte` | 任务进度条动效 |
