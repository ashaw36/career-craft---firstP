import { describe, expect, it } from "vitest";
import { TaskStore } from "../../src/shared/state/tasks";
import { renderTaskCenter } from "../../src/shared/components/task-center";

describe("task state", () => {
  it("supports started, progress, completed, failed and cancelled", () => {
    const store = new TaskStore();
    for (const phase of ["started", "progress", "completed", "failed", "cancelled"] as const) store.upsert({ id: phase, label: phase, phase, message: phase });
    expect(store.list().map((task) => task.phase)).toEqual(["started", "progress", "completed", "failed", "cancelled"]);
  });
  it("clamps measurable progress and renders recovery actions", () => {
    const store = new TaskStore();
    store.upsert({ id: "x", label: "生成简历", phase: "failed", message: "网络暂时不可用", progress: 120, retryable: true });
    expect(store.list()[0]?.progress).toBe(100);
    expect(renderTaskCenter(store.list())).toContain("重试");
  });
});
