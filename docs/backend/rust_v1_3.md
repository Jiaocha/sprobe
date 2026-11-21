# Rust 服务器探针（Sprobe）后端开发文档

> 文档版本：v1.3（M0 审核修订统一版）
>
> 编制日期：2026-11-21
>
> 适用产品：Sprobe 1.0（Agent 1.0.x，协议 `schema_version = "1.0"`）
>
> 文档性质：后端设计基线、实施约束与验收依据
>
> 目标读者：Rust 研发、SRE、测试、安全、运维
>
> 配套文档：《Sprobe Console UI 前端开发文档 v1.2》（`sprobe_v1_2.md`）、机读契约 `sprobe_contract_v1_0.schema.json`

**文档约定**

* “必须”表示强制要求，未满足不得进入验收；“应”表示推荐做法，偏离需在评审中记录理由；“可”表示可选能力。

* 本文不包含 Rust 代码实现。配置片段、JSON 片段、HTTP 契约属于接口与数据约定，不是实现代码。

* 文中所有阈值、频率、容量均为默认值，最终以配置 schema 为准；变更默认值必须同步更新本文与验收用例。

* 指标命名、单位、空值语义在全文档中保持一致；新增指标必须先在指标清单中登记，再进入开发。

* 本文只定义 Agent 与上报/拉取协议。Console UI 是独立前端项目，页面、只读 API 与部署见配套前端文档，不在本文交付范围内。

**v1.3 修订摘要（M0 审核统一）**

* **统一 HTTP 契约与错误体**：统一 6.4 与 6.12 的成功响应（同时包含 `schema_version` 与 `status`）和嵌套错误体 `{"error":{"code","message","retryable","request_id","server_time"}}`；对齐错误码表（纳入 `422 validation_failed`，统一 `500 internal`）；修正 6.12.1 心跳字段为顶层 `health_status`/`health_status_code`；统一请求级幂等头为 `X-Sprobe-Batch-Id`。
* **补齐快照字段与 PromQL 整机口径**：在 6.2 `cpu.cores[]` 补齐 `frequency_hertz`；11.4 `CPUHigh`/`CPUCritical` 改用基于 counter 的整机聚合表达式，避免分核序列触发单核误报。
* **拉取端 TLS 与本地 403 语义自洽**：明确 1.0 拉取端不内置 TLS（与 14.2 Q7 一致），开启 `allow_remote=true` 时改为启动 `warn` 并要求前置反代；修正 6.12.1 非回环返回 `403` 的矛盾描述。
* **内部阈值、周期与运维细节统一**：统一 CPU 告警窗口（5m）、`process_missing` 窗口（2 个 medium 周期 = 1m）、`warmup` 时长（第 0 轮基线 + 第 1 个 fast 周期约 10s）、WAL `fsync=batch` 断电丢数窗口（≤1 批，默认 30s）、在途批次数（1–3）、Windows 服务名（`SprobeAgent`）、故障注入数（22 项）；补充 K8s ConfigMap 符号链接热加载说明；修复末尾落款与附录 B 损坏链接。
* **版本矩阵**：产品 Sprobe 1.0；后端文档 v1.3；前端文档 v1.2（`sprobe_v1_2.md`）；机读契约 `sprobe_contract_v1_0.schema.json`；协议 `schema_version = "1.0"`。

**v1.2 修订摘要**

* **前后端拆分**：删除 v1.1 第 15 章“前端展示页面设计”，后端文档只保留 Agent、协议与部署边界；前端单独成册，消除“可视化大屏不在范围”与“新增 Console”的范围冲突。

* **接口与字段统一**：新增 6.12 接口速查与错误码；明确 `cpu.total.*`、`network.interfaces[].receive_errors_total`、`health.checks[]` 等完整 JSON 路径，前端不得使用简写字段。

* **可靠性修正**：`push_auth_failed` 首次 401/403 即锁存 critical，不适用“连续 2 次失败”规则；明确 WAL 跨重启的 `instance_id` 语义、批次不混实例、状态丢失时旧 WAL 隔离。

* **容量与验收统一**：断网/服务端不可用目标统一为 **1 小时**；WAL 默认上限提升为 **96 MiB**（`100663296` 字节），覆盖单快照 256 KiB × 1 小时的最坏情况。

* **平台边界**：FreeBSD 改为社区探索（Tier 3），不承诺官方构建产物；正式发布矩阵只保留 Linux musl/gnu、Windows、macOS。

* **阈值与告警一致**：11.4 告警规则统一引用 4.11 健康口径，指标名统一 `sprobe_` 前缀；内存、磁盘延迟、网络错误窗口统一为 5 分钟。

* **开发与运维补充**：新增 3.11 开发环境配置、7.11 快速部署与检查清单、15 常见问题、16 变更记录；目录与示例同步更新。

* **版本矩阵**：产品 Sprobe 1.0；后端文档 v1.2；前端文档 v1.1；协议 `schema_version = "1.0"`；Agent 1.0.x。v1.1/v1.0 文档保留为历史版本。

## 目录

 0. 执行摘要

 1. 项目目标与范围

 2. 总体架构与模块划分

 3. 功能模块设计与关键流程

 4. 指标采集设计

 5. 跨平台兼容策略

 6. 数据结构与接口约定

 7. 配置与部署

 8. 数据输出与上报

 9. 权限与安全

10. 错误处理与可靠性

11. 日志、自监控与告警

12. 测试与验收标准

13. 分阶段实施计划

14. 风险与开放问题

15. 常见问题（FAQ）

16. 变更记录

附录 A 指标速查表

附录 B 参考资料

## 0. 执行摘要

**版本与配套关系**

| 项 | 当前值 | 说明 |
| ----- | ----- | ----- |
| 产品版本 | Sprobe 1.0 | 首个正式交付基线 |
| 后端文档 | v1.3 | 本文，替代 v1.2 |
| 前端文档 | v1.2 | `sprobe_v1_2.md`，独立交付 |
| 机读契约 | `sprobe_contract_v1_0.schema.json` | 前后端共用 JSON Schema 单一事实来源 |
| Agent 版本 | 1.0.x | 与协议 schema 解耦 |
| 协议版本 | `schema_version = "1.0"` | 快照/批次/心跳共用 |
| 历史文档 | `rust_v1_2.md`、`rust_v1_1.md`、`sprobe_v1_1.md`、`sprobe_v1_0.md` | 仅存档，不再作为实施依据 |


Sprobe 是一款使用 Rust 编写的轻量级服务器探针，目标是在异构服务器集群中以单一静态二进制完成主机级指标采集、健康判断、结构化输出与安全上报。首个可交付版本（MVP）以 Linux x86_64/aarch64 与 Windows x86_64 为一级支持平台，同时把 macOS 纳入兼容设计，避免后续为跨平台重写采集层。

**MVP 必须达到的硬指标**

| 维度 | 目标 | 
| ----- | ----- | 
| 功能 | 覆盖 CPU、内存、磁盘容量与 I/O、网络、系统负载、进程、开机时长、基础健康状态 8 类指标 | 
| 平台 | Tier 1（Linux x86_64/aarch64 `musl` 静态二进制、Windows x86_64）必须通过验收；Tier 2（Linux `glibc`、macOS x86_64/arm64）在第三阶段达到可用 | 
| 性能 | 默认 10 秒采集周期下，单核 CPU 占用 ≤1%，常驻内存 ≤64 MiB，fast 采集 P95 ≤100 ms | 
| 可靠性 | 单采集器失败不导致整体失败；断网 1 小时不丢已采集数据（受缓冲区容量约束）；72 小时浸泡无内存/句柄泄漏 | 
| 安全 | 默认仅监听回环地址；上报必须 TLS；令牌或 mTLS 认证；不以明文记录密钥；基础指标无需 root | 
| 运维 | 单二进制、配置热加载、结构化日志、自监控指标、健康检查端点、systemd/Windows 服务/容器三种部署方式 | 
| 交付 | 10 周完成 1.0，含设计、实现、跨平台验证、安全加固与验收报告 | 

**五个关键取舍**

1. **单二进制 + 编译期采集器**：不引入动态插件，换取部署简单和攻击面可控；代价是新平台/新指标需要发版。

2. **推为主、拉为辅**：默认通过 HTTPS 批量推送，适配 NAT 和大规模节点；同时在回环地址暴露 Prometheus 端点，便于本地排障和已有监控体系接入。

3. **快照模型而非逐点流**：一次采集形成一个带版本号的快照，保留原始计数器并附带派生速率；牺牲少量实时性，换取原子性、可重放和协议简单。

4. **混合采集后端**：以跨平台库打底，关键指标直接读取操作系统原生接口；不把正确性完全押在单一第三方库上。

5. **缺失不等于零**：平台不支持或暂时不可读时输出 `null` 并标注能力/质量，绝不用 0 伪造；告警规则必须能区分“值为 0”和“没有数据”。

## 1. 项目目标与范围

### 1.1 背景

服务器集群通常混合了不同代际的 x86_64 与 ARM/aarch64 机器，操作系统、内核版本、容器运行时和权限模型不一致。现有方案常见三类问题：

* 依赖运行时或解释器，单机资源占用高，分发困难；

* 指标口径不统一，CPU、内存、磁盘在不同平台上的“使用率”含义不同，导致告警误报；

* 采集、上报、告警耦合，单点失败会拖垮整个采集周期，且缺少可解释的错误信息。

Sprobe 的定位是“主机侧的标准化数据源”：只负责可靠地采集、规范化、输出和自检，不承担长期存储、可视化、工单和自动修复。

### 1.2 项目目标

| 编号 | 目标 | 可度量标准 | 
| ----- | ----- | ----- | 
| G1 | 统一主机指标模型 | 8 类必需指标在 Tier 1 平台全部可用；同名指标跨平台语义、单位一致 | 
| G2 | 轻量与易部署 | 单一静态二进制；无运行时依赖；默认配置 5 分钟内完成部署 | 
| G3 | 异构兼容 | Linux x86_64/aarch64（`musl` 静态）、Windows x86_64 一级（Tier 1）；Linux `glibc`、macOS x86_64/arm64 二级（Tier 2）；FreeBSD 为社区探索（Tier 3，不承诺官方产物） | 
| G4 | 可靠采集 | 单采集器故障隔离；采集成功率 ≥ 99.9%（排除主机宕机）；断网可缓冲 | 
| G5 | 安全合规 | 最小权限、TLS、令牌/mTLS、密钥不落日志、供应链可审计 | 
| G6 | 可运维 | 配置热加载、结构化日志、自监控、健康检查、明确的错误码 | 
| G7 | 可验收 | 每项指标有口径、精度容差和对照工具；每阶段有退出标准 | 

### 1.3 范围

**纳入范围**

* 主机级资源指标：CPU、内存与交换、磁盘容量与 I/O、网络、系统负载、进程聚合与 Top-N、开机时长。

* 基础健康状态：采集器新鲜度、资源饱和度、磁盘可写性、关键进程存在性、上报链路状态。

* 本地采集与规范化；JSON 快照输出；Prometheus 文本暴露；HTTPS 批量上报。

* 配置加载/校验/热加载；日志；自监控；错误分类；离线缓冲。

* Linux（systemd）、Windows 服务、容器/Kubernetes DaemonSet 三种部署形态。

* 跨平台构建、测试、发布与升级回滚规范。

* 为独立 Console UI 提供稳定的快照、心跳、健康与能力契约；Console 实现与部署不在本文范围。

**不纳入范围（1.0）**

* 服务端长期存储、查询引擎、可视化大屏、告警通知渠道（邮件/短信/IM）；Console UI 作为独立前端项目实施，不在后端 Agent 交付范围内。

* 应用性能监控（APM）、链路追踪、日志采集与检索。

* 远程命令执行、文件下发、自动修复、补丁管理。

* 容器编排平台对象指标（Pod/Deployment 级）、Kubernetes 事件采集。

* GPU、eBPF、SNMP、数据库/中间件专项采集。

* 多租户权限体系；Sprobe 假定部署在单租户或由服务端做租户隔离。

Console UI 的页面、只读 API、D1 模型与部署见 `sprobe_v1_2.md`；后端只保证第 6 章契约稳定，不提供 `/ui`、查询引擎或长期存储。

### 1.4 非目标

以下事项明确不做，避免范围蔓延：

* 不做“全量进程逐指标长期上报”。默认只上报聚合值和 Top-N 进程，防止时序基数爆炸。

* 不做“自动发现所有挂载点和网卡并无限上报”。必须有默认过滤和数量上限。

* 不做“无权限时提权采集”。权限不足时降级并报告，不尝试绕过系统安全策略。

* 不做“agent 侧告警通知”。Agent 只输出健康状态与事件，告警判定、去重、抑制、通知由服务端负责。

### 1.5 角色与典型场景

| 角色 | 关注点 | 典型操作 | 
| ----- | ----- | ----- | 
| SRE | 主机是否健康、容量是否足够、告警是否可信 | 部署 agent、查看健康状态、处理磁盘/内存告警 | 
| 平台研发 | 指标口径、协议、扩展性 | 接入时序库、编写告警规则、扩展采集器 | 
| 安全 | 最小权限、传输与密钥、供应链 | 审核 systemd 权限、证书轮换、漏洞响应 | 
| 运维 | 批量安装、升级、回滚 | 使用配置管理工具分发、灰度升级、回滚 | 
| 测试 | 跨平台一致性、边界与故障 | 对照 `top/free/df/sar`、注入故障、验收 | 

典型场景：

1. 新机器入网：安装 agent，自动生成稳定 `agent_id`，5 分钟内出现在监控中。

2. 磁盘将满：agent 在本地健康检查中标记 `degraded`，服务端在 5 分钟窗口后触发 P1 告警。

3. 网络分区：agent 继续采集并写入本地环形缓冲，恢复后按序补发，服务端按幂等键去重。

4. ARM 机器替换 x86 机器：指标名、单位、告警规则不变，仅 `architecture` 标签变化。

### 1.6 假设与约束

* Linux Tier 1 要求内核 ≥ 4.14；3.10–4.13 为尽力支持（best-effort），其中 `MemAvailable` 需 ≥ 3.14，低版本走回退口径并标注质量。

* Linux 假定存在 `/proc`、`/sys`；不假定一定使用 systemd，但 systemd 是首选部署方式。

* Windows 假定 Windows 10/11 或 Windows Server 2019 及以上；性能计数器可用；服务默认以 LocalSystem 运行，支持降级账户。

* macOS 假定 13 及以上；仅作为开发/边缘节点支持，不作为大规模服务器主平台。

* Rust 工具链版本以 `rust-toolchain.toml` 锁定；第三方库升级必须通过 `cargo audit` 与兼容性测试。

* Agent 只主动向外发起连接；除本地只读端点外不监听公网端口。

* 服务端接收协议由本文定义；服务端实现不在本项目的代码交付范围内，但必须提供契约测试桩。

### 1.7 术语

| 术语 | 含义 | 
| ----- | ----- | 
| Agent / 探针 | 部署在被监控主机上的 Sprobe 进程 | 
| Collector / 采集器 | 负责某一类指标采集的独立模块 | 
| Snapshot / 快照 | 一次采集周期形成的、带 schema 版本的完整数据对象 | 
| Section / 分区 | 快照中按域划分的对象，如 `cpu`、`memory`、`disk` | 
| Counter / 计数器 | 自开机或自对象创建以来单调递增的值，重启后归零 | 
| Gauge / 瞬时值 | 可增可减的瞬时测量值 | 
| Ratio / 比率 | 归一化到 `[0, 1]` 的无量纲值；展示层再转换为百分比 | 
| 派生值 | 由相邻两次采样差分计算出的速率、利用率、延迟等 | 
| Tier | 支持等级：Tier 1 必须验收，Tier 2 可用，Tier 3 尽力支持 | 
| 能力（Capability） | 当前平台/权限/配置下某指标是否可采集的声明 | 
| 质量（Quality） | `measured`、`estimated`、`derived` 三档，表示数据可信程度 | 
| 健康状态 | `ok`、`degraded`、`critical`、`unknown` 四态 | 

## 2. 总体架构与模块划分

### 2.1 设计原则

1. **本地优先**：采集、规范化、健康判断在 agent 内完成；网络不可用时仍能工作。

2. **失败隔离**：任何采集器、编码器、输出端的失败都不得让整个 agent 崩溃或停止采集。

3. **契约先行**：先冻结指标清单、JSON Schema、错误码和 HTTP 契约，再写实现。

4. **能力驱动**：启动时探测平台与权限，形成能力清单；UI/规则据此解释“为什么没有数据”。

5. **有界资源**：队列、缓冲区、进程数、挂载点、网卡、日志量都有上限，超限时丢弃最旧数据并计数。

6. **可观测自身**：Agent 必须暴露自己的采集耗时、错误、积压、推送成功率，否则无法运维。

7. **安全默认**：默认最小权限、默认只监听回环、默认校验 TLS、默认不采集敏感命令行。

### 2.2 逻辑架构

```
                    ┌──────────────────────────────┐
                    │        Config / Lifecycle     │
                    │  load · validate · reload     │
                    └──────────────┬───────────────┘
                                   │ immutable snapshot
                                   ▼
┌──────────────┐   tick   ┌────────────────┐   SectionResult   ┌────────────────┐
│  Scheduler   │─────────▶│ Collector Core │──────────────────▶│  Normalizer    │
│  cadence ·   │          │ timeout · panic│                   │  units · null  │
│  no overlap  │          │ isolation      │                   │  derived rates │
└──────────────┘          └───────┬────────┘                   └───────┬────────┘
                                  │ platform calls                     │
                    ┌─────────────┴─────────────┐                      ▼
                    │ Linux / Windows / macOS   │              ┌────────────────┐
                    │ native backends           │              │ Snapshot Store │
                    └───────────────────────────┘              │ latest + seq   │
                                                               └───┬────────┬───┘
                                                                   │        │
                                          ┌────────────────────────┘        └──────────────────────┐
                                          ▼                                                    ▼
                                 ┌────────────────┐                                   ┌────────────────┐
                                 │ Prometheus HTTP│                                   │ JSON Encoder   │
                                 │ /metrics       │                                   │ + gzip         │
                                 └────────────────┘                                   └───────┬────────┘
                                                                                              ▼
                                                                                    ┌────────────────┐
                                                                                    │ Push Sink      │
                                                                                    │ retry · breaker│
                                                                                    └───────┬────────┘
                                                                                            ▼
                                                                                    ┌────────────────┐
                                                                                    │ WAL Ring Buffer│
                                                                                    │ replay · drop  │
                                                                                    └────────────────┘

```

**数据流说明**

* 调度器按“节奏组”触发采集器；采集器只返回本域结果，不直接写网络。

* 规范化器统一单位、空值、派生值和排序；生成不可变快照。

* 快照同时供本地 Prometheus 端点和 JSON 推送使用；两者共享同一份数据，避免口径漂移。

* 推送失败进入 WAL；WAL 按序重放，服务端按幂等键去重。

* 健康引擎读取最近快照与采集器状态，生成健康对象和状态变更事件。

### 2.3 模块职责

| 模块 | 核心职责 | 输入 / 输出 | 明确不做 | 
| ----- | ----- | ----- | ----- | 
| `config` | 加载、合并、校验配置；生成不可变配置快照；计算变更集 | 文件/环境变量/CLI → `Config` | 不执行业务采集；不在此处做网络 I/O | 
| `lifecycle` | 启动、就绪、重载、优雅退出；单实例锁；信号/服务控制 | 信号、服务管理器 → 生命周期事件 | 不解析指标 | 
| `scheduler` | 按节奏组触发采集；防重入；超时与并发上限；抖动 | 配置、时钟 → 采集任务 | 不做数据转换 | 
| `collector-core` | 采集器注册、能力探测、超时、panic 隔离、结果聚合 | 平台后端 → `SectionResult` | 不决定输出格式 | 
| `platform-linux` | 读取 `/proc`、`/sys`、netlink、cgroup | 内核接口 → 原始结构 | 不做跨平台兜底 | 
| `platform-windows` | PDH、IP Helper、`GetSystemTimes`、`GlobalMemoryStatusEx` 等 | Win32/PDH → 原始结构 | 不依赖 WMI 做高频采集 | 
| `platform-macos` | Mach、sysctl、getifaddrs、statfs、IOKit | Mach/BSD 接口 → 原始结构 | 不采集需要特殊授权的数据 | 
| `model` | 指标/快照/错误/健康/事件的数据结构；序列化约定 | 各模块 → 统一模型 | 不包含平台分支 | 
| `normalizer` | 单位换算、比率计算、计数器差分、排序、空值处理 | 原始结构 → 规范快照 | 不伪造缺失值 | 
| `health` | 本地健康检查、状态机、滞后与恢复 | 快照、采集器状态 → 健康对象/事件 | 不发送通知 | 
| `encoder-prom` | 将快照映射为 Prometheus 文本格式 | 快照 → HTTP 响应 | 不做告警判定 | 
| `sink-push` | JSON 编码、压缩、认证、重试、熔断 | 快照批次 → HTTPS 请求 | 不做长期存储 | 
| `wal` | 本地分段缓冲、校验、重放、容量淘汰 | 批次 → 磁盘/重放流 | 不保证无限期保留 | 
| `security` | TLS、令牌/证书加载、密钥零化、请求签名（可选） | 配置 → 安全上下文 | 不存储明文密钥 | 
| `self-telemetry` | 采集耗时、错误、队列、推送、缓冲区等自监控 | 各模块事件 → 指标 | 不替代业务指标 | 
| `logging` | 结构化日志、级别过滤、限流、脱敏 | 日志事件 → stdout/文件 | 不记录密钥和完整命令行 | 
| `cli` | 参数解析、`--check-config`、`--version`、`--diagnose`、`--once` 等 | 命令行 → 动作 | 不承载常驻业务逻辑 | 

### 2.4 部署拓扑

**模式 A：推送模式（默认，推荐）**

```
被监控主机                         监控后端
┌─────────────┐   HTTPS/JSON    ┌──────────────┐   ┌──────────┐   ┌────────────┐
│ Sprobe      │ ──────────────▶ │ Ingest API   │──▶│ TSDB     │──▶│ Alerting   │
│ + WAL       │                 │ auth · dedup │   │          │   │ + Notify   │
└─────────────┘                 └──────────────┘   └──────────┘   └────────────┘
       │ localhost:9101
       ▼
  Prometheus/curl（可选）

```

**模式 B：拉取模式（可选）**

由 Prometheus 或兼容采集器主动抓取 `http://127.0.0.1:9101/metrics`。适合已有 Prometheus 体系、且 agent 与 Prometheus 网络可达的场景。拉取模式不经过 WAL，因此不提供断网补发能力。

**模式 C：混合模式**

推送保证跨公网/NAT 的可靠投递；本地端点用于排障和秒级抓取。两种模式共享同一快照源，默认开启推送、关闭公网监听。

### 2.5 关键设计决策

| 编号 | 决策 | 备选方案 | 取舍理由 | 代价 / 约束 | 
| ----- | ----- | ----- | ----- | ----- | 
| D1 | 单二进制，采集器编译期注册 | 动态插件、独立 sidecar | 分发简单、启动快、攻击面小；避免插件 ABI 兼容问题 | 新平台/新指标需发版；二进制体积略大 | 
| D2 | 推为主、拉为辅 | 纯拉取、纯推送 | 适配 NAT/公网/大规模；本地拉取便于排障 | 需要服务端接收与去重；WAL 增加磁盘占用 | 
| D3 | 快照模型 | 逐指标点流、OpenMetrics 流 | 原子、可重放、易做 schema 校验；一次采集一次编码 | 实时性受采集周期约束；大主机快照较大 | 
| D4 | 混合采集：跨平台库 + 原生接口 | 完全依赖 `sysinfo`、完全手写 | 兼顾开发效率与指标精度；关键口径可审计 | 平台代码量增加；需维护能力矩阵 | 
| D5 | 阻塞采集放入有界阻塞线程池 | 全部 async、每采集器一线程 | 避免 `/proc`、PDH、statfs 阻塞 Tokio 反应器；线程数可控 | 需要超时/取消策略；线程池耗尽时降级 | 
| D6 | 单一调度器 + 多节奏组 | 每个采集器独立循环 | 防止重入和惊群；便于统一计算周期与抖动 | 慢采集器可能推迟同组任务；需 deadline 管理 | 
| D7 | 规范单位：bytes、seconds、ratio | 百分比、KB/MB、混合单位 | 避免单位换算错误；Prometheus 兼容 | 展示层需转换；文档必须明确 | 
| D8 | 缺失输出 `null`，不用 0 | 统一填 0、省略字段 | 0 是有效值；缺失与 0 混淆会导致误告警 | 消费端必须处理 null；Prometheus 跳过空值 | 
| D9 | 进程只报聚合 + Top-N | 全量进程逐指标 | 控制时序基数与内存；满足绝大多数排障场景 | 无法做全量进程历史分析；需按需扩展 | 
| D10 | Windows 负载为估算值 | 不提供、伪造 Unix 负载 | 保持“系统负载”能力覆盖，同时诚实标注质量 | 不可与 Unix 负载直接比较；告警需用独立规则 | 
| D11 | 本地端点默认只绑回环 | 默认 `0.0.0.0` | 降低暴露面；需要远程抓取时显式开启并加认证 | 跨机抓取需额外配置 | 
| D12 | 至少一次投递 + 幂等去重 | 最多一次、恰好一次 | 网络分区下不丢数据；实现简单可靠 | 服务端需去重；极端情况下可能重复 | 

### 2.6 与外部系统的边界

| 外部系统 | 交互 | 边界约定 | 
| ----- | ----- | ----- | 
| 服务端 Ingest API | HTTPS POST JSON 批次 | 负责认证、校验、去重、持久化、返回 2xx/4xx/5xx；不要求 agent 理解存储细节 | 
| Prometheus | HTTP GET `/metrics` | 文本格式 0.0.4；不返回时间戳；由 Prometheus 记录抓取时间 | 
| 服务管理器 | systemd / Windows SCM / 容器运行时 | Agent 实现就绪、健康、优雅退出；不依赖特定服务管理器才能采集 | 
| 配置管理 | Ansible/Puppet/Chef/镜像 | 配置文件为声明式 TOML；支持 `check-config` 干跑 | 
| 日志系统 | stdout / journald / 文件 | JSON Lines；由外部系统负责轮转与长期存储（文件模式除外） | 
| 告警系统 | 消费健康事件/时序数据 | Agent 不直接发通知；事件 schema 固定，通道由服务端实现 | 

## 3. 功能模块设计与关键流程

### 3.1 配置与生命周期

**职责**

* 按“默认值 < 配置文件 < 环境变量 < 命令行参数”的优先级合并配置。

* 启动时完成 schema 校验、语义校验和跨字段校验；任何一项失败都拒绝启动。

* 生成不可变配置快照，运行期通过 `Arc` 共享；重载时整体替换，避免读到半更新状态。

* 计算配置差异，只对受影响的模块执行热更新；不能热更新的字段明确拒绝并提示重启。

**校验分层**

| 层级 | 示例 | 失败处理 | 
| ----- | ----- | ----- | 
| 语法校验 | TOML 解析失败、未知字段 | 启动失败；重载失败并保留旧配置 | 
| 类型/范围 | 间隔 < 1s、缓冲区 < 1MiB、并发 > 64 | 启动失败；重载拒绝 | 
| 语义校验 | 推送开启但无 URL/认证；TLS 开启但证书不可读 | 启动失败；重载拒绝 | 
| 环境校验 | 数据目录不可写、端口被占用、权限不足 | 启动失败（端口/目录）；权限不足可降级并告警 | 
| 能力校验 | Windows 上配置了仅 Linux 的采集器 | 启动成功但该采集器标记 `unsupported`，不视为配置错误 | 

**生命周期状态**

```
starting → ready → reloading → ready → draining → stopped
              └──────────────→ degraded（采集/上报异常，但进程存活）

```

* `starting`：配置、身份、能力、WAL 初始化完成前。

* `ready`：配置、身份、WAL 恢复、本地端点监听与首轮（第 0 轮）基线采集完成（≤2s 达标）；此时差分指标仍处于 `warmup=true` 状态，待下一轮常规 fast 周期完成后置 `warmup=false`。

* `degraded`：关键采集器连续失败、推送持续失败、缓冲区积压超阈值；进程仍存活并继续尝试恢复。

* `draining`：收到退出信号，停止新任务，等待在途任务与最后一次落盘。

### 3.2 调度器

**节奏组**

| 节奏组 | 默认周期 | 包含采集器 | 说明 | 
| ----- | ----- | ----- | ----- | 
| `fast` | 10s | CPU 使用率、内存与交换、负载（含 `/proc/stat` 运行/阻塞进程数）、网络速率、开机时长、健康评估 | 告警最关心的指标；允许 1s–30s 覆盖 | 
| `medium` | 30s | 文件系统容量、磁盘 I/O、CPU 频率、内存缓存/脏页、进程聚合与 Top-N | 开销较高或变化较慢；允许 5s–300s 覆盖且为 `fast` 整数倍 | 
| `slow` | 300s | 主机清单、CPU 逻辑/物理核数与拓扑、文件系统清单、网卡清单与速率 | 变化极慢；允许 60s–3600s 覆盖 | 
| `push` | 30s | 批次发送（默认与 3 个 fast 快照组批对齐，积压时最高 6 快照/批） | 独立于采集；可配置 5s–60s | 
| `heartbeat` | 30s | 独立心跳（`POST /api/v1/heartbeat`） | 无快照或推送熔断时仍可发送；用于判断 agent 存活 | 

**调度规则**

1. 所有采集、推送与心跳周期必须是 1 秒的整数倍（配置非整数倍时向上取整并记录警告）；单次超时参数允许浮点秒（精度至毫秒）。

2. 采集 tick 对齐到 Unix 纪元（`timestamp % interval == 0`），同一台机器上各周期可预测；不跨机器强制对齐。

3. 防重入：同一采集器上一轮未结束时，本轮直接跳过，`skipped_total{collector}` 加一；连续跳过 3 次记录 `E_OVERLOAD` 警告。

4. 并发上限：同时运行的采集任务不超过 `max_concurrent_collectors`（默认 4）；慢采集器不得占满全部槽位，fast 组优先。

5. 总预算：单个采集器 2s，单次快照 5s，推送请求 10s；超时后取消等待并标记 `timeout`，但不强杀阻塞线程（见 10.5）。

6. 推送抖动：在周期上叠加 0–20% 的随机抖动，避免大规模节点同时上行；采集不打乱对齐。

7. 时钟：周期使用单调时钟计算，墙上时钟只用于时间戳；检测到墙上时钟跳变 > 5s 时，重置差分基线并记录 `clock_adjusted=true`。

### 3.3 采集器框架

**统一契约（概念级）**

每个采集器必须提供：

* `name`：稳定标识，如 `cpu`、`memory`、`disk`、`network`、`load`、`process`、`uptime`、`health`。

* `cadence`：所属节奏组及可覆盖范围。

* `capabilities()`：平台/权限/配置下的支持状态与原因。

* `collect(context) → SectionResult`：返回本域数据、采样时间、耗时、错误列表。

* `reset_baseline()`：配置重载、时钟跳变、计数器重置时清理差分状态。

**SectionResult 状态**

| 状态 | 含义 | 快照中的表现 | 
| ----- | ----- | ----- | 
| `ok` | 全部字段采集成功 | 正常字段；`errors=[]` | 
| `partial` | 部分字段失败/权限不足 | 成功字段 + `errors[]`；相关字段为 `null` | 
| `failed` | 本域整体失败 | 本域 `status=failed`；保留上一次值并标记 `stale=true`（可配置） | 
| `unsupported` | 平台不支持或未启用 | 本域 `status=unsupported`；不产生序列 | 
| `timeout` | 超过预算 | 同 `failed`，错误码 `E_TIMEOUT` | 

**硬性要求**

* 采集器不得调用网络、不得写文件、不得启动外部命令（SMART 等可选能力除外，且必须单独开关）。

* 采集器不得 `unwrap` 可预期的 I/O/解析错误；不得把错误吞掉后返回 0。

* 采集器不得跨平台 `cfg` 分支散落在业务代码中；平台差异收敛在 `platform-*` 模块。

* 采集器返回前必须检查数值：禁止 `NaN`、`Inf`、负的字节数/计数；超范围值截断并记录 `E_VALUE_RANGE`。

* 采集器必须可重入安全；同一采集器不会被并发调用。

**超时与 panic 隔离**

* 采集任务放入有界阻塞线程池执行；主调度器用异步超时等待结果。

