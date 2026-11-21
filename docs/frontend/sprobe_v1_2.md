# Sprobe Console UI 前端开发文档

> 文档版本：v1.2（M0 审核修订统一版）
>
> 编制日期：2026-11-21
>
> 适用产品：Sprobe Console UI 1.0（对接 Sprobe Agent 1.0.x / 协议 `schema_version = "1.0"`）
>
> 托管方案：GitHub + Cloudflare Pages；可选 Cloudflare Pages Functions + D1（定时清理需独立 Cron Worker 或 GitHub Actions）
>
> 目标读者：前端研发、全栈/Rust 研发、SRE、测试、安全与运维
>
> 配套文档：《Rust 服务器探针（Sprobe）后端开发文档 v1.3》（`rust_v1_3.md`）、机读契约 `sprobe_contract_v1_0.schema.json`

**文档约定**

* “必须”表示强制要求，未满足不得上线；“应”表示推荐做法，偏离需在评审中记录理由；“可”表示可选能力。
* 本文包含接口契约、D1 数据模型、配置与部署示例，不包含前端业务源码实现。
* 指标字段名、单位、空值语义以 `rust_v1_3.md` 第 6 章快照契约与附录 A 为准；本文只定义前端消费方式，不另行发明字段。
* 本文所有容量数字按 Cloudflare Free 额度估算；超出预算必须切换 Paid 或降低节点数，不得假设“免费无限”。

**v1.2 修订摘要（M0 审核统一）**

* **对齐后端 6.2 快照契约**：修正 6.3 `snapshot` 示例与公开详情白名单，将 `host.os/arch` 修正为 `agent.os`/`agent.arch` 与 `host.os_name`/`host.architecture`；将 `collection.capabilities` 键名对齐为后端 6.8 细粒度能力键；补齐所有分区必填字段（`status`/`quality`/`sampled_at`/`age_seconds`/`errors`）。
* **补齐 WAL/推送摘要与趋势图数据链路**：在 `agents` 表与 `GET /api/readonly/nodes/{agent_id}` 增加 `agent_runtime`（`last_push_at`/`last_heartbeat_at`/`wal_snapshots`/`online`），闭环心跳入库与 L4 抽屉展示；扩充 `metrics_1m` 与 `/history` 字段（增加 CPU iowait/steal 与内存构成字段），明确历史网络吞吐仅汇总 `class == "physical"` 接口。
* **修正 D1 幂等、重启覆盖与容量核算**：补齐同实例单快照 `sequence` 去重与跨实例按 `collected_at` 覆盖规则；重算 D1 写入行数（计入 `agents` 更新、心跳与 `DELETE`），将 Free 容量承诺修正为 **≤4–5 台**（60s 批周期 + 无快照才心跳时可至 8–10 台），10–20 台需 Workers Paid；明确定时清理通过独立 Cron Worker 或 GitHub Actions 执行，且形态 B 限制单批 ≤3 快照/256 KiB 并使用 `db.batch()` 适配 10ms CPU 预算。
* **补齐安全、鉴权与排期细节**：明确 Cloudflare Access 路径拦截策略与 SPA `fetch` 遇 401/302 整页跳转逻辑；新增 D1 `ingest_tokens` 表支持单机令牌绑定与 24h 双令牌轮换；公开总览 `labels` 改为白名单透传；明确 `online`（90s）与 `summary.alerts` 计算口径、基于 `generated_at - collected_at` 计算陈旧态，以及前端 W3 契约桩验收 + W5 端到端联调排期。

**v1.1 修订摘要**

* **拆分定位**：明确 Console 是独立前端项目，不再写在 Agent 文档中；与后端 v1.3 的边界、版本矩阵和接口契约对齐。
* **修复字段绑定**：CPU 使用 `cpu.total.usage_ratio/iowait_ratio/steal_ratio`；网络使用 `receive_errors_total/transmit_errors_total/receive_dropped_total/transmit_dropped_total`；`push_auth_failed` 只从 `health.checks[]` 读取。
* **修复隐私矛盾**：删除“公开匿名 + 完整脱敏 + 与 6.2 100% 一致”的自相矛盾；改为“公开总览默认脱敏；单机详情默认认证；公开详情需显式开启白名单”。
* **修复容量承诺**：Free 推荐 **≤10 台**；10–20 台需 Paid/优化；只存 latest + 1 分钟聚合，history 不再假设完整快照时序。
* **补齐接口契约**：新增 `overview`、`detail`、`history` 的正式响应结构、错误体、鉴权、缓存与分页/窗口约定。
* **补齐 D1 模型**：新增 `agents`、`snapshots_latest`、`metrics_1m`、`ingest_batches` 表、幂等键、保留与清理策略。
* **补齐开发/部署/FAQ/变更记录**：新增第 4、9、10、11、13 章与附录 B，删除过时的“Agent v1.1 协议”“20 台 0 元”“本地 9101/ui”等表述。

## 目录

1. 目标、定位与范围
2. 术语、角色与访问模式
3. 技术栈、目录与模块职责
4. 开发环境配置
5. 路由、页面与组件
6. 接口契约
7. D1 数据模型、保留与幂等
8. 状态、单位与异常展示
9. 部署与 CI/CD
10. 安全、缓存与额度
11. 测试与验收
12. 实施计划与依赖
13. 常见问题（FAQ）
14. 变更记录
附录 A 字段映射与组件绑定
附录 B 部署检查清单

---

## 1. 目标、定位与范围

### 1.1 定位

Sprobe Console UI 是面向 Sprobe 主机监控数据的**只读展示前端**。它不采集、不修改、不控制被监控主机，只消费后端提供的快照/健康/能力数据，回答三个问题：

1. 集群里哪些节点在线、健康、接近容量上限？
2. 单节点 CPU、内存、磁盘、网络、负载、进程、uptime 的当前状态和短期趋势如何？
3. 数据为什么缺失、为什么告警、下一步该查什么？

### 1.2 目标

| 编号 | 目标 | 验收口径 |
| ----- | ----- | ----- |
| U1 | 5 分钟内看到集群总览 | 部署完成后匿名访问 `/overview`，首屏 LCP ≤2s |
| U2 | 单机详情可解释 | 8 类指标、12 项健康检查、能力清单、事件均有展示或明确空态 |
| U3 | 缺失不等于零 | 所有 `null`/不支持字段显示 `—` 并给出原因，不渲染为 0 |
| U4 | 只读安全 | 无写操作；公开视图不含 PID、完整主机名、敏感挂载路径 |
| U5 | 低成本 | Free 额度默认配置支持 ≤4–5 台（60s 批周期优化后 ≤8–10 台）；10–20 台有明确 Paid 路径 |
| U6 | 可运维 | 部署、回滚、缓存、限流、错误码均有文档和检查清单 |

### 1.3 范围

**纳入 1.0**

* 集群总览：节点在线/失联、健康分布、核心 KPI 排序、按 labels/os/arch/health 筛选。
* 单机详情：CPU、内存/Swap、文件系统、磁盘 I/O、网络接口、负载、进程聚合/Top-N、uptime、健康检查、能力、事件、WAL/推送摘要。
* 8 类空/异常态：首次加载、预热、字段缺失、分区 partial/timeout/failed、stale、合法空态、集群空/筛选空、失联/鉴权失败。
* 浅色/深色主题、三档响应式、键盘可达、色弱友好。
* GitHub + Cloudflare Pages 部署；形态 B 的 Functions + D1；CI、回滚与基础监控。

**不纳入 1.0**

* 写操作：改配置、重启/终止进程、执行命令、远程修复。
* 多租户管理后台、复杂 RBAC、审计审批流。
* 浏览器端长期保存完整历史时序；history 只查 1 分钟聚合。
* 告警通知渠道（邮件/IM/电话）；通知由后端/监控平台负责。
* 移动端原生 App；只做响应式 Web。

### 1.4 设计取舍

