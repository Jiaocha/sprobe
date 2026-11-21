#!/usr/bin/env bash
# ==============================================================================
# Sprobe Agent 一键安装与服务配置脚本 (Sprobe Fast Installer)
# 支持 Debian, Ubuntu, CentOS, Rocky Linux, AlmaLinux, Alpine (x86_64, aarch64)
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}================================================================${NC}"
echo -e "${PURPLE}           Sprobe 极轻量高精度服务器探针 - 一键接入向导           ${NC}"
echo -e "${CYAN}================================================================${NC}"

# 默认参数
ENDPOINT="https://sprobe-console.pages.dev/api/v1/ingest"
TOKEN=""
HOSTNAME_VAL=$(hostname)
INTERVAL=10
REPO_SLUG="Jiaocha/sprobe"

# 解析命令行参数
while [[ $# -gt 0 ]]; do
  case $1 in
    --endpoint)
      ENDPOINT="$2"
      shift 2
      ;;
    --token)
      TOKEN="$2"
      shift 2
      ;;
    --name|--hostname)
      HOSTNAME_VAL="$2"
      shift 2
      ;;
    --interval)
      INTERVAL="$2"
      shift 2
      ;;
    --repo)
      REPO_SLUG="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

# 权限校验
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}[ERROR] 请使用 root 用户或 sudo 执行此安装脚本。${NC}"
  exit 1
fi

# 检查系统架构
ARCH=$(uname -m)
case $ARCH in
  x86_64|amd64)
    ARCH_TARGET="x86_64-unknown-linux-musl"
    ARCH_FALLBACK="x86_64-linux"
    ;;
  aarch64|arm64)
    ARCH_TARGET="aarch64-unknown-linux-musl"
    ARCH_FALLBACK="aarch64-linux"
    ;;
  *)
    echo -e "${RED}[ERROR] 暂不支持此 CPU 架构: ${ARCH}${NC}"
    exit 1
    ;;
esac

echo -e "${BLUE}[1/5] 系统架构识别:${NC} ${ARCH} (${ARCH_TARGET})"

# 检查基础依赖
command -v curl >/dev/null 2>&1 || (apt-get update && apt-get install -y curl || yum install -y curl || apk add curl)
command -v tar >/dev/null 2>&1 || (apt-get install -y tar || yum install -y tar || apk add tar)

# 创建 sprobe 系统用户与目录
echo -e "${BLUE}[2/5] 创建独立非特权运行用户与数据目录...${NC}"
id sprobe >/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin sprobe 2>/dev/null || adduser -S -s /sbin/nologin sprobe 2>/dev/null || true
mkdir -p /etc/sprobe /var/lib/sprobe/wal /var/log/sprobe /usr/local/bin
chown -R sprobe:sprobe /var/lib/sprobe /var/log/sprobe 2>/dev/null || true

# 下载或安装二进制
echo -e "${BLUE}[3/5] 部署 Sprobe 探针二进制...${NC}"
BIN_PATH="/usr/local/bin/sprobe"
TEMP_DIR=$(mktemp -d)

# 尝试从 GitHub Releases 拉取压缩包或直接二进制
DOWNLOAD_PKG_URL="https://github.com/${REPO_SLUG}/releases/latest/download/sprobe-${ARCH_TARGET}.tar.gz"
DOWNLOAD_RAW_URL="https://github.com/${REPO_SLUG}/releases/latest/download/sprobe-${ARCH_FALLBACK}"

echo -e "正在拉取最新发行版: ${DOWNLOAD_PKG_URL}"
if curl -fsSL --connect-timeout 15 -o "${TEMP_DIR}/sprobe.tar.gz" "${DOWNLOAD_PKG_URL}" 2>/dev/null; then
  tar -xzf "${TEMP_DIR}/sprobe.tar.gz" -C "${TEMP_DIR}"
  if [ -f "${TEMP_DIR}/sprobe" ]; then
    mv -f "${TEMP_DIR}/sprobe" "$BIN_PATH"
  fi
elif curl -fsSL --connect-timeout 15 -o "$BIN_PATH" "${DOWNLOAD_RAW_URL}" 2>/dev/null; then
  echo -e "${GREEN}成功拉取二进制文件${NC}"
else
  if [ -f "$BIN_PATH" ]; then
    echo -e "${YELLOW}[INFO] 未从公网拉取到新包，保留现有安装: ${BIN_PATH}${NC}"
  else
    echo -e "${RED}[ERROR] 未能下载探针二进制，请检查网络或 GitHub Release。${NC}"
    rm -rf "${TEMP_DIR}"
    exit 1
  fi
fi
rm -rf "${TEMP_DIR}"
chmod 755 "$BIN_PATH"

# 写入配置文件
echo -e "${BLUE}[4/5] 写入配置文件: /etc/sprobe/sprobe.toml...${NC}"
cat <<EOF > /etc/sprobe/sprobe.toml
schema_version = "1.0"

[agent]
hostname = "${HOSTNAME_VAL}"
labels = { group = "Default", provider = "Auto-Install" }

[collect]
fast_interval_seconds = ${INTERVAL}
slow_interval_seconds = 30
timeout_ms = 3000

[collect.processes]
enabled = true
max_count = 100
top_n = 10
sort_by = "cpu"

[collect.network]
ignore_virtual = true

[collect.storage]
mount_allowlist = ["/", "/home", "/var"]

[pipeline]
flush_interval_ms = 1000
batch_size = 50
worker_threads = 2

[pipeline.circuit_breaker]
consecutive_failures = 5
cooldown_seconds = 30
half_open_max_trials = 3

[wal]
enabled = true
dir = "/var/lib/sprobe/wal"
max_bytes = 104857600
fsync = "batch"

[output.push]
enabled = true
endpoint = "${ENDPOINT}"
format = "json"
timeout_ms = 5000
compression = "none"
auth_header = "Authorization"
auth_token = "${TOKEN}"
EOF

chmod 644 /etc/sprobe/sprobe.toml

# 配置 systemd 服务
if command -v systemctl >/dev/null 2>&1; then
  echo -e "${BLUE}[5/5] 配置并启动 systemd 系统守护进程...${NC}"
  cat <<'EOF' > /etc/systemd/system/sprobe.service
[Unit]
Description=Sprobe Agent - Server Monitoring Daemon
After=network.target

[Service]
Type=simple
User=sprobe
Group=sprobe
ExecStart=/usr/local/bin/sprobe --config /etc/sprobe/sprobe.toml
Restart=always
RestartSec=5s
LimitNOFILE=65535
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl unmask sprobe 2>/dev/null || true
  systemctl enable sprobe
  systemctl restart sprobe

  echo -e "${GREEN}================================================================${NC}"
  echo -e "${GREEN} Sprobe 探针安装成功！服务状态检查:${NC}"
  echo -e "${GREEN}================================================================${NC}"
  systemctl status sprobe --no-pager || true
else
  echo -e "${YELLOW}[WARN] 非 systemd 系统，请手动以后台进程运行 /usr/local/bin/sprobe --config /etc/sprobe/sprobe.toml${NC}"
fi
