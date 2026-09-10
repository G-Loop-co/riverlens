import { spawn } from 'node:child_process';
import { rustEnv } from './cargo.mjs';
const cli = process.platform === 'win32' ? 'node_modules/.bin/tauri.cmd' : 'node_modules/.bin/tauri';
const child = spawn(cli, process.argv.slice(2), { stdio: 'inherit', env: rustEnv(), shell: process.platform === 'win32' });
child.on('error', e => { console.error(e.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
