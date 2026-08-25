# Arcana 视觉系统

> 状态：Current · 实现基线：2026-08-25

本文定义 Arcana UI 的视觉语言：颜色、字体、形状、层级、状态表达、动效和缩放。页面职责、导航、键位、数据加载与窗口行为由 [UI 设计规范](./ui_design_spec.md) 定义；这里不维护逐页功能清单，也不复制组件中的具体坐标或 `clip-path`。

## 1. 视觉方向

Arcana 使用受 Persona 5 启发、但服务于自身信息结构的桌面 HUD 风格。当前实现有五个稳定特征：

- 红、黑、白构成高对比主界面，金色只强调 Status 数据，深灰只承担结构和弱化层。
- 硬边、斜切、旋转和不规则多边形替代常规圆角卡片。
- 星形、放射线、错位底板和遮挡形成层次；装饰不得盖过标题、数值和操作。
- 标题采用拼贴式字形和粗描边，正文保持可读，不把每段文字都做成装饰字。
- 选中、达成、完成等状态同时使用底板、位置、图标或文字变化，避免只依赖颜色。

当前 CSS 中没有使用渐变。新增 UI 应继续使用纯色、透明叠层、图片或 SVG，而不是 `linear-gradient`、`radial-gradient` 或 `conic-gradient`。

## 2. 颜色

全局视觉 token 定义在 `src/routes/+page.svelte` 的 `.rm-overlay`：

```css
--rm-black: #000000;
--rm-white: #ffffff;
--rm-red: #e5191c;
--rm-gray: #2e2e2e;
--rm-gold: #f5a623;
```

| Token | 职责 |
| --- | --- |
| `--rm-black` | 主底板、轮廓、遮挡层和最高对比文字背景 |
| `--rm-white` | 主文字、描边、分隔线和反相底板 |
| `--rm-red` | 当前选择、主要行动、强调和高进度状态 |
| `--rm-gray` | 背景结构、次级轨道和非关键装饰 |
| `--rm-gold` | Status 雷达、维度标签和数据强调；不作为通用强调色 |

整个界面叠加在 `rgba(30, 0, 0, 0.8)` 的深红透明蒙版上。窗口和页面根节点本身透明，因此桌面内容会透出；当前实现没有 Acrylic、Mica 或 CSS blur 层。

各 screen 可以为数据可视化或领域状态使用局部色值，例如雷达图的金色明暗面、技能节点红色和半透明白色。局部颜色应留在组件内，不应悄悄扩充全局 palette；需要跨页面复用时再提升为 token。

## 3. 字体与文字图形

### 3.1 字体栈

英数展示文字默认使用：

```css
font-family: "p5hatty", "Orbitron", Arial, sans-serif;
```

中文正文主要使用：

```css
font-family: "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", "Microsoft YaHei", sans-serif;
```

按键徽标默认使用 `"Bebas Neue", Arial, sans-serif`。这些字体没有随应用打包；缺少本机字体时会走 fallback，字宽和排版可能变化。字体安装要求以仓库根目录 `README.md` 为准。

### 3.2 文字角色

- `MenuItem`：主菜单和侧栏导航。逐字符改变尺寸、旋转、垂直偏移和黑白底板。
- `PromptWord`：操作提示。Canvas 以确定性抖动、黑色粗轮廓和外层反色描边绘制。
- `CallingCardText`：页面级展示标题。每次挂载可产生轻微不同的字符角度与红色点缀。
- `CollageLabel`：Status 维度和等级。金/黑碎片拼贴，不用于普通按钮或正文。
- 普通正文、元数据和错误文本：使用稳定字形、正常阅读顺序和足够行高，不增加逐字旋转。

文字图形可以夸张，但操作名称、状态和关键数值必须仍以真实文本或可访问名称表达。图片标题需要有效 `alt`；纯装饰图形使用 `aria-hidden="true"` 或空 `alt`。

## 4. 形状与构图

### 4.1 基础语法

- 主要容器使用直角、斜切角和 `clip-path: polygon(...)`。
- 相邻元素可轻微错位或旋转，但正文基线和数据列需要稳定。
- 底板通常由黑/白主体、红色选择层和粗描边组成；不要用默认浏览器按钮外观。
- 圆角只用于确有语义或素材需要的局部元素，不作为默认卡片语言。
- 阴影应短、硬、近似印刷错版；避免柔和的大面积投影。

### 4.2 当前构图模式

