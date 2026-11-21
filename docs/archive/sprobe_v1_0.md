# Sprobe 前端展示系统（Console UI）开发与部署文档

> 文档版本：v1.0（开发基线）  
> 编制日期：2026-11-21  
> 适用产品：Sprobe Console UI 1.0（适配 `Sprobe Agent v1.1` 协议 `schema_version = "1.0"`）  
> 托管方案：GitHub + Cloudflare Pages（可选叠加 Cloudflare Pages Functions / Workers + D1）  
> 目标读者：前端研发、全栈/Rust 研发、SRE、测试、安全与运维

**文档约定**

- “必须”表示强制要求，未满足不得上线；“应”表示推荐做法，偏离需在评审中记录理由；“可”表示可选能力。
- 本文档包含接口契约、配置文件示例与验收标准，不包含具体前端业务源码实现。
- 所有指标字段名、单位、空值语义与健康状态机严格对齐《Rust 服务器探针（Sprobe）开发文档 v1.1》第 4、6、11、15 章。

---

## 目录

1. 项目目标、定位与范围
2. 技术选型与工程目录规范
3. 路由架构与页面信息设计
4. 数据契约、状态管理与口径换算
5. 空数据、预热与异常状态规范
6. 视觉设计系统与响应式适配
7. GitHub + Cloudflare Pages 部署与 CI/CD 方案
8. 安全、缓存与免费额度保护
9. 测试与上线验收标准
10. 分阶段实施计划

---

## 1. 项目目标、定位与范围

### 1.1 项目定位
Sprobe Console UI 是面向 **Sprobe 服务器探针** 的轻量级、只读 Web 展示前端，依托 **GitHub + Cloudflare Pages** 实现零服务器成本的公开访问、响应式多端适配与持续自动化更新。

### 1.2 两种运行形态
前端采用同一套静态单页应用（SPA）代码，通过环境变量切换数据源形态：

| 运行形态 | 数据来源 | 适用场景 | 额外成本 |
| :--- | :--- | :--- | :--- |
| **形态 A：纯静态 / 对接既有 API** | 仓库内脱敏快照 JSON（演示/状态页）或既有监控后端只读 HTTP API | 快速搭建公开状态页、产品演示页，或对接已有自建后端 | 0 元（仅 GitHub + Cloudflare Pages） |
| **形态 B：Cloudflare 边缘实时闭环（推荐中小集群）** | Sprobe Agent 推送至同域 Cloudflare Pages Functions / Workers，持久化最新快照至 D1/KV，前端经同域只读 API 读取 | 无需自购服务器即可真实接收探针数据并实时公开展示 | 0 元（Cloudflare Free 额度内，建议 ≤ 20 台节点） |

### 1.3 范围与边界
- **纳入范围**：集群节点总览页、单机状态详情页、8 大监控域可视化与明细表、12 项健康检查与能力清单诊断抽屉、空/预热/降级/失联全量异常态、浅色/深色主题、三档响应式适配、GitHub + Cloudflare Pages 自动化发布与回滚。
- **不纳入范围（严控只读边界）**：不提供任何修改探针配置、远程重启、终止进程、执行命令等写操作；不内置复杂多租户权限管理后台；不在浏览器端长期存储海量历史时序。

---

## 2. 技术选型与工程目录规范

### 2.1 推荐技术栈与设计取舍

| 层面 | 推荐选型 | 取舍理由 | 约束 |
| :--- | :--- | :--- | :--- |
| **构建与语言** | Vite + TypeScript（严格模式） | 构建快（Cloudflare Pages 单次构建 < 30s），强类型直接约束 `schema_version = "1.0"` 快照契约 | 禁止 `any` 绕过快照类型校验 |
| **UI 框架** | React 18+ 或 Vue 3（单页路由） | 生态成熟、组件复用度高、局部状态更新性能稳定 | 禁止引入重型企业中台全量 UI 套件导致首屏膨胀 |
| **样式方案** | 原生 CSS Variables + Tailwind CSS / CSS Modules | 便于维护浅色/深色双主题语义令牌与三档响应式断点 | 所有数值强制 `tabular-nums` |
| **图表库** | ECharts（按需引入折线/面积/柱状模块）或 uPlot | 轻量、支持 `null` 断点不连线、高密度时序渲染流畅 | 严禁全量打包图表库；首屏 JS 压缩后 ≤ 200 KiB |
| **边缘接口（可选）** | Cloudflare Pages Functions（`functions/` 目录） | 与静态前端同仓库、同域名部署，天然免 CORS 且零运维 | 仅用于形态 B；禁止在前端代码中硬编码任何密钥 |

