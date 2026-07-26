import assert from "node:assert/strict";

export const ids = { experience: "e2e-exp", persona: "e2e-persona" };

export async function invoke(command, payload) {
  return browser.executeAsync((name, value, done) => {
    const core = globalThis.__TAURI__?.core;
    if (!core?.invoke) return done({ success: false, error: { code: "NO_TAURI", message: "global Tauri API unavailable" } });
    const args = name === "parse_jd" ? value : value === undefined ? {} : { payload: value };
    core.invoke(name, args).then(done).catch(error => done({ success: false, error: { code: "TRANSPORT", message: String(error) } }));
  }, command, payload);
}

export function ok(envelope) {
  assert.equal(envelope.success, true, JSON.stringify(envelope.error));
  return envelope.data;
}

export async function route(id) {
  const node = await $(`[data-route="${id}"]`);
  await node.waitForClickable();
  await node.click();
  await $("#page-title").waitForDisplayed();
}
