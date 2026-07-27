import type { TaskState } from "../state/tasks";
import { escapeHtml as esc } from "../html";
const labels: Record<TaskState["phase"], string> = { started: "已开始", progress: "进行中", completed: "已完成", failed: "失败", cancelled: "已取消" };
export function renderTaskCenter(tasks: TaskState[]): string {
  if (!tasks.length) return "";
  return `<aside class="task-center" aria-label="后台任务" aria-live="polite"><h2>任务状态</h2>${tasks.map((task) => {const id=esc(task.id);const failed=task.phase === "failed";const progress=Number.isFinite(task.progress)?Math.max(0,Math.min(100,Number(task.progress))):undefined;return `<section class="task-item task-item-${task.phase}" data-task-id="${id}"${failed?' role="alert"':''}><div><strong>${failed?'操作未完成：':''}${esc(task.label)}</strong><span class="status">${esc(labels[task.phase])}</span></div><p>${esc(task.message)}</p>${progress === undefined ? "" : `<progress max="100" value="${progress}">${progress}%</progress>`}${["started", "progress"].includes(task.phase) ? `<button class="ghost" data-cancel-task="${id}">取消</button>` : ""}${task.phase === "failed" && task.retryable ? `<button class="secondary" data-retry-task="${id}">重试</button>` : ""}${["completed", "failed", "cancelled"].includes(task.phase) ? `<button class="ghost" data-dismiss-task="${id}">关闭</button>` : ""}</section>`}).join("")}</aside>`;
}
