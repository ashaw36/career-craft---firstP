import {createDataSource,type JobActions,type ViewState,type WorkspaceDataSource} from "./api/data-source";
import {renderError,renderExperienceConfirmation,renderExperienceDialog,renderFitDialog,renderJobDialog,renderLoading,renderPage,renderPersonaDialog,renderProviderDialog,type ExperienceEntryState} from "./features/pages";
import type{Experience}from"./api/data-source";import type{StructurePreviewDto,StructuredExperienceDraftDto}from"./api/contracts";
import {completeOnboarding,needsOnboarding,renderOnboarding} from "./features/home/onboarding";
import {renderTaskCenter} from "./shared/components/task-center";
import {NavigationStore,routes,routeFromHash,type RouteId} from "./shared/state/navigation";
import {TaskStore} from "./shared/state/tasks";
import {bind as bindSkillsLearning} from "./features/actions/skills-learning";
import {bind as bindSettings,bindPortable} from "./features/actions/settings";
import {bind as bindResume} from "./features/actions/resume";
import {bind as bindReframe} from "./features/actions/reframe";
import {bind as bindExperience} from "./features/actions/experience";
import {bind as bindPersona} from "./features/actions/persona";
import {bind as bindJobUrl} from "./features/actions/job-url";
import {bind as bindResumeExtra} from "./features/actions/resume-extra";
export class App{
 readonly navigation=new NavigationStore();readonly tasks=new TaskStore();private state:ViewState={status:"loading"};private dialog=false;private overlay="";private readonly taskDeadlines=new Map<string,number>();
 private pendingExperience?:Experience;private experienceEntry:ExperienceEntryState={mode:"create",phase:"input",raw:""};private providerError="";
 constructor(private readonly root:HTMLElement,private readonly source:WorkspaceDataSource=createDataSource()){}
 mount(){this.render();void this.load();this.navigation.addEventListener("change",()=>this.render());this.tasks.addEventListener("change",()=>this.render());window.addEventListener("hashchange",()=>{const r=routeFromHash(location.hash);if(r!==this.navigation.current)this.navigation.navigate(r)})}
 private async load(){const keepSurface=this.state.status==="ready";if(!keepSurface){this.state={status:"loading"};this.render()}try{this.state={status:"ready",data:await this.source.load()};this.render()}catch(error){const e=error as Partial<Error>;this.state={status:"error",error:{code:"UNAVAILABLE",message:e.message??"Local service unavailable",retryable:true}};this.render()}}
 private content(){return this.state.status==="loading"?renderLoading():this.state.status==="error"?renderError(this.state.error.message):renderPage(this.navigation.current,this.state.data)}
 private render(){this.root.innerHTML=`<div class="shell"><aside class="sidebar"><div class="brand"><span class="brand-mark">C</span><span>CareerCraft</span></div><nav aria-label="主导航">${routes.map(r=>`<button class="nav-item${r.id===this.navigation.current?" active":""}" data-route="${r.id}" ${r.id===this.navigation.current?'aria-current="page"':""}><span aria-hidden="true">${r.icon}</span><span>${r.label}</span></button>`).join("")}</nav><p class="local-note">数据默认保存在本机</p></aside><main id="main-content" tabindex="-1">${this.content()}</main>${renderTaskCenter(this.tasks.list())}</div>${needsOnboarding()?renderOnboarding():""}${this.dialog?renderExperienceDialog(this.experienceEntry):""}${this.overlay}`;this.bind()}
 private bind(){
  this.root.querySelectorAll<HTMLElement>("[data-route]").forEach(n=>n.addEventListener("click",()=>this.navigation.navigate(n.dataset.route as RouteId)));
  this.root.querySelectorAll<HTMLElement>("[data-route-jump]").forEach(n=>n.addEventListener("click",()=>this.navigation.navigate(n.dataset.routeJump as RouteId)));
  this.root.querySelectorAll<HTMLElement>("[data-onboarding-start],[data-onboarding-skip]").forEach(n=>n.addEventListener("click",()=>{completeOnboarding();this.render()}));
  this.root.querySelectorAll<HTMLElement>("[data-dismiss-task]").forEach(n=>n.addEventListener("click",()=>this.tasks.dismiss(n.dataset.dismissTask??"")));
  this.root.querySelectorAll<HTMLElement>("[data-cancel-task]").forEach(n=>n.addEventListener("click",()=>void this.cancelTask(n.dataset.cancelTask??"")));
  this.root.querySelector<HTMLElement>('[data-action="retry-load"]')?.addEventListener("click",()=>void this.load());
  this.root.querySelectorAll<HTMLElement>('[data-action="add-experience"]').forEach(n=>n.addEventListener("click",()=>{this.experienceEntry={mode:"create",phase:"input",raw:""};this.dialog=true;this.render()}));
  this.root.querySelector<HTMLElement>("[data-close-dialog]")?.addEventListener("click",()=>{if(this.experienceEntry.taskId)void this.source.cancelTask?.(this.experienceEntry.taskId);this.dialog=false;this.render()});
  this.root.querySelector<HTMLElement>("[data-close-overlay]")?.addEventListener("click",()=>{this.overlay="";this.render()});
  this.root.querySelector<HTMLElement>('[data-action="add-persona"]')?.addEventListener("click",()=>{this.overlay=renderPersonaDialog();this.render()});
  this.root.querySelector<HTMLElement>('[data-action="add-provider"]')?.addEventListener("click",()=>{this.providerError="";this.overlay=renderProviderDialog(undefined,this.providerError);this.render()});
  this.root.querySelector<HTMLElement>('[data-action="analyze-job"]')?.addEventListener("click",()=>{this.overlay=renderJobDialog(this.snapshot()?.personas??[]);this.render()});
  this.root.querySelectorAll<HTMLElement>("[data-edit-persona]").forEach(n=>n.addEventListener("click",()=>{const p=this.snapshot()?.personas.find(x=>x.id===n.dataset.editPersona);if(p){this.overlay=renderPersonaDialog(p);this.render()}}));
  this.root.querySelectorAll<HTMLElement>("[data-delete-persona]").forEach(n=>n.addEventListener("click",()=>void this.removePersona(n.dataset.deletePersona??"")));
  this.root.querySelectorAll<HTMLElement>("[data-fit-persona]").forEach(n=>n.addEventListener("click",()=>void this.openFit(n.dataset.fitPersona??"")));
  this.root.querySelectorAll<HTMLElement>("[data-edit-provider]").forEach(n=>n.addEventListener("click",()=>{const p=this.snapshot()?.providers.find(x=>x.id===n.dataset.editProvider);if(p){this.providerError="";this.overlay=renderProviderDialog(p,this.providerError);this.render()}}));
  this.root.querySelectorAll<HTMLElement>("[data-edit-experience]").forEach(n=>n.addEventListener("click",()=>this.openEditExperience(n.dataset.editExperience??"")));
  this.root.querySelectorAll<HTMLElement>("[data-test-provider]").forEach(n=>n.addEventListener("click",()=>void this.testProvider(n.dataset.testProvider??"")));
  this.root.querySelector<HTMLFormElement>("[data-persona-form]")?.addEventListener("submit",e=>void this.savePersona(e));
  this.root.querySelector<HTMLFormElement>("[data-fit-form]")?.addEventListener("submit",e=>void this.saveFit(e));
  this.root.querySelector<HTMLFormElement>("[data-provider-form]")?.addEventListener("submit",e=>void this.saveProvider(e));
  this.root.querySelector<HTMLFormElement>("[data-job-form]")?.addEventListener("submit",e=>void this.saveJob(e));
  this.root.querySelectorAll<HTMLSelectElement>("[data-job-status]").forEach(n=>n.addEventListener("change",()=>void this.updateJobStatus(n.dataset.jobStatus??"",n.value)));
  this.root.querySelectorAll<HTMLElement>("[data-delete-job]").forEach(n=>n.addEventListener("click",()=>void this.deleteJob(n.dataset.deleteJob??"")));
  this.root.querySelectorAll<HTMLElement>("[data-delete-experience]").forEach(n=>n.addEventListener("click",()=>void this.deleteExperience(n.dataset.deleteExperience??"",Number(n.dataset.version??1))));
  this.root.querySelector<HTMLFormElement>("[data-experience-form]")?.addEventListener("submit",e=>void this.saveExperience(e));
  this.root.querySelector<HTMLElement>("[data-structure-experience]")?.addEventListener("click",()=>void this.structureExperience());
  this.root.querySelector<HTMLElement>("[data-cancel-structure]")?.addEventListener("click",()=>void this.cancelExperienceStructure());
  this.root.querySelector<HTMLFormElement>("[data-draft-confirm]")?.addEventListener("submit",e=>void this.confirmExperience(e));
  this.root.querySelector<HTMLElement>("[data-discard-draft]")?.addEventListener("click",()=>void this.discardExperience());
  this.root.querySelectorAll<HTMLElement>("[data-confirm-existing-draft]").forEach(n=>n.addEventListener("click",()=>void this.setExperienceStatus(n.dataset.confirmExistingDraft??"","confirmed")));
  this.root.querySelectorAll<HTMLElement>("[data-discard-existing-draft]").forEach(n=>n.addEventListener("click",()=>void this.setExperienceStatus(n.dataset.discardExistingDraft??"","discarded")));
  const ops:Record<string,string>={"generate-resume":"正在生成简历","analyze-job":"正在分析岗位","generate-learning":"正在生成学习路径","test-provider":"正在测试连接","import-experience":"正在导入经历"};
  this.root.querySelectorAll<HTMLButtonElement>("[data-action]").forEach(n=>{const action=n.dataset.action??"";const label=ops[action];if(label&&!['analyze-job','generate-resume','import-experience'].includes(action))n.addEventListener("click",()=>void this.demoTask(action,label));else if(!["add-persona","add-provider","add-experience","analyze-job","add-skill","generate-resume","import-experience","resume-refine","export-markdown"].includes(action)){n.disabled=true;n.title="该操作需要完整表单或选择上下文，当前版本暂不可用"}})
  bindSkillsLearning(this.root,this.source as WorkspaceDataSource&import("./api/data-source").SkillLearningActions,this.tasks,()=>this.load());
  bindSettings(this.root,this.source as WorkspaceDataSource&import("./api/data-source").V2Actions,this.tasks,()=>this.load());
  bindPortable(this.root,this.source as WorkspaceDataSource&import("./api/data-source").V2Actions,this.tasks);
  bindResume(this.root,this.source as WorkspaceDataSource&import("./api/data-source").ResumeActions,this.tasks,()=>this.load());
  bindReframe(this.root,this.source as WorkspaceDataSource&import("./api/data-source").ReframeActions,this.tasks,()=>this.load());
  bindExperience(this.root,this.source as WorkspaceDataSource&import("./api/data-source").ExperienceActions,this.tasks,()=>this.load(),raw=>void this.openImportedExperience(raw));
  bindPersona(this.root,this.tasks);
  bindJobUrl(this.root,this.source as WorkspaceDataSource&import("./api/data-source").UrlActions,this.tasks);
  bindResumeExtra(this.root,this.source as WorkspaceDataSource&import("./api/data-source").ResumeActions,this.tasks,()=>this.load());
  this.disableUnboundButtons();
 }
 private disableUnboundButtons(){const bound="[data-route],[data-route-jump],[data-onboarding-start],[data-onboarding-skip],[data-dismiss-task],[data-action],[data-close-dialog],[data-structure-experience],[data-cancel-structure],[data-manual-experience],[data-save-structured],[data-resume-version],[data-resume-versions],[data-resume-persona],[data-close-overlay],[data-delete-experience],[data-edit-experience],[data-save-experience],[data-discard-draft],[data-confirm-existing-draft],[data-discard-existing-draft],[data-edit-persona],[data-delete-persona],[data-fit-persona],[data-edit-provider],[data-test-provider],[data-delete-job],[data-reframe-job],[data-what-if],[data-skill-detail],[data-copy-resource],[data-open-resource],[data-edit-skill],[data-delete-skill],[data-complete-learning],[data-cancel-task],[data-retry-task],[data-experience-filter]";this.root.querySelectorAll<HTMLButtonElement>("button").forEach(button=>{if(button.matches(bound)||button.type==="submit")return;button.disabled=true;button.title="此操作需要先完成对应数据选择或等待后端能力接入";button.setAttribute("aria-describedby","unavailable-action-note")});if(this.root.querySelector("button:disabled")&&!this.root.querySelector("#unavailable-action-note")){const note=document.createElement("p");note.id="unavailable-action-note";note.className="sr-only";note.textContent="不可用操作会说明所需的前置条件";this.root.append(note)}}
 private openEditExperience(id:string){const value=this.snapshot()?.experiences.find(x=>x.id===id);if(!value)return;const typeMap={工作:"work",项目:"project",教育:"education",认证:"certification"}as const;this.experienceEntry={mode:"edit",editingId:value.id,editingVersion:value.version,phase:"edit",raw:value.original,proposal:{type:typeMap[value.kind]??"project",title:value.title,organization:value.organization||null,startDate:value.startDate||null,endDate:value.endDate||null,rawDescription:value.original,structuredAchievements:value.structuredAchievements??[],skillsDemonstrated:value.skillsDemonstrated??value.skills,industryTags:value.industryTags??[],educationLevel:value.educationLevel??"none",status:"draft"}};this.dialog=true;this.render()}
 private openImportedExperience(raw:string){this.navigation.navigate("experiences");this.experienceEntry={mode:"create",phase:"input",raw};this.dialog=true;this.render();void this.structureExperience(raw)}
 private async saveExperience(event:SubmitEvent){event.preventDefault();const v=new FormData(event.currentTarget as HTMLFormElement);if(this.experienceEntry.mode==="edit"&&this.experienceEntry.editingId){const list=(name:string)=>String(v.get(name)??"").split(/\r?\n/).map(x=>x.trim()).filter(Boolean);const type=String(v.get("kind")) as StructuredExperienceDraftDto["type"];const kindMap={work:"工作",project:"项目",education:"教育",certification:"认证"}as const;const current=this.snapshot()?.experiences.find(x=>x.id===this.experienceEntry.editingId);if(!current){this.fail("编辑经历",{message:"经历不存在或已刷新，请重试。"});return}try{await (this.source as WorkspaceDataSource&import("./api/data-source").ExperienceActions).updateExperience({...current,title:String(v.get("title")),organization:String(v.get("organization")),original:String(v.get("original")),startDate:String(v.get("startDate")),endDate:String(v.get("endDate")),period:[String(v.get("startDate")),String(v.get("endDate"))].filter(Boolean).join("—"),kind:kindMap[type]??current.kind,structuredAchievements:list("structuredAchievements"),skillsDemonstrated:list("skillsDemonstrated"),industryTags:list("industryTags"),educationLevel:String(v.get("educationLevel")??"none") as StructuredExperienceDraftDto["educationLevel"]});this.dialog=false;this.experienceEntry={mode:"create",phase:"input",raw:""};await this.load()}catch(error){this.experienceEntry={...this.experienceEntry,formError:(error as{message?:string}).message??"保存失败，请重试。"};this.fail("编辑经历",error);this.render()}return}const startDate=String(v.get("startDate"));const endDate=String(v.get("endDate"));const raw=this.experienceEntry.raw||String(v.get("original"));const list=(name:string)=>String(v.get(name)??"").split(/\r?\n/).map(x=>x.trim()).filter(Boolean);const structuredAchievements=list("structuredAchievements"),skillsDemonstrated=list("skillsDemonstrated"),industryTags=list("industryTags");const educationLevel=String(v.get("educationLevel")??"none") as StructuredExperienceDraftDto["educationLevel"];const type=String(v.get("kind")) as StructuredExperienceDraftDto["type"];const kindMap={work:"工作",project:"项目",education:"教育",certification:"认证"}as const;this.experienceEntry={...this.experienceEntry,mode:"create",phase:"committing",raw};this.render();try{this.pendingExperience=await this.source.addExperience({title:String(v.get("title")),organization:String(v.get("organization")),period:[startDate,endDate].filter(Boolean).join("—"),startDate,endDate,kind:kindMap[type]??String(v.get("kind")) as Experience["kind"],original:raw,structuredAchievements,skillsDemonstrated,industryTags,educationLevel,status:"draft"});this.dialog=false;this.overlay=renderExperienceConfirmation(this.pendingExperience);this.render()}catch(error){this.experienceEntry={...this.experienceEntry,mode:"create",phase:"error",error:(error as {message?:string}).message??"保存失败，请继续手填。"};this.fail("保存经历草稿",error);this.render()}}
 private normalizeStructureDraft(raw:string,value:Record<string,unknown>|undefined):StructuredExperienceDraftDto|null{
  if(!value||typeof value!=="object")return null;
  const typeRaw=String(value.type??value.experienceType??"");
  const types=new Set(["work","project","education","certification"]);
  if(!types.has(typeRaw))return null;
  const title=typeof value.title==="string"?value.title.trim():"";
  if(!title)return null;
  const nullable=(v:unknown)=>v===null||v===undefined?null:typeof v==="string"?v:null;
  const list=(v:unknown)=>Array.isArray(v)?v.map(x=>String(x).trim()).filter(Boolean):[];
  const education=typeof value.educationLevel==="string"?value.educationLevel:"none";
  const levels=new Set(["none","high_school","associate","bachelor","master","doctorate","other"]);
  return{
    type:typeRaw as StructuredExperienceDraftDto["type"],
    title,
    organization:nullable(value.organization),
    startDate:nullable(value.startDate),
    endDate:nullable(value.endDate),
    rawDescription:raw,
    structuredAchievements:list(value.structuredAchievements),
    skillsDemonstrated:list(value.skillsDemonstrated),
    industryTags:list(value.industryTags),
    educationLevel:(levels.has(education)?education:"none") as StructuredExperienceDraftDto["educationLevel"],
    status:"draft"
  };
}
 private async structureExperience(rawOverride?:string){const form=this.root.querySelector<HTMLFormElement>("[data-experience-form]");const rawField=form?.querySelector<HTMLTextAreaElement>('[name="original"]');const raw=rawOverride??rawField?.value??this.experienceEntry.raw;if(!raw.trim()){if(rawField&&!rawField.reportValidity())return;this.experienceEntry={...this.experienceEntry,mode:"create",phase:"error",raw,error:"请先填写具体经历，再使用 AI 整理。"};this.render();return}if(rawOverride===undefined&&rawField&&!rawField.reportValidity())return;try{const id=await this.source.startTask("structure_experience",{rawDescription:raw});this.experienceEntry={mode:"create",phase:"generating",raw,taskId:id};this.render();void this.pollExperienceStructure(id)}catch(error){this.experienceEntry={...this.experienceEntry,mode:"create",phase:"error",raw,error:(error as {message?:string}).message??"AI 整理暂不可用，请继续手填。"};this.render()}}
 private async pollExperienceStructure(id:string){if(!this.source.getTask||this.experienceEntry.taskId!==id)return;try{const task=await this.source.getTask(id);if(this.experienceEntry.taskId!==id)return;const state=String(task.state);if(state==="started"||state==="progress"){window.setTimeout(()=>void this.pollExperienceStructure(id),100);return}if(state==="cancelled"){this.experienceEntry={mode:"create",phase:"input",raw:this.experienceEntry.raw};this.render();return}if(state==="failed")throw task.error??new Error("AI 整理失败");if(state!=="completed")throw new Error("AI 整理返回了未知状态");const preview=(task.result??{}) as StructurePreviewDto&Record<string,unknown>;const draftSource=(preview.draft??preview) as Record<string,unknown>;const draft=this.normalizeStructureDraft(this.experienceEntry.raw,draftSource);if(!draft)throw new Error("AI 整理响应不符合 v3 契约，请继续手填。");this.experienceEntry={mode:"create",phase:"editablePreview",raw:this.experienceEntry.raw,proposal:draft};this.render()}catch(error){this.experienceEntry={mode:"create",phase:"error",raw:this.experienceEntry.raw,error:(error as {message?:string}).message??"AI 整理暂不可用，请继续手填。"};this.render()}}
 private async cancelExperienceStructure(){const id=this.experienceEntry.taskId;const raw=this.experienceEntry.raw;if(id)try{await this.source.cancelTask?.(id)}catch(error){this.fail("取消经历整理",error)}this.experienceEntry={mode:"create",phase:"input",raw};this.render()}
 private async confirmExperience(event:SubmitEvent){event.preventDefault();if(!this.pendingExperience)return;const v=new FormData(event.currentTarget as HTMLFormElement);try{await (this.source as WorkspaceDataSource&import("./api/data-source").ExperienceActions).updateExperience({...this.pendingExperience,title:String(v.get("title")),organization:String(v.get("organization")),original:String(v.get("original")),status:"confirmed"});this.pendingExperience=undefined;this.overlay="";await this.load()}catch(error){this.fail("确认经历",error)}}
 private async discardExperience(){if(!this.pendingExperience)return;try{await (this.source as WorkspaceDataSource&import("./api/data-source").ExperienceActions).updateExperience({...this.pendingExperience,status:"discarded"});this.pendingExperience=undefined;this.overlay="";await this.load()}catch(error){this.fail("丢弃经历草稿",error)}}
 private async setExperienceStatus(id:string,status:"confirmed"|"discarded"){const value=this.snapshot()?.experiences.find(x=>x.id===id);if(!value)return;try{await (this.source as WorkspaceDataSource&import("./api/data-source").ExperienceActions).updateExperience({...value,status});await this.load()}catch(error){this.fail(status==="confirmed"?"确认经历":"丢弃经历",error)}}
 private async deleteExperience(id:string,version:number){try{await this.source.deleteExperience(id,version);await this.load()}catch{this.tasks.upsert({id:crypto.randomUUID(),label:"删除经历",phase:"failed",message:"删除失败，记录可能已被修改，请刷新后重试。",retryable:true})}}
 private snapshot(){return this.state.status==="ready"?this.state.data:undefined}
 private async savePersona(event:SubmitEvent){event.preventDefault();const form=event.currentTarget as HTMLFormElement;const v=new FormData(form);const input={name:String(v.get("name")),targetRole:String(v.get("targetRole")),positioning:String(v.get("positioning"))};const existingId=form.dataset.id||"";try{const personaId=existingId?(await this.source.updatePersona(existingId,input),existingId):await this.source.createPersona(input);this.overlay="";await this.load();await this.recommendAndOpenFit(personaId)}catch(error){this.fail("保存角色",error)}}
 private async removePersona(id:string){try{await this.source.deletePersona(id);await this.load()}catch(error){this.fail("删除角色",error)}}
 private async recommendAndOpenFit(personaId:string){
  const confirmed=(this.snapshot()?.experiences??[]).filter(x=>x.status!=="draft");
  if(!confirmed.length){this.tasks.upsert({id:crypto.randomUUID(),label:"角色权重",phase:"completed",message:"角色已保存。暂无已确认经历，可稍后在「调整经历权重」中设置。",progress:100});return}
  const taskId=crypto.randomUUID();
  this.tasks.upsert({id:taskId,label:"推荐经历权重",phase:"started",message:"正在根据定位陈述分析经历库…",progress:10});
  try{
   if(!this.source.startTask||!this.source.getTask)throw{code:"UNAVAILABLE",message:"需要后台任务支持"};
   const id=await this.source.startTask("recommend_persona_weights",{personaId});
   for(;;){
    const task=await this.source.getTask(id);
    const state=String(task.state??"");
    if(state==="completed"){
     const result=(task.result??{}) as Record<string,unknown>;
     const scores=Array.isArray(result.scores)?result.scores as Record<string,unknown>[]:[];
     const byId=new Map(scores.map(x=>[String(x.experienceId??""),Number(x.relevanceScore??x.score??50)]));
     const existing=await this.source.getFitScores(personaId).catch(()=>[] as {experienceId:string;score:number;overridden:boolean}[]);
     const overridden=new Map(existing.filter(x=>x.overridden).map(x=>[x.experienceId,x.score]));
     this.overlay=renderFitDialog(personaId,confirmed.map(x=>({id:x.id,title:x.title,score:byId.get(x.id)??overridden.get(x.id)??50,overridden:overridden.has(x.id)})),"已根据定位陈述生成推荐权重，请确认或调整后保存。");
     this.tasks.upsert({id:taskId,label:"推荐经历权重",phase:"completed",message:"推荐已生成，请确认权重。",progress:100});
     this.render();
     return;
    }
    if(state==="failed")throw task.error??{code:"UNAVAILABLE",message:"推荐失败"};
    if(state==="cancelled"){this.tasks.upsert({id:taskId,label:"推荐经历权重",phase:"cancelled",message:"已取消推荐"});return}
    this.tasks.upsert({id:taskId,label:"推荐经历权重",phase:"progress",message:"正在根据定位陈述分析经历库…",progress:typeof task.progress==="number"?task.progress:40});
    await new Promise(r=>setTimeout(r,120));
   }
  }catch(error){
   this.tasks.upsert({id:taskId,label:"推荐经历权重",phase:"failed",message:(error as{message?:string}).message??"AI 推荐暂不可用，已打开当前权重供手动调整。",retryable:true});
   await this.openFit(personaId);
  }
 }
 private async openFit(personaId:string){try{const scores=await this.source.getFitScores(personaId);const byId=new Map(scores.map(x=>[x.experienceId,x]));this.overlay=renderFitDialog(personaId,(this.snapshot()?.experiences??[]).filter(x=>x.status!=="draft").map(x=>({...x,...byId.get(x.id)})),"权重越高，该经历越容易进入此角色的简历。可手动调整后保存。");this.render()}catch(error){this.fail("加载评分",error)}}
 private async saveFit(event:SubmitEvent){event.preventDefault();const form=event.currentTarget as HTMLFormElement;const personaId=form.dataset.personaId??"";const values=new FormData(form);try{await Promise.all([...values.entries()].map(([experienceId,score])=>this.source.setFitScore(personaId,experienceId,Number(score))));this.overlay="";await this.load();this.tasks.upsert({id:crypto.randomUUID(),label:"保存权重",phase:"completed",message:"经历权重已保存。",progress:100})}catch(error){this.fail("保存评分",error)}}
 private providerMessage(message:string){
  const lower=message.toLowerCase();
  if(lower.includes("must use https"))return "API 地址必须使用 HTTPS；仅当服务商名称为 local 时可用本机 HTTP";
  if(lower.includes("dns resolution failed"))return "无法解析 API 地址，请检查网络与域名";
  if(lower.includes("private, link-local")||lower.includes("metadata address"))return "不允许使用私网、链路本地或元数据地址";
  if(lower.includes("endpoint is invalid")||lower.includes("not allowed"))return "API 地址格式无效或不被允许";
  if(lower.includes("apikey must be entered")||lower.includes("credentialtarget changed"))return "修改 API 地址后需要重新填写 API Key";
  if(lower.includes("provider and model are required"))return "名称与默认模型不能为空";
  return message||"保存服务商失败，请检查配置后重试。";
}
private async saveProvider(event:SubmitEvent){event.preventDefault();const form=event.currentTarget as HTMLFormElement;const editing=form.dataset.editing==="1";const v=new FormData(form);const name=String(v.get("name")??"").trim();const baseUrl=String(v.get("baseUrl")??"").trim();const model=String(v.get("model")??"").trim();const apiKey=String(v.get("apiKey")??"");if(!editing&&!apiKey.trim()){this.providerError="新建服务商时 API Key 为必填项。";this.overlay=renderProviderDialog(undefined,this.providerError);this.render();return}try{await this.source.saveProvider({id:name,name,baseUrl,model,apiKey:apiKey||undefined,enabled:v.get("enabled")==="on",hasKey:Boolean(apiKey)});this.providerError="";this.overlay="";await this.load()}catch(error){const message=this.providerMessage((error as{message?:string}).message??"");this.providerError=message;this.overlay=renderProviderDialog(editing?{id:name,name,model,baseUrl,enabled:v.get("enabled")==="on"}:undefined,message);this.fail("保存服务商",{message});this.render()}}
 private testProviderMessage(message:string){
  const lower=message.toLowerCase();
  if(lower.includes("not found: credential"))return "未找到已保存的 API Key。请编辑服务商并重新填写密钥后保存。";
  if(lower.includes("not found: provider"))return "服务商不存在或未启用。请先勾选「启用」并保存。";
  if(lower.includes("no enabled provider"))return "没有已启用的服务商，请先添加并启用。";
  if(lower.includes("authentication failed"))return "鉴权失败：API Key 或权限不正确。";
  if(lower.includes("windows credential manager")||lower.includes("failed to store credential")||lower.includes("failed to read credential"))return "无法访问 Windows 凭据管理器，请检查系统权限后重试。";
  if(lower.includes("connection failed")||lower.includes("timed out"))return "无法连接服务商，请检查网络与 API 地址。";
  if(lower.includes("dns resolution failed"))return "无法解析 API 地址，请检查网络与域名。";
  if(lower.includes("must use https"))return "API 地址必须使用 HTTPS；仅当服务商名称为 local 时可用本机 HTTP。";
  if(lower.includes("private, link-local")||lower.includes("metadata address"))return "不允许使用私网、链路本地或元数据地址。";
  if(lower.includes("rejected request")||lower.includes("invalid json"))return "服务商拒绝了请求，请核对模型名称与 API 地址。";
  if(lower.includes("returned no text")||lower.includes("stream truncated"))return "服务商已连通，但返回内容异常。可重试或更换模型。";
  return this.providerMessage(message);
}
 private async testProvider(name:string){try{await this.source.testProvider(name);this.tasks.upsert({id:crypto.randomUUID(),label:"测试连接",phase:"completed",message:"连接成功。",progress:100})}catch(error){const message=this.testProviderMessage((error as{message?:string}).message??"");this.fail("测试连接",{message})}}
 private jobActions(){return this.source as WorkspaceDataSource&JobActions}
 private async saveJob(event:SubmitEvent){event.preventDefault();const v=new FormData(event.currentTarget as HTMLFormElement);try{await this.jobActions().analyzeJob(String(v.get("jdText")),String(v.get("personaId")));this.overlay="";await this.load()}catch(error){this.fail("分析岗位",error)}}
 private async updateJobStatus(id:string,status:string){try{await this.jobActions().updateJobStatus(id,status);await this.load()}catch(error){this.fail("更新投递状态",error);if((error as{code?:string}).code==="CONFLICT")await this.load()}}
 private async deleteJob(id:string){try{await this.jobActions().deleteJob(id);await this.load()}catch(error){this.fail("删除岗位",error)}}
 private fail(label:string,error:unknown){this.tasks.upsert({id:crypto.randomUUID(),label,phase:"failed",message:(error as {message?:string}).message??"操作失败，请重试。",retryable:true})}
 private async pollTask(id:string,label:string){if(!this.source.getTask)return;try{if(Date.now()>=(this.taskDeadlines.get(id)??0)){await this.cancelTask(id);this.taskDeadlines.delete(id);return}const task=await this.source.getTask(id);const phase=String(task.state)as "started"|"progress"|"completed"|"failed"|"cancelled";const error=task.error as{message?:string}|undefined;this.tasks.upsert({id,label,phase,message:phase==="completed"?"处理完成。":phase==="failed"?(error?.message??"处理失败。"):phase==="cancelled"?"操作已取消。":"正在处理，你可以继续使用其他页面。",progress:typeof task.progress==="number"?task.progress:undefined,retryable:phase==="failed"});if(phase==="started"||phase==="progress")window.setTimeout(()=>void this.pollTask(id,label),200);else this.taskDeadlines.delete(id)}catch(error){this.taskDeadlines.delete(id);this.fail(label,error)}}
 private async cancelTask(id:string){if(!this.source.cancelTask)return;try{await this.source.cancelTask(id);const current=this.tasks.list().find(x=>x.id===id);this.tasks.upsert({id,label:current?.label??"后台任务",phase:"cancelled",message:"操作已取消。"})}catch(error){this.fail("取消任务",error)}}
 private async demoTask(action:string,label:string){const map:Record<string,"generate_resume"|"parse_job"|"generate_learning_path"|"test_provider">={"generate-resume":"generate_resume","analyze-job":"parse_job","generate-learning":"generate_learning_path","test-provider":"test_provider","import-experience":"parse_job"};try{const id=await this.source.startTask(map[action]??"parse_job");if(!id)return;this.taskDeadlines.set(id,Date.now()+120_000);this.tasks.upsert({id,label,phase:"started",message:"任务已提交。",progress:0});void this.pollTask(id,label)}catch(error){this.fail(label,error)}}
}
