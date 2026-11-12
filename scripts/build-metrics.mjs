import { readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const targetRoot = process.argv[2];
if (!targetRoot) {
  console.error("usage: node scripts/build-metrics.mjs <target-dir>");
  process.exit(2);
}

function byteSize(path) {
  return statSync(path).size;
}

function directorySize(path) {
  let total = 0;
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const child = join(path, entry.name);
    total += entry.isDirectory() ? directorySize(child) : byteSize(child);
  }
  return total;
}

function existingSize(path, directory = false) {
  try {
    return directory ? directorySize(path) : byteSize(path);
  } catch {
    return null;
  }
}

function matchingFiles(directory, suffix) {
  try {
    return readdirSync(directory)
      .filter((name) => name.endsWith(suffix))
      .map((name) => ({
        name,
        bytes: byteSize(join(directory, name)),
      }));
  } catch {
    return [];
  }
}

const releaseRoot = join(targetRoot, "release");
const suffix = process.platform === "win32" ? ".exe" : "";
const bundleRoot = join(releaseRoot, "bundle");
const metrics = {
  target: targetRoot,
  binary_bytes: existingSize(join(releaseRoot, `riverlens${suffix}`)),
  macos_app_bytes: existingSize(
    join(bundleRoot, "macos", "RiverLens.app"),
    true,
  ),
  macos_app_zip: matchingFiles(bundleRoot, ".app.zip"),
  nsis_installer: matchingFiles(join(bundleRoot, "nsis"), ".exe"),
};

writeFileSync(
  join(releaseRoot, "build-metrics.json"),
  `${JSON.stringify(metrics, null, 2)}\n`,
);
