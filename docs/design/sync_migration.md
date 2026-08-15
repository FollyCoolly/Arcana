# Git 同步、版本与数据库初始化协议

> **状态**：Target / 实现协议已确定
> **最后更新**：2026-08-15

## 1. 同步仓库布局

```text
.gitattributes                      # 必需；固定 JSON LF 与 asset binary
arcana.json                         # 必需；仓库 Schema 与 enabled Pack
achievement-states.json             # 可选；用户 Achievement 状态
assistant-memory.json               # 可选；长期语义记忆
missions.json                       # 可选；已接受 Mission
records/
└── <namespace>.json                # 可选；按 namespace 聚合的用户 Record
packs/
└── <pack_id>/
    ├── manifest.json               # 必需
    ├── record-definitions.json     # 可选
    ├── dimensions.json             # 可选
    ├── achievements.json           # 可选
    ├── skills.json                 # 可选
    └── assets/                     # 可选
```

- `arcana.json.schema_version` 解释根级用户数据和 `records/`。
- 每个 `manifest.json.schema_version` 解释对应 Pack 的全部内容文件。
- 各实体文件不重复版本字段，也不使用通用 Envelope。
- 根目录未知 JSON、`records/` 中不符合命名规则的文件、Pack 中未知 JSON 都是校验错误，防止拼写错误被忽略。
- Arcana managed paths 仅包括固定的 `.gitattributes`、根级实体 JSON、`records/*.json`、规定的 Pack JSON 文件以及 `packs/<pack_id>/assets/**`。README、许可证等其他文件不属于 managed paths，导入忽略、导出不修改。
- `assets/` 只接受普通文件，不接受符号链接；asset 的每个路径 segment 只使用小写 ASCII 字母、数字、`.`、`_`、`-`，不得以点或空格结尾，也不得使用 Windows 保留名。导入同时拒绝 Unicode normalization 或大小写折叠后冲突的路径。
- 同步 Repository Codec 把 asset 原始 bytes 导入 `pack_assets`，再按规范相对路径原样导出，不做转码。
- 空的可选实体文件不导出；必定存在的文件是 `.gitattributes`、`arcana.json` 和每个 Pack 的 `manifest.json`。

所有规范 JSON 固定使用 UTF-8（无 BOM）、LF、两个空格缩进和一个文件末尾换行。固定字段按各 Schema 示例顺序输出，动态 object key 和要求排序的 array 按 Unicode code point 升序；非 ASCII 文本不转成 `\u` 转义，除 JSON 必需字符外不增加转义。整数不输出小数点，有限浮点数使用可 round-trip 的最短十进制表示，负零规范为 `0`。因此同一 Domain snapshot 在 Windows 与 macOS 上必须产生逐字节相同的 managed JSON。

`.gitattributes` 是 Codec 生成并校验的固定平台文件，内容为：

```gitattributes
*.json text eol=lf
packs/**/assets/** -text
```

这防止 Git 的 `core.autocrlf` 在不同设备改写 JSON 换行，并把 Pack asset 始终视为原始 bytes。内容缺失或被修改时 import 报配置错误，不静默继承设备级 Git 设置。

### 1.1 新仓库最小内容

新建用户仓库时先写入上述固定 `.gitattributes`，再写入根清单：

```json
{
  "schema_version": 1,
  "enabled_pack_ids": ["basic"]
}
```

并写入普通的标准 `basic` Pack：

```json
{
  "schema_version": 1,
  "id": "basic",
  "name": "基础"
}
```

```json
{
  "definitions": [
    {
      "id": "identity.birth_date",
      "name": "生日",
      "kind": "scalar",
      "value_type": "date"
    },
    {
      "id": "identity.nickname",
      "name": "昵称",
      "kind": "scalar",
      "value_type": "string"
    }
  ]
}
```