| 取舍 | 选择 | 理由 | 代价 |
| ----- | ----- | ----- | ----- |
| 公开 vs 认证 | 公开总览 + 默认认证详情 | 公开页有价值，但详情含主机名/进程/挂载点，默认不暴露 | 公开详情需额外白名单与评审 |
| 静态 vs 边缘 | 同一套 SPA，形态 A/B 切换 | 演示/状态页零成本；中小集群可闭环 | 形态 B 受 Workers/D1 额度约束 |
| 完整快照 vs 投影 | 详情返回快照；公开总览返回投影 | 详情字段完整，排障方便；总览小、可缓存 | 需要两套类型和字段白名单 |
| 历史 vs 最新 | latest + 1 分钟聚合 | 控制 D1 行数与成本；趋势够用 | 不能看秒级历史；窗口受限 |
| 轮询 vs 推送 | 前端轮询 + Edge Cache | 实现简单、可缓存、断线可恢复 | 实时性受 10s 缓存影响 |

---

## 2. 术语、角色与访问模式

### 2.1 术语

| 术语 | 含义 |
| ----- | ----- |
| Agent / 探针 | 部署在被监控主机上的 Sprobe 进程 |
| Snapshot / 快照 | 一次采集形成的完整 JSON 对象，`schema_version=1.0` |
| Section / 分区 | 快照中的 `cpu`、`memory`、`filesystems`、`network` 等对象 |
| `agent_id` | 主机级稳定 UUID；同一主机重装需重新生成 |
| `instance_id` | Agent 进程实例 UUID；每次启动变化 |
| `sequence` | 快照在实例内递增序号；与 `agent_id`、`instance_id` 组成幂等键 |
| `health.status` | `ok` / `degraded` / `critical` / `unknown` |
| `collection.capabilities` | 平台能力布尔表；`false` 表示不支持/不可用 |
| `quality` | `measured` / `derived` / `estimated` / `partial` |
| 公开视图 | 匿名可读的脱敏投影，不含敏感字段 |
| 认证视图 | Cloudflare Access 保护后的完整快照视图 |

### 2.2 角色

| 角色 | 关注点 | 权限 |
| ----- | ----- | ----- |
| 值班 SRE | 哪个节点异常、是否需要介入 | 认证视图 |
| 平台研发 | 字段、趋势、能力缺失原因 | 认证视图 |
| 访客/业务方 | 服务是否正常、集群整体健康 | 公开总览 |
| 安全/合规 | 是否泄露主机名、PID、路径 | 公开视图评审 |
| 前端研发 | 契约、状态、性能、部署 | 开发环境 |

### 2.3 两种访问模式

| 模式 | 入口 | 数据 | 鉴权 | 缓存 |
| ----- | ----- | ----- | ----- | ----- |
| `public` 公开状态页 | `/overview` | 集群汇总 + 节点摘要（脱敏） | 无 | `s-maxage=10`，可边缘缓存 |
| `private` 认证控制台 | `/overview`（匿名可读公开态）、`/nodes/:agent_id`（必须认证） | 完整快照 + 历史聚合 | Cloudflare Access 保护 `/nodes/*`、`/api/readonly/nodes/*` 与 `?view=full` | `private, no-store`（history `private, max-age=15`） |

* 默认 `VITE_ACCESS_MODE=public`；需要单机详情时设为 `private` 并配置 Access。
* `public` 模式访问 `/nodes/:id` 显示“需要登录”，不返回数据；不把“详情接口 401”伪装成空态。
* `private` 模式下 Cloudflare Access 仅拦截 `/nodes/*`、`/api/readonly/nodes/*` 及 `/api/readonly/overview?view=full`，确保访客仍可匿名访问 `/overview`。当 SPA 从 `/overview` 客户端路由跳转到 `/nodes/:id` 并调用 `fetch` 遇到 `401` 或 Access 返回的 `302`/opaque 重定向响应时，前端禁止 `fetch` 跟随跨域跳转，必须调用 `window.location.assign('/login?next=...')` 触发整页认证重定向。
* 如确需公开详情，必须显式设置 `PUBLIC_DETAIL=true`，并只返回 6.6 定义的白名单字段；该开关默认关闭，需安全评审。

---

## 3. 技术栈、目录与模块职责

### 3.1 技术栈

| 层面 | 选型 | 约束 |
| ----- | ----- | ----- |
| 构建 | Vite + TypeScript（strict） | 禁止 `any` 绕过契约；构建产物可静态托管 |
| UI | React 18+ 或 Vue 3（二选一，团队定） | 不引入重型企业中台套件；首屏 JS gzip ≤200 KiB |
| 样式 | 原生 CSS Variables + CSS Modules/Tailwind | 语义令牌统一；数值 `tabular-nums` |
| 图表 | ECharts 按需或 uPlot | 必须支持 `null` 断点不连线；不全量打包 |
| 数据层 | fetch + AbortController + 轻量缓存 | 不做全局长期缓存；切页取消请求 |
| 边缘 | Cloudflare Pages Functions（`functions/`） | 仅形态 B；同域、免 CORS |
| 存储 | Cloudflare D1 | 只存 latest + 1 分钟聚合；不存完整历史快照 |
| 测试 | Vitest + Testing Library + Playwright | 契约、异常态、三档视口、A11y |

### 3.2 目录

```text
sprobe-console/
├── public/
│   ├── _redirects                 # SPA 回退
│   ├── _headers                   # 安全头与缓存
│   └── mock/                      # 静态演示：overview.json / node.json / history.json
├── functions/
│   └── api/
│       ├── v1/
│       │   ├── ingest.ts          # Agent 批次接收
│       │   └── heartbeat.ts       # Agent 心跳
│       └── readonly/
│           ├── overview.ts
│           └── nodes/
│               ├── [agent_id].ts
│               └── [agent_id]/
│                   └── history.ts
├── migrations/                    # D1 schema 与索引
├── src/
│   ├── types/                     # Snapshot / Overview / History / Error 类型
│   ├── api/                       # 数据源适配：static | api | same-origin
│   ├── utils/                     # 单位、时间、空值、脱敏、排序
│   ├── components/                # 徽章、KPI、趋势、表格、空态、能力浮层
│   ├── pages/                     # Overview / NodeDetail / Login / NotFound
│   ├── styles/                    # tokens.css、布局、响应式
│   └── main.tsx
├── tests/                         # unit / contract / e2e / fixtures
├── wrangler.toml
├── package.json
└── package-lock.json
```

### 3.3 模块职责

| 模块 | 职责 | 不负责 |
| ----- | ----- | ----- |
| `api/` | 拼 URL、鉴权头、超时/取消、错误归一化、缓存策略 | 业务格式化 |
| `types/` | 与后端 schema 1.0 对应的类型；字段可选性/空值 | 运行时校验（用 schema 校验器） |
| `utils/format` | ratio→%、bytes→KiB/MiB/GiB、bps、时长、UTC→本地 | 数据采集/推断 |
| `utils/redact` | 公开视图白名单、hostname/PID/路径脱敏 | 认证决策 |
| `components/` | 纯展示、可复用、可测试 | 直接 fetch |
| `pages/` | 路由、数据加载、错误边界、空态编排 | 底层格式化 |
| `functions/` | ingest 校验、D1 写入、只读查询、缓存头 | 页面渲染 |
| `migrations/` | D1 表、索引、保留策略 | 业务逻辑 |

---

## 4. 开发环境配置

### 4.1 工具链

| 项 | 要求 |
| ----- | ----- |
| Node.js | 22 LTS（`.nvmrc`/`engines` 锁定） |
| 包管理器 | npm（提交 `package-lock.json`）；团队可统一 pnpm，但 CI 只认一种 |
| TypeScript | 5.x，`strict=true`，`noUncheckedIndexedAccess=true` |
| Wrangler | 与 Cloudflare Pages/Functions 当前兼容版本，锁定在 `devDependencies` |
| 浏览器 | 最近两个 Chrome/Edge/Firefox/Safari；移动端 Safari/Chrome |
| 代码质量 | ESLint + Prettier；`npm run lint` 必须零 error |

### 4.2 环境变量

