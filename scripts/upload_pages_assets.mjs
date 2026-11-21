import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const jwt = process.argv[2];
if (!jwt) {
  console.error("Missing upload JWT");
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
      results.push({ fullPath, relPath: '/' + relPath });
    }
  }
  return results;
}

const files = getAllFiles(distDir);
console.log(`Found ${files.length} files in ${distDir}`);

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

console.log("Checking missing hashes...");
const checkRes = await fetch('https://api.cloudflare.com/client/v4/pages/assets/check-missing', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${jwt}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({ hashes: allHashes })
});
const checkData = await checkRes.json();
console.log("Check-missing result:", checkData);

const missingHashes = new Set(checkData.result || []);
const neededUploads = uploadPayload.filter(item => missingHashes.has(item.key));

if (neededUploads.length > 0) {
  console.log(`Uploading ${neededUploads.length} missing files...`);
  const uploadRes = await fetch('https://api.cloudflare.com/client/v4/pages/assets/upload', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${jwt}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(neededUploads)
  });
  const uploadData = await uploadRes.json();
  console.log("Upload result:", uploadData);
  if (!uploadData.success) {
    console.error("Upload failed!");
    process.exit(1);
  }
} else {
  console.log("All assets already exist in store.");
}

// 保存 manifest
fs.writeFileSync('./target/pages_manifest.json', JSON.stringify(manifest, null, 2));
console.log("Manifest saved to ./target/pages_manifest.json");
