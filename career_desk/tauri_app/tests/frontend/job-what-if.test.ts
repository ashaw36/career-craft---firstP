import { describe, expect, it, vi } from "vitest";
import { renderPage } from "../../src/features/pages";
import { bind } from "../../src/features/actions/jobs";
import { TaskStore } from "../../src/shared/state/tasks";
import type { WorkspaceSnapshot } from "../../src/api/data-source";

describe("job-bound what-if", () => {
  it("derives immutable context from the match and hands confirmed skills to learning", async () => {
    vi.useFakeTimers();
    const snapshot: WorkspaceSnapshot = {experiences:[],personas:[{id:"p1",name:"产品角色",targetRole:"PM",positioning:"",fit:0}],jobs:[{id:"j1",jobDescId:"j1",matchId:"m1",personaId:"p1",title:"产品经理",company:"",status:"new",score:65,matched:["SQL"],missing:["实验设计"],scoreBreakdown:{skills:25,experience:20,industry:10,education:10}}],resumes:[],skills:[],learning:[],providers:[]};
    const simulateJobWhatIf=vi.fn().mockResolvedValue({baselineScore:65,simulatedScore:90,delta:25,baselineBreakdown:{skills:25},simulatedBreakdown:{skills:50},addedSkills:["实验设计"],remainingMissing:[]});
    const source={load:vi.fn().mockResolvedValue(snapshot),simulateJobWhatIf};
    const root=document.createElement("div"); root.innerHTML=renderPage("jobs",snapshot);
    const navigate=vi.fn(); bind(root,source as never,new TaskStore(),vi.fn(),navigate);
    root.querySelector<HTMLButtonElement>("[data-job-what-if]")!.click(); await Promise.resolve();
    const form=root.querySelector<HTMLFormElement>("[data-what-if-form]")!;
    expect(form.querySelector<HTMLInputElement>('[name="required"]')!.readOnly).toBe(true);
    expect(form.querySelector<HTMLInputElement>('[name="current"]')!.readOnly).toBe(true);
    await vi.runAllTimersAsync();
    expect(simulateJobWhatIf).toHaveBeenCalledWith("p1","m1",["实验设计"]);
    expect(root.querySelector("[data-what-if-result]")?.textContent).toContain("技能分项 25 → 50");
    root.querySelector<HTMLButtonElement>("[data-confirm-what-if-learning]")!.click(); await Promise.resolve();
    expect(JSON.parse(localStorage.getItem("careercraft:learning-context")!)).toMatchObject({skill:"实验设计",personaId:"p1",jobMatchId:"m1",origin:"what_if"});
    expect(navigate).toHaveBeenCalledWith("learning");
    vi.useRealTimers();
  });
});
