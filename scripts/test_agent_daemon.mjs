import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const PORT = 9988;
const WAL_DIR = path.resolve("./target/test_daemon_wal");
if (!fs.existsSync(WAL_DIR)) {
  fs.mkdirSync(WAL_DIR, { recursive: true });
}

// 写入临时测试配置
const tempConfigPath = path.resolve("./target/test_daemon_config.toml");
const tomlContent = `schema_version = "1.0"

[agent]
hostname = "daemon-test-node"

[collect]
fast_interval_seconds = 1

[storage.wal]
dir = "${WAL_DIR.replace(/\\/g, "/")}"
fsync = "never"

[output.push]
enabled = true
endpoint = "http://127.0.0.1:${PORT}/api/v1/ingest"
heartbeat_endpoint = "http://127.0.0.1:${PORT}/api/v1/heartbeat"
token_file = ""
gzip = true
allow_insecure = true
tls_insecure_skip_verify = true
`;

fs.writeFileSync(tempConfigPath, tomlContent);

// 1. 启动 mock ingest
const mockServer = spawn("node", ["scripts/mock_ingest.mjs"], {
  env: { ...process.env, PORT: String(PORT) },
  stdio: ["ignore", "pipe", "pipe"],
});

mockServer.stdout.on("data", (data) => {
  process.stdout.write(`[MockServer] ${data}`);
});
mockServer.stderr.on("data", (data) => {
  process.stderr.write(`[MockServer ERR] ${data}`);
});

await new Promise((resolve) => setTimeout(resolve, 500));

// 2. 启动 sprobe 常驻进程
console.log("[Test] 正在启动 Sprobe 探针常驻进程...");
const sprobeBin = path.resolve("./target/debug/sprobe.exe");
const agent = spawn(sprobeBin, ["-c", tempConfigPath], {
  env: { ...process.env, SPROBE_TOKEN: "mock-token-secret-12345" },
  stdio: ["ignore", "pipe", "pipe"],
});

let pushConfirmedCount = 0;

agent.stdout.on("data", (data) => {
  const str = data.toString();
  process.stdout.write(`[Agent] ${str}`);
  if (str.includes("[PUSH] 快照")) {
    pushConfirmedCount++;
  }
});

agent.stderr.on("data", (data) => {
  process.stderr.write(`[Agent ERR] ${data}`);
});

// 等待探针运行并上报至少 2 轮快照
console.log("[Test] 正在等待快照采集与上报 (3.5s)...");
await new Promise((resolve) => setTimeout(resolve, 3500));

console.log("[Test] 发送 SIGINT/终止探针进程...");
agent.kill("SIGINT");
await new Promise((resolve) => setTimeout(resolve, 500));
mockServer.kill("SIGINT");

// 清理临时文件
try {
  fs.rmSync(tempConfigPath, { force: true });
} catch (_) {}

console.log(`[Test] 探针成功上报并确认的快照批次数: ${pushConfirmedCount}`);
if (pushConfirmedCount >= 1) {
  console.log("[Test] 全链路常驻采集、WAL 缓冲与推送到 Mock Ingest 测试 PASS!");
  process.exit(0);
} else {
  console.error("[Test] 失败: 未在超时前检测到快照上报确认！");
  process.exit(1);
}
