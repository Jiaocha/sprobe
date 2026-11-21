# Sprobe

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Vue](https://img.shields.io/badge/Vue-3.5%2B-emerald.svg)](https://vuejs.org/)
[![Bundle Size](https://img.shields.io/badge/Console%20Gzip-~61%20KiB-brightgreen.svg)](console/)
[![Online Demo](https://img.shields.io/badge/Console-sprobe--console.pages.dev-blueviolet.svg)](https://sprobe-console.pages.dev)

Sprobe 是一款极轻量、高性能、跨平台的现代服务器状态监控探针：以**单个静态二进制**运行（内存占用 < 15 MiB，CPU < 0.5%），精确采集 CPU、内存、磁盘容量与 I/O、网络吞吐、系统负载、进程 Top-N、开机时长、多线骨干网络延迟及本地健康状态。支持通过 HTTPS 批量推送 JSON 快照（集成 WAL 断网离线缓存与回放），并可在本地暴露 Prometheus `/metrics` 抓取接口。

Sprobe 配套超轻量现代化前端 Console（打包体积仅 **~61 KiB gzip**，可零成本托管于 Cloudflare Pages 边缘网络），深度融合了**哪吒监控（Nezha）**的优秀交互特性与现代设计美学，提供经典服务器卡片网格与专业运维表格双视图、地区与国旗徽标识别、VPS 月结出账流量配额防超额告警、全球骨干网延迟矩阵以及实时网络瞬时带宽双折线面积图。

- 🌐 **在线演示体验**: [https://sprobe-console.pages.dev](https://sprobe-console.pages.dev)
- 🚀 **极速一键接入 (Linux)**:
```bash
curl -fsSL https://sprobe-console.pages.dev/install.sh | sudo bash -s -- --endpoint https://sprobe-console.pages.dev/api/v1/ingest --name "My-Server"
```

---

## 核心特性矩阵

- **跨平台单二进制**：纯 Rust 开发，无外部运行时依赖，统一抽象 Linux (`/proc`, `/sys`) 与 Windows (Win32 API / PDH) 底层采集。
- **借鉴哪吒监控（Nezha）标志性体验**：
  - **国旗与地区徽标**：根据配置标签（如 `region = "US"`、`CN`、`JP`、`HK`、`SG`、`DE` 等）自动匹配对应国家/地区旗帜。
  - **卡片网格与表格双视图**：卡片视图一屏总览每台 VPS 核心负荷与流量，表格模式适合大规模集群过滤筛选。
  - **VPS 月结出账周期流量监控 (Cycle Traffic)**：可设置每月出账日与配额限额，精确累加当期出站流量，并在 80% 与 90% 水位自动触发健康状态降级警告。
  - **智能虚拟网卡过滤**：默认自动剔除 `docker*`、`veth*`、`br-*` 等虚拟网桥，避免流量虚高翻倍统计。
- **高可用与断网保护**：内置环形 Write-Ahead Log（WAL），网络异常时自动持久化到本地磁盘，网络恢复后分批补发，零数据丢失。
- **独立心跳守护定时器**：独立保活线程，不受数据采集阻塞影响，毫秒级心跳上报。
- **双重可观测性**：Push 模式支持 Gzip 批量上报与 Token 认证；Pull 模式支持 Prometheus 0.0.4 标准抓取。

---

## 快速开始

### 1. 编译并单次运行探针

```bash
# 编译 Release 二进制产物
cargo build --release

# 单次采集并输出当前机器硬件与状态快照（JSON 格式）
./target/release/sprobe --once

# 运行脱敏系统诊断（检查硬件、内核、采集能力与权限）
./target/release/sprobe --diagnose
```

### 2. 启动探针常驻守护进程

```bash
# 前台启动（指定配置文件）
./target/release/sprobe -c /etc/sprobe/sprobe.toml

# 或者通过环境变量配置
export SPROBE_CONFIG=/etc/sprobe/sprobe.toml
export SPROBE_TOKEN=your-secret-token
./target/release/sprobe
```

### 3. 一键生成 Linux Systemd 守护进程服务

```bash
# 利用 sprobe 内置指令生成标准加固的 systemd unit 文件
./target/release/sprobe -c /etc/sprobe/sprobe.toml --generate-systemd | sudo tee /etc/systemd/system/sprobe.service

sudo systemctl daemon-reload
sudo systemctl enable --now sprobe
```

### 4. 启动 Console 前端控制台

```bash
cd console
npm install
npm run dev
# 浏览器访问 http://localhost:5173
```

---

## 代码目录结构

```text
.
├── src/                               # Sprobe Agent Rust 源码
│   ├── collectors/                    # Linux / Windows 各指标采集器与网卡过滤引擎
│   ├── config/                        # TOML 配置文件加载、校验与脱敏
│   ├── health/                        # 本地健康规则判定引擎与配额告警
│   ├── http/                          # 嵌入式微型 HTTP 服务 (/healthz, /readyz, /status)
│   ├── output/                        # Push 批量推送客户端与 Prometheus 格式化导出
│   ├── snapshot/                      # 核心数据模型与 JSON 快照结构
│   └── wal/                           # 离线 Write-Ahead Log 持久化与恢复引擎
├── console/                           # Console 现代控制台前端 (Vue 3 + Tailwind CSS)
│   ├── src/pages/Overview.vue         # 集群总览页 (卡片网格 / 表格双视图、国旗徽标、流量进度条)
│   ├── src/pages/NodeDetail.vue       # 节点详情页 (月度配额专卡、网络双折线 SVG 图、网卡细分)
│   └── src/mock/nodes.ts              # 纯前端全场景 Mock 数据源
├── contract/                          # 机读快照与 API 契约 (JSON Schema v1.0)
├── docs/                              # 实施文档与用户指南
│   ├── USER_GUIDE.md                  # 📖 详尽的用户使用与运维部署手册
│   ├── backend/rust_v1_3.md           # 后端底层架构规范 (v1.3)
│   ├── frontend/sprobe_v1_2.md        # 前端规范与设计标准 (v1.2)
│   └── plan/                          # 开发与交付计划
├── ops/                               # 运维部署参考资产 (Systemd / Windows / Docker / Prometheus)
└── scripts/                           # 本地与 CI 自动化验收测试套件
```

---

## 文档索引

- **[📖 用户使用与运维部署手册 (USER_GUIDE.md)](docs/USER_GUIDE.md)**：包含完整配置参数说明、Linux / Windows / Docker 生产部署流程、Prometheus 集成以及故障诊断排错 FAQ。
- **[后端开发设计规范](docs/backend/rust_v1_3.md)**：指标口径定义、采集算法、WAL 协议、安全加固。
- **[前端开发设计规范](docs/frontend/sprobe_v1_2.md)**：页面结构、只读 API 契约、状态字典。
- **[机读契约 Schema](contract/sprobe_contract_v1_0.schema.json)**：JSON Schema 单一事实之源。

---

## 验证与测试套件

```bash
# 运行后端全量单元测试与集成测试 (27 项测试)
cargo test

# 运行代码静态检查与代码风格审查
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# 前端构建测试 (校验 TypeScript 严格类型与打包体积)
npm run build --prefix console

# 运行全链路端到端集成自动化验收测试
node scripts/verify_m3_e2e.mjs
```