### 2.2 仓库目录结构规范

```text
sprobe-console/
├── .github/
│   └── workflows/
│       └── ci.yml                 # PR 校验（类型检查、单测、构建、Schema 校验）
├── public/
│   ├── _redirects                 # Cloudflare Pages SPA 路由回退规则
│   ├── _headers                   # 边缘缓存策略与浏览器安全响应头
│   └── mock/                      # 静态演示用脱敏 Sprobe v1.1 快照 JSON
│       ├── overview.json
│       └── node-web-01.json
├── functions/                     # 【可选，形态 B】Cloudflare Pages Functions
│   └── api/
│       ├── v1/
│       │   ├── ingest.ts          # 接收 Sprobe Agent 批次上报（校验 Token + 写 D1/KV）
│       │   └── heartbeat.ts       # 接收 Sprobe Agent 独立心跳
│       └── readonly/
│           ├── overview.ts        # 前端只读集群总览接口（带 Edge Cache）
│           └── nodes/
│               └── [agent_id].ts  # 前端只读单机详情接口（脱敏输出）
├── src/
│   ├── types/                     # Sprobe v1.1 Snapshot / Health / Capabilities TS 类型定义
│   ├── api/                       # 数据源适配器（Static Mock / Remote API / Same-Origin）
│   ├── utils/                     # 单位换算、空值格式化、时间格式化、脱敏过滤
│   ├── components/                # 通用组件（健康徽章、KPI 卡、空/异常态横幅、能力浮层）
│   ├── pages/
│   │   ├── OverviewPage.tsx       # /overview 集群总览页
│   │   ├── NodeDetailPage.tsx     # /nodes/:agent_id 单机详情页
│   │   └── NotFoundPage.tsx       # 404 兜底页
│   └── styles/
│       └── tokens.css             # 浅色/深色主题 CSS 语义变量
├── wrangler.toml                  # Cloudflare Pages / D1 / KV 绑定配置（不含密钥）
├── package.json                   # 依赖与构建脚本
└── package-lock.json              # 锁定依赖版本，必须提交
```

### 2.3 环境变量约定
- **前端构建变量（公开可见，仅放非敏感配置）**：
  - `VITE_DATA_MODE`：`static`（静态演示） | `api`（连接只读后端或同域 Functions）。
  - `VITE_API_BASE_URL`：只读 API 基地址（同域部署时留空 `""`）。
  - `VITE_POLL_INTERVAL_MS`：默认自动刷新间隔（默认 `10000`，对应 `fast` 周期）。
  - `VITE_SITE_TITLE`：页面标题（默认 `Sprobe Status Console`）。
- **边缘服务端密钥（仅形态 B，通过 Cloudflare Dashboard Secrets 注入）**：
  - `SPROBE_INGEST_TOKENS`：合法探针上报令牌映射，**严禁**写入 `.env`、`wrangler.toml` 或前端 `VITE_*` 变量。

---

## 3. 路由架构与页面信息设计

### 3.1 前端路由表

| 路径 | 页面 | 职责与访问规则 |
| :--- | :--- | :--- |
| `/` | 入口重定向 | 默认 302/客户端重定向至 `/overview`；单机本地模式（`127.0.0.1:9101/ui`）直接渲染单机详情 |
| `/overview` | 集群总览页 | 展示全集群健康分布、在线/失联统计、高负载/高磁盘预警与可筛选节点列表 |
| `/nodes/:agent_id` | 单机详情页 | 展示单个节点的 8 大监控域实时状态、趋势、存储与进程明细及诊断抽屉 |
| `*` | 404 页面 | 提示路由或 `agent_id` 不存在，提供返回 `/overview` 按钮 |

