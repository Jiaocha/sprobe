// Cloudflare Pages Functions: GET /api/readonly/overview (形态 B)
// 公开只读接口（HD-05 允许匿名，遵循 labels 白名单脱敏）

interface Env {
  DB: D1Database
}

export const onRequestGet: PagesFunction<Env> = async (context) => {
  const { env } = context

  try {
    const { results } = await env.DB.prepare(
      `SELECT
         a.agent_id,
         a.display_name,
         a.os,
         a.arch,
         a.labels_json,
         a.last_seen,
         a.health_status,
         s.snapshot_json
       FROM agents a
       LEFT JOIN snapshots_latest s ON a.agent_id = s.agent_id
       ORDER BY a.last_seen DESC`
    ).all()

    const now = Date.now()
    const items = (results || []).map((row: any) => {
      let labels: Record<string, string> = {}
      try {
        labels = JSON.parse(row.labels_json || '{}')
      } catch {
        // ignore
      }

      let cpuRatio: number | null = null
      let memRatio: number | null = null
      let diskRatio: number | null = null
      let load1m: number | null = null
      let rxBytes: number | null = null
      let txBytes: number | null = null

      if (row.snapshot_json) {
        try {
          const snap = JSON.parse(row.snapshot_json)
          cpuRatio = snap.cpu?.total?.usage_ratio ?? null
          memRatio = snap.memory?.usage_ratio ?? null
          load1m = snap.load?.average_1m ?? null

          // 取根分区使用率
          const rootFs = snap.filesystems?.items?.find((fs: any) => fs.mountpoint === '/')
          diskRatio = rootFs ? rootFs.usage_ratio : snap.filesystems?.items?.[0]?.usage_ratio ?? null

          // 网络第一张物理网卡速率
          const net = snap.network?.interfaces?.[0]
          if (net) {
            rxBytes = net.receive_bytes_per_second ?? null
            txBytes = net.transmit_bytes_per_second ?? null
          }
        } catch {
          // ignore
        }
      }

      // 超过 90 秒未收到快照或心跳判定为离线
      const lastSeenMs = new Date(row.last_seen).getTime()
      const isOnline = now - lastSeenMs < 90_000

      return {
        agent_id: row.agent_id,
        hostname: row.display_name,
        display_name: row.display_name,
        os: row.os,
        arch: row.arch,
        labels,
        health_status: row.health_status,
        is_online: isOnline,
        last_seen: row.last_seen,
        cpu_usage_ratio: isOnline ? cpuRatio : null,
        memory_usage_ratio: isOnline ? memRatio : null,
        disk_usage_ratio: isOnline ? diskRatio : null,
        load_1m: isOnline ? load1m : null,
        network_rx_bytes_per_sec: isOnline ? rxBytes : null,
        network_tx_bytes_per_sec: isOnline ? txBytes : null,
      }
    })

    return new Response(JSON.stringify(items, null, 2), {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'Cache-Control': 'public, max-age=15, stale-while-revalidate=15',
        'X-Content-Type-Options': 'nosniff',
      },
    })
  } catch (err: any) {
    return new Response(
      JSON.stringify(
        {
          error: {
            code: 'internal',
            message: `查询节点总览失败: ${err.message}`,
            retryable: true,
            request_id: `req_${crypto.randomUUID().slice(0, 8)}`,
            server_time: new Date().toISOString(),
          },
        },
        null,
        2
      ),
      {
        status: 500,
        headers: { 'Content-Type': 'application/json' },
      }
    )
  }
}
