# External-source schemas

核心 Record、Pack、Status、Achievement、Skill、Mission 与 AssistantMemory 合约已经统一到 [`docs/design/`](../design/README.md)，不再维护旧 JSON v1 Schema。

本目录只保留未纳入核心数据平台的外部适配器格式：

- [`items.md`](./items.md)：`item_sources.json` 与 Obsidian/Markdown 物品格式。

Gallery 与 Weather 当前只有简单配置示例，见 `data-example/gallery_sources.json` 和 `data-example/weather.json`。
