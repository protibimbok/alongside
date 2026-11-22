#!/usr/bin/env node
"use strict";

// Runs the prebuilt binary from the platform package that npm installed
// through optionalDependencies (only the one matching this os/cpu gets installed).

const { spawnSync } = require("node:child_process");

const SUPPORTED = ["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"];
const platform = `${process.platform}-${process.arch}`;
const pkg = `@concurtui/${platform}`;
const exe = process.platform === "win32" ? "concurtui.exe" : "concurtui";

function fail(message) {
  console.error(`concurtui: ${message}`);
  process.exit(1);
}

if (!SUPPORTED.includes(platform)) {
  fail(
    `no prebuilt binary for ${platform} (supported: ${SUPPORTED.join(", ")}).\n` +
      "You can build it from source with: cargo install concurtui"
  );
}

let binary;
try {
  binary = require.resolve(`${pkg}/bin/${exe}`);
} catch {
  fail(
    `the binary for ${platform} is missing (package ${pkg} is not installed).\n` +
      "This happens when optional dependencies are skipped (--no-optional, --omit=optional)\n" +
      "or when node_modules was copied from another platform. Try reinstalling:\n" +
      "  npm i -g concurtui"
  );
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  fail(`failed to run ${binary}: ${result.error.message}`);
}
if (result.signal) {
  // Die the same way the binary did, so the shell sees the signal.
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 1);