### 3.2 单机详情页（`/nodes/:agent_id`）五层信息架构
1. **顶部全局上下文栏（固定吸顶）**：返回 `/overview` 面包屑、节点切换下拉框（按 `hostname`/`agent_id`/`labels` 检索）、系统/架构标签（`os · arch · virtualization`）、节点健康总状态徽章、最新 `collected_at` 与 `age_seconds`、刷新控制（`10s`/`30s`/暂停/手动刷新）、主题切换。
2. **L0 全局状态与异常横幅区（按需展开）**：仅在 `warmup=true`、`health.status != ok`、`clock_adjusted=true` 或接口失联时置顶显示，提供锚点跳转至异常卡片或查看 Runbook 摘要。
3. **L1 核心 KPI 概览带（5 卡并排）**：CPU 综合利用率、内存与 Swap 使用率、系统负载（1m/5m/15m + 每核负载）、主机开机时长、基础健康与 Agent 自监控摘要。
4. **L2 实时趋势与资源构成区（双栏）**：左栏为 CPU 模式拆分趋势（支持切换分核视图 `cpu.cores[]`）；右栏为内存构成堆叠条与网络接口速率卡片组（`network.interfaces[]`）。
5. **L3 存储与进程明细区**：
   - **存储双页签**：文件系统容量与 Inode 表（`filesystems.items[]`） / 块设备 I/O 表（`disks.items[]`，默认筛选 `disk_class = physical` 并提示防重复求和）。
   - **进程面板**：进程总数与状态分布条 + 被监视进程（`watch[]`）状态胶囊 + Top-N 进程表（`top[]`，支持按 CPU/RSS 排序）。
6. **L4 诊断抽屉 / 折叠区**：12 项健康检查明细（`health.checks[]`）、平台能力矩阵（`collection.capabilities` + `unsupported_reasons`）、事件流（`events[]`）与 Agent 自监控详情（WAL 积压、采集耗时、推送成功率）。

### 3.3 核心组件与 Sprobe v1.1 字段绑定表

| 组件 | 绑定字段（对齐 Sprobe v1.1 第 6 章） | 关键展示规则 |
| :--- | :--- | :--- |
| **健康总徽章** | `health.status`、`status_code`、`since`、`recovered_at` | 颜色映射四态；悬停显示持续时长与上次恢复时间 |
| **CPU 概览卡** | `cpu.status`、`quality`、`total.usage_ratio`、`iowait_ratio`、`steal_ratio`、`logical_count` | 展示百分比与核数；`quality=estimated` 带虚线徽标 |
| **内存概览卡** | `memory.usage_ratio`、`used_bytes`、`available_bytes`、`available_source`、`swap_total_bytes`、`swap_usage_ratio` | 底部标注 `available_source`；`swap_total_bytes=0` 显示“未配置交换区” |
| **系统负载卡** | `load.average_1m/5m/15m`、`average_1m_per_core`、`load.quality`、`processor_queue_length` | Windows 合成值必带 `估算 (EWMA)` 标识并展示队列长度 |
| **开机时长卡** | `uptime.host_seconds`、`host_boot_time`、`agent_seconds`、`clock_adjusted` | 格式化为 `X天 X小时 X分`；`clock_adjusted=true` 点亮琥珀色图标 |
| **文件系统表** | `filesystems.status`、`errors`、`items[]`（`mountpoint`/`fstype`/`usage_ratio`/`available_bytes`/`reserved_bytes`/`inodes_usage_ratio`/`read_only`/`stale`） | 进度条 90%/95% 变色；`read_only=true` 置顶红标；`stale=true` 降对比度并标注陈旧 |
| **磁盘 I/O 表** | `disks.status`、`items[]`（`device`/`disk_class`/`read_bytes_per_second`/`write_bytes_per_second`/`utilization_ratio`/`read_latency_seconds`） | 默认按 `disk_class=physical` 过滤；空闲 `latency=null` 显示 `—` |
| **网络接口组** | `network.status`、`interfaces[]`（`name`/`class`/`up`/`receive_bytes_per_second`/`transmit_bytes_per_second`/`errors`/`dropped`） | `up=false` 置灰；仅错包/丢包增长时高亮警示；不展示未配置的全网合计 |
| **进程观测区** | `processes.visibility`、`count`、各状态数、`top[]`、`watch[]` | `watch[]` 缺席标红；`visibility=partial` 显示权限受限提示 |
| **诊断抽屉** | `health.checks[]`、`collection.capabilities`、`unsupported_reasons`、`events[]` | 异常检查置顶；能力表逐项解释所有 `null` 指标原因 |

