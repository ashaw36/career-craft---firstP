import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, unwrap } from "../../api/client";
import type { ResumeActions, ResumeInstructionType, WorkspaceDataSource } from "../../api/data-source";
import type { TaskStore } from "../../shared/state/tasks";
import { escapeHtml as esc } from "../../shared/html";
import { selectedPersonaId } from "./persona";
import { getResumeUiState, paintResumePaper } from "./resume";
import { RefinementTaskController, type RefinementSnapshot } from "./resume-refinement";

type Source = WorkspaceDataSource & ResumeActions;
type Obj = Record<string, unknown>;

interface UiState {
  controller?: RefinementTaskController;
  last?: { personaId: string; instruction: string; instructionType: ResumeInstructionType };
  proposal?: Obj;
}

const states = new WeakMap<HTMLElement, UiState>();
const report = (tasks: TaskStore, label: string, phase: "completed" | "failed", message: string) =>
  tasks.upsert({
    id: crypto.randomUUID(),
    label,
    phase,
    message,
    ...(phase === "completed" ? { progress: 100 } : { retryable: true }),
  });

async function exportMarkdownFile(markdown: string, defaultName: string) {
  const destinationPath = await saveDialog({
    defaultPath: defaultName,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  if (!destinationPath) return false;
  unwrap(await api.command("write_text_file", { destinationPath, content: markdown }));
  return true;
}

const types: [ResumeInstructionType, string, string][] = [
  ["leadership", "领导力", "突出带队、协作和影响力（中文输出）"],
  ["metrics", "量化成果", "强化已有数字和业务结果（中文输出）"],
  ["concise", "精炼表达", "压缩冗余并提高可读性（中文输出）"],
  ["technical_depth", "技术深度", "突出已有技术方案与复杂度（中文输出）"],
  ["job_alignment", "岗位对齐", "围绕目标岗位调整侧重点（中文输出）"],
  ["general", "通用（英文）", "General refinement request in English"],
];

const meta = (s: RefinementSnapshot) =>
  [
    ["服务商", s.meta.provider],
    ["模型", s.meta.model],
    ["回退", s.meta.fallback === undefined ? undefined : s.meta.fallback ? "是" : "否"],
    ["缓存", s.meta.cachePolicy],
    ["提示词", s.meta.promptVersion],
  ]
    .filter((x): x is [string, string] => typeof x[1] === "string")
    .map(([k, v]) => `<span><strong>${esc(k)}</strong> ${esc(v)}</span>`)
    .join("");

export function bind(root: HTMLElement, source: Source, tasks: TaskStore, reload?: () => Promise<void>) {
  const old = states.get(root);
  old?.controller?.invalidate();
  const state: UiState = {};
  states.set(root, state);

  const persona = async () => {
    const values = (await source.load()).personas;
    const select = root.querySelector<HTMLSelectElement>("[data-resume-persona]");
    const preferred = select?.value || selectedPersonaId();
    return values.find((x) => x.id === preferred) ?? values[0];
  };

  root.querySelectorAll<HTMLElement>('[data-action="export-markdown"]').forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        const p = await persona();
        if (!p) throw new Error("请先创建角色");
        const live = getResumeUiState(root);
        let markdown = live?.personaId === p.id ? live.markdown : undefined;
        if (!markdown) {
          const versions = await source.listResumeVersions(p.id);
          const latest = versions.at(-1) as Obj | undefined;
          markdown = typeof latest?.markdown === "string" ? latest.markdown : undefined;
        }
        if (!markdown) {
          const preview = await source.previewResume(p.id, "classic");
          markdown = preview.markdown;
        }
        if (!markdown) throw new Error("当前角色还没有可导出的预览或已保存版本");
        const saved = await exportMarkdownFile(markdown, `resume-${p.name || p.id}.md`);
        if (!saved) return;
        report(tasks, "导出 Markdown", "completed", "Markdown 已保存到本地。");
      } catch (error) {
        report(tasks, "导出 Markdown", "failed", (error as Error).message);
      }
    });
  });

  root.querySelectorAll<HTMLElement>('[data-action="resume-refine"]').forEach((button) =>
    button.addEventListener("click", () => showDialog()),
  );

  function showDialog() {
    root.querySelector("[data-refine-dialog]")?.remove();
    root.insertAdjacentHTML(
      "beforeend",
      `<div class="modal-backdrop" data-refine-dialog>
        <form class="onboarding form-dialog refine-dialog" data-refine-form role="dialog" aria-modal="true" aria-labelledby="refine-title">
          <header class="refine-header">
            <p class="eyebrow">AI 简历调优</p>
            <h1 id="refine-title">按经历调优表达</h1>
            <p class="helper">会改写各段经历要点与摘要侧重点，不编造事实；确认后才保存为新版本。经历库原文不受影响。除「通用（英文）」外默认中文输出。</p>
          </header>
          <div class="refine-layout">
            <section class="refine-controls">
              <fieldset class="refine-types">
                <legend>调优方向</legend>
                ${types
                  .map(
                    ([value, label, hint], index) =>
                      `<label class="refine-type-card"><input type="radio" name="instructionType" value="${value}" ${index === 0 ? "checked" : ""}><span class="refine-type-label">${esc(label)}</span><small>${esc(hint)}</small></label>`,
                  )
                  .join("")}
              </fieldset>
              <label>具体要求<textarea name="instruction" required rows="5" placeholder="例如：把成果写得更量化，保持原有事实不变"></textarea></label>
            </section>
            <section class="refine-preview-pane">
              <div class="refine-preview-head"><strong>预览结果</strong><span class="helper">流式生成会出现在这里</span></div>
              <div data-refine-output aria-live="polite" class="refine-output-empty"><p class="helper">提交后显示调优预览。</p></div>
            </section>
          </div>
          <div class="dialog-actions">
            <button type="button" class="ghost" data-close-refine>关闭</button>
            <button class="primary">生成预览</button>
          </div>
        </form>
      </div>`,
    );
    const dialog = root.querySelector<HTMLElement>("[data-refine-dialog]")!;
    dialog.querySelector("[data-close-refine]")?.addEventListener("click", () => {
      state.controller?.invalidate();
      dialog.remove();
    });
    dialog.querySelector<HTMLFormElement>("[data-refine-form]")?.addEventListener("submit", (e) => void submit(e, dialog));
  }

  async function submit(event: SubmitEvent, dialog: HTMLElement) {
    event.preventDefault();
    const values = new FormData(event.currentTarget as HTMLFormElement);
    const p = await persona();
    if (!p) return;
    const instruction = String(values.get("instruction") ?? "").trim();
    const instructionType = String(values.get("instructionType")) as ResumeInstructionType;
    state.last = { personaId: p.id, instruction, instructionType };
    state.proposal = undefined;
    state.controller = new RefinementTaskController(source, (snapshot) => renderSnapshot(dialog, snapshot));
    await state.controller.start(p.id, instruction, instructionType);
  }

  function renderSnapshot(dialog: HTMLElement, s: RefinementSnapshot) {
    const output = dialog.querySelector<HTMLElement>("[data-refine-output]");
    if (!output) return;
    output.className = "refine-output";
    const busy = ["started", "progress"].includes(s.phase);
    const previewText =
      s.phase === "completed" && typeof s.result?.refinedSummary === "string"
        ? String(s.result.refinedSummary)
        : s.text;
    output.innerHTML = `<section class="refine-result">
      <p class="refine-message" role="status">${esc(s.message)}</p>
      ${
        s.progress === undefined && busy
          ? '<p class="helper">进度未知，正在等待真实响应。</p>'
          : s.progress === undefined
            ? ""
            : `<progress max="100" value="${s.progress}">${s.progress}%</progress>`
      }
      <pre data-stream-output class="refine-stream">${esc(previewText || "（尚无文本）")}</pre>
      <div class="refine-meta">${meta(s)}</div>
      <div class="refine-result-actions">
        ${busy ? '<button type="button" class="ghost" data-cancel-refine>取消</button>' : ""}
        ${s.phase === "failed" && s.retryable ? '<button type="button" class="secondary" data-retry-refine>重试</button>' : ""}
        ${s.phase === "completed" ? '<button type="button" class="primary" data-confirm-refine>确认并保存为新版本</button>' : ""}
      </div>
    </section>`;
    output.querySelector("[data-cancel-refine]")?.addEventListener("click", () => void state.controller?.cancel());
    output.querySelector("[data-retry-refine]")?.addEventListener("click", () => {
      const x = state.last;
      if (x) {
        state.controller = new RefinementTaskController(source, (v) => renderSnapshot(dialog, v));
        void state.controller.start(x.personaId, x.instruction, x.instructionType);
      }
    });
    if (s.phase === "completed") {
      state.proposal = s.result;
      output.querySelector("[data-confirm-refine]")?.addEventListener("click", () => void confirmProposal(dialog));
    }
  }

  async function confirmProposal(dialog: HTMLElement) {
    const request = state.last;
    const proposal = state.proposal;
    if (!request || !proposal) return;
    const preview = String(proposal.refinedSummary ?? proposal.refinedPreview ?? "");
    const base = String(proposal.baseVersionId ?? "");
    const proposalId = String(proposal.proposalId ?? "");
    const contentHash = String(proposal.contentHash ?? "");
    if (!preview || !base || !proposalId || !contentHash) {
      report(tasks, "简历调优", "failed", "预览凭证不完整，请重新生成");
      return;
    }
    const confirmBtn = dialog.querySelector<HTMLButtonElement>("[data-confirm-refine]");
    if (confirmBtn) {
      confirmBtn.disabled = true;
      confirmBtn.textContent = "保存中…";
    }
    try {
      const saved = (await source.chatRefineResume(
        request.personaId,
        request.instruction,
        true,
        undefined,
        base,
        request.instructionType,
        proposalId,
        contentHash,
      )) as Record<string, unknown>;
      dialog.remove();
      const markdown = typeof saved.markdown === "string" ? saved.markdown : "";
      if (markdown) {
        paintResumePaper(root, markdown, {
          personaId: request.personaId,
          template: typeof saved.template === "string" ? (saved.template as import("../../api/contracts").ResumeTemplateId) : undefined,
          contentHash: typeof saved.contentHash === "string" ? saved.contentHash : undefined,
          versionId: typeof saved.versionId === "string" ? saved.versionId : undefined,
          revision: saved.revision as string | number | undefined,
        });
      }
      await reload?.();
      if (markdown) {
        paintResumePaper(root, markdown, {
          personaId: request.personaId,
          template: typeof saved.template === "string" ? (saved.template as import("../../api/contracts").ResumeTemplateId) : undefined,
          contentHash: typeof saved.contentHash === "string" ? saved.contentHash : undefined,
          versionId: typeof saved.versionId === "string" ? saved.versionId : undefined,
          revision: saved.revision as string | number | undefined,
        });
      } else {
        const versions = await source.listResumeVersions(request.personaId);
        const latest = versions.at(-1) as Record<string, unknown> | undefined;
        const fallback = typeof latest?.markdown === "string" ? latest.markdown : "";
        if (fallback) {
          paintResumePaper(root, fallback, {
            personaId: request.personaId,
            template: typeof latest?.template === "string" ? (latest.template as import("../../api/contracts").ResumeTemplateId) : undefined,
            contentHash: typeof latest?.contentHash === "string" ? latest.contentHash : undefined,
            versionId: String(latest?.versionId ?? latest?.id ?? ""),
            revision: latest?.revision as string | number | undefined,
          });
        }
      }
      report(tasks, "简历调优", "completed", "已按经历确认调优并保存新版本。");
    } catch (error) {
      if (confirmBtn) {
        confirmBtn.disabled = false;
        confirmBtn.textContent = "确认并保存为新版本";
      }
      report(tasks, "简历调优", "failed", (error as { message?: string }).message ?? "保存失败，请重试");
    }
  }
}