* 阻塞任务被超时后无法强制终止，因此线程池必须有上限，且超时任务计入“僵尸槽位”监控；网络文件系统（NFS/CIFS）`statfs` 必须隔离至独立 2 线程探测池，并按挂载点执行在途防重入（已有未返回探测时禁止再次派发），防止僵死挂载耗尽主采集线程池。

* 采集器 panic 由 `catch_unwind` 捕获，转为 `E_PANIC`；同一采集器连续 3 次 panic 后自动禁用，直到配置重载或进程重启。

* panic 的 backtrace 只写本地 debug 日志，不进入上报 payload。

### 3.4 平台采集器职责

| 采集器 | Linux 主来源 | Windows 主来源 | macOS 主来源 | 关键注意事项 | 
| ----- | ----- | ----- | ----- | ----- | 
| CPU | `/proc/stat`、`/sys/devices/system/cpu` | `GetSystemTimes`、PDH `Processor Information` | `host_processor_info`、`host_statistics` | 聚合必须用 tick 总量，不能平均各核百分比 | 
| 内存 | `/proc/meminfo`、cgroup `memory.current/max` | `GlobalMemoryStatusEx`、`GetPerformanceInfo` | `host_statistics64`、`sysctl hw.memsize` | “可用”口径跨平台不同，必须标注来源 | 
| 文件系统 | `statfs/statvfs`、`/proc/self/mountinfo` | `GetLogicalDrives`、`GetDiskFreeSpaceEx` | `getfsstat`、`statfs` | 伪文件系统、重复挂载、保留块、inode 为 0 | 
| 磁盘 I/O | `/proc/diskstats`、`/sys/block` | PDH `PhysicalDisk`、IOCTL | IOKit `IOBlockStorageDriver` | 分区与整盘重复计数；dm/md/bond 从属关系 | 
| 网络 | `/proc/net/dev`、netlink、`/sys/class/net` | IP Helper `GetIfTable2` | `getifaddrs`、`NET_RT_IFLIST2` | 虚拟网卡/从属网卡重复；32 位计数器回绕 | 
| 负载 | `getloadavg`、`/proc/loadavg`、`/proc/stat` | PDH `System\Processor Queue Length` | `getloadavg` | Windows 无原生 load average，必须标注估算 | 
| 进程 | `/proc/<pid>/stat`（含第 20 字段 `num_threads`，无需额外打开 `status`） | `EnumProcesses`、`GetProcessTimes`、`GetProcessMemoryInfo` | `proc_listpids`、`proc_pidinfo` | 权限、PID 复用、zombie、线程与进程区分 | 
| 开机时长 | `/proc/uptime`、`/proc/stat btime` | `GetTickCount64`、系统时间 | `sysctl kern.boottime` | 容器内 `/proc/uptime` 是宿主机口径 | 
| 健康 | 由上述指标 + 自监控合成 | 同左 | 同左 | 健康判断不能依赖单一指标 | 

### 3.5 规范化器

规范化器是“原始数据 → 契约数据”的唯一通道，职责如下：

1. **单位换算**：jiffies → 秒、扇区 → 字节、KB → 字节、百分比 → 比率、毫秒 → 秒。

2. **空值处理**：不可用 → `null`；不支持 → 字段不出现且能力清单标注；0 是有效值，不得转为 `null`。

3. **派生计算**：基于同一指标键的上一采样计算速率/利用率；首次采样不计算，返回 `null`。

4. **计数器重置**：当前值 < 上一次值 → 判定重置；本周期差值为 `null`，记录 `counter_reset=true`，用当前值重建基线。

5. **裁剪与限流**：挂载点、网卡、进程列表按配置过滤；超过上限时按稳定排序截断并记录 `truncated=true`。

6. **排序**：文件系统按 `mountpoint` 字典序、磁盘按 `device`、网卡按 `interface`、进程按 `pid`；保证同一主机快照可复现。

7. **数值边界**：比率裁剪到 `[0,1]`；利用率允许短暂 >1 时裁剪并计数；字节/计数用饱和运算，避免溢出。

8. **质量标注**：每个派生值标注 `derived`；估算值标注 `estimated`；无法确认来源的字段不得标 `measured`。

### 3.6 健康引擎

健康引擎在每个快照生成后运行，输出主机级健康状态和检查项。它只做“本地可判定的健康”，不替代服务端告警规则。

**健康检查项**

| 检查 ID | 判定 | 默认严重度 | 恢复条件 | 
| ----- | ----- | ----- | ----- | 
| `collector_freshness` | 任一启用采集器超过 3 × 周期未成功 | warning | 连续 3 次成功 | 
| `cpu_saturation` | `cpu_usage_ratio` 5 分钟均值 > 0.90 且 `cpu_iowait_ratio` < 0.20 | warning | 连续 3 次低于阈值 | 
| `memory_pressure` | `memory_usage_ratio` > 0.90 持续 5 分钟为 warning；> 0.95 持续 5 分钟升级为 critical | warning/critical | 连续 3 次低于阈值 | 
| `disk_space` | 使用率 > 0.90；> 0.95 升级为 critical | warning/critical | 连续 3 次低于阈值 | 
| `disk_inode` | inode 使用率 > 0.90 持续 5 分钟 | warning | 连续 3 次低于阈值 | 
| `filesystem_readonly` | 非只读设计的文件系统变为只读 | critical | 恢复可写并连续 3 次确认 | 
| `disk_io_latency` | 读或写平均延迟 5 分钟均值 > 100ms | warning | 连续 3 次低于阈值 | 
| `network_errors` | 错误/丢弃速率持续 > 1/s 且 5 分钟内增长 | warning | 连续 3 次低于阈值 | 
| `load_high` | `load_average_1m_per_core` > 2.0 持续 10 分钟 | warning | 连续 3 次低于阈值 | 
| `process_missing` | 被监视进程 `present=false` 持续 2 个周期 | critical | 进程恢复并稳定 1 分钟 | 
| `agent_backlog` | WAL 待发送快照 > 1000 或最旧 > 30 分钟 | warning | 积压回落并连续 3 次正常 | 
| `push_auth_failed` | 最近一次推送返回 401/403 | critical | 配置修正并成功一次 | 

阈值与状态机细节以 4.11 为准；本表只列模块级职责，避免两处阈值各自演化。

* **例外**：`push_auth_failed` 首次 401/403 即锁存 critical，不等待连续 2 次；成功推送一次后恢复。鉴权失败不是瞬时抖动，继续重试只会放大风险。

**状态机**

```
unknown ──预热结束且检查通过──▶ ok
ok ──任一 critical──▶ critical
ok ──任一 warning──▶ degraded
degraded ──出现 critical──▶ critical
critical ──critical 全部恢复但仍有 warning──▶ degraded
degraded/critical ──全部检查连续 3 次正常──▶ ok
任意 ──超过 3 个周期无数据──▶ unknown

```

* 状态变更必须有滞后（hysteresis），避免抖动；告警侧还有独立的 `for` 窗口，两者不重复计算。

* `health.since` 记录当前状态开始时间；`health.checks[]` 保留每个检查的当前值、阈值、消息。

* 健康状态为 `unknown` 时，服务端应触发“数据缺失”而不是“主机正常”。

### 3.7 输出与缓冲

* **编码器**：Prometheus 编码器与 JSON 编码器都只读快照，不修改快照；新增字段必须同时更新两个编码器和 schema。

* **推送批次**：默认每 6 个快照或 1MiB 或 30s 触发一次，先满足者为准；单批次最多 100 个快照。

* **压缩**：gzip 级别 6；小于 4KiB 的批次可不压缩；服务端必须支持 gzip。

* **WAL**：启用推送时，快照在发送前先追加到 WAL 并标记 `pending`；确认后删除；按 `(agent_id, instance_id, sequence)` 幂等；分段文件带 CRC32C。

* **淘汰**：容量或时间超限时丢弃最旧数据，`dropped_total` 与 `dropped_oldest_sequence` 上报；不得静默丢弃。

* **重放**：启动时按 WAL Ack Watermark 从最旧未确认批次开始重放；服务端返回 `200 OK` 且 `duplicates > 0` 视为幂等成功；`409 Conflict` 严格表示同序号内容冲突，不视为成功。

### 3.8 自监控

Agent 必须暴露自身运行指标，至少包括：

（与 8.2 Prometheus 自监控清单保持唯一命名一致，统一带 `sprobe_` 前缀与 `reason` 标签）：

* 采集：`sprobe_collector_duration_seconds{collector}`、`sprobe_collector_last_success_timestamp_seconds{collector}`、`sprobe_collector_errors_total{collector,reason}`、`sprobe_collector_skipped_total{collector}`、`sprobe_snapshot_age_seconds`。

* 缓冲：`sprobe_wal_snapshots`、`sprobe_wal_bytes`、`sprobe_wal_evicted_total`。

* 推送：`sprobe_push_attempts_total`、`sprobe_push_failures_total{reason}`、`sprobe_push_duration_seconds`、`sprobe_push_last_success_timestamp_seconds`。

* 进程：`sprobe_agent_up`、`sprobe_agent_info`、`sprobe_agent_warmup`、`sprobe_agent_uptime_seconds`、`sprobe_agent_memory_rss_bytes`、`sprobe_agent_cpu_usage_ratio`、`sprobe_agent_open_fds`。

* 配置：`sprobe_config_reload_total{result}`。

自监控指标使用与业务指标相同的命名/空值规范，但单独放在 `agent` 域，避免与主机指标混淆。

### 3.9 关键流程

#### 流程 1：启动

 1. 解析 CLI，确定配置文件路径、日志级别、`--check-config` 等一次性动作。

 2. 初始化早期日志（stderr），加载配置并完成三层校验；失败则输出错误码并以非 0 退出。

 3. 获取单实例锁（Linux 文件锁 / Windows 命名互斥量）；失败则退出，避免双采。

 4. 加载或生成身份：`agent_id`、`instance_id`；身份文件权限 0600。

 5. 探测平台能力与权限：逐采集器生成 `supported/available/reason`。

 6. 初始化 WAL 与数据目录；检查磁盘空间；恢复未确认批次索引。

 7. 执行一次 `slow` 清单采集（带 5s 超时），失败不阻塞启动。

 8. 启动本地 HTTP 端点（若启用），此时 `ready=false`、`warmup=true`。

 9. 启动调度器，在 t0 立即执行第 0 轮 fast 基线采集（仅建立 Counter 基线，目标 ≤2s 完成）。

10. 第 0 轮基线采集与本地端点监听就绪后立即标记 `ready=true` 并向服务管理器发送 `READY=1`；输出启动摘要（版本、平台、能力、配置哈希、监听地址）。待下一个 fast 周期（t0+10s）产出首个有效差分后，将 `warmup` 置为 `false`。

**启动失败原则**：配置/身份/端口/目录错误必须失败退出；单个采集器权限不足允许降级启动，但必须在启动摘要和健康状态中可见。

#### 流程 2：一次采集周期

1. 调度器在对齐 tick 触发，创建 `SnapshotContext`（序号、计划时间、单调时钟起点、配置版本）。

2. 按优先级提交采集任务：fast 组优先，其余按节奏组；受并发信号量限制。

3. 每个采集器在阻塞线程池执行，主协程等待并施加超时；超时/panic 转为错误结果。

4. 规范化器按域合并结果，计算派生值，应用过滤/排序/截断，生成不可变 `Snapshot`。

5. 健康引擎评估检查项，更新状态机；状态变化生成事件。

6. 快照写入“最新快照”槽位，唤醒 Prometheus 端点与推送编码器。

7. 推送编码器将快照放入有界批次队列；队列满时丢弃最旧快照并计数。

8. 记录本轮总耗时、各采集器耗时、错误；超过总预算则记录 `E_SNAPSHOT_BUDGET`。

**部分失败示例**：文件系统采集器因 NFS 挂起超时，快照仍包含 CPU/内存/网络；`filesystems.status=partial`（或全超时为 `timeout`），`filesystems.errors[]` 记录挂载点与错误码，`health` 标记 `degraded`，但不会让整个快照失败。

#### 流程 3：推送

1. 批次队列达到条数/字节/时间阈值，或收到 flush 信号，取出批次。

2. 生成 `batch_id` 并写入请求头 `X-Sprobe-Batch-Id`（作为请求级幂等键，重试复用同一 `batch_id`；快照级幂等由 `(agent_id, instance_id, sequence)` 保证）。

3. JSON 序列化 → gzip → 添加认证头 → HTTPS POST。

4. 2xx：标记成功，更新 `last_push_success_time` 与 WAL Ack Watermark；当整段最大 `sequence <= acked_sequence` 时删除该段；若响应体 `duplicates > 0` 计入重复统计并同样视为幂等成功。

5. 409 Conflict：表示同幂等键但载荷内容冲突，不重试，记录数据冲突错误与告警事件。

6. 408/425/429/5xx/网络错误：进入退避重试；重试期间批次保留在内存/WAL。

7. 401/403：停止重试，标记 `push_auth_failed`，触发 critical 健康检查；等待配置重载。

8. 其他 4xx：视为永久失败，落盘到 `rejected/` 目录（可选）并计数，避免无限重试。

9. 熔断：连续 5 次失败进入 open 状态 30s；半开时只放行 1 个探测请求。

#### 流程 4：配置热加载

1. 触发源：Unix `SIGHUP`、Windows 服务控制/文件变更监听（1.0 不提供独立 `sprobe reload` 本地管理子命令）。

2. 读取新文件 → 解析 → 校验；失败则记录 `E_CONFIG_RELOAD`，保留旧配置，不影响采集。

3. 计算差异并分类：

   * 可热更新：日志级别、采集间隔、过滤规则、标签、推送批次参数、告警阈值、令牌文件及 TLS 证书/私钥路径与内容（重建 `rustls` 客户端配置，失败保留旧配置）。

   * 需重启：监听地址、数据目录、状态文件名、单实例锁相关。

4. 应用：先生效安全/日志，再更新调度器，最后切换输出配置；每一步失败则回滚到旧配置。

5. 记录审计日志：配置哈希、变更字段、操作者（若可获取）、结果。

#### 流程 5：优雅退出

1. 收到 SIGTERM/SIGINT 或服务停止控制。

2. 状态切到 `draining`，停止调度器接受新任务。

3. 优雅退出总预算严格控制在 **15s** 以内（小于 systemd `TimeoutStopSec=25s`）：

   * 等待在途采集最多 3s；超时则放弃并记录；

   * 将当前未发送快照写入 WAL 并尝试最后一次推送与元数据刷盘（最多 10s）；

   * 关闭 HTTP 端点、停止后台任务并释放单实例锁（最多 2s）。

4. 记录退出原因，正常退出码 0。

5. 第二次信号立即退出，退出码 1；可能丢失最后一批数据，但 WAL 已落盘部分可恢复。

#### 流程 6：故障恢复

* **进程崩溃**：重启后读取 WAL 索引，校验分段 CRC；损坏段移入 `quarantine/`，从下一个完好段继续；记录 `E_WAL_CORRUPT`。

* **网络恢复**：熔断器半开探测成功后关闭，按序号重放；服务端去重。

* **权限变化**：每 5 分钟重新探测一次能力；权限恢复后自动重新启用采集器，记录状态变更。

* **时钟跳变**：重置所有差分基线，本周期派生值为 `null`；快照标记 `clock_adjusted=true`；不调整历史数据。

* **计数器重置/设备热插拔**：对应实体标记 `reset=true`，重新建立基线；不删除实体，保留 `last_seen`。

### 3.10 并发模型与资源预算

| 资源 | 预算 | 超限策略 | 
| ----- | ----- | ----- | 
| 异步运行时工作线程 | `min(4, CPU 核数)` | 固定，不随核数无限增长 | 
| 阻塞采集线程 | 主采集池默认 8（最大 16）；NFS/网络文件系统独立探测池 2 线程 | 队列满时跳过本轮并计数；单挂载点在途防重入 | 
| 同时采集任务 | 默认 4 | fast 组优先；其余排队/跳过 | 
| 快照队列 | 64 个 | 丢弃最旧，`snapshot_dropped_total` 加一 | 
| 推送批次队列 | 8 个 | 丢弃最旧批次并落盘（若开启 WAL） | 
| 单快照大小 | 256KiB 未压缩 | 截断 Top-N/挂载点并标记 `truncated` | 
| 单批次大小 | 1MiB 未压缩 / 100 快照 | 拆分批次 | 
| WAL | 96MiB / 1 小时 | 淘汰最旧段，记录丢弃序号 | 
| 日志 | 100MiB / 7 天（文件模式） | 轮转；rate limit | 
| 文件句柄 | 1024 | 启动时检查 ulimit；不足则拒绝启动或降级 | 

**并发禁忌**

* 不得在持有锁时执行 I/O 或等待网络。

* 不得为每个采集器创建无限线程；不得使用无界 channel。

* 不得让推送阻塞采集；两者通过有界队列解耦。

* 不得在异步任务中直接执行 `statfs`、PDH、`/proc` 大文件读取等阻塞调用。

### 3.11 开发环境配置

本节定义开发者从零搭建后端工程的最低环境。目标是“一条命令构建、一条命令跑测试、一个本地桩完成上报联调”，不依赖公网服务。

**3.11.1 工具链与系统依赖**

| 项 | 要求 | 说明 |
| ----- | ----- | ----- |
| Rust | 1.95（由 `rust-toolchain.toml` 锁定） | 与 `sysinfo 0.40.x` 的 MSRV 对齐；升级需单独评审 |
| 组件 | `rustfmt`、`clippy`、`llvm-tools` | CI 与本地使用同一套 |
| Linux 构建依赖 | `build-essential`、`musl-tools`、`pkg-config` | musl 目标需要 `musl-gcc`；不依赖 OpenSSL |
| Windows 构建依赖 | Visual Studio Build Tools（C++ 工作负载） | 使用 `x86_64-pc-windows-msvc` |
| macOS 构建依赖 | Xcode Command Line Tools | Intel 与 Apple Silicon 各一台/CI runner |
| 交叉编译 | `cross` 或 `cargo-zigbuild` | aarch64 产物必须在真机或 QEMU 冒烟 |
| 质量工具 | `cargo-nextest`、`cargo-audit`、`cargo-deny` | 本地可缺省，CI 必须执行 |

**3.11.2 仓库目录约定**

```text
sprobe/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── src/
│   ├── main.rs                 # 仅参数解析、装配、退出码
│   ├── config/                 # 配置结构、校验、热加载
│   ├── scheduler/              # 分组、防重入、超时、背压
│   ├── collectors/             # 平台采集器，按 target_os 条件编译
│   │   ├── linux/
│   │   ├── windows/
│   │   └── macos/
│   ├── normalize/              # 单位、空值、质量、能力
│   ├── snapshot/               # 快照模型与 schema 序列化
│   ├── health/                 # 12 项检查与状态机
│   ├── output/                 # JSON、Prometheus、推送客户端
│   ├── wal/                    # 分段、CRC、恢复、水位
│   ├── http/                   # 本地只读端点
│   └── telemetry/              # 日志、自监控、事件
├── tests/
│   ├── fixtures/               # /proc、PDH、macOS 采样与黄金 JSON
│   ├── contract/               # schema/批次/错误码契约测试
│   └── integration/            # 采集→快照→输出→WAL 集成
├── scripts/                    # 对比脚本、发布校验、mock 启停
└── docs/                       # 本文、runbook、变更记录
```

**3.11.3 常用命令**

```bash
# 格式与静态检查
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# 单元/集成/契约测试
cargo nextest run --all-features
cargo test --doc

# 依赖与许可证
cargo audit
cargo deny check

# 本地运行
cargo run -- --check-config -c ./sprobe.toml
cargo run -- --once -c ./sprobe.toml
cargo run -- --diagnose -c ./sprobe.toml

# 交叉构建
cross build --release --target aarch64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

**3.11.4 本地联调与测试数据**

* 测试桩必须覆盖 `200/400/401/403/409/413/429/500/超时/乱序/重复`，并能校验 `batch_id`、`(agent_id, instance_id, sequence)` 幂等键与 gzip 请求体。

* `tests/fixtures/` 至少包含：Linux `/proc/stat`、`/proc/meminfo`、`/proc/diskstats`、`/proc/net/dev` 的两次采样；Windows PDH 英文计数器样本；macOS `sysctl`/`ifaddrs` 样本；每个平台的黄金快照 JSON。

* 所有采集器必须支持“固定输入 → 固定快照”的离线测试；时间、随机抖动、主机名通过注入参数固定，保证 `--once` 输出可复现。

* `--once --sample-window 10s` 用于验收差分指标；首次采样只建立基线，第二个周期才输出有效速率/使用率。

**3.11.5 提交前门禁**

| 检查 | 命令/方式 | 通过标准 |
| ----- | ----- | ----- |
| 格式 | `cargo fmt --check` | 无差异 |
| Lint | `cargo clippy -D warnings` | 无 warning |
| 单测 | `cargo nextest run` | 全绿，新增代码覆盖率不低于团队阈值 |
| 契约 | `tests/contract` | schema/错误码/字段路径全通过 |
| 依赖 | `cargo audit`、`cargo deny check` | 无 high/critical，许可证白名单通过 |
| 跨平台 | CI matrix | Linux musl x86_64/aarch64、Windows msvc、macOS 冒烟 |
| 文档 | 本文与附录 A | 新增/变更指标已登记，示例与字段表一致 |

## 4. 指标采集设计

本章是开发与验收的核心依据。所有指标必须先在本章登记口径，再进入实现；实现与本章不一致时，以本章为准并走文档变更流程。

### 4.1 指标模型与通用约定

**一条指标必须包含的元信息**

| 字段 | 说明 | 示例 | 
| ----- | ----- | ----- | 
| `name` | 稳定指标键，符合 4.2 命名规范 | `cpu_usage_ratio` | 
| `type` | `gauge`、`counter`、`derived`、`info` | `gauge` | 
| `unit` | `bytes`、`seconds`、`hertz`、`ratio`、`count`、`celsius`、`bytes_per_second` | `ratio` | 
| `scope` | `host`、`container`、`process`、`agent` | `host` | 
| `quality` | `measured`、`estimated`、`derived` | `measured` | 
| `labels` | 有界维度集合 | `device`、`mountpoint`、`interface`、`cpu`、`mode` | 
| `description` | 一行口径说明，进入 Prometheus `# HELP` | “非空闲且非 iowait 的 CPU 时间占比” | 

**类型语义**

* `gauge`：瞬时值，可增可减；同一采集周期内取一次样本。

* `counter`：自开机或对象创建以来单调递增的累计值；重启/设备重建后归零；名称必须以 `_total` 结尾。

* `derived`：由相邻两次采样差分或比值计算；不单独存储原始状态；首次采样为 `null`。

* `info`：元数据，值恒为 1，实际信息在标签中；用于版本、平台、型号等。

**单位与精度**

| 单位 | 约定 | 精度/范围 | 
| ----- | ----- | ----- | 
| `bytes` | 字节，十进制整数；展示层再转 KiB/MiB/GiB | `u64`，JSON 用 number；超过 2^53 时消费端不得用于精确计费 | 
| `seconds` | 秒，浮点；累计时长与时间差都用秒 | 保留 3 位小数；时长可为 0 | 
| `hertz` | 赫兹，整数 | CPU 频率；未知为 `null` | 
| `ratio` | `[0,1]` 的无量纲比例 | 保留 4 位小数；展示层乘 100 | 
| `count` | 个/项，整数 | 进程数、队列长度、负载等 | 
| `bytes_per_second` | 字节/秒 | 保留 2 位小数；由差分或 PDH 速率得到 | 
| `celsius` | 摄氏度 | 可选；保留 1 位小数 | 

**空值与不支持**

* `null`：平台支持、配置启用，但本次采集不可用（权限不足、读取失败、首次采样、计数器重置）。

* 字段缺失：平台不支持或配置关闭；必须同时出现在 `capabilities` 中，值为 `false` 并带 `reason`。

* `0`：真实的零值。例如磁盘写入 0 字节、进程数为 0、空闲 CPU 为 0。

* 禁止用 `-1` 表示未知；禁止用空字符串表示数值缺失；禁止 `NaN`/`Inf` 进入 JSON 或 Prometheus 文本。

**时间**

* 快照时间：`collected_at`，UTC、RFC 3339、毫秒精度，例如 `2026-11-21T01:20:51.123Z`。

* 分区时间：`sampled_at`，用于慢采集器缓存；`age_seconds` 表示距当前快照的时间。

* 持续时间：使用单调时钟计算，不使用墙上时钟；墙上时钟跳变不得导致负的采集耗时。

* 所有时间戳必须是 UTC；本地时区只用于展示，不进入数据。

**标签与基数**

* 标签值必须稳定、有界、可枚举；禁止把 PID、命令行、完整文件路径、用户输入直接作为标签。

* 单指标标签组合上限：文件系统 ≤ 100、磁盘 ≤ 64、网卡 ≤ 64、CPU 核 ≤ 512；超限按稳定排序截断并标记 `truncated=true`。

* 标签值长度 ≤ 256 字节；非法 UTF-8 用替换字符处理，原始字节可放入诊断字段（默认关闭）。

* Prometheus 额外标签由服务端注入（`instance`、`job`、`env`），agent 不重复注入。

### 4.2 命名规范

**规范名（canonical name）**

采用 `域_对象_测量_单位` 的小写下划线形式，全部 ASCII：

```
cpu_usage_ratio
memory_available_bytes
filesystem_used_bytes
disk_read_bytes_total
network_receive_errors_total
load_average_1m
process_count
host_uptime_seconds

```

规则：

1. 只使用 `a-z`、`0-9`、`_`；不使用驼峰、连字符、点号、百分号。

2. 单位后缀只在非无量纲时出现：`_bytes`、`_seconds`、`_hertz`、`_celsius`、`_bytes_per_second`。

3. 比例统一 `_ratio`，值域 `[0,1]`；不使用 `_percent`、`_pct`。

4. 计数器统一 `_total`；`_total` 只用于 counter，不用于 gauge/derived。

5. 元数据用 `_info`，值恒为 1；具体信息放标签。

6. 不使用容易歧义的缩写；`cpu`、`io`、`os`、`mtu`、`pid` 是允许的固定缩写。

7. 名字一旦发布不得改名；只能新增别名或走 major schema 升级。

**Prometheus 映射**

* 统一加前缀 `sprobe_`，可通过配置 `metrics.namespace` 修改；默认 `sprobe`。

* 计数器保留 `_total`；Prometheus 服务端用 `rate()`/`increase()` 计算速率。

* 比率直接暴露 `_ratio`，不乘 100。

* 信息类指标输出 `sprobe_*_info{...} 1`。

* JSON 中的数组（CPU 核、磁盘、网卡、文件系统）在 Prometheus 中展开为带标签的序列；JSON 中的聚合值不重复输出，避免求和重复。

* 进程 Top-N 默认不进入 Prometheus；只输出聚合指标，防止基数爆炸。需要逐进程时使用 JSON 或单独的 opt-in 开关。

**标签命名**

| 标签 | 适用指标 | 取值 | 
| ----- | ----- | ----- | 
| `cpu` | CPU 分核指标 | `"0"`、`"1"`…；聚合值在 JSON 的 `cpu.total`，不输出 `cpu="total"` | 
| `mode` | CPU 时间 | `user`、`nice`、`system`、`idle`、`iowait`、`irq`、`softirq`、`steal` | 
| `device` | 磁盘/文件系统 | 设备名，如 `sda`、`nvme0n1`、`C:` | 
| `mountpoint` | 文件系统 | 规范化后的挂载点；去重后取最短路径 | 
| `fstype` | 文件系统 | `ext4`、`xfs`、`btrfs`、`ntfs`、`apfs` 等 | 
| `interface` | 网络 | 接口名，如 `eth0`、`ens5`、`Ethernet 2` | 
| `state` | 进程聚合 | `running`、`sleeping`、`blocked`、`zombie`、`stopped`、`unknown` | 
| `check` | 健康检查 | 检查 ID，如 `disk_space` | 
| `severity` | 健康/告警 | `info`、`warning`、`critical` | 

### 4.3 核心指标清单（CPU/内存）

磁盘、网络、负载、进程、开机时长与健康指标的规范名、类型、单位和默认频率统一汇总在附录 A；逐项口径、平台差异与边界处理见 4.6–4.11。本节先给出 CPU 与内存的逐项定义，因为这两类指标的口径争议最大。

#### 4.3.1 CPU

| 指标键 | 类型 | 单位 | 口径 | 默认频率 | 
| ----- | ----- | ----- | ----- | ----- | 
| `cpu_usage_ratio` | gauge | ratio | 非 idle、非 iowait、非 steal 的 CPU 时间占比；`(user+nice+system+irq+softirq)/total` | 10s | 
| `cpu_idle_ratio` | gauge | ratio | `idle/total` | 10s | 
| `cpu_iowait_ratio` | gauge | ratio | `iowait/total`；仅 Linux 有值，其余 `null` | 10s | 
| `cpu_steal_ratio` | gauge | ratio | `steal/total`；虚拟化环境有意义，其余 `null` | 10s | 
| `cpu_user_seconds_total` | counter | seconds | 用户态时间，含 guest（Linux guest 已计入 user，不重复加） | 10s | 
| `cpu_nice_seconds_total` | counter | seconds | 低优先级用户态时间；Windows/macOS 可为 `null` | 10s | 
| `cpu_system_seconds_total` | counter | seconds | 内核态时间；Windows 需从 kernel time 中减去 idle | 10s | 
| `cpu_idle_seconds_total` | counter | seconds | 空闲时间 | 10s | 
| `cpu_iowait_seconds_total` | counter | seconds | 等待 I/O 时间；仅 Linux | 10s | 
| `cpu_irq_seconds_total` | counter | seconds | 硬中断时间；仅 Linux | 10s | 
| `cpu_softirq_seconds_total` | counter | seconds | 软中断时间；仅 Linux | 10s | 
| `cpu_steal_seconds_total` | counter | seconds | 被 hypervisor 抢占的时间；Linux/虚拟化 | 10s | 
| `cpu_frequency_hertz` | gauge | hertz | 当前频率；分核可用；未知为 `null` | 30s | 
| `cpu_logical_count` | gauge | count | 逻辑 CPU 数 | 300s | 
| `cpu_physical_count` | gauge | count | 物理核数；无法可靠获取时为 `null` | 300s | 

说明：

* `cpu_usage_ratio + cpu_idle_ratio + cpu_iowait_ratio + cpu_steal_ratio = 1`（在字段可用的平台上）；由于四舍五入，允许 ±0.0002 的误差。

* 分核指标放在 `cpu.cores[]`，字段名相同；聚合值放在 `cpu.total`。**聚合必须由各核 tick 求和后计算，不能对各核 ratio 取平均**。

* 不采集 `guest` 独立指标，避免与 `user` 重复计数；如后续需要，新增 `cpu_guest_seconds_total` 并在文档中说明与 user 的关系。

* Windows 的 `GetSystemTimes` 返回 idle/kernel/user，其中 kernel 包含 idle；`system = kernel - idle`。Windows 无 iowait/steal/nice，对应字段为 `null`。

* macOS 使用 `host_processor_info` 的 CPU tick；`nice` 可用，`iowait/steal/irq` 为 `null`。

#### 4.3.2 内存与交换

| 指标键 | 类型 | 单位 | 口径 | 默认频率 | 
| ----- | ----- | ----- | ----- | ----- | 
| `memory_total_bytes` | gauge | bytes | 物理内存总量 | 10s | 
| `memory_available_bytes` | gauge | bytes | 可分配给新进程的估计可用内存；口径见 4.5 | 10s | 
| `memory_used_bytes` | gauge | bytes | `total - available`；不是 `total - free` | 10s | 
| `memory_free_bytes` | gauge | bytes | 完全空闲的物理内存 | 10s | 
| `memory_cached_bytes` | gauge | bytes | 页缓存/系统缓存；平台口径不同 | 30s | 
| `memory_buffers_bytes` | gauge | bytes | 内核缓冲区；仅 Linux | 30s | 
| `memory_usage_ratio` | gauge | ratio | `used / total`；`total=0` 时为 `null` | 10s | 
| `memory_dirty_bytes` | gauge | bytes | 待回写脏页；仅 Linux | 30s | 
| `memory_writeback_bytes` | gauge | bytes | 正在回写的页；仅 Linux | 30s | 
| `swap_total_bytes` | gauge | bytes | 交换区/页面文件总量 | 10s | 
| `swap_used_bytes` | gauge | bytes | 已使用交换区/页面文件 | 10s | 
| `swap_free_bytes` | gauge | bytes | 空闲交换区/页面文件 | 10s | 
| `swap_usage_ratio` | gauge | ratio | `used / total`；`total=0` 时为 `null` | 10s | 

说明：

