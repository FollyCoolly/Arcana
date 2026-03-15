# Items Schema

Items 模块管理个人物品清单。Rust 直读 Obsidian MD 文件（解析 YAML frontmatter），Obsidian 为唯一数据源。

## `item_sources.json`

```json
{
  "version": 1,
  "sources": [
    {
      "id": "clothing",
      "name": "衣柜",
      "path": "D:/Documents/obsidian/Labyrinth/衣柜",
      "icon": "👕"
    }
  ]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 源唯一标识，用于生成物品 ID |
| `name` | string | 是 | 显示名称 |
| `path` | string | 是 | Obsidian 目录绝对路径 |
| `icon` | string | 否 | emoji 图标，缺省 "📦" |

## Obsidian MD 文件格式

每个 `.md` 文件 = 一件物品。YAML frontmatter 包含元数据，正文可含 `![[image.png]]` 图片引用。

### 公共字段（从 frontmatter 提取）

| frontmatter 键 | 内部字段 | 类型 | 必填 | 说明 |
|----------------|----------|------|------|------|
| `名称` | name | string | 是 | 物品名称，缺省取文件名（去 .md） |
| `品牌` | brand | string | 否 | 品牌 |
| `价格` | price | number | 否 | 购入价格（元） |
| `购入日期` | purchase_date | string | 否 | YYYY-MM-DD |
| `购入方式` | purchase_channel | string | 否 | 购入渠道 |
| `主类` | main_category | string | 否 | 主分类 |
| `品类` | sub_category | string | 否 | 子分类 |
| `颜色` | color | string | 否 | 颜色 |

### 自由属性

frontmatter 中不在公共字段列表内的键值对，全部存入 `extra: Map<String, Value>`。

示例：衣物的 `长度`、`季节`、`风格`、`材质`、`图案` 会进入 extra。

### 图片

从 MD 正文解析 `![[filename.ext]]`，取第一个匹配作为 `image`，路径相对于 .md 文件所在目录。

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
  icon: string
  item_count: number
}

ItemWithComputed {
  id: string           // "<source_id>::<filename_stem>"
  source_id: string
  name: string
  brand: string | null
  price: number | null
  purchase_date: string | null
  purchase_channel: string | null
  main_category: string | null
  sub_category: string | null
  color: string | null
  image: string | null  // 绝对路径
  extra: Map<string, Value>
  days_owned: number | null
  daily_cost: number | null
}

ItemStats {
  total_items: number
  total_value: number         // sum of all prices (有价格的)
  average_daily_cost: number  // avg of all daily_costs (有值的)
  by_source: [SourceStats]
  by_main_category: [CategoryStats]
}

SourceStats {
  source_id: string
  source_name: string
  source_icon: string
  item_count: number
  total_value: number
}

CategoryStats {
  name: string
  item_count: number
  total_value: number
}
```
