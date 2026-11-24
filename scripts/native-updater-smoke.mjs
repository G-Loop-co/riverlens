// Build two isolated RiverLens installations. The smoke webview drives the same
// native Tauri plugin and RiverLens RPCs as the shipped UI.
import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { createServer } from 'node:http';
import { createReadStream, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as delay } from 'node:timers/promises';
import { build } from 'vite';

async function browserSmoke({ invoke, getVersion, check, relaunch, exit }, spec) {
  const rpc = request => invoke('api', { request });
  // Record through the same native IPC and SQLite store used by RiverLens.
  const report = (step, detail) => rpc({
    op: 'save_filter', name: spec.token, filter: { search: JSON.stringify({ step, detail }) },
  });
  const expect = (condition, message) => { if (!condition) throw new Error(message); };
  const profile = { id: 'qa', name: 'Updater test', hero: 'Hero', brand: 'Natural8', timezone: 'Asia/Hong_Kong' };
  const hands = () => rpc({ op: 'hands', filter: { profile: profile.id }, limit: 5 });
  try {
    const version = await getVersion();
    await report('launched', version);
    if (version === spec.before) {
      await rpc({ op: 'save_profile', profile });
      await rpc({ op: 'start_import', paths: [spec.handPath], profile });
      let rows = [];
      for (let n = 0; n < 60 && !rows.length; n++) {
        await new Promise(resolve => setTimeout(resolve, 250));
        rows = (await hands()).rows;
      }
      expect(rows.length === 1, 'Fixture hand import failed');
      await rpc({ op: 'save_annotation', id: rows[0].id, annotation: { note: spec.token, tags: ['qa'], reviewed: true } });
      await rpc({ op: 'mda', request: { op: 'create', name: spec.token, source: 'Synthetic', timezone: 'UTC' } });
      localStorage.setItem('riverlens-auto-check-updates', 'false');
      const update = await check({ timeout: 15000 });
      expect(update?.version === spec.after, 'Expected a newer version in the local feed');
      await update.download();
      await report('downloaded', update.version);
      // NSIS relaunches the installed program itself. On macOS install returns.
      await update.install();
      await relaunch();
    } else {
      expect(version === spec.after, 'Unexpected installed version');
      const { rows } = await hands();
      expect(rows.length === 1, 'Imported hand missing after restart');
      const { annotation } = await rpc({ op: 'hand', id: rows[0].id });
      expect(annotation.note === spec.token && annotation.reviewed && annotation.tags.includes('qa'), 'Annotation lost');
      const catalog = await rpc({ op: 'mda', request: { op: 'list' } });
      expect(catalog.datasets.some(c => c.dataset.name === spec.token), 'Research dataset lost');
      expect(localStorage.getItem('riverlens-auto-check-updates') === 'false', 'Preference lost');
      expect(await check({ timeout: 15000 }) === null, 'Current version still offered an update');
      await report('passed', version);
      await exit(0);
    }
  } catch (error) {
    await report('failed', String(error));
    await exit(1);
  }
}

const mac = process.platform === 'darwin';
const target = process.argv[2];
const nativeTarget = mac ? `${process.arch === 'arm64' ? 'aarch64' : 'x86_64'}-apple-darwin` : 'x86_64-pc-windows-msvc';
assert.ok((mac || (process.platform === 'win32' && process.arch === 'x64')) && target === nativeTarget, 'Run on the target OS and CPU');
const root = process.cwd();
// Tauri deliberately rejects a starting binary reached through macOS's /var symlink.
const tmp = mkdtempSync(join(realpathSync.native(tmpdir()), 'riverlens-native-updater-'));
const token = randomBytes(6).toString('hex');
const name = 'RiverLensUpdateQA' + token;
const identifier = 'app.riverlens.updaterqa.' + 'r' + token;
const installed = join(tmp, 'installed');
const front = join(tmp, 'frontend');
// Use the unique QA app's own default store even if NSIS relaunches without inheriting environment.
const data = mac ? join(homedir(), 'Library/Application Support', identifier) : join(process.env.APPDATA, identifier);
for (const dir of [installed, front]) mkdirSync(dir);
const targetDir = process.env.RIVERLENS_SMOKE_TARGET_DIR || (process.env.GITHUB_ACTIONS === 'true' ? resolve('target') : join(tmp, 'target'));
const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
const next = version.replace(/\d+$/, n => String(Number(n) + 1));
const platform = mac ? (process.arch === 'arm64' ? 'darwin-aarch64' : 'darwin-x86_64') : 'windows-x86_64';
const key = join(tmp, 'fixture.key');
const targetArgs = process.env.GITHUB_ACTIONS === 'true' ? ['--target', target] : [];
const stage = join(targetDir, targetArgs.length ? target : '', 'release', 'bundle');
const env = { ...process.env, CARGO_TARGET_DIR: targetDir, RIVERLENS_DATA_DIR: data };
for (const key of Object.keys(env)) if (key.startsWith('TAURI_SIGNING_') || key.startsWith('APPLE_')) delete env[key];
const runtimeEnv = { ...env, TEMP: tmp, TMP: tmp, TMPDIR: tmp };
const tauri = resolve('node_modules/@tauri-apps/cli/tauri.js');
const run = (tool, args, options = {}) => execFileSync(tool, args, { env, stdio: 'inherit', ...options });
let artifact;
let feed;
const requests = { manifest: 0, artifact: 0 };
const server = createServer((request, response) => {
  if (request.url === '/latest.json' && feed) {
    console.log('[native updater] manifest request', ++requests.manifest);
    response.setHeader('Content-Type', 'application/json');
    response.end(JSON.stringify(feed));
  } else if (request.url === '/artifact' && artifact) {
    console.log('[native updater] artifact request', ++requests.artifact);
    createReadStream(artifact).pipe(response);
  } else {
    response.writeHead(404).end();
  }
});