---

## 4. 数据契约、状态管理与口径换算

### 4.1 前端只读 API 契约
当 `VITE_DATA_MODE = "api"` 时，前端仅发起以下只读 `GET` 请求：

| 端点 | 返回内容 | 建议边缘缓存（Edge Cache） |
| :--- | :--- | :--- |
| `GET /api/readonly/overview` | 集群汇总计数 + 节点摘要列表（`agent_id`、`hostname`、`os`、`arch`、`labels`、`health`、核心 KPI、`collected_at`、`online`） | `Cache-Control: public, max-age=5, s-maxage=10` |
| `GET /api/readonly/nodes/:agent_id` | 该节点最新完整脱敏 `Snapshot` 对象（结构与 Sprobe v1.1 `6.2` 100% 一致） | `Cache-Control: public, max-age=5, s-maxage=10` |
| `GET /api/readonly/nodes/:agent_id/history?window=1h` | 可选：指定时间窗口（15m/1h/6h/24h）内的轻量趋势点数组（CPU/内存/负载/网络速率） | `Cache-Control: public, max-age=15, s-maxage=30` |

### 4.2 单位换算与格式化规范
- **比率（`*_ratio`）**：底层 `[0, 1]` 统一乘 100 保留 1 位小数展示（如 `0.2315 -> 23.2%`）；告警明细 Tooltip 保留 2 位小数。
- **容量（`*_bytes`）**：采用二进制前缀（1024 换算）自适应展示为 `B` / `KiB` / `MiB` / `GiB` / `TiB`，保留 1–2 位小数；悬停展示原始字节或精确 GiB。
- **速率（`*_bytes_per_second`）**：默认展示 `MiB/s` 或 `KiB/s`；提供全局单位切换开关支持 `Mbps`（`bytes/s × 8 ÷ 1,000,000`），明确标注单位防止混淆。
- **时间与时区**：接口一律接收 UTC RFC 3339；前端默认转换为浏览器本地时区展示（如 `HH:mm:ss`），并在悬停 Tooltip 中同时展示原始 UTC 时间与 `age_seconds`。
- **等宽数字**：所有动态变化的指标数字、表格数值列、时间戳必须应用 `font-variant-numeric: tabular-nums`。

### 4.3 轮询与前端状态控制
1. **智能轮询**：默认按 `10s` 轮询；监听浏览器 `document.visibilityState`，当标签页切换至后台时自动暂停轮询，切回前台时立即补拉一次，显著降低 Cloudflare Workers 免费额度消耗。
2. **交互防打断**：当用户正在展开下拉框、悬停 Tooltip 或鼠标停留在表格行上时，新数据仅更新数字单元格，暂缓触发行顺序重排。
3. **请求并发控制**：使用 `AbortController` 取消尚未完成的旧请求，避免慢响应覆盖新快照。
4. **组件级 Error Boundary**：8 大监控域各自包裹独立错误边界，单一字段异常或渲染异常仅在对应卡片内展示降级提示，绝不白屏崩溃。

---

## 5. 空数据、预热与异常状态规范

前端必须严格执行 **“缺失 ≠ 0”** 与 **“局部失败不拖垮全局”** 原则：