* `memory_used_bytes` 与 `memory_available_bytes` 互补；禁止同时用 `free` 计算“已用”，否则会把缓存算成已用。

* `memory_cached_bytes` 在 Windows 上取系统缓存（`GetPerformanceInfo` 的 `SystemCache` × 页大小）；在 macOS 上取 inactive + purgeable 的近似值；跨平台不可直接比较。

* `memory_usage_ratio` 的平台口径差异必须在 `memory.available_source` 中声明：`linux_memavailable`、`linux_fallback`、`windows_avail_phys`、`macos_free_inactive`。

### 4.4 CPU 采集细节

#### 4.4.1 时间模型

CPU 指标全部由“累计 tick”差分得到。设第 `n` 次采样时某 CPU 的各模式累计时间为 `T_n[mode]`，采样间隔为 `Δt`：

```
total_n = Σ T_n[mode]                  （所有模式求和，不含 guest 重复项）
usage_n = (total_n - total_{n-1} - Δidle - Δiowait - Δsteal) / (total_n - total_{n-1})
idle_n  = Δidle / Δtotal
iowait_n= Δiowait / Δtotal             （仅 Linux）
steal_n = Δsteal / Δtotal              （仅虚拟化平台）

```

约束：

* `Δtotal ≤ 0` 视为计数器异常，本周期所有 CPU 比例为 `null`，重建基线并记录 `counter_reset` 事件。

* 首次采样只有基线，`usage/idle/iowait/steal` 均为 `null`，不得输出 0。

* 采样间隔必须 ≥ 1 秒；小于 1 秒的配置在启动时拒绝。`sysinfo` 的 CPU 差分也有最小间隔约束，调度器必须保证同一采集器两次执行的实际间隔不小于 1 秒。

* 聚合 CPU 的 tick 必须由各核 tick 求和后再计算；**禁止对各核比例求平均**。8 核中 1 核 100%、7 核 0% 的平均比例是 12.5%，求和口径同样是 12.5%，但各核负载分布不同时两种算法会分叉；求和口径与 `top`/`vmstat` 的整机口径一致。

#### 4.4.2 平台数据源与映射

| 平台 | 数据源 | 模式映射 | 注意事项 | 
| ----- | ----- | ----- | ----- | 
| Linux | `/proc/stat` 的 `cpu`/`cpuN` 行 | `user`、`nice`、`system`、`idle`、`iowait`、`irq`、`softirq`、`steal`、`guest`、`guest_nice` | guest 已包含在 user 中，不另加；单位是 `USER_HZ`，通常 100，但必须用 `sysconf(_SC_CLK_TCK)` 换算，不能硬编码 | 
| Linux 容器 | `/proc/stat` + cgroup `cpu.stat` | 同上；容器 CPU 限额单独输出 | `/proc/stat` 是宿主机口径；容器内看到的核数是宿主机的，不能当作容器可用核数 | 
| Windows | `GetSystemTimes` 或 PDH `Processor(_Total)\% Processor Time` | `idle`、`kernel`、`user`；`system = kernel - idle` | `GetSystemTimes` 的 kernel 含 idle；PDH 的 `% Processor Time` 已是比例，不能当累计 counter 用 | 
| macOS | `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` | `user`、`system`、`idle`、`nice` | tick 为 `natural_t` 32 位，必须做回绕处理；Apple Silicon 上部分字段为 0 | 
| ARM/aarch64 Linux | `/proc/stat` | 同 Linux | 核数、频率、拓扑差异大；big.LITTLE 分核频率必须分核采集，不能取一个全局值 | 

#### 4.4.3 分核、聚合与拓扑

* `cpu.cores[]`：每个逻辑 CPU 一个元素，字段与聚合指标同名；`cpu` 标签/索引从 0 开始，与操作系统编号一致。

* `cpu.total`：整机聚合，只输出一份；Prometheus 不输出 `cpu="total"`，由查询端对分核序列求和，避免与聚合序列重复计数。

* `cpu_logical_count`：Linux 取 `/proc/cpuinfo` 的 processor 数或 `sysconf(_SC_NPROCESSORS_ONLN)`；Windows 取 `GetActiveProcessorCount(ALL_PROCESSOR_GROUPS)`；macOS 取 `hw.logicalcpu`。

* `cpu_physical_count`：Linux 通过 `/sys/devices/system/cpu/cpu*/topology/core_id` 与 `physical_package_id` 去重；Windows 用 `GetLogicalProcessorInformationEx(RelationProcessorCore)`；macOS 用 `hw.physicalcpu`。无法可靠获取时为 `null`，不得用逻辑核数代替。

* CPU 热插拔/离线：某个 `cpuN` 在本次快照中消失时，从 `cpu.cores[]` 移除并在 `events` 中记录 `cpu_offline`；重新出现时按首次采样处理（`null` 一个周期）。

* 核数超过 512 时，按编号排序截断并设置 `cpu.truncated=true`；聚合值仍按全部核计算。

#### 4.4.4 频率

* Linux：优先 `/sys/devices/system/cpu/cpuN/cpufreq/scaling_cur_freq`（kHz）；无 cpufreq 时尝试 `/proc/cpuinfo` 的 `cpu MHz`（仅 x86 常见）；ARM 的 `cpuinfo_cur_freq` 可能需要 root。都不可用时 `cpu_frequency_hertz=null`。

* Windows：`CallNtPowerInformation(ProcessorInformation)` 可获取当前频率，但依赖权限与电源策略；不可用时为 `null`，不用标称频率伪造。

* macOS：Intel 可用 `hw.cpufrequency`；Apple Silicon 通常不可用，输出 `null`。

* 频率是瞬时值，默认 30 秒采集；不参与 CPU 使用率计算。

### 4.5 内存采集细节

#### 4.5.1 可用内存的口径

“可用内存”直接影响告警，必须按平台声明来源，不能跨平台直接比较。

| 平台 | `available` 来源 | `used` | 说明 | 
| ----- | ----- | ----- | ----- | 
| Linux ≥ 3.14 | `/proc/meminfo` 的 `MemAvailable` | `MemTotal - MemAvailable` | 内核估算的可回收内存，最接近“还能给新进程用多少” | 
| Linux < 3.14 或字段缺失 | `MemFree + Buffers + Cached + SReclaimable - Shmem` | `MemTotal - available` | 近似公式；`Cached` 已含 `SReclaimable` 的一部分，按此公式扣减可避免重复；必须在 `available_source=linux_fallback` 中声明 | 
| Windows | `GlobalMemoryStatusEx.ullAvailPhys` | `ullTotalPhys - ullAvailPhys` | 系统给出的可用物理内存 | 
| macOS | `free + inactive` | `hw.memsize - available` | `speculative` 通常计入 free；`purgeable` 是否计入需在实现中固定并写入文档 | 
| 容器 | 宿主 `/proc/meminfo` 或 cgroup `memory.current/memory.max` | 按所选 scope 计算 | 默认 `scope=host`；cgroup 口径单独输出，不能混用 | 

字段补充：

* `memory_free_bytes` 是 `MemFree`/`ullAvailPhys` 之外的“完全空闲”值；Linux 取 `MemFree`，Windows 可取 `ullAvailPhys`（Windows 没有独立的 free 概念，允许与 available 相同并标注），macOS 取 `free`。

* `memory_cached_bytes`：Linux 取 `Cached + SReclaimable`（是否加 `Buffers` 需固定，建议分开：cached 只放 Cached+SReclaimable，buffers 单列）；Windows 取 `SystemCache × PageSize`；macOS 取 `inactive + purgeable` 的近似值。

* `memory_dirty_bytes`/`memory_writeback_bytes` 仅 Linux；其他平台字段缺失。

* `memory_usage_ratio = used / total`；`total=0` 时为 `null`，不是 0。

* 空值判定：`MemAvailable` 存在但为 0 且 `MemTotal>0` 时，视为内核异常，输出 `null` 并记录 `memory_available_invalid` 事件，不用 0 触发“内存耗尽”告警。

#### 4.5.2 交换区/页面文件

| 平台 | 总量 | 已用 | 说明 | 
| ----- | ----- | ----- | ----- | 
| Linux | `SwapTotal` | `SwapTotal - SwapFree` | 包含 swap 文件/分区；zram/zswap 是否计入需固定，默认计入 `SwapTotal`，并在 `swap.source` 标注 | 
| Windows | `EnumPageFiles` 各页面文件 `TotalSize` 之和 | `TotalInUse` 之和 | 不能用 `ullTotalPageFile`（它是提交限制，含物理内存）；页面文件可能多个，必须求和 | 
| macOS | `sysctl vm.swapusage` 的 `xsw_total` | `xsw_used` | 单位是字节；动态增长，采样间隔内可能变化很大 | 

* `swap_total_bytes=0` 时，`swap_used_bytes=0`、`swap_free_bytes=0`、`swap_usage_ratio=null`；`null` 表示“无交换区，比例无定义”，不是“采样失败”。

* 容器内默认不输出 swap 指标，除非显式启用 cgroup v2 `memory.swap.*`；避免把宿主 swap 当成容器 swap。

#### 4.5.3 其他边界

* 大页（HugePages）：`MemTotal` 不含大页预留时，`used` 会低估；v1 不单独折算大页，文档声明该差异；需要精确口径时后续新增 `memory_hugepages_*`。

* `MemAvailable` 可能大于 `MemFree`，这是正常的；不得对 `available` 做 `min(total, ...)` 之外的裁剪，也不得为负；若计算结果为负，置 0 并记录异常。

* 内存热插拔/虚拟机 balloon：总量变化时，`used` 可能突降；不视为计数器重置，不触发告警，仅在 `events` 中记录 `memory_total_changed`。

* 32 位进程/系统：`u64` 字段在 JSON 中用 number 表示；超过 2^53 时消费端按字符串或 decimal 解析；agent 内部始终用 `u64`。

### 4.6 磁盘与文件系统采集细节

#### 4.6.1 文件系统选择

* 数据源：Linux `statvfs`/`statfs`、Windows `GetDiskFreeSpaceExW`、macOS `statfs`。

* 默认排除的伪文件系统：`proc`、`sysfs`、`devtmpfs`、`devpts`、`cgroup`、`cgroup2`、`securityfs`、`debugfs`、`tracefs`、`pstore`、`bpf`、`configfs`、`fusectl`、`mqueue`、`hugetlbfs`、`autofs`、`binfmt_misc`、`rpc_pipefs`、`nsfs`、`squashfs`（只读镜像，容量无意义）。

* `overlay`：容器场景默认包含；物理机上的 overlay 挂载默认排除，由配置 `filesystems.include_fstypes` 覆盖。

* 网络文件系统（`nfs`、`nfs4`、`cifs`、`smbfs`、`fuse.sshfs`）：默认包含，但 `statfs` 必须运行在**独立 2 线程网络文件系统探测池**中，超时 2 秒；**按挂载点执行在途防重入**（若该挂载点已有未返回的 `statfs` 阻塞调用，后续周期禁止再次派发新线程，直接返回缓存并标 `stale=true`）；连续 3 次超时后触发该挂载点 5 分钟退避熔断并标记 `status=timeout`，绝不阻塞本地文件系统及其他采集器。

* 允许 `filesystems.include_mountpoints` / `exclude_mountpoints` 做精确覆盖；匹配使用规范化绝对路径，不做正则（避免误伤和性能问题）。

#### 4.6.2 容量口径

| 字段 | 公式 | 说明 | 
| ----- | ----- | ----- | 
| `filesystem_total_bytes` | `f_blocks × f_frsize` | 含保留块 | 
| `filesystem_free_bytes` | `f_bfree × f_frsize` | 含保留块，root 可用 | 
| `filesystem_available_bytes` | `f_bavail × f_frsize` | 非 root 可用；这是容量告警的依据 | 
| `filesystem_used_bytes` | `(f_blocks - f_bfree) × f_frsize` | 已分配使用的空间 | 
| `filesystem_usage_ratio` | `used / (used + available)` | 与 `df` 的 Use% 对齐；保留块导致 `used + available < total`，这是预期行为 | 
| `filesystem_reserved_bytes` | `total - used - available` | 保留块/元数据开销；非负，异常时置 `null` | 

* Windows 没有保留块概念，`available` 即用户可用空间，`used = total - available`，`usage_ratio = used/total`。

* `f_frsize=0` 的异常文件系统跳过并记录 `statfs_invalid`；不得用 4096 兜底猜测块大小（ARM 上可能是 16K/64K）。

* inode：Linux `f_files`、`f_ffree`、`f_favail`；`inodes_used = f_files - f_ffree`；`inodes_usage_ratio = used / total`（inode 无保留块概念）；`f_files=0` 时比率 `null`。Windows/macOS 无 inode 概念，字段缺失。

* 只读判定：Linux/macOS 取挂载标志 `ST_RDONLY`/`MNT_RDONLY`；Windows 取卷标志；`filesystem_read_only` 为 0/1，未知为 `null`。

* 挂载点去重：按 `(f_fsid, f_fstype)` 去重；bind mount 和重复挂载只保留最短挂载点，其余路径记录在 `mountpoints[]` 中（最多 8 个）。Prometheus 只输出规范化后的一个 `mountpoint`，避免同一文件系统被重复求和。

* 文件系统容量变化（扩容/缩容）不视为 counter reset；`used` 可能跳变，只记录 `filesystem_resized` 事件。

#### 4.6.3 磁盘 I/O

Linux `/proc/diskstats` 字段映射（单位必须换算）：

| 字段 | 含义 | 单位/换算 | 
| ----- | ----- | ----- | 
| `reads completed` | 成功完成的读次数 | count | 
| `sectors read` | 读扇区数 | 恒 512 字节/扇区，乘 512 得 bytes | 
| `read time` | 读操作累计耗时 | 毫秒，除以 1000 得秒 | 
| `writes completed` | 成功完成的写次数 | count | 
| `sectors written` | 写扇区数 | 乘 512 | 
| `write time` | 写操作累计耗时 | 毫秒 | 
| `io in progress` | 当前在途 I/O | count，gauge | 
| `io time` | 设备有 I/O 在途的累计时间 | 毫秒；用于利用率 | 
| `weighted io time` | 加权累计时间 | 毫秒；用于平均队列/延迟参考 | 

字段到规范指标的映射：`reads completed` → `disk_read_operations_total`，`writes completed` → `disk_write_operations_total`，`sectors read/written × 512` → `disk_read_bytes_total`/`disk_write_bytes_total`，`io time ÷ 1000` → `disk_io_time_seconds_total`，`weighted io time ÷ 1000` → `disk_io_time_weighted_seconds_total`，`io in progress` → `disk_in_flight_operations`。

派生指标：

* `disk_utilization_ratio = Δio_time / Δt`，裁剪到 `[0,1]`；`io_time` 在单设备上不应超过墙钟时间，超过说明时钟或驱动异常，置 `null` 并记录 `disk_io_time_invalid`。

* `disk_read_latency_seconds = Δread_time / Δreads`；`Δreads=0` 时为 `null`；写延迟同理。该值是“平均每次操作延迟”，不是分位数。

* `disk_in_flight_operations` 直接取当前值，不差分。

* `disk_read_bytes_per_second = Δbytes / Δt`；`disk_write_bytes_per_second` 同理；同时保留 `*_total` 原始 counter，服务端可自行计算任意窗口速率。

* 计数器回退（设备重建、驱动重置、虚拟机迁移）时，本周期的 `*_total` 仍输出当前累计值，但所有派生速率为 `null`，并记录 `disk_counter_reset`；下一周期重建基线。

设备选择与去重：

* 默认只采集整盘，不采集分区；判定依据：`/sys/block/<dev>/partition` 不存在，或 Windows 的 `PhysicalDriveN`。

* `dm-*`、`md*`、`bond*` 等虚拟设备默认包含，但必须带 `disk_class`（`physical`、`logical`、`virtual`）与 `parent`（若可解析）；文档明确“对所有设备求和会重复计算”，告警与容量报表必须按 `disk_class` 过滤。

* 设备名排序稳定；设备消失时从数组移除并记录 `disk_removed`，重新出现按首次采样处理。

* 分区表/RAID 重建期间 I/O 延迟会异常升高，不特殊处理，交由健康检查 `disk_io_latency` 判断。

Windows：

* 容量：`GetDiskFreeSpaceExW` 获取总容量、可用容量；卷 GUID 路径与盘符都要能处理。

* I/O：优先 `IOCTL_DISK_PERFORMANCE` 获取累计 `BytesRead/BytesWritten/ReadCount/WriteCount/ReadTime/WriteTime/IdleTime/QueueDepth`；该 IOCTL 通常需要管理员权限。权限不足时降级为 PDH `PhysicalDisk(*)` 的速率计数器，输出 `*_bytes_per_second`，`*_total` 为 `null`，`quality=estimated`，并在 `capabilities.disk_io_counters=false` 中声明。

* PDH 计数器名必须用英文计数器 API 添加，不能依赖本地化名称。

* 动态磁盘/存储空间：`PhysicalDisk` 与 `LogicalDisk` 可能重复，默认只取 `PhysicalDisk`，逻辑卷容量由文件系统指标覆盖。

macOS：

* 容量：`statfs`；`f_bsize` 与 `f_frsize` 不一致时以 `f_frsize` 为准。

* I/O：IOKit `IOBlockStorageDriver` 的 `Statistics` 字典提供累计字节与操作数；设备名映射到 `diskXsY`，只取整盘，避免分区重复。

* APFS 容器共享空间：多个卷的 `total/available` 可能相同，容量报表必须按容器去重，否则会高估总容量；v1 通过 `filesystem.container_id` 标识（能获取时），不能获取时在文档中声明该限制。

### 4.7 网络采集细节

#### 4.7.1 接口模型

每个接口输出一个对象，字段如下：

| 字段 | 说明 | 
| ----- | ----- | 
| `name` | 接口名，稳定标识 | 
| `class` | `physical`、`bond`、`bridge`、`vlan`、`veth`、`tunnel`、`loopback`、`other` | 
| `master` | 从属接口的 master 名；无则为 `null` | 
| `virtual` | 是否虚拟接口 | 
| `up` | `IFF_UP && IFF_RUNNING`（Windows 为 `IfOperStatusUp`）；管理 up 但无载波为 0 | 
| `mtu` | MTU 字节数 | 
| `mac` | 可选；默认不输出，避免资产指纹泄露 | 
| `speed_bytes_per_second` | 链路速率；未知为 `null`，不用 0 代替 | 
| `counters` | 收发包/字节/错误/丢弃累计值 | 

规范指标映射：`up` → `network_interface_up`，`mtu` → `network_mtu_bytes`，`speed_bytes_per_second` → `network_speed_bytes_per_second`；`counters` 中的字段分别映射到 4.7.2 的 `network_*_total`。

默认过滤：

* 排除 `loopback`（`lo`、`Loopback Pseudo-Interface`）。

* 默认排除从属接口（`master != null`），只保留 bond/bridge master；否则 bond 与 slave 会重复计数。

* 默认包含 veth/tunnel，但打 `class` 标签；是否纳入“总量”由消费端决定。

* 接口名不是稳定资产标识；跨重启可能变化，消费端应以 `agent_id + name` 组合并在服务端做别名映射。

#### 4.7.2 计数器与速率

| 指标 | 来源 | 说明 | 
| ----- | ----- | ----- | 
| `network_receive_bytes_total` | Linux `/sys/class/net/*/statistics/rx_bytes`；Windows `GetIfEntry2` 的 `InOctets`；macOS `NET_RT_IFLIST2` 的 `ifi_ibytes` | 64 位；32 位平台需处理回绕 | 
| `network_transmit_bytes_total` | 对应 `tx_bytes`/`OutOctets`/`ifi_obytes` | 同上 | 
| `network_receive_packets_total` | `rx_packets`/`InUcastPkts+InNUcastPkts`/`ifi_ipackets` | 广播/组播的包含关系需固定 | 
| `network_transmit_packets_total` | 对应字段 | 同上 | 
| `network_receive_errors_total` | `rx_errors`/`InErrors`/`ifi_ierrors` | 口径差异在文档声明 | 
| `network_transmit_errors_total` | `tx_errors`/`OutErrors`/`ifi_oerrors` | 同上 | 
| `network_receive_dropped_total` | `rx_dropped`；Windows 无直接对应时为 `null` | Linux 的 dropped 含驱动/协议栈丢弃 | 
| `network_transmit_dropped_total` | `tx_dropped`；Windows 为 `null` | 同上 | 
| `network_speed_bytes_per_second` | Linux `ethtool`/`/sys/class/net/*/speed`（Mbps ÷ 8）；Windows `TransmitLinkSpeed`/`ReceiveLinkSpeed`；macOS `ifi_baudrate` | 未知或 `-1` 时为 `null`；虚拟接口通常为 `null` | 
| `network_receive_bytes_per_second` | `Δrx_bytes / Δt` | 首周期 `null`；重置周期 `null` | 
| `network_transmit_bytes_per_second` | `Δtx_bytes / Δt` | 同上 | 
| `network_receive_utilization_ratio` | `rx_rate / speed` | `speed=null` 时 `null`；裁剪到 `[0,1]`；全双工分别计算 | 
| `network_transmit_utilization_ratio` | `tx_rate / speed` | 同上 | 

* 计数器重置判定：当前值 < 上次值，或平台明确报告 32 位回绕。重置时 `Δ=null`，不输出负数或 0 速率；从当前值重建基线。

* 接口 down/up：`up` 变化不重置计数器；接口消失（USB 网卡拔出、容器 veth 删除）时移除对象并记录 `interface_removed`；重新出现按首次采样。

* 不自动输出“全网合计”。需要总量时，由配置 `network.aggregate_interfaces` 显式列出接口名，agent 只对这些接口求和；未配置则没有合计字段。这样避免 bond/bridge/vlan/veth 重复计数。

* 带宽单位统一 bytes/s；展示层可转 Mbps，但数据层不混用 bit。

#### 4.7.3 平台差异

* Linux：`/sys/class/net` 读取统计，失败时回退 `getifaddrs` + `rtnetlink`；接口 up 状态用 `IFF_UP|IFF_RUNNING`；速率读取可能触发 `ethtool` ioctl，需权限，失败为 `null`。

* Windows：`GetIfEntry2` 获取 64 位统计与链路速率；`InterfaceIndex` 与名称映射；禁用/断开的接口可能仍返回，需按 `OperStatus` 过滤；`InOctets` 是否含 FCS 由驱动决定，文档声明。

* macOS：`NET_RT_IFLIST2` 获取 64 位 `if_data64`；`ifi_baudrate` 可能为 0；`utun`/`awdl`/`llw` 等虚拟接口默认包含但标记 `class=tunnel/other`。

* 容器：`eth0` 是 veth，计数器是容器命名空间口径；宿主侧 `vethXXX` 同时存在，二者相加会重复。默认容器内只输出 `eth0` 等非 loopback 接口，不输出宿主 veth。

### 4.8 系统负载采集细节

#### 4.8.1 Unix 原生负载

* Linux/macOS 使用 `getloadavg()`，分别得到 1/5/15 分钟负载；Linux 内核负载是“可运行 + 不可中断睡眠（D 状态）”任务数的指数衰减平均值。

* 输出：

  * `load_average_1m`、`load_average_5m`、`load_average_15m`，gauge，单位 count，不做单位后缀；

  * `load_average_1m_per_core = load1 / cpu_logical_count`，无量纲但**不命名为 ratio**，因为可以大于 1；`logical_count=0` 时为 `null`；

  * `processes_running`（Linux `/proc/stat procs_running`）、`processes_blocked`（`procs_blocked`）作为辅助；其他平台为 `null`。

* 不裁剪负载值；负载 100 在 8 核机器上不是错误，是否告警由健康检查按每核阈值判断。

* 容器内 `getloadavg` 是宿主口径；容器负载默认标记 `scope=host`，不能用容器 CPU 限额解释。cgroup v2 的 `cpu.pressure` 可作为补充，但 v1 不替代 load。

#### 4.8.2 Windows 合成负载

Windows 没有原生 load average。v1 的处理：

* 采集 PDH `System\Processor Queue Length`（就绪队列长度，5 秒采样）。

* 用三个不同时间常数的 EWMA 合成 1/5/15 分钟值，时间常数分别取 60s、300s、900s；启动后至少积累 5 个样本才输出，之前为 `null`。

* 合成值标记 `quality=estimated`、`method=windows_queue_length_ewma`，并同时输出原始 `system_processor_queue_length`。

* 文档明确：该值与 Unix load 不可直接比较，不能把 Unix 的 `load1 > 2×核数` 规则直接套到 Windows；Windows 告警用“队列长度持续 > 2×核数”加 CPU 使用率。

* 允许配置 `load.windows_synthetic=false` 关闭合成；关闭后 `load_average_*` 缺失，`system_processor_queue_length` 仍输出。

### 4.9 进程采集细节

#### 4.9.1 聚合指标

| 指标 | 定义 | 平台 | 
| ----- | ----- | ----- | 
| `process_count` | 当前可见进程总数 | 全平台 | 
| `process_running_count` | 状态为 running 的进程数 | 全平台 | 
| `process_sleeping_count` | 状态为 sleeping 的进程数 | Linux/macOS；Windows 用 waiting 近似 | 
| `process_blocked_count` | 不可中断/等待 I/O 的进程数 | Linux 的 D 状态；Windows 可为 `null` | 
| `process_zombie_count` | 僵尸进程数 | Linux/macOS；Windows 为 `null` | 
| `process_thread_count` | 线程总数 | Linux 直接累加 `/proc/<pid>/stat` 第 20 字段 `num_threads`（无需额外打开 `status`）；Windows 用 `NtQuerySystemInformation` 或 PDH `Thread Count`；macOS 用 `proc_pidinfo` | 

* 状态归一化：`running`、`sleeping`、`blocked`、`zombie`、`stopped`、`unknown`；未知状态计入 `unknown`，不丢弃。

* `process_count` 是“可见进程数”。若 `/proc` 以 `hidepid=2` 挂载或权限受限，输出 `process_visibility=partial`，并在 `capabilities` 中说明；不得把可见数伪装成总数。

* 进程数不设硬上限；Top-N 只保留 N 条，聚合计数仍按全量统计。

#### 4.9.2 Top-N 进程

默认 `N=20`，按“CPU 使用率降序、RSS 降序、PID 升序”稳定排序。每条记录：

| 字段 | 说明 | 
| ----- | ----- | 
| `pid` | 进程 ID | 
| `start_time` | 进程启动时间（UTC，秒级）；与 `pid` 组成稳定标识，防 PID 复用 | 
| `name` | 进程名；Linux 取 `comm`（最多 15 字符），Windows 取映像名，macOS 取 `p_comm` | 
| `exe` | 可执行文件 basename；默认关闭，开启时可能暴露路径信息 | 
| `state` | 归一化状态 | 
| `cpu_usage_ratio` | 相对**单核**的 CPU 使用率，可大于 1；`Δcpu_time / Δt`，不除以核数 | 
| `rss_bytes` | 常驻内存；Linux `/proc/<pid>/stat` 的 rss 页数 × 页大小；Windows `WorkingSetSize`；macOS `pti_resident_size` | 
| `vm_bytes` | 虚拟内存；可选，默认不输出（数值大且噪声高） | 

* Top-N 的 CPU 使用率需要两次采样；首次全为 `null`，按最近一次有效值排序，全无有效值时按 RSS 排序。

* 进程消失：从 Top-N 移除；不视为错误。进程新出现：本周期 CPU 为 `null`，下一周期才有值。

* 权限不足（其他用户进程、受保护进程）：该进程仍计数，但 `cpu_usage_ratio/rss_bytes` 为 `null`；不跳过该进程导致总数偏小。

* 不采集完整命令行（`cmdline`）作为默认字段；如后续需要，必须脱敏并单独开关。

#### 4.9.3 进程监视（watch）

* 配置 `processes.watch` 为进程名/可执行文件 basename 的精确列表，不支持正则（避免灾难性回溯和误匹配）。

* 每个被监视项输出 `present`、`count`、`pids[]`（最多 16 个）、`rss_bytes_total`、`cpu_usage_ratio_total`。

* 匹配规则：优先匹配 `exe` basename，其次匹配 `name`；同名多实例全部计数；`present = count > 0`。

* 用于健康检查 `process_missing`：被监视项从 `present=true` 变为 `false` 时触发，不因单个实例重启而误报。

* Prometheus 暴露 `sprobe_process_watch_present{name="..."}`（1/0），仅对被监视项输出；`name` 基数由 `processes.watch` 配置限制，默认不超过 32 项。

#### 4.9.4 平台差异与计数精度

* Linux：`/proc/<pid>/stat` 的 utime/stime 单位是 `USER_HZ`；RSS 单位是页，必须用 `sysconf(_SC_PAGESIZE)`，ARM 上可能是 64K。`/proc/<pid>/io` 仅 root 可读，v1 默认不采集。

* Windows：`GetProcessTimes` 获取内核/用户时间；`GetProcessMemoryInfo` 获取 WorkingSetSize；打开进程用 `PROCESS_QUERY_LIMITED_INFORMATION`，减少权限失败。系统空闲进程、System 进程的统计口径需在实现中固定。

* macOS：`proc_pidinfo(PROC_PIDTASKINFO)` 获取时间与内存；受 SIP 保护的进程可能失败，按 `null` 处理。

* PID 复用：`(pid, start_time)` 不同即视为新进程，不能沿用旧基线；Linux `start_time` 从 `/proc/<pid>/stat` 第 22 字段换算，Windows 用创建时间，macOS 用 `p_starttime`。

* 进程数巨大（>5000）时，Top-N 仍需全量扫描一次；扫描耗时超过采集超时则返回 `partial`，保留聚合值，Top-N 标记 `truncated=true`。

### 4.10 开机时长与时间采集细节

| 指标 | 类型 | 单位 | 定义 | 
| ----- | ----- | ----- | ----- | 
| `host_uptime_seconds` | gauge | seconds | 主机自启动以来的秒数；包含睡眠/挂起时间（若平台计数包含） | 
| `host_boot_time` | info/gauge | RFC 3339 | 启动时刻的墙上时间；用于展示与对时，不用于计算时长 | 
| `agent_uptime_seconds` | gauge | seconds | agent 进程自启动以来的秒数 | 
| `host_clock_adjusted` | gauge | 0/1 | 本次采样发现墙上时钟与单调时钟不一致超过阈值 | 

平台实现：

* Linux：`/proc/uptime` 第一字段为秒（含小数），来源是 `CLOCK_BOOTTIME`，包含 suspend；`/proc/stat` 的 `btime` 是启动时刻的 Unix 秒。`host_boot_time = btime`；若 `now - btime` 与 `/proc/uptime` 相差超过 5 秒，置 `clock_adjusted=1` 并记录事件。容器内 `/proc/uptime` 是宿主机口径，默认 `scope=host`；需要容器自身运行时长时，用 PID 1 的启动时间计算 `container_uptime_seconds`（可选字段），不得把宿主 uptime 当作容器 uptime。

* Windows：`GetTickCount64()` 返回自启动以来的毫秒数，包含睡眠；`host_boot_time = now - tick`；该值受系统时间调整影响，需与 `GetSystemTimePreciseAsFileTime` 交叉校验，偏差 >5 秒置 `clock_adjusted=1`。

* macOS：`sysctl kern.boottime` 给出启动时刻；`host_uptime_seconds = now - boottime`；睡眠时间是否计入由内核口径决定，文档声明不保证与 Linux 完全一致。

* 所有平台：持续时间计算使用单调时钟；墙上时钟回拨不得产生负的 uptime；`host_uptime_seconds` 单调不减（除重启），出现下降则判定主机重启，重建所有差分基线并记录 `host_reboot` 事件。

边界：

* 虚拟机迁移/恢复（快照还原）可能导致 uptime 回退或跳变；按“重启”处理，清空差分状态，避免 CPU/网络出现负速率。

* NTP 大步调整：`clock_adjusted=1` 只标记，不修改 uptime；告警规则不直接依赖该字段。

* 时区：所有时间输出 UTC；`host_boot_time` 不带本地时区偏移。

### 4.11 健康状态设计

#### 4.11.1 状态定义

| 状态 | 含义 | 典型原因 | 
| ----- | ----- | ----- | 
| `ok` | 所有启用的检查通过 | — | 
| `degraded` | 有非致命检查失败，服务仍可用 | 磁盘使用率 >90%、内存使用率 >90%、负载偏高、单个采集器超时 | 
| `critical` | 有致命检查失败，可能影响服务 | 磁盘可用空间 <5%、根文件系统只读、内存耗尽 | 
| `unknown` | 数据不足或采集器全部失败，无法判断 | 启动预热期、时钟异常、采集器连续失败 | 