function readEvents(database, afterId) {
  const db = new DatabaseSync(database, { readOnly: true });
  try {
    return db.prepare('SELECT id, data FROM saved_filters WHERE name = ? AND id > ? ORDER BY id').all(token, afterId);
  } catch (error) {
    // The app creates its schema on first launch.
    if (/no such table: saved_filters|database is (?:busy|locked)/i.test(error.message)) return [];
    throw error;
  } finally {
    db.close();
  }
}

async function observeUpgrade(app, database) {
  const events = [];
  let lastId = 0;
  let launchError;
  let sawDatabase = false;
  app.once('error', error => { launchError = error; });
  app.once('spawn', () => console.log('[native updater] launched process', app.pid, app.spawnfile));
  app.once('exit', (code, signal) => {
    console.log('[native updater] original process exited', { pid: app.pid, code, signal, hex: code == null ? null : '0x' + (code >>> 0).toString(16) });
  });
  const deadline = Date.now() + 180000;
  while (Date.now() < deadline) {
    if (launchError) throw launchError;
    if (existsSync(database)) {
      if (!sawDatabase) console.log('[native updater] found QA database', database);
      sawDatabase = true;
      for (const row of readEvents(database, lastId)) {
        const event = JSON.parse(JSON.parse(row.data).search);
        lastId = row.id;
        events.push(event);
        console.log('[native updater]', event.step, event.detail);
        if (event.step === 'failed') throw new Error(event.detail);
        if (event.step === 'passed') return events;
      }
    }
    if (app.signalCode || (app.exitCode !== null && (app.exitCode !== 0 || !events.some(e => e.step === 'downloaded')))) {
      throw new Error(`Original app exited before upgrade completed (code=${app.exitCode}, signal=${app.signalCode}, database=${sawDatabase}, last step=${events.at(-1)?.step || 'none'})`);
    }
    await delay(250);
  }
  throw new Error(`Timed out observing native upgrade (pid=${app.pid}, exit=${app.exitCode}, database=${sawDatabase}, last step=${events.at(-1)?.step || 'none'}, requests=${JSON.stringify(requests)})`);
}

