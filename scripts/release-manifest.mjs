import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync,
  renameSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';

const [inputDir, outputDir, option, testKeyPath, ...extra] = process.argv.slice(2);
if (!inputDir || !outputDir || extra.length || (option && (option !== '--test-key' || !testKeyPath))) {
  throw new Error('Usage: release-manifest.mjs INPUT_DIR NEW_OUTPUT_DIR [--test-key PUBLIC_KEY_FILE]');
}

const readJson = path => JSON.parse(readFileSync(path, 'utf8'));
const { version } = readJson('package.json');
const config = readJson('src-tauri/tauri.conf.json');
const tag = `v${version}`;
if (!/^\d+\.\d+\.\d+$/.test(version) || config.version !== version) {
  throw new Error('Expected a stable version matching the desktop bundle');
}

if (testKeyPath) {
  if (process.env.GITHUB_REF?.startsWith('refs/tags/')) throw new Error('A tag must use the production signing key');
} else {
  if (process.env.GITHUB_REF_NAME !== tag) throw new Error('Release tag and package version differ');
  if (!process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Release requires TAURI_SIGNING_PRIVATE_KEY');
}
const publicKey = testKeyPath ? readFileSync(testKeyPath, 'utf8') : config.plugins.updater.pubkey;
const commit = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (commit !== process.env.GITHUB_SHA) throw new Error('Build SHA mismatch');
const repository = process.env.GITHUB_REPOSITORY;
if (!repository || !/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('Invalid GitHub repository name');
const publicUrl = `https://github.com/${repository}/releases/download/${tag}`;
const notes = readFileSync(`docs/release-${tag}.md`, 'utf8').trim().split(/\r?\n\s*\r?\n/)[1];
if (!notes) throw new Error('Release notes need a summary paragraph');

const macArm = 'RiverLens_aarch64-apple-darwin.app';
const macIntel = 'RiverLens_x86_64-apple-darwin.app';
const payloads = {
  'darwin-aarch64': `${macArm}.tar.gz`,
  'darwin-x86_64': `${macIntel}.tar.gz`,
  'windows-x86_64': `RiverLens_${version}_x64-setup.exe`,
};
const expected = [`${macArm}.zip`, `${macIntel}.zip`, ...Object.values(payloads)].sort();

function filesIn(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return filesIn(path);
    if (!entry.isFile()) throw new Error(`Unexpected non-file: ${path}`);
    return [path];
  });
}

function fingerprint(dir, name) {
  const bytes = readFileSync(join(dir, name));
  return { name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') };
}

function assertBundleFormat(path) {
  const prefix = readFileSync(path).subarray(0, 4).toString('hex');
  const magic = path.endsWith('.exe') ? '4d5a' : path.endsWith('.zip') ? '504b0304' : '1f8b08';
  if (!prefix.startsWith(magic)) throw new Error(`Invalid bundle: ${basename(path)}`);
}

const inputs = filesIn(inputDir);
const names = inputs.map(path => basename(path)).sort();
if (JSON.stringify(names) !== JSON.stringify(expected)) throw new Error(`Unexpected bundle inventory: ${names}`);
const destination = resolve(outputDir);
mkdirSync(dirname(destination), { recursive: true });
const staged = mkdtempSync(join(dirname(destination), '.riverlens-release-'));
const scratch = mkdtempSync(join(tmpdir(), 'riverlens-minisign-'));
try {
  for (const path of inputs) {
    assertBundleFormat(path);
    copyFileSync(path, join(staged, basename(path)));
  }
  // Verify against the public key that update clients trust, including on the first release.
  const verifyingKey = join(scratch, 'updater.pub');
  writeFileSync(verifyingKey, Buffer.from(publicKey.trim(), 'base64'));
  const platforms = {};
  for (const [platform, name] of Object.entries(payloads)) {
    const path = join(staged, name);
    execFileSync(resolve('node_modules/.bin/tauri'), ['signer', 'sign', path], { stdio: ['ignore', 'ignore', 'inherit'] });
    const signature = readFileSync(`${path}.sig`, 'utf8').trim();
    const decoded = join(scratch, 'payload.minisig');
    writeFileSync(decoded, Buffer.from(signature, 'base64'));
    execFileSync('minisign', ['-V', '-m', path, '-p', verifyingKey, '-x', decoded], { stdio: 'inherit' });
    platforms[platform] = { url: `${publicUrl}/${encodeURIComponent(name)}`, signature };
  }
  const writeJson = (name, value) => writeFileSync(join(staged, name), JSON.stringify(value, null, 2) + '\n');
  writeJson('latest.json', { version, notes, platforms });
  const assets = readdirSync(staged).sort().map(name => fingerprint(staged, name));
  writeJson('build-manifest.json', { version, commit, run: process.env.GITHUB_RUN_ID, macosSigning: process.env.MACOS_SIGNING, assets });
  const checksums = [...assets, fingerprint(staged, 'build-manifest.json')];
  writeFileSync(join(staged, 'SHA256SUMS.txt'), checksums.map(file => `${file.sha256}  ${file.name}\n`).join(''));
  renameSync(staged, destination);
} finally {
  rmSync(scratch, { recursive: true, force: true });
  rmSync(staged, { recursive: true, force: true });
}
