# Dify Log Retriever

用于归档 Dify Console 对话日志、解析 workflow 节点执行，并导出消息级与 LLM/Agent 节点级 JSONL 评测数据的 Tauri 桌面工具。

## 功能

- 多 Dify 应用配置：保存 `base_url`、`app_id` 和加密后的 Bearer token。
- 手动增量同步：拉取会话、消息、workflow run、node executions，按 Dify ID upsert 去重。
- 原始副本归档：SQLite 保存 Dify 返回的完整 JSON。
- 评测导出：导出消息级 JSONL、节点级 JSONL 或两者混合文件。

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
