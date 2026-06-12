# Dify Log Retriever

用于归档 Dify Console 对话日志、解析 workflow 节点执行，并导出消息级与 LLM/Agent 节点级 JSONL 评测数据的 Tauri 桌面工具。

## 功能

- 多 Dify 应用配置：保存 `base_url`、`app_id` 和加密后的 Bearer token。
- 手动增量同步：拉取会话、消息、workflow run、node executions，按 Dify ID upsert 去重。
- 原始副本归档：SQLite 保存 Dify 返回的完整 JSON。
- 数据导出：支持会话导出、消息导出和节点评测导出。

### 导出字段说明（会话导出）

- CSV 至少包含：标题、用户或账户、状态、消息数、用户赞、用户踩、管理员赞、管理员踩、更新时间、创建时间。
- 状态来自会话级 `status_count` 聚合，示例：`Success`、`1 Failure`、`2 Success, 1 Failure`。
- 用户反馈和管理员反馈已拆分为独立的赞/踩数量列，便于表格统计与筛选。
- JSON/JSONL 与 CSV 使用同一组会话维度字段，并额外保留原始统计字段以便后续分析。

### 导出字段说明（消息导出）

- 消息导出保留原有明细字段：`query`、`answer`、`feedback`、`answer_tokens`、`prompt_tokens`、`elapsed_time`、`created_at`。
- 当勾选元数据或 Agent 思维链时，JSON/JSONL 会额外带出 `metadata`、`retriever_resources`、`agent_thoughts`。

## 开发

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run build
npm run tauri build
```

Linux 构建产物位于：

- `src-tauri/target/release/dify-log-retriever`
- `src-tauri/target/release/bundle/deb/`
- `src-tauri/target/release/bundle/rpm/`

## 数据与安全

- SQLite 数据库保存在 Tauri 应用数据目录中。
- token 使用 AES-GCM 加密后入库。
- 主密钥优先保存到系统 keyring；如果系统 keyring 不可用，会退回到应用数据目录下的本机密钥文件。
- 不要把真实 Bearer token 写入源码、README、`.env` 或提交记录。
