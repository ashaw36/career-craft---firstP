import {api,unwrap} from "../../api/client";
import type {Json} from "../../api/contracts";
import type {UrlActions,WorkspaceDataSource} from "../../api/data-source";
import type {TaskStore} from "../../shared/state/tasks";
const obj=(v:Json)=>v as Record<string,unknown>;
export function bind(root:HTMLElement,source:WorkspaceDataSource&UrlActions,tasks:TaskStore){
 const form=root.querySelector<HTMLFormElement>("[data-job-form]");if(!form)return;
 const box=document.createElement("div");
 const label=document.createElement("label");label.textContent="岗位网址";
 const input=document.createElement("input");input.type="url";input.placeholder="https://...";label.append(input);
 const button=document.createElement("button");button.type="button";button.className="secondary";button.textContent="读取网址";
 const note=document.createElement("p");note.hidden=true;note.setAttribute("role","status");note.setAttribute("aria-live","polite");note.textContent="网页需要登录或无法读取。可安全打开登录后，将岗位正文手动粘贴到下方文本框。";
 box.append(label,button,note);form.prepend(box);
 button.addEventListener("click",async()=>{const url=input.value.trim();try{
  const result=obj(await source.collectJobUrl(url));const text=typeof result.text==="string"?result.text:"";
  if(text){const area=form.querySelector<HTMLTextAreaElement>('[name="jdText"]');if(area)area.value=text;note.hidden=true;return}
  note.hidden=false;note.textContent=`${String(result.reason??"网页需要登录或无法读取")}。请登录后将岗位正文手动粘贴到下方。`;const token=typeof result.openToken==="string"?result.openToken:"";
  if(token)unwrap(await api.openExternalUrl(token));
 }catch(error){note.hidden=false;tasks.upsert({id:crypto.randomUUID(),label:"读取岗位网址",phase:"failed",message:(error as Error).message,retryable:true})}});
}
