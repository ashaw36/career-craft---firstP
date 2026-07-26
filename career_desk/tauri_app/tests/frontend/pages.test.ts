import {describe,expect,it} from "vitest";
import {FixtureDataSource} from "../../src/api/data-source";
import {renderError,renderLoading,renderPage} from "../../src/features/pages";
import {routes} from "../../src/shared/state/navigation";
describe("operable product pages",()=>{
 it("renders every frozen page and key journeys",async()=>{const data=await new FixtureDataSource().load();for(const route of routes){const html=renderPage(route.id,data);expect(html).toContain('id="page-title"');expect(html).toContain("button")}expect(renderPage("jobs",data)).toContain("假设分析");expect(renderPage("resumes",data)).toContain("比较版本");expect(renderPage("learning",data)).toContain("完成并转为经历")});
 it("renders loading and recoverable error states",()=>{expect(renderLoading()).toContain('aria-busy="true"');expect(renderError("断网")).toContain("重试")});
 it("renders industry, education rank and honest evidence provenance safely",async()=>{const data=await new FixtureDataSource().load();data.jobs=[{id:"j",title:"Role",company:"Co",status:"new",score:88,matched:[],missing:[],industryTags:["金融","<img src=x onerror=1>"],educationLevels:["master","bachelor"],scoreBreakdown:{skills:30,experience:20,industry:18,education:20},evidenceSources:{jobIndustry:"persisted",jobEducation:"legacy_heuristic",candidateIndustry:"persisted",candidateEducation:"legacy_heuristic",candidateSkills:"persisted",candidateExperience:"persisted"}}];const html=renderPage("jobs",data);expect(html).toContain("行业匹配");expect(html).toContain("学历匹配");expect(html).toContain("硕士、本科");expect(html).toContain("旧数据启发式推断（非结构化事实）");const root=document.createElement("div");root.innerHTML=html;expect(root.querySelector("img")).toBeNull()});
});
