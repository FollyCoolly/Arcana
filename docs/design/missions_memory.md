# Mission 与 AssistantMemory

> **状态**：Target / 领域边界已确定，物理 Schema 尚未定稿
> **最后更新**：2026-08-15

## 1. MissionSuggestion 与 Mission

AI 生成但用户尚未接受的内容不是正式 Mission：

```text
MissionSuggestion --accept--> Mission
```

### MissionSuggestion

- 状态为 pending 或 rejected；
- 只保留在本机，不进入 Git 同步；
- 接受时可以沿用稳定 ID 创建 Mission；
- 拒绝后的长期偏好可以被精炼进 AssistantMemory，但建议实体本身不同步。

### Mission

- 只有用户接受后才创建并同步；
- active、completed、archived 是同一实体的生命周期状态；
- 不再在 current/archive 文件之间搬运成不同类型；
- parent-child 继续通过稳定 Mission ID 表达；
- `days_remaining` 等值按需计算，不持久化。

主菜单 countdown、hints、progress 等展示选择属于本机 Dashboard 配置，不属于 Mission 领域实体，也不跨设备同步。

## 2. AssistantMemory

AssistantMemory 保存跨会话仍有价值的长期语义信息，例如：

- focus areas；
- 接受/拒绝偏好；
- 稳定习惯与约束；
- 精炼的对话摘要；
- 对用户的重要长期观察；
- 用户暂时不想补充、但未来可能影响 Achievement 的历史信息提醒。

AssistantMemory 可以同步，并使用普通 Git 冲突处理方式。

## 3. 不属于 AssistantMemory

- 完整 Agent Session；
- 原始聊天记录；
- `completed_mission_log`：应直接查询 Mission；
- `last_generation`：属于本机 MissionSuggestion 生成器状态；
- API key、provider、Telegram 和其他凭证；
- UI event 和短期缓存。

## 4. 精炼与清理

新模型不使用固定长度 FIFO 无条件丢弃记忆。实现前仍需确定：

- Memory entry 的稳定 ID；
- 相同事实的合并和替换规则；
- 过时事实如何标记或删除；
- 摘要何时精炼；
- 如何避免把临时判断固化为长期用户事实。

这些规则只影响 AssistantMemory，不得替代 RecordData、Mission 或用户成就状态等权威数据。