| 变量 | 作用 | 默认 | 是否公开 |
| ----- | ----- | ----- | ----- |
| `VITE_DATA_MODE` | `static` / `api` | `static` | 是 |
| `VITE_API_BASE_URL` | 只读 API 基地址；同域留空 | `""` | 是 |
| `VITE_ACCESS_MODE` | `public` / `private` | `public` | 是 |
| `VITE_POLL_INTERVAL_MS` | 轮询间隔 | `10000` | 是 |
| `VITE_SITE_TITLE` | 页面标题 | `Sprobe Status` | 是 |
| `SPROBE_INGEST_TOKENS` | 开发/≤5 台演示期 JSON 令牌映射（生产推荐 D1 `ingest_tokens` 表） | 无 | **否，Secrets** |
| `SPROBE_CLEANUP_TOKEN` | 受保护清理端点鉴权令牌（供 Cron Worker/Actions 调用） | 无 | **否，Secrets** |
| `SPROBE_PUBLIC_DETAIL` | 是否允许公开详情 | `false` | 否，Functions 环境变量 |
| `SPROBE_ACCESS_TEAM_DOMAIN` | Cloudflare Access 团队域 | 无 | 否 |

* 所有 `VITE_*` 会进入前端产物，**不得**放令牌、私钥、内部地址。
* 本地 `.env.local` 不提交；仓库只提交 `.env.example`。

### 4.3 本地启动

```bash
npm ci
npm run dev                 # Vite，默认 http://127.0.0.1:5173
npm run typecheck
npm run lint
npm test -- --run
npm run build
```

形态 B 本地联调：

```bash
npx wrangler d1 create sprobe-metrics     # 首次
npx wrangler d1 migrations apply SPROBE_DB --local
npx wrangler pages dev dist --d1=SPROBE_DB --compatibility-date=2026-11-01
```

* 本地 D1 数据在 `.wrangler/state`，不提交。
* 没有 Cloudflare 账号时，用 `VITE_DATA_MODE=static` + `public/mock/` 完成页面开发；mock 必须覆盖 8 类异常态。注意：形态 A（`static`）仅用于本地开发与静态演示，不承担生产实时监控数据更新。
* 契约测试使用 `tests/fixtures/` 的黄金 JSON；字段变更先改 fixture 与类型，再改组件。

### 4.4 提交前门禁

| 检查 | 命令 | 通过标准 |
| ----- | ----- | ----- |
| 类型 | `npm run typecheck` | 零 error |
| Lint | `npm run lint` | 零 error |
| 单测 | `npm test -- --run` | 全绿，核心格式化/空态覆盖 |
| 构建 | `npm run build` | 成功，首屏 JS ≤200 KiB gzip |
| 契约 | `npm run test:contract` | overview/detail/history/error schema 全通过 |
| E2E | `npm run test:e2e` | 三档视口、路由刷新、空态通过 |

---

## 5. 路由、页面与组件

### 5.1 路由

| 路径 | 页面 | 访问 | 说明 |
| ----- | ----- | ----- | ----- |
| `/` | 重定向 | 公开 | 302/客户端跳 `/overview` |
| `/overview` | 集群总览 | 公开 | 汇总 + 节点表 |
| `/nodes/:agent_id` | 单机详情 | 认证（默认） | 公开模式显示登录提示 |
| `/login` | 登录/无权限 | 公开 | Access 跳转失败时的手动入口 |
| `*` | 404 | 公开 | 返回总览 |

### 5.2 总览页

* 顶部：站点标题、环境标签、刷新状态、主题切换、登录/退出。
* 汇总卡：总节点、在线、失联、健康分布（ok/degraded/critical/unknown）、高负载/高磁盘节点数。
* 筛选：`labels`、`os`、`arch`、`health.status`、关键词（`display_name`/`agent_id`）。
* 节点表：显示名、OS/架构、健康、CPU%、内存%、load1、uptime、最近上报、在线状态；行点击进入详情（无权限则登录）。
* 空态：无节点显示部署引导；筛选无结果显示重置按钮。

### 5.3 单机详情页

采用“顶部上下文 + 异常横幅 + 四层信息流 + 诊断抽屉”：

1. 顶部：返回、节点切换、`os · arch · virtualization`、健康徽章、`collected_at` 与 `age_seconds`、刷新控制、主题切换。
2. L0 横幅：`warmup`、`health.status != ok`、`clock_adjusted`、请求失联时显示；可跳转对应区块。
3. L1 KPI：CPU、内存/Swap、负载、uptime、健康/Agent 摘要。
4. L2 趋势：CPU 模式拆分（`cpu_usage_ratio`/`cpu_iowait_ratio`/`cpu_steal_ratio`，`idle` 由 `1 - usage - iowait - steal` 派生）、内存构成（`memory_used_bytes`/`memory_cached_bytes`/`swap_used_bytes`）、物理网卡汇总速率（`metrics_1m` 仅汇总 `class == "physical"` 接口，界面显式标注口径；单接口实时速率在网络卡片展示）。
5. L3 存储与进程：文件系统表、磁盘 I/O 表（按 `disk_class` 过滤）、进程状态分布、watch、Top-N。
6. L4 抽屉：12 项健康检查、能力矩阵、事件、WAL/推送摘要（来自接口顶层 `agent_runtime`）。

### 5.4 组件绑定

| 组件 | 绑定字段（完整路径） | 规则 |
| ----- | ----- | ----- |
| 健康徽章 | `health.status`、`health.status_code`、`health.since`、`health.recovered_at` | 四态颜色 + 文字；`unknown` 不显示绿色 |
| CPU 卡 | `cpu.total.usage_ratio`、`cpu.total.iowait_ratio`、`cpu.total.steal_ratio`、`cpu.logical_count`、`cpu.quality` | `null` 显示 `—`；`estimated` 虚线徽标 |
| 内存卡 | `memory.usage_ratio`、`memory.used_bytes`、`memory.available_bytes`、`memory.available_source`、`memory.swap_total_bytes`、`memory.swap_usage_ratio` | swap total=0 显示“未配置交换区” |
| 负载卡 | `load.average_1m/5m/15m`、`load.average_1m_per_core`、`load.quality`、`load.processor_queue_length` | Windows 合成值标“估算 (EWMA)” |
| uptime 卡 | `uptime.host_seconds`、`uptime.host_boot_time`、`uptime.agent_seconds`、`uptime.clock_adjusted` | `clock_adjusted` 显示时钟警示 |
| 文件系统表 | `filesystems.status`、`filesystems.errors`、`filesystems.items[]` 的 `mountpoint/fstype/usage_ratio/available_bytes/reserved_bytes/inodes_usage_ratio/read_only/stale` | 分区无 `status`；状态看 section；90%/95% 变色 |
| 磁盘表 | `disks.status`、`disks.items[]` 的 `device/disk_class/read_bytes_per_second/write_bytes_per_second/utilization_ratio/read_latency_seconds` | 默认 `disk_class=physical`；不跨类求和 |
| 网络组 | `network.interfaces[]` 的 `name/class/up/receive_bytes_per_second/transmit_bytes_per_second/receive_errors_total/transmit_errors_total/receive_dropped_total/transmit_dropped_total` | 无“全网合计”；`up=false` 置灰 |
| 进程区 | `processes.visibility/count/running_count/sleeping_count/blocked_count/zombie_count/top[]/watch[]` | `top[].pid` 仅认证视图；公开视图不返回 |
| 诊断抽屉 | `health.checks[]`、`collection.capabilities`、`unsupported_reasons`、`events[]`、`agent_runtime.*` | `push_auth_failed` 从 `health.checks[]` 中按 `id` 查找；WAL/推送摘要取自 `agent_runtime` |

---

## 6. 接口契约

### 6.1 通用约定

