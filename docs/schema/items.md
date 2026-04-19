# Items Schema

Items 模块管理个人物品清单。Rust 直读 Obsidian MD 文件（解析 YAML frontmatter），Obsidian 为唯一数据源。

## `item_sources.json`

每个物品子目录对应一个 source 条目。

```json
{
  "version": 1,
  "sources": [
    { "id": "衣物",     "name": "衣物",     "path": "D:/Documents/obsidian/Labyrinth/物品/衣物" },
    { "id": "鞋子",     "name": "鞋子",     "path": "D:/Documents/obsidian/Labyrinth/物品/鞋子" },
    { "id": "配饰",     "name": "配饰",     "path": "D:/Documents/obsidian/Labyrinth/物品/配饰" },
    { "id": "电子产品", "name": "电子产品", "path": "D:/Documents/obsidian/Labyrinth/物品/电子产品" },
    { "id": "生活电器", "name": "生活电器", "path": "D:/Documents/obsidian/Labyrinth/物品/生活电器" },
    { "id": "手办",     "name": "手办",     "path": "D:/Documents/obsidian/Labyrinth/物品/手办" },
    { "id": "家具",     "name": "家具",     "path": "D:/Documents/obsidian/Labyrinth/物品/家具" },
    { "id": "实体书",   "name": "实体书",   "path": "D:/Documents/obsidian/Labyrinth/物品/实体书" },
    { "id": "专辑",     "name": "专辑",     "path": "D:/Documents/obsidian/Labyrinth/物品/专辑" }
  ]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 源唯一标识，用于生成物品 ID |
| `name` | string | 是 | 显示名称 |
| `path` | string | 是 | Obsidian 目录绝对路径 |

## Obsidian MD 文件格式

每个 `.md` 文件 = 一件物品。文件名（去 `.md`）即物品名，YAML frontmatter 包含元数据，正文可含 `![[image.png]]` 图片引用。名为 `*总览.md` 的索引文件会被自动跳过（解析时忽略无 frontmatter 的文件）。

### 公共字段（从 frontmatter 提取）

| frontmatter 键 | 内部字段 | 类型 | 必填 | 说明 |
|----------------|----------|------|------|------|
| `名称` | name | string | 否 | 物品名称，缺省取文件名（去 .md） |
| `品牌` | brand | string | 否 | 品牌 |
| `价格` | price | number | 否 | 购入价格（元） |
| `购入日期` | purchase_date | string | 否 | YYYY-MM-DD |
| `购入方式` | purchase_channel | string | 否 | 购入渠道 |
| `类别` | category | string | 否 | 物品类别（通常与所在目录对应） |
| `颜色` | color | string | 否 | 颜色 |

### 类别专属字段（进入 extra）

frontmatter 中不在公共字段列表内的键值对全部存入 `extra: Map<String, Value>`。常见的类别专属字段：

| 类别 | 典型 extra 字段 |
|------|----------------|
| 衣物 | 主类、品类、长度、季节、风格、材质、图案 |
| 鞋子 | 型号、尺码 |
| 配饰 | 子类、型号 |
| 电子产品 | 型号、尺寸、芯片、内存、存储容量 |
| 生活电器 | 型号 |
| 手办 | 作品、角色、比例 |
| 实体书 | 作者、出版社、阅读状态 |
| 专辑 | 艺术家、发行年份、介质 |
| 家具 | 型号 |

### 图片

从 MD 正文解析 `![[filename.ext]]`，取第一个匹配作为 `image`，路径为绝对路径（相对于 .md 文件所在目录解析）。

## 计算字段

| 字段 | 公式 | 说明 |
|------|------|------|
| `days_owned` | `today - purchase_date` | 拥有天数，无日期则 null |
| `daily_cost` | `price / max(days_owned, 1)` | 日均花费，无价格或日期则 null |

## 返回数据结构 (ItemData)

```
ItemData {
  sources: [ItemSourceInfo]
  items: [ItemWithComputed]
  stats: ItemStats
}

ItemSourceInfo {
  id: string
  name: string
  item_count: number
}

ItemWithComputed {
  id: string              // "<source_id>::<filename_stem>"
  source_id: string
  name: string
  brand: string | null
  price: number | null
  purchase_date: string | null
  purchase_channel: string | null
  category: string | null // 来自 frontmatter `类别`
  color: string | null
  image: string | null    // 绝对路径
  extra: Map<string, Value>
  days_owned: number | null
  daily_cost: number | null
}

ItemStats {
  total_items: number
  total_value: number         // 所有有价格物品之和
  average_daily_cost: number  // 所有有值 daily_cost 的均值
  by_source: [SourceStats]    // 按数据源统计
  by_category: [CategoryStats] // 按 frontmatter `类别` 字段统计，按数量降序
}

SourceStats {
  source_id: string
  source_name: string
  item_count: number
  total_value: number
}

CategoryStats {
  name: string       // 与 frontmatter `类别` 值一致
  item_count: number
  total_value: number
}
```
