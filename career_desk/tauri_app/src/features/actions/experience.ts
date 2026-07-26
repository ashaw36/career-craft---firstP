import type { ExperienceActions, WorkspaceDataSource } from "../../api/data-source";
import type { TaskStore } from "../../shared/state/tasks";

const report = (tasks: TaskStore, label: string, phase: "completed" | "failed", message: string) =>
  tasks.upsert({
    id: crypto.randomUUID(),
    label,
    phase,
    message,
    ...(phase === "completed" ? { progress: 100 } : { retryable: true }),
  });

const errorMessage = (error: unknown) =>
  typeof error === "object" && error !== null && "message" in error
    ? String((error as { message: unknown }).message)
    : "操作失败，请重试。";

function applyExperienceFilters(root: HTMLElement) {
  const query =
    root.querySelector<HTMLInputElement>("[data-experience-search]")?.value.trim().toLowerCase() ??
    "";
  const kind = root.querySelector<HTMLSelectElement>("[data-experience-kind]")?.value ?? "";
  const rows = [...root.querySelectorAll<HTMLElement>(".rows .content-row[data-experience-id]")];
  let visible = 0;
  for (const row of rows) {
    const text = (row.dataset.searchText ?? "").toLowerCase();
    const rowKind = row.dataset.kind ?? "";
    const matchQuery = !query || text.includes(query);
    const matchKind = !kind || rowKind === kind;
    const show = matchQuery && matchKind;
    row.hidden = !show;
    row.classList.toggle("is-filtered-out", !show);
    if (show) visible += 1;
  }
  const empty = root.querySelector<HTMLElement>("[data-experience-empty]");
  if (empty) {
    const showEmpty = visible === 0 && rows.length > 0;
    empty.hidden = !showEmpty;
    empty.classList.toggle("is-filtered-out", !showEmpty);
  }
}

export function bind(
  root: HTMLElement,
  source: WorkspaceDataSource & ExperienceActions,
  tasks: TaskStore,
  reload: () => Promise<void>,
  onImportRaw?: (raw: string) => void,
) {
  root.querySelector<HTMLElement>('[data-action="import-experience"]')?.addEventListener("click", () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".txt,.md,.json,.pdf,.docx";
    input.addEventListener("change", async () => {
      const file = input.files?.[0];
      if (!file) return;
      try {
        // Extract only; open AI structure dialog instead of committing a raw row first.
        const imported = await source.importFile(file, { commit: false });
        const raw = imported.content.trim();
        if (!raw) throw new Error("文件中没有可整理的经历文本");
        report(tasks, "导入经历", "completed", `已读取 ${file.name}，正在 AI 整理…`);
        onImportRaw?.(raw);
      } catch (error) {
        report(tasks, "导入经历", "failed", errorMessage(error));
      }
    });
    input.click();
  });

  const search = root.querySelector<HTMLInputElement>("[data-experience-search]");
  const kind = root.querySelector<HTMLSelectElement>("[data-experience-kind]");
  let timer = 0;
  const runFilter = () => applyExperienceFilters(root);
  search?.addEventListener("input", () => {
    clearTimeout(timer);
    timer = window.setTimeout(runFilter, 120);
  });
  search?.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      clearTimeout(timer);
      runFilter();
    }
  });
  kind?.addEventListener("change", runFilter);
  root.querySelector<HTMLElement>("[data-experience-filter]")?.addEventListener("click", runFilter);
  applyExperienceFilters(root);
}