* 所有只读接口为 `GET`，返回 `application/json`；时间统一 UTC RFC 3339。
* 响应顶层包含 `schema_version`、`generated_at`；错误使用统一错误体。
* 字段缺失/不支持用 `null` 或缺失字段，不返回 `0` 伪造；前端必须结合 `collection.capabilities` 解释。
* 分页：总览默认返回全部节点（≤200）；超过时用 `?cursor=&limit=`，`limit` 最大 100；响应含 `next_cursor`。
* 认证：`private` 模式由 Cloudflare Access 在边缘拦截；Functions 校验 `Cf-Access-Jwt-Assertion`，失败返回 401。`public` 模式只允许 `view=public`。
* 缓存：只有公开总览可边缘缓存；认证响应一律 `Cache-Control: private, no-store`。

### 6.2 `GET /api/readonly/overview`

请求参数：

| 参数 | 类型 | 默认 | 说明 |
| ----- | ----- | ----- | ----- |
| `view` | `public`/`full` | `public` | `full` 需认证；公开只返回白名单 |
| `os` | string | 空 | 过滤 |
| `arch` | string | 空 | 过滤 |
| `health` | string | 空 | `ok/degraded/critical/unknown` |
| `label` | string | 空 | `key:value`，可重复 |
| `cursor` | string | 空 | 分页 |
| `limit` | int | 50 | 1–100 |

响应示例（公开视图）：

```json
{
  "schema_version": "1.0",
  "generated_at": "2026-11-21T02:00:10Z",
  "view": "public",
  "summary": {
    "total": 10,
    "online": 9,
    "offline": 1,
    "health": { "ok": 7, "degraded": 1, "critical": 1, "unknown": 1 },
    "alerts": { "cpu_high": 1, "memory_high": 0, "disk_high": 2 }
  },
  "nodes": [
    {
      "agent_id": "5b0c2f7e-6d1a-4c2e-9b7f-2f3a1c9d0e11",
      "display_name": "web-01",
      "os": "linux",
      "arch": "x86_64",
      "labels": { "env": "prod", "role": "web" },
      "health": { "status": "ok", "since": "2026-11-21T01:55:00Z" },
      "kpis": {
        "cpu_usage_ratio": 0.2315,
        "memory_usage_ratio": 0.612,
        "load_average_1m": 1.24,
        "uptime_seconds": 864000
      },
      "collected_at": "2026-11-21T02:00:00Z",
      "online": true
    }
  ],
  "next_cursor": null
}
```

* `display_name` 优先取 `labels.display_name`；没有则用脱敏短 ID（如 `node-5b0c2f7e`），**不默认返回完整 hostname**。
* 公开视图对 `labels` 实施键白名单过滤（默认仅透传 `env`、`role`、`region`、`display_name`），避免泄露资产/业务敏感标签；不返回 `processes`、`filesystems.items[].mountpoint`、`network.interfaces[].name` 等敏感字段。
* `online` 判定口径：`now - max(last_push_at, last_heartbeat_at) <= 90s`（3×30s 周期；若节点改为 60s 周期则按 180s 计算）。
* `summary.alerts` 统计口径：统计各在线节点最近快照 `health.checks[]` 中 `id` 为 `cpu_saturation`、`memory_pressure`、`disk_space` 且 `status != "ok"` 的节点数。
* `full` 视图仅认证可用，字段可扩展，但不得改变已发布字段语义。

### 6.3 `GET /api/readonly/nodes/{agent_id}`

请求参数：

| 参数 | 类型 | 默认 | 说明 |
| ----- | ----- | ----- | ----- |
| `view` | `public`/`full` | `public` | `full` 需认证；`public` 需 `PUBLIC_DETAIL=true` |
| `include` | string | 空 | 可选 `history`，不改变主结构 |

响应示例（认证视图，内部 `snapshot` 严格对齐 `rust_v1_3.md` 6.2 与 `sprobe_contract_v1_0.schema.json`）：

```json
{
  "schema_version": "1.0",
  "generated_at": "2026-11-21T02:00:10.000Z",
  "view": "full",
  "agent_runtime": {
    "online": true,
    "last_push_at": "2026-11-21T02:00:02.345Z",
    "last_heartbeat_at": "2026-11-21T02:00:05.100Z",
    "wal_snapshots": 0
  },
  "snapshot": {
    "schema_version": "1.0",
    "snapshot_id": "8f0e6a7c-2b1d-4f6e-9c3a-7d5e1f0a2b44",
    "sequence": 128,
    "collected_at": "2026-11-21T02:00:00.000Z",
    "agent": {
      "agent_id": "5b0c2f7e-6d1a-4c2e-9b7f-2f3a1c9d0e11",
      "instance_id": "2a7d9c1e-4b6f-4e2a-8c1d-9f0b3a6e5d22",
      "version": "1.0.3",
      "hostname": "web-01",
      "os": "linux",
      "arch": "x86_64",
      "labels": { "env": "prod", "role": "web" }
    },
    "host": {
      "hostname": "web-01",
      "os_name": "Ubuntu 24.04.1 LTS",
      "kernel": "6.8.0-45-generic",
      "architecture": "x86_64",
      "virtualization": "kvm",
      "cpu_model": "AMD EPYC 7B13 64-Core Processor",
      "boot_time": "2026-11-11T02:00:00Z"
    },
    "collection": {
      "interval_seconds": 10,
      "warmup": false,
      "partial": false,
      "capabilities": {
        "cpu_modes": true,
        "cpu_frequency": true,
        "memory_available": true,
        "swap": true,
        "filesystem_inodes": true,
        "disk_io_counters": true,
        "network_speed": true,
        "process_top": true,
        "process_io": false,
        "cgroup_metrics": false
      },
      "unsupported_reasons": { "process_io": "permission_denied", "cgroup_metrics": "not_configured" }
    },
    "cpu": {
      "status": "ok",
      "quality": "measured",
      "sampled_at": "2026-11-21T01:59:59.980Z",
      "age_seconds": 0.020,
      "truncated": false,
      "errors": [],
      "logical_count": 8,
      "physical_count": 4,
      "total": { "usage_ratio": 0.2315, "idle_ratio": 0.7420, "iowait_ratio": 0.0210, "steal_ratio": 0.0055 },
      "cores": []
    },
    "memory": {
      "status": "ok",
      "quality": "measured",
      "sampled_at": "2026-11-21T01:59:59.985Z",
      "age_seconds": 0.015,
      "errors": [],
      "available_source": "linux_memavailable",
      "total_bytes": 17179869184,
      "available_bytes": 6657199309,
      "used_bytes": 10522669875,
      "free_bytes": 1073741824,
      "cached_bytes": 5583457485,
      "buffers_bytes": 134217728,
      "usage_ratio": 0.6120,
      "swap_total_bytes": 2147483648,
      "swap_used_bytes": 0,
      "swap_free_bytes": 2147483648,
      "swap_usage_ratio": 0.0000
    },
    "filesystems": { "status": "ok", "quality": "measured", "sampled_at": "2026-11-21T01:59:40.000Z", "age_seconds": 20.000, "truncated": false, "errors": [], "items": [] },
    "disks": { "status": "ok", "quality": "measured", "sampled_at": "2026-11-21T01:59:40.010Z", "age_seconds": 19.990, "truncated": false, "errors": [], "items": [] },
    "network": { "status": "ok", "quality": "measured", "sampled_at": "2026-11-21T01:59:59.990Z", "age_seconds": 0.010, "truncated": false, "errors": [], "interfaces": [] },
    "load": {
      "status": "ok",
      "quality": "measured",
      "sampled_at": "2026-11-21T01:59:59.992Z",
      "age_seconds": 0.008,
      "errors": [],
      "average_1m": 1.24,
      "average_5m": 1.10,
      "average_15m": 0.98,
      "average_1m_per_core": 0.155,
      "processes_running": 2,
      "processes_blocked": 0,
      "processor_queue_length": null
    },
    "processes": {
      "status": "ok",
      "quality": "measured",
      "sampled_at": "2026-11-21T01:59:40.020Z",
      "age_seconds": 19.980,
      "errors": [],
      "visibility": "full",
      "count": 412,
      "running_count": 2,
      "sleeping_count": 409,
      "blocked_count": 0,
      "zombie_count": 1,
      "thread_count": 1380,
      "truncated": false,
      "top": [],
      "watch": []
    },
    "uptime": {
      "status": "ok",
      "quality": "measured",
      "sampled_at": "2026-11-21T01:59:59.995Z",
      "age_seconds": 0.005,
      "errors": [],
      "host_seconds": 864000,
      "host_boot_time": "2026-11-11T02:00:00Z",
      "agent_seconds": 3600,
      "clock_adjusted": false
    },
    "health": { "status": "ok", "status_code": 0, "since": "2026-11-21T01:55:00Z", "recovered_at": null, "checks": [] },
    "events": []
  }
}
```

