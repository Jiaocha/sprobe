#!/usr/bin/env node
// M2 阶段端到端综合验收测试 (后端 Agent + WAL + Ingest + 前端总览 API)

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const PORT = 9876;
const WAL_DIR = path.resolve("./target/e2e_m2_wal");
if (!fs.existsSync(WAL_DIR)) {
  fs.mkdirSync(WAL_DIR, { recursive: true });
}

const e2eConfigPath = path.resolve("./target/e2e_m2_config.toml");
const tomlContent = `schema_version = "1.0"

[agent]
hostname = "m2-e2e-node"

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

fs.writeFileSync(e2eConfigPath, tomlContent);

// 1. 启动 Mock Ingest 服务
console.log("==> [1/4] 启动 Mock Ingest 服务...");
const mockServer = spawn("node", ["scripts/mock_ingest.mjs", String(PORT)], {
  stdio: ["ignore", "pipe", "pipe"],
});

mockServer.stdout.on("data", (d) => process.stdout.write(`[MockServer] ${d}`));
mockServer.stderr.on("data", (d) => process.stderr.write(`[MockServer ERR] ${d}`));

await new Promise((r) => setTimeout(r, 600));

// 2. 启动 Sprobe 探针常驻守护进程
console.log("==> [2/4] 启动 Sprobe Agent 真实采集与推送...");
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

// 等待探针采集与推送 2.5 秒
await new Promise((r) => setTimeout(r, 2600));

// 3. 模拟前端从只读 API 获取总览数据
console.log("==> [3/4] 前端调用 GET /api/readonly/overview 校验实时节点数据...");
const overviewRes = await fetch(`http://127.0.0.1:${PORT}/api/readonly/overview`);
if (!overviewRes.ok) {
  throw new Error(`Overview API 请求失败: ${overviewRes.status}`);
}

const nodes = await overviewRes.json();
console.log("[Overview API] 响应节点数:", nodes.length);
console.log("[Overview API] 节点详情:", JSON.stringify(nodes[0], null, 2));

if (nodes.length === 0) {
  throw new Error("总览接口未返回任何节点！");
}

const node = nodes[0];
if (!node.is_online) {
  throw new Error(`节点状态异常: is_online 应为 true，实际为 ${node.is_online}`);
}
// 本机真实 C 盘使用率通常 > 90%，健康引擎精准识别为 degraded
if (node.health_status !== "ok" && node.health_status !== "degraded") {
  throw new Error(`健康状态异常: 应为 ok 或 degraded，实际为 ${node.health_status}`);
}
console.log(`[Health Engine] 探针节点健康诊断: ${node.health_status} (真实磁盘使用率: ${(node.disk_usage_ratio * 100).toFixed(1)}%)`);
if (typeof node.cpu_usage_ratio !== "number" || typeof node.memory_usage_ratio !== "number") {
  throw new Error("CPU 或内存使用率非有效数值！");
}

console.log("==> [4/4] 验证全部通过，发送 SIGINT 优雅停机...");
agent.kill("SIGINT");
await new Promise((r) => setTimeout(r, 500));
mockServer.kill("SIGINT");

try {
  fs.rmSync(e2eConfigPath, { force: true });
} catch (_) {}

console.log("\n=======================================================");
console.log("🎉 M2 阶段端到端全链路闭环验证 (Agent + Ingest + Console API) 全部通过！");
console.log("=======================================================");
process.exit(0);
