import type { Json } from "../../api/contracts";
import type { ReframeActions, WorkspaceDataSource } from "../../api/data-source";
import type { TaskStore } from "../../shared/state/tasks";
import { escapeHtml as esc } from "../../shared/html";
const obj=(v:Json)=>v as Record<string,unknown>;
const fail=(t:TaskStore,e:unknown)=>t.upsert({id:crypto.randomUUID(),label:"定向重述",phase:"failed",message:(e as Error).message,retryable:true});
export function bind(root:HTMLElement,source:WorkspaceDataSource&ReframeActions,tasks:TaskStore,reload:()=>Promise<void>){
 root.querySelectorAll<HTMLElement>("[data-reframe-job]").forEach(node=>node.addEventListener("click",async()=>{try{
  const result=obj(await source.reframeJob(node.dataset.reframeJob??""));const rows=Array.isArray(result.reframes)?result.reframes as Json[]:[];
  root.insertAdjacentHTML("beforeend",`<div class="modal-backdrop"><section class="onboarding"><h1>定向重述</h1>${rows.map(v=>{const x=obj(v);return`<form data-reframe-form data-id="${esc(x.id)}"><textarea name="summary">${esc(x.reframedSummary)}</textarea><button>保存</button><button type="button" data-reset-reframe="${esc(x.id)}">重置</button></form>`}).join("")}<button data-close-reframe>关闭</button></section></div>`);
  root.querySelector<HTMLElement>("[data-close-reframe]")?.addEventListener("click",()=>root.querySelector(".modal-backdrop:last-child")?.remove());
  root.querySelectorAll<HTMLFormElement>("[data-reframe-form]").forEach(form=>form.addEventListener("submit",async e=>{e.preventDefault();try{await source.updateReframe(form.dataset.id??"",String(new FormData(form).get("summary")));await reload()}catch(error){fail(tasks,error)}}));
  root.querySelectorAll<HTMLElement>("[data-reset-reframe]").forEach(button=>button.addEventListener("click",async()=>{try{await source.resetReframe(button.dataset.resetReframe??"");await reload()}catch(error){fail(tasks,error)}}));
 }catch(error){fail(tasks,error)}}));
 root.querySelectorAll<HTMLSelectElement>("[data-job-status]").forEach(select=>{const button=document.createElement("button");button.className="ghost";button.dataset.jobHistory=select.dataset.jobStatus;button.textContent="完整状态历史";select.after(button);button.addEventListener("click",async()=>{try{const events=await source.getJobStatusEvents(button.dataset.jobHistory??"");root.insertAdjacentHTML("beforeend",`<div class="modal-backdrop"><section class="onboarding" role="dialog" aria-modal="true"><h1>岗位状态时间线</h1><ol>${events.map(event=>`<li>${esc(event.changedAt)} · ${esc(event.fromStatus??"初始")} → ${esc(event.toStatus)}</li>`).join("")}</ol><button data-close-job-history>关闭</button></section></div>`);root.querySelector("[data-close-job-history]")?.addEventListener("click",()=>root.querySelector(".modal-backdrop:last-child")?.remove())}catch(error){fail(tasks,error)}})});
}