* `snapshot` 必须与后端 6.2 100% 一致；前端只读，不补字段、不改单位。
* `agent_runtime` 由服务端根据最近一次 `ingest` 与 `heartbeat` 合成，支撑单机详情 L4 的 WAL/推送摘要展示。
* `view=public` 且 `PUBLIC_DETAIL=false` 时返回 `403 public_detail_disabled`；不要用空快照伪装。
* 公开详情白名单：仅 `agent.os`/`agent.arch`、`host.os_name`/`host.architecture`、`health`、`cpu.total`、`memory`、`filesystems` 的容量/使用率、`disks` 的聚合吞吐、`network` 的分类速率、`load`、`uptime`；**不含** `hostname`、`kernel`、`processes`、`mountpoint`、接口名、PID。

### 6.4 `GET /api/readonly/nodes/{agent_id}/history`

请求参数：

| 参数 | 类型 | 默认 | 范围 | 说明 |
| ----- | ----- | ----- | ----- | ----- |
| `window` | string | `1h` | `15m/1h/6h/24h` | 查询窗口 |
| `step` | string | `60s` | `60s/300s` | 聚合步长；24h 建议 300s |

响应示例：

```json
{
  "schema_version": "1.0",
  "generated_at": "2026-11-21T02:00:10Z",
  "agent_id": "5b0c2f7e-6d1a-4c2e-9b7f-2f3a1c9d0e11",
  "window": "1h",
  "step": "60s",
  "network_aggregation": "physical_only",
  "points": [
    {
      "ts": "2026-11-21T01:00:00Z",
      "cpu_usage_ratio": 0.21,
      "cpu_iowait_ratio": 0.02,
      "cpu_steal_ratio": 0.005,
      "memory_usage_ratio": 0.60,
      "memory_used_bytes": 10307921510,
      "memory_cached_bytes": 5368709120,
      "swap_used_bytes": 0,
      "load_average_1m": 1.1,
      "receive_bytes_per_second": 1250000,
      "transmit_bytes_per_second": 430000
    },
    {
      "ts": "2026-11-21T01:01:00Z",
      "cpu_usage_ratio": null,
      "cpu_iowait_ratio": null,
      "cpu_steal_ratio": null,
      "memory_usage_ratio": 0.61,
      "memory_used_bytes": 10479720202,
      "memory_cached_bytes": 5261334937,
      "swap_used_bytes": 0,
      "load_average_1m": 1.2,
      "receive_bytes_per_second": 1300000,
      "transmit_bytes_per_second": 410000
    }
  ],
  "truncated": false
}
```

* 只查 `metrics_1m` 聚合，不查完整快照；没有数据的点为 `null`，不补 0。
* `receive_bytes_per_second`/`transmit_bytes_per_second` 在服务端写入 `metrics_1m` 时仅汇总 `class == "physical"` 的非从属物理接口（避免 bond/bridge/veth 重复计数，与后端 4.7.2 原则兼容），无物理接口时为 `null`。
* 公开模式调用 history 返回 401/403；公开总览不展示 history。

### 6.5 错误体

```json
{
  "error": {
    "code": "not_found",
    "message": "agent not found",
    "retryable": false,
    "request_id": "req_01J8Z3K4",
    "server_time": "2026-11-21T02:00:10.000Z"
  }
}
```

| HTTP | `code` | 前端行为 |
| ----- | ----- | ----- |
| 400 | `invalid_query` | 显示参数错误，提示重置筛选 |
| 401 | `unauthenticated` | 跳转登录；不清空已显示数据 |
| 403 | `forbidden` / `public_detail_disabled` | 显示无权限说明，不伪装空态 |
| 404 | `not_found` | 节点不存在页，返回总览 |
| 429 | `rate_limited` | 按 `Retry-After` 退避，显示“请求过多” |
| 500 | `internal` | 局部错误态 + 重试；不白屏 |
| 503 | `upstream_unavailable` | 显示“数据暂不可用”，保留上次快照并置灰 |

### 6.6 缓存与鉴权矩阵

| 端点 | 公开 | 认证 | 缓存 |
| ----- | ----- | ----- | ----- |
| `/api/readonly/overview?view=public` | 允许 | 允许 | `public, s-maxage=10, stale-while-revalidate=30` |
| `/api/readonly/overview?view=full` | 拒绝 | 允许 | `private, no-store` |
| `/api/readonly/nodes/{id}?view=public` | 仅 `PUBLIC_DETAIL=true` | 允许 | `public, s-maxage=10` |
| `/api/readonly/nodes/{id}?view=full` | 拒绝 | 允许 | `private, no-store` |
| `/history` | 拒绝 | 允许 | `private, max-age=15` |

* 认证响应禁止写入共享缓存；`Vary: Authorization, Cf-Access-Jwt-Assertion`。
* 公开总览的 `ETag` 用 `generated_at` + 节点数生成；304 必须不返回敏感字段。

---

## 7. D1 数据模型、保留与幂等

### 7.1 表

| 表 | 主键/唯一键 | 用途 | 保留 |
| ----- | ----- | ----- | ----- |
| `agents` | `agent_id` | 节点元数据、心跳/WAL 摘要、最近健康与进度 | 永久（失联 30 天后可归档） |
| `snapshots_latest` | `agent_id` | 每节点最新完整快照 | 1 行/节点 |
| `metrics_1m` | `(agent_id, bucket_ts)` | 1 分钟聚合，供 history | 7 天（独立 Cron 清理） |
| `ingest_batches` | `(agent_id, instance_id, batch_id)` | 批次幂等/审计 | 24 小时（独立 Cron 清理） |
| `ingest_tokens` | `token_hash` | 节点独立令牌绑定与 24h 双令牌轮换 | 按轮换策略清理 |

### 7.2 D1 表结构示例

```sql
CREATE TABLE agents (
  agent_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  labels_json TEXT NOT NULL DEFAULT '{}',
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  last_push_at TEXT,
  last_heartbeat_at TEXT,
  wal_snapshots INTEGER NOT NULL DEFAULT 0,
  last_instance_id TEXT,
  last_sequence INTEGER NOT NULL DEFAULT -1,
  last_collected_at TEXT,
  health_status TEXT NOT NULL DEFAULT 'unknown',
  health_json TEXT,
  capabilities_json TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE snapshots_latest (
  agent_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  collected_at TEXT NOT NULL,
  snapshot_json TEXT NOT NULL,
  received_at TEXT NOT NULL
);

CREATE TABLE metrics_1m (
  agent_id TEXT NOT NULL,
  bucket_ts TEXT NOT NULL,
  cpu_usage_ratio REAL,
  cpu_iowait_ratio REAL,
  cpu_steal_ratio REAL,
  memory_usage_ratio REAL,
  memory_used_bytes INTEGER,
  memory_cached_bytes INTEGER,
  swap_used_bytes INTEGER,
  load_average_1m REAL,
  receive_bytes_per_second REAL,
  transmit_bytes_per_second REAL,
  health_status TEXT,
  PRIMARY KEY (agent_id, bucket_ts)
);

CREATE TABLE ingest_batches (
  agent_id TEXT NOT NULL,
  instance_id TEXT NOT NULL,
  batch_id TEXT NOT NULL,
  first_sequence INTEGER NOT NULL,
  last_sequence INTEGER NOT NULL,
  received_at TEXT NOT NULL,
  status TEXT NOT NULL,
  PRIMARY KEY (agent_id, instance_id, batch_id)
);

CREATE TABLE ingest_tokens (
  token_hash TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  expires_at TEXT,
  revoked INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);
CREATE INDEX idx_ingest_tokens_agent ON ingest_tokens(agent_id);
```

