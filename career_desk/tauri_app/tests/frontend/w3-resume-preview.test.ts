import { beforeEach, describe, expect, it, vi } from "vitest";
import { bind as bindResume, getResumeUiState } from "../../src/features/actions/resume";
import { bind as bindExtra } from "../../src/features/actions/resume-extra";
import { TaskStore } from "../../src/shared/state/tasks";
import type { ResumeActions, WorkspaceDataSource, WorkspaceSnapshot } from "../../src/api/data-source";
import type { ResumePreviewDto, ResumeTemplateId } from "../../src/api/contracts";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn().mockResolvedValue("D:/tmp/resume.md"),
}));
vi.mock("../../src/api/client", () => ({
  api: {
    command: vi.fn().mockResolvedValue({ success: true, data: { saved: true } }),
  },
  unwrap: (result: { success: true; data: unknown } | { success: false; error: unknown }) => {
    if (!result.success) throw result.error;
    return result.data;
  },
}));

const tick = () => new Promise((r) => setTimeout(r, 0));
const snapshot: WorkspaceSnapshot = {
  experiences: [],
  personas: [
    { id: "p1", name: "One", targetRole: "", positioning: "", fit: 0 },
    { id: "p2", name: "Two", targetRole: "", positioning: "", fit: 0 },
  ],
  jobs: [],
  resumes: [
    { id: "v-old", title: "Old", persona: "p2", template: "classic", version: 1, updatedAt: "old", preview: "# Old" },
    { id: "v-new", title: "New", persona: "p2", template: "modern", version: 2, updatedAt: "new", preview: "# New" },
  ],
  skills: [],
  learning: [],
  providers: [],
};
const preview = (template: ResumeTemplateId, markdown = `# ${template}`, personaId = "p2"): ResumePreviewDto => ({
  personaId,
  template,
  markdown,
  contentHash: `hash-${template}`,
  selectedExperienceIds: [],
  fitScores: [],
  warnings: [],
});

function setup(
  previewResume = vi.fn().mockImplementation((_p: string, t: ResumeTemplateId) => Promise.resolve(preview(t))),
  data: WorkspaceSnapshot = snapshot,
  listResumeVersions = vi.fn().mockImplementation(async (personaId: string) =>
    (data.resumes ?? [])
      .filter((x) => x.persona === personaId)
      .map((x) => ({
        versionId: x.id,
        personaId: x.persona,
        revision: x.version,
        template: x.template,
        markdown: x.preview,
        createdAt: x.updatedAt,
      })),
  ),
) {
  const root = document.createElement("div");
  root.innerHTML = `
    <button data-action="generate-resume">生成并保存</button>
    <div class="toolbar resume-context">
      <select data-resume-persona><option value="p1">One</option><option value="p2" selected>Two</option></select>
      <p data-resume-status></p>
      <button type="button" data-resume-versions>版本管理</button>
    </div>
    <section class="paper resume-stage"><section class="empty-state"></section></section>
    <div class="toolbar resume-actions">
      <button data-action="export-markdown">导出 Markdown</button>
    </div>`;
  const source = {
    load: vi.fn().mockResolvedValue(data),
    previewResume,
    generateResume: vi.fn(),
    listResumeVersions,
    diffResumeVersions: vi.fn(),
    restoreResumeVersion: vi.fn(),
    chatRefineResume: vi.fn(),
  } as unknown as WorkspaceDataSource & ResumeActions;
  bindResume(root, source, new TaskStore(), vi.fn());
  return { root, source, previewResume, listResumeVersions };
}

describe("W3 paper-first resume preview", () => {
  beforeEach(() => {
    localStorage.setItem("careercraft:selected-persona", "p2");
  });

  it("saves with default template without requiring a prior preview click", async () => {
    const { root, source } = setup();
    await tick();
    root.querySelector<HTMLButtonElement>('[data-action="generate-resume"]')!.click();
    await tick();
    expect(source.generateResume).toHaveBeenCalledWith("p2", expect.any(String));
  });

  it("exports markdown via save dialog without generating", async () => {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { api } = await import("../../src/api/client");
    const { root, source } = setup();
    bindExtra(root, source, new TaskStore());
    await tick();
    root.querySelector<HTMLButtonElement>('[data-action="export-markdown"]')!.click();
    await tick();
    await tick();
    expect(source.generateResume).not.toHaveBeenCalled();
    expect(save).toHaveBeenCalled();
    expect(api.command).toHaveBeenCalledWith(
      "write_text_file",
      expect.objectContaining({ destinationPath: "D:/tmp/resume.md" }),
    );
  });

  it("loads saved paper preview when switching persona", async () => {
    const data = {
      ...snapshot,
      resumes: [
        { id: "v-p1", title: "P1", persona: "p1", template: "classic", version: 1, updatedAt: "old", preview: "# P1 paper" },
        { id: "v-p2", title: "P2", persona: "p2", template: "modern", version: 1, updatedAt: "new", preview: "# P2 paper" },
      ],
    };
    localStorage.setItem("careercraft:selected-persona", "p1");
    const { root, previewResume } = setup(vi.fn(), data);
    await tick();
    const select = root.querySelector<HTMLSelectElement>("[data-resume-persona]")!;
    select.value = "p1";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();
    await tick();
    expect(root.textContent).toContain("P1 paper");
    expect(getResumeUiState(root)).toMatchObject({ personaId: "p1", markdown: "# P1 paper", mode: "saved" });
    select.value = "p2";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();
    await tick();
    expect(root.textContent).toContain("P2 paper");
    expect(root.textContent).not.toContain("P1 paper");
    expect(getResumeUiState(root)).toMatchObject({ personaId: "p2", markdown: "# P2 paper" });
    expect(previewResume).not.toHaveBeenCalled();
  });

  it("falls back to live preview when persona has no saved version", async () => {
    const data = {
      ...snapshot,
      resumes: [],
    };
    const { root, previewResume } = setup(
      vi.fn().mockResolvedValue(preview("classic", "# live preview", "p1")),
      data,
    );
    await tick();
    const select = root.querySelector<HTMLSelectElement>("[data-resume-persona]")!;
    select.value = "p1";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();
    await tick();
    expect(previewResume).toHaveBeenCalledWith("p1", "classic");
    expect(root.textContent).toContain("live preview");
    expect(getResumeUiState(root)).toMatchObject({ personaId: "p1", mode: "preview" });
  });

  it("opens version management dialog", async () => {
    const { root, source } = setup();
    await tick();
    (source.listResumeVersions as ReturnType<typeof vi.fn>).mockResolvedValue([
      { versionId: "v-new", template: "modern", revision: 2, createdAt: "today", markdown: "# New" },
    ]);
    root.querySelector<HTMLButtonElement>("[data-resume-versions]")!.click();
    await tick();
    expect(root.querySelector("[data-resume-version-dialog]")).not.toBeNull();
    expect(root.textContent).toContain("版本管理");
  });
});