* `health_status` 输出字符串，`health_status_code` 输出 0/1/2/3（ok/degraded/critical/unknown），便于 Prometheus 告警。

* 逐项检查在 Prometheus 中映射为 `health_check_status{check,severity}`（1=异常，0=正常）；JSON 中保留 `health.checks[]` 的完整对象。

* `health.since` 是当前状态开始时间；`health.checks[]` 列出每项检查的 `id`、`status`、`severity`、`message`、`observed`、`threshold`、`since`。

#### 4.11.2 检查项

| 检查 ID | 默认条件 | 严重级别 | 数据来源 | 
| ----- | ----- | ----- | ----- | 
| `collector_freshness` | 任一启用采集器超过 3×周期未成功 | warning | 调度器 | 
| `cpu_saturation` | `cpu_usage_ratio` 5 分钟均值 > 0.90 且 `cpu_iowait_ratio` < 0.20 | warning | CPU | 
| `memory_pressure` | `memory_usage_ratio` > 0.90 持续 5 分钟为 warning；> 0.95 持续 5 分钟为 critical | warning/critical | 内存 | 
| `disk_space` | 任一文件系统 `usage_ratio` > 0.90 为 warning；> 0.95 为 critical | warning/critical | 文件系统 | 
| `disk_inode` | 任一文件系统 inode 使用率 > 0.90 持续 5 分钟 | warning | 文件系统 | 
| `filesystem_readonly` | 非只读设计的文件系统变为只读 | critical | 文件系统 | 
| `disk_io_latency` | 读或写平均延迟 5 分钟均值 > 100ms | warning | 磁盘 I/O | 
| `network_errors` | 错误/丢弃速率持续 > 1/s 且 5 分钟内增长 | warning | 网络 | 
| `load_high` | `load_average_1m_per_core` > 2.0 持续 10 分钟 | warning | 负载 | 
| `process_missing` | 被监视进程 `present=false` 持续 2 个周期 | critical | 进程 | 
| `agent_backlog` | WAL 待发送快照 > 1000 或最旧 > 30 分钟 | warning | 推送队列 | 
| `push_auth_failed` | 最近一次推送返回 401/403 | critical | 推送客户端 | 

阈值全部可在配置中覆盖；默认值写入附录 A。健康检查只做本地判定，**agent 不发送告警通知**；通知由服务端根据上报数据统一处理，避免多 agent 重复告警。

#### 4.11.3 状态机与滞后

* 进入异常：检查连续 2 次失败才从 `ok` 进入 `degraded/critical`，避免单次抖动。

* **例外**：`push_auth_failed` 首次 401/403 立即进入 critical 并锁存，不适用“连续 2 次”规则；成功推送一次后恢复。

* 恢复：检查连续 3 次通过才恢复为 `ok`；恢复时间写入 `health.recovered_at`。

* `critical` 优先于 `degraded`：任一 critical 检查失败即整体 critical；否则任一 warning 失败即 degraded；无数据为 unknown。

* 启动后第 0 轮基线采集完成即 `ready=true`，保持 `warmup=true` 与健康 `unknown`；待第 1 个常规 fast 周期（约 10s）产出首笔有效差分后置 `warmup=false`，按实际检查结果计算。

* 检查自身异常（数据为 `null`）不等于失败：标记该检查 `unknown`，整体状态不因单个 `unknown` 升级；若全部检查 unknown，整体 `unknown`。

* 状态变化时记录事件 `health_changed`，包含旧状态、新状态、触发检查；事件进入推送批次，服务端可据此做通知去重。

### 4.12 采集频率、预热与数据新鲜度

#### 4.12.1 频率分组

| 分组 | 默认周期 | 指标 | 理由 | 
| ----- | ----- | ----- | ----- | 
| fast | 10s | CPU 使用率、内存、swap、负载（含 `/proc/stat` 运行/阻塞数）、网络速率、开机时长、健康检查 | 告警需要秒级到十秒级响应 | 
| medium | 30s | CPU 频率、缓存/脏页、磁盘 I/O 派生、进程聚合与 Top-N、文件系统容量 | 变化较慢或遍历开销高，降低 CPU 占用 | 
| slow | 300s | CPU 逻辑/物理核数、拓扑、文件系统清单、接口清单、系统信息 | 几乎不变，避免频繁扫描 | 
| push | 30s | 批量上报（默认聚批 3–6 个快照） | 降低小包请求开销；可配置 5–60s | 
| heartbeat | 30s | 独立心跳（`POST /api/v1/heartbeat`） | 无快照或推送熔断时的存活信号 | 

* 每个指标只能属于一个分组；分组周期可配置，但必须满足 `fast ≤ 30s`、`medium` 是 `fast` 的整数倍、`slow ≥ 60s`。

* 同一采集器被多个分组引用时，按最短周期执行；不同分组共享采集器结果，不重复执行。

* 所有周期加 0–20% 随机抖动，按 agent 启动时生成的稳定种子计算，避免大规模同时采样造成服务端尖峰。

#### 4.12.2 预热与首次采样

* 启动后立即执行第 0 轮 fast 基线采集（目标 ≤2s）；此时 CPU、网络、磁盘、进程 CPU 的比例/速率均为 `null`，累计 counter 有值。

* 第 0 轮基线与本地监听完成后即置 `ready=true`（满足 ≤2s 启动就绪），同时保持 `warmup=true`；下一个常规 fast 周期（约 10s 后）产生首笔有效差分后置 `warmup=false`。

* 健康状态在 `warmup=true` 期间为 `unknown`；推送可以开始，但批次中标记 `warmup=true`，服务端不得用该批次触发告警。

* 进程 Top-N 的 CPU 需要两次扫描；首次按 RSS 排序，CPU 字段为 `null`。

* Windows 合成负载需要至少 5 个 5 秒样本（约 25s）才输出；此前 `load_average_*` 为 `null`。

#### 4.12.3 数据新鲜度

* 每个分区带 `sampled_at` 与 `age_seconds`；`age_seconds` 由快照时间减分区采样时间得到，不得为负（时钟异常时置 0 并标记）。

* 慢采集器结果被复用时，`stale=true` 当 `age_seconds > 2×该指标周期`；`stale` 数据仍参与展示，但不参与告警判定（告警使用最近一次非 stale 值，超过 3×周期则检查 unknown）。

* 采集超时：单采集器默认 2s；超时返回 `partial`，保留已采到的字段，未采到的字段为 `null`，并记录 `collector_timeout`；连续 3 次超时，健康检查 `collector_freshness` 置 warning。

* 背压：调度器发现上一轮同类采集未完成时跳过本轮并计数 `collector_skipped_total`；不得并发执行同一采集器，避免 `/proc` 读取放大和基线错乱。

### 4.13 边界与异常处理总表

| 场景 | 风险 | 处理规则 | 可观测信号 | 
| ----- | ----- | ----- | ----- | 
| 首次采样 | 差分为负/巨大 | 只建基线，比例/速率为 `null` | `warmup=true` | 
| 计数器回退 | 负速率、告警误报 | 本周起 `delta=null`，重建基线 | `counter_reset` 事件 | 
| 32 位计数器回绕 | 速率突然极大 | 按回绕处理，`delta=null` | `counter_wrap` 事件 | 
| 系统时钟跳变 | 负耗时、时间戳乱序 | 用单调时钟算耗时；`clock_adjusted=1` | `clock_adjusted` | 
| 主机重启/VM 恢复 | uptime 回退、基线失效 | 清空全部差分状态 | `host_reboot` | 
| CPU 热插拔 | 核数变化、序列错位 | 移除/新增核，新增核首周期 `null` | `cpu_offline/online` | 
| 文件系统消失 | 空指针/陈旧数据 | 从数组移除，记录事件 | `filesystem_removed` | 
| NFS statfs 卡住 | 采集线程阻塞 | 2s 超时，使用缓存并标 `stale` | `filesystem_timeout` | 
| 保留块导致 used+avail<total | 误判数据错误 | 按 df 口径解释，输出 `reserved_bytes` | 文档/字段 | 
| inode 总量为 0 | 除零 | 比率 `null` | — | 
| 分区与整盘重复 | 求和翻倍 | 默认排除分区，标 `disk_class` | 文档/标签 | 
| dm/md 与底层盘重复 | 求和翻倍 | 标 `disk_class`，报表按类过滤 | 文档/标签 | 
| 网卡 bond/bridge 重复 | 流量翻倍 | 默认排除 slave，不自动合计 | `class`/`master` | 
| 链路速率未知 | 利用率误报 | 速率为 `null`，利用率为 `null` | — | 
| 接口消失/重现 | 计数器基线错乱 | 移除对象，重现按首次采样 | `interface_removed` | 
| 权限不足 | 字段缺失被当成 0 | 字段 `null` + `capabilities=false` | `permission_denied` | 
| `hidepid`/受保护进程 | 进程总数偏小 | `process_visibility=partial` | 能力声明 | 
| PID 复用 | CPU 归属错误 | 用 `(pid,start_time)` 识别 | — | 
| 进程瞬时消失 | Top-N 抖动 | 移除即可，不告警 | — | 
| 进程数过多 | 扫描超时 | 返回 `partial`，Top-N 截断 | `truncated=true` | 
| 内存 available 异常 | 误报告警 | 置 `null`，不输出 0 | `memory_available_invalid` | 
| swap 总量为 0 | 除零/误报 | 比率 `null`，用量 0 | — | 
| 容器 vs 宿主口径 | 指标不可比 | `scope` 字段声明；默认 host | `scope` | 
| Windows 性能计数器本地化 | 采集失败 | 只用英文计数器 API | `pdh_error` | 
| Windows 权限不足（磁盘 I/O） | 无累计 counter | 降级 PDH 速率并标 estimated | `capabilities` | 
| macOS 受 SIP 保护 | 进程字段缺失 | 字段 `null`，计数保留 | — | 
| ARM 64K 页 | RSS/内存换算错误 | 一律用 `sysconf(_SC_PAGESIZE)` | — | 
| `CLK_TCK` 非 100 | CPU tick 换算错误 | 用 `sysconf(_SC_CLK_TCK)` | — | 
| 非 UTF-8 进程名 | JSON 序列化失败 | 替换非法字节，保留原始字节哈希 | `encoding_replaced` | 
| 超大数值 >2^53 | JSON 精度丢失 | 后端用 u64；消费端按 decimal 解析 | 文档 | 
| NaN/Inf | JSON/Prometheus 非法 | 输出前统一转为 `null` | `invalid_number` | 
| 采样间隔 <1s | CPU 差分噪声 | 启动时拒绝配置 | 配置错误 | 
| 采集器 panic | agent 崩溃 | `catch_unwind` 隔离，标记 failed | `collector_panic` | 
| WAL 写满 | 数据丢失 | 淘汰最旧分段并计数 | `wal_evicted` | 
| 推送 401/403 | 无限重试 | 停止重试，健康 critical | `push_auth_failed` | 
| 服务端 5xx/超时 | 数据积压 | 指数退避 + 熔断，WAL 缓冲 | `push_retry` | 

### 4.14 指标质量分级

| 质量 | 定义 | 使用规则 | 
| ----- | ----- | ----- | 
| `measured` | 直接来自内核/系统 API 的原始值或简单换算 | 可用于告警与容量决策 | 
| `derived` | 由相邻样本差分/比值得到 | 可用于告警；窗口越短噪声越大，阈值需滞后 | 
| `estimated` | 平台无原生指标，由近似模型合成 | 必须带 `method`；默认不用于 critical 告警，需在告警规则中显式允许 | 

* 质量标注放在分区级（如 `cpu.quality`、`disk.quality`），不逐指标重复，避免标签爆炸；同一分区内个别指标降级时，在指标对象上单独标 `quality`。

* Prometheus 侧通过 `sprobe_*_quality` info 指标或 `quality` 标签暴露；默认不加标签，避免序列膨胀，由配置 `metrics.expose_quality` 开启。

* `estimated` 指标在健康检查中使用时必须注明；Windows 合成负载只参与 warning，不参与 critical。

## 5. 跨平台兼容策略

### 5.1 支持矩阵与分级

| 层级 | 平台 | 架构 | 支持承诺 | 
| ----- | ----- | ----- | ----- | 
| Tier 1 | Linux（musl 静态，内核 ≥ 4.14） | x86_64、aarch64 | 完整功能、零 glibc 运行时依赖、CI 全量测试、发布官方静态二进制、缺陷优先修复 | 
| Tier 1 | Windows 10/11、Windows Server 2019+ | x86_64 | 完整功能（Windows 合成负载为 estimated）、CI 测试、1.0 交付 zip + 服务安装（MSI 为可选） | 
| Tier 2 | Linux（glibc 动态，glibc ≥ 2.28） | x86_64、aarch64 | 完整功能，作为需系统 NSS/DNS 兼容场景的备选构建 | 
| Tier 2 | macOS 13+ | x86_64、arm64（Apple Silicon） | 只读监控可用；部分指标（频率、温度、磁盘 I/O）可能缺失；不作为发布阻断项 | 
| Tier 3 | FreeBSD 13+ | x86_64 | 社区探索；不承诺官方构建产物、CI 与验收，1.0 不阻塞发布 | 
| 不支持 | 32 位 x86/armv7、Android、iOS、NetBSD | — | 1.0 不承诺；ARMv7 受 time_t/Y2038 与 32 位计数器限制 | 

“支持”的含义：能编译、能启动、能输出本章定义的指标；缺失的指标必须显式 `null`/缺失并声明能力，不得静默伪造。

### 5.2 平台抽象原则

1. **统一模型，平台实现**：上层只依赖规范指标模型；平台差异封装在采集器内部，不把 `cfg(target_os)` 散布到调度、健康、输出层。

2. **编译期选择**：每个平台一个采集器实现，通过条件编译注册；不做运行时动态加载，避免 ABI 与供应链风险。

3. **能力探测优先于平台假设**：不因 `target_os=linux` 就假定有 `/proc`、cgroup v2、`ethtool`；启动时探测并写入 `capabilities`。

4. **单一 `sysinfo` 实例**：`sysinfo` 的 CPU、内存、网络等多数字段基于前后差分；同一进程必须复用同一个 `System` 实例，禁止每次采集新建实例，否则差分恒为 0 或异常。

5. **`sysinfo` 只作打底**：能用内核原生接口获得更准口径的指标（CPU 模式拆分、`MemAvailable`、`diskstats`、cgroup）优先用原生接口；`sysinfo` 用于跨平台兜底与系统信息。

6. **不依赖 locale**：所有系统调用/API 使用宽字符或英文计数器名；日志与错误消息不解析本地化文本。

7. **字节与路径**：Linux 路径按字节处理，非 UTF-8 路径用 `OsStr`；输出时非法 UTF-8 替换并记录，不因单个路径导致整批失败。

8. **整数与时间**：统一用 `u64`/`i64` 纳秒或秒；32 位平台显式处理回绕；不使用 `time_t` 直接做差值。

### 5.3 架构相关注意事项

**x86_64（AMD/Intel）**

* 不依赖 CPU 厂商：AMD EPYC/Ryzen 与 Intel Xeon/Core 走同一路径；不读取 MSR，不使用 RAPL 作为必需指标。

* 温度不是必需指标：AMD 常用 `k10temp`，Intel 常用 `coretemp`，虚拟机/云主机通常不可见；v1 不把温度纳入健康检查，缺失不影响 `ok`。

* 频率：优先 `cpufreq`；`/proc/cpuinfo` 的 `cpu MHz` 在部分 AMD 平台是标称值或估算值，标记 `quality=estimated`。

* 虚拟化：`hypervisor` 标志、`steal` 指标可用；云主机上部分磁盘/网络计数器被 hypervisor 过滤，按缺失处理。

**aarch64（ARM64）**

* 页大小可能是 4K、16K、64K：RSS、内存页换算一律用 `sysconf(_SC_PAGESIZE)`，禁止硬编码 4096；解析 `/proc/meminfo` 时用 kB 字段换算，不依赖页大小。

* `USER_HZ`/`CLK_TCK` 不能假定 100：用 `sysconf(_SC_CLK_TCK)`；`/proc/stat` 的 tick 换算错误会导致 CPU 使用率整体偏移。

* big.LITTLE/大小核：分核频率可能不同，`cpu_frequency_hertz` 必须分核；聚合频率默认不输出，避免用一个数代表全部核。

* 拓扑：`core_id`、`physical_package_id` 在部分 ARM 平台上缺失或重复；物理核数无法确定时为 `null`，不用逻辑核数代替。

* 树莓派：建议交叉编译或使用官方 aarch64 构建；`/sys/class/thermal` 与 `vcgencmd` 不作为必需依赖；USB 网卡/磁盘可能热插拔，按设备消失/重现处理。

* 32 位 ARM 不进入 Tier 1：`time_t` 32 位、Y2038、32 位网络计数器回绕会显著增加复杂度；1.0 明确不支持，避免“能编译但数据不可信”。

**Windows**

* PDH 计数器名本地化：必须用英文计数器 API 添加；中文/德文系统上按英文名查找会失败。

* 性能计数器可能损坏：`PdhCollectQueryData` 返回 `PDH_CSTATUS_INVALID_DATA` 时重建查询，连续失败降级为 `null`，不阻塞其他指标。

* `GetSystemTimes` 的 kernel 含 idle；`GetProcessTimes` 的权限要求；`GlobalMemoryStatusEx` 的 `ullAvailPhys` 与 `ullAvailPageFile` 含义不同，不能混用。

* 路径/卷：盘符可能变化，卷 GUID 更稳定；文件系统指标同时保留 `device`（盘符）与 `volume_id`（可获取时）。

* 服务运行在 Session 0，不能依赖交互式桌面 API；不使用 WMI（慢且可能被策略禁用），优先 Win32 API/PDH。

**macOS**

* Apple Silicon 与 Intel 差异：频率、温度、部分 IOKit 字段不可用；按 `null` 处理。

* SIP/TCC：受保护进程信息可能读取失败；不请求额外权限，按缺失处理。

* APFS：卷共享容器空间，容量去重见 4.6；`statfs` 的 `f_bsize` 与 `f_frsize` 可能不同。

### 5.4 构建与发布矩阵

| 目标 | triple | 产物 | 说明 | 
| ----- | ----- | ----- | ----- | 
| Linux x86_64 musl | `x86_64-unknown-linux-musl` | `sprobe-x86_64-linux-musl.tar.gz` | Tier 1 官方默认静态基线（无外部动态库） | 
| Linux aarch64 musl | `aarch64-unknown-linux-musl` | `sprobe-aarch64-linux-musl.tar.gz` | Tier 1 官方默认静态基线（交叉编译 + 真机验证） | 
| Windows x86_64 | `x86_64-pc-windows-msvc` | `sprobe-x86_64-windows.zip`（MSI 可选） | Tier 1 官方构建，签名可选 | 
| Linux x86_64 gnu | `x86_64-unknown-linux-gnu` | `sprobe-x86_64-linux-gnu.tar.gz` | Tier 2 备选动态构建（glibc ≥ 2.28） | 
| Linux aarch64 gnu | `aarch64-unknown-linux-gnu` | `sprobe-aarch64-linux-gnu.tar.gz` | Tier 2 备选动态构建（glibc ≥ 2.28） | 
| macOS x86_64 | `x86_64-apple-darwin` | `.tar.gz` | Tier 2 | 
| macOS arm64 | `aarch64-apple-darwin` | `.tar.gz` | Tier 2 |

* FreeBSD 不进入 1.0 官方发布矩阵；仅接受社区自行编译与反馈，不提供官方二进制、安装包或验收承诺。 

* 发布产物附带 `SHA256SUMS`、构建信息（版本、commit、Rust 版本、目标 triple、构建时间）。

* 最低 Rust 版本（MSRV）由 `sysinfo` 等依赖决定；当前锁定 `sysinfo 0.40.x`，MSRV 为 **1.95**，CI 用该版本做一次构建校验。

* 交叉编译优先使用 `cross` 或 `cargo-zigbuild`；aarch64 产物必须在真实 arm64 机器或 QEMU 上跑冒烟测试，不能只编译通过。

* 不在运行时下载任何平台相关库；所有依赖静态/随包分发，避免目标机缺库。

### 5.5 容器、虚拟机与裸金属

| 环境 | 差异 | 处理 | 
| ----- | ----- | ----- | 
| 裸金属 | 指标最完整 | 默认 `scope=host`，全部采集器启用 | 
| 虚拟机 | 部分 CPU/磁盘/网络计数器由 hypervisor 过滤；`steal` 有意义 | 缺失字段 `null`；`cpu_steal_ratio` 正常输出；磁盘/网卡按虚拟设备标 `virtual` | 
| 容器（host 视角） | 能看到宿主全部进程与网卡 | 默认 host 口径；`process_count` 是宿主进程数，需在文档声明 | 
| 容器（容器视角） | `/proc` 可能是宿主或命名空间口径；cgroup 限额才是容器配额 | 默认 host 口径；启用 cgroup 采集时额外输出 `container_*` 指标，不覆盖 host 指标 | 
| Windows 容器 | 性能计数器按容器命名空间隔离 | 只支持进程内可见计数器；缺失按 `null` | 
| 只读根文件系统 | 无法写 WAL/日志 | 启动时检测，WAL 目录不可写则降级为内存队列并告警；日志输出 stderr | 

### 5.6 兼容性测试策略

* CI 矩阵：Linux x86_64、Linux aarch64（QEMU 或原生 runner）、Windows x86_64；macOS 做编译+基础冒烟，不作为发布阻断。

* 每个平台至少覆盖：启动/退出、`/metrics` 可抓取、快照 JSON schema 校验、CPU/内存/磁盘/网络/负载/进程/uptime/健康字段存在性、权限不足降级、计数器重置模拟。

* 架构专项：ARM 上验证 64K 页、非 100 `CLK_TCK`、大小核频率；x86_64 上验证 AMD/Intel 各一台；Windows 上验证中文/英文系统各一台。

* 兼容性回归以“字段存在 + 类型正确 + 口径抽样比对”为准，不要求跨平台数值相等。

## 6. 数据结构与接口约定

本章定义 agent 内部快照、上报批次与服务端响应的数据契约。所有字段名、类型、空值语义都是接口的一部分；实现可以调整内部结构，但序列化结果必须符合本章。

### 6.1 通用约定

| 约定 | 规则 | 
| ----- | ----- | 
| 编码 | UTF-8，无 BOM；JSON 不允许注释、尾随逗号、`NaN`、`Infinity` | 
| 命名 | 字段一律 `snake_case`；布尔字段用 `true/false`，不用 0/1（Prometheus 侧另转 0/1） | 
| 数值 | 整数用 JSON number；比率保留 4 位小数；字节/计数为整数；时间戳为字符串 | 
| 空值 | 未知/不可用用 `null`；不支持用字段缺失 + `capabilities` 声明；禁止 `-1`、空字符串、`"N/A"` | 
| 数组 | 无数据时为 `[]`，不是 `null`；数组元素顺序稳定（按名称/ID 排序） | 
| 对象 | 固定字段，不动态增删；扩展字段放 `labels`/`attributes` | 
| 字符串长度 | 普通字段 ≤ 256 字节；`message` ≤ 1024 字节；超长截断并标 `truncated` | 
| 标签 | `labels` 为 `map<string,string>`，键 ≤ 64 字节、值 ≤ 256 字节，最多 32 对 | 
| 时间 | UTC、RFC 3339、毫秒精度；持续时间用秒（浮点） | 
| 版本 | `schema_version` 为 `"major.minor"`，当前 `"1.0"` | 

**规范名与 JSON 路径的映射**

JSON 采用分区嵌套，字段名用短名；规范指标名由“分区名 + 字段名”拼接，例外单独列出。这样既避免 `memory.memory_total_bytes` 的重复，也保证文档、Prometheus、JSON 三者可机械对应。

| JSON 路径 | 规范指标名 | 
| ----- | ----- | 
| `cpu.total.usage_ratio` | `cpu_usage_ratio` | 
| `cpu.cores[].user_seconds_total` | `cpu_user_seconds_total{cpu="N"}` | 
| `memory.total_bytes` | `memory_total_bytes` | 
| `filesystems.items[].available_bytes` | `filesystem_available_bytes{mountpoint="..."}` | 
| `disks.items[].read_bytes_total` | `disk_read_bytes_total{device="..."}` | 
| `network.interfaces[].receive_bytes_total` | `network_receive_bytes_total{interface="..."}` | 
| `load.average_1m` | `load_average_1m` | 
| `processes.count` | `process_count` | 
| `uptime.host_seconds` | `host_uptime_seconds`（例外，不拼成 `uptime_host_seconds`） | 
| `health.status_code` | `health_status_code`（例外） | 

### 6.2 快照对象

快照是 agent 一次采集的完整结果，也是推送与 `/metrics` 的共同数据源。所有监控域统一封装为分区对象，携带 `status`、`quality`、`sampled_at`、`age_seconds`、`errors` 字段，确保局部失败与降级可被服务端精确识别。

```
{
  "schema_version": "1.0",
  "snapshot_id": "0f4f4a0e-6d7a-4a0e-9f0e-2f6d8c1b7a11",
  "sequence": 123456,
  "collected_at": "2026-11-21T01:20:51.123Z",
  "agent": {
    "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
    "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
    "version": "1.0.0",
    "hostname": "web-01",
    "os": "linux",
    "arch": "x86_64",
    "labels": {"env": "prod", "role": "web"}
  },
  "host": {
    "hostname": "web-01",
    "os_name": "Ubuntu 24.04.1 LTS",
    "kernel": "6.8.0-45-generic",
    "architecture": "x86_64",
    "virtualization": "kvm",
    "cpu_model": "AMD EPYC 7B13 64-Core Processor",
    "boot_time": "2026-11-01T03:12:44Z"
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
      "process_io": false
    },
    "unsupported_reasons": {
      "process_io": "permission_denied"
    }
  },
  "cpu": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:51.100Z",
    "age_seconds": 0.023,
    "truncated": false,
    "errors": [],
    "logical_count": 8,
    "physical_count": 4,
    "total": {
      "usage_ratio": 0.2315,
      "idle_ratio": 0.7480,
      "iowait_ratio": 0.0180,
      "steal_ratio": 0.0025,
      "user_seconds_total": 184223.45,
      "nice_seconds_total": 12.30,
      "system_seconds_total": 22110.02,
      "idle_seconds_total": 991234.56,
      "iowait_seconds_total": 8123.45,
      "irq_seconds_total": 0.00,
      "softirq_seconds_total": 345.67,
      "steal_seconds_total": 120.00
    },
    "cores": [
      {
        "cpu": "0",
        "usage_ratio": 0.3010,
        "idle_ratio": 0.6800,
        "iowait_ratio": 0.0190,
        "steal_ratio": 0.0000,
        "user_seconds_total": 23110.12,
        "nice_seconds_total": 1.20,
        "system_seconds_total": 2870.33,
        "idle_seconds_total": 123456.78,
        "iowait_seconds_total": 1023.45,
        "irq_seconds_total": 0.00,
        "softirq_seconds_total": 42.10,
        "steal_seconds_total": 0.00,
        "frequency_hertz": 2450000000
      }
    ]
  },
  "memory": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:51.105Z",
    "age_seconds": 0.018,
    "errors": [],
    "available_source": "linux_memavailable",
    "total_bytes": 68719476736,
    "available_bytes": 41234567168,
    "used_bytes": 27484909568,
    "free_bytes": 8123456716,
    "cached_bytes": 20123456789,
    "buffers_bytes": 123456789,
    "usage_ratio": 0.4000,
    "dirty_bytes": 102400,
    "writeback_bytes": 0,
    "swap_total_bytes": 8589934592,
    "swap_used_bytes": 123456789,
    "swap_free_bytes": 8466477803,
    "swap_usage_ratio": 0.0144
  },
  "filesystems": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:30.050Z",
    "age_seconds": 21.073,
    "truncated": false,
    "errors": [],
    "items": [
      {
        "device": "/dev/nvme0n1p2",
        "volume_id": null,
        "mountpoint": "/",
        "mountpoints": ["/"],
        "fstype": "ext4",
        "read_only": false,
        "total_bytes": 107374182400,
        "used_bytes": 42949672960,
        "free_bytes": 64424509440,
        "available_bytes": 60129542144,
        "reserved_bytes": 4294967296,
        "usage_ratio": 0.4167,
        "inodes_total": 6553600,
        "inodes_used": 812345,
        "inodes_free": 5741255,
        "inodes_usage_ratio": 0.1239,
        "stale": false,
        "quality": "measured"
      }
    ]
  },
  "disks": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:30.060Z",
    "age_seconds": 21.063,
    "truncated": false,
    "errors": [],
    "items": [
      {
        "device": "nvme0n1",
        "disk_class": "physical",
        "parent": null,
        "read_bytes_total": 1234567890123,
        "write_bytes_total": 987654321098,
        "read_operations_total": 45678901,
        "write_operations_total": 34567890,
        "io_time_seconds_total": 87654.32,
        "io_time_weighted_seconds_total": 123456.78,
        "in_flight_operations": 2,
        "read_bytes_per_second": 1048576.00,
        "write_bytes_per_second": 524288.00,
        "utilization_ratio": 0.4210,
        "read_latency_seconds": 0.00042,
        "write_latency_seconds": 0.00061,
        "quality": "measured"
      }
    ]
  },
  "network": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:51.110Z",
    "age_seconds": 0.013,
    "truncated": false,
    "errors": [],
    "interfaces": [
      {
        "name": "eth0",
        "class": "physical",
        "master": null,
        "virtual": false,
        "up": true,
        "mtu": 1500,
        "speed_bytes_per_second": 1250000000,
        "receive_bytes_total": 987654321098,
        "transmit_bytes_total": 876543210987,
        "receive_packets_total": 812345678,
        "transmit_packets_total": 712345678,
        "receive_errors_total": 0,
        "transmit_errors_total": 0,
        "receive_dropped_total": 12,
        "transmit_dropped_total": 3,
        "receive_bytes_per_second": 12500000.00,
        "transmit_bytes_per_second": 6250000.00,
        "receive_utilization_ratio": 0.0100,
        "transmit_utilization_ratio": 0.0050
      }
    ]
  },
  "load": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:51.112Z",
    "age_seconds": 0.011,
    "errors": [],
    "average_1m": 1.85,
    "average_5m": 1.72,
    "average_15m": 1.55,
    "average_1m_per_core": 0.2313,
    "processes_running": 3,
    "processes_blocked": 0,
    "processor_queue_length": null
  },
  "processes": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:30.080Z",
    "age_seconds": 21.043,
    "errors": [],
    "visibility": "full",
    "count": 187,
    "running_count": 2,
    "sleeping_count": 180,
    "blocked_count": 1,
    "zombie_count": 0,
    "thread_count": 642,
    "truncated": false,
    "top": [
      {
        "pid": 12345,
        "start_time": "2026-11-20T10:11:12Z",
        "name": "nginx",
        "state": "sleeping",
        "cpu_usage_ratio": 0.4210,
        "rss_bytes": 134217728
      }
    ],
    "watch": [
      {
        "name": "nginx",
        "present": true,
        "count": 4,
        "pids": [12345, 12346, 12347, 12348]
      }
    ]
  },
  "uptime": {
    "status": "ok",
    "quality": "measured",
    "sampled_at": "2026-11-21T01:20:51.118Z",
    "age_seconds": 0.005,
    "errors": [],
    "host_seconds": 1712345.67,
    "host_boot_time": "2026-11-01T03:12:44Z",
    "agent_seconds": 86400.12,
    "clock_adjusted": false
  },
  "health": {
    "status": "degraded",
    "status_code": 1,
    "since": "2026-11-21T01:19:41Z",
    "recovered_at": null,
    "checks": [
      {
        "id": "disk_space",
        "status": "degraded",
        "severity": "warning",
        "message": "filesystem / usage 91.2% > 90%",
        "observed": 0.912,
        "threshold": 0.90,
        "since": "2026-11-21T01:19:41Z"
      }
    ]
  },
  "events": [
    {
      "event_id": "c1f2e3d4-5a6b-4c7d-8e9f-0a1b2c3d4e5f",
      "type": "collector_timeout",
      "severity": "warning",
      "occurred_at": "2026-11-21T01:19:58Z",
      "details": {"collector": "filesystem", "timeout_seconds": 2}
    }
  ]
}

```

**必填与可选**

* 顶层必填：`schema_version`、`snapshot_id`、`sequence`、`collected_at`、`agent`、`host`、`collection`、`cpu`、`memory`、`filesystems`、`disks`、`network`、`load`、`processes`、`uptime`、`health`、`events`。

* `agent.agent_id` 持久化，跨重启不变；`instance_id` 每次进程启动重新生成；`sequence` 持久化单调递增。