* **单批事务与 10ms CPU 约束**：形态 B 每次 `ingest` 必须将 `ingest_batches`、`agents`、`snapshots_latest`、`metrics_1m` 合并在单次 `env.SPROBE_DB.batch([...])` 中提交；Agent 接入形态 B 时建议配置 `batch_max_snapshots = 3`、`batch_max_bytes = 262144`（256 KiB），避免超过 Workers Free 10ms CPU 上限。
* **心跳入库（`POST /api/v1/heartbeat`）**：校验令牌与 `agent_id` 绑定后，更新 `agents` 的 `last_seen`、`last_heartbeat_at`、`health_status`、`wal_snapshots`（若 `last_push_at` 距今 < 30s 且 `wal_snapshots` 未变化，可跳过写 D1 以节省行写入额度）。
* **定时清理**：Cloudflare Pages Functions **不支持原生 Cron Triggers**。`metrics_1m`（7 天）与 `ingest_batches`（24 小时）的清理必须由独立轻量 Cron Worker 或 GitHub Actions 定时调用受 `SPROBE_CLEANUP_TOKEN` 保护的清理端点完成，严禁在 `ingest` 请求中顺带执行 `DELETE`。

### 7.3 幂等与顺序

* **批次级幂等**：`ingest_batches` 主键 `(agent_id, instance_id, batch_id)` 命中且 `status=accepted` 时，直接返回 `200` + `duplicates = snapshot_count`，不重复写表。
* **单快照级幂等（应对 413 拆批与重启重放）**：当生成新 `batch_id` 但复用旧 `sequence` 时，按 `agents.last_instance_id` 与 `agents.last_sequence` 判定：同一 `instance_id` 内 `sequence <= last_sequence` 的快照视为幂等重复计入 `duplicates`，不重复覆盖 `snapshots_latest` 且不重复聚合。
* **实例切换（Agent 重启）**：`instance_id` 变化表示进程重启，允许新实例从较小 `sequence`（如 0）开始；跨 `instance_id` 时**以 `collected_at` 新者覆盖 `snapshots_latest`**，同时更新 `agents.last_instance_id` 并重置 `last_sequence` 为新实例序号，避免新实例被旧实例的高 `sequence` 永久阻塞。
* **分钟桶聚合**：`metrics_1m` 按 `bucket_ts`（截断至分钟）使用 `INSERT ... ON CONFLICT(agent_id, bucket_ts) DO UPDATE` 覆盖写入该分钟内最新有效值，防止重复投递污染均值。
* **失败语义**：D1 写入失败返回 `500 internal`，Agent 按退避重试；严禁先 ACK 后落盘。

### 7.4 保留与容量（含全量写入与清理核算）

| 写入项 | 单节点/天（默认 30s 批 + 30s 心跳） | 5 节点/天 | 10 节点/天 | 说明 |
| ----- | ----- | ----- | ----- | ----- |
| `snapshots_latest` UPSERT | 2,880 行 | 14,400 行 | 28,800 行 | 每批 1 行 |
| `agents`（ingest + 心跳） | 5,760 行 | 28,800 行 | 57,600 行 | 心跳跳过优化后减半 |
| `metrics_1m` UPSERT | 1,440 行 | 7,200 行 | 14,400 行 | 每分钟 1 行 |
| `ingest_batches` INSERT | 2,880 行 | 14,400 行 | 28,800 行 | 24h 保留 |
| 定时清理 `DELETE`（D1 计入写入） | 4,320 行 | 21,600 行 | 43,200 行 | 删 batches + 1m 过期行 |
| **合计 D1 写入** | **~17,280 行** | **~86,400 行** | **~172,800 行** | **D1 Free 上限 100,000 行/天** |

* **Free 推荐 ≤4–5 台**（默认 30s 批 + 30s 心跳）；若调整 Agent 为 **60s 批周期 + 仅无快照时发心跳**，单节点降至 ~8,640 行/天，Free 可支撑 **≤8–10 台**。
* **10–20 台**：必须升级 Workers Paid（\$5/月，含更高 D1 写入与 30s CPU 上限），避免触顶导致 `503/429` 雪崩。
* D1 Free 存储 5 GB；10 节点 7 天约几十 MB，瓶颈在日写入行数与 CPU 时长而非存储容量。

---

## 8. 状态、单位与异常展示

### 8.1 单位

| 类型 | 规则 |
| ----- | ----- |
| `*_ratio` | 0–1，展示乘 100，保留 1 位小数；`null` 显示 `—` |
| `*_bytes` | 1024 进制，B/KiB/MiB/GiB/TiB，1–2 位小数 |
| `*_bytes_per_second` | 默认 MiB/s；可切换 Mbps，必须标注单位 |
| 时间 | 接口 UTC；展示浏览器本地时区；Tooltip 同时给 UTC 与 `age_seconds` |
| 时长 | `X天 X小时 X分`；小于 1 分钟显示秒 |
| 数值 | 等宽数字 `tabular-nums`；排序用原始数值，不用格式化字符串 |

### 8.2 八类异常态

| 场景 | 判定 | 展示 |
| ----- | ----- | ----- |
| 首次加载 | 请求未返回 | 固定尺寸骨架屏，CLS ≤0.05 |
| 预热 | `collection.warmup=true` | 信息横幅；速率/使用率显示“预热中”，不画 0 曲线 |
| 字段缺失/不支持 | `null` 或 `capabilities[k]=false` | `—` + 原因 Popover；不显示 0 |
| 分区局部失败 | `filesystems.status` 等为 `partial/timeout/failed` | 仅该分区提示 + `errors[]`，其他正常 |
| 陈旧 | `stale=true` 或服务端基准 `(generated_at - collected_at) > 2×周期`（禁止直接用浏览器本地 `Date.now()`） | 降对比度 + “陈旧”标签，不用于实时告警 |
| 合法空态 | `swap_total_bytes=0`、`watch=[]`、`events=[]` | 中性文案，不用错误色 |
| 集群/筛选空 | 节点数组为空 | 部署引导或重置筛选 |
| 失联/鉴权失败 | 请求连续失败；`health.checks[]` 中 `id=push_auth_failed` | 红色横幅，区分“失联/401/网络中断”；保留上次快照并置灰 |

* `push_auth_failed` 不是顶层字段：遍历 `health.checks[]` 找 `id`，`status=critical` 时展示。
* `health.status=unknown` 不等于“正常”；显示灰色未知态和原因。
* 错误边界按分区包裹，单卡片异常不白屏。

### 8.3 轮询与交互

* 默认 10s；`document.visibilityState=hidden` 时暂停，回前台立即补拉。
* `AbortController` 取消旧请求；避免慢响应覆盖新数据。
* 用户悬停/展开/滚动时，只更新数字单元格，暂缓表格重排。
* 不做乐观写操作（本系统只读）。

---

## 9. 部署与 CI/CD

### 9.1 Cloudflare Pages

1. GitHub 仓库连接 Cloudflare Pages；生产分支 `main`，构建命令 `npm run build`，输出 `dist`。
2. 环境变量：`VITE_DATA_MODE=api`、`VITE_API_BASE_URL=""`、`VITE_ACCESS_MODE=private`（如需详情）。
3. `public/_redirects`：`/* /index.html 200`。
4. `public/_headers`：安全头与缓存；`/index.html` 不缓存，`/assets/*` 长缓存。

### 9.2 Functions + D1（形态 B）

```toml
name = "sprobe-console"
compatibility_date = "2026-11-01"
pages_build_output_dir = "dist"

[[d1_databases]]
binding = "SPROBE_DB"
database_name = "sprobe-metrics"
database_id = "<wrangler d1 create 生成>"
```

