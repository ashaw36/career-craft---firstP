import type { Json, ResumeTemplateId } from "../../api/contracts";
import type { ResumeActions, WorkspaceDataSource } from "../../api/data-source";
import { escapeHtml as esc } from "../../shared/html";
import type { TaskStore } from "../../shared/state/tasks";
import { selectedPersonaId } from "./persona";

const obj = (v: Json) => v as Record<string, unknown>;
const DEFAULT_TEMPLATE: ResumeTemplateId = "classic";
const report = (t: TaskStore, l: string, p: "completed" | "failed", m: string) =>
  t.upsert({
    id: crypto.randomUUID(),
    label: l,
    phase: p,
    message: m,
    ...(p === "completed" ? { progress: 100 } : { retryable: true }),
  });

export interface ResumeUiState {
  personaId?: string;
  template: ResumeTemplateId;
  markdown?: string;
  contentHash?: string;
  selectedVersionId?: string;
  mode: "empty" | "preview" | "saved";
  request: number;
}

const states = new WeakMap<HTMLElement, ResumeUiState>();
export const getResumeUiState = (root: HTMLElement) => states.get(root);

function setStatus(root: HTMLElement, text: string) {
  const node = root.querySelector<HTMLElement>("[data-resume-status]");
  if (node) node.textContent = text;
}

function paintPreview(
  root: HTMLElement,
  state: ResumeUiState,
  markdown: string,
  meta: { personaId: string; template?: ResumeTemplateId; contentHash?: string },
) {
  const host =
    root.querySelector<HTMLElement>(".resume-preview") ??
    root.querySelector<HTMLElement>(".empty-state");
  if (!host) return;
  host.textContent = markdown;
  host.className = "resume-preview";
  host.dataset.personaId = meta.personaId;
  const template = meta.template ?? state.template;
  host.dataset.template = template;
  if (meta.contentHash) host.dataset.contentHash = meta.contentHash;
  else delete host.dataset.contentHash;
  state.personaId = meta.personaId;
  state.template = template;
  state.markdown = markdown;
  state.contentHash = meta.contentHash;
}

/** Paint a saved/generated version onto the paper without waiting for a full page reload. */
export function paintResumePaper(
  root: HTMLElement,
  markdown: string,
  meta: {
    personaId: string;
    template?: ResumeTemplateId;
    contentHash?: string;
    versionId?: string;
    revision?: string | number;
  },
) {
  const state = states.get(root) ?? {
    template: DEFAULT_TEMPLATE,
    request: 0,
    mode: "empty" as const,
  };
  states.set(root, state);
  paintPreview(root, state, markdown, meta);
  state.selectedVersionId = meta.versionId;
  state.mode = "saved";
  setStatus(
    root,
    meta.revision !== undefined && meta.revision !== ""
      ? `已保存 · 第 ${String(meta.revision)} 版`
      : "已保存",
  );
}

function clearPreview(root: HTMLElement, state: ResumeUiState, message: string) {
  const host =
    root.querySelector<HTMLElement>(".resume-preview") ??
    root.querySelector<HTMLElement>(".empty-state");
  if (host) {
    host.className = "empty-state";
    host.innerHTML = `<h2>还没有简历预览</h2><p>${esc(message)}</p>`;
    delete host.dataset.personaId;
    delete host.dataset.template;
    delete host.dataset.contentHash;
  }
  state.markdown = undefined;
  state.contentHash = undefined;
  state.mode = "empty";
  setStatus(root, "尚未预览或保存");
}

function hydrateFromDom(root: HTMLElement, state: ResumeUiState) {
  const host = root.querySelector<HTMLElement>(".resume-preview");
  const markdown = host?.textContent?.trim();
  if (!host || !markdown) return false;
  state.markdown = markdown;
  state.personaId = host.dataset.personaId || state.personaId;
  state.template = (host.dataset.template as ResumeTemplateId) || state.template;
  state.contentHash = host.dataset.contentHash;
  state.mode = "saved";
  return true;
}

