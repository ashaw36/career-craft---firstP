import type { Job, SkillLearningActions, WorkspaceDataSource } from "../../api/data-source";
import type { TaskStore } from "../../shared/state/tasks";
import { escapeHtml as esc } from "../../shared/html";
import { renderWhatIfDialog } from "../pages";

type Source = WorkspaceDataSource & SkillLearningActions;
const report = (tasks: TaskStore, label: string, phase: "completed" | "failed", message: string) =>
  tasks.upsert({
    id: crypto.randomUUID(),
    label,
    phase,
    message,
    ...(phase === "completed" ? { progress: 100 } : { retryable: true }),
  });

function evidenceHtml(job: Job) {
  const breakdown = job.scoreBreakdown;
  const sources = job.evidenceSources ?? {};
  const rows = breakdown
    ? `<ul class="job-breakdown"><li>技能 ${breakdown.skills}</li><li>经历 ${breakdown.experience}</li><li>行业 ${breakdown.industry}</li><li>学历 ${breakdown.education}</li></ul>`
    : `<p class="helper">暂无分项数据。</p>`;
  const provenance = Object.keys(sources).length
    ? Object.entries(sources)
        .map(
          ([key, value]) =>
            `<p>${esc(key)}：<span class="status-label">${value === "legacy_heuristic" ? "旧数据启发式推断（非结构化事实）" : "结构化持久证据"}</span></p>`,
        )
        .join("")
    : `<p class="helper">暂无证据来源说明。</p>`;
  return `<div class="modal-backdrop" data-job-evidence-dialog><section class="onboarding form-dialog" role="dialog" aria-modal="true"><h1>分项与证据来源</h1><h2>匹配分项</h2>${rows}<h2>证据来源</h2>${provenance}<div class="dialog-actions"><button type="button" class="primary" data-close-job-evidence>完成</button></div></section></div>`;
}

