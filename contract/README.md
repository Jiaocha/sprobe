# 机读契约

本目录存放跨端共享的 JSON Schema。

- `sprobe_contract_v1_0.schema.json`：快照协议 v1.0，包含 `Snapshot`、`IngestBatch`、`HeartbeatRequest`、`IngestSuccessResponse`、`ErrorResponse`、`Capabilities`、`SectionMeta` 等定义。

使用说明见 [docs/contract/README.md](../docs/contract/README.md)。契约是后端、前端和测试的共同依据；修改前先读后端文档第 6 章。
