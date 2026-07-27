import { describe, expect, it, vi } from "vitest";
import type { SkillLearningActions, WorkspaceDataSource, WorkspaceSnapshot } from "../../src/api/data-source";
import { bind } from "../../src/features/actions/skills-learning";
import { renderPage } from "../../src/features/pages";
import { TaskStore } from "../../src/shared/state/tasks";

const snapshot: WorkspaceSnapshot = {
  experiences: [], personas: [], jobs: [], resumes: [], learning: [], providers: [],
  skills: [{ id: "rust", name: "Rust", level: "2", evidence: 0, category: "technical", resources: [] }],
};

describe("skill action flows", () => {
  it("hands the selected skill to the job-bound what-if flow", () => {
    const source = {};
    const openJobWhatIf = vi.fn();
    const root = document.createElement("div");
    root.innerHTML = renderPage("skills", snapshot, { selectedSkillId: "rust" });
    bind(root, source as unknown as WorkspaceDataSource & SkillLearningActions, new TaskStore(), vi.fn(), { onOpenJobWhatIf: openJobWhatIf });

    root.querySelector<HTMLButtonElement>("[data-what-if]")!.click();
    expect(openJobWhatIf).toHaveBeenCalledWith("Rust");
    expect(root.querySelector("[data-what-if-form]")).toBeNull();
  });
});