* `host` 中无法获取的字段用 `null`；`cpu_model`、`virtualization` 可为 `null`。

* 每个分区对象均必填 `status`（`ok`/`partial`/`failed`/`unsupported`/`timeout`）、`sampled_at`、`age_seconds`（当前快照时间减采样时间，保留 3 位小数）与 `errors`（无错为 `[]`）。

* `events` 只放本快照周期内发生的事件，可为空数组；事件至少投递一次，服务端按 `event_id` 去重。

### 6.3 上报批次

推送体是批次对象；启用压缩时整体 gzip，解压后为 UTF-8 JSON。

```
{
  "schema_version": "1.0",
  "batch_id": "3b0c1f2e-7d8a-4b6c-9e0f-1a2b3c4d5e6f",
  "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
  "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
  "created_at": "2026-11-21T01:21:02.000Z",
  "sequence_first": 123451,
  "sequence_last": 123456,
  "snapshot_count": 6,
  "snapshots": []
}

```

| 字段 | 规则 | 
| ----- | ----- | 
| `batch_id` | UUID v4；重试时必须保持不变，服务端据此幂等 | 
| `sequence_first/last` | 批次内快照序号范围；必须连续，缺号说明 WAL 淘汰，需在 `gaps` 中列出 | 
| `snapshot_count` | 1–100；与 `snapshots` 长度一致 | 
| `gaps` | 可选，缺号区间数组，如 `[{"from":123440,"to":123442}]` | 
| `snapshots` | 快照数组，按 `sequence` 升序；单批未压缩 ≤1 MiB | 

* 批次上限：默认 6 个快照、1 MiB 未压缩、30 秒组批窗口（常规 10s 采集下每 30s 聚批 3 个快照，补发或高频采集时最高 6 个/批）；任一条件触发即发送；最多 100 个快照。

* 压缩：gzip 级别 6；`Content-Encoding: gzip`；压缩后超过 4 MiB 时拆分批次。

* 批次不跨 `instance_id`；进程重启后新批次使用新 `instance_id`，旧 WAL 中同实例数据仍可继续发送，但不得与新实例混批。

### 6.4 HTTP 契约

| 项 | 约定 | 
| ----- | ----- | 
| 方法与路径 | `POST /api/v1/ingest` | 
| 认证 | `Authorization: Bearer <token>` 或 mTLS；令牌不得放 query | 
| 请求头 | `Content-Type: application/json`、`Content-Encoding: gzip`（可选）、`X-Sprobe-Agent-Id`、`X-Sprobe-Instance-Id`、`X-Sprobe-Batch-Id`、`X-Sprobe-Schema-Version` | 
| 响应 | `application/json`；2xx 成功；4xx 不重试（408/429 除外）；5xx 重试 | 
| 超时 | 连接 3s、请求 10s；响应体 ≤64 KiB | 
| 幂等 | `(agent_id, instance_id, sequence)` 唯一；同一 `batch_id` 重放返回首次结果 | 
| 限流 | 429 带 `Retry-After`；agent 遵守，最大等待 60s | 

成功响应：

```json
{
  "schema_version": "1.0",
  "status": "accepted",
  "batch_id": "3b0c1f2e-7d8a-4b6c-9e0f-1a2b3c4d5e6f",
  "accepted": 6,
  "duplicates": 0,
  "rejected": [],
  "server_time": "2026-11-21T01:21:02.345Z"
}
```

部分成功：

```json
{
  "schema_version": "1.0",
  "status": "partial",
  "batch_id": "3b0c1f2e-7d8a-4b6c-9e0f-1a2b3c4d5e6f",
  "accepted": 5,
  "duplicates": 0,
  "rejected": [
    {"sequence": 123456, "code": "validation_failed", "message": "cpu.total.usage_ratio must be <= 1", "retryable": false}
  ],
  "server_time": "2026-11-21T01:21:02.345Z"
}
```

* `status=partial` 时，被拒绝的快照不重试（除非 `code` 标注 `retryable=true`）；agent 记录并继续发送后续快照，避免单条坏数据堵住队列。

* 2xx 但 `rejected` 非空，仍视为本次请求成功；不重试整批。

错误响应（与 6.12.3 及前端只读接口统一使用嵌套 `error` 结构）：

```json
{
  "error": {
    "code": "unauthorized",
    "message": "invalid or expired token",
    "retryable": false,
    "request_id": "req_01J8Z3K4",
    "server_time": "2026-11-21T01:21:02.345Z"
  }
}
```

错误码（全系统唯一枚举，与 6.12.3 一致）：

| HTTP | code | 含义 | agent 行为 | 
| ----- | ----- | ----- | ----- | 
| 400 | `invalid_json` / `invalid_gzip` | 请求体不可解析或解压失败 | 不重试，记录并丢弃该批 | 
| 400 | `unsupported_schema` | 主版本不兼容 | 不重试，健康 critical，停止推送 | 
| 401 | `unauthorized` | 令牌缺失/无效 | 不重试，首次即锁存 `push_auth_failed` critical，停止推送 | 
| 403 | `forbidden` | 令牌无权限/租户不匹配 | 同 401，不重试 | 
| 404 | `not_found` | 路径错误 | 不重试，配置错误，健康 critical | 
| 408 | `request_timeout` | 服务端处理超时 | 退避重试 | 
| 409 | `conflict` | 同序号不同内容（非幂等重复） | 不重试，记录数据冲突 | 
| 413 | `payload_too_large` | 压缩后或解压后超限 | 减半拆分后重试，最多 3 次 | 
| 422 | `validation_failed` | 字段/类型/范围/语义校验失败 | 不重试，记录并丢弃该批 | 
| 429 | `rate_limited` | 限流 | 按 `Retry-After` 等待（上限 60s）后重试 | 
| 500 | `internal` | 服务端内部错误 | 退避重试 | 
| 502/503/504 | `upstream_unavailable` | 网关/上游不可用 | 退避重试 | 

**独立心跳契约（`POST /api/v1/heartbeat`）**

当无新快照可发或常规推送队列处于退避/熔断期时，agent 按 `output.push.heartbeat_interval_seconds`（默认 30s）发送轻量心跳，复用相同认证头与 TLS 配置，不经过 WAL：

```
{
  "schema_version": "1.0",
  "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
  "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
  "sent_at": "2026-11-21T01:21:30.000Z",
  "uptime_seconds": 86430.12,
  "ready": true,
  "warmup": false,
  "health_status": "degraded",
  "health_status_code": 1,
  "last_sequence": 123456,
  "wal_snapshots": 12
}

```

服务端返回 `200 OK`（`{"status": "ok", "server_time": "..."}`）；心跳失败不阻塞采集与常规批次推送。

### 6.5 错误对象（agent 本地）

用于日志、`/status`、诊断事件，不直接上报给服务端。

| 字段 | 说明 | 
| ----- | ----- | 
| `code` | 稳定机器码，如 `collector_timeout`、`permission_denied`、`counter_reset` | 
| `message` | 人类可读，英文或中文均可，不含密钥/完整命令行 | 
| `severity` | `info`、`warning`、`error` | 
| `scope` | 采集器/模块名 | 
| `occurred_at` | UTC 时间 | 
| `retryable` | 是否可通过重试恢复 | 
| `details` | 有界键值对；禁止放原始文件内容、令牌、用户数据 | 

### 6.6 事件对象

* `event_id`：UUID v4，agent 生成；服务端按它去重。

* `type`：稳定枚举，如 `host_reboot`、`counter_reset`、`collector_timeout`、`health_changed`、`push_auth_failed`、`wal_evicted`、`config_reloaded`。

* `severity`：`info`、`warning`、`critical`。

* `occurred_at`：UTC。

* `details`：类型相关，字段有界；不得包含密钥、完整命令行、文件内容。

* 事件不保证顺序；服务端按 `occurred_at` 排序展示，按 `event_id` 去重。

* 事件与快照同批发送；事件量默认每批 ≤100 条，超出丢弃低优先级 `info` 并计数。

### 6.7 健康对象

| 字段 | 类型 | 说明 | 
| ----- | ----- | ----- | 
| `status` | string | `ok/degraded/critical/unknown` | 
| `status_code` | integer | 0/1/2/3，与 status 一一对应 | 
| `since` | string | 当前状态开始时间 | 
| `recovered_at` | string/null | 最近一次从异常恢复为 `ok` 的 UTC 时间；未发生或当前异常时为 `null` | 
| `checks` | array | 检查结果数组；无检查时 `[]` | 
| `checks[].id` | string | 检查 ID，稳定 | 
| `checks[].status` | string | `ok/degraded/critical/unknown` | 
| `checks[].severity` | string | `info/warning/critical` | 
| `checks[].message` | string | 人类可读，≤1024 字节 | 
| `checks[].observed` | number/null | 实际值 | 
| `checks[].threshold` | number/null | 阈值 | 
| `checks[].since` | string | 该检查进入当前状态的时间 | 

* `status` 与 `status_code` 必须一致；服务端以 `status_code` 做告警判断，以 `status` 做展示。

* `checks` 只包含启用的检查；禁用的检查不出现。

* 检查的 `message` 不得包含主机名以外的敏感信息；主机名可由服务端补充。

### 6.8 能力声明

`collection.capabilities` 的键是稳定能力名，值为布尔；`collection.unsupported_reasons` 为同一键到原因码的映射，仅当值为 `false` 时出现。

| 能力键 | 含义 | 常见 false 原因 | 
| ----- | ----- | ----- | 
| `cpu_modes` | 是否支持 CPU 模式拆分 | 平台不提供 | 
| `cpu_frequency` | 是否支持频率 | 无 cpufreq/权限不足 | 
| `memory_available` | 是否有可靠 available | 内核过旧 | 
| `swap` | 是否有交换区 | 未配置 swap | 
| `filesystem_inodes` | 是否有 inode | Windows/macOS | 
| `disk_io_counters` | 是否有累计磁盘计数器 | Windows 非管理员 | 
| `network_speed` | 是否有链路速率 | 虚拟接口/驱动不支持 | 
| `process_top` | 是否支持 Top-N | 权限/平台限制 | 
| `process_io` | 是否支持进程 I/O | 权限不足/默认关闭 | 
| `cgroup_metrics` | 是否支持容器配额 | 无 cgroup v2 | 

原因码：`unsupported_platform`、`permission_denied`、`not_configured`、`kernel_too_old`、`temporarily_unavailable`。

* 能力声明在每次快照中重复，便于服务端按批次判断，不依赖首次注册。

* 能力变化（如从非管理员变为管理员）记录 `capabilities_changed` 事件，值以本次为准。

### 6.9 身份、序号与幂等

* `agent_id`：安装时生成 UUID v4，写入状态文件；重装/克隆镜像时必须重新生成，避免多机同 ID。状态文件丢失时，若无法恢复，生成新 ID 并记录 `agent_id_regenerated`。

* `instance_id`：进程启动时生成 UUID v4；用于区分同一 agent 的不同运行实例，避免重启后序号冲突。

* 每个快照记录**生成时**的 `instance_id`；重启后旧 WAL 记录保留原 `instance_id`，不得改写为新实例。批次内 `instance_id` 必须一致，跨实例必须拆批。

* `agent_id` 状态文件丢失且无法恢复时，生成新 ID；旧 WAL 不得用新 ID 重放。旧 WAL 应隔离到 `wal/orphan-<old_agent_id>/` 或按策略丢弃，并记录 `agent_id_regenerated` 与 `wal_orphaned` 事件，由运维确认后再清理。

* `sequence`：每个快照递增 1，持久化到状态文件；重启后从持久化值继续。若状态文件损坏，从 0 重新开始但更换 `instance_id`。

* 幂等键：`(agent_id, instance_id, sequence)`；服务端唯一约束。同一键重复提交返回首次结果，不重复入库。

* `batch_id` 用于请求级幂等：同一批次重试必须复用同一 `batch_id`；重新组批生成新 ID。

* 时钟不可靠时，`sequence` 仍是主排序依据；`collected_at` 只用于展示与滞后判断。

### 6.10 Schema 演进

* **minor 升级**：只新增可选字段/能力键/事件类型；服务端必须忽略未知字段；agent 不得删除已有字段或改变语义。

* **major 升级**：删除/重命名字段、改变单位/类型/空值语义；必须提供迁移说明与服务端双版本接收窗口（至少 2 个 minor 周期）。

* 服务端按 `schema_version` 主版本路由；主版本不兼容返回 400 `unsupported_schema`，agent 停止推送并进入 critical。

* 字段废弃：先标记 deprecated，保留至少 2 个 minor 版本，再在 major 删除；文档附录记录变更。

* 任何口径变化（如 `cpu_usage_ratio` 是否含 iowait）都视为 major，不得在 minor 中静默修改。

### 6.11 大小、性能与安全限制

| 限制 | 默认值 | 超限行为 | 
| ----- | ----- | ----- | 
| 单快照未压缩 | 256 KiB | 截断 Top-N/事件，标 `truncated=true` | 
| 单批次未压缩 | 1 MiB | 拆分批次 | 
| 单批次快照数 | 100 | 拆分 | 
| 单批次事件数 | 100 | 丢弃 `info` 事件并计数 | 
| 压缩后请求体 | 4 MiB | 拆分 | 
| 字符串字段 | 256 字节 | 截断并标 `truncated` | 
| 标签对 | 32 | 截断并记录 | 
| `events` 保留 | 最近 1000 条或 1 小时 | 淘汰最旧 | 

* 服务端必须限制请求体大小、解压后大小（防 zip bomb）、JSON 深度与数组长度；解压后超过 8 MiB 直接 413。

* JSON 解析必须拒绝重复键、非法 UTF-8、超深嵌套；错误信息不得回显原始请求体。

### 6.12 接口速查与错误码

**6.12.1 端点矩阵**

| 方法/路径 | 用途 | 认证 | 请求要点 | 成功 | 主要失败 |
| ----- | ----- | ----- | ----- | ----- | ----- |
| `POST /api/v1/ingest` | 批量上报快照 | Bearer 或 mTLS | `application/json`，可 `gzip`；批次含 `batch_id`、`snapshots[]` | `200` + `accepted/duplicates/rejected` | `400/401/403/409/413/422/429/500` |
| `POST /api/v1/heartbeat` | 独立心跳 | Bearer 或 mTLS | `agent_id`、`instance_id`、`sent_at`、`health_status`、`health_status_code`、`wal_snapshots` | `200` | `401/403/429/5xx` |
| `GET /metrics` | Prometheus 文本 | 默认仅回环（`allow_remote=true` 时需前置反代 + TLS） | 无 | `200 text/plain` | `405`（非 GET/HEAD） |
| `GET /status` | 本地摘要 | 仅回环 | 无 | `200 application/json` | `403`（跨回环转发拒绝） |
| `GET /healthz` | 存活探针 | 仅回环 | 无 | `200` | `503` |
| `GET /readyz` | 就绪探针 | 仅回环 | 无 | `200` | `503` |

* **不提供** `/ui`、`/api/v1/query`、`/api/v1/snapshot/latest` 等本地查询/展示接口；Console 只读 API 由独立前端项目定义，见 `sprobe_v1_2.md`。

* 所有写请求必须带 `Content-Type: application/json`；压缩请求带 `Content-Encoding: gzip`。令牌只放 `Authorization` 头，禁止放 query。

**6.12.2 成功响应**

```json
{
  "schema_version": "1.0",
  "status": "accepted",
  "batch_id": "0f4f4d9e-7a0e-4f6e-9f0e-2a6d4a0b7c11",
  "accepted": 3,
  "duplicates": 0,
  "rejected": [],
  "server_time": "2026-11-21T02:00:00Z"
}
```

* `accepted` 表示首次接收；`duplicates` 表示幂等命中；`rejected[]` 不为空时，客户端按 6.4 决定是否重试/丢弃。

**6.12.3 统一错误体**

```json
{
  "error": {
    "code": "validation_failed",
    "message": "snapshot 2: cpu.total.usage_ratio out of range",
    "retryable": false,
    "request_id": "req_01J8Z3K4",
    "server_time": "2026-11-21T02:00:00Z"
  }
}
```

| HTTP | `code` | 含义 | 客户端行为 |
| ----- | ----- | ----- | ----- |
| 400 | `invalid_json` / `invalid_gzip` | 请求体不可解析 | 不重试，记录并丢弃该批 |
| 400 | `unsupported_schema` | 主版本不兼容 | 停止推送，critical，升级 agent |
| 401 | `unauthorized` | 令牌缺失/无效 | 停止推送，`push_auth_failed` 锁存 critical |
| 403 | `forbidden` | 令牌无权/租户不符 | 同上，不重试 |
| 409 | `conflict` | 数据冲突（非幂等重复） | 不重试，记录 `rejected`，人工排查 |
| 413 | `payload_too_large` | 压缩后/解压后超限 | 拆分批次，最多 3 次 |
| 422 | `validation_failed` | 字段/范围/语义错误 | 不重试，丢弃并记录 |
| 429 | `rate_limited` | 限流 | 按 `Retry-After` 退避 |
| 500 | `internal` | 服务端错误 | 指数退避重试 |

* 幂等重复统一返回 `200` 并计入 `duplicates`；`409` 只表示同一幂等键对应不同内容等数据冲突，不用于普通重复。

* 错误响应不得回显令牌、完整请求体、主机敏感路径；`request_id` 用于服务端日志关联。

**6.12.4 字段路径约定**

* JSON 字段路径以 6.2 为准；前端/服务端不得使用简写。典型易错路径：

| 语义 | 正确路径 | 常见错误写法 |
| ----- | ----- | ----- |
| CPU 总使用率/等 | `cpu.total.usage_ratio`、`cpu.total.iowait_ratio`、`cpu.total.steal_ratio` | `cpu.iowait_ratio` |
| 逻辑核数 | `cpu.logical_count` | `cpu.total.logical_count` |
| 网络错包/丢包 | `network.interfaces[].receive_errors_total`、`transmit_errors_total`、`receive_dropped_total`、`transmit_dropped_total` | `errors`、`dropped` |
| 推送鉴权失败 | `health.checks[]` 中 `id == "push_auth_failed"` | 顶层 `push_auth_failed` 布尔字段 |
| 文件系统分区状态 | `filesystems.status`、`filesystems.errors`（section 级） | `filesystems.items[].status` |
| 进程 PID | `processes.top[].pid`（仅认证视图） | `processes.pid` |

## 7. 配置与部署

### 7.1 配置原则

* **单一配置文件**：默认 `/etc/sprobe/sprobe.toml`（Windows 为 `C:\ProgramData\Sprobe\config.toml`），TOML 格式；不使用多文件 include，避免配置漂移。

* **优先级**：命令行参数 > 环境变量 > 配置文件 > 内置默认值。高优先级覆盖低优先级，不做合并（除 `labels` 按 key 覆盖）。

* **启动即校验**：未知键、类型错误、越界值、互斥组合一律启动失败，并输出具体字段路径；不做“猜意图”修正。

* **可热加载**：仅限明确列出的字段（见 7.8）；其余字段修改后必须重启，热加载时忽略并记录 warning。

* **密钥不落配置**：令牌、私钥只引用文件路径；配置文件中出现疑似密钥值时启动失败，防止误提交。

* **单位写进键名**：`*_seconds`、`*_bytes`、`*_ratio`；禁止裸数字含义不明。

### 7.2 配置结构

以下是完整示例，字段即为 1.0 的配置契约；未列出的键在 1.0 中不存在，出现即报错。

```
# /etc/sprobe/sprobe.toml
schema_version = "1.0"

[agent]
data_dir = "/var/lib/sprobe"
state_file = "state.json"
hostname = ""                      # 空 = 自动取系统主机名
proc_prefix = ""                   # 容器内挂载宿主 /proc 时设为 /host/proc
sys_prefix = ""                    # 容器内挂载宿主 /sys 时设为 /host/sys
labels = { env = "prod", role = "web" }

[collect]
fast_interval_seconds = 10
medium_interval_seconds = 30
slow_interval_seconds = 300
collector_timeout_seconds = 2.0
snapshot_timeout_seconds = 5.0
max_concurrent_collectors = 4
jitter_ratio = 0.2

[cpu]
enabled = true
per_core = true
frequency = true

[memory]
enabled = true
include_swap = true

[filesystems]
enabled = true
root_prefix = ""                   # 容器内挂载宿主根时设为 /host/root
# 物理机默认排除 overlay；容器场景按需将 "overlay" 加入 include_fstypes
include_fstypes = ["ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "ntfs", "apfs", "vfat", "exfat", "nfs", "nfs4", "cifs"]
exclude_fstypes = ["proc", "sysfs", "devtmpfs", "devpts", "cgroup", "cgroup2", "securityfs", "debugfs", "tracefs", "pstore", "bpf", "configfs", "fusectl", "mqueue", "hugetlbfs", "autofs", "binfmt_misc", "rpc_pipefs", "nsfs", "squashfs"]
include_mountpoints = []
exclude_mountpoints = ["/snap", "/var/lib/docker/overlay2"]
network_timeout_seconds = 2.0

[disks]
enabled = true
include_partitions = false
include_virtual = true

[network]
enabled = true
include_loopback = false
include_slaves = false
include_virtual = true
aggregate_interfaces = []          # 空 = 不输出合计

[processes]
enabled = true
top_n = 20
include_exe = false
watch = ["nginx", "sshd"]

[load]
enabled = true
windows_synthetic = true

[health]
enabled = true
disabled_checks = []               # 可按检查 ID 单独禁用，如 ["disk_io_latency"]

[health.thresholds]
cpu_usage_ratio = 0.90
cpu_iowait_ratio = 0.20
memory_usage_warning = 0.90
memory_usage_critical = 0.95
disk_usage_warning = 0.90
disk_usage_critical = 0.95
inode_usage_ratio = 0.90
disk_latency_seconds = 0.100
network_error_rate = 1.0
load_per_core = 2.0
agent_backlog_snapshots = 1000
agent_backlog_age_seconds = 1800

[metrics]
namespace = "sprobe"
expose_quality = false

[output.push]
enabled = true
endpoint = "https://monitor.example.com/api/v1/ingest"
heartbeat_endpoint = "https://monitor.example.com/api/v1/heartbeat"
token_file = "/etc/sprobe/token"
interval_seconds = 30
heartbeat_interval_seconds = 30
batch_max_snapshots = 6
batch_max_bytes = 1048576
allow_out_of_order = false
max_in_flight_batches = 1          # 仅 allow_out_of_order=true 时可设 1–3
request_timeout_seconds = 10.0
connect_timeout_seconds = 3.0
gzip = true
tls_ca_file = ""
tls_insecure_skip_verify = false
allow_insecure = false             # 仅与 tls_insecure_skip_verify=true 同时为 true 时允许明文 http://
client_cert_file = ""
client_key_file = ""

[output.prometheus]
enabled = true
listen = "127.0.0.1:9101"
path = "/metrics"
allow_remote = false

[storage.wal]
dir = "/var/lib/sprobe/wal"
max_bytes = 100663296             # 96 MiB，覆盖 1 小时断网最坏情况
max_age_seconds = 3600
fsync = "batch"                    # always | batch | never

[security]
allow_loose_token_permissions = false

[log]
level = "info"                     # error | warn | info | debug | trace
format = "json"                    # json | text
file = ""                          # 空 = stderr；systemd 下建议 stderr
max_size_bytes = 10485760
max_files = 5

[limits]
max_events_per_snapshot = 100
max_string_bytes = 256

```

### 7.3 路径与目录

**Linux（FHS）**

| 路径 | 用途 | 权限 | 
| ----- | ----- | ----- | 
| `/etc/sprobe/sprobe.toml` | 主配置 | `0640 root:sprobe` | 
| `/etc/sprobe/token` | 推送令牌（可选） | `0600 sprobe:sprobe` | 
| `/var/lib/sprobe/` | 状态、WAL | `0750 sprobe:sprobe` | 
| `/var/lib/sprobe/wal/` | WAL 分段 | `0750 sprobe:sprobe` | 
| `/var/log/sprobe/` | 文件日志（可选） | `0750 sprobe:sprobe` | 
| `/usr/local/bin/sprobe` | 二进制 | `0755 root:root` | 
| `/etc/systemd/system/sprobe.service` | 服务单元 | `0644 root:root` | 

**Windows**

| 路径 | 用途 | 
| ----- | ----- | 
| `C:\ProgramData\Sprobe\config.toml` | 主配置 | 
| `C:\ProgramData\Sprobe\token` | 令牌（ACL 仅服务账户） | 
| `C:\ProgramData\Sprobe\data\` | 状态、WAL | 
| `C:\ProgramData\Sprobe\logs\` | 日志 | 
| `C:\Program Files\Sprobe\sprobe.exe` | 二进制 | 

* Windows 使用服务账户 ACL 限制目录；`ProgramData` 默认对 Users 可读，安装器必须收紧到 `SYSTEM` + `Administrators` + 服务账户。

* 安装/卸载不删除 `data` 目录；卸载时保留配置，除非显式 `--purge`。

* 容器内路径固定为 `/etc/sprobe/sprobe.toml`、`/var/lib/sprobe`、`/var/log/sprobe`，通过 volume 覆盖。

### 7.4 环境变量与命令行

环境变量只覆盖少量启动参数，避免配置分散：

| 环境变量 | 覆盖 | 
| ----- | ----- | 
| `SPROBE_CONFIG` | 配置文件路径 | 
| `SPROBE_LOG_LEVEL` | 日志级别 | 
| `SPROBE_PUSH_ENDPOINT` | 推送地址 | 
| `SPROBE_PUSH_TOKEN_FILE` | 令牌文件路径 | 
| `SPROBE_LABELS` | 追加标签，格式 `k1=v1,k2=v2` | 

命令行：

| 参数 | 说明 | 
| ----- | ----- | 
| `--config <path>` | 指定配置文件路径 | 
| `--check-config` | 校验配置后退出；成功 0，失败 2 | 
| `--dry-run` | 校验配置并打印生效配置（密钥脱敏）后退出 | 
| `--once` | 采集并输出单个 JSON 快照到 stdout 后退出；默认等待 `--sample-window`（1s）以输出完整差分速率 | 
| `--sample-window <sec>` | 与 `--once` 联用，指定两次采样间隔（默认 `1`，范围 `1–10`） | 
| `--raw` | 与 `--once` 联用，仅执行单次瞬时基线采集（不等待，差分字段为 `null`） | 
| `--no-state` | 与 `--once` 联用，禁止读写 `state.json` 序号 | 
| `--diagnose` | 输出脱敏配置摘要、能力矩阵、最近错误与 WAL 统计（整合原 `doctor` 能力） | 
| `--regenerate-id` | 重新生成持久化 `agent_id`（用于克隆镜像/重装后去重） | 
| `--install-service` / `--uninstall-service` | Windows 专用：注册或卸载 Windows 服务（配合 `--purge` 清理数据目录） | 
| `--version` | 输出版本、commit、构建时间、目标 triple | 
| `--log-level <level>` | 临时覆盖日志级别 | 

* 未知参数退出码 2；`--once` 不写 WAL、不推送，默认不修改状态文件外的数据（验收时推荐搭配 `--no-state`）。

* 环境变量中的令牌不推荐；如使用，进程启动后立即从环境删除，且不写入日志。

### 7.5 Linux 部署（systemd）

**安装步骤**

1. 创建系统用户：`useradd --system --no-create-home --shell /usr/sbin/nologin sprobe`。

2. 创建目录并按 7.3 设置权限。

3. 放置二进制、配置、令牌。

4. `systemctl daemon-reload && systemctl enable --now sprobe`。

5. 验证：`systemctl status sprobe`、`curl -s http://127.0.0.1:9101/metrics | head`、`journalctl -u sprobe -n 50`。

**服务单元示例**

```
[Unit]
Description=Sprobe server agent
Documentation=https://example.com/sprobe/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/local/bin/sprobe --config /etc/sprobe/sprobe.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s
TimeoutStopSec=25s
User=sprobe
Group=sprobe
UMask=0027
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=false
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
SystemCallArchitectures=native
ReadWritePaths=/var/lib/sprobe /var/log/sprobe
# 需要读取其他用户进程的 /proc 信息时，才添加以下能力；默认不需要
# AmbientCapabilities=CAP_SYS_PTRACE
# CapabilityBoundingSet=CAP_SYS_PTRACE

[Install]
WantedBy=multi-user.target

```

* `Type=notify` 要求 agent 在 `ready` 后发送 `READY=1`；退出前发送 `STOPPING=1`。若实现暂不支持 notify，改用 `Type=exec`，不得用 `Type=simple` 掩盖启动失败。

* `ProtectSystem=strict` 下只有 `ReadWritePaths` 可写；WAL/状态目录必须包含在内，否则启动失败并给出明确错误。

* `RestrictAddressFamilies` 必须包含 `AF_NETLINK`，否则网络接口枚举可能失败。

* `PrivateDevices=true` 会隐藏 `/dev` 下的磁盘设备；需要磁盘 I/O 时保持 `false` 或显式放行。

* 需要跨用户进程详情时，优先把 agent 加入受限组并调整 `/proc` 权限；`CAP_SYS_PTRACE` 是最后手段，必须在文档中说明风险。

**权限与能力**

| 需求 | 最低权限 | 
| ----- | ----- | 
| CPU/内存/负载/uptime/文件系统容量 | 普通用户 | 
| 网络接口统计 | 普通用户（`/sys/class/net` 可读） | 
| 全量进程列表（非 hidepid） | 普通用户 | 
| 其他用户进程的 I/O、部分受保护进程 | root 或 `CAP_SYS_PTRACE` | 
| 磁盘 I/O 累计计数（Linux） | 普通用户读 `/proc/diskstats` | 
| 温度/频率（部分平台） | 普通用户或 root，视驱动 | 
| 绑定 <1024 端口 | 不需要（默认 9101） | 

### 7.6 Windows 部署

**安装方式**

* MSI 安装包：安装服务 `SprobeAgent`、写配置模板、创建数据目录、注册防火墙出站规则（如需要）。

* 绿色 zip：解压后 `sprobe.exe --install-service` 注册服务；`--uninstall-service` 卸载。

* 服务启动类型 `Automatic (Delayed Start)`，避免开机时与系统服务争抢。

* 服务失败恢复：第一次/第二次/后续均重启，间隔 5s/10s/30s，重置计数 1 天。

**服务账户**

* 默认 `LocalSystem`：可获取完整磁盘性能计数器与进程信息；权限最大，需配合目录 ACL。

* 可选 gMSA/低权限账户：磁盘 I/O 累计计数器、部分进程信息会降级；`capabilities.disk_io_counters=false`，agent 仍可运行。

* 不推荐使用交互式用户账户；服务运行在 Session 0，不能依赖桌面会话。

**网络与防火墙**

* 推送模式只需出站 TCP 443（或自定义端口）；不需要入站。

* 拉取模式默认只监听 `127.0.0.1:9101`；`allow_remote=true` 时必须配置防火墙白名单，并建议启用 mTLS。

* 代理：遵循系统 WinHTTP 代理或配置 `HTTPS_PROXY`；代理认证凭据不写入配置文件。

**升级/卸载**

* MSI 主版本升级采用“先装新服务、切换、再删旧文件”；升级期间数据目录保留。

* 卸载默认保留 `C:\ProgramData\Sprobe`；`--purge` 才删除配置、状态与 WAL。

* 降级仅允许同一 major 内；跨 major 降级不支持，需备份并重建状态。

### 7.7 容器与 Kubernetes

**容器运行要求**

* 只读根文件系统；`/etc/sprobe`、`/var/lib/sprobe` 用 volume 挂载。

* 采集宿主指标时：

  * `--pid=host`（或挂载宿主 `/proc` 到 `/host/proc` 并配置 `agent.proc_prefix="/host/proc"`）；

  * 挂载宿主 `/sys` 只读（按需配置 `agent.sys_prefix="/host/sys"`）；

  * 挂载宿主根到 `/host/root`，并设置 `filesystems.root_prefix="/host/root"`；

  * 网络指标默认是容器命名空间口径；需要宿主网卡时使用 `--network=host`，或只采集容器自身。

* 非特权运行：UID/GID 65532，`readOnlyRootFilesystem: true`，`allowPrivilegeEscalation: false`，`capabilities.drop: ["ALL"]`。

* 需要磁盘 I/O 累计计数时，Linux 读 `/proc/diskstats` 即可，不需要 `CAP_SYS_ADMIN`；不要用 `privileged: true` 作为默认方案。

**Kubernetes DaemonSet 要点**

* `hostPID: true`、`hostNetwork: true`（按需）；`tolerations` 覆盖所有节点；`priorityClassName: system-node-critical`。

