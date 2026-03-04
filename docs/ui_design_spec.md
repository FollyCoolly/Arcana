# Reality Mod - UI Design Spec (v0.1)

> 状态: Draft  
> 最后更新: 2026-03-03

## 1. 信息架构

Reality Mod 的呼出界面采用三层结构:

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
- 菜单项必须有明显的 `focused / active / disabled` 区分
- 菜单项可按奇偶项做轻微旋转与横向错位，强化堆叠感
- 固定显示键位提示（如 `Enter: Select`, `Esc: Hide`）

### 4.2 子菜单页（Submenu）

- v0.1 仅 `Status` 子菜单可进入
- 子菜单页面需保留明显返回提示（`Esc: Back`）
- 子菜单不提供直接 `Hide` 按钮，统一先 `Esc` 回主菜单再 `Hide`

### 4.3 窗口形态约束

- 界面为无边框沉浸式窗口
- 不出现系统窗口控制按钮（最小化/最大化/关闭）
- 窗口背景透明，视觉主体由不规则图形组成，不使用整屏规则矩形底板
- 推荐构成：左侧主菜单堆叠 + 右侧斜切信息板 + 背景几何层（如切片/星形）

## 5. 状态设计

### 5.1 主菜单状态

- `normal`: 正常可导航
- `disabled-item`: 禁用项可见但不可进入，显示 `Coming Soon`

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

## 7. 验收清单

1. 任意时刻从隐藏态呼出，首屏必须是 `MainMenu`。
2. `Status` 可进入，其他五个模块均为禁用且提示 `Coming Soon`。
3. 子菜单按 `Esc` 回主菜单，不直接隐藏。
4. 主菜单按 `Esc` 或选择 `Hide` 都会隐藏界面。
5. 下次呼出不会直接显示上次子菜单内容。
6. 菜单支持鼠标点击与键盘 `ArrowUp/ArrowDown/Enter/Esc` 完整链路。
7. 界面不显示系统窗口控制按钮。
8. 窗口背景透明，主视觉仍保持清晰可读，不出现“普通窗口”观感。
9. 页面仅使用 `#000000 / #ffffff / #ff0033` 三色，且不使用渐变。
