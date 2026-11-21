# Console 前端与边缘服务

Console 是 Sprobe 的独立前端与边缘控制台项目。

- 🌐 **在线生产演示环境**: [https://sprobe-console.pages.dev](https://sprobe-console.pages.dev)
- 📖 **详细技术规范与设计**: [docs/frontend/sprobe_v1_2.md](../docs/frontend/sprobe_v1_2.md)

---

## 核心架构与特性

1. **极致轻量高颜值**：
   - 基于 **Vue 3 + Vite + TypeScript + Tailwind CSS**；
   - 生产打包体积仅 **~61 KiB (gzip)**，首屏毫秒级加载；
   - 深度复刻并优化了**哪吒监控（Nezha）**的卡片/表格双视图、国旗识别、月度出账周期流量消耗条与网络双折线图。

2. **Cloudflare 0 成本高并发架构 (Zero-KV-Write Guard)**：
   - 边缘端服务采用 `console/public/_worker.js`；
   - 探针数据上报与心跳完全通过 **Cloudflare Cache API (`caches.default`)** 与边缘实例热内存交换；
   - **完全杜绝 Workers KV 写入（单日写操作锁定为 0）**，彻底避免免费版 1,000 次/天写配额耗尽导致的 HTTP 429 崩溃，实现零成本永久稳定运行。

3. **演示站安全防护体系 (Admin Auth & Visitor Read-Only)**：
   - 默认启用**访客只读模式**，访客可自由浏览大屏但无法执行篡改操作；
   - 「添加服务器」、「编辑服务器」、「下线/删除节点」、「修改通知告警」均受到管理员密码验证保护；
   - 默认管理口令：`sprobe2026`（可在设置页面安全修改或一键重置）。

---

## 快速开发与构建

```bash
# 安装依赖
npm install

# 启动本地开发服务 (http://localhost:5173)
npm run dev

# 生产环境打包 (输出至 console/dist)
npm run build
```

---

## 目录结构

```text
console/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
├── postcss.config.js
├── public/
│   ├── _worker.js              # Cloudflare Pages 边缘服务 (零 KV 写保护模式)
│   ├── install.sh              # 官方一键接入安装脚本
│   └── favicon.svg
├── src/
│   ├── App.vue                 # 根组件 (顶部导航、只读状态胶囊、管理解锁)
│   ├── router/                 # 页面路由 (/overview, /node/:id, /settings)
│   ├── stores/                 # 节点状态与设置管理
│   ├── utils/auth.ts           # 管理员口令认证状态管理
│   ├── components/             # AuthModal 密码拦截弹窗、Cards/Table 视图组件
│   └── pages/                  # Overview, NodeDetail, Settings
└── dist/                       # 生产打包产物
```