| 状态场景 | 判定依据 | 界面展示与交互规范 |
| :--- | :--- | :--- |
| **首次加载** | 首个请求尚未完成 | 按实际卡片与表格尺寸渲染骨架屏（Skeleton），控制 CLS ≤ 0.05 |
| **探针预热中** | `collection.warmup == true` | 顶部展示信息蓝横幅“探针预热中，速率类指标将在下一采集周期就绪”；速率卡片显示“预热中”，折线图断空不画 0 |
| **字段缺失 / 不支持** | 值为 `null` 或 `capabilities[k] == false` | 统一显示 `—`（严禁显示 `0`），附带灰色说明图标；悬停弹出 `unsupported_reasons` 或“计数器重置，下周期恢复” |
| **分区局部失败 / 超时** | 分区 `status` 为 `partial` / `timeout` / `failed` | 仅在该分区顶部展示黄/红局部提示条与 `errors[]` 摘要，其余正常分区照常展示 |
| **数据陈旧（Stale）** | `stale == true` 或 `age_seconds > 2×周期` | 对应卡片/表格行降低透明度并附加琥珀色 `陈旧 (Xs 前)` 标签，提示不作为实时告警依据 |
| **合法业务空态** | `swap_total_bytes == 0`、`watch == []`、`events == []` | 展示中性引导文案（如“未配置交换区”“未配置关键进程监视项”“本周期无异常事件”），不使用错误色 |
| **集群无节点 / 筛选无果** | 节点数组为空 | 无节点时展示部署指引与文档链接；筛选无结果时提供一键“重置筛选”按钮 |
| **失联 / 推送鉴权失败 / 断网** | `push_auth_failed`、`health.status == unknown` 或接口连续失败 | 顶部展示红色阻断横幅（明确区分“Agent 失联”“推送鉴权失败 401/403”或“浏览器网络中断”），保留最后一次成功快照并置灰，附带 Runbook 要点与“立即重试”按钮 |

---

## 6. 视觉设计系统与响应式适配

### 6.1 语义色彩令牌（Light / Dark 双主题）

| 语义角色 | 适用状态 | 浅色模式主色参考 | 深色模式主色参考 |
| :--- | :--- | :--- | :--- |
| `status-ok` | `ok`、在线、恢复 | `#15803D`（绿） | `#4ADE80` |
| `status-warn` | `degraded`、`warning`、`stale` | `#B45309`（琥珀） | `#FBBF24` |
| `status-crit` | `critical`、`failed`、`read_only` | `#B91C1C`（红） | `#F87171` |
| `status-unknown` | `unknown`、`null`、`unsupported` | `#64748B`（中性灰） | `#94A3B8` |
| `status-info` | `warmup`、`estimated`、交互高亮 | `#1D4ED8`（信息蓝） | `#60A5FA` |

### 6.2 响应式断点适配策略

| 断点区间 | 目标设备 | 布局与交互适配规则 |
| :--- | :--- | :--- |
| **≥ 1280px** | 桌面显示器 / 运维大屏 | 12 栅格：L1 为 5 列并排 KPI 卡；L2 为 6:6 双栏趋势区；L3 完整表格展开；L4 诊断抽屉右侧滑出 |
| **768px – 1279px** | 平板 / 窄屏笔记本 | 顶部栏次要元信息收纳进 Popover；L1 折为 2–3 列网格；L2 上下堆叠；L3 表格固定首列（挂载点/设备/进程名）并允许横向滚动 |
| **< 768px** | 手机端值班应急 | 单列纵向堆叠；首屏置顶健康横幅与紧凑 KPI 卡；趋势图简化图例与刻度；L3 明细表转为折叠卡片列表（主显名称、核心进度条与状态徽标，点击展开全部字段） |

### 6.3 性能与无障碍（A11y）预算
- 首屏核心 JS（gzipped）≤ 200 KiB；LCP ≤ 2.0s；CLS ≤ 0.05。
- 所有状态不仅依赖颜色区分，必须同时配合文字标签或图标（照顾色弱用户）；所有交互控件支持键盘 `Tab` 聚焦与 `aria-label` 语义标注。

---

## 7. GitHub + Cloudflare Pages 部署与 CI/CD 方案

### 7.1 免费额度基线与容量规划
- **Cloudflare Pages（Free）**：静态站点请求与流量**不限量**，每月 **500 次构建**、1 并发构建，单站最多 20,000 文件、单文件 ≤ 25 MiB。
- **Cloudflare Workers / Pages Functions + D1（Free，仅形态 B）**：Workers 每日 **100,000 次请求**（00:00 UTC 重置）、单次 CPU ≤ 10ms（网络与数据库等待不计入）；D1 免费存储 **5 GB**。
  - **容量估算**：假设 15 台服务器按 `30s` 推送一批 + `30s` 心跳，每天上报请求约 `15 × 2 × 2 × 60 × 24 = 86,400` 次（若仅在无快照时发心跳，则仅约 `43,200` 次/天）；配合前端 `10s` Edge Cache，可稳定运行在每日 10 万次免费额度内。
