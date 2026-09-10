import { existsSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { delimiter } from 'node:path';
export function rustEnv() {
  const root = '/private/tmp/h2n-toolchain';
  if (process.env.CARGO_HOME || !existsSync(`${root}/cargo/bin/cargo`)) return process.env;
  return { ...process.env, CARGO_HOME: `${root}/cargo`, RUSTUP_HOME: `${root}/rustup`, PATH: `${root}/cargo/bin${delimiter}${process.env.PATH}` };
}
if (process.argv[1]?.endsWith('cargo.mjs')) {
  const child = spawn('cargo', process.argv.slice(2), { stdio: 'inherit', env: rustEnv() });
  child.on('error', e => { console.error(e.message); process.exitCode = 1; });
  child.on('exit', code => { process.exitCode = code ?? 1; });
}
