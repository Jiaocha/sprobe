// Cloudflare Pages Functions: POST /api/v1/heartbeat (形态 B)

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

  const authHeader = request.headers.get('Authorization')
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return errorResponse('unauthorized', '缺少有效的 Bearer 认证令牌', 401)
  }

  const token = authHeader.slice(7).trim()
  if (env.INGEST_TOKEN && token !== env.INGEST_TOKEN) {
    return errorResponse('unauthorized', '推送令牌无效', 401)
  }

  let body: any
  try {
    body = await request.json()
  } catch (err: any) {
    return errorResponse('invalid_json', `JSON 解析失败: ${err.message}`, 400)
  }

  if (!body.agent_id || !body.instance_id) {
    return errorResponse('validation_failed', '缺少 agent_id 或 instance_id', 422)
  }

  const nowIso = new Date().toISOString()
  const healthStatus = body.health_status || 'ok'
  const walSnapshots = body.wal_snapshots || 0

  try {
    await env.DB.prepare(
      `UPDATE agents SET
         last_seen = ?,
         last_heartbeat_at = ?,
         health_status = ?,
         wal_snapshots = ?,
         updated_at = ?
       WHERE agent_id = ?`
    )
      .bind(nowIso, nowIso, healthStatus, walSnapshots, nowIso, body.agent_id)
      .run()

    return jsonResponse({
      schema_version: '1.0',
      status: 'accepted',
      server_time: nowIso,
    })
  } catch (err: any) {
    return errorResponse('internal', `更新心跳失败: ${err.message}`, 500, true)
  }
}
