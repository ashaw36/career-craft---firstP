import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const artifacts = resolve(process.env.E2E_ARTIFACTS || join(root, "artifacts", "desktop-e2e", new Date().toISOString().replace(/[:.]/g, "-")));
mkdirSync(artifacts, { recursive: true });
const env = { ...process.env, E2E_ARTIFACTS: artifacts, CAREERCRAFT_DATA_DIR: join(artifacts, "profile") };
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const npx = process.platform === "win32" ? "npx.cmd" : "npx";

function run(command, args, phase = "build") {
  const childEnv = { ...env, ...(phase === "phase1" ? { E2E_PHASE: "1" } : phase === "phase2" ? { E2E_PHASE: "2" } : {}) };
  const result = spawnSync(command, args, { cwd: root, env: childEnv, encoding: "utf8", stdio: "pipe", shell: process.platform === "win32", maxBuffer: 16 * 1024 * 1024, timeout: phase === "build" ? 240_000 : 180_000 });
  writeFileSync(join(artifacts, `${phase}.log`), `${result.stdout || ""}${result.stderr || ""}${result.error ? `\n${result.error.stack || result.error}` : ""}`);
  if (result.status !== 0) {
    process.stderr.write(result.stdout || "");
    process.stderr.write(result.stderr || "");
    if (result.error) process.stderr.write(`\n${result.error.stack || result.error}\n`);
    process.exit(result.status || 1);
  }
}

run(npm, ["run", "desktop:build:e2e"]);
run(npx, ["wdio", "run", "wdio.conf.mjs"], "phase1");
run(npx, ["wdio", "run", "wdio.conf.mjs"], "phase2");
console.log(`Desktop E2E passed. Artifacts: ${artifacts}`);
