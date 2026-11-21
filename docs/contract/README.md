# 契约说明

机读契约文件位于仓库根目录：

- [sprobe_contract_v1_0.schema.json](../../contract/sprobe_contract_v1_0.schema.json)

## 用途

- 校验 Agent 产出的 `Snapshot`、`IngestBatch`、`HeartbeatRequest`。
- 校验服务端返回的 `IngestSuccessResponse`、`ErrorResponse`。
- 作为后端文档 6.2–6.10、前端文档第 6–7 章和测试 fixtures 的共同依据。

## 约束

- `schema_version` 固定为 `1.0`；不兼容变更必须提升主版本并新增文件，不能原地改语义。
- 分区级字段（`status`、`quality`、`sampled_at`、`age_seconds`、`errors`）由 `SectionMeta` 统一约束。
- 缺失值用 `null`，不支持用能力位表达；不能用 `0` 伪造缺失。
- 契约变更必须同步后端文档、前端文档和前端类型定义。

## 校验建议

- CI 对 `tests/fixtures` 的黄金快照执行 schema 校验。
- 对 `IngestBatch` 额外校验 `snapshot_count` 与 `snapshots` 长度一致、序号连续。
- 错误响应只使用契约中列出的错误码，不把内部错误文本直接暴露给客户端。
