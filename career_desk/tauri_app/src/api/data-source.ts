import { api, type CareerCraftClient } from "./client";
import type { AppError, Json, ResumePreviewDto, ResumeTemplateId } from "./contracts";
export interface Job {
    personaId?: string;
}
export interface SkillLearningActions {
    simulateJobWhatIf(personaId: string, jobMatchId: string, hypotheticalSkills: string[]): Promise<Json>;
}
export interface Job {
    industryTags?: string[];
    educationLevels?: import("./contracts").EducationLevel[];
    scoreBreakdown?: {
        skills: number;
        experience: number;
        industry: number;
        education: number;
    };
    evidenceSources?: Record<string, string>;
}
type Obj = Record<string, unknown>;
const obj = (v: Json): Obj => v as unknown as Obj;
const text = (v: unknown, f = "") => typeof v === "string" ? v : f;
const num = (v: unknown, f = 0) => typeof v === "number" ? v : f;
export type ResumeInstructionType = import("./contracts").ResumeInstructionType;
export type LearningExperienceDraft = import("./contracts").LearningExperienceDraftDto & {
    conversionId: string;
    sourceSnapshot: import("./contracts").LearningSourceSnapshotDto;
};
export interface Job {
    statusEvents?: import("./contracts").JobStatusEventDto[];
}
export interface SkillResource {
    resourceId?: string;
    skillId?: string;
}
export interface LearningItem {
    version?: number;
}
export interface Experience {
    id: string;
    version: number;
    title: string;
    organization: string;
    period: string;
    startDate?: string;
    endDate?: string;
    kind: "工作" | "项目" | "教育" | "认证";
    original: string;
    skills: string[];
    structuredAchievements?: string[];
    skillsDemonstrated?: string[];
    industryTags?: string[];
    educationLevel?: import("./contracts").EducationLevel;
    status?: "draft" | "confirmed" | "discarded" | "archived";
    overlapExperienceIds?: string[];
    warnings?: {
        code: string;
        experienceIds: string[];
    }[];
}
export interface Persona {
    id: string;
    name: string;
    targetRole: string;
    positioning: string;
    fit: number;
}
export interface Job {
    id: string;
    title: string;
    company: string;
    status: string;
    score: number;
    matched: string[];
    missing: string[];
    updatedAt?: string;
    matchId?: string;
    jobDescId?: string;
    rawText?: string;
}
export interface Resume {
    id: string;
    title: string;
    persona: string;
    template: string;
    version: number;
    updatedAt: string;
    preview?: string;
}
export interface SkillResource {
    kind: string;
    title: string;
    source: string;
    url: string;
    estimatedHours: number;
}
export interface Skill {
    id: string;
    name: string;
    level: string;
    evidence: number;
    custom?: boolean;
    category?: string;
    description?: string;
    aliases?: string[];
    prerequisites?: string[];
    parentId?: string;
    resources?: SkillResource[];
}
export interface LearningItem {
    id: string;
    title: string;
    skill: string;
    source: string;
    effort: string;
    status: string;
    resourceUrl?: string;
    pathId?: string;
    personaId?: string;
    pathStatus?: string;
    pathVersion?: number;
    sequence?: number;
    objective?: string;
    practiceTask?: string;
    completionCriteria?: string;
    convertedExperienceId?: string;
    origin?: string;
    jobMatchId?: string;
    jobTitle?: string;
    personaName?: string;
    generationMode?: string;
    reason?: string;
    guidance?: string;
}
export interface Provider {
    id: string;
    name: string;
    model: string;
    baseUrl?: string;
    enabled: boolean;
    hasKey: boolean;
}
export interface FitRow {
    experienceId: string;
    score: number;
    overridden: boolean;
}
export interface WorkspaceSnapshot {
    experiences: Experience[];
    personas: Persona[];
    jobs: Job[];
    resumes: Resume[];
    skills: Skill[];
    learning: LearningItem[];
    providers: Provider[];
}
export type ViewState = {
    status: "loading";
} | {
    status: "error";
    error: AppError;
} | {
    status: "ready";
    data: WorkspaceSnapshot;
};
export type NewExperience = Omit<Experience, "id" | "version" | "skills" | "status"> & {
    status?: "draft";
};
export interface WorkspaceDataSource {
    load(): Promise<WorkspaceSnapshot>;
    addExperience(input: NewExperience): Promise<Experience>;
    deleteExperience(id: string, version: number): Promise<void>;
    createPersona(input: {
        name: string;
        targetRole: string;
        positioning: string;
    }): Promise<string>;
    updatePersona(id: string, input: {
        name: string;
        targetRole: string;
        positioning: string;
    }): Promise<void>;
    deletePersona(id: string): Promise<void>;
    getFitScores(personaId: string): Promise<FitRow[]>;
    setFitScore(personaId: string, experienceId: string, score: number): Promise<void>;
    saveProvider(provider: Provider & {
        apiKey?: string;
    }): Promise<void>;
    testProvider(name: string): Promise<void>;
    startTask(operation: "generate_resume" | "parse_job" | "generate_learning_path" | "test_provider" | "structure_experience" | "chat_refine_resume" | "recommend_persona_weights", payload?: Obj): Promise<string>;
    getTask?(taskId: string): Promise<Obj>;
    cancelTask?(taskId: string): Promise<Obj>;
}
const fixture: WorkspaceSnapshot = { experiences: [{ id: "exp-1", version: 1, title: "供应链数据分析", organization: "示例科技", period: "2024.03—至今", kind: "工作", original: "搭建供应风险看板。", skills: ["数据分析"] }], personas: [{ id: "per-1", name: "数据产品方向", targetRole: "数据产品经理", positioning: "用数据推动决策", fit: 82 }], jobs: [{ id: "job-1", jobDescId: "job-1", matchId: "match-1", title: "高级数据产品经理", company: "示例公司", status: "new", score: 78, matched: ["数据分析"], missing: ["A/B 测试"], rawText: "高级数据产品经理\n要求数据分析与 A/B 测试。" }], resumes: [], skills: [{ id: "sk-1", name: "数据分析", level: "熟练", evidence: 3 }], learning: [], providers: [] };
export class FixtureDataSource implements WorkspaceDataSource, JobActions {
    private data = structuredClone(fixture);
    async load() { return structuredClone(this.data); }
    async addExperience(input: Omit<Experience, "id" | "version" | "skills">) { const item = { ...input, id: crypto.randomUUID(), version: 1, skills: [] }; this.data.experiences.unshift(item); return structuredClone(item); }
    async deleteExperience(id: string, _version: number) { this.data.experiences = this.data.experiences.filter(x => x.id !== id); }
    async createPersona(input: {
        name: string;
        targetRole: string;
        positioning: string;
    }) { const id = crypto.randomUUID(); this.data.personas.push({ ...input, id, fit: 0 }); return id; }
    async updatePersona(id: string, input: {
        name: string;
        targetRole: string;
        positioning: string;
    }) { this.data.personas = this.data.personas.map(x => x.id === id ? { ...x, ...input } : x); }
    async deletePersona(id: string) { this.data.personas = this.data.personas.filter(x => x.id !== id); }
    async getFitScores(_personaId: string) { return this.data.experiences.map(x => ({ experienceId: x.id, score: 50, overridden: false })); }
    async setFitScore(_personaId: string, _experienceId: string, _score: number) { }
    async saveProvider(provider: Provider & {
        apiKey?: string;
    }) { const { apiKey: _, ...value } = provider; this.data.providers = this.data.providers.filter(x => x.id !== provider.id); this.data.providers.push(value); }
    async testProvider(_name: string) { }
    async analyzeJob(jdText: string, _personaId: string) { this.data.jobs.push({ id: crypto.randomUUID(), title: jdText.split("\n")[0] || "岗位", company: "", status: "new", score: 0, matched: [], missing: [] }); }
    async deleteJob(id: string) { this.data.jobs = this.data.jobs.filter(x => x.id !== id); }
    async updateJobStatus(id: string, status: string) { this.data.jobs = this.data.jobs.map(x => x.id === id || x.matchId === id ? { ...x, status } : x); }
    async reframeJob(_jobId: string) { return { reframes: [] }; }
    async startTask() { return crypto.randomUUID(); }
}
const unwrap = <T extends Json | Json[]>(r: {
    success: true;
    data: T;
} | {
    success: false;
    error: AppError;
}): T => { if (!r.success)
    throw r.error; return r.data; };
