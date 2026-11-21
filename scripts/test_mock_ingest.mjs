// 快速自测 Mock Ingest 服务
import http from 'node:http'
import { spawn } from 'node:child_process'

const serverProcess = spawn('node', ['scripts/mock_ingest.mjs', '9988'], {
  stdio: 'pipe',
})

serverProcess.stdout.on('data', (d) => console.log(d.toString().trim()))
serverProcess.stderr.on('data', (d) => console.error(d.toString().trim()))

setTimeout(async () => {
  try {
    // 1. 测试未鉴权 401
    const unauthRes = await fetch('http://127.0.0.1:9988/api/v1/heartbeat', {
      method: 'POST',
    })
    console.log('未鉴权请求返回状态码:', unauthRes.status)
    const errJson = await unauthRes.json()
    console.log('未鉴权错误体:', errJson)
    if (unauthRes.status !== 401 || errJson.error.code !== 'unauthorized') {
      throw new Error('401 测试失败')
    }

    // 2. 测试带有效鉴权正常请求
    const validBatch = {
      schema_version: '1.0',
      batch_id: '11111111-2222-3333-4444-555555555555',
      agent_id: 'aaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
      instance_id: '99999999-8888-7777-6666-555555555555',
      created_at: new Date().toISOString(),
      sequence_first: 1,
      sequence_last: 1,
      snapshot_count: 1,
      snapshots: [{ sequence: 1 }],
    }

    const postBatch = async () => {
      const res = await fetch('http://127.0.0.1:9988/api/v1/ingest', {
        method: 'POST',
        headers: {
          Authorization: 'Bearer test-token',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(validBatch),
      })
      return res.json()
    }

    const firstRes = await postBatch()
    console.log('首次推送结果:', firstRes)
    if (firstRes.accepted !== 1 || firstRes.duplicates !== 0) {
      throw new Error('首次推送测试失败')
    }

    // 3. 测试幂等重复接收
    const secondRes = await postBatch()
    console.log('二次重复推送结果:', secondRes)
    if (secondRes.accepted !== 0 || secondRes.duplicates !== 1) {
      throw new Error('幂等测试失败')
    }

    // 3. 测试只读总览接口
    const overviewRes = await fetch('http://127.0.0.1:9988/api/readonly/overview')
    const overviewData = await overviewRes.json()
    console.log('只读总览接口返回:', overviewData)
    if (!Array.isArray(overviewData) || overviewData.length !== 1) {
      throw new Error('只读总览未返回正确的节点数据')
    }

    console.log('Mock Ingest 所有自动化校验通过！')
  } catch (err) {
    console.error('测试异常:', err)
    process.exitCode = 1
  } finally {
    serverProcess.kill()
  }
}, 600)
