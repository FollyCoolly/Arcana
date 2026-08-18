# 初始化、导入导出与未来 Git 同步

> **状态**：Current + Remaining work
> **最后更新**：2026-08-16

## 1. 当前已经实现

- `arcana-data init` 创建 live JSON repository、Record-only SQLite 和标准 `basic` Pack。
- runtime 路径由 `~/.arcana/settings.json` 的 `runtime_dir` 与 `repository_dir` 控制；缺失时分别使用 `~/.arcana/runtime`、`~/.arcana/repository`。
- `arcana-data json export --output <new-directory>` 将 live semantic JSON 与 SQLite Records 合成为完整确定性目录。
- `arcana-data json import --input <directory>` 完整解析和校验输入，然后把 semantic entities 更新到 configured repository、Records 替换到 SQLite；runtime-local state 不从输入覆盖。
- runtime 通过 OS advisory lock 串行化初始化、导入导出和普通命令。
- SQLite v1 可就地升级为 record-only v2；更早的旧版应用 JSON 不迁移。

`data_dir` 与上述路径无关，只用于 Items/Gallery/Weather 外部来源。

## 2. 当前 repository 格式

完整布局与人工编辑规则见 [data_platform.md](./data_platform.md)。当前 repository Schema 为 2，Pack Schema 为 2（仍可读取 Pack Schema 1）；遇到不支持版本必须拒绝，不能尽力猜测。旧的 repository Schema 1 不迁移。

Live runtime 会直接读取 semantic managed paths：

```text
arcana.json
achievement-states/**
assistant-memory.json
missions.json
packs/**
```

`records/*.json` 是 SQLite Records 的可读同步投影，只在完整 import/export 中读写。普通 Pack/Mission/Memory 命令不会用仓库中陈旧的 `records/` 覆盖 SQLite，也不会删除它。

## 3. Git 的预期使用方式

Arcana 不搭建服务器；一个 private Git repository 对应一个用户。当前用户可以自行执行 Git 命令，且应遵守：

- 一次只在一台设备修改；
- pull 出现冲突时用普通 Git 工具解决，Arcana 不自动 merge/rebase；
- 不同步 runtime database、WAL/SHM、lock、`local-state.json` 或 credentials；
- Git history 提供快照历史，不生成 changelog/oplog/tombstone。

Semantic JSON 已经是 live files，可直接由 Git 管理。Records 仍需要在提交前 export、拉取后 import；这正是尚未完成的同步闭环。

## 4. 尚未实现的安全闭环

未来 `sync` 编排至少需要：

1. 检查 Git conflict/uncommitted managed changes；
2. 在 pull 前确认 SQLite Records 没有未导出的变化，或先把规范 `records/` 更新到工作区；
3. 只允许 fast-forward pull，不自动 merge/rebase；
4. pull 后完整校验 repository，并把 `records/` import 到 SQLite；
5. 在 commit 前把 SQLite Records 确定性 export 到同一 live repository；
6. 保存本机 digest/revision，检测 repository 与 SQLite Records 同时变化的 `both_changed`；
7. 为多文件更新增加 journal 与崩溃恢复；
8. 把 Git credentials 完全交给系统 Git/credential manager。

在这些能力完成前，`json export` 仍只写新目录，Codec 不执行 add/commit/pull/push。

## 5. 冲突策略

- 不实现 CRDT 或字段级自动合并。
- JSON/Git 冲突直接暴露；用户解决后重新校验/import。
- 同一份数据若在 repository 与 SQLite Record 投影两侧都发生变化，应停止并报告，不选择“较新”一侧。
- 语义 JSON 手工编辑错误不做静默修复。

## 6. 失败与恢复边界

当前保证 SQLite Record transaction 原子、单个 JSON 文件替换具备备份恢复路径。当前不保证一次涉及多个 JSON 文件或 JSON+SQLite import 在进程崩溃瞬间完全原子；future sync journal 应补足这一点。文档与 CLI 不应把现状描述成已经具备数据库整体切换或 Git 同步事务。