后两个文件分别是 `packs/basic/manifest.json` 与 `packs/basic/record-definitions.json`。没有用户值时不创建 `records/identity.json`。`basic` 没有隐藏特权，之后可以像普通 Pack 一样编辑展示元数据或停用；两个标准 Definition 的破坏性修改仍必须改用新 ID。

新仓库入口为：

```text
arcana-data init --repo <repository_dir> [--runtime <runtime_dir>]
```

命令只在目标没有 Arcana managed paths 时执行；已有普通 README 等文件可以保留。目录还不是 Git working tree 时执行本地 `git init`，随后写入上述最小内容、创建 SQLite 并完成首次 round-trip 校验；全部成功后才把 repository/runtime 路径写入本机配置。它不创建远端、不 commit、不 push；remote、认证和默认 branch 继续使用普通 Git 工具配置。

## 2. Schema 版本

- 第一版仓库和 Pack Schema 均为整数 `1`。
- 应用遇到高于自身支持范围的版本必须拒绝导入，不能尽力猜测。
- 第一版只接受精确的仓库/Pack Schema `1`。其他版本一律拒绝导入；未来若增加同步格式升级，必须作为独立、显式且经过 round-trip 验证的 Codec 实现。
- Pack 不保存内容发布版本；个人内容历史由 Git 表达。
- 运行时 SQLite 使用应用内置、不可修改的有序 migration，不复用同步 JSON 的 Schema 版本。

## 3. 本机运行时布局与进程间锁

同步仓库与本机运行时目录相互独立。本机 `~/.arcana/settings.json` 保存数据平台路径，不进入 Git：

```json
{
  "repository_dir": "/absolute/path/to/arcana-user-data",
  "runtime_dir": "/absolute/path/to/arcana-runtime"
}
```

两个值都必须是解析后的绝对目录路径；Windows JSON 路径中的反斜杠按 JSON 规则转义。`repository_dir` 在 init 成功前可以缺失，`runtime_dir` 缺失时默认使用 `~/.arcana/runtime`。配置不保存 user ID、Git credential、branch、remote 或同步 revision；这些分别由“一仓库一用户”、系统 Git 和 SQLite `sync_state` 表达。数据平台不得删除同一 settings 文件中其他模块拥有的本机配置 key。

运行时布局固定为：

```text
<runtime_dir>/
├── arcana.sqlite3
├── arcana.sqlite3-wal             # SQLite 按需创建
├── arcana.sqlite3-shm             # SQLite 按需创建
├── arcana.lock
├── sync-export-journal.json       # 仅 export 进行中或恢复时存在
├── sync-export-work/              # 仅 export 进行中或恢复时存在
└── migration-backups/
    └── <backup_id>/
```

这些文件一律不进入用户同步仓库。`arcana.lock` 必须使用操作系统 advisory file lock，不能把“锁文件存在”当作已加锁；进程崩溃后 OS 会自动释放。

- 普通 read/write command 获取 shared lock。SQLite WAL 继续负责普通并发事务。
- Git import/export、数据库替换、SQLite migration 和首次初始化获取 exclusive lock。
- exclusive lock 持有期间禁止新命令打开或访问活动数据库；这是 Windows 上安全替换数据库文件的必要条件。
- 普通命令默认等待 5 秒后报告 busy；显式 sync/migrate 命令可以显示等待状态并允许用户取消。
- 所有 UI、CLI 和 Agent Skill 入口使用同一锁实现，不能各自创建锁协议。

## 4. SQLite 与工作区变更检测

本机 `sync_state` 保存：

- `repository_digest`：上次成功 import/export 后，将全部 managed path 的规范相对路径与原始 bytes（包括 asset）按路径排序后计算的 SHA-256；
- `data_revision`：每次同步实体事务成功后递增；
- `exported_revision`：最近一次成功 export 对应的 revision。

本机 Status/Dashboard 选择和 MissionSuggestion 不递增 `data_revision`，因为它们不进入 Git。

由此区分四种状态：

