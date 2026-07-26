export type TaskPhase = "started" | "progress" | "completed" | "failed" | "cancelled";
export interface TaskState { id: string; label: string; phase: TaskPhase; message: string; progress?: number; retryable?: boolean; }
export class TaskStore extends EventTarget {
  private readonly tasks = new Map<string, TaskState>();
  list(): TaskState[] { return [...this.tasks.values()]; }
  upsert(task: TaskState): void {
    const progress = task.progress === undefined ? undefined : Math.max(0, Math.min(100, task.progress));
    this.tasks.set(task.id, { ...task, progress });
    this.dispatchEvent(new Event("change"));
  }
  dismiss(id: string): void { this.tasks.delete(id); this.dispatchEvent(new Event("change")); }
}
