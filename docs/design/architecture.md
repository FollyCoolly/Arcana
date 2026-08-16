# 数据平台架构

> **状态**：Current
> **最后更新**：2026-08-16

## 1. 分层

```text
Svelte UI / Arcana Skills
          |
Tauri IPC / arcana-data CLI
          |
Application Commands
          |
Domain Models + ArcanaRepository traits
          |
DataRepository
  |             |                 |
live JSON    Record SQLite    runtime-local JSON
```

依赖始终向下。UI、CLI 与 Skill 不拼 SQL，也不自行解释 JSON Schema；领域规则不依赖 Tauri 或某种存储。

## 2. Composite Repository

`DataRepository` 实现既有 `ArcanaRepository` interface，并根据实体所有权路由：

- semantic store：Pack、enabled Pack、AchievementState、Mission、AssistantMemory；
- record store：Record；
- local store：MissionSuggestion、Status selection、Dashboard selection。

读取时三者组成 `SyncedRepositorySnapshot` 与 local state view。这里的 `SyncedRepositorySnapshot` 是领域聚合名称，不表示所有成员物理上都来自同一数据库。

## 3. Command 边界

Application Commands 是唯一业务写入口。它们负责：

1. 获取 runtime OS lock；
2. 从 live JSON 建立 Definition registry；
3. 读取必要的 Records 与本机状态；
4. 执行领域校验和 mutation；
5. 把变化提交给拥有该实体的 store。

多操作 `batch apply` 限定为 `record.*`，因此具有清晰的单 SQLite transaction 语义。JSON-backed mutation 仍支持单命令 `--dry-run`，但不能与 Record 或其他 JSON mutation 组成跨存储 batch。

## 4. 初始化与运行

默认路径：

```text
~/.arcana/settings.json
~/.arcana/repository/             # live semantic JSON
~/.arcana/runtime/arcana.sqlite3  # Records
~/.arcana/runtime/local-state.json
~/.arcana/runtime/arcana.lock
```

`settings.json` 可用 `runtime_dir` 与 `repository_dir` 覆盖路径。`data_dir` 是 Items/Gallery/Weather 外部来源目录，与核心 repository 无关。

首次 `init` 创建普通且启用的 `basic` Pack，并从 bundled JSON 资源导入 `identity.nickname`、`identity.birth_date` 两个 Definitions；它们之后与其他 Pack 内容一样从 live repository 读取。

## 5. 一致性限制

- runtime lock 防止 Arcana 自身多个进程同时修改 live files。
- 每个被替换的 JSON 文件先写临时文件再切换。
- SQLite Record batch 原子提交。
- 多个 JSON 文件之间、JSON 与 SQLite 之间目前没有 crash-atomic transaction；命令不会伪装成具有该保证。
- 用户直接编辑 JSON 时必须避免同时让另一 Arcana 进程写同一 repository；语法、Schema 或引用错误会在下次读取时明确返回。

## 6. 非核心边界

Items、Gallery、Weather 由 adapter 读取外部文件/API。Agent Session、凭证、Git remote/branch 与窗口配置也不进入核心领域仓库。