- **GitHub（Free）**：支持私有或公开仓库托管代码，含每月 **2,000 分钟** GitHub Actions 额度用于 PR 质量门禁。

### 7.2 核心配置文件规范

#### 1. SPA 路由回退：`public/_redirects`
确保直接打开或刷新 `/overview`、`/nodes/<agent_id>` 不触发 404（API 路径由 `functions/` 自动优先拦截）：

```text
/*    /index.html   200
```

#### 2. 缓存与安全响应头：`public/_headers`
确保 `index.html` 更新后立即生效，带内容哈希的构建产物长缓存，并默认施加安全头：

```text
/*
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: camera=(), microphone=(), geolocation=()
  Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' https:; frame-ancestors 'none'

/index.html
  Cache-Control: no-cache, no-store, must-revalidate

/assets/*
  Cache-Control: public, max-age=31536000, immutable
```

#### 3. 边缘运行时与存储绑定：`wrangler.toml`（仅形态 B 需要）

```toml
name = "sprobe-console"
compatibility_date = "2026-11-01"
pages_build_output_dir = "dist"

[[d1_databases]]
binding = "SPROBE_DB"
database_name = "sprobe-metrics"
database_id = "<通过 wrangler d1 create 生成>"
```

#### 4. PR 质量门禁：`.github/workflows/ci.yml`
在合并至 `main` 前拦截类型错误、契约不匹配与构建失败，避免浪费 Cloudflare Pages 每月 500 次构建额度：

```yaml
name: Frontend CI
on:
  pull_request:
    branches: [main]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
      - run: npm ci
      - run: npm run typecheck
      - run: npm test -- --run
      - run: npm run build
```

### 7.3 部署接入与后续更新流程
1. **首次接入**：登录 Cloudflare Dashboard -> `Workers & Pages` -> `Create` -> `Pages` -> `Connect to Git`，授权选择 GitHub 目标仓库；设置生产分支为 `main`，构建命令 `npm run build`，输出目录 `dist`，按需填入 `VITE_*` 环境变量（形态 B 同步在 Settings -> Environment variables 添加加密 Secret `SPROBE_INGEST_TOKENS` 并绑定 D1）。
2. **PR 预览与内容更新**：开发者基于新分支修改页面文案、布局或静态快照 -> 提交 PR -> GitHub Actions 运行 `ci.yml`，同时 Cloudflare Pages 自动生成独立预览链接 `https://<hash>.<project>.pages.dev` -> 评审通过后合并至 `main` -> Cloudflare Pages 在 1–2 分钟内自动构建并发布至全球边缘节点。
3. **一键回滚**：若线上发现显示异常，在 Cloudflare Pages 的 `Deployments` 列表中找到上一个健康版本，点击 `Rollback to this deployment` 即可秒级回退，无需重新构建。

### 7.4 公开访问地址获取与自定义域名
- **默认免费地址**：首次部署成功后自动获得永久 HTTPS 地址 `https://<project-name>.pages.dev`。
- **绑定自定义域名**：在项目 `Custom domains` 中添加子域名（如 `status.example.com`）。DNS 托管在 Cloudflare 时自动创建 `CNAME` 并签发/自动续期免费 SSL 证书；外部 DNS 服务商只需手动添加 `CNAME` 指向 `<project-name>.pages.dev`。完成后在 SSL/TLS 中开启 **Always Use HTTPS**。

---

## 8. 安全、缓存与免费额度保护

1. **零密钥暴露**：前端构建产物与 Git 仓库严禁出现任何上报 Token、私钥或内网管理地址；形态 B 仅通过同域 `/api/readonly/*` 返回已脱敏的公开视图字段。
2. **只读接口限流与缓存**：
   - 在 `/api/readonly/*` 响应头设置 `s-maxage=10`，结合 Cloudflare 边缘缓存合并并发访客请求，防止高并发访问消耗 Workers 免费额度。
   - 在 Cloudflare Dashboard `Security -> WAF -> Rate limiting rules` 中配置免费限流规则（如单 IP 每 10 秒超过 30 次 API 请求返回 429）。