* 资源：requests `50m CPU / 64Mi`，limits `200m CPU / 256Mi`；采集大机器（>256 核）时上调。

* 挂载：`/proc`、`/sys`、`/etc/os-release`、`/var/lib/sprobe`（hostPath）、配置 ConfigMap。

* 每个节点只能运行一个 agent；用 `DAEMON_SET` 的节点亲和避免重复；agent 自身也用文件锁防双实例。

* 滚动更新：`maxUnavailable: 10%`，先启动新 Pod 再停旧 Pod 会导致短暂双实例；文件锁在 hostPath 上生效，新实例启动失败后重试，直到旧实例退出。

### 7.8 配置校验与热加载

**启动校验（失败即退出，退出码 2）**

* `schema_version` 必须为 `"1.0"`；未知顶层键报错。

* 间隔：`fast_interval_seconds` 1–30；`medium` 是 `fast` 的整数倍；`slow ≥ 60`；`push.interval ≥ 5`。

* 超时：`collector_timeout_seconds` 0.5–10；`snapshot_timeout_seconds` 必须大于单个采集器超时。

* `batch_max_bytes` 64 KiB–4 MiB；`batch_max_snapshots` 1–100。

* `top_n` 1–100；`max_concurrent_collectors` 1–16。

* 文件系统 include/exclude 不能同时包含同一 fstype；挂载点必须是绝对路径。

* 推送启用时 `endpoint` 必须是 `https://`（允许 `http://` 仅当 `tls_insecure_skip_verify=true` 且显式 `allow_insecure=true`，默认禁止）。

* 令牌文件必须存在、权限不宽于 `0600`（Windows ACL 仅服务账户）；否则启动失败。

* 监听地址必须是回环，除非显式设置 `allow_remote=true`（1.0 拉取端不内置 TLS，开启后启动记录 `warn`，必须前置反向代理 + TLS/白名单）。

**热加载（SIGHUP / Windows 服务控制）**

| 字段 | 可热加载 | 说明 | 
| ----- | ----- | ----- | 
| `log.level`、`log.format` | 是 | 立即生效 | 
| `health.thresholds.*` | 是 | 下一轮健康计算生效 | 
| `processes.watch`、`top_n` | 是 | 下一轮采集生效 | 
| `collect.*_interval_seconds`、`jitter_ratio` | 是 | 重新排期，不打断在途采集 | 
| `network.aggregate_interfaces` | 是 | 下一轮采集生效 | 
| `output.push.endpoint`、`batch_*`、`gzip` | 是 | 下一批生效；失败保留旧值 | 
| `output.push.token_file`、TLS 证书 | 是 | 重新读取；失败保留旧证书并告警 | 
| `output.prometheus.listen` | 否 | 需重启 | 
| `agent.data_dir`、`state_file` | 否 | 需重启 | 
| `filesystems.include_*`、`exclude_*` | 是 | 下一轮 slow 采集生效 | 
| `storage.wal.max_bytes`、`max_age_seconds` | 是 | 下一轮清理生效 | 
| `agent.hostname`、`agent.labels` | 是 | 影响快照元数据，不改变 `agent_id` | 

* 热加载流程：读取 → 解析 → 全量校验 → 构建新配置 → 原子替换 → 记录 `config_reloaded` 事件。任一阶段失败，保留旧配置，记录 error 与失败字段，不退出进程。

* 不可热加载字段被修改时，记录 `config_restart_required` warning，继续使用旧值；不得静默忽略。

* 配置文件监听使用 500ms 防抖；同一文件短时间内多次修改只加载最后一次。容器/Kubernetes ConfigMap 挂载通过符号链接原子切换更新，文件监听必须监控父目录或统一通过 `SIGHUP` 触发热加载。

* 热加载不得改变 `agent_id`；尝试修改身份文件路径的配置直接拒绝。

### 7.9 升级、回滚与状态兼容

* 版本号语义：`major.minor.patch`；配置 `schema_version` 与快照 `schema_version` 独立演进。

* 升级前：备份配置、状态文件、WAL；停止服务；替换二进制；启动并验证 `ready`。

* 升级后：检查 `--version`、`/metrics`、最近一批推送的 `accepted`；观察 10 分钟无 `collector_panic`/`wal_evicted`。

* 回滚：保留上一版本二进制与配置；状态文件含 `state_schema_version`，minor 升级必须向前兼容（新版本可读旧状态）；major 升级需提供迁移工具，回滚需恢复备份。

* 不允许“新版本写状态、旧版本直接读”的跨 major 回滚；文档必须明确该限制。

* 发布包保留最近 3 个版本；EOL 版本提前 90 天公告。

### 7.10 单实例与身份

* 启动时对 `data_dir/agent.lock` 加独占锁；已锁定时输出 `another instance is running` 并以退出码 3 退出，不自动杀进程。

* 锁文件内容包含 PID、启动时间、版本，便于排查；进程异常退出后锁自动释放（OS 文件锁语义）。

* 容器/Windows 服务同样使用该锁；hostPath 挂载时跨 Pod 生效。

* `agent_id` 写入 `state.json`；克隆虚拟机/容器镜像后必须删除状态文件或运行 `--regenerate-id`，否则多机同 ID 会被服务端视为同一 agent。

### 7.11 快速部署与检查清单

**7.11.1 Linux（systemd，5 分钟路径）**

1. 创建系统用户与目录：`sprobe` 用户、`/etc/sprobe`、`/var/lib/sprobe`、`/var/log/sprobe`；WAL 目录与状态文件分离。
2. 安装二进制到 `/usr/local/bin/sprobe`，写入 `/etc/sprobe/sprobe.toml`，令牌文件权限 `0600`，配置 `0640`。
3. 执行 `sprobe --check-config -c /etc/sprobe/sprobe.toml`；失败按退出码 2 处理，不启动服务。
4. 安装 systemd unit，`systemctl daemon-reload && systemctl enable --now sprobe`。
5. 验证：`systemctl status sprobe`、`curl --unix-socket` 不可用时用回环 `curl 127.0.0.1:9101/healthz`、`/readyz`、`/metrics`。
6. 观察首轮 `warmup` 结束（第 0 轮基线 + 第 1 个常规 fast 周期，约 10s），确认 `/status` 的 `ready=true`、`warmup=false` 且 `last_push_at` 成功。

**7.11.2 Windows（服务）**

1. 解压到 `C:\Program Files\Sprobe`，配置写到 `C:\ProgramData\Sprobe\config.toml`。
2. `sprobe.exe --check-config -c ...` 通过后执行 `sprobe.exe --install-service`。
3. 服务账户默认 LocalSystem；低权限方案需授予性能计数器读取与事件日志权限，缺失指标按能力声明降级。
4. 验证 `Get-Service SprobeAgent`、`curl http://127.0.0.1:9101/healthz`、事件查看器与日志文件。

**7.11.3 容器/Kubernetes**

* 只挂载配置、WAL 卷与只读的宿主 `/proc`、`/sys`；不要 `privileged`，不要挂载 docker socket。

* 容器内默认 `scope=host`；如果只监控容器自身，必须显式配置 cgroup 口径并在 UI 标注，不得把宿主进程数当容器进程数。

* 健康检查用 `/healthz`；就绪检查用 `/readyz`；`/metrics` 仅回环，集群抓取需 sidecar/反代并加认证。

**7.11.4 部署检查清单**

| 类别 | 检查项 | 通过标准 |
| ----- | ----- | ----- |
| 身份 | `agent_id` 唯一、`state.json` 可写 | 克隆镜像后重新生成；无重复 ID |
| 配置 | schema 版本、端点、周期、WAL 路径 | `--check-config` 通过；无未知键 |
| 权限 | 配置/令牌/WAL/日志权限 | 令牌 `0600`，配置 `0640`，WAL 目录仅服务用户可写 |
| 网络 | 出站 HTTPS、DNS、代理、CA | 推送成功；TLS 校验开启；无明文回退 |
| 时间 | NTP/chrony 同步 | 偏差在阈值内；`clock_adjusted` 可解释 |
| 容量 | WAL 96 MiB、日志轮转、磁盘余量 | 断网 1 小时不触发淘汰 |
| 观测 | `/status`、`/metrics`、日志、心跳 | 首轮 warmup 后 ready；服务端可见 |
| 回滚 | 上一版本二进制、配置备份 | 10 分钟内可回滚，WAL 可被旧版本识别或隔离 |

**7.11.5 升级与回滚**

* 升级前备份配置与 `state.json`；确认新版本 `schema_version` 仍为 `1.0` 或提供迁移说明。

* 先灰度 1 台，观察 30 分钟；再分批滚动。回滚只替换二进制，不回退 WAL；旧版本无法识别的 WAL 段必须隔离，不得删除。

* 配置变更走 `--check-config` 与热加载；热加载失败保留旧配置并记录 `config_reload_failed`，不退出进程。

## 8. 数据输出与上报

### 8.1 输出模式

| 模式 | 默认 | 用途 | 数据源 | 
| ----- | ----- | ----- | ----- | 
| 推送（push） | 开启 | 跨网络、NAT/防火墙后的 agent 主动上报 | WAL 中的快照批次 | 
| 拉取（pull） | 开启，仅回环 | Prometheus 抓取、本机排障 | 最近一次快照 | 
| 单次输出（`--once`） | 关闭 | 调试、验收、离线采集 | 现场采集一次，stdout 输出 JSON | 
| 本地文件 | 不提供 | 1.0 不写 JSON 文件，避免磁盘放大与轮转复杂度 | — | 

* 推送与拉取共享同一份快照，不允许各自触发独立采集，避免同一时刻出现两套数值。

* 推送是主路径；拉取是辅助路径。关闭推送时，拉取仍可工作；关闭拉取不影响推送。

* `--once` 不写 WAL、不更新推送状态；退出码见 8.9。

### 8.2 Prometheus 暴露

* 路径默认 `/metrics`，监听 `127.0.0.1:9101`；`Content-Type: text/plain; version=0.0.4; charset=utf-8`。

* 每次抓取返回最近一次快照的序列化结果；不触发新采集。快照不存在时返回 `sprobe_agent_up 0` 与 `sprobe_agent_warmup 1`。

* 指标名加 `sprobe_` 前缀（可配置 `metrics.namespace`）；标签只使用 4.2 定义的有界维度。

* 同一指标只输出一次 `# HELP`/`# TYPE`；类型与 4.1 一致：gauge/counter；`derived` 在 Prometheus 中按 gauge 暴露，但语义上仍是差分结果。

* 不输出逐进程指标（默认）；不输出 `cpu="total"` 聚合序列；不输出重复的 bond/master 流量。

**映射表**

| 规范名 | Prometheus 名 | 类型 | 标签 | 
| ----- | ----- | ----- | ----- | 
| `cpu_usage_ratio` | `sprobe_cpu_usage_ratio` | gauge | `cpu`（分核时） | 
| `cpu_user_seconds_total` | `sprobe_cpu_user_seconds_total` | counter | `cpu` | 
| `memory_available_bytes` | `sprobe_memory_available_bytes` | gauge | — | 
| `filesystem_available_bytes` | `sprobe_filesystem_available_bytes` | gauge | `device`、`mountpoint`、`fstype` | 
| `disk_read_bytes_total` | `sprobe_disk_read_bytes_total` | counter | `device`、`disk_class` | 
| `network_receive_bytes_total` | `sprobe_network_receive_bytes_total` | counter | `interface`、`class` | 
| `load_average_1m` | `sprobe_load_average_1m` | gauge | — | 
| `process_count` | `sprobe_process_count` | gauge | — | 
| `host_uptime_seconds` | `sprobe_host_uptime_seconds` | gauge | — | 
| `health_status_code` | `sprobe_health_status_code` | gauge | — | 
| `health` 检查项 | `sprobe_health_check_status` | gauge | `check`、`severity` | 

**自监控指标（agent 自身）**

| 指标 | 类型 | 含义 | 
| ----- | ----- | ----- | 
| `sprobe_agent_up` | gauge | 进程存活，恒 1 | 
| `sprobe_agent_info` | info | 版本、commit、os、arch、agent_id | 
| `sprobe_agent_warmup` | gauge | 1 = 预热中 | 
| `sprobe_agent_uptime_seconds` | gauge | 进程运行时长 | 
| `sprobe_collector_duration_seconds` | gauge | 最近一次采集耗时，标签 `collector` | 
| `sprobe_collector_last_success_timestamp_seconds` | gauge | 最近成功时间，标签 `collector` | 
| `sprobe_collector_errors_total` | counter | 采集错误数，标签 `collector`、`reason` | 
| `sprobe_collector_skipped_total` | counter | 背压跳过数，标签 `collector` | 
| `sprobe_snapshot_age_seconds` | gauge | 最近快照年龄 | 
| `sprobe_wal_snapshots` | gauge | WAL 中待发送快照数 | 
| `sprobe_wal_bytes` | gauge | WAL 占用字节 | 
| `sprobe_wal_evicted_total` | counter | 因容量/过期淘汰的快照数 | 
| `sprobe_wal_oldest_age_seconds` | gauge | 最旧待发送快照的年龄；WAL 为空时为 0 | 
| `sprobe_push_attempts_total` | counter | 推送尝试数 | 
| `sprobe_push_failures_total` | counter | 推送失败数，标签 `reason` | 
| `sprobe_push_last_success_timestamp_seconds` | gauge | 最近成功推送时间 | 
| `sprobe_push_duration_seconds` | gauge | 最近一次推送耗时 | 
| `sprobe_config_reload_total` | counter | 配置重载次数，标签 `result` | 
| `sprobe_tls_client_cert_expiry_seconds` | gauge | mTLS 客户端证书剩余秒数；未启用 mTLS 时不暴露 | 

* 自监控指标不以快照形式上报，避免循环依赖；由 `/metrics` 与日志暴露。需要远端可见时，服务端可通过拉取或独立心跳获取。

* 高基数禁止：`collector` 标签限定为固定枚举；`reason` 限定为错误码枚举；不得把 PID、路径、原始错误消息作为标签。

**文本示例（节选）**

```
# HELP sprobe_agent_up Agent process is running
# TYPE sprobe_agent_up gauge
sprobe_agent_up 1
# HELP sprobe_agent_info Agent build information
# TYPE sprobe_agent_info gauge
sprobe_agent_info{version="1.0.0",commit="a1b2c3d",os="linux",arch="x86_64",agent_id="6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f"} 1
# HELP sprobe_cpu_usage_ratio CPU usage ratio excluding idle, iowait and steal
# TYPE sprobe_cpu_usage_ratio gauge
sprobe_cpu_usage_ratio{cpu="0"} 0.3010
sprobe_cpu_usage_ratio{cpu="1"} 0.1820
# HELP sprobe_memory_available_bytes Estimated memory available for new processes
# TYPE sprobe_memory_available_bytes gauge
sprobe_memory_available_bytes 41234567168
# HELP sprobe_filesystem_available_bytes Filesystem space available to non-root users
# TYPE sprobe_filesystem_available_bytes gauge
sprobe_filesystem_available_bytes{device="/dev/nvme0n1p2",mountpoint="/",fstype="ext4"} 60129542144
# HELP sprobe_disk_read_bytes_total Total bytes read from disk
# TYPE sprobe_disk_read_bytes_total counter
sprobe_disk_read_bytes_total{device="nvme0n1",disk_class="physical"} 1234567890123
# HELP sprobe_network_receive_bytes_total Total bytes received
# TYPE sprobe_network_receive_bytes_total counter
sprobe_network_receive_bytes_total{interface="eth0",class="physical"} 987654321098
# HELP sprobe_health_status_code Health status: 0=ok 1=degraded 2=critical 3=unknown
# TYPE sprobe_health_status_code gauge
sprobe_health_status_code 1
sprobe_health_check_status{check="disk_space",severity="warning"} 1

```

* 字符串型状态（`health.status`、`interface.up` 的原因）不直接作为序列值；用数值/信息指标表达，字符串放 `info` 标签或 JSON。

* 空值指标不输出该序列（Prometheus 没有 null）；消费端用 `absent()` 或 `unless` 判断缺失。文档明确：**缺失 ≠ 0**。

* counter 回退时仍输出当前累计值；`rate()` 会出现负值/重置，Prometheus 会自动处理 counter reset；agent 侧不伪造。

### 8.3 推送流程

1. 快照写入 WAL（若启用推送），标记 `pending`。

2. 组批器按“6 个快照 / 1 MiB / 30s”任一条件触发，从 WAL 取最旧的连续序号。

3. 生成 `batch_id`，压缩，发送 `POST /api/v1/ingest`。

4. 2xx：解析响应，删除已确认的快照；`duplicates` 也视为确认；`rejected` 按 6.4 处理。

5. 4xx（非 408/429）：按错误码决定丢弃或停止；不得无限重试。

6. 5xx/超时/网络错误：指数退避重试，同一批次复用 `batch_id`。

7. 进程重启：从 WAL 恢复 `pending` 快照，使用原 `sequence` 与新 `batch_id` 重发；每条快照保留其**原 `instance_id`**。新实例产生的快照使用新 `instance_id`，批次按实例拆分，禁止混批；旧实例记录用旧身份 drain，直至清空或按策略隔离。

* 推送顺序按 `sequence` 升序；允许前一批失败时后一批等待（队头阻塞），以保证服务端按序处理。若配置允许乱序，最多允许 3 个批次在途，服务端按 `sequence` 排序。

* 批次过大时按序号中点拆分，保持连续；拆分后的 `batch_id` 重新生成。

* 推送成功但响应体解析失败：按失败重试；服务端幂等保证不重复入库。

### 8.4 重试、退避与熔断

| 情况 | 行为 | 
| ----- | ----- | 
| 连接失败/DNS 失败 | 重试；退避 1s 起，指数至 60s，抖动 ±20% | 
| 请求超时 | 重试；同一批次最多 5 次快速重试，之后进入常规退避 | 
| 5xx | 重试；退避同上 | 
| 429 | 读 `Retry-After`，取 `min(值, 60s)`；无该头则按退避 | 
| 408 | 重试 | 
| 413 | 批次减半拆分，最多 3 次；仍失败则丢弃最旧快照并告警 | 
| 400/409/422 | 不重试；记录 `rejected`，丢弃该批；连续 10 条触发健康 critical | 
| 401/403 | 停止推送，健康 `push_auth_failed=critical`；不删除 WAL；令牌修复后手动/自动恢复 | 
| 404 | 不重试；配置错误，健康 critical | 
| 连续 5 次失败 | 熔断 30s，期间不发送；熔断结束后半开重试 1 次 | 

* 退避状态持久化到状态文件，重启后不立即风暴式重试；首次重试加 0–1s 随机。

* 熔断只影响推送，不影响采集与 `/metrics`；健康状态标记 `degraded`。

* 令牌轮换：读取令牌文件变化后下一批生效；旧令牌 401 时重新读取一次，仍失败才停止。

### 8.5 WAL 设计

**目标**：服务端短时不可用时，agent 不丢数据、不无限占内存；进程崩溃后能恢复。

| 项 | 设计 | 
| ----- | ----- | 
| 目录 | `storage.wal.dir`，与状态文件分离，便于单独限额 | 
| 分段 | 按大小滚动，默认 8 MiB/段；段名含序号与创建时间 | 
| 记录 | 每条记录：魔数、版本、长度、CRC32C、payload（快照 JSON）；追加写 | 
| 完整性 | 启动时从最旧段扫描，遇到 CRC 失败/截断即停止，截断该段尾部；坏段保留为 `.corrupt` 供诊断，不参与发送 | 
| 刷盘 | `fsync=always` 每条；`batch` 每批；`never` 交给 OS。默认 `batch`，在“极端断电最多丢 1 个批次窗口（默认 30s）数据”与性能间折中 | 
| 上限 | `max_bytes=96MiB`、`max_age_seconds=3600`；超限淘汰最旧段并计数 `wal_evicted` | 
| 磁盘满 | 先删最旧段；仍失败则停止采集写入，健康 critical，保留 `/metrics` 与日志 | 
| 恢复 | 读取所有有效段，按 `sequence` 排序；重复序号保留最新一条；缺失序号记 `gaps` | 
| 确认水位与清理 | 维护持久化 Ack Watermark（`acked_sequence` 写入 `wal.ack`/`state.json`）；重启恢复时跳过 `sequence <= acked_sequence` 的记录避免大规模重复重发；当整段最大 `sequence <= acked_sequence` 时删除该段 | 

* 容量校验：单快照上限 256 KiB × 360 个（1 小时 / 10s）≈ 90 MiB；96 MiB 覆盖最坏情况。实际快照通常 10–50 KiB，64 MiB 只够典型负载，不再作为默认上限。

* WAL 只存待发送快照，不存已确认数据；不做长期存储。

* 状态文件 `state.json` 采用“临时文件 + fsync + 原子 rename + fsync 目录”写入；损坏时从 `.bak` 恢复，仍失败则重新生成身份并告警。

* WAL 与状态文件不放在网络文件系统上；NFS 上的文件锁与 fsync 语义不可靠，启动时检测并拒绝。

### 8.6 投递语义

* **至少一次**：成功响应前不删除 WAL；崩溃/超时可能导致重复发送。

* **不保证恰好一次**：服务端必须按 `(agent_id, instance_id, sequence)` 幂等去重；agent 不承诺端到端 exactly-once。

* **顺序**：同一 `instance_id` 内按 `sequence` 升序；跨实例不保证顺序。

* **丢失场景**：WAL 淘汰、4xx 丢弃、`fsync=never` 时 OS 崩溃、磁盘满停止写入；这些情况必须产生事件/指标，不能静默。

* **重复场景**：响应丢失、重试、进程重启后重发；服务端按幂等键处理，重复计数上报 `duplicates`。

* **缺口**：WAL 淘汰导致序号不连续时，下一批带 `gaps`；服务端据此区分“agent 没采到”与“数据丢了”。

### 8.7 服务端接收契约

服务端不在本文件实现范围内，但必须满足以下契约，否则 agent 行为无法保证：

1. 认证：Bearer token 或 mTLS；令牌与 agent/租户绑定；拒绝时返回 401/403，不返回 200。

2. 校验：按 `schema_version` 校验结构、类型、范围；非法快照单条拒绝，不整批 500。

3. 幂等：唯一约束 `(agent_id, instance_id, sequence)`；重复请求返回首次结果，`duplicates` 计数。

4. 乱序容忍：按 `sequence` 排序存储；允许批次重试导致的重复/乱序。

5. 响应：2xx 只代表“已持久化/已入队”，不能在校验前返回成功；建议先写持久化队列再 ACK。

6. 限流：429 + `Retry-After`；不得用 200 包裹错误。

7. 监控：按 agent 统计最近上报时间；超过 3×推送周期无数据触发“agent 失联”告警，而不是依赖 agent 自报。

8. 保留：原始快照与聚合数据保留策略由服务端定义；agent 不假设服务端永久保存。

### 8.8 背压与资源保护

* 待发送快照超过 `agent_backlog_snapshots`（默认 1000）或最旧超过 `agent_backlog_age_seconds`（默认 1800s）时：

  * 健康 `degraded`（触发 `agent_backlog` 检查），**不提前停止写入 WAL**，以保障断网 1 小时不丢数目标；

  * 仅当 WAL 触及 `max_bytes`（96 MiB）或 `max_age_seconds`（3600s）上限时，按 FIFO 淘汰最旧分段并计数 `sprobe_wal_evicted_total`；

  * 仅在磁盘写满导致 I/O 失败时，才降级为停写 WAL 并保留内存最新快照（健康 `critical`）。

* 内存上限：WAL 索引 + 在途批次 ≤ 64 MiB；不得把整个 WAL 读入内存。

* 网络预算：单 agent 推送带宽默认 ≤ 1 Mbps；批次压缩后仍超限时降低推送频率（不低于 30s），并记录 `push_throttled`。

* CPU 预算：采集稳态均值 ≤ 单核 1%，峰值保护阈值 ≤ 单核 2%；持续超过 2% 时优先降低 slow/medium 频率，不降低 fast 告警指标。

### 8.9 调试输出与退出码

`--once` 输出单个快照 JSON（与 6.2 相同），到 stdout；日志到 stderr；退出码：

| 退出码 | 含义 | 
| ----- | ----- | 
| 0 | 成功 | 
| 1 | 运行时错误（采集失败但已输出部分数据、推送失败等） | 
| 2 | 配置错误/参数错误 | 
| 3 | 已有实例在运行（文件锁） | 
| 4 | 启动前置条件失败（目录不可写、令牌缺失、端口占用） | 
| 5 | 不支持的平台/架构 | 

* 退出码稳定，供 systemd/Windows 服务/容器探针使用；不得用 0 表示“部分失败”。

* `--once` 默认不加载 WAL、不读取推送状态；用于验收时可通过 `--no-state` 保证不修改状态文件。

* 诊断包：`--diagnose` 输出配置（脱敏）、能力、最近错误、WAL 统计到 stdout；不包含令牌、完整命令行、进程环境变量。

## 9. 权限与安全

### 9.1 威胁模型

| 资产 | 风险 | 对手 | 
| ----- | ----- | ----- | 
| 主机指标数据 | 泄露主机名、IP、进程、容量等资产信息 | 网络窃听者、被攻陷的监控服务端 | 
| 推送令牌/客户端私钥 | 被窃取后伪造上报、污染数据 | 本机低权限用户、备份/镜像泄露 | 
| 配置文件 | 暴露内网地址、拓扑、租户信息 | 误提交、配置管理泄露 | 
| agent 进程 | 被利用提权、执行任意命令 | 本地攻击者、恶意配置文件 | 

信任边界：

* agent 运行在受监控主机上，**不信任网络**；服务端证书必须校验，默认不允许明文 HTTP。

* agent 不信任本地非特权用户：配置、令牌、WAL 目录必须不可被普通用户改写；不读取用户可控路径下的动态库/插件。

* 服务端不信任 agent 上报内容：必须做 schema、范围、速率与租户校验；agent 被攻陷不应影响其他租户。

### 9.2 最小权限

| 采集项 | Linux | Windows | 说明 | 
| ----- | ----- | ----- | ----- | 
| CPU/内存/负载/uptime | 普通用户 | 普通用户 | 不需要特权 | 
| 文件系统容量 | 普通用户 | 普通用户 | `statfs`/`GetDiskFreeSpaceEx` | 
| 网络接口统计 | 普通用户 | 普通用户 | `/sys/class/net`、IP Helper | 
| 进程聚合/Top-N | 普通用户 | 普通用户 | 其他用户进程的部分字段可能缺失 | 
| 其他用户进程 I/O | root 或 `CAP_SYS_PTRACE` | 管理员 | 默认关闭，按需开启 | 
| 磁盘 I/O 累计计数 | 普通用户 | 管理员（`IOCTL_DISK_PERFORMANCE`） | Windows 非管理员降级为 PDH 速率 | 
| CPU 频率/温度 | 视驱动，可能 root | 管理员/驱动 | 不作为核心功能，缺失不影响 `ok` | 

原则：

* 默认以专用非 root/非管理员账户运行；仅在明确需要时提升单项权限。

* 不使用 setuid/setgid 二进制；不通过 `sudo` 调用外部命令采集。

* 不执行 shell；所有采集通过系统调用/API 完成，避免命令注入与 PATH 劫持。

* 不加载用户可写的动态库/插件；1.0 无插件机制。

### 9.3 传输安全

* 仅支持 TLS 1.2+，优先 TLS 1.3；使用 rustls，禁用已知弱套件；不提供“关闭证书校验”的默认选项。

* 证书校验：主机名 + 证书链 + 有效期；支持自定义 CA 文件（私有 CA）；`tls_insecure_skip_verify=true` 仅在显式 `allow_insecure=true` 时允许，并记录 critical 告警。

* mTLS：可选；客户端证书/私钥文件权限 `0600`；证书轮换通过热加载，失败保留旧证书。

* 令牌：`Authorization: Bearer`；禁止放在 URL query、日志、错误消息、User-Agent；令牌文件权限 `0600`（Windows ACL 仅服务账户）。

* 令牌轮换：支持双令牌过渡（旧令牌保留 24h）；agent 读取文件变化后下一批使用新令牌；401 时重读一次。

* 代理：支持 `HTTPS_PROXY`/系统代理；代理凭据不写入配置文件；不通过代理发送明文。

* 时间校验：证书有效期依赖系统时间；时钟偏差 >5 分钟时记录 warning，但不在 agent 内自动改时钟。

### 9.4 认证与授权（服务端契约）

* 每个 agent 一个令牌或客户端证书，绑定 `agent_id` 与租户；禁止多租户共用令牌。

* 服务端校验令牌后，必须校验请求中的 `agent_id` 与令牌绑定一致；不一致返回 403，不静默改写。

* 令牌权限最小化：只允许 `ingest`，不允许查询/管理接口。

* 失败响应不泄露“令牌是否存在”：401/403 使用统一错误消息；审计日志记录来源 IP、agent_id、时间，不记录令牌。

* 重放防护：TLS 已提供传输保护；幂等键防止重复入库；不额外要求时间戳签名（避免时钟依赖），但服务端可对超大批次做速率限制。

### 9.5 本地暴露面

* `/metrics` 默认只监听 `127.0.0.1:9101`；`allow_remote=false` 时拒绝非回环绑定，配置校验直接失败。

* 需要远程抓取时：

  * 优先使用反向代理 + mTLS/基本认证；

  * 或设置 `allow_remote=true` 并配置 TLS（agent 内置 TLS 终止能力可选）；

  * 同时配置防火墙白名单，禁止公网直接暴露。

* HTTP 方法只允许 `GET`/`HEAD`；其他方法返回 405；不提供写接口。

* 不设置宽松 CORS；不提供目录列表、调试端点、pprof（除非显式编译调试版并仅回环）。

* `/status` 与 `/healthz` 仅回环；`/healthz` 返回进程存活，不返回敏感指标。

### 9.6 配置与密钥管理

* 配置文件不含明文密钥；出现 `token = "..."`、`password = "..."`、`private_key = "..."` 等键直接启动失败。

* 令牌文件独立存放，权限 `0600`；启动时检查权限，过宽则拒绝启动（可通过 `security.allow_loose_token_permissions=true` 临时放宽，但记录 critical）。

* 日志、`--diagnose`、错误消息统一脱敏：令牌、私钥、Authorization 头、代理凭据、完整命令行、环境变量。

* 不在崩溃转储/核心转储中保留令牌：启动后尽量不长期持有明文字符串；Rust 的字符串零化不保证，文档不承诺“内存绝对安全”，但要求不打印、不落盘。

* 配置管理：配置文件纳入版本控制时使用模板/占位符；令牌由 secret 管理系统注入文件；禁止把令牌写进镜像层。

* 容器：使用 Secret volume 挂载令牌；不要用环境变量传令牌（`docker inspect`/`/proc` 可见）。

### 9.7 数据最小化与隐私

* 默认不采集：完整命令行、环境变量、打开的文件、用户列表、MAC 地址、登录会话、进程所属用户。

* 可选采集：`processes.include_exe`、`agent.labels` 中的资产标签；开启时在文档中说明可能包含路径/业务信息。

* 主机名默认上报；如主机名含敏感信息，可配置 `agent.hostname` 覆盖或关闭（服务端用 `agent_id` 关联）。

* 不上传原始 `/proc` 内容、日志文件、配置文件；只上传规范化指标与事件。

* 事件 `details` 限制键值对与长度，禁止放文件内容、命令行、令牌。

* 数据保留与删除由服务端负责；agent 本地 WAL 只保留 1 小时，退出/卸载时按策略删除。

### 9.8 供应链与构建安全

* 提交 `Cargo.lock`；CI 执行 `cargo audit`（或同等依赖漏洞扫描）与许可证检查；高危漏洞阻断发布。

* 依赖最小化：不引入运行时动态加载、脚本引擎、HTTP 服务端大框架；优先成熟、维护活跃的 crate。

* 构建可复现：固定 Rust 工具链版本（MSRV 1.95 起），记录 commit、构建时间、目标 triple；发布产物附 `SHA256SUMS`。

* 发布签名：Linux 提供 GPG 签名，Windows 提供 Authenticode 签名（可选但推荐）；容器镜像签名/摘要固定。

* SBOM：发布时生成 SPDX/CycloneDX SBOM，列出直接与传递依赖版本。

* 不在构建脚本中下载未校验的二进制；`build.rs` 仅做编译期常量生成。

### 9.9 本地攻击面

* WAL/状态目录权限 `0750`，文件 `0640`；防止本地用户伪造快照或读取历史数据。

* 配置文件路径不接受来自环境变量的任意相对路径穿越；解析后规范化，拒绝符号链接指向 world-writable 目录。

