// Sprobe 边缘端服务 - 零 KV 配额消耗保护模式 (Zero-KV-Write Guard)
// 通过 Cloudflare Cache API 与边缘内存完全接管读写，彻底切断所有 KV 交互（0 写、0 读），永久免疫 429 配额耗尽。

const ZERO_KV_MODE = true; // 强制启用 0 KV 模式（禁止任何 KV 读写）

const inMemoryNodesMeta = new Map();
const inMemorySnapshots = new Map();
const knownNodeIdsSet = new Set();
const lastAlertSentMap = new Map();
let cachedSettings = null;

async function sendNotification(env, title, message) {
  try {
    let settings = cachedSettings;
    if (!settings || !settings.notifications) return;

    const notif = settings.notifications;

    // 1. Webhook 发送 (兼容飞书、钉钉、企微、Slack)
    if (notif.webhookEnabled && notif.webhookUrl) {
      await fetch(notif.webhookUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          msg_type: 'text',
          text: `[Sprobe 警报] ${title}\n${message}\n时间: ${new Date().toLocaleString()}`,
          content: { text: `[Sprobe 警报] ${title}\n${message}` }
        })
      });
    }

    // 2. Telegram Bot 发送
    if (notif.telegramEnabled && notif.telegramBotToken && notif.telegramChatId) {
      const tgUrl = `https://api.telegram.org/bot${notif.telegramBotToken}/sendMessage`;
      await fetch(tgUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          chat_id: notif.telegramChatId,
          text: `⚠️ *[Sprobe 监控报警]* ${title}\n\n${message}\n\n*时间*: \`${new Date().toISOString()}\``,
          parse_mode: 'Markdown'
        })
      });
    }
  } catch (err) {
    console.error("Failed to send alert notification:", err);
  }
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // 跨域响应头
    const corsHeaders = {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization, Content-Encoding',
    };

    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: corsHeaders });
    }

    const cache = caches.default;

    // 1. Sprobe Agent 数据上报入口 (完全基于 Cache API + 内存，0 KV 消耗)
    if (url.pathname === '/api/v1/ingest' && request.method === 'POST') {
      try {
        let bodyText = '';
        const contentEncoding = request.headers.get('content-encoding') || '';
        
        if (contentEncoding.includes('gzip')) {
          const ds = new DecompressionStream('gzip');
          const decompressedStream = request.body.pipeThrough(ds);
          bodyText = await new Response(decompressedStream).text();
        } else {
          bodyText = await request.text();
        }

        const batch = JSON.parse(bodyText);
        const agentId = batch.agent_id;
        const snapshots = batch.snapshots || [];
        const latest = snapshots[snapshots.length - 1];

        if (agentId && latest) {
          // 计算概览 meta
          const rxTotal = latest.network?.interfaces?.reduce((s, i) => s + (i.receive_bytes_per_second || 0), 0) || 0;
          const txTotal = latest.network?.interfaces?.reduce((s, i) => s + (i.transmit_bytes_per_second || 0), 0) || 0;
          const diskRatio = latest.filesystems?.items?.[0]?.usage_ratio || 0;

          const meta = {
            agent_id: agentId,
            hostname: latest.agent?.hostname || latest.host?.hostname || 'unknown',
            display_name: latest.agent?.labels?.provider ? `${latest.agent?.hostname} (${latest.agent?.labels?.provider})` : latest.agent?.hostname,
            os: latest.agent?.os || 'linux',
            arch: latest.agent?.arch || 'x86_64',
            labels: latest.agent?.labels || {},
            health_status: latest.health?.status || 'ok',
            is_online: true,
            last_seen: new Date().toISOString(),
            cpu_usage_ratio: latest.cpu?.total?.usage_ratio ?? 0,
            memory_usage_ratio: latest.memory?.usage_ratio ?? 0,
            disk_usage_ratio: diskRatio,
            load_1m: latest.load?.average_1m ?? 0,
            network_rx_bytes_per_sec: rxTotal,
            network_tx_bytes_per_sec: txTotal,
            cycle_outbound_bytes: latest.network?.cycle_outbound_bytes || 0,
            cycle_limit_bytes: latest.network?.cycle_limit_bytes || 1099511627776,
            cycle_outbound_ratio: latest.network?.cycle_outbound_ratio || 0,
          };

          // 1. 存入边缘全局热内存 (零成本、瞬时完成)
          inMemorySnapshots.set(agentId, latest);
          inMemoryNodesMeta.set(agentId, meta);
          knownNodeIdsSet.add(agentId);

          // 2. 存入 Cache API (跨 Worker 实例共享，完全免费无上限)
          const singleCacheUrl = new URL(`/api/_cache/snapshot/${agentId}`, request.url);
          await cache.put(singleCacheUrl.toString(), new Response(JSON.stringify(latest), {
            headers: { 'Content-Type': 'application/json', 'Cache-Control': 'public, max-age=300' }
          }));

          const metaCacheUrl = new URL(`/api/_cache/meta/${agentId}`, request.url);
          await cache.put(metaCacheUrl.toString(), new Response(JSON.stringify(meta), {
            headers: { 'Content-Type': 'application/json', 'Cache-Control': 'public, max-age=300' }
          }));

          const idsCacheUrl = new URL(`/api/_cache/ids`, request.url);
          await cache.put(idsCacheUrl.toString(), new Response(JSON.stringify(Array.from(knownNodeIdsSet)), {
            headers: { 'Content-Type': 'application/json', 'Cache-Control': 'public, max-age=300' }
          }));

          // 3. 告警阈值检测 (10 分钟冷却防风暴)
          const nowAlert = Date.now();
          const lastAlert = lastAlertSentMap.get(agentId) || 0;
          if (nowAlert - lastAlert >= 600000) {
            const cpuThresh = cachedSettings?.alertThresholds?.cpuRatio ? cachedSettings.alertThresholds.cpuRatio / 100 : 0.90;
            const memThresh = cachedSettings?.alertThresholds?.memRatio ? cachedSettings.alertThresholds.memRatio / 100 : 0.95;

            if (meta.cpu_usage_ratio >= cpuThresh) {
              lastAlertSentMap.set(agentId, nowAlert);
              await sendNotification(env, `服务器高 CPU 告警: ${meta.hostname}`, `CPU 使用率已达 ${(meta.cpu_usage_ratio * 100).toFixed(1)}% (警戒阈值: ${(cpuThresh * 100).toFixed(0)}%)`);
            } else if (meta.memory_usage_ratio >= memThresh) {
              lastAlertSentMap.set(agentId, nowAlert);
              await sendNotification(env, `服务器高内存告警: ${meta.hostname}`, `内存使用率已达 ${(meta.memory_usage_ratio * 100).toFixed(1)}% (警戒阈值: ${(memThresh * 100).toFixed(0)}%)`);
            }
          }
        }

        const resp = {
          schema_version: '1.0',
          status: 'ok',
          batch_id: batch.batch_id || '00000000-0000-0000-0000-000000000000',
          accepted: snapshots.length || 1,
          duplicates: 0,
          rejected: [],
          server_time: new Date().toISOString(),
        };

        return new Response(JSON.stringify(resp), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      } catch (err) {
        return new Response(JSON.stringify({ error: err.message }), {
          status: 400,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      }
    }

    // 2. Sprobe 心跳接口 (0 KV 消耗)
    if (url.pathname === '/api/v1/heartbeat' && request.method === 'POST') {
      try {
        const hb = await request.json();
        const agentId = hb.agent_id;
        if (agentId) {
          const existing = inMemoryNodesMeta.get(agentId);
          if (existing) {
            existing.last_seen = new Date().toISOString();
            existing.is_online = true;
          }
        }
        return new Response(JSON.stringify({ status: 'ok', server_time: new Date().toISOString() }), {
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      } catch (err) {
        return new Response(JSON.stringify({ status: 'error', message: err.message }), {
          status: 400,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      }
    }

    // 3. 前端拉取总览数据 (优先内存与 Cache API，免受 429 影响)
    if (url.pathname === '/api/readonly/overview' && request.method === 'GET') {
      try {
        // 优先 1：读取当前 Worker 内存中的活跃节点
        if (inMemoryNodesMeta.size > 0) {
          return new Response(JSON.stringify(Array.from(inMemoryNodesMeta.values())), {
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }

        // 优先 2：从 Cache API 读取缓存的 ID 与 Meta
        const idsCacheUrl = new URL(`/api/_cache/ids`, request.url);
        const cachedIdsResp = await cache.match(idsCacheUrl.toString());
        let ids = [];
        if (cachedIdsResp) {
          ids = await cachedIdsResp.json();
        } else if (knownNodeIdsSet.size > 0) {
          ids = Array.from(knownNodeIdsSet);
        }

        const list = [];
        for (const id of ids) {
          const metaCacheUrl = new URL(`/api/_cache/meta/${id}`, request.url);
          const cached = await cache.match(metaCacheUrl.toString());
          if (cached) {
            list.push(await cached.json());
          }
        }

        if (list.length > 0) {
          return new Response(JSON.stringify(list), {
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }
      } catch (_) {}

      return new Response(JSON.stringify([]), {
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      });
    }

    // 4. 前端拉取单节点实时快照 (优先内存与 Cache API)
    if (url.pathname.startsWith('/api/readonly/nodes/') && request.method === 'GET') {
      const parts = url.pathname.split('/');
      const id = parts[parts.length - 1];
      if (id) {
        try {
          // 优先 1：内存直接命中
          let snapshot = inMemorySnapshots.get(id);

          // 优先 2：Cache API 命中
          if (!snapshot) {
            const singleCacheUrl = new URL(`/api/_cache/snapshot/${id}`, request.url);
            const cached = await cache.match(singleCacheUrl.toString());
            if (cached) snapshot = await cached.json();
          }

          if (snapshot) {
            const detailResponse = {
              schema_version: '1.0',
              generated_at: new Date().toISOString(),
              view: 'full',
              agent_runtime: {
                online: true,
                last_push_at: snapshot.collected_at || new Date().toISOString(),
                last_heartbeat_at: new Date().toISOString(),
                wal_snapshots: 0,
              },
              snapshot,
            };
            return new Response(JSON.stringify(detailResponse), {
              headers: { ...corsHeaders, 'Content-Type': 'application/json' },
            });
          }
        } catch (_) {}
      }
      return new Response(JSON.stringify({ error: 'not_found' }), {
        status: 404,
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      });
    }

    // 5. 管理员身份安全认证接口
    if (url.pathname === '/api/auth/verify' && request.method === 'POST') {
      try {
        const body = await request.json();
        const expected = env.ADMIN_PASSWORD || 'sprobe2026';
        if (body && body.password && body.password.trim() === expected) {
          return new Response(JSON.stringify({ success: true, token: expected }), {
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          });
        }
        return new Response(JSON.stringify({ success: false, message: '管理员密码不正确' }), {
          status: 401,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      } catch (e) {
        return new Response(JSON.stringify({ success: false, error: e.message }), {
          status: 400,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        });
      }
    }

    // 辅助安全鉴权函数
    function isAuthorizedAdmin(req, environment) {
      const authHeader = req.headers.get('Authorization') || '';
      const expected = environment.ADMIN_PASSWORD || 'sprobe2026';
      if (authHeader.startsWith('Bearer ')) {
        const token = authHeader.substring(7).trim();
        if (token === expected || token === (environment.ADMIN_TOKEN || '')) {
          return true;
        }
      }
      return false;
    }

    // 6. 系统设置与测试告警接口 (受管理员安全保护，基于 Cache API 与内存，0 KV 消耗)
    if (url.pathname === '/api/settings') {
      if (request.method === 'GET') {
        if (cachedSettings) {
          return new Response(JSON.stringify(cachedSettings), { headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
        }
        const settingsCacheUrl = new URL(`/api/_cache/settings`, request.url);
        const cached = await cache.match(settingsCacheUrl.toString());
        if (cached) {
          cachedSettings = await cached.json();
          return new Response(JSON.stringify(cachedSettings), { headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
        }
        return new Response(JSON.stringify({}), { headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
      }

      if (request.method === 'POST') {
        if (!isAuthorizedAdmin(request, env)) {
          return new Response(JSON.stringify({
            error: 'unauthorized',
            message: '安全设置已开启：公开演示环境禁止未经授权的写入操作。请先在控制台验证管理员口令。'
          }), {
            status: 401,
            headers: { ...corsHeaders, 'Content-Type': 'application/json' }
          });
        }

        try {
          const body = await request.json();
          cachedSettings = body;
          const settingsCacheUrl = new URL(`/api/_cache/settings`, request.url);
          await cache.put(settingsCacheUrl.toString(), new Response(JSON.stringify(body), {
            headers: { 'Content-Type': 'application/json', 'Cache-Control': 'public, max-age=2592000' }
          }));
          return new Response(JSON.stringify({ status: 'ok' }), { headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
        } catch (e) {
          return new Response(JSON.stringify({ error: e.message }), { status: 400, headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
        }
      }
    }

    if (url.pathname === '/api/alerts/test' && request.method === 'POST') {
      if (!isAuthorizedAdmin(request, env)) {
        return new Response(JSON.stringify({
          error: 'unauthorized',
          message: '安全设置已开启：公开演示环境禁止未经授权的告警测试。请先在控制台验证管理员口令。'
        }), {
          status: 401,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' }
        });
      }

      try {
        const body = await request.json();
        if (body.notifications) {
          cachedSettings = { ...cachedSettings, notifications: body.notifications };
        }
        await sendNotification(env, '测试告警连通性', '恭喜！这是一条来自 Cloudflare 边缘 Worker 自动发送的 Sprobe 监控测试消息。');
        return new Response(JSON.stringify({ status: 'ok', message: '测试消息已派发' }), { headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
      } catch (e) {
        return new Response(JSON.stringify({ error: e.message }), { status: 400, headers: { ...corsHeaders, 'Content-Type': 'application/json' } });
      }
    }

    // 静态资源回退与 SPA Fallback
    const response = await env.ASSETS.fetch(request);
    if (response.status === 404 && !url.pathname.startsWith('/api/') && !url.pathname.includes('.')) {
      return env.ASSETS.fetch(new Request(new URL('/index.html', request.url), request));
    }
    return response;
  },
};

