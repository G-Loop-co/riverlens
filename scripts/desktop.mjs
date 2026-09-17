import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { rustEnv } from './cargo.mjs';
import { signingEnv } from './signing-env.mjs';
const require = createRequire(import.meta.url);
const cli = require.resolve('@tauri-apps/cli/tauri.js');
const child = spawn(process.execPath, [cli, ...process.argv.slice(2)], { stdio: 'inherit', env: signingEnv(rustEnv()) });
child.on('error', e => { console.error(e.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
