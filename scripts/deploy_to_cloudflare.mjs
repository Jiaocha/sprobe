import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const accountId = process.env.CLOUDFLARE_ACCOUNT_ID || '';
const projectName = process.env.CLOUDFLARE_PROJECT_NAME || 'sprobe-console';
const apiToken = process.env.CLOUDFLARE_API_TOKEN || '';

if (!accountId || !apiToken) {
  console.error("Missing CLOUDFLARE_ACCOUNT_ID or CLOUDFLARE_API_TOKEN environment variable.");
  process.exit(1);
}

const distDir = path.resolve('./console/dist');

function getMimeType(filePath) {
  if (filePath.endsWith('.html')) return 'text/html';
  if (filePath.endsWith('.css')) return 'text/css';
  if (filePath.endsWith('.js')) return 'application/javascript';
  if (filePath.endsWith('.json')) return 'application/json';
  if (filePath.endsWith('.svg')) return 'image/svg+xml';
  if (filePath.endsWith('.png')) return 'image/png';
  if (filePath.endsWith('.ico')) return 'image/x-icon';
  if (filePath.endsWith('_redirects') || filePath.endsWith('_headers')) return 'text/plain';
  return 'application/octet-stream';
}

function getAllFiles(dir, base = '') {
  let results = [];
  const list = fs.readdirSync(dir);
  for (const item of list) {
    const fullPath = path.join(dir, item);
    const relPath = path.join(base, item).replace(/\\/g, '/');
    const stat = fs.statSync(fullPath);
    if (stat.isDirectory()) {
      results = results.concat(getAllFiles(fullPath, relPath));
    } else {
      // 忽略 _worker.js，因为它是服务端脚本
      if (item !== '_worker.js') {
        results.push({ fullPath, relPath: '/' + relPath });
      }
    }
  }
  return results;
}

async function run() {
  console.log("1. Fetching upload token...");
  const tokenRes = await fetch(`https://api.cloudflare.com/client/v4/accounts/${accountId}/pages/projects/${projectName}/upload-token`, {
    headers: { Authorization: `Bearer ${apiToken}` }
  });
  const tokenData = await tokenRes.json();
  if (!tokenData.success) {
    console.error("Failed to get upload token:", tokenData);
    process.exit(1);
  }
  const jwt = tokenData.result.jwt;

  console.log("2. Hashing static assets...");
  const files = getAllFiles(distDir);
  const manifest = {};
  const uploadPayload = [];
  const allHashes = [];

  for (const file of files) {
    const content = fs.readFileSync(file.fullPath);
    const hash = crypto.createHash('md5').update(content).digest('hex');
    manifest[file.relPath] = hash;
    allHashes.push(hash);
    uploadPayload.push({
      key: hash,
      value: content.toString('base64'),
      metadata: { contentType: getMimeType(file.fullPath) },
      base64: true
    });
  }

  console.log("3. Checking missing assets in Cloudflare Pages store...");
  const checkRes = await fetch('https://api.cloudflare.com/client/v4/pages/assets/check-missing', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${jwt}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ hashes: allHashes })
  });
  const checkData = await checkRes.json();
  const missingHashes = new Set(checkData.result || []);
  const neededUploads = uploadPayload.filter(item => missingHashes.has(item.key));

  if (neededUploads.length > 0) {
    console.log(`4. Uploading ${neededUploads.length} missing files...`);
    const uploadRes = await fetch('https://api.cloudflare.com/client/v4/pages/assets/upload', {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${jwt}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(neededUploads)
    });
    const uploadData = await uploadRes.json();
    if (!uploadData.success) {
      console.error("Upload assets failed:", uploadData);
      process.exit(1);
    }
    console.log("Assets uploaded successfully!");
  } else {
    console.log("All static assets already exist in store.");
  }

  console.log("5. Creating deployment with _worker.js Functions...");
  const workerPath = path.join(distDir, '_worker.js');
  const workerContent = fs.readFileSync(workerPath, 'utf-8');

  const form = new FormData();
  form.append('manifest', JSON.stringify(manifest));
  form.append('_worker.js', new Blob([workerContent], { type: 'application/javascript+module' }), '_worker.js');

  const deployRes = await fetch(`https://api.cloudflare.com/client/v4/accounts/${accountId}/pages/projects/${projectName}/deployments`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${apiToken}` },
    body: form
  });
  const deployData = await deployRes.json();
  if (!deployData.success) {
    console.error("Deploy failed:", deployData);
    process.exit(1);
  }

  const deployId = deployData.result.id;
  console.log(`Deployment created: ${deployId}, URL: ${deployData.result.url}`);

  // 轮询检查部署完成
  for (let i = 0; i < 15; i++) {
    await new Promise(r => setTimeout(r, 1500));
    const statusRes = await fetch(`https://api.cloudflare.com/client/v4/accounts/${accountId}/pages/projects/${projectName}/deployments/${deployId}`, {
      headers: { Authorization: `Bearer ${apiToken}` }
    });
    const statusData = await statusRes.json();
    const stage = statusData.result?.latest_stage;
    console.log(`Checking deployment status: ${stage?.name} -> ${stage?.status}`);
    if (stage?.status === 'success') {
      console.log("Deployment is LIVE & SUCCESSFUL!");
      break;
    }
  }
}

run().catch(console.error);
