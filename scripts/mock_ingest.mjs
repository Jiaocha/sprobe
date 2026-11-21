#!/usr/bin/env node
// Sprobe Mock Ingest Server (任务 D-04)
// 用于本地端到端与推送链路联调，完全无需依赖公网云服务

import http from 'node:http'
import zlib from 'node:zlib'
import crypto from 'node:crypto'

const PORT = parseInt(process.env.PORT || process.argv[2] || '9876', 10)
const HOST = '127.0.0.1'

// 已接收批次集合，用于幂等测试：key = `${agent_id}:${instance_id}:${batch_id}`
const receivedBatches = new Set()
// 最新快照缓存：agent_id -> snapshot
const latestSnapshots = new Map()

function sendJson(res, statusCode, body) {
  const jsonStr = JSON.stringify(body, null, 2)
  res.writeHead(statusCode, {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(jsonStr),
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization, Content-Encoding, X-Mock-Status',
    'X-Sprobe-Mock': 'true',
  })
  res.end(jsonStr)
}

function sendError(res, statusCode, code, message, retryable = false) {
  const requestId = `req_mock_${crypto.randomBytes(4).toString('hex')}`
  const errorBody = {
    error: {
      code,
      message,
      retryable,
      request_id: requestId,
      server_time: new Date().toISOString(),
    },
  }
  sendJson(res, statusCode, errorBody)
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://${req.headers.host || HOST}`)
  console.log(`[Mock Ingest] ${req.method} ${url.pathname}`)

  // 1. 允许显式故障注入（通过请求头 X-Mock-Status）
  const mockStatusHeader = req.headers['x-mock-status']
  if (mockStatusHeader) {
    const mockCode = parseInt(mockStatusHeader, 10)
    console.log(`[Mock Ingest] 触发注入故障状态码: ${mockCode}`)
    switch (mockCode) {
      case 400:
        return sendError(res, 400, 'invalid_json', '注入故障: 请求体 JSON 解析失败')
      case 401:
        return sendError(res, 401, 'unauthorized', '注入故障: 推送令牌无效或已过期')
      case 403:
        return sendError(res, 403, 'forbidden', '注入故障: 租户不匹配或已被禁止')
      case 409:
        return sendError(res, 409, 'conflict', '注入故障: 批次数据发生冲突')
      case 413:
        return sendError(res, 413, 'payload_too_large', '注入故障: 批次数据大小超出上限')
      case 429:
        res.setHeader('Retry-After', '5')
        return sendError(res, 429, 'rate_limited', '注入故障: 上报频率超限', true)
      case 500:
      default:
        return sendError(res, 500, 'internal', '注入故障: 服务端内部异常', true)
    }
  }

  // 2. 跨域预检与路由处理
  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization, Content-Encoding, X-Mock-Status',
    })
    return res.end()
  }

  if (req.method === 'GET' && url.pathname === '/api/readonly/overview') {
    const now = Date.now()
    const items = []

    for (const [agent_id, snap] of latestSnapshots.entries()) {
      const lastSeenMs = new Date(snap.collected_at).getTime()
      const isOnline = now - lastSeenMs < 90_000

      const cpuRatio = snap.cpu?.total?.usage_ratio ?? null
      const memRatio = snap.memory?.usage_ratio ?? null
      const load1m = snap.load?.average_1m ?? null
      const rootFs = snap.filesystems?.items?.find((fs) => fs.mountpoint === '/' || fs.mountpoint === 'C:\\')
      const diskRatio = rootFs ? rootFs.usage_ratio : snap.filesystems?.items?.[0]?.usage_ratio ?? null

      items.push({
        agent_id,
        hostname: snap.agent?.hostname || snap.host?.hostname || 'unknown-host',
        display_name: snap.agent?.hostname || snap.host?.hostname || 'unknown-host',
        os: snap.agent?.os || 'linux',
        arch: snap.agent?.arch || 'x86_64',
        labels: snap.agent?.labels || {},
        health_status: snap.health?.status || 'ok',
        is_online: isOnline,
        last_seen: snap.collected_at,
        cpu_usage_ratio: isOnline ? cpuRatio : null,
        memory_usage_ratio: isOnline ? memRatio : null,
        disk_usage_ratio: isOnline ? diskRatio : null,
        load_1m: isOnline ? load1m : null,
        network_rx_bytes_per_sec: isOnline ? 1024 * 150 : null,
        network_tx_bytes_per_sec: isOnline ? 1024 * 80 : null,
      })
    }

    return sendJson(res, 200, items)
  }

  if (req.method === 'GET' && url.pathname.startsWith('/api/readonly/nodes/')) {
    const agentId = url.pathname.replace('/api/readonly/nodes/', '').trim()
    const snap = latestSnapshots.get(agentId)
    if (!snap) {
      return sendError(res, 404, 'node_not_found', `未找到节点: ${agentId}`)
    }
    const now = new Date()
    const lastSeenMs = new Date(snap.collected_at).getTime()
    const isOnline = now.getTime() - lastSeenMs < 90_000

    return sendJson(res, 200, {
      schema_version: '1.0',
      generated_at: now.toISOString(),
      view: 'full',
      agent_runtime: {
        online: isOnline,
        last_push_at: snap.collected_at,
        last_heartbeat_at: snap.collected_at,
        wal_snapshots: 0,
      },
      snapshot: snap,
    })
  }

  if (req.method === 'POST' && url.pathname === '/api/v1/heartbeat') {
    // 心跳鉴权校验
    const authHeader = req.headers['authorization']
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return sendError(res, 401, 'unauthorized', 'Heartbeat 缺少有效的 Bearer 令牌')
    }

    return sendJson(res, 200, {
      schema_version: '1.0',
      status: 'accepted',
      server_time: new Date().toISOString(),
    })
  }

  if (req.method === 'POST' && url.pathname === '/api/v1/ingest') {
    // 鉴权检查
    const authHeader = req.headers['authorization']
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return sendError(res, 401, 'unauthorized', 'Ingest 缺少有效的 Bearer 令牌')
    }

    // 读取请求体并支持 gzip 解压
    const chunks = []
    req.on('data', (chunk) => chunks.push(chunk))

    req.on('end', () => {
      const rawBuffer = Buffer.concat(chunks)

      // 检查解压前大小（单批不能超过 1 MiB）
      if (rawBuffer.length > 1048576) {
        return sendError(res, 413, 'payload_too_large', '压缩后请求体超出 1 MiB 限制')
      }

      let jsonBuffer = rawBuffer
      const contentEncoding = req.headers['content-encoding']
      if (contentEncoding === 'gzip') {
        try {
          jsonBuffer = zlib.gunzipSync(rawBuffer)
        } catch (err) {
          return sendError(res, 400, 'invalid_gzip', `Gzip 解包失败: ${err.message}`)
        }
      }

      let batch
      try {
        batch = JSON.parse(jsonBuffer.toString('utf-8'))
      } catch (err) {
        return sendError(res, 400, 'invalid_json', `JSON 语法解析失败: ${err.message}`)
      }

      // 契约必填检查
      if (!batch.batch_id || !batch.agent_id || !batch.instance_id || !Array.isArray(batch.snapshots)) {
        return sendError(res, 422, 'validation_failed', '批次缺失 batch_id, agent_id, instance_id 或 snapshots')
      }

      const snapshotCount = batch.snapshots.length
      const idempotentKey = `${batch.agent_id}:${batch.instance_id}:${batch.batch_id}`

      // 幂等性处理：重复上报返回 duplicates
      if (receivedBatches.has(idempotentKey)) {
        console.log(`[Mock Ingest] 命中批次幂等重复: ${idempotentKey}`)
        return sendJson(res, 200, {
          schema_version: '1.0',
          status: 'accepted',
          batch_id: batch.batch_id,
          accepted: 0,
          duplicates: snapshotCount,
          rejected: [],
          server_time: new Date().toISOString(),
        })
      }

      receivedBatches.add(idempotentKey)
      if (batch.snapshots.length > 0) {
        latestSnapshots.set(batch.agent_id, batch.snapshots[batch.snapshots.length - 1])
      }
      console.log(`[Mock Ingest] 成功接收批次 ${batch.batch_id}，包含 ${snapshotCount} 个快照`)

      return sendJson(res, 200, {
        schema_version: '1.0',
        status: 'accepted',
        batch_id: batch.batch_id,
        accepted: snapshotCount,
        duplicates: 0,
        rejected: [],
        server_time: new Date().toISOString(),
      })
    })

    return
  }

  // 404
  return sendError(res, 404, 'not_found', `端点不存在: ${req.method} ${url.pathname}`)
})

server.listen(PORT, HOST, () => {
  console.log(`[Mock Ingest] 模拟上报服务已启动: http://${HOST}:${PORT}`)
  console.log(`[Mock Ingest] 支持接口: POST /api/v1/ingest, POST /api/v1/heartbeat`)
  console.log(`[Mock Ingest] 支持 Header 故障注入: X-Mock-Status: 401 | 409 | 413 | 429 | 500`)
})
