# Crafting Schema

Crafting 模块管理个人收集的制作配方（Recipe Book）。Rust 直读 Obsidian MD 文件（解析 YAML frontmatter），Obsidian 为唯一数据源。Schema 抽象为通用的"材料 + 步骤"模型，当前以菜谱为主。

## `recipe_sources.json`

```json
{
  "version": 1,
  "sources": [
    {
      "id": "cooking",
      "name": "菜谱",
      "path": "D:/Documents/obsidian/vault/菜谱",
      "icon": "🍳"
    }
  ]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 源唯一标识，用于生成配方 ID |
| `name` | string | 是 | 显示名称 |
| `path` | string | 是 | Obsidian 目录绝对路径 |
| `icon` | string | 否 | emoji 图标，缺省 "📖" |

## Obsidian MD 文件格式

每个 `.md` 文件 = 一个配方。YAML frontmatter 包含元数据，正文可含 `![[image.png]]` 图片引用。

### 公共字段（从 frontmatter 提取，中英文键均支持）

| 中文键 | 英文键 | 内部字段 | 类型 | 必填 | 说明 |
|--------|--------|----------|------|------|------|
| `名称` | `name` | name | string | 否 | 配方名称，缺省取文件名 |
| `类型` | `type` | recipe_type | string | 否 | 分类（如：主菜、甜点、手工） |
| `份数` | `servings` | servings | string | 否 | 份量（如"4人份"、"1个"） |
| `难度` | `difficulty` | difficulty | string | 否 | 难度（自由文本） |
| `时间` | `time` | time | string | 否 | 所需时间（如"90分钟"） |
| `材料` | `ingredients` | ingredients | string[] | 否 | 材料列表，纯文本 |
| `步骤` | `steps` | steps | string[] | 否 | 步骤列表，纯文本 |
| `标签` | `tags` | tags | string[] | 否 | 标签 |
| `来源` | `source` | source | string | 否 | 配方出处 |

### 自由属性

frontmatter 中不在公共字段列表内的键值对，全部存入 `extra: Map<String, Value>`。

### 图片

从 MD 正文解析 `![[filename.ext]]`，取第一个匹配作为 `image`，路径相对于 .md 文件所在目录。

## 计算字段

| 字段 | 公式 | 说明 |
|------|------|------|
| `ingredient_count` | `ingredients.len()` | 材料数量 |
| `step_count` | `steps.len()` | 步骤数量 |

## 返回数据结构 (CraftingData)

```
CraftingData {
  sources: [RecipeSourceInfo]
  recipes: [RecipeWithComputed]
  stats: RecipeStats
}

RecipeSourceInfo {
  id: string
  name: string
  icon: string
  recipe_count: number
}

RecipeWithComputed {
  id: string              // "<source_id>::<filename_stem>"
  source_id: string
  name: string
  recipe_type: string | null
  servings: string | null
  difficulty: string | null
  time: string | null
  ingredients: string[]
  steps: string[]
  tags: string[]
  source: string | null
  image: string | null    // 绝对路径
  extra: Map<string, Value>
  ingredient_count: number
  step_count: number
}

RecipeStats {
  total_recipes: number
  by_source: [RecipeSourceStats]
  by_type: [RecipeTypeStats]
}

RecipeSourceStats {
  source_id: string
  source_name: string
  source_icon: string
  recipe_count: number
}

RecipeTypeStats {
  name: string
  recipe_count: number
}
```