* 创建 D1：`npx wrangler d1 create sprobe-metrics`；把 `database_id` 写入 `wrangler.toml`。
* 迁移：`npx wrangler d1 migrations apply SPROBE_DB --remote`。
* Secret：`npx wrangler pages secret put SPROBE_INGEST_TOKENS`；不得写进仓库。
* 自定义域名：Pages → Custom domains → `status.example.com`；开启 Always Use HTTPS。
* 回滚：Deployments → 选择上一个健康版本 → Rollback；D1 schema 必须向后兼容。

### 9.3 CI

| 阶段 | 内容 | 失败处理 |
| ----- | ----- | ----- |
| PR | typecheck、lint、unit、contract、build | 阻止合并 |
| 预览 | Pages 自动预览 URL | 人工验收 |
| 生产 | 合并 `main` 自动部署 | 失败回滚上一版本 |
| 安全 | `npm audit --audit-level=high`、Dependabot | 高危阻断发布 |

* CI 不持有 Cloudflare API Token；部署由 Pages Git 集成完成。
* 契约测试必须使用与后端 v1.3 6.2 一致的 fixture；字段变更先合后端 schema。

---

## 10. 安全、缓存与额度

### 10.1 安全

* 前端零密钥：`VITE_*` 不含令牌；仓库不含 `.env`、私钥、内部地址。
* ingest：Bearer 令牌（生产用 D1 `ingest_tokens` 存哈希并校验 `agent_id` 绑定，支持 24h 双令牌轮换）；校验 `schema_version`、批次大小（形态 B 建议 ≤3 快照 / 未压缩 ≤256 KiB，硬上限 4 MiB）、字段范围；失败返回统一嵌套错误体 `401/403/400/413/422`，不返回 200。
* 只读：公开总览脱敏；详情默认 Cloudflare Access；`view=full` 必须校验 JWT。
* 响应头：CSP、`X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy`、`Permissions-Policy`。
* 数据最小化：公开视图不返回 hostname、kernel、PID、mountpoint、接口名、完整 cmdline。
* 供应链：提交 lockfile、锁定 Wrangler 版本、Dependabot、发版前 audit。

### 10.2 缓存

| 内容 | 策略 |
| ----- | ----- |
| `index.html` | `no-cache, no-store, must-revalidate` |
| `/assets/*` | `public, max-age=31536000, immutable` |
| 公开 overview | `public, s-maxage=10, stale-while-revalidate=30` |
| 认证 detail/history | `private, no-store` 或 `private, max-age=15` |
| 错误响应 | `no-store` |

### 10.3 免费额度与容量

| 项 | Free | 说明 |
| ----- | ----- | ----- |
| Pages 静态请求 | 不限量 | 500 次构建/月 |
| Workers/Pages Functions | 100,000 请求/天 | 00:00 UTC 重置；含 API 与页面 Functions |
| Workers CPU | 10ms/请求 | gzip + JSON + D1 写入需实测；超限转 Paid |
| D1 | 5 GB；约 10 万行写入/天 | 本模型 10 节点约 7.2 万行/天 |
| GitHub Actions | 2,000 分钟/月 | PR 门禁 |

**容量结论**

* **Free 推荐 ≤4–5 台（默认 30s 周期）**：计入 `agents` 更新、心跳与清理 `DELETE` 后，5 台 D1 写入约 86,400 行/天，接近 100,000 行/天上限；同时需注意认证视图（`no-store`）的前端轮询消耗 Workers 请求数。
* **Free 优化上限 ≤8–10 台**：将 Agent 推送周期调为 60s 且仅在无快照上报时发送心跳，同时公开总览依赖 10s Edge Cache。
* **10–20 台**：必须升级 Workers Paid（\$5/月），同步解决 D1 写入行数、100k 请求/天与 10ms CPU 限制。

### 10.4 监控

* 记录 Functions 的 `ingest` 成功率、401/413/500 计数、D1 写入延迟、缓存命中率。
* 公开总览被刷时，用 Cloudflare WAF 限流（单 IP 10s 内 >30 次返回 429）；不要用前端限流替代。
* 每日检查 D1 行数/存储与 Workers 请求量；接近 80% 时告警。

---

## 11. 测试与验收

### 11.1 分层

| 层 | 内容 | 工具 |
| ----- | ----- | ----- |
| 单元 | 单位换算、空值、排序、脱敏、状态映射 | Vitest |
| 契约 | overview/detail/history/error 与后端 schema 1.0 对齐 | JSON Schema + fixtures |
| 组件 | KPI/表格/抽屉/空态/错误边界 | Testing Library |
| E2E | 路由刷新、筛选、主题、三档视口、登录跳转 | Playwright |
| 性能 | 首屏 JS、LCP、CLS | Lighthouse/CI |
| 安全 | 公开视图无敏感字段、CSP、缓存头 | 自动断言 + 人工评审 |

### 11.2 验收清单

| 维度 | 通过标准 |
| ----- | ----- |
| 公开访问 | `/overview` 匿名 200；HTTPS；刷新不 404 |
| 详情权限 | 未登录访问 `/nodes/:id` 跳登录；不泄露快照 |
| 字段一致 | 所有绑定字段在 fixture 中存在；无 `errors/dropped` 简写 |
| 缺失语义 | `null` 显示 `—`；无 0 伪造；能力原因可见 |
| 异常态 | 8 类场景全部通过；局部失败不影响其他卡片 |
| 性能 | 首屏 JS ≤200 KiB gzip；LCP ≤2s；CLS ≤0.05 |
| 部署 | 合并 `main` 2 分钟内生效；回滚 5 分钟内完成 |
| 额度 | Free 档按 5 节点默认配置（或 10 节点 60s 优化配置）压测 24h 不超 D1 行数/Workers 请求与 10ms CPU 预算 |

### 11.3 测试数据

* `tests/fixtures/` 至少包含：正常、预热、null、partial/timeout、stale、Windows 合成负载、失联、401、空集群。
* 每个 fixture 带 `schema_version` 与 `generated_at`；契约测试拒绝未知主版本。
* 公开视图测试必须断言不含 `hostname`、`kernel`、`pid`、`mountpoint`、接口名。

---

## 12. 实施计划与依赖

### 12.1 3 周计划

| 阶段 | 周期 | 交付 | 退出标准 |
| ----- | ----- | ----- | ----- |
| F1 骨架 | W1 | Vite+TS、类型、mock、路由、_headers/_redirects、Pages 上线 | 预览 URL 可访问；CI 绿 |
| F2 视图 | W2 | 总览、详情、8 类异常态、三档响应式、主题 | Mock 全场景通过；单测覆盖核心 |
| F3 接口与发布 | W3（W5 端到端联调） | Functions/D1、鉴权、history、契约桩压测、回滚 | W3 契约桩 24h 额度与验收全过；W5 与后端 M2 真实 Agent 完成端到端联调 |

### 12.2 依赖

| 依赖 | 需要时间 | 风险/应对 |
| ----- | ----- | ----- |
| 后端 v1.3 6.2/6.4 契约冻结 | W1 | 未冻结则先用 fixture，字段变更走契约测试 |
| 后端 W5 数据可用 | W5（M2） | 前端 W3 基于契约测试桩完成 Functions/D1 验收，W5 与真实 Agent 端到端联调 |
| Cloudflare 账号/D1/Access | W1 | 提前申请；无账号先做形态 A |
| 安全评审（公开字段） | W2 | 公开详情默认关闭，避免阻塞 |
| 域名/证书 | W3 | 可先用 `pages.dev` |

### 12.3 里程碑

* M1（W1 末）：Pages 预览可访问，公开总览 mock 可看。
* M2（W2 末）：详情/异常态/响应式通过，字段契约冻结。
* M3（W3 末）：形态 B 联通，10 节点压测与回滚演练通过。

---

## 13. 常见问题（FAQ）

**Q1：为什么公开页看不到单机详情？**
A：详情含主机名、进程、挂载点、接口名等敏感信息。默认公开只到总览；详情需 Cloudflare Access。确需公开详情时，显式开启 `PUBLIC_DETAIL=true` 并只返回白名单字段，且要过安全评审。

