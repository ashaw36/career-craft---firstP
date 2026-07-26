import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname);
const artifacts = resolve(process.env.E2E_ARTIFACTS || join(root, "artifacts", "desktop-e2e", "manual"));
mkdirSync(artifacts, { recursive: true });
process.env.CAREERCRAFT_DATA_DIR ||= join(artifacts, "profile");
const binary = process.env.CAREERCRAFT_E2E_BINARY || join(root, "src-tauri", "target", "debug", process.platform === "win32" ? "careercraft-desktop.exe" : "careercraft-desktop");

export const config = {
  runner: "local",
  specs: [join(root, "tests", "desktop-e2e", process.env.E2E_PHASE === "2" ? "phase2.spec.mjs" : "phase1.spec.mjs")],
  maxInstances: 1,
  capabilities: [{ browserName: "tauri", "tauri:options": { application: binary } }],
  services: [["@wdio/tauri-service", {
    appBinaryPath: binary,
    driverProvider: "embedded",
    embeddedPort: Number(process.env.TAURI_WEBDRIVER_PORT || 4445),
    startTimeout: 60_000,
    captureBackendLogs: true,
    captureFrontendLogs: true,
    logDir: artifacts
  }]],
  framework: "mocha",
  reporters: ["spec", ["junit", { outputDir: artifacts, outputFileFormat: () => `junit-phase${process.env.E2E_PHASE || "1"}.xml` }]],
  waitforTimeout: 15_000,
  connectionRetryTimeout: 90_000,
  connectionRetryCount: 1,
  mochaOpts: { ui: "bdd", timeout: 90_000 },
  afterTest: async function (_test, _context, result) {
    if (!result.passed) await browser.saveScreenshot(join(artifacts, `failure-${Date.now()}.png`));
  }
};
