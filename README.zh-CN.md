# Arcana

[English](README.md) | 简体中文

一个由 AI 辅助、采用 Persona 5 风格的游戏化人生管理桌面 HUD。

Arcana 把现实中的事实与目标组织为 Status、Achievement、Skill 和 Mission。便于阅读的定义与同步状态保存在 JSON repository，Records 使用本机 SQLite；AI 能力由外部 Arcana Skills 提供，它们通过类型化的 `arcana-data` CLI 工作，应用本身不再内置模型运行时。

> [!IMPORTANT]
> 所需字体不会随项目分发，详见[字体要求](#字体要求)。

## 截图

| 主菜单 |
| --- |
| ![Arcana 主菜单](docs/screenshots/main-menu.jpg) |

<table>
  <tr><th width="50%">状态</th><th width="50%">任务</th></tr>
  <tr><td><img src="docs/screenshots/status.jpg" alt="状态" width="100%"></td><td><img src="docs/screenshots/missions.jpg" alt="任务" width="100%"></td></tr>
  <tr><th>成就</th><th>技能</th></tr>
  <tr><td><img src="docs/screenshots/achievements.jpg" alt="成就" width="100%"></td><td><img src="docs/screenshots/skills.jpg" alt="技能" width="100%"></td></tr>
  <tr><th>物品</th><th>图鉴</th></tr>
  <tr><td><img src="docs/screenshots/items.jpg" alt="物品" width="100%"></td><td><img src="docs/screenshots/gallery.jpg" alt="图鉴" width="100%"></td></tr>
</table>

## 功能

- **Record**：Status 与 Achievement 共用的扁平事实层。Definition 由已启用 Pack 提供，Record 值归用户所有。
- **Status**：Pack 中的 Dimension 从数值 Record 计算 0～100 子分数，以加权平均得到最终分数，并派生 Lv.0～Lv.5。
- **Achievement**：带前置关系的里程碑，只保存最小的 `tracked` / `achieved` 状态；只有 achieved 才计分。
- **Skill**：由 Pack 定义、从 achieved Achievement 派生的技能图；等级与节点状态不重复持久化。
- **Mission**：AI 推荐先作为本机 MissionSuggestion；用户接受后才成为可同步 Mission。Mission 支持 active、completed、archived 生命周期。
- **Pack**：可分层组织 RecordDefinition、Dimension、Achievement、Skill 与资源。桌面 Pack 页面可管理已安装内容、启用状态与安全删除影响；父子关系只负责组织，不构成启用依赖。
- **AssistantMemory**：可与用户仓库一起同步的长期语义上下文。
- **Items 与 Gallery**：读取用户指定的外部文件，外部来源保持权威，不复制进核心数据平台。

## AI 集成

Arcana 不内置 LLM 运行时，也不启动 Agent 服务。`plugins/arcana/` 中的 canonical plugin 提供三个外部 Skill：

- **Velvet Room**：记录事实、进度、修正、Achievement 状态、Status 选择和 AssistantMemory。
- **Phan Site**：生成并管理 MissionSuggestion。
- **Pack Manager**：创建、扩展和校验领域 Pack。

所有 Skill 都调用 `arcana-data`，因此 UI 与 AI 写入共享同一套校验和事务边界。`.claude/skills` 是生成的兼容镜像，不应手工修改。

## 架构

```text
src/                              SvelteKit 前端
src-tauri/src/
  application/                    类型化用例与运行时锁
  domain/                         领域模型与校验
  storage/data_repository.rs      组合存储边界
  storage/sqlite/                 Record-only migration 与 adapter
  storage/json_repository.rs      live 语义 JSON 与确定性 Codec
  storage/local_state.rs          本机 Suggestion 与选择
  commands/                       Tauri IPC 边界
  models/                         Items、Gallery、Weather 适配模型
  bin/arcana_data.rs              机器可读数据 CLI
plugins/arcana/                   canonical 外部 Agent plugin 与 Skills
docs/design/                      数据平台合约与设计决策
data-example/                     Items/Gallery/Weather 配置示例
```

live 语义仓库默认位于 `~/.arcana/repository`，Records 位于 `~/.arcana/runtime/arcana.sqlite3`，本机 Suggestion/选择位于 `~/.arcana/runtime/local-state.json`。`arcana-data json import|export` 可在组合状态与确定性、便于人工阅读的目录之间转换。目前尚未实现 Git pull/commit/push 编排。

Items、Gallery 和 Weather 从 `~/.arcana/data`（或 `ARCANA_DATA_DIR`）读取少量适配配置；它们不属于可同步的核心用户数据。

## 快速开始

前置要求：stable Rust、Node.js 18+，以及当前平台所需的 Tauri 依赖。

```bash
npm install
npm run tauri dev
```

构建并检查数据 CLI：

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin arcana-data
./src-tauri/target/debug/arcana-data capabilities
./src-tauri/target/debug/arcana-data init
./src-tauri/target/debug/arcana-data context summary
```

CLI 提供 Record、Pack、Status、Achievement、Skill、Mission、AssistantMemory、batch、dry-run 与确定性 JSON import/export 命令。多操作 batch 只支持 Record；JSON-backed mutation 使用单独命令。当前合约以 `arcana-data help` 和各命令组的 `help` 为准。

## 检查

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
python scripts/sync_agent_skills.py --check
```

## 外部来源配置

把 `data-example/` 中需要的文件复制到 `~/.arcana/data/`，再按本机情况修改：

- `item_sources.json`：Obsidian/Markdown 物品目录的绝对路径。
- `gallery_sources.json`：生成后的媒体文件路径。
- `weather.json`：Open-Meteo 使用的城市或经纬度。

媒体导入脚本位于 `scripts/fetch_bangumi.py`、`scripts/fetch_steam.py` 和 `scripts/fetch_douban.py`。凭据写入被忽略的 `scripts/config.json`，可参考 `scripts/config.example.json`。

## 字体要求

为了获得预期的视觉效果，请从合法来源安装 `p5hatty`、`Source Han Sans SC` 和 `Bebas Neue`。缺失时应用会回退到系统字体，部分布局会有所不同。

当前 UI 主要在 Windows 4K/100% 缩放环境下开发。其他分辨率、Windows 缩放比例和 macOS/Retina 仍需要统一适配。

## 文档

- [当前架构](docs/architecture.md)
- [数据平台设计与合约](docs/design/README.md)
- [Items 外部来源 Schema](docs/schema/items.md)
- [视觉风格指南](docs/visual_style_guide.md)
- [UI 设计规范](docs/ui_design_spec.md)

## 致谢

- [Mive82/Persona-5-Calendar](https://github.com/Mive82/Persona-5-Calendar)
- [sjpiper145/MakerSkillTree](https://github.com/sjpiper145/MakerSkillTree)
- [NERvGear/SAO-Utils](https://github.com/NERvGear/SAO-Utils)
- [aliubo/persona-text-gen](https://github.com/aliubo/persona-text-gen)

## License

MIT
