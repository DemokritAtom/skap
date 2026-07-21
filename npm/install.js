#!/usr/bin/env node
/**
 * skap npm post-install hook.
 *
 * Downloads the matching pre-built `skap` binary from the GitHub release
 * for this package's version and places it at `bin/skap`. Falls back to a
 * helpful error message if no asset matches the platform.
 */
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const https = require('https');
const { pipeline } = require('stream');
const { promisify } = require('util');
const zlib = require('zlib');

const pump = promisify(pipeline);

const VERSION = require('./package.json').version;
const REPO = 'DemokritAtom/skap';

function detectTarget() {
  const platform = os.platform();
  const arch = os.arch();
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  if (platform === 'linux' && arch === 'arm64') return 'aarch64-unknown-linux-gnu';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin';
  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  throw new Error(`unsupported platform: ${platform}/${arch}`);
}

function follow(url) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { 'User-Agent': 'skap-npm-installer' } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        resolve(follow(res.headers.location));
      } else if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      } else {
        resolve(res);
      }
    });
    req.on('error', reject);
  });
}

async function main() {
  const target = detectTarget();
  const asset = `skap-v${VERSION}-${target}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
  const binDir = path.join(__dirname, 'bin');
  fs.mkdirSync(binDir, { recursive: true });
  const tmpFile = path.join(binDir, 'skap.tar.gz');

  console.log(`[skap] downloading ${url}`);
  const res = await follow(url);
  await pump(res, fs.createWriteStream(tmpFile));

  // Extract a single binary from the tar.gz without depending on a
  // tar parser by shelling out to `tar`. macOS and every Linux distro
  // ship with it.
  const { execFileSync } = require('child_process');
  execFileSync('tar', ['-xzf', tmpFile, '-C', binDir, 'skap'], { stdio: 'inherit' });
  fs.unlinkSync(tmpFile);
  fs.chmodSync(path.join(binDir, 'skap'), 0o755);
  console.log('[skap] installed to', path.join(binDir, 'skap'));
}

main().catch((err) => {
  console.error('[skap] post-install failed:', err.message);
  console.error('[skap] you can install manually via: cargo install skap');
  process.exit(0); // do not block npm install completely
});
