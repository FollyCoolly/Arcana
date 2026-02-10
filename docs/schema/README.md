# Schema 文档总览

本目录用于维护 Reality Mod 的数据结构规范文档，重点描述本地 JSON 文件的字段定义、约束和演进方式。

## 目标

- 统一各模块数据格式，避免前后端理解偏差。
- 为 Rust 数据结构和前端 TypeScript 类型提供单一依据。
- 支持后续字段扩展与版本迁移。

## 文档约定

- 模块文档命名：`<module>.md`，例如 `status.md`。
- 每个文档建议包含：文件路径、字段定义、最小示例、校验规则、版本说明。
- 日期格式默认使用 `YYYY-MM-DD`。
- 时间戳如需精确时间，使用 ISO 8601（例如 `2026-02-10T20:15:00+08:00`）。

## 全局基础 Schema

### `user_profile.json`

用途：存储用户基础信息（身份相关，低频变更）。

建议路径：`data/user_profile.json`

最小示例：

```json
{
  "username": "User01",
  "birth_date": "1998-01-01"
}
```

字段说明：

- `username` (`string`, 必填)：显示名或用户名。
- `birth_date` (`string`, 必填)：出生日期，格式 `YYYY-MM-DD`。

## 模块 Schema 索引

- `app_settings.md`：应用行为配置（启用模块、主题、首页等）。
- `status.md`：身体与生活状态数据（体重、训练、配速等）。
- `achievements.md`：成就定义与解锁状态。
- `skills.md`：技能树节点、积分与等级计算输入。
- `items.md`：物品记录与统计字段。
- `gallery.md`：阅读/观影/游戏媒体聚合数据。
- `crafting.md`：配方与材料结构。

## 下一步建议

1. 先补 `app_settings.md` 与 `status.md`，用于支撑 MVP 开发。
2. 同步在 Rust 端定义对应结构体并加基础反序列化校验。
3. 前端按 Schema 生成对应 TypeScript 类型，避免手写漂移。
