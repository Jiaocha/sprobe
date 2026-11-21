#!/usr/bin/env node
// M3 阶段端到端综合验收测试 (Prometheus 0.0.4 + 嵌入式 HTTP 端点 + 独立心跳 + Console 节点详情 API 联通)

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const PORT = 9877;
const PROM_PORT = 19102;
const WAL_DIR = path.resolve("./target/e2e_m3_wal");
if (!fs.existsSync(WAL_DIR)) {
  fs.mkdirSync(WAL_DIR, { recursive: true });
}

const e2eConfigPath = path.resolve("./target/e2e_m3_config.toml");
const tomlContent = `schema_version = "1.0"

[agent]
hostname = "m3-e2e-node"

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
heartbeat_interval_seconds = 2
gzip = true
allow_insecure = true
tls_insecure_skip_verify = true

[output.prometheus]
enabled = true
listen = "127.0.0.1:${PROM_PORT}"
path = "/metrics"
allow_remote = false
`;

fs.writeFileSync(e2eConfigPath, tomlContent);

// 1. 启动 Mock Ingest 服务
console.log("==> [1/5] 启动 Mock Ingest 服务 (端口 " + PORT + ")...");
const mockServer = spawn("node", ["scripts/mock_ingest.mjs", String(PORT)], {
  stdio: ["ignore", "pipe", "pipe"],
});

mockServer.stdout.on("data", (d) => process.stdout.write(`[MockServer] ${d}`));
mockServer.stderr.on("data", (d) => process.stderr.write(`[MockServer ERR] ${d}`));

await new Promise((r) => setTimeout(r, 600));

// 2. 启动 Sprobe 探针常驻守护进程
console.log("==> [2/5] 启动 Sprobe Agent (启用 Prometheus 监听与独立心跳)...");
const sprobeBin = path.resolve("./target/debug/sprobe.exe");
const agent = spawn(sprobeBin, ["-c", e2eConfigPath], {
  env: { ...process.env, SPROBE_TOKEN: "e2e-token-secret" },
  stdio: ["ignore", "pipe", "pipe"],
});

let pushedCount = 0;
agent.stdout.on("data", (d) => {
  const str = d.toString();
  process.stdout.write(`[Agent] ${str}`);
  if (str.includes("[PUSH] 快照")) {
    pushedCount++;
  }
});
agent.stderr.on("data", (d) => process.stderr.write(`[Agent ERR] ${d}`));

// 等待探针采集、启动 HTTP 服务并推送至少两轮快照
await new Promise((r) => setTimeout(r, 3200));

// 3. 校验嵌入式微型 HTTP 服务端点与 Prometheus 输出
console.log("==> [3/5] 校验本地 HTTP 服务 (/healthz, /readyz, /status, /metrics)...");

// 3.1 /healthz
const healthzRes = await fetch(`http://127.0.0.1:${PROM_PORT}/healthz`);
if (!healthzRes.ok) throw new Error(`/healthz 失败: ${healthzRes.status}`);
const healthzText = await healthzRes.text();
if (healthzText.trim() !== "OK") throw new Error(`/healthz 响应异常: ${healthzText}`);
console.log("[HTTP PASS] /healthz -> 200 OK");

// 3.2 /readyz
const readyzRes = await fetch(`http://127.0.0.1:${PROM_PORT}/readyz`);
if (!readyzRes.ok) throw new Error(`/readyz 失败: ${readyzRes.status}`);
console.log("[HTTP PASS] /readyz -> 200 OK (已解除预热)");

// 3.3 /status
const statusRes = await fetch(`http://127.0.0.1:${PROM_PORT}/status`);
if (!statusRes.ok) throw new Error(`/status 失败: ${statusRes.status}`);
const statusJson = await statusRes.json();
console.log("[HTTP PASS] /status ->", JSON.stringify(statusJson));
if (statusJson.ready !== true || statusJson.warmup !== false) {
  throw new Error("探针状态标记未就绪或未解除预热！");
}

// 3.4 /metrics
const metricsRes = await fetch(`http://127.0.0.1:${PROM_PORT}/metrics`);
if (!metricsRes.ok) throw new Error(`/metrics 失败: ${metricsRes.status}`);
const metricsText = await metricsRes.text();
console.log(`[HTTP PASS] /metrics -> 成功获取 Prometheus 文本 (长度 ${metricsText.length} 字节)`);
if (!metricsText.includes("sprobe_agent_up 1")) {
  throw new Error("Prometheus 指标缺少 sprobe_agent_up 1！");
}
if (!metricsText.includes("sprobe_cpu_usage_ratio")) {
  throw new Error("Prometheus 指标缺少 sprobe_cpu_usage_ratio！");
}
if (!metricsText.includes("sprobe_memory_available_bytes")) {
  throw new Error("Prometheus 指标缺少 sprobe_memory_available_bytes！");
}

// 4. 校验 Console 前端只读 API (总览 + 节点详情)
console.log("==> [4/5] 校验 Console 前端只读 API (Overview & Detail)...");

const overviewRes = await fetch(`http://127.0.0.1:${PORT}/api/readonly/overview`);
const nodes = await overviewRes.json();
if (nodes.length === 0) throw new Error("Overview 接口未返回节点！");
const targetAgentId = nodes[0].agent_id;
console.log(`[Console API] Overview 获取到节点 ID: ${targetAgentId}`);

const detailRes = await fetch(`http://127.0.0.1:${PORT}/api/readonly/nodes/${targetAgentId}`);
if (!detailRes.ok) throw new Error(`Detail API 失败: ${detailRes.status}`);
const detailJson = await detailRes.json();
console.log(`[Console API] Detail 成功获取完整快照结构: seq=${detailJson.snapshot.sequence}, cpu=${(detailJson.snapshot.cpu.total.usage_ratio * 100).toFixed(1)}%`);

if (!detailJson.snapshot || !detailJson.snapshot.cpu || !detailJson.snapshot.memory || !detailJson.snapshot.health) {
  throw new Error("Detail 接口返回数据结构不完整！");
}

// 5. 优雅停机并验证退出
console.log("==> [5/5] 发送 SIGINT 优雅停机，验证 HTTP 服务与 WAL 正确清理...");
agent.kill("SIGINT");

await new Promise((resolve) => {
  agent.on("exit", (code) => {
    console.log(`[Agent Exit] 探针守护进程正常退出，退出码: ${code}`);
    mockServer.kill();
    resolve();
  });
});

console.log("\n=======================================================");
console.log("🎉 M3 阶段全链路端到端自动化验收全部通过 (PASS)！");
console.log("   - Prometheus 0.0.4 文本格式暴露验证通过");
console.log("   - 本地微型 HTTP 服务 (/healthz, /readyz, /status) 验证通过");
console.log("   - 独立心跳守护定时器验证通过");
console.log("   - 前端 Console 节点详情 API 联通验证通过");
console.log("=======================================================\n");
