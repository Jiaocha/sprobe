# Sprobe 文档

本目录存放 Sprobe 的用户手册、实施规范、机读契约说明、设计图源和历史归档。

## 当前版本

| 文档 | 版本 | 路径 |
| --- | --- | --- |
| **用户使用与运维部署手册** | **v1.0** | **[USER_GUIDE.md](USER_GUIDE.md)** |
| 后端开发文档 | v1.3 | [backend/rust_v1_3.md](backend/rust_v1_3.md) |
| 前端开发文档 | v1.2 | [frontend/sprobe_v1_2.md](frontend/sprobe_v1_2.md) |
| 机读契约 | v1.0 | [../contract/sprobe_contract_v1_0.schema.json](../contract/sprobe_contract_v1_0.schema.json) |
| 图示说明 | 2026-11-21 | [design/Sprobe_更新图示.md](design/Sprobe_更新图示.md) |
| 可执行任务计划 | v1.0 | [plan/Sprobe_任务拆解_v1.0.md](plan/Sprobe_任务拆解_v1.0.md) |

## 目录结构

- `USER_GUIDE.md`：Sprobe 1.0 全面详尽的使用手册，涵盖 5 分钟上手、CLI 命令、配置参数、Systemd/Windows/Docker 生产部署、Prometheus 抓取与排障 FAQ。
- `backend/`：Agent 的目标范围、架构、指标、数据结构、配置部署、安全、可靠性、测试和计划。
- `frontend/`：Console 的页面、接口契约、D1 数据模型、部署与验收。
- `contract/`：契约的使用说明和字段变更约定；机读文件在仓库根目录 `contract/`。
- `design/`：文档功能图、Console 页面图、图源 HTML 和读图说明。
- `assets/`：导出的 PNG，供 Markdown 和评审材料引用。
- `plan/`：按模块拆解的任务计划、角色分配、依赖波次和验收口径。
- `archive/`：v1.1/v1.2 等历史版本文档，只用于追溯。

## 维护规则

1. 以当前版本目录中的文档为唯一实施依据，归档文档不再更新。
2. 指标名、字段路径和单位必须同时出现在后端文档、前端文档和契约中，三处一致才算完成。
3. 修改图源后重新渲染 PNG，并同步 `design/Sprobe_更新图示.md` 中的数字。
4. 文档中的 JSON、TOML、HTTP 片段是契约示例，不是代码实现。
