# Dify Log Retriever

用于归档 Dify Console 对话日志、解析 workflow 节点执行，并导出消息级与 LLM/Agent 节点级 JSONL 评测数据的 Tauri 桌面工具。

## 功能

- 多 Dify 应用配置：保存 `base_url`、`app_id` 和加密后的 Bearer token。
- 手动增量同步：拉取会话、消息、workflow run、node executions，按 Dify ID upsert 去重。
- 原始副本归档：SQLite 保存 Dify 返回的完整 JSON。
- 数据导出：支持会话导出、消息导出和节点评测导出，其中会话/消息支持 Excel、CSV、JSON、JSONL。

### 导出字段说明（会话导出）

- CSV 至少包含：标题、用户或账户、成功消息数、失败消息数、消息数、用户赞、用户踩、管理员赞、管理员踩、更新时间、创建时间。
- Excel 导出与 CSV 使用同一组会话字段，更适合承载长文本而不破坏记录边界；超长单元格内容会在写入前截断，避免 Excel 导出失败。
- 成功/失败消息数来自会话级 `status_count` 聚合；`partial_success` 会保留在 JSON/JSONL 原始统计字段中。
- 用户反馈和管理员反馈已拆分为独立的赞/踩数量列，便于表格统计与筛选。
- JSON/JSONL 与 CSV 使用同一组会话维度字段，并额外保留原始统计字段以便后续分析。

### 导出字段说明（消息导出）

- CSV 至少包含：`id`、`message_id`、`conversation_id`、`user_or_account`、用户赞、用户踩、用户反馈内容、`query`、`answer`、`feedback`、`answer_tokens`、`prompt_tokens`、`elapsed_time`、`created_at`。
- Excel 导出与 CSV 字段一致；对于 `query`、`answer`、JSON 字符串等长文本，会压平换行后写入单元格，避免导出后看起来像多行记录；超出 Excel 单元格上限时会截断并追加标记。
- 消息导出的 `created_at` 使用东八区可读时间格式；JSON/JSONL 同时额外提供 `created_at_human` 便于显式区分。
- 消息导出的赞/踩以消息自身的 `feedback` 当前值为准；当 `feedback` 缺失时，再回退到 `feedbacks` 的首条评分明细，不再复用会话级聚合统计。
- JSON/JSONL 额外提供 `user_feedback_like`、`user_feedback_dislike`、`user_feedback_content`。
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