**Q2：为什么接口不返回 `network.interfaces[].errors`？**
A：后端字段是 `receive_errors_total`、`transmit_errors_total`、`receive_dropped_total`、`transmit_dropped_total`。前端必须用完整路径；`errors/dropped` 是旧文档的简写，已废弃。

**Q3：为什么 `push_auth_failed` 不是布尔字段？**
A：它是 `health.checks[]` 的一项，`id="push_auth_failed"`、`status="critical"`。前端遍历 checks 查找，不要读顶层字段。

**Q4：为什么 history 只有 1 分钟精度？**
A：D1 存完整 10s 快照会迅速超过 Free 行数与存储；1 分钟聚合足够看趋势，成本可控。需要秒级历史时接后端时序库，不把 D1 当时序数据库。

**Q5：Free 到底能撑多少台？**
A：默认 30s 周期下推荐 **≤4–5 台**（完整计入 `agents`、心跳与每日清理 `DELETE` 后约 8.6 万行/天，逼近 D1 Free 10 万行上限）。若调为 60s 周期且仅无快照时发心跳，可支撑 **≤8–10 台**；10–20 台必须升级 Workers Paid。

**Q6：公开总览为什么可以缓存，详情不行？**
A：公开总览是脱敏投影，可共享缓存；详情含敏感字段且按用户鉴权，必须 `private/no-store`，否则会跨用户泄露。

**Q7：Agent 重启后 history 会断吗？**
A：`instance_id` 变化，但 `metrics_1m` 按 `agent_id` 聚合，趋势连续；详情以最新快照为准，前端不要假设 sequence 跨实例连续。

**Q8：D1 写入失败怎么办？**
A：Functions 返回 500，Agent 重试；不得先 ACK 后写。前端显示“数据暂不可用”，保留上次快照并置灰。

**Q9：如何本地没有 Cloudflare 账号开发？**
A：`VITE_DATA_MODE=static` + `public/mock/`；所有页面和异常态都能用 fixture 开发。接口联调再用 `wrangler pages dev`。

**Q10：为什么公开视图的节点名不是 hostname？**
A：默认用 `labels.display_name` 或脱敏短 ID。hostname 可能含内部命名/角色信息；需要展示时由运维在 labels 中显式配置可公开名称。

**Q11：前端能改配置/重启进程吗？**
A：不能。Console 只读；所有写操作由独立运维通道完成，前端不提供按钮/接口。

**Q12：升级后旧 D1 数据还能用吗？**
A：schema 变更必须向后兼容；新增列用默认值，删除列先保留一个版本。回滚 Pages 版本不自动回滚 D1，发布前要备份/迁移评审。

---

## 14. 变更记录

| 版本 | 日期 | 类型 | 主要变更 |
| ----- | ----- | ----- | ----- |
| v1.2 | 2026-11-21 | M0 审核统一版 | 对齐 `rust_v1_3.md` 6.2 快照结构与 `capabilities` 键名；新增 `agent_runtime` 与心跳入库闭环；扩充 `metrics_1m` 支撑 L2 趋势图；修正单快照幂等与跨实例 `collected_at` 覆盖规则；重算 D1 写入行数并将 Free 调整为 ≤4–5 台（优化后 ≤8–10 台）；明确独立 Cron 清理、Access SPA 跳转、D1 `ingest_tokens` 与 W3/W5 联调节奏 |
| v1.1 | 2026-11-21 | 整理版 | 与后端 v1.3 对齐；拆分公开/认证视图；修复 CPU/网络/健康字段路径；新增正式 API schema、D1 模型、开发环境、部署、FAQ、变更记录；Free 容量改为 ≤10 台；删除本地 9101/ui、Agent v1.1 协议、20 台 0 元、完整脱敏 100% 一致等过时表述 |
| v1.0 | 2026-11-21 | 初版 | 建立 10 章结构、Vite+TS、Cloudflare Pages、3 周计划、8 类异常态 |

**兼容性说明**

* 只读 API 路径保持 `/api/readonly/*`；响应从“裸快照”调整为 `{schema_version, generated_at, view, snapshot}` 信封，前端需按 v1.1 适配。
* 公开总览新增 `display_name`，不再默认返回 `hostname`；私有详情仍保留完整 `host.hostname`。
* `history` 从“可选、无存储定义”改为“必须基于 `metrics_1m`”；没有数据返回 `null`，不补 0。
* 旧 `sprobe_v1_0.md` 保留为历史版本，不再作为实施依据。

---

## 附录 A 字段映射与组件绑定

| 语义 | JSON 路径 | 组件 | 展示/备注 |
| ----- | ----- | ----- | ----- |
| 健康状态 | `health.status` | 徽章/筛选 | 四态；`unknown` 灰色 |
| 健康检查 | `health.checks[]` | 诊断抽屉 | 按 `severity` 排序；`push_auth_failed` 按 id 查 |
| CPU 总使用率 | `cpu.total.usage_ratio` | CPU 卡/趋势 | 0–1 → % |
| CPU iowait | `cpu.total.iowait_ratio` | CPU 趋势 | 不含在 usage 内，单独展示 |
| CPU steal | `cpu.total.steal_ratio` | CPU 趋势 | 云主机重点 |
| 逻辑核数 | `cpu.logical_count` | CPU 卡 | 不是 `cpu.total.logical_count` |
| 内存使用率 | `memory.usage_ratio` | 内存卡 | 结合 `available_source` |
| 可用内存 | `memory.available_bytes` | 内存卡 | 不是 `free` |
| 文件系统状态 | `filesystems.status` | 存储区 | section 级，不是 item 级 |
| 文件系统使用率 | `filesystems.items[].usage_ratio` | 文件系统表 | 90/95% 变色 |
| 磁盘吞吐 | `disks.items[].read_bytes_per_second` | 磁盘表 | 按 `disk_class` 过滤 |
| 网络收包速率 | `network.interfaces[].receive_bytes_per_second` | 网络卡 | 按接口，不合计 |
| 网络错包 | `network.interfaces[].receive_errors_total` | 网络卡 | 完整字段名 |
| 网络丢包 | `network.interfaces[].transmit_dropped_total` | 网络卡 | 完整字段名 |
| 负载 1m | `load.average_1m` | 负载卡 | gauge，不是 counter |
| 每核负载 | `load.average_1m_per_core` | 负载卡 | 可 >1，不裁剪 |
| 进程数 | `processes.count` | 进程区 | 公开视图不返回明细 |
| Top 进程 | `processes.top[]` | 进程表 | `pid` 仅认证视图 |
| 开机时长 | `uptime.host_seconds` | uptime 卡 | 格式化 |
| 时钟调整 | `uptime.clock_adjusted` | uptime 卡 | 琥珀警示 |
| 能力 | `collection.capabilities` | 诊断抽屉 | `false` 解释 null |
| 质量 | `quality` | 各卡片 | `estimated` 虚线/徽标 |

## 附录 B 部署检查清单

| 检查 | 命令/位置 | 通过标准 |
| ----- | ----- | ----- |
| 构建 | `npm run build` | 成功，dist 完整 |
| 路由 | 直接刷新 `/overview`、`/nodes/:id` | 200，不 404 |
| 安全头 | DevTools/Response Headers | CSP、nosniff、DENY 存在 |
| 公开脱敏 | 公开 overview 响应 | 无 hostname/kernel/PID/mountpoint/接口名 |
| 鉴权 | 未登录访问 detail | 401/跳转登录，不返回快照 |
| 缓存 | `CF-Cache-Status` | 公开 overview HIT；认证响应不缓存 |
| D1 | `wrangler d1 execute ...` | 表/索引存在；迁移成功 |
| Secret | Pages Secrets | `SPROBE_INGEST_TOKENS` 存在，未进仓库 |
| 额度 | Cloudflare 分析 | 请求/行数 <80% 预算 |
| 回滚 | Pages Deployments | 5 分钟内回上一版本 |

**文档结束**

版本：v1.2（M0 审核修订统一版）
日期：2026-11-21