| Git managed files | SQLite | 行为 |
| --- | --- | --- |
| 未变 | 未变 | clean |
| 已变 | 未变 | 可以 import 人工编辑 |
| 未变 | 已变 | 可以 export SQLite |
| 已变 | 已变 | `both_changed`，拒绝覆盖，必须由用户先选择/整理一侧 |

文件 digest 只用于本机防覆盖，不进入同步仓库。格式调整也算 Git 一侧变化；成功 import 后下一次 export 会恢复规范格式。

## 5. Import

`arcana-data sync import`：

1. 获取 exclusive lock。
2. 若 SQLite 有未 export 的同步实体修改，拒绝 import。
3. 确认目标是 Git working tree，检查 Git index 没有 unmerged entry，再检查 managed path 和 Schema 版本。残留在 JSON 结构中的文本冲突标记会自然触发解析/Schema 错误，不使用可能误伤合法字符串的全文件关键字扫描。
4. 解析完整仓库，校验 ID、引用、DAG、表达式和 Pack 兼容性。
5. 在同目录创建随机命名的临时 SQLite 数据库并执行全部 migration。
6. 写入同步实体；从旧活动数据库复制有效的本机选择、Dashboard slot 和 MissionSuggestion。
7. 若新 Mission 已使用某 Suggestion ID，不复制该 Suggestion。
8. 执行 `foreign_key_check`、`integrity_check` 和 Domain 全量校验。
9. 从临时数据库导出到另一临时目录，比较实体数量、ID、引用和规范化语义。
10. 关闭全部连接并 checkpoint WAL，原子替换活动数据库；失败时保留旧数据库。
11. 写入新的 digest，并令 `data_revision == exported_revision`。

找不到 RecordDefinition 的 Record 和找不到 AchievementDefinition 的用户状态允许作为 unresolved 保留；其他损坏引用按各领域 Schema 报错。

## 6. Export

`arcana-data sync export`：

1. 获取 exclusive lock。
2. 重新计算 managed file digest；若与 `sync_state.repository_digest` 不同则拒绝覆盖，并要求先 import/处理人工编辑。
3. 在一致性 SQLite read transaction 中构建完整 Domain snapshot。
4. 将全部 managed JSON 与 Pack asset 渲染到仓库外的临时目录。
5. 对临时输出执行完整 import 与 round-trip 校验。
6. 替换前把所有将被覆盖或删除的旧文件复制到本机 `sync-export-work/<operation_id>/old/`，并用 `sync-export-journal.json` 记录本次 snapshot digest、旧文件 digest、待替换/待删除 managed paths 和每项状态。逐项使用仓库同目录临时文件 + atomic rename 更新；journal 本身也以临时文件原子写入。崩溃后下次启动必须先在 exclusive lock 下完成全部剩余替换，若临时输出不完整则用 `old/` 全量恢复，不能在部分新、部分旧的仓库上继续工作。
7. 删除已经变空的可选 managed 文件，但绝不删除非 managed 文件。
8. 成功后更新 digest，并令 `exported_revision = data_revision`。

成功完成并持久化 `sync_state` 后删除 journal 和对应 work directory。它们只是短期崩溃恢复材料，不是 operation log；恢复失败时保留现场并报告，不能猜测删除。

export 不自动 `git add`、commit 或 push；这些属于显式 Git sync 命令。

## 7. Git 命令

- `sync status`：显示 SQLite dirty、managed worktree dirty、Git ahead/behind/diverged 和 unresolved 实体数量。
- `sync pull`：要求 SQLite clean、managed worktree clean；只执行 fetch + fast-forward，然后 import。无法 fast-forward 时停止。
- `sync push`：必要时先 export；只 stage Arcana managed paths，创建 `arcana: sync <RFC3339 timestamp>` commit，然后 push。不得纳入用户的 README、其他文件或已经 staged 的无关改动；实现必须使用 path-limited commit 或隔离的临时 index，并保留原 index 状态。
- `sync`：按 status 选择安全的 export、commit、fetch、fast-forward/import 或 push；绝不自动 merge、rebase 或解决冲突。

