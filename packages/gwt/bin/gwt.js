#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

function libc() {
  if (process.platform !== "linux") return "";
  if (process.report && typeof process.report.getReport === "function") {
    const report = process.report.getReport();
    if (report.header && report.header.glibcVersionRuntime) return "gnu";
  }
  return "musl";
}

const platform = process.platform;
const arch = process.arch;
const suffix = platform === "linux" ? `-${libc()}` : "";
const packageName = `@autumn-k/gwt-${platform}-${arch}${suffix}`;

if (!(["darwin", "linux"].includes(platform)) || !(["arm64", "x64"].includes(arch))) {
  console.error(`gwt: unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

let binary;
try {
  binary = require.resolve(`${packageName}/bin/gwt`);
} catch {
  console.error(`gwt: platform package ${packageName} is not installed.`);
  console.error("Try reinstalling @autumn-k/gwt without disabling optional dependencies.");
  process.exit(1);
}

if (typeof process.execve === "function") {
  try {
    process.execve(binary, [binary, ...process.argv.slice(2)], process.env);
  } catch (error) {
    console.error(`gwt: failed to exec ${binary}: ${error.message}`);
    process.exit(1);
  }
}

// Node.js before 22.15 does not expose execve, so retain equivalent stdio and
// signal/exit behavior with a synchronous child process on older installations.
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`gwt: failed to start ${binary}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
} else {
  process.exit(result.status ?? 1);
}