export class ClientDataSource implements WorkspaceDataSource, ReframeActions, V2Actions, SkillLearningActions, ResumeActions, ExperienceActions, UrlActions {
    private readonly jobVersions = new Map<string, number>();
    constructor(private readonly client: CareerCraftClient = api) { }
    async load(): Promise<WorkspaceSnapshot> {
        const [er, pr, jr, sr, lr, st] = await Promise.all([this.client.getExperiences(), this.client.getPersonas(), this.client.listJobs(), this.client.getSkillGraph(), this.client.getLearningPaths(), this.client.getSettings()]);
        const rawExperiences = unwrap(er).map(obj), pe = unwrap(pr).map(obj), jobs = unwrap(jr).map(obj), skills = unwrap(sr).map(obj), learning = unwrap(lr).map(obj), settings = obj(unwrap(st));
        const experiences = rawExperiences.map(v => { const startDate = text(v.startDate), endDate = text(v.endDate); const skillsDemonstrated = Array.isArray(v.skillsDemonstrated) ? v.skillsDemonstrated.map(String) : []; return { id: text(v.id), version: num(v.version, 1), title: text(v.title), organization: text(v.organization), startDate, endDate, period: [startDate, endDate].filter(Boolean).join("—"), kind: ({ work: "工作", project: "项目", education: "教育", certification: "认证" } as Record<string, Experience["kind"]>)[text(v.type)] ?? "项目", original: text(v.rawDescription), skills: skillsDemonstrated, structuredAchievements: Array.isArray(v.structuredAchievements) ? v.structuredAchievements.map(String) : [], skillsDemonstrated, industryTags: Array.isArray(v.industryTags) ? v.industryTags.map(String) : [], educationLevel: text(v.educationLevel, "none") as import("./contracts").EducationLevel, status: text(v.status, "draft") as Experience["status"], overlapExperienceIds: Array.isArray(v.overlapExperienceIds) ? v.overlapExperienceIds.map(String) : [], warnings: Array.isArray(v.warnings) ? v.warnings.map(obj).map(w => ({ code: text(w.code), experienceIds: Array.isArray(w.experienceIds) ? w.experienceIds.map(String) : [] })) : [] }; }).filter(x => x.status !== "discarded" && x.status !== "archived");
        const personas = pe.map(v => ({ id: text(v.id), name: text(v.name), targetRole: text(v.targetRole, "未设置目标"), positioning: text(v.identityStatement), fit: num(v.fit) }));
        const selected = localStorage.getItem("careercraft:selected-persona");
        const personaId = personas.some(p => p.id === selected) ? selected ?? "" : personas[0]?.id ?? "";
        const matchDetails: Obj[] = typeof this.client.getJobMatches === "function" ? await Promise.all(jobs.map(async (job) => { const result = await this.client.getJobMatches(text(job.id)); if (!result.success)
            return {}; const rows = result.data.map(obj); if (personaId)
            return rows.find(row => text(row.personaId) === personaId) ?? {}; return rows.sort((a, b) => text(b.updatedAt).localeCompare(text(a.updatedAt)) || text(b.id).localeCompare(text(a.id)))[0] ?? {}; })) : jobs.map(() => (Object.create(null) as Obj));
        this.jobVersions.clear();
        matchDetails.forEach(match => { const id = text(match.id); if (id)
            this.jobVersions.set(id, num(match.version, 1)); });
        const timelines = typeof this.client.getJobStatusEvents === "function" ? await Promise.all(matchDetails.map(async (match) => { const id = text(match.id); if (!id)
            return []; const result = await this.client.getJobStatusEvents(id); return result.success ? result.data : []; })) : matchDetails.map(() => []);
        const versionResult = personaId && typeof this.client.listResumeVersions === "function" ? obj(unwrap(await this.client.listResumeVersions(personaId))) : {};
        const versionRows = Array.isArray(versionResult.items) ? versionResult.items as Obj[] : [];
        const resumes = versionRows.map(v => ({ id: text(v.versionId, text(v.id)), title: `${text(v.template, "resume")} 简历`, persona: text(v.personaId, personaId), template: text(v.template), version: num(v.revision, 1), updatedAt: text(v.createdAt, "本地"), preview: text(v.markdown) }));
        const providersRaw = Array.isArray(settings.providers) ? settings.providers as Obj[] : [];
        skills.forEach(skill => { skill.id = skill.skillId; skill.prerequisites = skill.prerequisiteSkillIds; });
        learning.forEach(path => { path.id = path.pathId; if (Array.isArray(path.items))
            (path.items as Obj[]).forEach(item => { item.id = item.itemId; item.resourceSource = item.source; item.source = path.sourceType; }); });
        const learningItems = learning.flatMap(v => {
            const context = obj(v.context as Json);
            return Array.isArray(v.items) ? (v.items as Obj[]).map(i => ({
                id: text(i.id), title: text(i.title), skill: text(i.skillId, text(v.targetGap)), source: text(i.source, text(v.sourceType)), effort: `约 ${num(i.estimatedHours)} 小时`, status: text(i.status), resourceUrl: text(i.resourceUrl), pathId: text(v.id), personaId: text(v.personaId), pathStatus: text(v.status), pathVersion: num(v.version), sequence: num(i.sequence, 1), objective: text(i.objective), practiceTask: text(i.practiceTask), completionCriteria: text(i.completionCriteria), convertedExperienceId: text(i.convertedExperienceId), origin: text(context.origin, text(v.sourceType)), jobMatchId: text(context.jobMatchId), jobTitle: text(context.jobTitle), personaName: text(context.personaName), generationMode: text(context.generationMode, "rules_only"), reason: text(context.reason), guidance: text(context.guidance, text(v.guidance)),
            })) : [];
        });
        return { experiences, personas, jobs: jobs.map((job, index) => { const match = matchDetails[index] ?? {}; const jobDescId = text(job.id); const matchId = text(match.id); return { id: jobDescId, jobDescId, matchId: matchId || undefined, title: text(job.title, text(match.title, "未命名岗位")), company: text(job.company), rawText: text(job.rawText), status: text(match.trackingStatus, text(job.status, "new")), score: num(match.matchScore, num(job.score)), matched: Array.isArray(match.matchedSkills) ? match.matchedSkills.map(String) : [], missing: Array.isArray(match.missingSkills) ? match.missingSkills.map(String) : [], industryTags: Array.isArray(job.industryTags) ? job.industryTags.map(String) : [], educationLevels: Array.isArray(job.educationLevels) ? job.educationLevels.map(String) as import("./contracts").EducationLevel[] : [], scoreBreakdown: typeof match.scoreBreakdown === "object" && match.scoreBreakdown ? { skills: num(obj(match.scoreBreakdown as Json).skills), experience: num(obj(match.scoreBreakdown as Json).experience), industry: num(obj(match.scoreBreakdown as Json).industry), education: num(obj(match.scoreBreakdown as Json).education) } : undefined, updatedAt: text(match.updatedAt), evidenceSources: typeof match.evidenceSources === "object" && match.evidenceSources ? Object.fromEntries(Object.entries(obj(match.evidenceSources as Json)).map(([k, value]) => [k, String(value)])) : undefined, statusEvents: timelines[index] }; }), resumes, skills: skills.map(v => ({ id: text(v.id), name: text(v.name), level: text(v.level, "待评估"), evidence: num(v.evidence), custom: v.custom === true, category: text(v.category), description: text(v.description), aliases: Array.isArray(v.aliases) ? v.aliases.map(String) : [], prerequisites: Array.isArray(v.prerequisites) ? v.prerequisites.map(String) : [], parentId: text(v.parentId), resources: Array.isArray(v.resources) ? v.resources.map(obj).map(r => ({ kind: text(r.kind), title: text(r.title), source: text(r.source), url: text(r.url), estimatedHours: num(r.estimatedHours) })) : [] })), learning: learningItems, providers: providersRaw.map(v => ({ id: text(v.name), name: text(v.name), model: text(v.defaultModel), baseUrl: text(v.baseUrl), enabled: v.enabled !== false, hasKey: v.hasKey === true })) };
    }
    async addExperience(input: Omit<Experience, "id" | "version" | "skills">) { const result = unwrap(await this.client.saveExperience({ newId: crypto.randomUUID(), type: ({ "工作": "work", "项目": "project", "教育": "education", "认证": "certification" } as const)[input.kind], title: input.title, organization: input.organization, startDate: input.startDate || null, endDate: input.endDate || null, rawDescription: input.original, structuredAchievements: input.structuredAchievements ?? [], skillsDemonstrated: input.skillsDemonstrated ?? [], industryTags: input.industryTags ?? [], educationLevel: input.educationLevel ?? "none", status: "draft" })); const v = obj(result); return { ...input, period: [input.startDate, input.endDate].filter(Boolean).join("—"), status: "draft" as const, id: text(v.id), version: num(v.version, 1), skills: input.skillsDemonstrated ?? [], overlapExperienceIds: Array.isArray(v.overlapExperienceIds) ? v.overlapExperienceIds.map(String) : [], warnings: Array.isArray(v.warnings) ? v.warnings.map(obj).map(w => ({ code: text(w.code), experienceIds: Array.isArray(w.experienceIds) ? w.experienceIds.map(String) : [] })) : [] }; }
    async deleteExperience(id: string, version: number) { unwrap(await this.client.deleteExperience(id, version)); }
    async importFile(file: File, options: {
        commit?: boolean;
    } = {}) { const bytes = new Uint8Array(await file.arrayBuffer()); let binary = ""; for (const byte of bytes)
        binary += String.fromCharCode(byte); const result = obj(await this.background("import_file", { fileName: file.name, base64Content: btoa(binary), commit: options.commit !== false })); return { count: num(result.count), content: text(result.content) }; }
    async updateExperience(value: Experience) { const types = { "工作": "work", "项目": "project", "教育": "education", "认证": "certification" } as const; unwrap(await this.client.saveExperience({ id: value.id, version: value.version, type: types[value.kind], title: value.title, organization: value.organization, startDate: value.startDate || null, endDate: value.endDate || null, rawDescription: value.original, structuredAchievements: value.structuredAchievements ?? [], skillsDemonstrated: value.skillsDemonstrated ?? value.skills, industryTags: value.industryTags ?? [], educationLevel: value.educationLevel ?? "none", status: value.status })); }
    async createPersona(input: {
        name: string;
        targetRole: string;
        positioning: string;
    }) { const id = crypto.randomUUID(); unwrap(await this.client.createPersona({ id, name: input.name, identityStatement: input.positioning, targetJobProfiles: [input.targetRole], maxExperiences: 5 })); return id; }
    async updatePersona(id: string, input: {
        name: string;
        targetRole: string;
        positioning: string;
    }) { unwrap(await this.client.updatePersona(id, { name: input.name, identityStatement: input.positioning, targetJobProfiles: [input.targetRole] })); }
    async deletePersona(id: string) { unwrap(await this.client.deletePersona(id)); }
    async getFitScores(personaId: string) { return (unwrap(await this.client.getFitScores(personaId)) as Json[]).map(v => { const x = obj(v); return { experienceId: text(x.experienceId), score: num(x.relevanceScore), overridden: x.userOverridden === true }; }); }
    async setFitScore(personaId: string, experienceId: string, score: number) { unwrap(await this.client.updateFitScore(personaId, experienceId, score)); }
    async resetFitScore(personaId: string, experienceId: string) { unwrap(await this.client.resetFitScore(personaId, experienceId)); }
    async saveProvider(provider: Provider & {
        apiKey?: string;
    }) { const current = this.client.getSettings(); const value = obj(unwrap(await current)); const providers = Array.isArray(value.providers) ? value.providers as Obj[] : []; const next = providers.filter(x => text(x.name) !== provider.name); next.push({ name: provider.name, baseUrl: provider.baseUrl, defaultModel: provider.model, enabled: provider.enabled, ...(provider.apiKey ? { apiKey: provider.apiKey } : {}) }); unwrap(await this.client.saveSettings({ providers: next })); }
    async testProvider(name: string) { unwrap(await this.client.testLLMConnection(name)); }
    async deleteProvider(name: string) { unwrap(await this.client.deleteProvider(name)); }
    async exportPortableBackup(destinationPath: string) { return unwrap(await this.client.exportPortableBackup(destinationPath, true)); }
    async inspectPortableBackup(archivePath: string) { return unwrap(await this.client.inspectPortableBackup(archivePath)); }
    async importPortableBackup(archivePath: string) { return unwrap(await this.client.importPortableBackup(archivePath, true)); }
    private async background(operation: string, payload: Obj = {}) { const started = obj(unwrap(await this.client.startBackgroundTask(operation, payload))); const taskId = text(started.taskId); if (!taskId)
        throw { code: "INTERNAL", message: "后端未返回任务编号" }; const deadline = Date.now() + 600000; for (;;) {
        if (Date.now() >= deadline) {
            await this.client.cancelBackgroundTask(taskId);
            throw { code: "UNAVAILABLE", message: "后台任务超时并已取消" };
        }
        await new Promise(resolve => setTimeout(resolve, 100));
        const task = obj(unwrap(await this.client.getBackgroundTask(taskId)));
        const state = text(task.state);
        if (state === "completed")
            return (task.result ?? null) as Json;
        if (state === "failed")
            throw obj(task.error as Json);
        if (state === "cancelled")
            throw { code: "CANCELLED", message: "操作已取消" };
    } }
    async generateResume(personaId: string, template: ResumeTemplateId) { localStorage.setItem("careercraft:selected-persona", personaId); return this.background("generate_resume", { personaId, template }); }
    async previewResume(personaId: string, template: ResumeTemplateId) { return unwrap(await this.client.previewResume(personaId, template)) as ResumePreviewDto; }
    async chatRefineResume(personaId: string, instruction: string, confirm?: boolean, refinedSummary?: string, baseVersionId?: string, instructionType?: ResumeInstructionType, proposalId?: string, contentHash?: string) { return this.background("chat_refine_resume", { personaId, instruction, ...(instructionType && !confirm ? { instructionType } : {}), ...(proposalId ? { proposalId } : {}), ...(contentHash ? { contentHash } : {}), ...(confirm === undefined ? {} : { confirm }), ...(refinedSummary === undefined ? {} : { refinedSummary }), ...(baseVersionId === undefined ? {} : { baseVersionId }) }); }
    async listResumeVersions(personaId: string) { const result = obj(unwrap(await this.client.listResumeVersions(personaId))); return Array.isArray(result.items) ? result.items as Json[] : []; }
    async diffResumeVersions(left: string, right: string) { return unwrap(await this.client.diffResumeVersions(left, right)); }
    async restoreResumeVersion(personaId: string, versionId: string) { unwrap(await this.client.restoreResumeVersion(personaId, versionId)); }
    async analyzeJob(jdText: string, personaId: string) { const parsed = obj(unwrap(await this.client.parseJD(jdText))); unwrap(await this.client.matchJob(text(parsed.id), personaId)); }
    async collectJobUrl(url: string) { return unwrap(await this.client.collectJobUrl(url)); }
    async deleteJob(id: string) { unwrap(await this.client.deleteJob(id)); }
    async updateJobStatus(matchId: string, status: string) { const expectedVersion = this.jobVersions.get(matchId); if (!expectedVersion)
        throw { code: "CONFLICT", message: "岗位状态已变化，请刷新后重试" }; const updated = obj(unwrap(await this.client.updateMatchStatusVersioned(matchId, status, expectedVersion))); this.jobVersions.set(matchId, num(updated.version, expectedVersion + 1)); }
    async getJobStatusEvents(matchId: string) { return unwrap(await this.client.getJobStatusEvents(matchId)); }
    async reframeJob(jobId: string) { const matches = unwrap(await this.client.getJobMatches(jobId)) as Json[]; const first = matches[0]; if (!first)
        throw { code: "NOT_FOUND", message: "该岗位还没有匹配记录" }; const matchId = text(obj(first).id); await this.background("reframe_resume", { matchId }); return unwrap(await this.client.getReframeResults(matchId)); }
    async updateReframe(id: string, summary: string) { unwrap(await this.client.updateReframe(id, summary)); }
    async resetReframe(id: string) { unwrap(await this.client.resetReframe(id)); }
    async searchSkills(query: string) { return (unwrap(await this.client.searchSkills(query)) as Json[]).map(v => { const x = obj(v); return { id: text(x.id), name: text(x.name), level: text(x.level, "待评估"), evidence: num(x.evidence), custom: x.custom === true }; }); }
    async getSkillResources(skillId: string) { return (unwrap(await this.client.getSkillResources(skillId)) as Json[]).map(obj).map(r => ({ kind: text(r.kind), title: text(r.title), source: text(r.source), url: text(r.url), estimatedHours: num(r.estimatedHours) })); }
    async openResource(url: string) { const collected = obj(unwrap(await this.client.collectJobUrl(url))); const token = text(collected.openToken); if (!token)
        throw { code: "UNAVAILABLE", message: "无法为该资源创建安全打开令牌" }; unwrap(await this.client.openExternalUrl(token)); }
    async saveCustomSkill(skill: {
        id?: string;
        name: string;
        category: string;
        level: number;
    }) { const value = { ...skill, id: skill.id ?? crypto.randomUUID(), ownerId: "default" }; unwrap(await (skill.id ? this.client.updateCustomSkill(value) : this.client.createCustomSkill(value))); try {
        const result = obj(await this.background("enrich_custom_skill_resources", { skillId: value.id, ownerId: "default" }));
        return { resourceCount: num(result.resourceCount) };
    }
    catch (error) {
        return { resourceWarning: text(obj(error as Json).message, "AI 学习资源分析暂不可用，可检查 AI 设置后编辑技能重试。") };
    } }
    async deleteCustomSkill(id: string) { unwrap(await this.client.deleteCustomSkill(id)); }
    async deleteLearningPath(pathId: string, expectedVersion: number) { unwrap(await this.client.deleteLearningPath(pathId, expectedVersion)); }
    async simulateWhatIf(requiredSkills: string[], currentSkills: string[], hypotheticalSkills: string[]) { return unwrap(await this.client.simulateSkillWhatIf({ requiredSkills, currentSkills, hypotheticalSkills })); }
    async simulateJobWhatIf(personaId: string, jobMatchId: string, hypotheticalSkills: string[]) { return unwrap(await this.client.simulateSkillWhatIf({ personaId, jobMatchId, hypotheticalSkills })); }
    async updateLearning(id: string, status: string, _expectedVersion = 1, completionNote?: string) { let [pathId = "", itemId = ""] = id.split(":"); if (!itemId) {
        itemId = pathId;
        pathId = "";
    } const paths = unwrap(await this.client.getLearningPaths()) as Json[]; const path = paths.map(obj).find(p => (!pathId || text(p.pathId, text(p.id)) === pathId) && Array.isArray(p.items) && (p.items as Obj[]).some(i => text(i.id) === itemId)); pathId = text(path?.pathId, text(path?.id, pathId)); const item = Array.isArray(path?.items) ? (path.items as Obj[]).find(i => text(i.id) === itemId) : undefined; const expectedVersion = num(item?.version, 1); unwrap(await this.client.updateLearningProgress({ pathId, itemId, status: status as import("./contracts").LearningProgressRequest["status"], expectedVersion, ...(completionNote ? { completionNote } : {}) })); }
    async completeLearning(id: string, draft: {
        title: string;
        organization: string;
        rawDescription: string;
    }) { const [, itemId = ""] = id.split(":"); const result = obj(unwrap(await this.client.completeLearningToExperience({ itemId, experienceId: crypto.randomUUID(), ...draft }))), value = obj(result.draft as Json); return { ...value, conversionId: text(result.conversionId), sourceSnapshot: obj(result.sourceSnapshot as Json) } as unknown as LearningExperienceDraft; }
    async resolveLearningExperience(draft: LearningExperienceDraft, status: "confirmed" | "discarded") { unwrap(await this.client.saveExperience({ id: draft.id, version: draft.version, status })); }
    async startTask(operation: "generate_resume" | "parse_job" | "generate_learning_path" | "test_provider" | "structure_experience" | "chat_refine_resume" | "recommend_persona_weights", payload: Obj = {}) { if (typeof this.client.startBackgroundTask !== "function") {
        if (operation === "structure_experience" || operation === "chat_refine_resume" || operation === "recommend_persona_weights")
            throw { code: "UNAVAILABLE", message: "AI 操作需要后台任务支持" };
        const result = operation === "parse_job" ? await this.client.parseJD(text(payload.jdText)) : operation === "test_provider" ? await this.client.testLLMConnection() : operation === "generate_learning_path" ? await this.client.getLearningPath(text(payload.skill)) : await this.client.generateResume(text(payload.personaId), text(payload.template, "classic") as ResumeTemplateId);
        if (!result.success)
            throw result.error;
        return "sync-complete";
    } const names = { parse_job: "parse_jd", test_provider: "test_llm_connection", generate_learning_path: "generate_learning_path", generate_resume: "generate_resume", structure_experience: "structure_experience", chat_refine_resume: "chat_refine_resume", recommend_persona_weights: "recommend_persona_weights" } as const; const started = obj(unwrap(await this.client.startBackgroundTask(names[operation], payload))); return text(started.taskId); }
    async getTask(taskId: string) { return obj(unwrap(await this.client.getBackgroundTask(taskId))); }
    async cancelTask(taskId: string) { return obj(unwrap(await this.client.cancelBackgroundTask(taskId))); }
}
export interface JobActions {
    analyzeJob(jdText: string, personaId: string): Promise<void>;
    deleteJob(id: string): Promise<void>;
    updateJobStatus(jobId: string, status: string): Promise<void>;
    reframeJob(jobId: string): Promise<Json>;
}
export interface UrlActions {
    collectJobUrl(url: string): Promise<Json>;
}
export interface ReframeActions extends JobActions {
    updateReframe(id: string, summary: string): Promise<void>;
    resetReframe(id: string): Promise<void>;
    getJobStatusEvents(matchId: string): Promise<import("./contracts").JobStatusEventDto[]>;
}
export interface V2Actions {
    resetFitScore(personaId: string, experienceId: string): Promise<void>;
    deleteProvider(name: string): Promise<void>;
    exportPortableBackup(destinationPath: string): Promise<import("./contracts").PortableExportDto>;
    inspectPortableBackup(archivePath: string): Promise<import("./contracts").PortableInspectDto>;
    importPortableBackup(archivePath: string): Promise<import("./contracts").PortableImportDto>;
}
export interface ResumeActions {
    generateResume(personaId: string, template: ResumeTemplateId): Promise<Json>;
    previewResume(personaId: string, template: ResumeTemplateId): Promise<ResumePreviewDto>;
    chatRefineResume(personaId: string, instruction: string, confirm?: boolean, refinedSummary?: string, baseVersionId?: string, instructionType?: ResumeInstructionType, proposalId?: string, contentHash?: string): Promise<Json>;
    listResumeVersions(personaId: string): Promise<Json[]>;
    diffResumeVersions(left: string, right: string): Promise<Json>;
    restoreResumeVersion(personaId: string, versionId: string): Promise<void>;
}
export interface ExperienceActions {
    importFile(file: File, options?: {
        commit?: boolean;
    }): Promise<{
        count: number;
        content: string;
    }>;
    updateExperience(value: Experience): Promise<void>;
}
export interface SkillLearningActions {
    searchSkills(query: string): Promise<Skill[]>;
    getSkillResources(skillId: string): Promise<SkillResource[]>;
    openResource(url: string): Promise<void>;
    saveCustomSkill(skill: {
        id?: string;
        name: string;
        category: string;
        level: number;
    }): Promise<{
        resourceCount?: number;
        resourceWarning?: string;
    } | void>;
    deleteCustomSkill(id: string): Promise<void>;
    deleteLearningPath(pathId: string, expectedVersion: number): Promise<void>;
    simulateWhatIf(requiredSkills: string[], currentSkills: string[], hypotheticalSkills: string[]): Promise<Json>;
    updateLearning(id: string, status: string): Promise<void>;
    completeLearning(id: string, draft: {
        title: string;
        organization: string;
        rawDescription: string;
    }): Promise<LearningExperienceDraft>;
    resolveLearningExperience(draft: LearningExperienceDraft, status: "confirmed" | "discarded"): Promise<void>;
}
export interface FixtureDataSource extends V2Actions {
}
Object.assign(FixtureDataSource.prototype, { async resetFitScore() { }, async deleteProvider(this: FixtureDataSource, name: string) { await this.saveProvider({ id: name, name, model: "", enabled: false, hasKey: false }); }, async exportPortableBackup() { throw { code: "UNAVAILABLE", message: "演示模式不可导出数据包" }; }, async inspectPortableBackup() { throw { code: "UNAVAILABLE", message: "演示模式不可导入数据包" }; }, async importPortableBackup() { throw { code: "UNAVAILABLE", message: "演示模式不可导入数据包" }; } });
export interface FixtureDataSource extends ResumeActions {
}
Object.assign(FixtureDataSource.prototype, { async generateResume() { return {}; }, async previewResume(_personaId: string, template: ResumeTemplateId) { return { personaId: _personaId, template, markdown: "# Preview", contentHash: "fixture", selectedExperienceIds: [], fitScores: [], warnings: [] }; }, async chatRefineResume() { return {}; }, async listResumeVersions() { return []; }, async diffResumeVersions() { return {}; }, async restoreResumeVersion() { } });
export interface FixtureDataSource extends ExperienceActions {
}
Object.assign(FixtureDataSource.prototype, { async importFile() { return { count: 0, content: "" }; }, async updateExperience(this: FixtureDataSource, value: Experience) { const state = this as unknown as {
        data: WorkspaceSnapshot;
    }; state.data.experiences = state.data.experiences.map(x => x.id === value.id ? value : x); } });