- 主菜单以大、小两组黑白星形和斜向分割线建立视觉中心，六个菜单项沿放射方向排列。
- 主菜单、Achievements、Items 和 Gallery 的导航都使用不规则底板与独立移动的选择四边形；选择层通过 `mix-blend-mode: difference` 反相文字。
- Status 以星形雷达、金色拼贴标签和右上标题图组织页面。
- Skills 使用左侧塔罗式图片卡和右侧交错六边形节点网格；当前 screen 没有渲染 Three.js 星云。
- Items 的列表行围绕左侧圆心形成滚动扇面；Gallery 以轻微旋转的封面墙构图；Missions 使用印章、等级字母和斜切详情卡。

这些是可复用的构图模式，不是必须复制的坐标。新增页面应先建立一个清晰视觉锚点，再添加少量方向一致的装饰，避免同时堆叠多套放射中心。

## 5. 交互状态

当前实现的主要反馈模式如下：

| 状态 | 表达方式 |
| --- | --- |
| 导航 hover / active | 底板由黑变红、字符反相、选择四边形移动到目标 |
| 键盘焦点 | 原生 `button` 焦点能力；主菜单额外显示白色 `:focus-visible` 轮廓 |
| Achievement / Skill 达成 | `✓`/`○`、状态文字、节点或卡片配色共同变化 |
| Mission 状态 | 独立状态印章、状态文字、行样式和可用操作共同变化 |
| 选中列表行 | 形状或指示三角与底色同时出现 |
| disabled / busy | 禁用交互，并显示省略号、禁用样式或错误信息 |
| loading / empty / error | 内容区内的明确文字；错误使用红色和加粗 |

新状态至少要有一种非颜色信号，例如文字、图标、边框、形状或位置。透明度可以辅助弱化，但不能成为完成、错误或当前选择的唯一信号。

## 6. 动效

动效用于确认选择和保持空间连续性，不用于持续吸引注意。当前组件采用以下区间，而不是单一的全局 timing token：

- `100–160ms`：hover、焦点、菜单四边形移动和颜色切换。
- `180–260ms`：详情卡、modal 和较大状态变化。
- `300–400ms`：进度条填充。
- 常见曲线为 `ease`、`ease-out` 和 `cubic-bezier(0.2, 0.8, 0.2, 1)`。

优先过渡 `transform`、位置、尺寸和颜色。选择层需要让形状与目标一起移动，进度条只对宽度变化做一次性过渡。当前代码尚未实现 `prefers-reduced-motion`；在补齐该能力前，不应新增自动循环或长时间装饰动画。

## 7. 尺寸与适配

UI 以 3840px 宽的设计基准使用 rem：

```css
:root {
  font-size: calc(100vw / 240);
}
```

因此基于 rem 的几何和字体随 viewport 宽度线性缩放。大量关键尺寸同时使用 `clamp()`，用于限制极端大小。当前只有主菜单在 `max-width: 980px` 下提供专门布局降级，并隐藏移动选择四边形；其他 screen 还没有完整的小窗口响应式方案。

不要把这一实现描述为已完成的跨平台适配。应用目前以全屏主显示器为运行形态，Windows 4K/100% 是主要设计环境；缩放与平台行为见 [UI 设计规范](./ui_design_spec.md#8-窗口与缩放契约)。

## 8. 共享视觉原语

活跃页面直接复用的原语包括：

| 原语 | 视觉职责 |
| --- | --- |
| `src/lib/MenuItem.svelte` | 拼贴式导航文字 |
| `src/lib/KeyHint.svelte` | 白底黑框按键徽标 |
| `src/lib/PromptWord.svelte` | 操作提示文字图形 |
| `src/lib/CallingCardText.svelte` | Calling-card 风格标题 |
| `src/lib/CollageLabel.svelte` | 金/黑碎片标签 |
| `src/lib/Calendar.svelte` | 日期、星期、时段和天气组合图 |
| `src/lib/PhanSiteProgress.svelte` | Phan-Site 问题与进度条 |
| `src/lib/components/RadarChart.svelte` | 可交互 Status 雷达图 |

`src/lib/components/SkillNebula.svelte`、`CardTitle.svelte` 及 `src/routes/+page.svelte` 中的全局塔罗卡样式仍在仓库中，但当前 Skills screen 不使用它们。它们不是现行页面的视觉依赖，复用前应先确认是否保留。

## 9. 修改检查

视觉改动提交前至少确认：

- 使用现有 token，或为新增跨页面颜色明确建立 token。
- 没有引入渐变、默认圆角卡片或无底板的主操作。
- active、disabled、error、achieved 等状态不只依赖颜色或透明度。
- 装饰元素不拦截点击，且具有正确的可访问性标记。
- 字体 fallback 下仍可读，长文本不会穿出底板。
- 在全屏目标分辨率和较窄 viewport 下检查遮挡、滚动与焦点轮廓。

页面结构和交互发生变化时更新 UI 设计规范；只有视觉语言或共享视觉原语变化时才更新本文。
