import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ResumeActions, WorkspaceDataSource, WorkspaceSnapshot } from "../../src/api/data-source";
import { bind } from "../../src/features/actions/resume-extra";
import { TaskStore } from "../../src/shared/state/tasks";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../../src/api/client";

vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn().mockResolvedValue("D:/tmp/resume.md") }));
vi.mock("../../src/api/client", () => ({
  api: { command: vi.fn().mockResolvedValue({ success: true, data: { saved: true } }) },
  unwrap: (result: { success: boolean; data?: unknown; error?: unknown }) => {
    if (!result.success) throw result.error;
    return result.data;
  },
}));

const snapshot = {
  experiences: [],
  personas: [{ id: "p1", name: "P", targetRole: "R", positioning: "X", fit: 1 }],
  jobs: [],
  resumes: [],
  skills: [],
  learning: [],
  providers: [],
} satisfies WorkspaceSnapshot;
const wait = () => new Promise((r) => setTimeout(r, 20));

const setup = (chatRefineResume: ReturnType<typeof vi.fn>, task: Record<string, unknown>) => {
  const root = document.createElement("div");
  root.innerHTML =
    '<select data-resume-persona><option value="p1" selected>P</option></select><div class="toolbar resume-actions"><button data-action="resume-refine">对话调优</button><button data-action="export-markdown">导出 Markdown</button></div><section class="paper"></section>';
  const source = {
    load: vi.fn().mockResolvedValue(snapshot),
    startTask: vi.fn().mockResolvedValue("task-1"),
    getTask: vi.fn().mockResolvedValue(task),
    cancelTask: vi.fn(),
    chatRefineResume,
    generateResume: vi.fn(),
    listResumeVersions: vi.fn(),
    diffResumeVersions: vi.fn(),
    restoreResumeVersion: vi.fn(),
  } as unknown as WorkspaceDataSource & ResumeActions;
  const tasks = new TaskStore();
  bind(root, source, tasks);
  return { root, source, tasks };
};

const launch = async (root: HTMLElement) => {
  root.querySelector<HTMLButtonElement>('[data-action="resume-refine"]')!.click();
  const form = root.querySelector<HTMLFormElement>("[data-refine-form]")!;
  form.querySelector<HTMLTextAreaElement>("textarea")!.value = "Improve";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  await wait();
};

describe("resume chat two-phase confirmation", () => {
  beforeEach(() => localStorage.removeItem("careercraft:selected-persona"));
  it("previews then commits the same proposal credential against its base version", async () => {
    const chat = vi.fn().mockResolvedValue({ id: "v-new", versionId: "v-new", markdown: "# Saved" });
    const { root, tasks } = setup(chat, {
      taskId: "task-1",
      state: "completed",
      progress: 100,
      events: [{ kind: "delta", text: "Preview" }],
      result: {
        requiresConfirmation: true,
        refinedSummary: "Preview",
        baseVersionId: "v-base",
        proposalId: "proposal-1",
        contentHash: "hash-1",
      },
    });
    await launch(root);
    root.querySelector<HTMLButtonElement>("[data-confirm-refine]")!.click();
    await wait();
    expect(chat).toHaveBeenCalledWith(
      "p1",
      "Improve",
      true,
      undefined,
      "v-base",
      "leadership",
      "proposal-1",
      "hash-1",
    );
    expect(tasks.list().at(-1)?.phase).toBe("completed");
  });
  it("cancel makes no persistence call", async () => {
    const chat = vi.fn();
    const { root, source } = setup(chat, {
      taskId: "task-1",
      state: "progress",
      progress: null,
      events: [],
    });
    void launch(root);
    await wait();
    root.querySelector<HTMLButtonElement>("[data-cancel-refine]")!.click();
    await wait();
    expect(source.cancelTask).toHaveBeenCalledWith("task-1");
    expect(chat).not.toHaveBeenCalled();
  });
});

describe("Markdown export", () => {
  it("writes the persisted preview through the native save dialog without generating", async () => {
    const root = document.createElement("div");
    root.innerHTML =
      '<select data-resume-persona><option value="p1" selected>P</option></select><button data-action="export-markdown">导出 Markdown</button>';
    const generateResume = vi.fn();
    const load = vi.fn().mockResolvedValue({
      ...snapshot,
      resumes: [
        {
          id: "v1",
          title: "Resume",
          persona: "p1",
          template: "classic",
          version: 3,
          updatedAt: "now",
          preview: "# Persisted",
        },
      ],
    });
    const source = {
      load,
      generateResume,
      chatRefineResume: vi.fn(),
      listResumeVersions: vi.fn().mockResolvedValue([{ markdown: "# Persisted" }]),
    } as unknown as WorkspaceDataSource & ResumeActions;
    bind(root, source, new TaskStore());
    root.querySelector<HTMLButtonElement>('[data-action="export-markdown"]')!.click();
    await wait();
    expect(generateResume).not.toHaveBeenCalled();
    expect(save).toHaveBeenCalledOnce();
    expect(api.command).toHaveBeenCalledWith("write_text_file", {
      destinationPath: "D:/tmp/resume.md",
      content: "# Persisted",
    });
  });
});
