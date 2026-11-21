# Sprobe 1.0 用户使用与运维部署手册

Sprobe（Server Probe）是一款使用 Rust 编写的高性能、轻量级、跨平台服务器指标探针，专为现代基础设施、混合云与 VPS 节点监控而设计。探针以单个静态单二进制运行，提供极低的 CPU 与内存占用（常驻内存通常 < 15 MiB，CPU 占用 < 0.5%），支持 Linux 与 Windows 双平台。

Sprobe 前端 Console 采用 Vue 3 + Tailwind CSS 构建，采用经典卡片与表格双向视图，打包体积严格控制在 **61 KiB gzip**（远低于 200 KiB 规范），并深度借鉴参考了**哪吒监控（Nezha）**的标志性优秀特性。

---

## 目录

- [一、核心特性与亮点](#一核心特性与亮点)
- [二、5 分钟快速上手指南 (Quick Start)](#二5-分钟快速上手指南-quick-start)
- [三、CLI 命令行工具完整参考](#三cli-命令行工具完整参考)
- [四、配置文件全参数详尽指南 (`sprobe.toml`)](#四配置文件全参数详尽指南-sprobetoml)
- [五、生产环境部署实战](#五生产环境部署实战)
  - [1. Linux Systemd 守护进程部署](#1-linux-systemd-守护进程部署)
  - [2. Windows 服务部署](#2-windows-服务部署)
  - [3. Docker 容器化部署](#3-docker-容器化部署)
  - [4. Prometheus 抓取与监控集成](#4-prometheus-抓取与监控集成)
- [六、Console 控制台前端部署与使用](#六console-控制台前端部署与使用)
- [七、运维诊断与常见问题 (FAQ)](#七运维诊断与常见问题-faq)

---

## 一、核心特性与亮点

1. **单二进制跨平台运行**：
   - Rust 编写，无 glibc 动态链接强依赖（支持 musl 静态编译），零外部运行时环境依赖。
   - 统一抽象 Linux (`/proc`, `/sys`) 与 Windows (Win32 API, Pdh, GetSystemTimes, Performance Counters) 底层指标。

2. **借鉴哪吒监控（Nezha）标志性特性**：
   - **地区与国旗旗帜徽标**：根据配置标签（如 `region = "US-West"` / `CN-Shanghai` 等）自动匹配并呈现节点所在国家/地区国旗与胶囊标签。
   - **卡片网格 vs 详细表格双视图**：Console 前端一键切换 Nezha 经典 VPS 服务器卡片网格视图与专业运维表格视图，具备 `localStorage` 自动记忆。
   - **VPS 月结出账周期流量监控 (Cycle Traffic)**：可设置每月出账重置日（如每月 1 日），精确累计出站流量配额；在 80% 与 90% 水位自动触发健康状态降级告警（`degraded` / `critical`）。
   - **网卡白名单与虚拟网卡智能过滤**：默认自动过滤 `docker*`、`veth*`、`br-*`、`cni*`、`tailscale*` 等虚拟桥接网卡，消除多网卡流量重复计量与虚高问题。

3. **高可靠传输与本地离线缓冲 (WAL)**：
   - 内置环形 Write-Ahead Log（WAL），网络闪断或服务端维护时自动持久化到本地磁盘，网络恢复后幂等分批补发，零数据丢失。
   - 独立轻量级心跳守护线程（Heartbeat Daemon），不受数据采集或重试堵塞影响，毫秒级保活。

4. **双重可观测性接入**：
   - **Push 模式**：通过 HTTP(S) 批量上报标准 JSON 快照，原生支持 Gzip 压缩与 Token 鉴权。
   - **Pull 模式**：可选启用回环地址嵌入式微型 HTTP 服务，暴露 Prometheus `/metrics` 端点，以及 `/healthz`、`/readyz`、`/status` 健康自检接口。

5. **极速高颜值轻量前端**：
   - 纯 SVG 绘制实时计算走势图（CPU/内存双曲线）与实时网络上下行（Rx/Tx）瞬时带宽走势图，不依赖任何第三方大型图表库，首屏加载如闪电。

---

## 二、5 分钟快速上手指南 (Quick Start)

### 步骤 1：编译或获取二进制文件

确保本机已安装 Rust 1.85+（推荐 1.95+）：

```bash
# 克隆代码库并编译 Release 产物
cargo build --release

# 编译生成产物位于：
# Linux: target/release/sprobe
# Windows: target/release/sprobe.exe
```

### 步骤 2：单次采样体验与快照预览（无需配置上报端）

使用 `--once` 参数直接在控制台输出单次完整快照 JSON，验证底层硬件采集能力：

```bash
# 执行一次差分采样（默认采样窗口 1 秒）并输出美化的 JSON 结构
target/release/sprobe --once

# 仅执行瞬时采样（不等待，差分速率指标为 null）
target/release/sprobe --once --raw
```

### 步骤 3：启动本地 Mock 上报服务与 Agent 推送测试

仓库内置了功能完备的 Mock Ingest 收集端与端到端测试套件：

```bash
# 启动本地 Mock Ingest 服务（监听 9877 端口）
node scripts/mock_ingest.mjs 9877

# 在另一个终端启动 Sprobe，指定测试配置
target/release/sprobe -c target/e2e_m3_config.toml
```

观察终端输出，您将看到探针按周期采集、本地 WAL 记录、Gzip 压缩推送以及接收端成功确认日志。

### 步骤 4：启动 Console 控制台前端

```bash
cd console
npm install
npm run dev
```

浏览器打开 `http://localhost:5173`，即可体验与哪吒监控一致的服务器卡片网格、国旗徽标、月度流量配额进度条与网络双折线实时带宽图。

---

## 三、CLI 命令行工具完整参考

`sprobe` 提供了丰富的命令行管理工具与自检指令：

```text
用法: sprobe [选项]

选项:
  -c, --config <FILE>          指定配置文件路径 (亦可通过环境变量 SPROBE_CONFIG 传入)
      --check-config           校验配置文件合法性后退出 (成功退出码 0，错误退出码 2)
      --dry-run                校验配置并输出脱敏后的 TOML 配置内容后退出
      --once                   执行单次采样并输出快照 JSON 到 stdout 后退出
      --sample-window <SEC>    配合 --once 使用，两次快照的采样时间间隔（默认 1 秒，范围 1-10 秒）
      --raw                    配合 --once 使用，仅做瞬时采样（不等待，差分速率置为 null）
      --no-state               配合 --once 使用，禁止读写 state.json 运行时序号
      --diagnose               输出系统环境、权限、采集能力矩阵与脱敏诊断包
      --generate-systemd       生成适用于当前系统的标准 Linux systemd 服务单元模板
      --regenerate-id          重新生成并持久化本机的全局唯一 agent_id
      --install-service        [Windows 专属] 注册 Sprobe 为 Windows 后台服务
      --uninstall-service      [Windows 专属] 卸载 Sprobe Windows 后台服务
      --purge                  配合 --uninstall-service 使用，彻底清除数据与 WAL 目录
  -h, --help                   打印帮助信息
  -V, --version                打印版本信息
```

### 常用命令范例

```bash
# 1. 验证配置文件语法与字段有效性
sprobe -c /etc/sprobe/sprobe.toml --check-config

# 2. 查看脱敏后的实际生效配置
sprobe -c /etc/sprobe/sprobe.toml --dry-run

# 3. 输出全平台脱敏系统诊断报告（排障首选）
sprobe --diagnose

# 4. 生成 Systemd 启动配置文件并直接安装
sprobe -c /etc/sprobe/sprobe.toml --generate-systemd > /etc/systemd/system/sprobe.service
```

---

## 四、配置文件全参数详尽指南 (`sprobe.toml`)

默认配置文件搜索路径：
- **Linux**: `/etc/sprobe/sprobe.toml`
- **Windows**: `C:\ProgramData\Sprobe\config.toml`

以下为包含所有核心参数的生产级配置模板：

```toml
schema_version = "1.0"

# -----------------------------------------------------------------------------
# 1. 探针身份与标签 (Agent Meta)
# -----------------------------------------------------------------------------
[agent]
# 主机名，留空则自动读取操作系统主机名
hostname = "vps-us-prod-01"

# 节点元数据标签（前端 Console 将根据 region/country 自动匹配呈现国旗）
# 示例：US (美西/美国旗), CN (中国旗), JP (日本旗), HK (香港旗), SG (新加坡旗), DE (德国旗)
labels = { env = "production", region = "US-SiliconValley", group = "backend" }

# -----------------------------------------------------------------------------
# 2. 采集频率与指标控制 (Collect Settings)
# -----------------------------------------------------------------------------
[collect]
# 快速指标采集间隔（秒）：CPU、内存、网络吞吐、系统负载（建议 1~10 秒）
fast_interval_seconds = 5

# 慢速指标采集间隔（秒）：磁盘容量、挂载点、Top-N 进程榜单（建议 30~60 秒）
slow_interval_seconds = 30

# 采集超时限制（毫秒）
timeout_ms = 3000

# Top-N 进程采集配置
[collect.processes]
enabled = true
# 进程采集上限数量
max_count = 100
# 采集 Top-N 进程排序数量
top_n = 10
# 排序依据: "cpu" 或 "memory"
sort_by = "cpu"

# 磁盘与文件系统配置
[collect.filesystems]
# 忽略的虚拟文件系统类型
ignore_fstypes = ["tmpfs", "devtmpfs", "overlay", "squashfs", "iso9660"]
# 忽略的挂载路径前缀通配符
ignore_mountpoints = ["/dev/*", "/proc/*", "/sys/*", "/run/*", "/var/lib/docker/*"]

# -----------------------------------------------------------------------------
# 🌟 网络采集与哪吒特性优化 (Network & Nezha Enhancements)
# -----------------------------------------------------------------------------
[collect.network]
# 是否自动忽略虚拟网卡（默认为 true，自动剔除 /sys/devices/virtual/net 设备）
ignore_virtual = true

# 网卡包含白名单（支持通配符，非空时只统计匹配的网卡）
# include_interfaces = ["eth*", "ens*", "enp*"]
include_interfaces = []

# 网卡排除黑名单（支持通配符，默认过滤常见容器与虚拟网桥）
exclude_interfaces = [
    "docker*", "veth*", "br-*", "cni*", "flannel*", "kube-ipvs*",
    "cali*", "tun*", "tap*", "tailscale*", "wg*"
]

# 🌟 VPS 月结出账周期流量配额防超额配置 (Cycle Traffic)
[collect.network.cycle]
# 是否开启出账周期流量计算
enabled = true
# 每月出账起始日期（1~31，例如 1 代表每月 1 日 00:00:00 重置账期）
cycle_start_day = 1
# 月度出站流量限额（单位：字节，例如 1 TiB = 1099511627776）
monthly_outbound_limit_bytes = 1099511627776
# 出站流量高位警戒比例（默认 0.8，即消耗达到 80% 触发健康状态降级警告）
warning_ratio = 0.80

# -----------------------------------------------------------------------------
# 3. 本地离线持久化缓冲 (WAL - Write-Ahead Log)
# -----------------------------------------------------------------------------
[storage.wal]
# WAL 存储目录
dir = "/var/lib/sprobe/wal"
# WAL 最大占用磁盘空间（字节），超限后遵循 FIFO 自动丢弃最早的快照
max_bytes = 104857600   # 100 MiB
# 刷盘策略: "always" (每次快照 fsync), "every_sec" (每秒), "never" (依赖系统写入)
fsync = "every_sec"

# -----------------------------------------------------------------------------
# 4. 指标推送输出 (Push Output)
# -----------------------------------------------------------------------------
[output.push]
enabled = true
# 上报接收服务端点
endpoint = "https://monitor.example.com/api/v1/ingest"
# 鉴权 Token（优先读取环境变量 SPROBE_TOKEN，亦可指定文件路径）
token_file = "/etc/sprobe/token"
# 请求超时时间（毫秒）
timeout_ms = 5000
# 开启 Gzip 压缩推送（极大减少网络带宽占用，建议开启）
gzip = true
# 独立心跳保活端点
heartbeat_endpoint = "https://monitor.example.com/api/v1/heartbeat"
# 独立心跳上报间隔（秒）
heartbeat_interval_seconds = 10
# 批量推送配置
batch_size = 5
flush_interval_seconds = 5

# -----------------------------------------------------------------------------
# 5. Prometheus 与本地 HTTP 调试端点 (Pull Mode)
# -----------------------------------------------------------------------------
[output.prometheus]
enabled = true
# 监听地址（强烈建议绑定回环地址 127.0.0.1，如需外部抓取请配置 allow_remote = true）
listen = "127.0.0.1:9102"
path = "/metrics"
allow_remote = false

# -----------------------------------------------------------------------------
# 6. 本地健康检查规则与告警阈值 (Health Engine)
# -----------------------------------------------------------------------------
[health]
enabled = true
[health.thresholds]
# CPU 持续高使用率告警阈值
cpu_usage_ratio = 0.90
# 内存可用率不足告警阈值
memory_available_ratio = 0.05
# 磁盘使用率告警阈值
filesystem_usage_ratio = 0.90
```

---

## 五、生产环境部署实战

### 1. Linux Systemd 守护进程部署

#### 标准自动化安装

1. **放置二进制文件**：
   ```bash
   sudo cp target/release/sprobe /usr/local/bin/sprobe
   sudo chmod 755 /usr/local/bin/sprobe
   ```

2. **创建运行用户与工作目录**：
   ```bash
   sudo useradd -r -s /bin/false sprobe
   sudo mkdir -p /etc/sprobe /var/lib/sprobe/wal
   sudo chown -R sprobe:sprobe /var/lib/sprobe
   ```

3. **创建配置文件**：
   ```bash
   sudo cp sprobe.toml /etc/sprobe/sprobe.toml
   sudo chown -R root:sprobe /etc/sprobe
   sudo chmod 640 /etc/sprobe/sprobe.toml
   ```

4. **一键生成并启用 Systemd 服务**：
   ```bash
   # 利用 sprobe 内置命令直接生成加固后的 systemd unit 文件
   sprobe -c /etc/sprobe/sprobe.toml --generate-systemd | sudo tee /etc/systemd/system/sprobe.service

   # 重载并启动服务
   sudo systemctl daemon-reload
   sudo systemctl enable --now sprobe

   # 查看运行状态
   sudo systemctl status sprobe
   ```

5. **查看实时日志**：
   ```bash
   sudo journalctl -u sprobe -f -o cat
   ```

---

### 2. Windows 服务部署

Sprobe 原生支持作为 Windows 后台服务运行（支持开机自启、故障自动重启）：

1. 以**管理员身份**打开 PowerShell 或 CMD。
2. 将编译好的 `sprobe.exe` 和 `config.toml` 复制到安装目录，例如 `C:\Program Files\Sprobe\`。
3. 执行服务安装命令：
   ```powershell
   cd "C:\Program Files\Sprobe"
   .\sprobe.exe --install-service -c "C:\Program Files\Sprobe\config.toml"
   ```
4. 启动服务：
   ```powershell
   Start-Service sprobe
   # 检查服务状态
   Get-Service sprobe
   ```
5. 若需卸载服务：
   ```powershell
   # 卸载服务并清理 WAL 离线缓存
   .\sprobe.exe --uninstall-service --purge
   ```

---

### 3. Docker 容器化部署

当使用容器运行 Sprobe 时，由于需采集宿主机核心硬件指标，必须挂载宿主机的 procfs、sysfs 与 devfs：

```bash
docker run -d \
  --name sprobe \
  --restart always \
  --net host \
  --pid host \
  -v /:/rootfs:ro \
  -v /var/run/dbus/system_bus_socket:/var/run/dbus/system_bus_socket \
  -v /etc/sprobe/sprobe.toml:/etc/sprobe/sprobe.toml:ro \
  -v /var/lib/sprobe/wal:/var/lib/sprobe/wal:rw \
  sprobe:1.0
```

> **提示**：`--net host` 可确保获取宿主机真实的物理网卡速率与 IP 状态；`--pid host` 允许采集宿主机维度的 Top-N 真实进程清单。

---

### 4. Prometheus 抓取与监控集成

当在 `sprobe.toml` 中配置 `[output.prometheus]` 后，Sprobe 会启动轻量 HTTP 服务器，导出符合 Prometheus 0.0.4 规范的纯文本指标。

#### Prometheus 抓取配置示例 (`prometheus.yml`)

```yaml
scrape_configs:
  - job_name: 'sprobe_nodes'
    scrape_interval: 10s
    metrics_path: '/metrics'
    static_configs:
      - targets: ['127.0.0.1:9102']
        labels:
          node: 'vps-us-prod-01'
          region: 'US'
```

#### 导出核心指标概览

| Prometheus 指标名 | 类型 | 说明 |
| :--- | :--- | :--- |
| `sprobe_cpu_usage_ratio` | Gauge | 全局 CPU 综合使用率 (0.0 ~ 1.0) |
| `sprobe_memory_used_bytes` | Gauge | 物理内存已用字节数 |
| `sprobe_memory_available_bytes` | Gauge | 系统可用内存字节数 |
| `sprobe_filesystem_used_bytes` | Gauge | 挂载点磁盘已用字节数（包含 `mountpoint` 标签） |
| `sprobe_network_receive_bytes_total` | Counter | 物理网卡累计下行接收字节数 |
| `sprobe_network_transmit_bytes_total` | Counter | 物理网卡累计上行发送字节数 |
| `sprobe_network_cycle_outbound_bytes` | Gauge | **当前月度出账周期累计出站流量字节数** |
| `sprobe_process_count` | Gauge | 当前系统存活总进程数 |

---

## 六、Console 控制台前端部署与使用

Sprobe Console 专为提供媲美哪吒监控（Nezha）的流畅 VPS 监控体验而设计，同时保持极限的轻量化（打包产物仅 ~61 KiB gzip）。

### 1. 视图模式与操作说明

- **卡片网格模式 (Cards Mode)**：
  - 点击右上角切换按钮切为卡片模式。
  - 每台主机呈独立卡片，包含所属国家/地区国旗徽标、实时在线状态、CPU/内存/磁盘 3 大硬件负荷进度条、**月结流量配额防超额消耗条**以及实时上下行带宽瞬时值。
- **列表表格模式 (Table Mode)**：
  - 适合大规模集群批量运维，一屏查看 50+ 节点核心指标，支持按操作系统、运行状态及关键字即时过滤检索。
- **节点详情页**：
  - 包含月度流量出账配额监控卡片（带 80% 告警线与自动扣除虚拟网卡过滤提示）。
  - 双折线 SVG 动态面积走势图（Rx 蓝线 vs Tx 绿线），直观呈现突发流量脉冲。
  - 物理接口与虚拟设备细分列表，展示每个网卡的 UP/DOWN 状态与丢包/错包计数。

### 2. 生产环境构建与 Nginx 反向代理

1. **构建前端产物**：
   ```bash
   cd console
   npm run build
   # 生成的纯静态产物位于 console/dist 目录
   ```

2. **Nginx 反向代理配置参考**：
   ```nginx
   server {
       listen 80;
       server_name monitor.example.com;

       # Console 前端静态资源
       location / {
           root /var/www/sprobe-console/dist;
           index index.html;
           try_files $uri $uri/ /index.html;
       }

       # 后端 Ingest 上报接口与只读查询 API 转发
       location /api/ {
           proxy_pass http://127.0.0.1:9877;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       }
   }
   ```

---

## 七、运维诊断与常见问题 (FAQ)

### Q1: 如何快速排查探针为什么无法正常上报？

执行一键脱敏诊断命令：
```bash
sprobe -c /etc/sprobe/sprobe.toml --diagnose
```
Sprobe 会按检查项依次列出：
- 操作系统版本、内核、架构与虚拟化环境；
- 各模块数据采集权限是否正常；
- WAL 离线队列目录是否存在未发送快照积压；
- 上报端 HTTP 连通性测试与错误码。

### Q2: 宿主机存在大量 Docker/K8s 容器，导致网络流量虚高翻倍怎么办？

Sprobe 默认开启了虚拟网卡过滤。请检查 `sprobe.toml` 中的 `[collect.network]` 配置：
```toml
[collect.network]
ignore_virtual = true
exclude_interfaces = ["docker*", "veth*", "br-*", "cni*"]
```
开启后，容器生成的虚拟网桥将仅记录在详情接口明细中，**不会累加进全局带宽与月结流量配额**。

### Q3: 为什么健康状态显示为 `Degraded`（降级）？

当发生以下情况时，探针会自动触发降级，并在 Console 详情顶部弹出警报横幅：
1. **月度出站流量达到 80% 水位**（`traffic_quota` 检查项报警）；
2. **CPU 或磁盘使用率达到 90%**；
3. **上报连续发生鉴权失败（401）或网络中断导致 WAL 队列持续积压**。

您可在 Console 节点详情页下方的“**健康诊断规则与探针能力矩阵**”抽屉中查看具体的触发规则与阈值观测值。

### Q4: 本地断网时，数据会丢失吗？

**不会**。Sprobe 内置持久化 WAL 机制。当网络断开时，快照将严格按时间戳写入磁盘 WAL 队列。网络恢复连通后，探针将在后台自动分批重新推送，并保证幂等上报不重不漏。

---

*文档版本：v1.0 (2026-11-21) | 适配组件：Sprobe Agent 1.0.x / Console 1.0.x*
