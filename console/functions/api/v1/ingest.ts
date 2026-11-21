// Cloudflare Pages Functions: POST /api/v1/ingest (形态 B)
// 对齐 docs/frontend/sprobe_v1_2.md 第 7 章与契约 v1.0

interface Env {
  DB: D1Database
  INGEST_TOKEN?: string
}

function jsonResponse(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      'Content-Type': 'application/json',
      'X-Content-Type-Options': 'nosniff',
    },
  })
}

function errorResponse(code: string, message: string, status: number, retryable = false): Response {
  const requestId = `req_${crypto.randomUUID().slice(0, 8)}`
  return jsonResponse(
    {
      error: {
        code,
        message,
        retryable,
        request_id: requestId,
        server_time: new Date().toISOString(),
      },
    },
    status
  )
}

export const onRequestPost: PagesFunction<Env> = async (context) => {
  const { request, env } = context

  // 1. Bearer Token 校验
  const authHeader = request.headers.get('Authorization')
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return errorResponse('unauthorized', '缺少有效的 Bearer 认证令牌', 401)
  }

  const token = authHeader.slice(7).trim()
  if (env.INGEST_TOKEN && token !== env.INGEST_TOKEN) {
    return errorResponse('unauthorized', '推送令牌无效', 401)
  }

  // 2. 解包 gzip 或普通 json
  let bodyText: string
  const contentEncoding = request.headers.get('Content-Encoding')

  try {
    if (contentEncoding === 'gzip') {
      const decompressedStream = request.body?.pipeThrough(new DecompressionStream('gzip'))
      if (!decompressedStream) {
        return errorResponse('invalid_gzip', '无法解压 gzip 数据流', 400)
      }
      const response = new Response(decompressedStream)
      bodyText = await response.text()
    } else {
      bodyText = await request.text()
    }
  } catch (err: any) {
    return errorResponse('invalid_gzip', `Gzip 解包失败: ${err.message}`, 400)
  }

  // 3. 校验单批解压后上限（256 KiB 推荐，最大 1 MiB）
  if (bodyText.length > 1048576) {
    return errorResponse('payload_too_large', '解压后快照批次超出 1 MiB 限制', 413)
  }

  let batch: any
  try {
    batch = JSON.parse(bodyText)
  } catch (err: any) {
    return errorResponse('invalid_json', `JSON 语法解析失败: ${err.message}`, 400)
  }

  if (
    !batch.batch_id ||
    !batch.agent_id ||
    !batch.instance_id ||
    !Array.isArray(batch.snapshots) ||
    batch.snapshots.length === 0
  ) {
    return errorResponse('validation_failed', '快照批次缺失必要字段或快照列表为空', 422)
  }

  const nowIso = new Date().toISOString()
  const snapshotCount = batch.snapshots.length

  // 4. 批次级幂等性检查（查询 D1）
  try {
    const existing = await env.DB.prepare(
      'SELECT status FROM ingest_batches WHERE agent_id = ? AND instance_id = ? AND batch_id = ?'
    )
      .bind(batch.agent_id, batch.instance_id, batch.batch_id)
      .first()

    if (existing && existing.status === 'accepted') {
      // 幂等重复，直接返回 200 + duplicates
      return jsonResponse({
        schema_version: '1.0',
        status: 'accepted',
        batch_id: batch.batch_id,
        accepted: 0,
        duplicates: snapshotCount,
        rejected: [],
        server_time: nowIso,
      })
    }

    // 5. 单批事务提交至 D1 (单次 batch 提交，确保 10ms CPU 预算内完成)
    const statements: D1PreparedStatement[] = []

    // 5.1 记录批次
    const firstSeq = batch.sequence_first ?? batch.snapshots[0].sequence
    const lastSeq = batch.sequence_last ?? batch.snapshots[snapshotCount - 1].sequence

    statements.push(
      env.DB.prepare(
        `INSERT INTO ingest_batches (agent_id, instance_id, batch_id, first_sequence, last_sequence, received_at, status)
         VALUES (?, ?, ?, ?, ?, ?, 'accepted')
         ON CONFLICT (agent_id, instance_id, batch_id) DO UPDATE SET status = 'accepted'`
      ).bind(batch.agent_id, batch.instance_id, batch.batch_id, firstSeq, lastSeq, nowIso)
    )

    // 5.2 获取批次中最新的快照更新 latest
    const latestSnapshot = batch.snapshots[snapshotCount - 1]
    const latestJson = JSON.stringify(latestSnapshot)

    statements.push(
      env.DB.prepare(
        `INSERT INTO snapshots_latest (agent_id, instance_id, sequence, collected_at, snapshot_json, received_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (agent_id) DO UPDATE SET
           instance_id = excluded.instance_id,
           sequence = excluded.sequence,
           collected_at = excluded.collected_at,
           snapshot_json = excluded.snapshot_json,
           received_at = excluded.received_at`
      ).bind(
        batch.agent_id,
        batch.instance_id,
        latestSnapshot.sequence,
        latestSnapshot.collected_at || nowIso,
        latestJson,
        nowIso
      )
    )

    // 5.3 更新 agents 表
    const hostname = latestSnapshot.agent?.hostname || 'unknown'
    const os = latestSnapshot.agent?.os || 'unknown'
    const arch = latestSnapshot.agent?.arch || 'unknown'
    const labelsJson = JSON.stringify(latestSnapshot.agent?.labels || {})
    const healthStatus = latestSnapshot.health?.status || 'ok'
    const healthJson = JSON.stringify(latestSnapshot.health || {})

    statements.push(
      env.DB.prepare(
        `INSERT INTO agents (
           agent_id, display_name, os, arch, labels_json, first_seen, last_seen, last_push_at,
           last_instance_id, last_sequence, last_collected_at, health_status, health_json, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (agent_id) DO UPDATE SET
           display_name = excluded.display_name,
           os = excluded.os,
           arch = excluded.arch,
           labels_json = excluded.labels_json,
           last_seen = excluded.last_seen,
           last_push_at = excluded.last_push_at,
           last_instance_id = excluded.last_instance_id,
           last_sequence = excluded.last_sequence,
           last_collected_at = excluded.last_collected_at,
           health_status = excluded.health_status,
           health_json = excluded.health_json,
           updated_at = excluded.updated_at`
      ).bind(
        batch.agent_id,
        hostname,
        os,
        arch,
        labelsJson,
        nowIso,
        nowIso,
        nowIso,
        batch.instance_id,
        latestSnapshot.sequence,
        latestSnapshot.collected_at || nowIso,
        healthStatus,
        healthJson,
        nowIso
      )
    )

    // 5.4 分钟桶写入（metrics_1m）
    const bucketTs = (latestSnapshot.collected_at || nowIso).slice(0, 16) + ':00Z'
    const cpuUsage = latestSnapshot.cpu?.total?.usage_ratio ?? null
    const cpuIowait = latestSnapshot.cpu?.total?.iowait_ratio ?? null
    const cpuSteal = latestSnapshot.cpu?.total?.steal_ratio ?? null
    const memUsage = latestSnapshot.memory?.usage_ratio ?? null
    const memUsed = latestSnapshot.memory?.used_bytes ?? null
    const memCached = latestSnapshot.memory?.cached_bytes ?? null
    const swapUsed = latestSnapshot.memory?.swap_used_bytes ?? null
    const load1m = latestSnapshot.load?.average_1m ?? null

    statements.push(
      env.DB.prepare(
        `INSERT INTO metrics_1m (
           agent_id, bucket_ts, cpu_usage_ratio, cpu_iowait_ratio, cpu_steal_ratio,
           memory_usage_ratio, memory_used_bytes, memory_cached_bytes, swap_used_bytes,
           load_average_1m, health_status
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (agent_id, bucket_ts) DO UPDATE SET
           cpu_usage_ratio = excluded.cpu_usage_ratio,
           cpu_iowait_ratio = excluded.cpu_iowait_ratio,
           cpu_steal_ratio = excluded.cpu_steal_ratio,
           memory_usage_ratio = excluded.memory_usage_ratio,
           memory_used_bytes = excluded.memory_used_bytes,
           memory_cached_bytes = excluded.memory_cached_bytes,
           swap_used_bytes = excluded.swap_used_bytes,
           load_average_1m = excluded.load_average_1m,
           health_status = excluded.health_status`
      ).bind(
        batch.agent_id,
        bucketTs,
        cpuUsage,
        cpuIowait,
        cpuSteal,
        memUsage,
        memUsed,
        memCached,
        swapUsed,
        load1m,
        healthStatus
      )
    )

    // 执行事务批处理
    await env.DB.batch(statements)

    return jsonResponse({
      schema_version: '1.0',
      status: 'accepted',
      batch_id: batch.batch_id,
      accepted: snapshotCount,
      duplicates: 0,
      rejected: [],
      server_time: nowIso,
    })
  } catch (err: any) {
    return errorResponse('internal', `D1 存储写入失败: ${err.message}`, 500, true)
  }
}
