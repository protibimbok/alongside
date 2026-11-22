#!/usr/bin/env node
"use strict";

// Usage: node npm/set-version.js <version>
// Sets every package under npm/ to <version>, including the main package's
// optionalDependencies, so they are always published as one matching set.

const fs = require("node:fs");
const path = require("node:path");

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version || "")) {
  console.error("usage: node npm/set-version.js <version>   (e.g. 1.2.3)");
  process.exit(1);
}

for (const dir of fs.readdirSync(__dirname, { withFileTypes: true })) {
  const file = path.join(__dirname, dir.name, "package.json");
  if (!dir.isDirectory() || !fs.existsSync(file)) continue;
  const pkg = JSON.parse(fs.readFileSync(file, "utf8"));
  pkg.version = version;
  for (const name of Object.keys(pkg.optionalDependencies || {})) {
    pkg.optionalDependencies[name] = version;
  }
  fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");
  console.log(`${pkg.name}@${version}`);
}
