# 数据平台、所有权与 JSON

> **状态**：Current
> **最后更新**：2026-08-17

## 1. 为什么不是“所有东西都进 SQLite”

Arcana 的内容定义与大部分用户状态需要便于阅读、手动编辑和用 Git 同步；Records 则需要高频增量更新、集合/事件修正与可靠并发。因此存储按需求分工，而不是按旧子模块各建一套数据层。

| 数据 | 权威源 | 跨设备 |
| --- | --- | --- |
| Pack manifest、Definitions、assets | live JSON repository | 是 |
| enabled Pack IDs | `arcana.json` | 是 |
| AchievementState | `achievement-states/<pack_id>.json` | 是 |
| 已接受 Mission | `missions.json` | 是 |
| AssistantMemory | `assistant-memory.json` | 是 |
| Record | SQLite | 通过导入/导出的 `records/*.json` 投影 |
| MissionSuggestion | runtime `local-state.json` | 否 |
| Status 五项选择、Mission Dashboard 槽位 | runtime `local-state.json` | 否 |
| Items/Gallery/Weather | 外部来源 | 不由核心层负责 |

## 2. Live repository 布局

```text
arcana.json
achievement-states/                 # 可选；用户状态按 Pack 分文件
└── <pack_id>.json
assistant-memory.json               # 可选
missions.json                       # 可选
records/                            # 完整 export/import 时使用
└── <namespace>.json
packs/
└── <pack_id>/
    ├── manifest.json
    ├── record-definitions.json     # 可选
    ├── derived-values.json         # 可选，Pack schema v2
    ├── dimensions.json             # 可选
    ├── achievements.json           # 可选
    ├── skills.json                 # 可选
    └── assets/                     # 可选
```

普通运行时直接读取根级语义文件和 `packs/`，但忽略 live repository 中可能存在的 `records/`；Record 的运行时权威源始终是 SQLite。完整 `json export` 才把 SQLite Records 规范化写入 `records/`，`json import` 才反向写回 SQLite。

`arcana.json` 只保存 repository Schema 与 enabled Pack：

```json
{
  "schema_version": 2,
  "enabled_pack_ids": ["basic"]
}
```

不使用通用 Envelope，不保存 Profile、导出时间、本机路径、SQLite row ID、缓存或凭证。空的可选实体文件省略。

AchievementState 在领域模型中仍是一份全局用户状态集合，保证每个 Achievement 最多一条状态；JSON 仅为降低人工浏览和 Git 冲突范围，按 Achievement ID 的 Pack 前缀拆成 `achievement-states/<pack_id>.json`。状态不嵌入 `packs/`，因为 Pack 是可替换的定义，而 AchievementState 是用户数据。即使对应 Pack 已删除，其文件和 unresolved 状态仍保留。

## 3. 人工编辑

允许用户手动编辑 JSON。下一次命令会重新解析并校验整个语义 repository，包括：Schema、ID、排序前的语义唯一性、Pack 引用、Definition 兼容、Status 表达式和 asset 路径。

Arcana 不猜测错误，也不自动修正 Git conflict marker。错误会阻止相关运行时读取或写入，用户修改文件后重试。为避免覆盖，用户不应在 Arcana 命令正在写入时同时编辑同一文件；更完整的外部修改 digest/journal 尚未实现。

## 4. 确定性 JSON Codec

Codec 负责：

- 稳定字段与数组顺序、UTF-8、两个空格缩进和文件末尾换行；
- 空可选文件省略；
- Records 按 Definition namespace 聚合；
- Pack assets 保持原始 bytes；
- 写入后的语义 round-trip 校验；
- 完整 import 时把 semantic entities 路由到 live JSON、Records 路由到 SQLite，并保留本机状态。

`json export --output` 只写不存在的新目录，不覆盖现有目录。Codec 本身不执行 Git 命令，也不生成 commit。

## 5. 初始化与升级

一个 repository 对应一个用户，不建立 Profile。新运行时创建 Pack schema v2 的 `basic` Pack；昵称和生日是可选 Records，游戏天数是由生日计算且不持久化的 DerivedValue。缺少昵称时 UI 使用默认值，缺少生日时游戏天数为缺失。

当前代码支持从已实现过的 SQLite Schema v1 升级：升级前读取其中的 Pack/semantic/local entities，写入新 JSON store，再执行 Schema v2 migration 删除非 Record 表。旧版应用的其他 JSON 格式按此前决策完全抛弃，不提供转换。

运行时会把内容与旧内置 `basic` Pack 完全一致的 schema v1 副本升级为当前 schema v2，以补充 `identity.game_days`；只要用户改过该 Pack 的任意内容就不自动覆盖，需通过 Pack 工具显式升级。

## 6. 删除、历史与安全

- 使用普通 hard delete；不建立 tombstone 或永久 op log。
- Achievement 不因相关 Record 改变自动撤销，但允许显式撤销误操作。
- 删除/停用 Pack 不自动删除已存在的 Record 或 AchievementState；它们可变为 unresolved 并显式报告。
- Git commit 作为粗粒度历史。个人数据仓库应使用 private remote；认证只交给系统 Git credential manager。
- 当前不实现 CRDT、字段级自动合并或冲突自动选择。

Git 闭环的当前范围见 [sync_migration.md](./sync_migration.md)，SQLite 细节见 [sqlite_storage.md](./sqlite_storage.md)。
