import assert from "node:assert/strict";
import { ids, invoke, ok } from "./helpers.mjs";

let jobId = ""; let matchId = "";
describe("CareerCraft native desktop journeys: restart and mutation phase", () => {
  it("E2E-05 survives a full application process restart", async () => {
    assert(ok(await invoke("get_experiences", {})).some(value => value.id === ids.experience));
    assert(ok(await invoke("get_personas", {})).some(value => value.id === ids.persona));
  });
  it("E2E-06 persists a parsed job description", async () => {
    const job = ok(await invoke("parse_jd", { jdText: "Senior Rust Engineer with SQL and five years experience" })); jobId = job.id;
    assert(jobId); assert(ok(await invoke("list_jobs", {})).some(value => value.id === jobId));
  });
  it("E2E-07 creates a real job match", async () => {
    const match = ok(await invoke("match_job", { jobDescId: jobId, personaId: ids.persona })); matchId = match.id;
    assert.equal(match.trackingStatus, "new"); assert(Number.isFinite(match.matchScore));
  });
  it("E2E-08 writes status and audit history", async () => {
    const changed = ok(await invoke("update_match_status", { matchId, status: "interested", expectedVersion: 1 }));
    assert.equal(changed.trackingStatus, "interested");
    assert.deepEqual(ok(await invoke("get_job_status_events", { matchId })).map(event => event.toStatus), ["new", "interested"]);
  });
  it("E2E-09 cancels a native background task", async () => {
    const task = ok(await invoke("start_background_task", { operation: "chat_refine_resume", payload: { personaId: ids.persona, instruction: "cancel me", instructionType: "general" } }));
    assert.equal(ok(await invoke("cancel_background_task", { taskId: task.taskId })).state, "cancelled");
  });
  it("E2E-10 enforces external-link tokens and can exercise the OS handoff", async () => {
    assert.equal((await invoke("open_external_url", { token: "forged-token" })).success, false);
    if (process.env.E2E_ALLOW_EXTERNAL === "1") { const collected = ok(await invoke("collect_job_url", { url: "https://example.com/careercraft-e2e" })); assert(collected.openToken); assert.equal(ok(await invoke("open_external_url", { token: collected.openToken })).opened, true); }
  });
  it("E2E-11 exposes validation and optimistic-lock errors without corruption", async () => {
    const validation = await invoke("parse_jd", { jdText: "" }); assert.equal(validation.success, false); assert.equal(validation.error.code, "VALIDATION");
    const stale = await invoke("update_match_status", { matchId, status: "applied", expectedVersion: 1 }); assert.equal(stale.success, false); assert.equal(stale.error.code, "CONFLICT");
  });
  it("E2E-12 deletes test records and verifies durable cleanup", async () => {
    ok(await invoke("delete_job", { jobDescId: jobId })); ok(await invoke("delete_persona", { personaId: ids.persona }));
    const experience = ok(await invoke("get_experiences", {})).find(value => value.id === ids.experience);
    ok(await invoke("delete_experience", { experienceId: ids.experience, version: experience.version }));
    assert(!ok(await invoke("get_experiences", {})).some(value => value.id === ids.experience)); assert(!ok(await invoke("get_personas", {})).some(value => value.id === ids.persona));
  });
});