* 原子写临时文件使用 `O_EXCL` + 随机后缀，防止符号链接攻击；写入后 `fsync` 并 `rename`。

* 日志文件不跟随符号链接；轮转使用安全创建。

* 不监听 Unix socket 的 world-writable 路径；如提供本地控制 socket，权限 `0600`。

* 崩溃处理不执行外部命令；核心转储默认关闭（systemd `LimitCore=0`），避免泄露内存中的令牌。

### 9.10 安全审计与检查清单

发布前必须通过：

* \[ \] 默认仅回环，远程访问需显式开启并配置 TLS/白名单。

* \[ \] 令牌/私钥权限检查生效，配置中无明文密钥。

* \[ \] 日志与诊断输出经脱敏测试，无令牌/命令行泄露。

* \[ \] 依赖漏洞扫描无 high/critical；许可证合规。

* \[ \] 以非 root/非管理员账户可完成核心采集；权限不足时降级而非崩溃。

* \[ \] WAL/状态/配置目录权限正确，符号链接攻击测试通过。

* \[ \] 服务端证书校验不可被配置轻易关闭；关闭时产生 critical 告警。

* \[ \] 无 shell 调用、无动态插件、无世界可写路径加载。

* \[ \] 发布产物签名/校验和/SBOM 齐全。

## 10. 错误处理与可靠性

### 10.1 错误分类

| 类别 | 例子 | 启动期 | 运行期 | 
| ----- | ----- | ----- | ----- | 
| 配置错误 | 未知字段、间隔越界、令牌缺失 | 失败退出（2/4） | 热加载失败，保留旧配置 | 
| 权限错误 | 无法读 `/proc/<pid>/io`、磁盘 IOCTL 拒绝 | 核心路径失败则退出（4）；非核心降级 | 字段 `null` + 能力降级 | 
| 瞬时错误 | 文件暂时不可读、PDH 查询失败、NFS 超时 | 不适用 | 重试 1 次；仍失败则本周期 `partial` | 
| 数据质量错误 | 计数器回退、值为负、超出范围 | 不适用 | 丢弃该值/置 `null`，记录事件 | 
| 协议错误 | 服务端 4xx、响应格式错误 | 不适用 | 按 8.4 分类处理 | 
| 资源错误 | 内存不足、fd 耗尽、磁盘满 | 目录不可写则退出（4） | 降级/停止写入，健康告警 | 
| 内部错误 | panic、序列化失败、逻辑断言 | 退出（1） | 隔离该采集器/该批次，继续运行 | 

### 10.2 处理原则

1. **启动期 fail-fast，运行期 fail-soft**：配置、身份、目录、端口等硬错误立即退出；单个采集器/单条数据失败不得拖垮进程。

2. **不伪造数据**：失败输出 `null` 或缺失，不用 0、-1、上次值冒充本次值（缓存复用必须标 `stale`）。

3. **隔离边界**：每个采集器独立任务/线程，panic 不传播；每个批次独立处理，单条坏数据不阻塞整批。

4. **有界重试**：只对瞬时/可重试错误重试；4xx 不重试；重试有上限、退避、抖动。

5. **幂等**：写 WAL、推送、状态文件更新都可重放；重复不产生副作用。

6. **可观测**：每次降级/丢弃/重试都有指标或事件；禁止静默吞错。

7. **保留最后良好值**：慢采集器失败时用缓存并标 `stale`；超过阈值不再参与告警。

8. **资源有界**：队列、WAL、事件、Top-N、日志都有上限；超限按策略淘汰并计数。

### 10.3 故障矩阵

| 故障 | 检测 | agent 行为 | 恢复 | 用户可见 | 
| ----- | ----- | ----- | ----- | ----- | 
| 单个采集器超时 | 2s 超时 | 本周期 `partial`，字段 `null`，计数 | 下周期重试 | `sprobe_collector_errors_total` | 
| 采集器 panic | `catch_unwind` | 标记 failed，继续其他采集器 | 下周期重试；连续 3 次禁用该采集器并告警 | 日志/事件 | 
| `/proc` 不可读 | 读取失败 | 相关字段 `null`，能力降级 | 权限修复后自动恢复 | `capabilities` | 
| 磁盘满 | 写 WAL/日志失败 | 停写 WAL，保留内存最新快照，健康 critical | 清理空间后恢复 | 健康/日志 | 
| WAL 损坏 | CRC 失败 | 截断坏记录，保留好记录，事件 | 自动 | `wal_corrupt` 事件 | 
| 状态文件损坏 | JSON/校验失败 | 从 `.bak` 恢复；失败则重建身份 | 手动/自动 | `agent_id_regenerated` | 
| 服务端 5xx | HTTP 状态 | 退避重试，WAL 缓冲 | 服务端恢复后自动 | `push_failures_total` | 
| 服务端 401 | HTTP 401 | 停止推送，保留 WAL，critical | 令牌修复/重读 | 健康 critical | 
| 网络分区 | 连接失败 | 同 5xx；WAL 增长 | 网络恢复后补发 | `wal_snapshots` | 
| 时钟跳变 | 单调/墙上对比 | 标记 `clock_adjusted`，不修改数据 | 自动 | 事件 | 
| 主机重启 | uptime 下降 | 清空差分基线，重建 | 自动 | `host_reboot` | 
| 配置文件损坏 | 热加载校验失败 | 保留旧配置，记录 error | 修复后重载 | 日志 | 
| 令牌文件权限过宽 | 启动检查 | 拒绝启动（4） | 修正权限 | 退出码 | 
| 端口被占用 | bind 失败 | 启动失败（4） | 换端口/释放 | 退出码 | 
| 已有实例 | 文件锁 | 退出（3） | 停止旧实例 | 退出码 | 
| 内存不足 | 分配失败/OS OOM | 进程可能被杀；supervisor 重启 | 自动重启，降低采集频率 | 服务重启 | 
| fd 耗尽 | 打开失败 | 关闭缓存 fd，降级；记录 | 自动/重启 | 日志 | 

### 10.4 panic、崩溃与重启

* 每个采集器在独立任务中执行，外层 `catch_unwind`；panic 信息写入日志（不含敏感数据），该采集器本周期标记 failed。

* 同一采集器连续 3 次 panic：禁用该采集器，健康 `degraded`，记录 `collector_disabled`；不自动重新启用，需重启或配置重载。

* 主循环 panic：进程退出码 1；systemd/Windows 服务自动重启；重启间隔采用 5s/10s/30s 退避，避免崩溃循环打满日志。

* 连续 5 次启动后 10 秒内退出：进入“崩溃循环”状态，日志输出 `crash_loop`，仍按 30s 重启；服务端可通过心跳缺失发现。

* 不在 panic 处理中再次 panic；`panic=abort` 与 `catch_unwind` 冲突，发布构建使用 `panic=unwind`。

* 核心转储默认关闭；需要诊断时由运维显式开启，并注意内存中可能含令牌。

### 10.5 超时、取消与退出

* 单采集器超时默认 2s；快照总超时 5s；推送请求 10s；连接 3s；WAL fsync 不设短超时（由 OS 决定）。

* 超时通过取消令牌传播：采集器应尽快返回；阻塞系统调用（如 `statfs`）放在独立线程，超时后放弃该结果，线程自然结束，不无限堆积。

* 退出流程（总预算 ≤15s，严格小于 systemd `TimeoutStopSec=25s`）：

  1. 收到 SIGTERM/服务停止：停止调度新采集；

  2. 等待在途采集 ≤3s；

  3. 尝试发送当前批次并刷盘 WAL/状态 ≤10s；

  4. 关闭 HTTP 端点、释放锁 ≤2s，退出 0。

* 第二次信号立即退出（退出码 130/1），不再等待；未发送数据保留在 WAL。

* SIGINT 在交互模式等价于 SIGTERM；SIGHUP 触发热加载，不退出。

* 退出超时后强制退出，记录 `shutdown_timeout`；不得无限期挂起导致 systemd 杀进程。

### 10.6 数据完整性

* WAL 记录 CRC32C；状态文件写临时文件 + fsync + 原子 rename；读取时校验版本与校验和。

* JSON 序列化失败（非法 UTF-8、NaN）在写入前拦截，字段替换/置 `null`，不让整批失败。

* 批次发送前计算内容哈希（可选 header `X-Sprobe-Content-SHA256`），服务端可校验传输完整性；TLS 已提供完整性，哈希用于排查代理损坏。

* 时间戳单调性：同一实例内 `collected_at` 不应回退；回退时以 `sequence` 为准，记录 `clock_backward`。

* 不把浮点数用于计数；字节/包数用整数，避免精度丢失。

### 10.7 资源耗尽保护

| 资源 | 上限/策略 | 
| ----- | ----- | 
| 采集并发 | 4（可配 1–16）；同类采集器不并发 | 
| 采集线程 | 主阻塞池默认 8（最大 16）；NFS 独立探测池 2 线程；队列满则跳过并计数 | 
| 快照内存 | 单快照 ≤256 KiB；在途批次默认 1（仅 `allow_out_of_order=true` 时允许 1–3） | 
| WAL | 96 MiB / 1h；超限淘汰最旧 | 
| 事件 | 每快照 ≤100；内存保留 ≤1000 条 | 
| 日志 | 单文件 10 MiB × 5；stderr 由 journald 轮转 | 
| 文件描述符 | 启动时提升 soft limit 至 4096（若允许）；打开文件即用即关，不缓存大量 fd | 
| 进程扫描 | Top-N 全量扫描一次；>5000 进程允许 partial；不递归读 `/proc/<pid>/task` 全量 | 
| 网络 | 单次响应 ≤64 KiB；批次 ≤4 MiB 压缩后 | 

### 10.8 启动失败判定

必须失败退出：

* 配置文件不存在/语法错误/未知键/值越界；

* `schema_version` 不支持；

* 数据目录不可创建/不可写；

* 状态文件损坏且无法恢复，且配置要求保留身份；

* 推送启用但令牌文件缺失/权限过宽/endpoint 非法；

* 拉取启用但监听地址被占用/无权限；

* 已有实例持有锁；

* 平台不支持（编译期未启用该平台）。

允许降级启动：

* 单个采集器权限不足；

* 磁盘 I/O 计数器不可用；

* CPU 频率/温度不可用；

* 网络链路速率未知；

* 进程可见性受限（hidepid）；

* 部分文件系统 `statfs` 超时。

降级必须写入 `capabilities` 与启动日志，不能只留在代码里。

### 10.9 恢复与自愈

* 采集器连续失败后按指数退避（最长 5 分钟）重试，不完全禁用（panic 除外）。

* 推送熔断后半开重试；成功后清零失败计数。

* WAL 恢复后按序号补发；服务端幂等，重复无害。

* 配置热加载失败不改变运行状态；修复后下一次 SIGHUP/文件变化生效。

* 主机重启/VM 恢复后，所有差分基线重建；首周期 `warmup=true`，服务端不告警。

## 11. 日志、自监控与告警

### 11.1 日志规范

**格式**

默认 JSON（每行一个对象），字段固定：

| 字段 | 类型 | 说明 | 
| ----- | ----- | ----- | 
| `ts` | string | UTC RFC 3339，毫秒 | 
| `level` | string | `error`、`warn`、`info`、`debug`、`trace` | 
| `msg` | string | 简短、稳定、可检索；不作为机器解析依据 | 
| `component` | string | `scheduler`、`collector.cpu`、`push`、`wal`、`config`、`http` 等 | 
| `event` | string | 稳定事件码，如 `collector_timeout`、`push_failed`、`config_reloaded` | 
| `agent_id` | string | 可选，启动后所有日志带 | 
| `instance_id` | string | 可选 | 
| `sequence` | integer | 与快照相关时带 | 
| `error_code` | string | 稳定错误码，可选 | 
| `duration_ms` | number | 耗时，可选 | 
| `fields` | object | 结构化细节，键值有界；禁止嵌套过深 | 

* 文本格式仅用于交互式排障（`--once`、前台运行），生产默认 JSON。

* 时间统一 UTC；不输出本地时区偏移。

* 单条日志 ≤4 KiB；超长截断并标 `truncated=true`。

* 日志不跨行（异常栈除外）；栈信息单独字段，最多 32 行。

**级别使用**

| 级别 | 使用场景 | 
| ----- | ----- | 
| `error` | 功能失败且需要人工介入：配置加载失败、WAL 损坏、推送认证失败、目录不可写 | 
| `warn` | 可自愈/降级：采集器超时、权限不足、字段缺失、重试、NFS 超时、时钟跳变 | 
| `info` | 生命周期与状态变化：启动/退出、ready、配置重载、健康状态变化、推送恢复 | 
| `debug` | 排障细节：每轮采集耗时、批次大小、跳过原因；默认关闭 | 
| `trace` | 逐字段/逐接口细节；仅开发构建或短时开启，禁止长期开启 | 

* 同一错误在 1 分钟内重复出现时，按 `error_code + component` 聚合，首条完整输出，后续只输出计数（每 60s 汇总一次），避免日志风暴。

* 启动时输出一次生效配置摘要（脱敏）：版本、平台、间隔、推送开关、监听地址、能力列表；不输出令牌、路径全量、标签值。

* 退出时输出原因、运行时长、采集/推送统计；异常退出尽量在 panic hook 中输出。

**禁止记录**

* 令牌、私钥、Authorization/Cookie、代理凭据。

* 完整命令行、环境变量、进程打开的文件、用户输入。

* 原始 `/proc` 内容、配置文件全文、请求/响应体（除非 debug 且已脱敏）。

* 主机上其他用户的敏感路径/用户名（进程名可保留，路径默认不记录）。

### 11.2 自监控

* `/metrics` 暴露 8.2 中的自监控指标；`/status` 返回 JSON 摘要（仅回环）：

```
{
  "version": "1.0.0",
  "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
  "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
  "uptime_seconds": 86400.12,
  "ready": true,
  "warmup": false,
  "health": "degraded",
  "last_snapshot_at": "2026-11-21T01:20:51.123Z",
  "last_push_at": "2026-11-21T01:21:02.345Z",
  "wal": {"snapshots": 12, "bytes": 1048576, "oldest_age_seconds": 120},
  "collectors": [
    {"name": "cpu", "last_success_at": "2026-11-21T01:20:51.100Z", "last_duration_ms": 12, "status": "ok"}
  ]
}

```

* `/healthz`：进程存活返回 200；不检查依赖，不返回指标。用于容器 liveness。

* `/readyz`：`ready=true` 返回 200，否则 503；用于 readiness，避免预热期接流量（拉取模式）。

* 自监控指标不得包含高基数标签；`collector` 为固定枚举，`reason` 为错误码枚举。

### 11.3 告警职责边界

* **agent 只产生健康状态与事件，不发送通知**。原因：多 agent 重复通知、通知渠道凭据不应下发到每台主机、服务端才能做去重/抑制/分组。

* 服务端基于上报数据、心跳缺失、自监控指标统一告警；agent 本地只写日志与 `/metrics`。

* 健康状态是“事实”，告警规则是“策略”；同一健康状态可在不同环境映射为不同严重级别。

* 告警必须可执行：每条规则有 owner、runbook、抑制条件；无动作的告警降级为面板/日报。

### 11.4 告警规则

以下为默认建议，阈值以 4.11 健康口径为准；`for` 为持续时间，避免瞬时抖动。表中指标名统一带 `sprobe_` 前缀，未带前缀的字段仅作可读说明。

| 告警 | 条件（示意） | for | 级别 | 处理 |
| ----- | ----- | ----- | ----- | ----- |
| `AgentDown` | `time() - sprobe_push_last_success_timestamp_seconds > 3 * 30` 或 `sprobe_agent_up == 0` | 2m | critical | 检查主机/网络/agent 进程 |
| `AgentRestartLoop` | `sprobe_agent_uptime_seconds < 60` 且 30 分钟内 ≥3 次 | 0m | warning | 查崩溃日志/配置 |
| `CollectorStale` | `time() - sprobe_collector_last_success_timestamp_seconds > 3 * 周期` | 5m | warning | 查权限/内核接口 |
| `CPUHigh` | `1 - sum without(cpu)(rate(sprobe_cpu_idle_seconds_total[5m]) + rate(sprobe_cpu_iowait_seconds_total[5m]) + rate(sprobe_cpu_steal_seconds_total[5m])) / sum without(cpu)(rate(sprobe_cpu_user_seconds_total[5m]) + rate(sprobe_cpu_nice_seconds_total[5m]) + rate(sprobe_cpu_system_seconds_total[5m]) + rate(sprobe_cpu_idle_seconds_total[5m]) + rate(sprobe_cpu_iowait_seconds_total[5m]) + rate(sprobe_cpu_irq_seconds_total[5m]) + rate(sprobe_cpu_softirq_seconds_total[5m]) + rate(sprobe_cpu_steal_seconds_total[5m])) > 0.90`（基于 counter 整机聚合，避免分核误报） | 5m | warning | 查进程/容量 |
| `CPUCritical` | 同上整机聚合表达式 `> 0.95` | 5m | critical | 同上，升级 |
| `MemoryHigh` | `avg_over_time(sprobe_memory_usage_ratio[5m]) > 0.90` | 5m | warning | 查泄漏/容量 |
| `MemoryCritical` | `avg_over_time(sprobe_memory_usage_ratio[5m]) > 0.95` | 5m | critical | 可能 OOM，立即处理 |
| `DiskSpaceLow` | `sprobe_filesystem_usage_ratio > 0.90` | 5m | warning | 清理/扩容 |
| `DiskSpaceCritical` | `sprobe_filesystem_usage_ratio > 0.95` | 5m | critical | 立即清理，防写满 |
| `DiskInodeLow` | `sprobe_filesystem_inodes_usage_ratio > 0.90` | 5m | warning | 查小文件/日志 |
| `FilesystemReadonly` | `sprobe_filesystem_read_only == 1` | 1m | critical | 查内核/磁盘故障 |
| `DiskLatencyHigh` | `avg_over_time(sprobe_disk_read_latency_seconds[5m]) > 0.1` 或写延迟同理 | 5m | warning | 查磁盘/RAID/云盘限流 |
| `NetworkErrors` | `rate(sprobe_network_receive_errors_total[5m]) > 1` 或 dropped 同理 | 5m | warning | 查网卡/驱动/链路 |
| `LoadHigh` | `sprobe_load_average_1m_per_core > 2.0` | 10m | warning | 查 CPU/IO 等待 |
| `ProcessMissing` | `sprobe_process_watch_present == 0` | 1m（连续 2 个 medium 周期） | critical | 拉起进程/查崩溃 |
| `PushFailing` | `time() - sprobe_push_last_success_timestamp_seconds > 3 * 30` | 5m | warning | 查网络/服务端 |
| `PushAuthFailed` | `sprobe_health_check_status{check="push_auth_failed"} == 1` | 1m | critical | 检查令牌/证书 |
| `WALBacklog` | `sprobe_wal_snapshots > 1000` 或 `sprobe_wal_oldest_age_seconds > 1800` | 10m | warning | 查推送/扩容 WAL |
| `ClockAdjusted` | `sprobe_host_clock_adjusted == 1` | 5m | info | 查 NTP，不自动改时钟 |
| `CertExpiring` | `sprobe_tls_client_cert_expiry_seconds < 14 * 86400` | 1h | warning | 轮换证书 |
| `ConfigReloadFailed` | `increase(sprobe_config_reload_total{result="failed"}[10m]) > 0` | 0m | warning | 查配置 |

* Windows 不使用 `LoadHigh` 直接套 Unix 规则；改用 `sprobe_system_processor_queue_length > 2 × 核数` + `CPUHigh`。

* 合成负载（`quality=estimated`）不触发 critical；只触发 warning，并标注数据质量。

* 同一主机上 `AgentDown` 抑制该主机其他告警；`FilesystemReadonly`/`DiskSpaceCritical` 抑制同文件系统的容量 warning。

* 维护窗口内静默，但 `AgentDown` 仍保留（避免“维护期间全静默”掩盖真故障）；维护结束后补发一次摘要。

### 11.5 去重、分组与通知

* 去重键：`alertname + agent_id + 关键标签`（如 `mountpoint`、`device`、`interface`）；同一键只保留一个活动告警。

* 分组：按 `env + service + agent_id` 分组；同一主机的多个磁盘告警合并为一条，附 top 3 明细。

* 通知路由：

  * critical → 值班通道（电话/IM），立即；

  * warning → 团队 IM，5 分钟聚合一次；

  * info → 日报/面板，不推送。

* 告警内容必须包含：主机、指标当前值/阈值、持续时间、runbook 链接、最近一次变更（如部署）。

* 恢复通知与触发通知使用同一去重键；恢复后保留 24 小时记录，便于复盘。

* 禁止把原始 JSON 快照直接发到通知渠道；只发摘要与链接。

### 11.6 面板与巡检

前端页面入口、组件绑定、空/异常态展示与响应式布局规范见配套前端文档 `sprobe_v1_2.md`；本文只定义服务端面板所需的指标、健康状态与事件。

**总览面板**

* 在线 agent 数、失联数、健康分布（ok/degraded/critical/unknown）。

* CPU 使用率 Top 10、内存使用率 Top 10、磁盘剩余最少 Top 10。

* 推送成功率、WAL 积压、最近 24h 告警数。

**单机面板**

* CPU：分核使用率、iowait、steal；负载/每核负载。

* 内存：used/available/cached/swap；内存使用率趋势。

* 磁盘：各文件系统使用率、inode、读写速率/延迟/利用率；只读状态。

* 网络：各接口收发速率/利用率/错误/丢弃；up 状态。

* 进程：总数、状态分布、Top-N CPU/RSS、被监视进程存在性。

* agent：采集耗时、失败、WAL、推送、配置重载、运行时长。

**巡检建议**

* 每日：失联 agent、容量 >85% 文件系统、持续 warning。

* 每周：容量趋势预测（7 天线性外推）、WAL 淘汰、证书到期。

* 每月：告警噪声复盘（Top 10 规则）、阈值校准、runbook 更新。

### 11.7 Runbook 要点

**AgentDown**

1. 确认主机是否可达（ping/SSH/云控制台）。

2. 检查 agent 进程/服务状态、端口、日志最近错误。

3. 检查网络/DNS/代理/证书；确认服务端入口可用。

4. 若主机正常但 agent 退出，查崩溃循环、配置、磁盘满、令牌权限。

5. 恢复后确认 `ready=true`、最近批次 `accepted`、WAL 开始下降。

**DiskSpaceCritical**

1. 确认文件系统与挂载点，排除 `statfs` 缓存/stale 误报。

2. 查大文件/日志/容器层/删除但未释放的文件（`lsof +L1`）。

3. 清理或扩容；不要直接删除 WAL/状态文件以外的 agent 数据。

4. 若根文件系统只读，先查内核日志与云盘状态，必要时重启/换盘。

**PushAuthFailed**

1. 确认令牌是否过期/轮换/租户变更。

2. 检查令牌文件权限与内容（不回显令牌）。

3. 更新令牌后等待下一批；确认 401 停止计数归零、WAL 开始下降。

4. 若 mTLS，检查客户端证书链/有效期/CA。

## 12. 测试与验收标准

### 12.1 测试分层

| 层级 | 范围 | 工具/方式 | 通过标准 | 
| ----- | ----- | ----- | ----- | 
| 单元测试 | 单位换算、差分、状态机、命名、序列化 | `cargo test` | 行覆盖率 ≥70%，核心模块 ≥85% | 
| 集成测试 | 配置加载、采集器、快照、WAL、HTTP 推送 | 测试桩/本地服务 | 全部用例通过，无 flaky | 
| 跨平台测试 | Linux x86_64/aarch64、Windows x86_64 | CI + 真机 | 字段存在/类型/口径抽样通过 | 
| 性能测试 | CPU/内存/启动/采集耗时/推送 | 基准脚本 | 满足 12.4 预算 | 
| 稳定性测试 | 72h 持续运行、故障注入 | soak/chaos | 无泄漏、无崩溃、无静默丢数 | 
| 安全测试 | 权限、脱敏、依赖漏洞、路径攻击 | 扫描 + 手工 | 9.10 清单全过 | 
| 兼容性测试 | 升级/回滚、旧 schema、容器/VM | 矩阵 | 12.8 通过 | 

### 12.2 指标验收口径

| 指标 | 验收方法 | 容差 | 
| ----- | ----- | ----- | 
| CPU 使用率 | 与 `top`/`vmstat` 同窗口对比（排除 iowait/steal 口径差异） | ±3 个百分点 | 
| CPU 分核 | 与 `/proc/stat` 手工计算对比 | ±2 个百分点 | 
| 内存 used/available | 与 `free -b` 的 available/used 对比 | ±2% | 
| swap | 与 `free -b`/`swapon --show` 对比 | ±1 MiB 或 ±2% | 
| 文件系统容量 | 与 `df -B1` 的 Size/Used/Avail 对比 | 字节级一致（允许 1 个块误差） | 
| inode | 与 `df -i` 对比 | ±1% | 
| 磁盘读写字节 | 与 `/proc/diskstats` 差分对比 | ±1% | 
| 磁盘延迟 | 在持续稳定 I/O 压测下与 `iostat -x` 的 `r_await`/`w_await` 对比（空闲 `Δops=0` 时为 `null`） | ±20%（口径差异需说明） | 
| 网络收发字节 | 与 `/sys/class/net/*/statistics` 差分对比 | ±1% | 
| 网络速率 | 与 `sar -n DEV 1` 对比 | ±5% | 
| 负载 | 与 `uptime`/`cat /proc/loadavg` 对比 | 1 分钟值 ±0.05 | 
| 进程数 | 与 `ps -e --no-headers | wc -l` 对比 | ±1%（hidepid 除外） | 
| 开机时长 | 与 `cat /proc/uptime` 对比 | ±2 秒 | 
| 健康状态 | 构造阈值场景，检查状态/滞后/恢复 | 与规则完全一致 | 

**Windows（Tier 1）指标对照验收口径**

| 指标 | Windows 对照工具 / 接口 | 容差与验收方法 | 
| ----- | ----- | ----- | 
| CPU 使用率 | `typeperf "\Processor(_Total)\% Processor Time"` / Task Manager | ±3 个百分点（同 10s 窗口） | 
| 内存 used/available | Task Manager / Resource Monitor / `Get-CimInstance Win32_OperatingSystem` | ±2%（按 `ullAvailPhys` 口径） | 
| 页面文件（Swap） | `typeperf "\Paging File(_Total)\% Usage"` / `Win32_PageFileUsage` | ±2% 或 ±1 MiB | 
| 文件系统容量 | PowerShell `Get-Volume` / `Get-PSDrive` | 字节级一致（允许 1 个簇误差） | 
| 磁盘 I/O | `typeperf "\PhysicalDisk(*)\Disk Read Bytes/sec"` 等 | ±5%（管理员校验累计值，非管理员校验 PDH 降级与 `capabilities.disk_io_counters=false`） | 
| 网络收发字节 | PowerShell `Get-NetAdapterStatistics` | ±1% | 
| 合成负载 | 离线固定队列序列单元测试验证 EWMA 公式 + 压测校验 `system_processor_queue_length` 联动 | 离线公式误差 ≤ 1e-4；实机标注 `quality=estimated` | 

* 验收必须在同一时间窗口、同一口径下对比；跨平台不要求数值相等。

* 容器/VM 场景需标注 `scope`，宿主与容器数值不同不算失败。

* 所有 `null`/缺失必须有 `capabilities` 或原因码解释，不能出现“无解释的缺失”。

### 12.3 功能验收清单

* \[ \] CPU：分核/聚合、user/system/idle/iowait/steal 口径正确；首次采样为 `null`；计数器重置不产生负值。

* \[ \] 内存：available 来源正确；used=total-available；swap=0 时比率为 `null`；缓存不重复计入 used。

* \[ \] 磁盘：容量与 `df` 一致；保留块口径正确；伪文件系统排除；网络文件系统超时不阻塞；分区不重复。

* \[ \] 网络：接口过滤正确；bond/bridge 不重复；速率未知为 `null`；利用率不超 1；重置处理正确。

* \[ \] 负载：Unix 与 `getloadavg` 一致；Windows 合成值标 estimated；每核负载不裁剪。

* \[ \] 进程：总数/状态/Top-N/监视项正确；PID 复用不串数据；hidepid 标 partial。

* \[ \] 开机时长：与系统口径一致；时钟跳变标记；重启后基线重建。

* \[ \] 健康：12 项检查可配置；滞后/恢复/unknown 行为正确；critical 优先级正确。

* \[ \] 配置：完整示例可启动；非法配置退出码 2；热加载成功/失败行为正确。

* \[ \] 推送：批次/压缩/重试/幂等/WAL 恢复正确；401 停止；413 拆分。

* \[ \] 拉取：`/metrics` 可抓取、类型正确、无重复序列；空值不输出序列。

* \[ \] 安全：默认回环；令牌权限检查；日志脱敏；无 shell 调用。

* \[ \] 跨平台：Linux x86_64/aarch64、Windows x86_64 冒烟通过；ARM 页大小/CLK_TCK 专项通过。

### 12.4 非功能验收

| 项 | 目标 | 测量 | 
| ----- | ----- | ----- | 
| 启动时间 | ≤2s 到 `ready`（完成第 0 轮基线与本地监听；不含 10s 差分预热） | 日志时间戳 | 
| 常驻内存 | ≤64 MiB（基准画像：8 核 / 1000 进程 / Top-N=20 / 10 挂载点 / 4 网卡 / WAL `fsync=batch` + gzip 推送） | RSS 采样 | 
| CPU 占用 | 稳态均值 ≤单核 1%、峰值 ≤2%（同上基准画像，10s 周期） | `pidstat`/cgroup | 
| 快照耗时 | fast 采集 p95 ≤100ms；slow ≤2s | 自监控指标 | 
| 推送延迟 | 采集到服务端可见 p95 ≤3×周期 | 端到端时间戳 | 
| WAL 恢复 | 96 MiB WAL 启动恢复 ≤5s | 启动计时 | 
| 二进制大小 | ≤20 MiB（strip 后） | 发布产物 | 
| 72h 稳定性 | 无崩溃、RSS 增长 ≤10%、无 fd 泄漏 | soak 报告 | 
| 丢数率 | 正常网络 0；服务端 1 小时不可用且 WAL 未满时 0 | 序号连续性 | 

### 12.5 故障注入用例

| 用例 | 注入方式 | 期望 | 
| ----- | ----- | ----- | 
| 采集器超时 | 人为 sleep/挂起 NFS | 本周期 partial，其他采集器正常，指标计数 | 
| 采集器 panic | 测试桩触发 panic | 进程不崩溃，采集器 failed，连续 3 次禁用 | 
| 计数器重置 | 重启网络/重建设备 | delta=null，重建基线，无负值 | 
| 时钟跳变 | 修改系统时间/NTP step | `clock_adjusted=1`，无负耗时，序号继续 | 
| 主机重启 | 重启 VM | uptime 回退，基线清空，`host_reboot` | 
| 服务端 500 | 测试服务端返回 500 | 退避重试，WAL 增长，恢复后补发 | 
| 服务端 401 | 返回 401 | 停止推送，critical，WAL 保留 | 
| 服务端 413 | 限制请求体 | 批次拆分，最终成功或按策略丢弃 | 
| 网络分区 | 断网 1 小时 | WAL 缓冲，恢复后序号连续；WAL 未淘汰 | 
| WAL 损坏 | 截断/改字节 | 启动扫描截断坏记录，保留好记录，事件 | 
| 磁盘满 | 填满数据盘 | 停写 WAL，critical，进程不崩溃 | 
| 配置文件损坏 | 写入非法 TOML | 热加载失败，保留旧配置，进程继续 | 
| 令牌权限过宽 | chmod 644 | 启动失败（4）或 critical（按配置） | 
| 端口占用 | 先占 9101 | 启动失败（4），明确错误 | 
| 双实例 | 启动两个进程 | 第二个退出码 3 | 
| 高进程数 | 启动 5000+ 进程 | 快照 partial/截断，不超时崩溃 | 
| ARM 64K 页 | QEMU/真机 | RSS/内存换算正确，无 4096 硬编码 | 
| Windows 中文系统 | 中文 locale | PDH 英文计数器可用，字段完整 | 
| 敏感信息脱敏（安全） | 注入含明文令牌的 401 响应、非法配置、采集器 panic | 日志与 `--diagnose` 输出 100% 不含明文令牌/私钥 | 
| TLS 负向校验（安全） | 服务端桩提供自签名/过期/域名不匹配证书 | 默认配置拒绝连接并记录错误，不降级明文 | 
| 符号链接攻击（安全） | 将 `state.json` 或日志目标指向 `/tmp` 符号链接 | 启动或原子写入直接拒绝，不覆盖目标文件 | 
| 静态构建与依赖审计（安全） | CI 执行 `ldd` 检查 `musl` 产物与 `cargo audit` | 无外部动态库依赖；无未修复 high/critical 漏洞 | 