export function bind(
  root: HTMLElement,
  source: WorkspaceDataSource & ResumeActions,
  tasks: TaskStore,
  reload: () => Promise<void>,
) {
  const state = states.get(root) ?? {
    template: DEFAULT_TEMPLATE,
    request: 0,
    mode: "empty" as const,
  };
  states.set(root, state);

  const loadPersonaPreview = async (personaId: string) => {
    const request = ++state.request;
    const hadDom = hydrateFromDom(root, state);
    try {
      const versions = await source.listResumeVersions(personaId);
      if (request !== state.request) return;
      const latest = versions.at(-1);
      if (latest) {
        const v = obj(latest);
        const markdown = typeof v.markdown === "string" ? v.markdown : "";
        if (markdown) {
          paintPreview(root, state, markdown, {
            personaId,
            template: (String(v.template || DEFAULT_TEMPLATE) as ResumeTemplateId) || DEFAULT_TEMPLATE,
            contentHash: typeof v.contentHash === "string" ? v.contentHash : undefined,
          });
          state.selectedVersionId = String(v.versionId ?? v.id ?? "");
          state.mode = "saved";
          setStatus(root, `已保存 · 第 ${String(v.revision ?? v.version ?? "?")} 版`);
          return;
        }
      }
      const preview = await source.previewResume(personaId, state.template || DEFAULT_TEMPLATE);
      if (request !== state.request) return;
      paintPreview(root, state, preview.markdown, {
        personaId: preview.personaId,
        template: preview.template,
        contentHash: preview.contentHash,
      });
      state.selectedVersionId = undefined;
      state.mode = "preview";
      setStatus(root, "实时预览 · 未保存为新版本");
    } catch {
      if (request !== state.request) return;
      // Keep SSR/DOM paper if we already have something visible; only clear when empty.
      if (!hadDom && !state.markdown) {
        clearPreview(root, state, "该角色还没有简历。可直接点「生成并保存」。");
      }
    }
  };

  const persona = async (forceLoad = false) => {
    const data = await source.load();
    const select = root.querySelector<HTMLSelectElement>("[data-resume-persona]");
    const fromSelect = select?.value || selectedPersonaId();
    const p = data.personas.find((x) => x.id === fromSelect) ?? data.personas[0];
    if (!p) return p;
    const changed = state.personaId !== p.id;
    state.personaId = p.id;
    localStorage.setItem("careercraft:selected-persona", p.id);
    if (select && select.value !== p.id) select.value = p.id;
    if (forceLoad || changed || !state.markdown) {
      await loadPersonaPreview(p.id);
    }
    return p;
  };

  root.querySelector<HTMLSelectElement>("[data-resume-persona]")?.addEventListener("change", async (event) => {
    const id = (event.currentTarget as HTMLSelectElement).value;
    localStorage.setItem("careercraft:selected-persona", id);
    state.personaId = undefined;
    state.markdown = undefined;
    state.selectedVersionId = undefined;
    await persona(true);
  });

  root.querySelector<HTMLElement>('[data-action="generate-resume"]')?.addEventListener("click", async () => {
    try {
      const p = await persona();
      if (!p) throw new Error("请先创建角色");
      setStatus(root, "正在按经历权重生成并保存…");
      const result = obj(await source.generateResume(p.id, state.template || DEFAULT_TEMPLATE));
      const markdown = typeof result.markdown === "string" ? result.markdown : "";
      if (markdown) {
        paintResumePaper(root, markdown, {
          personaId: p.id,
          template: (String(result.template || state.template) as ResumeTemplateId) || DEFAULT_TEMPLATE,
          contentHash: typeof result.contentHash === "string" ? result.contentHash : undefined,
          versionId: String(result.versionId ?? ""),
          revision: result.revision as string | number | undefined,
        });
      }
      await reload();
      if (!markdown) {
        state.markdown = undefined;
        await persona(true);
      } else if (markdown) {
        // reload() re-renders from snapshot; re-assert paper in case SSR lagged.
        paintResumePaper(root, markdown, {
          personaId: p.id,
          template: (String(result.template || state.template) as ResumeTemplateId) || DEFAULT_TEMPLATE,
          contentHash: typeof result.contentHash === "string" ? result.contentHash : undefined,
          versionId: String(result.versionId ?? ""),
          revision: result.revision as string | number | undefined,
        });
      }
      report(tasks, "生成并保存", "completed", "已按经历权重生成并保存新版本（高权重经历保留更多要点）。");
    } catch (error) {
      report(tasks, "生成并保存", "failed", (error as Error).message || "生成失败");
    }
  });

  root.querySelector<HTMLElement>("[data-resume-versions]")?.addEventListener("click", () => void openVersions());

  async function openVersions() {
    const p = await persona();
    if (!p) return report(tasks, "简历版本", "failed", "请先创建角色。");
    const versions = await source.listResumeVersions(p.id);
    root.querySelector("[data-resume-version-dialog]")?.remove();
    const rows = versions.length
      ? versions
          .map((raw) => {
            const v = obj(raw);
            const id = String(v.versionId ?? v.id ?? "");
            const revision = String(v.revision ?? v.version ?? "");
            const created = String(v.createdAt ?? "");
            const markdown = typeof v.markdown === "string" ? v.markdown : "";
            const selected = id === state.selectedVersionId ? " selected" : "";
            return `<button type="button" class="content-row${selected}" data-pick-version="${esc(id)}" data-version-markdown="${esc(markdown)}" data-version-revision="${esc(revision)}"><span><strong>第 ${esc(revision)} 版</strong><small>${esc(created)}</small></span></button>`;
          })
          .join("")
      : `<p class="empty-state">还没有已保存版本。先「生成并保存」。</p>`;
    root.insertAdjacentHTML(
      "beforeend",
      `<div class="modal-backdrop" data-resume-version-dialog><section class="onboarding form-dialog resume-version-dialog" role="dialog" aria-modal="true" aria-labelledby="resume-version-title"><h1 id="resume-version-title">版本管理</h1><p class="helper">每角色最多保留约 5 个版本。点选一版即可载入纸面预览。</p><div class="resume-version-list" data-version-list>${rows}</div><div class="dialog-actions"><button type="button" class="primary" data-close-versions>完成</button></div></section></div>`,
    );
    const dialog = root.querySelector<HTMLElement>("[data-resume-version-dialog]")!;
    dialog.querySelectorAll<HTMLElement>("[data-pick-version]").forEach((node) => {
      node.addEventListener("click", () => {
        state.selectedVersionId = node.dataset.pickVersion ?? "";
        dialog.querySelectorAll("[data-pick-version]").forEach((x) => x.classList.toggle("selected", x === node));
        const markdown = node.dataset.versionMarkdown ?? "";
        if (markdown) {
          paintPreview(root, state, markdown, { personaId: p.id });
          state.mode = "saved";
          setStatus(root, `已保存 · 第 ${node.dataset.versionRevision ?? "?"} 版`);
        }
      });
    });
    dialog.querySelector<HTMLElement>("[data-close-versions]")?.addEventListener("click", () => dialog.remove());
  }

  // Always hydrate paper when the resumes page binds (route enter / full re-render).
  void persona(true);
}
