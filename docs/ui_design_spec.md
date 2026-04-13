# Arcana - UI Design Spec (v0.3.0)

> 状态: Draft
> 最后更新: 2026-03-08

## 1. 信息架构

Arcana 的呼出界面采用三层结构:

- `Hidden`: 界面隐藏（回到现实）。
- `MainMenu`: 主菜单层（每次呼出默认进入这里）。
- `Submenu`: 子菜单层（模块内容页）。

主菜单第一版固定包含以下项（按顺序）:

1. `Status`（可用）
2. `Skills`（禁用，Coming Soon）
3. `Achievements`（禁用，Coming Soon）
4. `Items`（禁用，Coming Soon）
5. `Gallery`（禁用，Coming Soon）
6. `Crafting`（禁用，Coming Soon）
7. `Hide`（可用，隐藏界面）

## 2. 交互状态机

状态转换规则如下:

1. 全局快捷键呼出: `Hidden -> MainMenu`
2. 主菜单选择 `Status`: `MainMenu -> Submenu(Status)`
3. 主菜单选择禁用项: 留在 `MainMenu`，显示 `Coming Soon`
4. 子菜单按 `Esc`: `Submenu(*) -> MainMenu`
5. 主菜单按 `Esc`: `MainMenu -> Hidden`
6. 主菜单选择 `Hide`: `MainMenu -> Hidden`
7. 全局快捷键在可见状态触发: 直接进入 `Hidden`

隐藏后的回归规则:

- 下次呼出一律进入 `MainMenu`
- 不直接恢复到之前的 `Submenu`

## 3. 输入规则

支持 `鼠标 + 键盘`。

键盘规则:

- `ArrowUp / ArrowDown`: 在主菜单项间移动焦点
- `Enter`: 激活当前菜单项
- `Esc`:
  - 在 `Submenu` 时返回主菜单
  - 在 `MainMenu` 时隐藏界面

鼠标规则:

- 单击菜单项 = 激活该项
- 单击禁用项 = 提示 `Coming Soon`

焦点规则:

- 每次从 `Hidden` 呼出时，默认焦点落在 `Status`

## 4. 页面骨架

### 4.1 主菜单页（MainMenu）

- 主视觉为菜单列表，不展示子菜单数据内容
- 菜单项必须有明显的 `focused / active / disabled` 区分；`disabled` 项与正常项外观一致，仅交互不可用
- 菜单项默认为纯黑底白字，无多层投影；焦点/悬停状态切换为红底白字
- 菜单项形状为**梯形**：`clip-path: polygon(0 15%, 100% 0%, 100% 100%, 0 85%)`，左侧高度为右侧 70%
- 菜单项角度呈**放射状**：第 1 项 −9°，每项递增 3°，第 7 项 +9°，从左侧星星向右辐射
- 奇偶项额外有 `translateX` 横向错位，叠加旋转形成节奏感
- 固定显示键位提示（如 `Enter: Select`, `Esc: Hide`）

### 4.2 子菜单页（Submenu）

- v0.1 仅 `Status` 子菜单可进入
- 子菜单页面需保留明显返回提示（`Esc: Back`）
- 子菜单不提供直接 `Hide` 按钮，统一先 `Esc` 回主菜单再 `Hide`

### 4.3 窗口形态约束

- 界面为无边框沉浸式窗口
- 不出现系统窗口控制按钮（最小化/最大化/关闭）
- 呼出时窗口自动扩展至主显示器全屏尺寸，并置顶（always-on-top）；隐藏时取消置顶
- 窗口启用 OS 级 Acrylic 磨砂模糊效果，将桌面及其他应用窗口内容模糊化
- 全屏背景叠加深红色蒙版（`rgba(150, 0, 15, 0.80)`），营造游戏暂停画面的沉浸感
- UI 主体（菜单、信息板）浮于红色蒙版之上，不使用整屏规则矩形底板
- 推荐构成：7 层同心五角星（80vh，居左 25%）+ 菜单列表（梯形放射状，起于左 30%）+ 右侧斜切信息板

## 5. 状态设计

### 5.1 主菜单状态

- `normal`: 正常可导航
- `disabled-item`: 禁用项可见但不可进入，外观与正常项相同（不降低透明度），显示 `Coming Soon`

### 5.2 Status 子菜单状态

- `loading`: 数据加载中
- `empty`: 无可展示指标
- `error`: 数据读取或解析失败
- `normal`: 正常展示指标卡片

## 6. 关键文案与语义

- `Hide`: 隐藏呼出界面，回到现实；不是退出应用进程
- `Coming Soon`: 模块尚未开放
- `Esc: Back`: 在子菜单返回主菜单
- `Esc: Hide`: 在主菜单隐藏界面

## 7. 分辨率适配

### 7.1 适配目标

- 最低分辨率：1920×1080（1080p）
- 最高分辨率：3840×2160（4K）
- 不要求适配 1080p 以下分辨率

### 7.2 适配方案

所有尺寸相关的 CSS 属性使用 `clamp(min, preferred, max)` 表达：

- `preferred` 使用视口单位（`vw` / `vh`），使尺寸随分辨率线性缩放
- `min` 防止在低分辨率下过小；`max` 防止在高分辨率下失控放大
- 字体使用 `clamp(…, …vw, …)` 确保在 4K 下依然清晰可读
- 间距、内边距使用 `clamp(…, …vw/vh, …)` 同步缩放

图片素材均以 **2× CSS 像素** 导出（`OUTPUT_SCALE = 2`），在 1080p 标准 DPI 下等比显示，在 4K 高 DPI 下保持清晰。

### 7.3 Status 标题图片

- 素材路径：`/ui/Status.png`（500×320px，2× 输出）
- 位置：`position: fixed`，右上角
  - `top: clamp(0.8rem, 1.5vh, 3rem)`
  - `right: clamp(1.2rem, 2.5vw, 5rem)`
- 尺寸：`height: clamp(9rem, 15vh, 27rem)`，宽度自适应
  - 1080p：约 162px 高
  - 4K：约 324px 高
- `pointer-events: none`，不拦截鼠标事件
- 替代原文字标题 `<h2>Status</h2>`

## 8. 验收清单

1. 任意时刻从隐藏态呼出，首屏必须是 `MainMenu`。
2. `Status` 可进入，其他五个模块均为禁用且提示 `Coming Soon`。
3. 子菜单按 `Esc` 回主菜单，不直接隐藏。
4. 主菜单按 `Esc` 或选择 `Hide` 都会隐藏界面。
5. 下次呼出不会直接显示上次子菜单内容。
6. 菜单支持鼠标点击与键盘 `ArrowUp/ArrowDown/Enter/Esc` 完整链路。
7. 界面不显示系统窗口控制按钮。
8. 窗口背景透明，主视觉仍保持清晰可读，不出现”普通窗口”观感。
9. 页面 UI 组件仅使用 `#000000 / #ffffff / #E5191C` 三色，且不使用渐变。
10. 呼出时窗口覆盖整个主显示器，不留边距。
11. 全屏背景呈现深红色半透明蒙版，桌面/其他窗口内容在其下可见且已模糊。
12. 界面可见时置顶，不被其他窗口遮挡；隐藏后取消置顶。
