import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import assert from "node:assert/strict";

const executable = process.argv[2];
if (!executable)
  throw new Error("Usage: node scripts/mcp-smoke.mjs <desktop-executable>");
const child = spawn(
  executable,
  ["--mcp", "--socket", "riverlens-ci-disabled"],
  {
    stdio: ["pipe", "pipe", "pipe"],
  },
);
const pending = new Map();
let id = 0;
const timer = setTimeout(() => {
  for (const request of pending.values())
    request.reject(new Error("MCP timeout"));
  child.kill();
}, 20000);
child.on("error", (error) => {
  for (const request of pending.values()) request.reject(error);
});
child.on("exit", (code) => {
  for (const request of pending.values())
    request.reject(new Error(`MCP exited: ${code}`));
});
createInterface({ input: child.stdout }).on("line", (line) => {
  let response;
  try {
    response = JSON.parse(line);
  } catch {
    return;
  }
  const request = pending.get(response.id);
  if (!request) return;
  pending.delete(response.id);
  if (response.error) request.reject(new Error(JSON.stringify(response.error)));
  else request.resolve(response.result);
});
// Drain diagnostics without retaining user environment or credential output.
child.stderr.on("data", () => {});
function request(method, params) {
  const requestId = ++id;
  return new Promise((resolve, reject) => {
    pending.set(requestId, { resolve, reject });
    child.stdin.write(
      JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params }) + "\n",
    );
  });
}
try {
  const initialized = await request("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "riverlens-packaged-smoke", version: "1" },
  });
  assert.ok(initialized.capabilities.tools);
  child.stdin.write(
    JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) +
      "\n",
  );
  const { tools } = await request("tools/list", {});
  assert.equal(tools.length, 17);
  assert.equal(new Set(tools.map((tool) => tool.name)).size, 17);
  for (const tool of tools)
    assert.equal(tool.inputSchema.additionalProperties, false);
  assert.ok(tools.some((tool) => tool.name === "get_decision_context"));
  assert.ok(tools.some((tool) => tool.name === "create_practice_draft"));
  assert.ok(!tools.some((tool) => /restore|delete|accept|sql/.test(tool.name)));
  const denied = await request("tools/call", {
    name: "get_data_catalog",
    arguments: {},
  });
  assert.equal(denied.isError, true);
  console.log(
    JSON.stringify({
      protocol: initialized.protocolVersion,
      tools: tools.length,
      disabledAccessDenied: true,
    }),
  );
} finally {
  clearTimeout(timer);
  child.stdin.end();
  child.kill();
}