let appProcess;
try {
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const spec = { before: version, after: next, token, handPath: resolve('tests/fixtures/study-nl10.txt') };
  const source = `import { invoke } from '@tauri-apps/api/core'; import { getVersion } from '@tauri-apps/api/app';
import { check } from '@tauri-apps/plugin-updater'; import { exit, relaunch } from '@tauri-apps/plugin-process';
(${browserSmoke.toString()})({invoke,getVersion,check,exit,relaunch},${JSON.stringify(spec)});`;
  const entry = 'virtual:riverlens-updater-smoke';
  await build({
    root, configFile: false, publicDir: false, logLevel: 'warn',
    plugins: [{
      name: 'riverlens-updater-smoke',
      resolveId(id) { if (id === entry) return '\0' + entry; },
      load(id) { if (id === '\0' + entry) return source; },
    }],
    build: { outDir: front, emptyOutDir: false, target: 'es2020', rollupOptions: { input: entry, output: { entryFileNames: 'fixture.js' } } },
  });
  writeFileSync(join(front, 'index.html'), '<!doctype html><meta charset="utf-8"><title>RiverLens updater test</title><p>Checking this isolated test installation…</p><script src="fixture.js" defer></script>');
  run(process.execPath, [tauri, 'signer', 'generate', '--ci', '-p', 'fixture', '-w', key], { stdio: 'ignore' });
  const config = join(tmp, 'smoke.json');
  function makeInstaller(releaseVersion) {
    writeFileSync(config, JSON.stringify({
      productName: name, identifier, version: releaseVersion,
      build: { beforeBuildCommand: '', frontendDist: front },
      app: {
        windows: [{ label: 'main', title: 'RiverLens updater test', width: 440, height: 160, focus: false }],
        security: {
          capabilities: ['default', { identifier: 'qa-exit', windows: ['main'], permissions: ['process:allow-exit'] }],
        },
      },
      bundle: { createUpdaterArtifacts: false, macOS: { signingIdentity: '-' } },
      plugins: { updater: { endpoints: [origin + '/latest.json'], pubkey: readFileSync(key + '.pub', 'utf8').trim(), dangerousInsecureTransportProtocol: true } },
    }));
    run(process.execPath, ['scripts/desktop.mjs', 'build', ...targetArgs, '--bundles', mac ? 'app' : 'nsis', '--config', config]);
    return mac ? join(stage, 'macos', name + '.app') : join(stage, 'nsis', `${name}_${releaseVersion}_x64-setup.exe`);
  }
  const before = makeInstaller(version);
  let executable;
  if (mac) {
    run('/usr/bin/codesign', ['--force', '--sign', '-', before]);
    const installedApp = join(installed, name + '.app');
    run('/usr/bin/ditto', [before, installedApp]);
    executable = join(installedApp, 'Contents/MacOS/riverlens');
  } else {
    run(before, ['/S', '/D=' + installed], { env: runtimeEnv, windowsVerbatimArguments: true });
    const names = readdirSync(installed).filter(s => s.endsWith('.exe') && !s.toLowerCase().includes('uninstall'));
    assert.equal(names.length, 1, 'Expected one installed application');
    executable = join(installed, names[0]);
  }
  const after = makeInstaller(next);
  if (mac) {
    run('/usr/bin/codesign', ['--force', '--sign', '-', after]);
    artifact = join(tmp, name + '.app.tar.gz');
    run('/usr/bin/tar', ['-czf', artifact, '-C', dirname(after), name + '.app'], { env: { ...env, COPYFILE_DISABLE: '1' } });
  } else artifact = after;
  run(process.execPath, [tauri, 'signer', 'sign', '-f', key, '-p', 'fixture', artifact], { stdio: 'ignore' });
  feed = { version: next, platforms: { [platform]: { url: origin + '/artifact', signature: readFileSync(artifact + '.sig', 'utf8').trim() } } };
  appProcess = spawn(executable, [], { env: runtimeEnv, stdio: 'inherit' });
  const events = await observeUpgrade(appProcess, join(data, 'riverlens.db'));
  assert.ok(requests.manifest >= 2 && requests.artifact >= 1, 'Both app versions must check the feed and the original must download');
  assert.deepEqual(events.map(e => e.step), ['launched', 'downloaded', 'launched', 'passed']);
  assert.deepEqual(events.map(e => e.detail), [version, next, next, next]);
  console.log(`PASS: ${target} ${version} -> ${next}; native install, restart and data preservation`);
} finally {
  server.closeAllConnections();
  server.close();
  // Give process:exit time to release the DuckDB and installer handles.
  await delay(1500);
  if (appProcess && appProcess.exitCode === null) appProcess.kill();
  if (!mac) {
    const uninstall = join(installed, 'uninstall.exe');
    if (existsSync(uninstall)) run(uninstall, ['/S', '_?=' + installed], { env: runtimeEnv, windowsVerbatimArguments: true });
  }
  const appData = mac
    ? ['Application Support', 'Caches', 'WebKit', 'HTTPStorages', 'Logs'].map(p => join(homedir(), 'Library', p, identifier))
    : [process.env.APPDATA, process.env.LOCALAPPDATA].filter(Boolean).map(p => join(p, identifier));
  if (mac) appData.push(join(homedir(), 'Library/Preferences', identifier + '.plist'), join(homedir(), 'Library/Saved Application State', identifier + '.savedState'));
  for (const path of [...appData, tmp]) rmSync(path, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
}
