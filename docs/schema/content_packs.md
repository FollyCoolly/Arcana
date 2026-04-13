# Content Packs Schema

内容包（Content Pack）是 Arcana 的可插拔数据单元，每个包包含成就定义、技能树定义和包元数据。

## 文件路径

- `data/loaded_packs.json`：已加载的内容包列表
- `data/packs/<pack_id>/manifest.json`：包元数据
- `data/packs/<pack_id>/achievements.json`：成就定义
- `data/packs/<pack_id>/skills.json`：技能树定义

## 缺省值约定

- 可选字段省略，不写 `null`。
- `tags`、`prerequisites`、`required_key_achievements` 等数组字段省略时视为 `[]`。

## ID 命名规则

成就 ID 和技能 ID 均使用 `<pack_id>::<name>` 格式：

- 前缀必须等于 `manifest.id`
- `name` 部分使用 snake_case
- 例：`programmer::hello_world`、`fitness::5k_run`

跨包 ID 唯一性由前缀保证。

## `loaded_packs.json`

### 结构

```json
{
  "version": 1,
  "packs": []
}
```

### 字段

- `version` (`number`, 必填)：Schema 版本号
- `packs` (`string[]`, 必填)：已加载的包 ID 列表，每个 ID 对应 `data/packs/` 下的目录名

### 示例

```json
{
  "version": 1,
  "packs": ["programmer", "fitness"]
}
```

## `manifest.json`（每包）

### 结构

```json
{
  "id": "pack_id",
  "name": "Display Name",
  "description": "Pack description.",
  "version": "1.0.0",
  "author": "Author",
  "tags": []
}
```

### 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 包 ID，必须等于目录名 |
| `name` | string | 是 | 显示名称 |
| `description` | string | 是 | 简短描述 |
| `version` | string | 是 | 语义化版本号（如 `1.0.0`） |
| `author` | string | 是 | 作者 |
| `tags` | string[] | 否 | 标签，用于分类筛选 |

### 示例

```json
{
  "id": "programmer",
  "name": "Programmer",
  "description": "Achievements and skills for software developers.",
  "version": "1.0.0",
  "author": "Arcana",
  "tags": ["career", "tech"]
}
```

## 校验规则

1. `manifest.id` 必须等于包所在的目录名
2. 包内所有成就 ID 和技能 ID 必须以 `<manifest.id>::` 开头
3. `loaded_packs.json` 中的每个 ID 必须对应 `data/packs/` 下的有效目录
4. 跨包 ID 无重复（由前缀机制保证）

## 卸载行为

卸载包时：
- 从 `loaded_packs.json` 移除包 ID
- `achievement_progress.json` 中的解锁记录保留
- 重新加载包时，已有解锁状态自动恢复
