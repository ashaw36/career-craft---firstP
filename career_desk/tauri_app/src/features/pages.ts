import type { Experience, WorkspaceSnapshot } from "../api/data-source";
import type { StructuredExperienceDraftDto } from "../api/contracts";
import type { RouteId } from "../shared/state/navigation";
const esc = (v: string) => v.replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] ?? c));
const head = (t: string, d: string, a: string, id: string) => `<header class="page-header"><div><h1 id="page-title">${t}</h1><p>${d}</p></div><button class="primary" data-action="${id}">${a}</button></header>`;
const tags = (v: string[]) => `<span class="tags">${v.map(x => `<span>${esc(x)}</span>`).join("")}</span>`;
const rows = (items: string[]) => `<section class="rows">${items.join("")}</section>`;
function home(d: WorkspaceSnapshot) { return `${head("你好，欢迎回来", "把散落的工作经历整理成可复用的职业资产。", "添加经历", "add-experience")}<section class="document-section"><h2>下一步</h2><p>你已有 ${d.experiences.length} 段经历和 ${d.personas.length} 个角色档案。可以继续完善证据，或针对目标岗位生成简历。</p><button class="secondary" data-route-jump="jobs">分析岗位</button> <button class="ghost" data-route-jump="resumes">查看简历</button></section><section class="document-section"><h2>最近工作</h2>${d.resumes.map(x => `<button class="content-row"><span><strong>${esc(x.title)}</strong><small>${esc(x.persona)} · 第 ${x.version} 版</small></span><small>${x.updatedAt}</small></button>`).join("")}</section>`; }
export function experienceConflictIds(items: WorkspaceSnapshot["experiences"]) { return new Set(items.filter(x => (x.overlapExperienceIds?.length ?? 0) > 0 || x.warnings?.some(w => w.code === "DATE_OVERLAP")).map(x => x.id)); }
function experiences(d: WorkspaceSnapshot) { const conflicts = experienceConflictIds(d.experiences); return `${head("经历库", "保存工作、项目和教育经历，原始描述始终保留。", "添加经历", "add-experience")}<div class="toolbar"><input type="search" data-experience-search aria-label="搜索经历" placeholder="搜索经历、组织或技能"><select data-experience-kind aria-label="经历类型"><option value="">全部类型</option><option value="工作">工作</option><option value="项目">项目</option><option value="教育">教育</option><option value="认证">认证</option></select><button type="button" class="secondary" data-experience-filter>筛选</button><button class="secondary" data-action="import-experience">导入文件</button></div>${rows(d.experiences.map(x => { const haystack = esc(`${x.title} ${x.organization} ${x.skills.join(" ")}`.toLowerCase()); return `<article class="content-row" data-experience-id="${esc(x.id)}" data-kind="${esc(x.kind)}" data-search-text="${haystack}"><div class="experience-main"><div class="experience-title-row"><strong>${esc(x.title)}</strong><div class="experience-actions">${x.status === "draft" ? `<button class="ghost" data-edit-experience="${esc(x.id)}">编辑</button><button class="primary" data-confirm-existing-draft="${esc(x.id)}">确认</button><button class="ghost danger-text" data-discard-existing-draft="${esc(x.id)}">丢弃</button>` : `<button class="ghost" data-edit-experience="${esc(x.id)}">编辑</button><button class="ghost danger-text" data-delete-experience="${esc(x.id)}" data-version="${x.version}">删除</button>`}</div></div><small>${esc(x.organization)} · ${x.period} · ${x.kind}</small>${x.status === "draft" ? '<span class="status-label">待确认</span>' : ''}${conflicts.has(x.id) ? '<p class="warning" role="status">日期与其他经历重叠，请确认是否属实。</p>' : ''}${tags(x.skills)}<details><summary>查看原始描述</summary><p>${esc(x.original)}</p></details></div></article>`; }))}<p class="empty-state" data-experience-empty hidden>没有符合条件的经历。</p>`; }
function personas(d: WorkspaceSnapshot) { return `${head("角色档案", "为不同目标岗位组织定位与经历权重。", "创建角色", "add-persona")}${rows(d.personas.map(x => `<article class="content-row"><div><strong>${esc(x.name)}</strong><small>目标：${esc(x.targetRole)}</small><p>${esc(x.positioning)}</p></div><div class="persona-actions"><button class="secondary" data-fit-persona="${esc(x.id)}">调整经历权重</button><button class="ghost" data-edit-persona="${esc(x.id)}">编辑</button><button class="ghost danger-text" data-delete-persona="${esc(x.id)}">删除</button></div></article>`))}<p class="helper">删除角色不会删除关联的原始经历。权重决定该角色生成简历时优先选用哪些经历。</p>`; }
function resumes(d: WorkspaceSnapshot) {
    const selectedPersona = d.personas.find(x => typeof localStorage !== "undefined" && localStorage.getItem("careercraft:selected-persona") === x.id) ?? d.personas[0];
    const selected = d.resumes.filter(x => !selectedPersona || x.persona === selectedPersona.id).at(-1);
    return `${head("简历", "为当前角色生成、调优并导出 Markdown。", "生成并保存", "generate-resume")}
<div class="toolbar resume-context" aria-label="简历上下文">
  <label class="resume-persona">角色
    <select data-resume-persona aria-label="选择角色">${d.personas.map(x => `<option value="${esc(x.id)}" ${selectedPersona?.id === x.id ? "selected" : ""}>${esc(x.name)}</option>`).join("")}${d.personas.length ? "" : '<option value="">请先创建角色</option>'}</select>
  </label>
  <p class="resume-status" data-resume-status role="status">${selected ? `已保存 · 第 ${selected.version} 版` : "尚未预览或保存"}</p>
  <button type="button" class="ghost" data-resume-versions>版本管理</button>
</div>
<section class="paper resume-stage">
  <p class="eyebrow">纸面预览</p>
  ${selected?.preview ? `<pre class="resume-preview" data-persona-id="${esc(selected.persona)}" data-template="${esc(selected.template)}">${esc(selected.preview)}</pre>` : `<section class="empty-state"><h2>还没有简历预览</h2><p>切换角色会自动载入该角色最新版本；也可直接点「生成并保存」。</p></section>`}
</section>
<div class="toolbar resume-actions" aria-label="简历操作">
  <button type="button" class="secondary" data-action="resume-refine">对话调优</button>
  <button type="button" class="primary" data-action="export-markdown">导出 Markdown</button>
</div>`;
}
const educationLabels: Record<string, string> = { none: "未指定", high_school: "高中", associate: "专科", bachelor: "本科", master: "硕士", doctorate: "博士", other: "其他" };
const statusLabels: Record<string, string> = { new: "新岗位", interested: "感兴趣", applied: "已投递", interviewing: "面试中", offered: "收到录用", rejected: "未通过", ghosted: "暂无回应", accepted: "已接受", declined: "已婉拒" };
const statusTransitions: Record<string, string[]> = {
    new: ["interested", "applied", "rejected", "declined"],
    interested: ["applied", "rejected", "declined"],
    applied: ["interviewing", "rejected", "ghosted", "declined"],
    interviewing: ["offered", "rejected", "ghosted", "declined"],
    offered: ["accepted", "declined"],
    ghosted: ["interviewing", "rejected", "declined"],
};
const statusOptions = (current: string) => [current, ...(statusTransitions[current] ?? [])].map(v => `<option value="${v}" ${current === v ? "selected" : ""}>${statusLabels[v] ?? v}</option>`).join("");
function jobs(d: WorkspaceSnapshot) {
    const selectedPersona = d.personas.find(x => typeof localStorage !== "undefined" && localStorage.getItem("careercraft:selected-persona") === x.id) ?? d.personas[0];
    const savedJobId = typeof localStorage !== "undefined" ? localStorage.getItem("careercraft:selected-job") : null;
    const selected = d.jobs.find(x => x.id === savedJobId) || d.jobs[0];
    const personaName = selectedPersona?.name ?? "当前角色";
    const index = d.jobs.map(x => {
        const active = selected && x.id === selected.id ? " selected" : "";
        return `<button type="button" class="content-row${active}" data-job-row data-select-job="${esc(x.id)}" data-status="${esc(x.status)}" aria-pressed="${selected && x.id === selected.id ? "true" : "false"}"><span><strong>${esc(x.title)}</strong><small>${esc(x.company || "未填公司")} · ${esc(statusLabels[x.status] ?? x.status)}</small><small class="job-index-score">匹配 ${x.score}</small></span></button>`;
    }).join("");
    const detail = selected ? (() => {
        const gapList = selected.missing.length ? selected.missing.map(skill => `<li><span>${esc(skill)}</span> <button type="button" class="ghost" data-learn-skill="${esc(skill)}">安排学习</button></li>`).join("") : "<li class=\"helper\">暂无明确补强项</li>";
        const matchedList = selected.matched.length ? selected.matched.map(skill => `<li>${esc(skill)}</li>`).join("") : "<li class=\"helper\">暂无已匹配技能</li>";
        const industry = selected.industryTags?.length ? selected.industryTags.map(esc).join("、") : "";
        const education = selected.educationLevels?.length ? selected.educationLevels.map(v => educationLabels[v] ?? v).join("、") : "";
        const raw = selected.rawText?.trim() || "暂无岗位原文。可在「分析岗位」时粘贴完整 JD。";
        const summary = `与「${esc(personaName)}」匹配 ${selected.score} 分${selected.missing.length ? ` — 需补强 ${esc(selected.missing.slice(0, 2).join("、"))}${selected.missing.length > 2 ? " 等" : ""}` : selected.matched.length ? " — 技能覆盖较好" : ""}。`;
        const breakdown = selected.scoreBreakdown ? `<p class="helper" aria-label="匹配分项">行业匹配 ${selected.scoreBreakdown.industry} · 学历匹配 ${selected.scoreBreakdown.education} · 技能 ${selected.scoreBreakdown.skills} · 经历 ${selected.scoreBreakdown.experience}</p>` : "";
        const provenance = selected.evidenceSources ? `<details><summary>证据来源</summary>${Object.entries(selected.evidenceSources).map(([key, value]) => `<p>${esc(key)}：<span class="status-label">${value === "legacy_heuristic" ? "旧数据启发式推断（非结构化事实）" : "结构化持久证据"}</span></p>`).join("")}</details>` : "";
        return `<section class="job-detail" data-job-detail="${esc(selected.id)}">
<section class="paper job-stage">
  <p class="eyebrow">岗位原文</p>
  <h2 class="job-paper-title">${esc(selected.title)}</h2>
  <p class="helper">${esc(selected.company || "未填公司")} · ${esc(statusLabels[selected.status] ?? selected.status)}</p>
  <pre class="job-raw">${esc(raw)}</pre>
</section>
<section class="document-section job-summary-block">
  <h2>匹配摘要</h2>
  <p>${summary}</p>
  ${breakdown}
  ${provenance}
  <button type="button" class="ghost" data-job-evidence="${esc(selected.id)}">查看分项与证据来源</button>
</section>
<section class="document-section job-skills-block">
  <h2>技能对照</h2>
  <div class="job-skill-columns">
    <div><h3>已具备</h3><ul class="job-skill-list">${matchedList}</ul></div>
    <div><h3>需补强</h3><ul class="job-skill-list">${gapList}</ul></div>
  </div>
  ${industry || education ? `<p class="helper">${industry ? `<strong>行业：</strong>${industry}` : ""}${industry && education ? " · " : ""}${education ? `<strong>学历：</strong>${education}` : ""}</p>` : ""}
</section>
<div class="toolbar job-actions" aria-label="岗位操作">
  <label class="job-status-label">投递状态
    <select data-job-status="${esc(selected.matchId || selected.id)}" data-job-desc="${esc(selected.id)}" data-current-status="${esc(selected.status)}" aria-label="投递状态">${statusOptions(selected.status)}</select>
  </label>
  <button type="button" class="secondary" data-reframe-job="${esc(selected.id)}">定向重述</button>
  <button type="button" class="secondary" data-job-what-if="${esc(selected.id)}">试算技能影响</button>
  <button type="button" class="ghost danger-text" data-delete-job="${esc(selected.id)}">删除岗位</button>
</div>
</section>`;
    })() : `<section class="empty-state"><h2>还没有岗位</h2><p>点右上角「分析岗位」，粘贴 JD 后即可查看匹配证据。</p></section>`;
    return `${head("岗位匹配", "保留岗位原文，查看与当前角色的匹配证据与差距。", "分析岗位", "analyze-job")}
<div class="toolbar job-context" aria-label="岗位匹配上下文">
  <label class="resume-persona">匹配角色
    <select data-job-persona aria-label="选择匹配角色">${d.personas.map(x => `<option value="${esc(x.id)}" ${selectedPersona?.id === x.id ? "selected" : ""}>${esc(x.name)}</option>`).join("")}${d.personas.length ? "" : '<option value="">请先创建角色</option>'}</select>
  </label>
  <label class="resume-persona">状态筛选
    <select data-job-filter aria-label="按投递状态筛选"><option value="">全部状态</option>${["new", "interested", "applied", "interviewing", "offered", "rejected", "ghosted", "accepted", "declined"].map(v => `<option value="${v}">${statusLabels[v]}</option>`).join("")}</select>
  </label>
</div>
<div class="split-layout job-layout">
  <aside class="job-index" aria-label="岗位列表">${d.jobs.length ? index : `<p class="helper">暂无岗位记录</p>`}</aside>
  <div class="job-detail-host">${detail}</div>
</div>`;
}
const skillCategoryLabel = (value?: string) => ({ product_management: "产品", technical: "技术", management: "管理", industry: "行业" }[value ?? ""] ?? "其他");
const skillLevelLabel = (value: string) => ({ "1": "基础", "2": "进阶", "3": "高阶" }[value] ?? "待评估");
function skills(d: WorkspaceSnapshot, selectedId?: string, detailOpen = false, filter?: {
    query?: string;
    category?: string;
}) {
    const selected = d.skills.find(x => x.id === selectedId) ?? d.skills[0];
    const dependents = selected ? d.skills.filter(x => x.prerequisites?.includes(selected.id)) : [];
    const related = (ids: string[] | undefined, attribute: string) => ids?.length ? ids.map(id => { const item = d.skills.find(x => x.id === id); return `<button type="button" class="skill-relation" ${attribute}="${esc(id)}" data-skill-detail="${esc(id)}">${esc(item?.name ?? id)}</button>`; }).join("") : '<span class="helper">暂无</span>';
    const resources = selected?.resources?.length ? selected.resources.map(item => `<article class="skill-resource"><div><strong>${esc(item.title)}</strong><small>${esc(item.source)} · ${esc(item.kind)} · 约 ${item.estimatedHours} 小时</small></div><div class="skill-resource-actions"><button type="button" class="ghost" data-copy-resource="${esc(item.url)}">复制链接</button><button type="button" class="secondary" data-open-resource="${esc(item.url)}">安全打开</button></div></article>`).join("") : '<p class="helper">暂无可靠学习资源。</p>';
    const detail = selected ? `<section class="skill-detail-panel form-dialog" data-skill-detail-panel role="dialog" aria-modal="${detailOpen ? "true" : "false"}" aria-labelledby="skill-detail-title"><header class="skill-detail-heading"><div><span class="status-label">${skillCategoryLabel(selected.category)} · ${skillLevelLabel(selected.level)}</span><h2 id="skill-detail-title">${esc(selected.name)}</h2></div><button type="button" class="ghost skill-detail-close" data-close-skill-detail aria-label="关闭技能详情">关闭</button></header><p class="skill-description">${esc(selected.description || "暂无技能说明。")}</p><section class="skill-relation-block"><h3>学习关系</h3><div><span>建议先学</span><p>${related(selected.prerequisites, "data-skill-prerequisite")}</p></div><div><span>后续可学</span><p>${related(dependents.map(x => x.id), "data-skill-dependent")}</p></div></section><section><h3>学习资源</h3><div class="skill-resource-list">${resources}</div></section>${selected.aliases?.length ? `<details class="skill-aliases"><summary>搜索别名</summary><p>${selected.aliases.map(esc).join("、")}</p></details>` : ""}<div class="dialog-actions skill-detail-actions"><button type="button" class="primary" data-generate-skill-path="${esc(selected.name)}">为此技能生成学习路径</button></div></section>` : `<section class="empty-state skill-detail-empty"><h2>选择一个技能</h2><p>查看学习关系和可靠资源。</p></section>`;
    const list = d.skills.map(x => `<article class="skill-index-row${selected?.id === x.id ? " selected" : ""}" data-skill-row data-category="${esc(x.category ?? "")}" data-search-text="${esc([x.name, ...(x.aliases ?? [])].join(" ").toLowerCase())}"><button type="button" class="skill-index-select" data-skill-detail="${esc(x.id)}"><strong>${esc(x.name)}</strong><small>${skillCategoryLabel(x.category)} · ${skillLevelLabel(x.level)}${x.custom ? " · 自定义" : ""}</small></button>${x.custom ? `<span class="skill-custom-actions"><button type="button" class="ghost" data-edit-skill="${esc(x.id)}">编辑</button><button type="button" class="ghost danger-text" data-delete-skill="${esc(x.id)}">删除</button></span>` : ""}</article>`).join("");
    const query = filter?.query ?? "", category = filter?.category ?? "";
    return `${head("技能图谱", "从技能关系中找到下一项值得学习的能力。", "添加自定义技能", "add-skill")}<div class="toolbar skill-toolbar"><input type="search" data-skill-search aria-label="搜索技能" placeholder="按名称或别名搜索" value="${esc(query)}"><select data-skill-category aria-label="技能分类"><option value="" ${category === "" ? "selected" : ""}>全部分类</option><option value="product_management" ${category === "product_management" ? "selected" : ""}>产品</option><option value="technical" ${category === "technical" ? "selected" : ""}>技术</option><option value="management" ${category === "management" ? "selected" : ""}>管理</option><option value="industry" ${category === "industry" ? "selected" : ""}>行业</option></select><span class="helper" data-skill-count aria-live="polite">${d.skills.length} 项</span><button type="button" class="secondary" data-what-if>假设分析</button></div><div class="skill-layout"><aside class="skill-index" data-skill-catalog aria-label="技能目录">${list || '<p class="empty-state">暂无技能。</p>'}<p class="empty-state" data-skill-empty hidden>没有符合条件的技能。</p></aside><div class="skill-detail-host${detailOpen ? " is-open" : ""}" data-skill-dialog>${detail}</div></div>`;
}
function learning(d: WorkspaceSnapshot) {
    let pending: Record<string, unknown> = {};
    try { pending = JSON.parse(typeof localStorage !== "undefined" ? localStorage.getItem("careercraft:learning-context") ?? "{}" : "{}"); }
    catch { pending = {}; }
    const focus = String(pending.skill ?? "");
    const grouped = new Map<string, typeof d.learning>();
    d.learning.forEach(item => { const key = item.pathId ?? "legacy"; grouped.set(key, [...(grouped.get(key) ?? []), item]); });
    const statusLabel: Record<string, string> = { pending: "待开始", in_progress: "进行中", completed: "已完成", skipped: "已跳过" };
    const originLabel: Record<string, string> = { skill_graph: "技能图谱", job_gap: "岗位缺口", what_if: "假设分析", manual: "手动规划" };
    const pathEntries = [...grouped.entries()];
    const pathSummary = { active: 0, completed: 0, archived: 0 };
    pathEntries.forEach(([, values]) => { const state = values[0]?.pathStatus ?? "active"; if (state in pathSummary) pathSummary[state as keyof typeof pathSummary]++; });
    const index = pathEntries.map(([pathId, values]) => {
        const first = values[0]!;
        const completed = values.filter(item => item.status === "completed").length;
        const skipped = values.filter(item => item.status === "skipped").length;
        const state = first.pathStatus === "completed" ? "已完成" : first.pathStatus === "archived" ? "已归档" : "进行中";
        const context = [first.personaName, first.jobTitle].filter(Boolean).join(" · ");
        return `<button type="button" class="learning-index-link${first.pathStatus === "archived" ? " is-archived" : ""}" data-learning-jump="${esc(pathId)}"><span><strong>${esc(first.skill)}</strong><small>${esc(originLabel[first.origin ?? ""] ?? first.origin ?? "学习计划")}${context ? ` · ${esc(context)}` : ""}</small></span><span class="learning-index-progress"><small>${completed} 完成${skipped ? ` · ${skipped} 跳过` : ""} / ${values.length}</small><span class="status-label">${state}</span></span></button>`;
    }).join("");
    const paths = pathEntries.map(([pathId, values]) => {
        const first = values[0]!;
        const completed = values.filter(item => item.status === "completed").length;
        const skipped = values.filter(item => item.status === "skipped").length;
        const items = values.sort((a, b) => (a.sequence ?? 0) - (b.sequence ?? 0)).map(item => {
            const id = item.id.includes(":") ? item.id : `${pathId}:${item.id}`;
            const status = statusLabel[item.status] ?? item.status;
            const marker = item.status === "completed" ? "✓" : String(item.sequence ?? 1);
            const resultAction = item.status === "completed" ? (item.convertedExperienceId ? `<span class="learning-result-done">✓ 已整理为成果</span>` : `<button class="ghost learning-result-action" data-complete-learning="${esc(id)}">整理为学习成果</button>`) : "";
            const resource = item.resourceUrl ? `<div class="learning-resource-row"><div><strong>${esc(item.title)}</strong><small>${esc(item.source)} · 外部地址将在打开前校验</small></div><div><button class="ghost" data-copy-resource="${esc(item.resourceUrl)}">复制链接</button><button class="secondary" data-open-resource="${esc(item.resourceUrl)}">安全打开</button></div></div>` : "";
            return `<li class="learning-step learning-step-${esc(item.status)}"><span class="learning-step-marker" aria-hidden="true">${marker}</span><article data-learning-row data-source="${esc(item.source)}"><div class="learning-step-body"><header class="learning-step-heading"><div><small>第 ${item.sequence ?? 1} 步 · ${esc(status)}</small><h3>${esc(item.title)}</h3></div><small>${esc(item.source)} · ${esc(item.effort)}</small></header><p class="learning-objective">${esc(item.objective || "掌握本项核心内容")}</p><dl class="learning-step-spec"><div><dt>实践任务</dt><dd>${esc(item.practiceTask || "完成一次实践并记录结果")}</dd></div><div><dt>完成标准</dt><dd>${esc(item.completionCriteria || "提交实践说明或复盘")}</dd></div></dl>${resource}</div><aside class="learning-step-actions"><label><span>学习状态</span><select data-learning-status="${esc(id)}" aria-label="${esc(item.title)}学习进度"><option value="pending" ${item.status === "pending" ? "selected" : ""}>待开始</option><option value="in_progress" ${item.status === "in_progress" ? "selected" : ""}>进行中</option><option value="completed" ${item.status === "completed" ? "selected" : ""}>已完成</option><option value="skipped" ${item.status === "skipped" ? "selected" : ""}>跳过</option></select></label>${resultAction || `<small class="learning-result-hint">完成后可整理为经历证据</small>`}</aside></article></li>`;
        }).join("");
        const status = first.pathStatus === "completed" ? "路径已完成" : first.pathStatus === "archived" ? "已归档" : "进行中";
        const context = [first.personaName ? `角色：${first.personaName}` : "", first.jobTitle ? `岗位：${first.jobTitle}` : ""].filter(Boolean).join(" · ");
        return `<section class="learning-path${first.pathStatus === "archived" ? " is-archived" : ""}" id="learning-path-${esc(pathId)}" data-learning-path="${esc(pathId)}" aria-labelledby="learning-title-${esc(pathId)}"><header class="learning-path-header"><div class="learning-path-heading"><div class="learning-path-kicker"><span class="eyebrow">${esc(originLabel[first.origin ?? ""] ?? first.origin ?? "学习计划")}</span><span class="status-label">${status}</span></div><h2 id="learning-title-${esc(pathId)}">${esc(first.skill)}</h2>${context ? `<p class="learning-context-line">${esc(context)}</p>` : ""}<p>${esc(first.reason || `为「${first.personaName || "当前角色"}」安排能力补强`)}</p><small>${first.generationMode === "ai_enhanced" ? "AI 个性化建议；资源来自可信目录" : "基础路径；无需 AI 也可使用"}</small></div><div class="learning-path-control"><div class="learning-progress"><span>完成 ${completed} / 跳过 ${skipped} / 共 ${values.length}</span><progress value="${completed + skipped}" max="${values.length}" aria-label="${esc(first.skill)}路径进度">${completed + skipped}/${values.length}</progress></div>${first.pathStatus === "active" ? `<button class="secondary" data-action="regenerate-learning" data-skill="${esc(first.skill)}" data-persona-id="${esc(first.personaId ?? "")}" data-origin="${esc(first.origin ?? "skill_graph")}" data-job-match-id="${esc(first.jobMatchId ?? "")}">重新规划</button><small>重新规划会归档当前路径</small>` : ""}<button type="button" class="ghost danger-text" data-delete-learning-path="${esc(pathId)}" data-version="${first.pathVersion ?? 1}">删除路径</button></div></header>${first.guidance ? `<aside class="learning-guidance"><strong>路径建议</strong><p>${esc(first.guidance)}</p></aside>` : ""}<ol class="learning-steps">${items}</ol></section>`;
    }).join("");
    const contextParts = [String(pending.personaName ?? ""), String(pending.jobTitle ?? "")].filter(Boolean).join(" · ");
    const banner = focus ? `<section class="learning-context-banner" role="status"><div><strong>待规划技能：${esc(focus)}</strong><p>${contextParts ? `${esc(contextParts)} · ` : ""}生成时会保留角色与来源上下文。</p></div><span class="status-label">准备生成</span></section>` : "";
    const toolbar = `<div class="learning-toolbar"><p><strong>${pathSummary.active}</strong> 进行中 · <strong>${pathSummary.completed}</strong> 已完成 · <strong>${pathSummary.archived}</strong> 已归档</p><label><span>资源来源</span><select data-learning-source aria-label="按资源来源筛选"><option value="">全部来源</option>${[...new Set(d.learning.map(x => x.source))].map(x => `<option value="${esc(x)}">${esc(x)}</option>`).join("")}</select></label></div>`;
    const workspace = paths ? `<div class="learning-workspace"><nav class="learning-path-index" aria-label="学习路径列表"><h2>路径目录</h2>${index}</nav><div class="learning-path-detail">${paths}</div></div>` : `<section class="empty-state learning-empty"><h2>还没有学习路径</h2><p>先从岗位缺口、假设分析或技能图谱选择目标技能，把学习目标转化为实践成果。</p></section>`;
    return `${head("学习路径", "围绕角色和岗位差距，把学习转化为可验证的实践成果。", "生成学习路径", "generate-learning")}${banner}${toolbar}${workspace}`;
}
function settings(d: WorkspaceSnapshot) { return `${head("设置", "管理 AI 服务、隐私和本地数据。", "添加服务商", "add-provider")}<section class="settings-group"><h2>AI 服务商</h2>${d.providers.map(x => `<article class="setting-row"><div><strong>${esc(x.name)}</strong><small>${esc(x.model)} · 密钥${x.hasKey ? "已安全保存" : "未设置"}</small></div><div><span class="status-label">${x.enabled ? "已启用" : "已停用"}</span><button class="secondary" data-test-provider="${esc(x.name)}">测试连接</button><button class="ghost" data-edit-provider="${esc(x.id)}">编辑</button></div></article>`).join("")}${d.providers.length ? "" : `<p>尚未配置服务商。添加后才能测试连接。</p>`}</section><section class="settings-group"><h2>隐私与本地数据</h2><p>数据默认保存在本机。调用 AI 前会说明发送范围。</p><button class="secondary" disabled title="等待文件系统 capability">打开备份位置</button><button class="ghost" disabled title="等待数据库恢复命令">查看恢复选项</button></section><section class="settings-group" data-portable-data><h2>迁移数据包</h2><p>用于迁移业务数据和非敏感设置；AI 服务密钥不会包含在数据包内。</p><button class="secondary" data-export-portable>导出数据包</button><button class="ghost" data-import-portable>导入数据包</button></section>`; }
export function renderPage(route: RouteId, d: WorkspaceSnapshot, ui?: {
    selectedSkillId?: string;
    skillDetailOpen?: boolean;
    skillFilter?: {
        query?: string;
        category?: string;
    };
}): string { const v: Record<RouteId, () => string> = { home: () => home(d), experiences: () => experiences(d), personas: () => personas(d), resumes: () => resumes(d), jobs: () => jobs(d), skills: () => skills(d, ui?.selectedSkillId, ui?.skillDetailOpen, ui?.skillFilter), learning: () => learning(d), settings: () => settings(d) }; return `<article class="page" aria-labelledby="page-title">${v[route]()}</article>`; }
export const renderLoading = () => `<article class="page" aria-busy="true"><div class="skeleton title"></div><div class="skeleton line"></div><div class="skeleton block"></div><span class="sr-only">正在加载职业工作台</span></article>`;
export const renderError = (m: string) => `<article class="page"><section class="error-state" role="alert"><h1>暂时无法加载</h1><p>${esc(m)}</p><button class="primary" data-action="retry-load">重试</button></section></article>`;
export type ExperienceEntryState = {
    mode: "create" | "edit";
    editingId?: string;
    editingVersion?: number;
    phase: "input" | "generating" | "editablePreview" | "edit" | "committing" | "error";
    raw: string;
    error?: string;
    formError?: string;
    taskId?: string;
    proposal?: StructuredExperienceDraftDto;
    fieldErrors?: {
        structuredAchievements?: string;
        skillsDemonstrated?: string;
    };
};
export const renderExperienceDialog = (state: ExperienceEntryState = { mode: "create", phase: "input", raw: "" }) => { const editing = state.mode === "edit"; const p = state.proposal ?? { title: "", organization: "", type: "work" as const, startDate: null, endDate: null, rawDescription: state.raw, structuredAchievements: [], skillsDemonstrated: [], industryTags: [], educationLevel: "none" as const, status: "draft" as const }; const busy = state.phase === "generating" || state.phase === "committing"; const showFields = editing || state.phase === "editablePreview" || state.phase === "edit" || state.phase === "error" || state.phase === "input"; const title = editing ? "编辑经历" : state.phase === "editablePreview" ? "检查 AI 整理结果" : "添加经历"; return `<div class="modal-backdrop"><form class="onboarding form-dialog" data-experience-form data-mode="${state.mode}" data-phase="${state.phase}" role="dialog" aria-busy="${busy}" aria-modal="true" aria-labelledby="experience-dialog-title"><h1 id="experience-dialog-title">${title}</h1>${state.phase === "error" || state.formError ? `<p class="warning" role="alert">${esc(state.formError ?? state.error ?? "操作失败，请重试。")}</p>` : ""}<label>经历标题<input required name="title" value="${esc(p.title)}" ${state.phase === "generating" ? "disabled" : ""}></label><label>具体经历<textarea required name="original" rows="12" class="experience-body" ${state.phase === "editablePreview" && !editing ? "readonly" : ""} ${state.phase === "generating" ? "disabled" : ""} placeholder="可输入中文或 English，写下真实工作、行动和结果。">${esc(state.raw || p.rawDescription)}</textarea><small>原始描述始终以你的输入为准，AI 不会覆盖。</small></label>${state.phase === "generating" ? '<p role="status" aria-live="polite">正在整理经历，请稍候……</p>' : showFields ? `<label>组织或学校<input required name="organization" value="${esc(p.organization ?? "")}"></label><div class="form-grid"><label>类型<select name="kind"><option value="work" ${p.type === "work" ? "selected" : ""}>工作</option><option value="project" ${p.type === "project" ? "selected" : ""}>项目</option><option value="education" ${p.type === "education" ? "selected" : ""}>教育</option><option value="certification" ${p.type === "certification" ? "selected" : ""}>认证</option></select></label><label>开始日期<input required type="date" name="startDate" value="${esc(p.startDate ?? "")}"></label><label>结束日期<input type="date" name="endDate" value="${esc(p.endDate ?? "")}"><small>留空表示至今</small></label></div><label>结构化成果（每行一项）<textarea name="structuredAchievements" rows="4">${esc(p.structuredAchievements.join("\n"))}</textarea>${state.fieldErrors?.structuredAchievements ? `<small class="warning" role="alert">${esc(state.fieldErrors.structuredAchievements)}</small>` : ""}</label><label>技能（每行一项）<textarea name="skillsDemonstrated" rows="3">${esc(p.skillsDemonstrated.join("\n"))}</textarea>${state.fieldErrors?.skillsDemonstrated ? `<small class="warning" role="alert">${esc(state.fieldErrors.skillsDemonstrated)}</small>` : ""}</label><label>行业标签（每行一项）<textarea name="industryTags" rows="2">${esc(p.industryTags.join("\n"))}</textarea></label><label>学历层级<select name="educationLevel"><option value="none" ${p.educationLevel === "none" ? "selected" : ""}>未指定</option><option value="high_school" ${p.educationLevel === "high_school" ? "selected" : ""}>高中</option><option value="associate" ${p.educationLevel === "associate" ? "selected" : ""}>专科</option><option value="bachelor" ${p.educationLevel === "bachelor" ? "selected" : ""}>本科</option><option value="master" ${p.educationLevel === "master" ? "selected" : ""}>硕士</option><option value="doctorate" ${p.educationLevel === "doctorate" ? "selected" : ""}>博士</option><option value="other" ${p.educationLevel === "other" ? "selected" : ""}>其他</option></select></label>` : ""}<div class="dialog-actions"><button type="button" class="secondary" data-close-dialog>取消</button>${editing ? `<button class="primary" data-save-experience>保存</button>` : state.phase === "generating" ? '<button type="button" class="ghost" data-cancel-structure>取消整理</button>' : state.phase === "editablePreview" ? '<button class="primary" data-save-structured>保存为草稿</button>' : `<button type="button" class="secondary" data-structure-experience>AI 整理</button><button class="primary" data-manual-experience>直接手填并预览</button>`}</div></form></div>`; };
export const renderExperienceConfirmation = (x: Experience) => `<div class="modal-backdrop"><form class="onboarding form-dialog" role="dialog" aria-modal="true" data-draft-confirm><h1>预览并确认经历</h1><label>标题<input required name="title" value="${esc(x.title)}"></label><label>组织<input required name="organization" value="${esc(x.organization)}"></label><p>${esc(x.kind)} · ${esc(x.period)}</p>${x.warnings?.some(w => w.code === "DATE_OVERLAP") ? '<p class="warning" role="alert">日期与已有经历重叠，请确认时间是否属实；你仍可继续确认。</p>' : ''}<label>原始描述<textarea required name="original" rows="6">${esc(x.original)}</textarea></label><p class="helper">确认后才会进入简历与匹配；这里仍可编辑，原始事实不会被 AI 覆盖。</p><div class="dialog-actions"><button type="button" class="ghost danger-text" data-discard-draft>丢弃草稿</button><button class="primary">确认经历</button></div></form></div>`;
export const renderPersonaDialog = (p?: {
    id: string;
    name: string;
    targetRole: string;
    positioning: string;
}) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-persona-form data-id="${p?.id ?? ""}" role="dialog" aria-modal="true"><h1>${p ? "编辑" : "创建"}角色档案</h1><label>名称<input required name="name" value="${esc(p?.name ?? "")}"></label><label>目标岗位<input required name="targetRole" value="${esc(p?.targetRole ?? "")}"></label><label>定位陈述<textarea name="positioning" rows="4">${esc(p?.positioning ?? "")}</textarea></label><div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>取消</button><button class="primary">保存角色</button></div></form></div>`;
export const renderFitDialog = (personaId: string, items: {
    id: string;
    title: string;
    score?: number;
    overridden?: boolean;
}[], note?: string) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-fit-form data-persona-id="${esc(personaId)}" role="dialog" aria-modal="true"><h1>确认经历权重</h1><p class="helper">${esc(note ?? "权重越高，生成简历时该经历越靠前、要点越详细；权重为 0 的工作/项目不会进入简历。保存后立即生效。")}</p>${items.map(x => `<label>${esc(x.title)}${x.overridden ? '<small>曾手动调整</small>' : ''}<input name="${esc(x.id)}" type="number" min="0" max="100" value="${x.score ?? 50}"></label>`).join("")}<div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>稍后调整</button><button class="primary">保存权重</button></div></form></div>`;
export const renderProviderDialog = (p?: {
    id: string;
    name: string;
    model: string;
    baseUrl?: string;
    enabled: boolean;
}, error?: string) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-provider-form data-editing="${p ? "1" : "0"}" role="dialog" aria-modal="true"><h1>${p ? "编辑" : "添加"}服务商</h1>${error ? `<p class="warning" role="alert">${esc(error)}</p>` : ""}<label>名称<input required name="name" value="${esc(p?.name ?? "")}" ${p ? "readonly" : ""} placeholder="例如 tongyi 或 openai"><small>本机调试 HTTP 仅当名称为 local 时可用</small></label><label>API 地址<input required name="baseUrl" type="url" value="${esc(p?.baseUrl ?? "")}" placeholder="https://dashscope.aliyuncs.com/compatible-mode/v1"><small>须为可解析的 HTTPS 公网地址</small></label><label>默认模型<input required name="model" value="${esc(p?.model ?? "")}" placeholder="qwen-max"></label><label>API Key<input name="apiKey" type="password" autocomplete="new-password" ${p ? "" : "required"} placeholder="${p ? "留空表示不修改" : "必填"}"></label><label><input name="enabled" type="checkbox" ${p?.enabled !== false ? "checked" : ""}> 启用</label><div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>取消</button><button class="primary">安全保存</button></div></form></div>`;
export const renderJobDialog = (personas: {
    id: string;
    name: string;
}[]) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-job-form role="dialog" aria-modal="true"><h1>分析岗位</h1><label>目标角色<select required name="personaId"><option value="">请选择角色</option>${personas.map(x => `<option value="${esc(x.id)}">${esc(x.name)}</option>`).join("")}</select></label><label>岗位描述<textarea required name="jdText" rows="10" placeholder="粘贴完整中文或英文 JD"></textarea></label><div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>取消</button><button class="primary">解析并匹配</button></div></form></div>`;
export const renderSkillDialog = (s?: {
    id: string;
    name: string;
    level: string;
    category?: string;
}) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-skill-form data-id="${s?.id ?? ""}" role="dialog" aria-modal="true"><h1>${s ? "编辑" : "添加"}自定义技能</h1><label>名称<input required name="name" value="${esc(s?.name ?? "")}"></label><label>分类<select required name="category">${[["product_management", "产品"], ["technical", "技术"], ["management", "管理"], ["industry", "行业"]].map(([value, label]) => `<option value="${value}" ${s?.category === value ? "selected" : ""}>${label}</option>`).join("")}</select></label><label>等级<select name="level"><option value="1" ${s?.level === "1" ? "selected" : ""}>基础</option><option value="2" ${s?.level === "2" ? "selected" : ""}>进阶</option><option value="3" ${s?.level === "3" ? "selected" : ""}>高阶</option></select></label><div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>取消</button><button class="primary">保存技能</button></div></form></div>`;
export const renderWhatIfDialog = (seed?: {
    required?: string;
    current?: string;
    hypothetical?: string;
}) => `<div class="modal-backdrop"><form class="onboarding form-dialog" data-what-if-form role="dialog" aria-modal="true"><h1>假设技能分析</h1><p class="helper">临时试算，不写入真实档案。</p><label>岗位要求技能<input required name="required" value="${esc(seed?.required ?? "")}" placeholder="Rust, SQL"></label><label>当前技能<input name="current" value="${esc(seed?.current ?? "")}" placeholder="Rust"></label><label>假设新增技能<input required name="hypothetical" value="${esc(seed?.hypothetical ?? "")}" placeholder="SQL"></label><div class="dialog-actions"><button type="button" class="secondary" data-close-overlay>取消</button><button class="primary">计算提升</button></div></form></div>`;