export interface FixtureDataSource extends ReframeActions {
}
Object.assign(FixtureDataSource.prototype, { async updateReframe() { }, async resetReframe() { } });
export interface SkillLearningActions {
    updateLearning(id: string, status: string, expectedVersion?: number, completionNote?: string): Promise<void>;
}
Object.assign(FixtureDataSource.prototype, { async getJobStatusEvents() { return []; }, async simulateJobWhatIf(_personaId: string, _jobMatchId: string, h: string[]) { return { baselineScore: 50, simulatedScore: 50 + h.length * 10, delta: h.length * 10, baselineBreakdown: { skills: 10, experience: 20, industry: 10, education: 10 }, simulatedBreakdown: { skills: 10 + h.length * 10, experience: 20, industry: 10, education: 10 }, addedSkills: h, remainingMissing: [], assumption: "假设掌握不等于已有项目证据，结果仅供学习规划" }; } });
export interface FixtureDataSource extends UrlActions {
}
Object.assign(FixtureDataSource.prototype, { async collectJobUrl(_url: string) { return { manualFallback: true }; } });
export interface FixtureDataSource extends SkillLearningActions {
}
Object.assign(FixtureDataSource.prototype, { async getSkillResources() { return []; }, async openResource() { }, async saveCustomSkill(this: FixtureDataSource, skill: {
        id?: string;
        name: string;
        category: string;
        level: number;
    }) { const data = await this.load(); (this as unknown as {
        data: WorkspaceSnapshot;
    }).data.skills = data.skills.filter(x => x.id !== skill.id).concat({ id: skill.id ?? crypto.randomUUID(), name: skill.name, level: String(skill.level), evidence: 0, custom: true }); }, async deleteCustomSkill(this: FixtureDataSource, id: string) { const state = this as unknown as {
        data: WorkspaceSnapshot;
    }; state.data.skills = state.data.skills.filter(x => x.id !== id); }, async deleteLearningPath(this: FixtureDataSource, pathId: string) { const state = this as unknown as { data: WorkspaceSnapshot }; state.data.learning = state.data.learning.filter(item => item.pathId !== pathId); }, async simulateWhatIf(_r: string[], _c: string[], h: string[]) { return { delta: h.length * 10 }; }, async updateLearning() { }, async completeLearning(_id: string, draft: {
        title: string;
        organization: string;
        rawDescription: string;
    }) { return { id: crypto.randomUUID(), version: 1, status: "draft", ...draft, sourceLearningTitle: draft.title, completionNote: draft.rawDescription }; }, async resolveLearningExperience() { } });
Object.assign(FixtureDataSource.prototype, { async searchSkills(this: FixtureDataSource, q: string) { return (await this.load()).skills.filter(x => x.name.toLowerCase().includes(q.toLowerCase())); } });
Object.assign(FixtureDataSource.prototype, { async completeLearning(_id: string, draft: {
        title: string;
        organization: string;
        rawDescription: string;
    }) { const itemId = _id.split(":").at(-1) ?? _id, pathId = _id.includes(":") ? _id.split(":")[0] : "fixture-path"; return { id: crypto.randomUUID(), version: 1, status: "draft", ...draft, sourceLearningTitle: draft.title, completionNote: draft.rawDescription, conversionId: crypto.randomUUID(), sourceSnapshot: { itemId, pathId, skillId: "fixture-skill", title: draft.title, completionNote: draft.rawDescription } }; } });
export function createDataSource() { return import.meta.env.MODE === "test" || import.meta.env.MODE === "demo" ? new FixtureDataSource() : new ClientDataSource(); }