export function bind(
  root: HTMLElement,
  source: Source,
  tasks: TaskStore,
  reload: () => Promise<void>,
  navigate?: (route: "learning") => void,
) {
  const persist = (id: string) => localStorage.setItem("careercraft:selected-job", id);

  root.querySelectorAll<HTMLElement>("[data-select-job]").forEach((node) => {
    node.addEventListener("click", async () => {
      const id = node.dataset.selectJob ?? "";
      if (!id) return;
      persist(id);
      await reload();
    });
  });

  root.querySelector<HTMLSelectElement>("[data-job-persona]")?.addEventListener("change", async (event) => {
    const id = (event.currentTarget as HTMLSelectElement).value;
    localStorage.setItem("careercraft:selected-persona", id);
    await reload();
  });

  root.querySelectorAll<HTMLElement>("[data-job-evidence]").forEach((node) => {
    node.addEventListener("click", async () => {
      const id = node.dataset.jobEvidence ?? "";
      const job = (await source.load()).jobs.find((x) => x.id === id);
      if (!job) return;
      root.insertAdjacentHTML("beforeend", evidenceHtml(job));
      root.querySelector("[data-close-job-evidence]")?.addEventListener("click", () =>
        root.querySelector("[data-job-evidence-dialog]")?.remove(),
      );
    });
  });

  root.querySelectorAll<HTMLElement>("[data-learn-skill]").forEach((node) => {
    node.addEventListener("click", async () => {
      const skill = node.dataset.learnSkill ?? "";
      if (skill) {
        const jobId = node.closest<HTMLElement>("[data-job-detail]")?.dataset.jobDetail ?? "";
        const snapshot = await source.load();
        const job = snapshot.jobs.find(item => item.id === jobId);
        const selected = localStorage.getItem("careercraft:selected-persona");
        const personaId = job?.personaId ?? (snapshot.personas.some(p => p.id === selected) ? selected : snapshot.personas[0]?.id) ?? "";
        localStorage.setItem("careercraft:learning-context", JSON.stringify({ skill, personaId, jobMatchId: job?.matchId ?? "", origin: "job_gap" }));
        localStorage.setItem("careercraft:focus-skill", skill);
      }
      report(tasks, "安排学习", "completed", skill ? `已记下「${skill}」，可到学习路径生成计划。` : "请到学习路径继续。");
      navigate?.("learning");
    });
  });

  root.querySelectorAll<HTMLElement>("[data-job-what-if-legacy]").forEach((node) => {
    node.addEventListener("click", async () => {
      const id = node.dataset.jobWhatIf ?? "";
      const job = (await source.load()).jobs.find((x) => x.id === id);
      if (!job) return;
      const required = [...job.matched, ...job.missing].join(", ");
      const current = job.matched.join(", ");
      const hypothetical = job.missing[0] ?? "";
      root.insertAdjacentHTML("beforeend", renderWhatIfDialog({ required, current, hypothetical }));
      const form = root.querySelector<HTMLFormElement>("[data-what-if-form]");
      if (!form) return;
      const close = () => root.querySelector(".modal-backdrop:last-child")?.remove();
      form.querySelector("[data-close-overlay]")?.addEventListener("click", close);
      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        const values = new FormData(form);
        const list = (name: string) =>
          String(values.get(name) ?? "")
            .split(/[,，]/)
            .map((x) => x.trim())
            .filter(Boolean);
        try {
          const result = await source.simulateWhatIf(list("required"), list("current"), list("hypothetical"));
          report(tasks, "假设分析", "completed", `预计匹配变化：${JSON.stringify(result)}`);
          close();
        } catch (error) {
          report(tasks, "假设分析", "failed", (error as Error).message);
        }
      });
      // Live preview like skills page
      const output = document.createElement("output");
      output.dataset.whatIfResult = "true";
      output.setAttribute("aria-live", "polite");
      form.querySelector(".dialog-actions")?.before(output);
      let timer = 0;
      let request = 0;
      form.addEventListener("input", () => {
        clearTimeout(timer);
        const currentReq = ++request;
        timer = window.setTimeout(async () => {
          const values = new FormData(form);
          const list = (name: string) =>
            String(values.get(name) ?? "")
              .split(/[,，]/)
              .map((x) => x.trim())
              .filter(Boolean);
          try {
            const result = (await source.simulateWhatIf(
              list("required"),
              list("current"),
              list("hypothetical"),
            )) as Record<string, unknown>;
            if (currentReq === request) output.textContent = `实时变化：${String(result.delta ?? 0)} 分`;
          } catch (error) {
            if (currentReq === request) output.textContent = `暂时无法计算：${(error as Error).message}`;
          }
        }, 150);
      });
    });
  });

  root.querySelectorAll<HTMLElement>("[data-job-what-if]").forEach(node => node.addEventListener("click", async () => {
    const snapshot = await source.load();
    const job = snapshot.jobs.find(x => x.id === node.dataset.jobWhatIf);
    const personaId = job?.personaId ?? localStorage.getItem("careercraft:selected-persona") ?? snapshot.personas[0]?.id;
    if (!job?.matchId || !personaId) { report(tasks,"假设分析","failed","请先选择角色并完成一次岗位匹配分析。"); return; }
    const requestedSkill = localStorage.getItem("careercraft:what-if-skill") ?? "";
    const hypothetical = job.missing.some(skill => skill.toLowerCase() === requestedSkill.toLowerCase()) ? requestedSkill : job.missing[0] ?? "";
    localStorage.removeItem("careercraft:what-if-skill");
    root.insertAdjacentHTML("beforeend", renderWhatIfDialog({required:[...job.matched,...job.missing].join(", "),current:job.matched.join(", "),hypothetical}));
    const form=root.querySelector<HTMLFormElement>("[data-what-if-form]"); if(!form)return;
    form.querySelector("[data-close-overlay]")?.addEventListener("click",()=>form.closest(".modal-backdrop")?.remove());
    const output=document.createElement("output"); output.dataset.whatIfResult="true"; output.setAttribute("aria-live","polite"); form.querySelector(".dialog-actions")?.before(output);
    form.querySelectorAll<HTMLInputElement>('[name="required"],[name="current"]').forEach(input=>{input.readOnly=true;input.setAttribute("aria-readonly","true")});
    let request=0,timer=0,last:Record<string,unknown>|undefined;
    const skills=()=>String(new FormData(form).get("hypothetical")??"").split(/[,，]/).map(x=>x.trim()).filter(Boolean);
    const calculate=async()=>{const own=++request;try{const result=await source.simulateJobWhatIf(personaId,job.matchId!,skills()) as Record<string,unknown>;if(own!==request)return;last=result;const before=result.baselineBreakdown as Record<string,unknown>|undefined,after=result.simulatedBreakdown as Record<string,unknown>|undefined,missing=Array.isArray(result.remainingMissing)?result.remainingMissing.map(String):[];output.textContent=`总匹配 ${result.baselineScore} → ${result.simulatedScore}（${Number(result.delta)>=0?"+":""}${result.delta}）；技能分项 ${before?.skills??0} → ${after?.skills??0}${missing.length?`；仍缺：${missing.join("、")}`:"；岗位技能要求已覆盖"}。假设掌握不等于已有项目证据。`;}catch(error){if(own===request)output.textContent=`暂时无法计算：${(error as Error).message}`}};
    form.addEventListener("input",()=>{clearTimeout(timer);timer=window.setTimeout(calculate,150)});
    form.addEventListener("submit",event=>{event.preventDefault();void calculate()});
    const confirm=document.createElement("button");confirm.type="button";confirm.className="primary";confirm.dataset.confirmWhatIfLearning="true";confirm.textContent="据此生成学习路径";form.querySelector(".dialog-actions")?.append(confirm);
    confirm.addEventListener("click",async()=>{if(!last)await calculate();const chosen=Array.isArray(last?.addedSkills)?last!.addedSkills.map(String):skills();if(!chosen.length)return;localStorage.setItem("careercraft:learning-context",JSON.stringify({skill:chosen[0],skills:chosen,personaId,jobMatchId:job.matchId,origin:"what_if"}));localStorage.setItem("careercraft:focus-skill",chosen[0]!);form.closest(".modal-backdrop")?.remove();navigate?.("learning")});
    void calculate();
  }));

  // Keep selected job id in sync when page first paints with a default.
  const selected = root.querySelector<HTMLElement>("[data-job-detail]");
  if (selected?.dataset.jobDetail) persist(selected.dataset.jobDetail);
}