push 被拒绝或本地/远端 diverged 时保留本地 commit，并给出普通 Git 状态。用户使用 Git 工具解决后运行 `sync import`；应用不猜测字段级合并结果。

## 8. SQLite migration runner

- 应用必须捆绑 SQLite `>= 3.43.0`，不依赖操作系统 SQLite；目标 Schema 使用 JSON 函数、STRICT table 和 deferred foreign key。
- migration 以连续正整数编号，编译进应用，每个 migration 保存稳定名称和 SQL 内容 SHA-256。
- 已发布 migration 永不修改；`schema_migrations` 中 checksum 不匹配时拒绝启动并报告安装损坏。
- 执行 pending migration 前复制数据库、`-wal` 和 `-shm`（checkpoint 后）到带时间戳的本机备份目录。
- 每个 migration 在事务中执行；完成后运行 foreign key/integrity check，再记录 version、name、checksum、applied_at。
- migration 失败回滚事务并继续使用原数据库，不自动删除备份。
- 数据库设置固定 `PRAGMA application_id = 0x41524341`（ASCII `ARCA`）；打开其他 application ID 的数据库时拒绝写入。

## 9. 新数据平台初始化与切换

新系统没有旧 JSON 导入器，也不提供 `migrate plan/apply/rollback` 命令。首次使用只允许两条路径：

- `arcana-data init` 创建全新的 v1 Git 仓库、标准 `basic` Pack 和空 SQLite；
- 已存在的有效 v1 同步仓库通过 `sync import` 创建新的本机 SQLite。

初始化必须先在临时位置生成仓库内容和 SQLite，完成 Domain 校验、`foreign_key_check`、`integrity_check` 与 JSON → SQLite → JSON round-trip 后，再写入本机路径配置。目标已有 Arcana managed paths 时，`init` 拒绝覆盖并要求使用 `sync import`。

旧版应用 JSON 不参与探测、映射、备份或验证。UI/CLI/Skill 切换到新 Repository 后，旧 `data_dir` 不再是 Arcana 数据源；旧文件不会被新系统修改或删除，用户如需保留只能按普通文件自行归档。

## 10. 验证标准

- 所有目标 JSON、Pack asset、Domain Model 和 SQLite Schema 校验通过；
- managed files 重复 export 字节完全相同；
- JSON → SQLite → JSON 规范化语义等价；
- 实体计数、稳定 ID、引用、用户状态集合以及 asset 路径/bytes digest 一致；
- unresolved Record/Achievement 状态逐项报告，不能静默丢弃；
- 本机选择、Dashboard slot 与 MissionSuggestion 不得意外进入同步 snapshot。

## 11. 备份与失败恢复

- 新数据库和同步输出始终先写临时路径，通过验证后再切换活动指针。
- 对已有 Arcana SQLite 执行未来 pending migration 前，checkpoint 并备份数据库、`-wal`、`-shm` 和当前 migration checksum 清单。
- SQLite migration 失败时回滚事务并继续使用原数据库；Git export 中断时按 export journal 完成或恢复整组 managed files。
- 恢复流程不自动 reset、rebase 或删除 Git commit；Git 历史仍由用户按普通 Git 工具处理。
- 任何长期备份清理由用户显式执行，不设置自动过期删除。

## 12. RecordDefinition 内容演进

第一版不提供表达式式或脚本式内容 migration：

- 兼容的可选字段增加可直接应用。
- kind、类型、required、单位或字段删除等破坏性变化必须创建新的 Definition ID。
- 用户或 Agent 通过 typed command 明确写入新 Record、更新 Achievement/Dimension 引用，并在确认后删除旧 Record/Definition。
- 应用可以提供 diff 和受影响引用列表，但不能自动转换数值、单位或业务语义。

这与 SQLite Schema migration 是两个独立问题：前者属于用户内容，后者属于应用内部数据库格式。