### 12.6 测试环境

* Linux：Ubuntu 22.04/24.04、Rocky 9、Debian 12；x86_64 与 aarch64 各一台；容器环境各一套。

* Windows：Windows 10 22H2、Windows Server 2022；中文/英文各一台；管理员与非管理员账户各测一次。

* macOS（Tier 2）：macOS 14 Intel/Apple Silicon 各一台，做冒烟。

* 云环境：至少一家 KVM 云主机（验证 steal/虚拟化字段）、一家物理机。

* 服务端桩：可模拟 2xx/400/401/403/413/429/500/超时/乱序/重复。

### 12.7 验收流程

1. 开发自测：单元/集成/本地 `--once` 快照校验。

2. CI：lint、格式、测试、依赖审计、构建矩阵。

3. 平台验收：按 12.2 在真机抽样对比，记录原始命令与结果。

4. 稳定性：72h soak + 故障注入，输出报告。

5. 安全：9.10 清单 + 依赖扫描 + 脱敏抽查。

6. 发布评审：DoD 全过，签署发布记录。

### 12.8 DoD（完成的定义）

* 代码合并、CI 绿、无已知 high/critical 缺陷。

* 文档与实现一致：指标名、单位、频率、空值语义、配置键、退出码。

* 所有 Tier 1 平台冒烟通过；Tier 2 平台有明确“未验证/已知限制”说明。

* 性能/稳定性/安全验收通过，报告归档。

* 升级/回滚演练通过；发布包、校验和、SBOM、签名齐全。

* 运维文档（部署、配置、告警、runbook）可独立使用。

## 13. 分阶段实施计划

### 13.1 交付策略

* **先纵向切片，再横向扩平台**：第一版只做 Linux x86_64，但打通“配置 → 采集 → 快照 → 推送/拉取 → 验收”全链路，避免最后才联调。

* **先事实，后策略**：先保证指标口径与数据正确，再做健康/告警；否则错误指标会污染告警规则与历史数据。

* **平台按 Tier 推进**：Linux x86_64 → Linux aarch64 → Windows x86_64 → macOS/FreeBSD 冒烟；每个平台必须过“字段存在 + 口径抽样 + 故障注入”。

* **文档与实现同批交付**：指标/配置/schema 变更必须同步改文档与测试，不允许“代码先行、文档后补”。

* **每周可演示**：每周五产出可运行二进制 + `--once` 快照 + 对比报告，降低集成风险。

### 13.2 团队与职责

| 角色 | 人数 | 职责 | 
| ----- | ----- | ----- | 
| Tech Lead/Rust 架构 | 1 | 架构、schema、核心框架、代码评审、发布 | 
| Rust 采集开发 | 1–2 | CPU/内存/磁盘/网络/进程/Linux 采集器、规范化 | 
| 平台开发（Windows） | 1 | Windows 采集器、服务/MSI、PDH/权限 | 
| SRE/测试 | 1 | 测试环境、对比验收、soak/chaos、告警规则 | 
| 安全/合规 | 0.5 | 威胁模型、权限、依赖审计、脱敏评审 | 
| 技术写作/产品 | 0.5 | 文档、runbook、发布说明 | 

最小配置 4 人（架构 + 2 开发 + 1 SRE），Windows 与文档可阶段性投入；低于 3 人时砍掉 macOS 冒烟、FreeBSD 社区适配与容器高级场景，不砍 Tier 1 指标与测试。

### 13.3 阶段计划（10 周）

| 阶段 | 周期 | 目标 | 主要交付 | 退出标准 | 
| ----- | ----- | ----- | ----- | ----- | 
| P0 基础 | W1 | 可启动骨架、配置、日志、身份、schema、双架构 CI | 单二进制、`--check-config`、`--once`、状态文件、锁、`x86_64`/`aarch64` `musl` CI 门禁 | 配置校验/退出码/日志格式通过；schema 评审通过；双架构静态编译绿 | 
| P1 Linux 核心 | W2–W3 | CPU/内存/文件系统/磁盘/网络/负载/进程/uptime | 采集器 + 规范化 + 快照 + `/metrics` + 对比脚本 + aarch64 QEMU 冒烟 | 与 `df/free/top/vmstat/iostat/uptime/ps` 抽样误差达标 | 
| P2 上报链路 | W4–W5 | 推送、心跳、批次、WAL + Watermark、重试、幂等、ARM 真机冒烟 | HTTP 客户端、WAL、组批、退避、服务端桩、ARM 64K 页真机验证 | 服务端宕机 1 小时（WAL 容量内）不丢数；401/409/413/500 行为正确；ARM 专项通过 | 
| P3 跨平台 | W6–W7 | Windows x86_64 全量采集与服务化、能力声明（MSI 与 macOS 冒烟不阻断关键路径） | Windows 采集器、zip + `--install-service`、Windows 中/英文对照验收 | Tier 1 三平台冒烟与对照验收全过；Windows 权限降级符合预期 | 
| P4 健康与安全 | W8 | 健康引擎、自监控、加固 | 12 项检查、状态机、自监控、systemd 加固、脱敏 | 故障注入通过；安全清单全过 | 
| P5 稳定与验收 | W9 | 性能、72h soak、文档、告警 | 基准报告、soak 报告、运维文档、告警规则 | 12.4 指标达标；DoD 90% | 
| P6 发布 | W10 | RC → GA、回滚演练 | 发布包、签名、SBOM、升级/回滚记录 | 回滚演练通过；发布评审签署 | 

* W1 与 W2 之间设 schema/指标口径评审 gate；未通过不进入采集器实现。

* W5 与 W6 之间设“端到端联调 gate”；推送链路未通不扩 Windows，避免双线返工。

* W9 的 72h soak 必须在 W8 结束前启动，否则 W10 无缓冲。

* 预留 15% 缓冲（约 1.5 周），用于 Windows 权限/ARM 环境/服务端联调等不可控项。

### 13.4 里程碑

| 里程碑 | 时间 | 判据 | 
| ----- | ----- | ----- | 
| M0 架构基线 | W1 末 | schema、指标清单、配置契约冻结；CI 可构建 | 
| M1 Linux 可观测 | W3 末 | `--once` 输出完整快照；8 类指标对比通过 | 
| M2 数据可达 | W5 末 | 推送/拉取双通道可用；WAL 恢复/幂等验证通过 | 
| M3 跨平台 | W7 末 | Linux x86_64/aarch64、Windows x86_64 冒烟通过 | 
| M4 可运维 | W8 末 | 健康/自监控/告警/加固完成；故障注入通过 | 
| M5 可发布 | W9 末 | 性能/soak/安全/文档验收通过 | 
| M6 GA | W10 末 | 发布包、签名、SBOM、回滚演练完成 | 

### 13.5 工作分解与估算

| 工作包 | 内容 | 估算（人日） | 
| ----- | ----- | ----- | 
| 核心骨架 | CLI、配置、日志、身份、状态文件、锁、退出码 | 8 | 
| 调度与采集框架 | 分组、防重入、超时、panic 隔离、背压 | 8 | 
| Linux CPU/内存 | `/proc/stat`、`meminfo`、频率、差分、聚合 | 8 | 
| Linux 文件系统/磁盘 | statvfs、diskstats、设备分类、NFS 超时 | 9 | 
| Linux 网络/负载/进程/uptime | sysfs/netlink、getloadavg、proc 扫描、btime | 12 | 
| 规范化与快照 | 单位、空值、能力、schema 序列化 | 6 | 
| Prometheus 输出 | 文本格式、映射、自监控指标 | 4 | 
| 推送客户端 | 批次、gzip、TLS、重试、熔断 | 6 | 
| WAL | 分段、CRC、恢复、淘汰、fsync | 8 | 
| 服务端桩/联调 | mock ingest、幂等/乱序/错误注入 | 4 | 
| Windows 采集 | PDH、GetSystemTimes、内存、页面文件、磁盘、进程 | 12 | 
| aarch64/ARM 专项 | 交叉编译、页大小、CLK_TCK、拓扑、真机 | 5 | 
| 健康引擎 | 12 项检查、状态机、滞后、事件 | 5 | 
| 安全加固 | 权限、脱敏、TLS/mTLS、依赖审计、符号链接 | 5 | 
| 打包部署 | systemd、MSI、容器、升级/回滚 | 6 | 
| 测试 | 单元/集成/跨平台/故障注入/soak | 12 | 
| 文档 | 开发文档、部署、runbook、发布说明 | 5 | 
| **合计** |  | **123** | 

* 123 人日约等于 4 人 × 10 周 × 3.1 天/周的有效投入；实际按团队规模调整，不能压缩 P1 的对比验收与 P5 的 soak。

* 关键路径：schema → Linux 采集 → 快照 → 推送/WAL → 跨平台 → 稳定性；Windows 可与 P2 部分并行，但不得跳过 M2 gate。

### 13.6 依赖与前置条件

| 依赖 | 需要时间 | 风险 | 
| ----- | ----- | ----- | 
| Rust 1.95 工具链与 CI runner | W1 | 低 | 
| Linux x86_64/aarch64 测试机（含 64K 页 ARM） | W2 | ARM 机器难获取 → 用 QEMU 先验，W6 前补真机 | 
| Windows 测试机（中文/英文、管理员/非管理员） | W4 | 中；提前准备 | 
| 服务端 ingest 桩或真实服务端 | W4 | 高；服务端延期会阻塞 M2 → 先做桩，契约冻结 | 
| 私有 CA/令牌/租户信息 | W4 | 中；安全团队提前介入 | 
| 发布签名/仓库 | W9 | 中；提前申请 | 
| 安全评审 | W8 | 中；预留 3 天 | 

### 13.7 阶段验收 gate

* **G0（W1）**：schema 与指标清单评审通过；配置示例可启动；CI 绿。

* **G1（W3）**：8 类指标在真机对比达标；空值/能力声明无遗漏；`--once` 可复现。

* **G2（W5）**：服务端不可用 1 小时不丢数（WAL 容量内）；重复/乱序/401/413 行为符合 8.4；WAL 恢复通过。

* **G3（W7）**：Tier 1 三平台冒烟通过；ARM 专项通过；Windows 权限降级符合预期。

* **G4（W8）**：12 项健康检查与滞后通过；12.5 故障注入 22 项（含 4 项安全用例）全过；安全清单全过。

* **G5（W9）**：性能/72h soak/文档验收通过；无 high/critical 缺陷。

* **G6（W10）**：发布评审签署；回滚演练成功；发布包可复现。

## 14. 风险与开放问题

### 14.1 风险登记

| 风险 | 概率 | 影响 | 缓解 | 
| ----- | ----- | ----- | ----- | 
| 指标口径跨平台不一致导致误告警 | 高 | 高 | 口径写入文档；平台字段/能力显式声明；告警规则按平台区分 | 
| Windows 磁盘/进程权限不足 | 高 | 中 | 默认降级 + 能力声明；MSI 说明管理员/非管理员差异；提供 PDH 速率兜底 | 
| ARM 真机/64K 页环境不足 | 中 | 高 | QEMU 先验 + 云 ARM 实例；W6 前补真机；CI 保留 aarch64 runner | 
| 服务端 ingest 延期 | 中 | 高 | W4 前冻结契约并做桩；agent 侧先完成 WAL/重试；联调不依赖服务端全功能 | 
| schema/指标频繁变更 | 中 | 高 | M0 冻结；变更走评审；minor 只增不改；major 双版本窗口 | 
| 采集开销超预算（大核数/多进程） | 中 | 中 | 分组/缓存/Top-N；基准测试；大机器配置上限；动态降频 | 
| NFS/坏盘导致采集中断 | 中 | 中 | 独立线程 + 2s 超时 + 缓存；连续超时降级；不阻塞其他文件系统 | 
| 安全评审发现令牌/权限问题 | 中 | 高 | W4 前完成威胁模型；启动权限检查；脱敏测试；预留 3 天修复 | 
| 打包/签名/仓库延期 | 中 | 中 | W8 启动签名申请；先用 zip/tar 过渡；MSI 可后补 | 
| 文档与实现漂移 | 高 | 中 | schema/指标变更必须同 PR 改文档；CI 校验指标清单与代码注册表一致 | 
| upstream `sysinfo` API 变化 | 低 | 中 | 锁定 0.40.x；封装适配层；升级前跑全量对比 | 
| 32 位 ARM/旧内核需求插入 | 低 | 高 | 1.0 明确不支持；评估后放 1.x，避免中途改架构 | 

### 14.2 开放问题

| ID | 问题 | 影响 | 建议 | 决策人/时间 | 
| ----- | ----- | ----- | ----- | ----- | 
| Q1 | 服务端 ingest/存储/告警是否在本项目范围？ | 范围与工期 | 本文件只定义契约；服务端由独立项目/团队实现 | 项目负责人，W1 | 
| Q2 | 是否需要 32 位 ARMv7/旧内核？ | 架构与测试矩阵 | 1.0 不支持；有明确需求再评估 | 产品，W2 | 
| Q3 | macOS 是否提升为 Tier 1？ | 测试资源 | 1.0 保持 Tier 2，只做冒烟 | 项目负责人，W6 | 
| Q4 | 容器 cgroup 指标是否进入 1.0？ | 功能范围 | 1.0 只做 host 口径 + 能力声明；cgroup 放 1.1 | 产品，W4 | 
| Q5 | 温度是否纳入健康检查？ | 平台差异/误报 | 1.0 不纳入；作为可选指标后续加 | 产品，W8 | 
| Q6 | 是否采集进程命令行/所属用户？ | 隐私/基数 | 默认不采集；`include_exe` 已留开关，cmdline 不进入 1.0 | 安全，W4 | 
| Q7 | 远程拉取是否内置 TLS/认证？ | 安全/实现量 | 默认只回环；远程用反代/mTLS；内置 TLS 放 1.1 | 架构，W6 | 
| Q8 | 多租户/多环境标签模型？ | 服务端契约 | agent 只带 `labels`，租户由令牌绑定；不在 agent 做租户隔离 | 架构，W5 | 
| Q9 | 数据保留与删除策略？ | 合规 | 服务端定义；agent WAL 1h；文档声明 | 安全/运维，W9 | 
| Q10 | Windows 默认服务账户？ | 权限/安全 | 默认 LocalSystem，文档给出低权限降级方案 | 平台/安全，W6 | 

### 14.3 待冻结事项（M0 前）

* 指标清单与规范名（第 4 章、附录 A）。

* 快照/批次/错误 schema（第 6 章）。

* 配置键与默认值（第 7 章）。

* 健康检查项与默认阈值（4.11、11.4）。

* 退出码与错误码（8.9、6.4）。

* Tier 1 平台与构建矩阵（5.1、5.4）。

以上任一变更在 M0 后必须走变更评审，并同步更新测试用例与告警规则。

## 15. 常见问题（FAQ）

**Q1：为什么后端文档不再包含前端页面设计？**
A：Console UI 是独立项目，有自己的技术栈、部署与 D1 模型。后端只保证快照/心跳/健康/能力契约稳定；前端见 `sprobe_v1_2.md`。这样避免“可视化大屏不在范围”与“新增第 15 章”自相矛盾，也避免 10 周后端计划被前端工作污染。

**Q2：Agent 是否提供本地 `/ui` 页面？**
A：不提供。Agent 本地只暴露 `/metrics`、`/status`、`/healthz`、`/readyz`，且默认仅回环。现场排障用 `--once`、`--diagnose`、`/status` 和日志；展示统一走 Console。

**Q3：为什么 CPU 使用率不包含 iowait/steal？**
A：`cpu_usage_ratio = (user + nice + system + irq + softirq) / total`，排除 idle、iowait、steal，避免把“等 I/O”或“被宿主机抢占”误判为 CPU 忙。iowait/steal 作为独立 ratio 输出，告警可单独使用。

**Q4：为什么 `memory.used_bytes` 不等于 `total - free`？**
A：`used = total - available`。Linux 优先 `MemAvailable`，低内核回退 `free + buffers + cached + SReclaimable - Shmem`；Windows 用 `ullAvailPhys`；macOS 用 `free + inactive`。`available_source` 声明口径，跨平台比较必须看该字段。

**Q5：为什么 `filesystem.used_bytes + available_bytes` 小于 `total_bytes`？**
A：文件系统保留块（root reserve）不计入 `available`，所以 `used = (blocks - bfree) * frsize`、`available = f_bavail * frsize`，`usage_ratio = used / (used + available)`。这是为了与 `df` 的 Use% 对齐，不是采集错误。

**Q6：为什么没有“全网合计”网络指标？**
A：bond/bridge/vlan/veth 会造成重复计数；默认按接口输出，从属接口默认排除。需要合计时由服务端按明确的接口白名单计算，agent 不自动求和。

**Q7：Windows 为什么没有原生 load average？**
A：Windows 没有 Unix 口径的 load average。默认用 PDH `System\Processor Queue Length` 做 EWMA 合成，标 `quality=estimated`，并输出原始队列长度；告警用队列长度 + CPU，不直接套 Unix `load1` 阈值。

**Q8：为什么 401/403 一次就 critical？**
A：鉴权失败不是瞬时抖动，继续重试只会扩大风险；首次 401/403 即锁存 `push_auth_failed=critical` 并停止推送，保留 WAL，修复令牌后成功一次恢复。

**Q9：断网 1 小时需要多大 WAL？**
A：按单快照 256 KiB 上限、10s 周期，360 个快照约 90 MiB；默认 96 MiB（`100663296` 字节）。实际快照通常 10–50 KiB，余量充足；WAL 满时按 FIFO 淘汰并计数，不静默丢数。

**Q10：容器里为什么看到宿主进程/网卡？**
A：默认 `scope=host`，因为容器内 `/proc` 往往是宿主口径；cgroup 限额指标在 1.1 才纳入。若只想看容器自身，必须显式配置并标注口径，不能把宿主进程数当容器进程数。

**Q11：FreeBSD 还能用吗？**
A：Tier 3 社区探索，不承诺官方构建产物、CI 和验收；1.0 发布矩阵不含 FreeBSD。社区可自行编译并反馈，但不阻塞发布。

**Q12：本地如何联调推送？**
A：启动测试桩（覆盖 2xx/4xx/5xx/超时/乱序/重复），用 `--once --sample-window 10s` 生成快照，配置 `endpoint` 指向桩；检查 WAL 追加、批次、幂等键与 `duplicates`。测试桩与 fixtures 属于仓库测试资产，不进入发布包。

**Q13：升级后 WAL 还能用吗？**
A：同一 `schema_version` 内可以；跨 major 必须先 drain 或隔离旧 WAL。回滚只替换二进制，不回退 WAL；旧版本无法识别的段隔离保留，不删除。

**Q14：为什么前端字段名和后端 Prometheus 名不一样？**
A：JSON 快照字段是 camel/snake 路径（如 `cpu.total.usage_ratio`），Prometheus 指标是扁平名（如 `sprobe_cpu_usage_ratio`）。前端只消费 JSON 契约；Prometheus 名用于抓取与告警，两者通过附录 A/8.2 映射，不得混用。

## 16. 变更记录

| 版本 | 日期 | 类型 | 主要变更 |
| ----- | ----- | ----- | ----- |
| v1.3 | 2026-11-21 | M0 审核统一版 | 统一 6.4/6.12 响应与嵌套错误体、错误码（含 422）、心跳顶层字段与 `X-Sprobe-Batch-Id`；补齐 `cpu.cores[].frequency_hertz`；修正 11.4 整机 CPU PromQL 与 `ProcessMissing` 1m 窗口；修正 1.0 拉取端 `allow_remote` 无内置 TLS 规则；统一 `warmup` 1 周期、`fsync=batch` 30s、在途批次 1–3、`SprobeAgent` 服务名、G4 22 项故障注入与附录 A.6/B；同步机读契约 `sprobe_contract_v1_0.schema.json` |
| v1.2 | 2026-11-21 | 整理版 | 前后端拆分，删除第 15 章；新增 3.11 开发环境、6.12 接口速查、7.11 快速部署、15 FAQ、16 变更记录；修复 401 锁存、WAL 96 MiB/1 小时、实例语义、FreeBSD Tier3、告警阈值与字段路径；版本矩阵统一为 Sprobe 1.0 / 后端 v1.2 / 前端 v1.1 / schema 1.0 |
| v1.1 | 2026-11-21 | 评审基线 | 统一 Tier 分级；解耦 ready/warmup；重构 6.2 快照与 heartbeat；统一健康/告警阈值、频率、CLI/配置；WAL 背压/Ack Watermark、NFS 线程池、`--once --sample-window`；新增第 15 章前端设计（v1.2 已移除） |
| v1.0 | 2026-11-21 | 初版 | 建立 0–14 章结构、8 类指标、快照/批次/推送/WAL/安全/测试/10 周计划 |

**兼容性说明**

* 协议 `schema_version` 仍为 `1.0`；本次不删除/重命名快照字段，不改变单位与空值语义。

* 配置键未删除；`storage.wal.max_bytes` 默认值由 `67108864` 调整为 `100663296`，旧配置可继续使用，但 1 小时断网验收按 96 MiB 执行。

* `push_auth_failed` 的判定从“连续 2 次”改为“首次 401/403 锁存”，属于行为修正，不改变字段与事件结构。

* 删除第 15 章前端设计属于文档范围调整，不影响 Agent 协议；前端迁移见 `sprobe_v1_2.md` 的变更记录。

## 附录 A 指标速查表

### A.1 CPU

| 指标 | 类型 | 单位 | 频率 | 平台 | 
| ----- | ----- | ----- | ----- | ----- | 
| `cpu_usage_ratio` | gauge | ratio | 10s | 全平台 | 
| `cpu_idle_ratio` | gauge | ratio | 10s | 全平台 | 
| `cpu_iowait_ratio` | gauge | ratio | 10s | Linux | 
| `cpu_steal_ratio` | gauge | ratio | 10s | Linux/虚拟化 | 
| `cpu_user_seconds_total` | counter | seconds | 10s | 全平台 | 
| `cpu_nice_seconds_total` | counter | seconds | 10s | Linux/macOS | 
| `cpu_system_seconds_total` | counter | seconds | 10s | 全平台 | 
| `cpu_idle_seconds_total` | counter | seconds | 10s | 全平台 | 
| `cpu_iowait_seconds_total` | counter | seconds | 10s | Linux | 
| `cpu_irq_seconds_total` | counter | seconds | 10s | Linux | 
| `cpu_softirq_seconds_total` | counter | seconds | 10s | Linux | 
| `cpu_steal_seconds_total` | counter | seconds | 10s | Linux/虚拟化 | 
| `cpu_frequency_hertz` | gauge | hertz | 30s | 部分平台 | 
| `cpu_logical_count` | gauge | count | 300s | 全平台 | 
| `cpu_physical_count` | gauge | count | 300s | 部分平台 | 

### A.2 内存与交换

| 指标 | 类型 | 单位 | 频率 | 平台 | 
| ----- | ----- | ----- | ----- | ----- | 
| `memory_total_bytes` | gauge | bytes | 10s | 全平台 | 
| `memory_available_bytes` | gauge | bytes | 10s | 全平台 | 
| `memory_used_bytes` | gauge | bytes | 10s | 全平台 | 
| `memory_free_bytes` | gauge | bytes | 10s | 全平台 | 
| `memory_cached_bytes` | gauge | bytes | 30s | 全平台（口径不同） | 
| `memory_buffers_bytes` | gauge | bytes | 30s | Linux | 
| `memory_usage_ratio` | gauge | ratio | 10s | 全平台 | 
| `memory_dirty_bytes` | gauge | bytes | 30s | Linux | 
| `memory_writeback_bytes` | gauge | bytes | 30s | Linux | 
| `swap_total_bytes` | gauge | bytes | 10s | 全平台 | 
| `swap_used_bytes` | gauge | bytes | 10s | 全平台 | 
| `swap_free_bytes` | gauge | bytes | 10s | 全平台 | 
| `swap_usage_ratio` | gauge | ratio | 10s | 全平台（total=0 为 null） | 

### A.3 文件系统与磁盘

| 指标 | 类型 | 单位 | 频率 | 平台 | 
| ----- | ----- | ----- | ----- | ----- | 
| `filesystem_total_bytes` | gauge | bytes | 30s | 全平台 | 
| `filesystem_used_bytes` | gauge | bytes | 30s | 全平台 | 
| `filesystem_free_bytes` | gauge | bytes | 30s | 全平台 | 
| `filesystem_available_bytes` | gauge | bytes | 30s | 全平台 | 
| `filesystem_reserved_bytes` | gauge | bytes | 30s | Linux | 
| `filesystem_usage_ratio` | gauge | ratio | 30s | 全平台 | 
| `filesystem_inodes_total` | gauge | count | 30s | Linux | 
| `filesystem_inodes_used` | gauge | count | 30s | Linux | 
| `filesystem_inodes_free` | gauge | count | 30s | Linux | 
| `filesystem_inodes_usage_ratio` | gauge | ratio | 30s | Linux | 
| `filesystem_read_only` | gauge | 0/1 | 30s | 全平台 | 
| `disk_read_bytes_total` | counter | bytes | 30s | 全平台（Windows 可降级） | 
| `disk_write_bytes_total` | counter | bytes | 30s | 同上 | 
| `disk_read_operations_total` | counter | count | 30s | 全平台 | 
| `disk_write_operations_total` | counter | count | 30s | 全平台 | 
| `disk_io_time_seconds_total` | counter | seconds | 30s | Linux/macOS | 
| `disk_io_time_weighted_seconds_total` | counter | seconds | 30s | Linux | 
| `disk_in_flight_operations` | gauge | count | 30s | 全平台 | 
| `disk_read_bytes_per_second` | gauge | bytes_per_second | 30s | 全平台 | 
| `disk_write_bytes_per_second` | gauge | bytes_per_second | 30s | 全平台 | 
| `disk_utilization_ratio` | gauge | ratio | 30s | 全平台 | 
| `disk_read_latency_seconds` | gauge | seconds | 30s | 全平台 | 
| `disk_write_latency_seconds` | gauge | seconds | 30s | 全平台 | 

### A.4 网络

| 指标 | 类型 | 单位 | 频率 | 平台 | 
| ----- | ----- | ----- | ----- | ----- | 
| `network_receive_bytes_total` | counter | bytes | 10s | 全平台 | 
| `network_transmit_bytes_total` | counter | bytes | 10s | 全平台 | 
| `network_receive_packets_total` | counter | count | 10s | 全平台 | 
| `network_transmit_packets_total` | counter | count | 10s | 全平台 | 
| `network_receive_errors_total` | counter | count | 10s | 全平台 | 
| `network_transmit_errors_total` | counter | count | 10s | 全平台 | 
| `network_receive_dropped_total` | counter | count | 10s | Linux/macOS | 
| `network_transmit_dropped_total` | counter | count | 10s | Linux/macOS | 
| `network_receive_bytes_per_second` | gauge | bytes_per_second | 10s | 全平台 | 
| `network_transmit_bytes_per_second` | gauge | bytes_per_second | 10s | 全平台 | 
| `network_speed_bytes_per_second` | gauge | bytes_per_second | 300s | 全平台（未知 null） | 
| `network_receive_utilization_ratio` | gauge | ratio | 10s | 全平台（未知 null） | 
| `network_transmit_utilization_ratio` | gauge | ratio | 10s | 全平台（未知 null） | 
| `network_interface_up` | gauge | 0/1 | 10s | 全平台 | 
| `network_mtu_bytes` | gauge | bytes | 300s | 全平台 | 

### A.5 负载、进程、uptime、健康

| 指标 | 类型 | 单位 | 频率 | 平台 | 
| ----- | ----- | ----- | ----- | ----- | 
| `load_average_1m` | gauge | count | 10s | Unix；Windows estimated | 
| `load_average_5m` | gauge | count | 10s | 同上 | 
| `load_average_15m` | gauge | count | 10s | 同上 | 
| `load_average_1m_per_core` | gauge | count/core | 10s | 同上 | 
| `system_processor_queue_length` | gauge | count | 5s | Windows | 
| `processes_running` | gauge | count | 10s | Linux | 
| `processes_blocked` | gauge | count | 10s | Linux | 
| `process_count` | gauge | count | 30s | 全平台 | 
| `process_running_count` | gauge | count | 30s | 全平台 | 
| `process_sleeping_count` | gauge | count | 30s | 全平台 | 
| `process_blocked_count` | gauge | count | 30s | Linux | 
| `process_zombie_count` | gauge | count | 30s | Linux/macOS | 
| `process_thread_count` | gauge | count | 30s | 全平台 | 
| `process_watch_present` | gauge | 0/1 | 30s | 配置了 `processes.watch` 时 | 
| `host_uptime_seconds` | gauge | seconds | 10s | 全平台 | 
| `host_boot_time` | info | RFC 3339 | 300s | 全平台 | 
| `agent_uptime_seconds` | gauge | seconds | 10s | 全平台 | 
| `host_clock_adjusted` | gauge | 0/1 | 10s | 全平台 | 
| `health_status_code` | gauge | enum | 10s | 全平台 | 
| `health_check_status` | gauge | enum | 10s | 全平台 | 

### A.6 默认阈值速查

| 项 | warning | critical | 
| ----- | ----- | ----- | 
| CPU 使用率（5m 均值） | 0.90（健康检查另附 `iowait < 0.20`） | 0.95（5m） | 
| 内存使用率 | 0.90 | 0.95（5m） | 
| 文件系统使用率 | 0.90 | 0.95 | 
| inode 使用率 | 0.90 | — | 
| 磁盘平均延迟 | 0.10s | — | 
| 每核负载（10m） | 2.0 | — | 
| 网络错误/丢弃速率 | 1/s | — | 
| 被监视进程 | — | 缺失 1m（连续 2 个 medium 周期） | 
| WAL 积压 | 1000 快照或 1800s | — | 
| 推送失联 | 3×周期 | 401/403 立即 critical | 
| 证书剩余 | 14 天 | — | 

## 附录 B 参考资料

* `sysinfo` crate 文档（0.40.x，跨平台系统信息、CPU 差分语义、MSRV）：https://www.google.com/search?q=https://docs.rs/sysinfo/0.40.4

* Linux `proc(5)`（`/proc/stat`、`/proc/meminfo`、`/proc/diskstats`、`/proc/<pid>`）：https://man7.org/linux/man-pages/man5/proc.5.html

* Linux 内核块设备统计（`diskstats` 字段与单位）：https://docs.kernel.org/admin-guide/iostats.html

* Linux cgroup v2（`cpu.stat`、`memory.current`、`memory.max`）：https://docs.kernel.org/admin-guide/cgroup-v2.html

* Linux PSI（压力停滞信息，后续扩展参考）：https://docs.kernel.org/accounting/psi.html

* Prometheus 指标命名最佳实践：https://prometheus.io/docs/practices/naming/

* Prometheus 文本暴露格式：\[suspicious link removed\]

* OpenTelemetry System Semantic Conventions（指标名映射参考）：https://opentelemetry.io/docs/specs/semconv/system/

* Prometheus `node_exporter`（指标命名与采集口径参考）：https://github.com/prometheus/node_exporter

* Windows PDH（性能计数器，英文计数器 API）：https://learn.microsoft.com/en-us/windows/win32/perfctrs/using-the-pdh-functions-to-consume-counter-data

* Windows `GetSystemTimes`：https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes

* Windows `GlobalMemoryStatusEx`：https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex

* Windows `EnumPageFiles`（页面文件统计）：https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-enumpagefilesw

* Windows IP Helper（`GetIfEntry2`、接口统计）：https://www.google.com/search?q=https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getifentry2

* macOS `sysctl`/Mach 接口（`host_statistics64`、`kern.boottime`）：https://developer.apple.com/documentation/kernel

* IOKit 块设备统计（macOS 磁盘 I/O）：https://developer.apple.com/documentation/iokit

* RFC 3339（时间格式）：https://www.rfc-editor.org/rfc/rfc3339

* TOML v1.0.0：https://toml.io/en/v1.0.0

* Semantic Versioning 2.0.0：https://semver.org/

* systemd 执行环境与沙箱（`systemd.exec`）：https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html

* rustls（TLS 实现）：https://github.com/rustls/rustls

* RFC 1952（gzip 格式）：https://www.rfc-editor.org/rfc/rfc1952

* RFC 9110（HTTP 语义）：https://www.rfc-editor.org/rfc/rfc9110

**文档结束**

版本：v1.3（M0 审核修订统一版）
日期：2026-11-21