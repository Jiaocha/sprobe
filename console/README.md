# Console 前端

Console 是 Sprobe 的独立前端项目，实施文档见 [docs/frontend/sprobe_v1_2.md](../docs/frontend/sprobe_v1_2.md)。

## 技术栈与决议（已冻结）

根据人工决策门确认：
- **HD-01**：后端只做 Agent 单二进制，不含服务端存储/UI；Console 为独立前端与服务端。
- **HD-02**：部署与数据形态采用 **形态 B（Cloudflare Pages Functions + D1 数据库）**。
- **HD-03**：前端框架采用 **Vue 3 + Vite + TypeScript + Tailwind CSS**。
- **HD-04**：Agent 认证使用 Bearer Token + TLS。
- **HD-05**：公开范围：`/overview` 允许公开访问，节点详情默认鉴权（`PUBLIC_DETAIL=false`）。
- **HD-06**：Tier1 平台承诺：Linux (x86_64, aarch64) 与 Windows x86_64，macOS 仅冒烟支持。

## 目录结构

```text
console/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
├── postcss.config.js
├── public/                 # _redirects、_headers、mock 演示数据
├── functions/              # Cloudflare Pages Functions（形态 B）
│   └── api/
│       ├── v1/             # ingest、heartbeat
│       └── readonly/       # overview、nodes/[agent_id]、history
├── migrations/             # D1 schema 与索引
├── src/                    # types、api、utils、components、pages、styles
└── tests/                  # unit、contract、e2e、fixtures
```

## 约束

- 字段路径必须与后端 6.2 和契约 v1.0 一致，不另造同义字段。
- 公开视图只使用 labels 白名单；认证响应禁止边缘缓存。
- 单批不超过 3 快照 / 256 KiB；D1 只保留 latest 与 1 分钟聚合。
- 不在前端保存 ingest token；token 只放服务端 Secret。
