# 脚本

本目录用于本地开发和 CI 辅助脚本，当前不包含实现。

计划中的脚本：

- `check-docs`：校验 Markdown 表格、代码围栏、相对链接和图片路径。
- `check-contract`：用 JSON Schema 校验 `tests/fixtures` 的黄金快照与批次。
- `mock-ingest`：本地模拟服务端 200/400/401/409/413/429/500 和超时。
- `render-diagrams`：从 `docs/design/src/*.html` 重新渲染 PNG。
- `release-check`：版本号、CHANGELOG、二进制大小和许可证检查。

脚本只做开发辅助，不进入 Agent 运行时；不要把密钥或生产地址写入脚本。
