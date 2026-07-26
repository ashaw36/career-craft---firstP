import assert from "node:assert/strict";
import { ids, invoke, ok, route } from "./helpers.mjs";

describe("CareerCraft native desktop journeys: write phase", () => {
  it("E2E-01 starts the real native app and command bridge", async () => {
    assert.equal(ok(await invoke("health")).status, "ok");
    await $(".brand").waitForDisplayed();
  });
  it("E2E-02 writes a confirmed experience to SQLite", async () => {
    ok(await invoke("save_experience", { newId: ids.experience, type: "work", title: "E2E Platform Engineer", organization: "CareerCraft QA", startDate: "2024-01-01", rawDescription: "Built Rust desktop systems", structuredAchievements: ["Reduced startup time by 40%"], skillsDemonstrated: ["Rust", "SQL"], industryTags: ["software"], educationLevel: "bachelor", status: "confirmed" }));
    assert(ok(await invoke("get_experiences", {})).some(value => value.id === ids.experience));
  });
  it("E2E-03 renders the persisted experience through the production UI", async () => {
    await browser.refresh(); await route("experiences");
    await expect($("main")).toHaveText(expect.stringContaining("E2E Platform Engineer"));
  });
  it("E2E-04 creates and renders a persona", async () => {
    ok(await invoke("create_persona", { id: ids.persona, name: "E2E Rust Persona", identityStatement: "Native desktop engineer", targetJobProfiles: ["Rust Engineer"], capabilityWeights: { Rust: 1 } }));
    await browser.refresh(); await route("personas");
    await expect($("main")).toHaveText(expect.stringContaining("E2E Rust Persona"));
  });
});
