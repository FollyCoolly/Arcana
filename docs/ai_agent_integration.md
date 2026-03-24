# AI Agent 集成方案

## 目标功能

1. **AI 人生教练 / NPC 对话系统** — Agent 读取角色状态、成就、技能数据，像 RPG NPC 一样给出任务建议和反馈
2. **智能成就/任务生成** — 根据用户数据和目标，自动生成个性化的 achievement pack
3. **自然语言数据录入** — "今天跑了5公里，体重75.2" → 自动更新 status.json

## 技术方案：MCP Server

采用 MCP（Model Context Protocol）作为核心接入层。原因：

- **Nanobot 本身基于 MCP 构建**，写 MCP Server = 写 Nanobot skill，一石二鸟
- MCP Server 同时可被 Claude Code、Cursor 等开发工具直接调用
- 底层复用现有 Rust 后端逻辑，MCP 层只是换一个传输协议

### 架构

```
RealityMod Data Layer (JSON/Markdown in data/)
        │
   Rust 核心逻辑层（读写、校验、计算）
        │
   ┌────┼────────────┐
   ▼    ▼            ▼
Tauri  MCP Server   (未来: HTTP API)
IPC     │
 │      ├── Nanobot → IM (飞书/QQ/Telegram/Discord/Slack/WhatsApp)
 │      ├── Claude Code / Cursor / Windsurf（开发调试）
 │      └── 其他 MCP 兼容客户端
 ▼
原生 P5 UI (Svelte)
```

### MCP Tools 设计

| Tool | 用途 | 对应需求 |
|------|------|----------|
| `get_status` | 读取角色状态 | NPC 了解用户 |
| `update_status` | 自然语言更新状态 | 自然语言录入 |
| `list_achievements` | 查询成就进度 | NPC 分析进展 |
| `unlock_achievement` | 解锁成就 | Agent 确认完成 |
| `create_achievement_pack` | 生成成就包 | 智能生成 |
| `query_skills` | 查询技能树状态 | 了解能力分布 |
| `search_recipes` | 搜索食谱 | 附带功能 |
| `get_gallery` | 查询图鉴 | 附带功能 |

## Agent 平台调研（2026.03）

### 个人 AI Agent 平台

| 平台 | 特点 | IM 支持 | 成本 |
|------|------|---------|------|
| **OpenClaw** | 最火开源 agent，100+ skills，生态最大 | WhatsApp, Telegram, Signal, Discord, Slack, iMessage, Teams, 微信（腾讯适配） | 自托管免费，API token $300-750/月 |
| **Nanobot** | 港大出品，极轻量（4000行），基于 MCP | Telegram, Discord, Slack, WhatsApp, 飞书, QQ | 自托管免费，API $5-50/月 |
| **ZeroClaw** | 性能优先的 OpenClaw 替代 | 类似 OpenClaw | 自托管免费 |
| **NanoClaw** | 安全优先方案 | 类似 | 自托管免费 |

### 推荐：Nanobot

- 本身是 MCP host，写 MCP Server 即获得全部 IM 支持
- 轻量、Python、易部署
- 支持飞书/QQ/Telegram，国内友好
- API 费用可控

### 注意事项

- OpenClaw 有多个 CVE，17% 社区 skill 被标记恶意
- Nanobot 生态较小但安全
- 腾讯已基于 OpenClaw 做微信适配（2026.03）

## 实施步骤

1. **MCP Server**（Rust）— 包装现有 Tauri commands 为 MCP tools
2. **接入 Nanobot** — 验证 IM 场景（飞书/QQ/Telegram）
3. **原生对话 UI** — Svelte 端加 P5 风格 NPC 对话框，调 Claude API

## 参考链接

- [OpenClaw vs Nanobot - DataCamp](https://www.datacamp.com/blog/openclaw-vs-nanobot)
- [What is NanoBot? - Medium](https://medium.com/data-science-in-your-pocket/what-is-nanobot-ultra-lightweight-ai-agent-framework-c43ad6c40b11)
- [OpenClaw Explained - KDnuggets](https://www.kdnuggets.com/openclaw-explained-the-free-ai-agent-tool-going-viral-already-in-2026)
- [Agent Skills Guide 2026](https://serenitiesai.com/articles/agent-skills-guide-2026)
