import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../../src/app";
import type { Experience, WorkspaceSnapshot } from "../../src/api/data-source";
import { bind as bindExperience } from "../../src/features/actions/experience";
import { renderPage } from "../../src/features/pages";
import { TaskStore } from "../../src/shared/state/tasks";

const tick = () => new Promise((r) => setTimeout(r, 0));

const experiences: Experience[] = [
  {
    id: "e1",
    version: 1,
    title: "供应链看板",
    organization: "示例公司",
    period: "2024",
    kind: "工作",
    original: "缺货率下降",
    skills: ["SQL"],
    status: "confirmed",
  },
  {
    id: "e2",
    version: 1,
    title: "开源贡献",
    organization: "社区",
    period: "2023",
    kind: "项目",
    original: "Rust CLI",
    skills: ["Rust"],
    status: "confirmed",
  },
];

const snapshot: WorkspaceSnapshot = {
  experiences,
  personas: [],
  jobs: [],
  resumes: [],
  skills: [],
  learning: [],
  providers: [],
};

describe("UX-EXP-SETTINGS-001 experience library", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
    localStorage.clear();
  });

  it("renders title-row actions without revision history", () => {
    const root = document.createElement("div");
    root.innerHTML = renderPage("experiences", snapshot);
    expect(root.textContent).not.toContain("修改记录");
    expect(root.querySelectorAll("[data-edit-experience]")).toHaveLength(2);
    expect(root.querySelectorAll("[data-delete-experience]")).toHaveLength(2);
    expect(root.querySelector(".experience-actions")).not.toBeNull();
  });

  it("filters by search text and kind", async () => {
    const root = document.createElement("div");
    root.innerHTML = renderPage("experiences", snapshot);
    const source = {
      load: vi.fn().mockResolvedValue(snapshot),
      importFile: vi.fn(),
      updateExperience: vi.fn(),
    };
    bindExperience(root, source as never, new TaskStore(), vi.fn());
    const search = root.querySelector<HTMLInputElement>("[data-experience-search]")!;
    search.value = "rust";
    search.dispatchEvent(new Event("input", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 150));
    expect(root.querySelector<HTMLElement>('[data-experience-id="e1"]')!.hidden).toBe(true);
    expect(root.querySelector<HTMLElement>('[data-experience-id="e2"]')!.hidden).toBe(false);
    const kind = root.querySelector<HTMLSelectElement>("[data-experience-kind]")!;
    kind.value = "工作";
    kind.dispatchEvent(new Event("change", { bubbles: true }));
    expect(root.querySelector<HTMLElement>('[data-experience-id="e2"]')!.hidden).toBe(true);
    expect(root.querySelector<HTMLElement>("[data-experience-empty]")!.hidden).toBe(false);
  });

  it("opens the shared editable dialog for edit actions", async () => {
    localStorage.setItem("careercraft:onboarding-complete", "true");
    const root = document.createElement("div");
    const source = {
      load: vi.fn().mockResolvedValue(snapshot),
      addExperience: vi.fn(),
      deleteExperience: vi.fn(),
      createPersona: vi.fn(),
      updatePersona: vi.fn(),
      deletePersona: vi.fn(),
      getFitScores: vi.fn().mockResolvedValue([]),
      setFitScore: vi.fn(),
      saveProvider: vi.fn(),
      testProvider: vi.fn(),
      startTask: vi.fn(),
      updateExperience: vi.fn(),
      importFile: vi.fn(),
    };
    const app = new App(root, source as never);
    app.navigation.navigate("experiences");
    app.mount();
    await tick();
    await tick();
    root.querySelector<HTMLButtonElement>('[data-edit-experience="e1"]')!.click();
    await tick();
    expect(source.deleteExperience).not.toHaveBeenCalled();
    const form = root.querySelector<HTMLFormElement>("[data-experience-form]")!;
    expect(form.dataset.mode).toBe("edit");
    expect(form.querySelector("#experience-dialog-title")?.textContent).toBe("编辑经历");
    expect(form.querySelector<HTMLTextAreaElement>('[name="original"]')?.value).toContain("缺货率");
  });

  it("exposes an explicit filter button that applies search", async () => {
    const root = document.createElement("div");
    root.innerHTML = renderPage("experiences", snapshot);
    bindExperience(root, { importFile: vi.fn(), updateExperience: vi.fn() } as never, new TaskStore(), vi.fn());
    const search = root.querySelector<HTMLInputElement>("[data-experience-search]")!;
    search.value = "rust";
    root.querySelector<HTMLButtonElement>("[data-experience-filter]")!.click();
    expect(root.querySelector<HTMLElement>('[data-experience-id="e1"]')!.hidden).toBe(true);
    expect(root.querySelector<HTMLElement>('[data-experience-id="e2"]')!.hidden).toBe(false);
  });
});