3. **防恶意上报（形态 B）**：`/api/v1/ingest` 必须校验 `Authorization: Bearer` 令牌、限制请求体大小（与 Sprobe v1.1 `6.11` 对齐：压缩后 ≤ 4 MiB）并在校验失败时直接返回 `401`/`400`/`413`。
4. **供应链安全**：提交 `package-lock.json`；开启 GitHub Dependabot 告警，发版前运行 `npm audit --audit-level=high` 确保无高危漏洞。

---

## 9. 测试与上线验收标准

### 9.1 测试分层
- **单元测试**：覆盖 `bytes`/`ratio`/`bytes_per_second` 换算、`null`/`NaN`/`Inf` 防御、`warmup` 与健康状态映射。
- **契约与异常态测试**：使用涵盖正常、预热、`null`、分区 `partial`/`timeout`、`stale`、Windows 合成负载、失联 8 类场景的 Mock JSON Fixtures，断言页面渲染符合第 5 章规范。
- **E2E 与响应式测试**：在 1440px / 1024px / 375px 三个视口执行路由跳转、刷新不 404、筛选排序、主题切换与空态检查。

### 9.2 上线验收清单

| 验收维度 | 检查项 | 合格标准 |
| :--- | :--- | :--- |
| **公开可达与 HTTPS** | 匿名公网访问与路由刷新 | 访问首页、`/overview`、`/nodes/<id>` 均返回 HTTP 200；HTTP 自动 301 跳转 HTTPS 且证书有效；深层路由直接刷新不报 404 |
| **响应式与视觉** | 1440px / 1024px / 375px 实测 | 无异常横向滚动条与重叠截断；指标数值等宽对齐不抖动；浅色/深色模式对比度达标 |
| **契约与异常态** | 第 5 章 8 类场景全覆盖 | 单位换算误差为 0；`null`/能力缺失显示 `—` 而非 `0`；局部失败不影响其余卡片；失联横幅与重试按钮可用 |
| **自动更新与回滚** | Git Push 闭环演练 | 合并 `main` 后 2 分钟内公网生效且无旧 `index.html` 缓存；控制台一键回滚验证通过 |
| **安全与性能** | 安全头、脱敏与包体检查 | DevTools 无明文 Token/敏感路径；响应头含 `_headers` 定义的 CSP/nosniff；首屏 JS ≤ 200 KiB gzipped，LCP ≤ 2.0s |

### 9.3 常见上线故障排障
- **刷新 `/nodes/:id` 返回 404**：检查 `public/_redirects` 是否正确复制到了最终构建目录 `dist/_redirects`。
- **更新后浏览器仍看到旧页面**：检查 `public/_headers` 中 `/index.html` 是否配置 `no-cache`，并在 Cloudflare Dashboard 核对未开启覆盖静态 HTML 的强缓存 Page Rule。
- **Workers 免费额度提前耗尽**：检查前端后台标签页是否已通过 `Page Visibility API` 暂停轮询、`/api/readonly/*` 是否命中 `CF-Cache-Status: HIT`，以及 Agent 是否开启了不必要的冗余心跳。

---

## 10. 分阶段实施计划（3 周交付）

| 阶段 | 周期 | 核心交付内容 | 退出验收标准 |
| :--- | :--- | :--- | :--- |
| **F1：骨架与部署打通** | 第 1 周 | 初始化 Vite + TS 工程、定义 Sprobe v1.1 类型与 Mock Fixtures、配置 `_redirects`/`_headers`/`ci.yml` 并完成 GitHub + Cloudflare Pages 首次上线 | `*.pages.dev` 公开可访问；PR 预览与 `main` 自动部署闭环通过 |
| **F2：核心视图与异常态** | 第 2 周 | 实现 `/overview` 与 `/nodes/:agent_id` 全量组件、单位换算、诊断抽屉、8 类空/异常态与三档响应式适配 | 全部 Mock 场景与三档断点测试通过；单元测试覆盖率 ≥ 80% |
| **F3：数据对接与正式发布** | 第 3 周 | 对接只读 API（或部署 Pages Functions + D1 实时接收）、配置边缘缓存与限流、绑定自定义域名并执行上线验收与回滚演练 | 9.2 上线验收清单 100% 通过，正式发布 v1.0 |
