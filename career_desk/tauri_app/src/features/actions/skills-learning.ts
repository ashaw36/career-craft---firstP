import type { LearningExperienceDraft, SkillLearningActions, WorkspaceDataSource } from "../../api/data-source";
import type { TaskStore } from "../../shared/state/tasks";
import { renderSkillDialog } from "../pages";
type Source = WorkspaceDataSource & SkillLearningActions;
type SkillUiCallbacks = { onSelectSkill?: (id: string, trigger?: HTMLElement) => void; onCloseSkill?: () => void; onGenerateLearning?: (skill: string) => void; onOpenJobWhatIf?: (skill: string) => void; onSkillFilterChange?: (filter: { query: string; category: string }) => void };
const report = (tasks: TaskStore, label: string, phase: "completed" | "failed", message: string) => tasks.upsert({ id: crypto.randomUUID(), label, phase, message, ...(phase === "completed" ? { progress: 100 } : { retryable: true }) });
export function bind(root: HTMLElement, source: Source, tasks: TaskStore, reload: () => Promise<void>, ui: SkillUiCallbacks = {}) {
    const toast = (message: string) => { root.querySelector("[data-toast]")?.remove(); const node = document.createElement("div"); node.dataset.toast = "true"; node.setAttribute("role", "status"); node.setAttribute("aria-live", "polite"); node.className = "status-label"; node.textContent = message; root.append(node); setTimeout(() => node.remove(), 2500); };
    const close = () => root.querySelector(".modal-backdrop:last-child")?.remove();
    const wire = () => {
        root.querySelector<HTMLElement>("[data-close-overlay]")?.addEventListener("click", close);
        root.querySelector<HTMLFormElement>("[data-skill-form]")?.addEventListener("submit", async (event) => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const submit = form.querySelector<HTMLButtonElement>('button[type="submit"],button.primary'); if (submit?.disabled) return; const originalLabel = submit?.textContent ?? ""; if (submit) { submit.disabled = true; submit.textContent = "正在分析学习资源…"; } form.setAttribute("aria-busy", "true"); const v = new FormData(form); try {
            const saved = await source.saveCustomSkill({ id: form.dataset.id || undefined, name: String(v.get("name")), category: String(v.get("category")), level: Number(v.get("level")) });
            close();
            await reload();
            if (saved?.resourceWarning)
                report(tasks, "AI 学习资源分析", "failed", `技能已保存，但资源暂未挂载：${saved.resourceWarning}`);
            else if (saved?.resourceCount)
                report(tasks, "AI 学习资源分析", "completed", `已自动分析并挂载 ${saved.resourceCount} 条学习资源。`);
        }
        catch (error) {
            report(tasks, "保存技能", "failed", (error as Error).message);
            form.removeAttribute("aria-busy");
            if (submit) { submit.disabled = false; submit.textContent = originalLabel; }
        } });
        root.querySelector<HTMLFormElement>("[data-what-if-form]")?.addEventListener("submit", async (event) => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const v = new FormData(form); const list = (name: string) => String(v.get(name)).split(/[,，]/).map(x => x.trim()).filter(Boolean); try {
            const result = await source.simulateWhatIf(list("required"), list("current"), list("hypothetical"));
            showWhatIfResult(form, result as Record<string, unknown>);
        }
        catch (error) {
            report(tasks, "假设分析", "failed", (error as Error).message);
        } });
        const whatIf = root.querySelector<HTMLFormElement>("[data-what-if-form]");
        if (whatIf) {
            const output = document.createElement("output");
            output.dataset.whatIfResult = "true";
            output.setAttribute("aria-live", "polite");
            whatIf.querySelector(".dialog-actions")?.before(output);
            let timer = 0, request = 0;
            whatIf.addEventListener("input", () => { clearTimeout(timer); const current = ++request; timer = window.setTimeout(async () => { const v = new FormData(whatIf); const list = (name: string) => String(v.get(name)).split(/[,，]/).map(x => x.trim()).filter(Boolean); try {
                const result = await source.simulateWhatIf(list("required"), list("current"), list("hypothetical")) as Record<string, unknown>;
                if (current === request)
                    showWhatIfResult(whatIf, result, output);
            }
            catch (error) {
                if (current === request)
                    output.textContent = `暂时无法计算：${(error as Error).message}`;
            } }, 150); });
        }
    };
    const showWhatIfResult = (form: HTMLFormElement, result: Record<string, unknown>, target?: HTMLOutputElement) => {
        const output = target ?? form.querySelector<HTMLOutputElement>("[data-what-if-result]");
        if (!output) return;
        const baseline = Number(result.baselineScore ?? 0);
        const simulated = Number(result.simulatedScore ?? baseline);
        const delta = Number(result.delta ?? simulated - baseline);
        const remaining = Array.isArray(result.remainingMissing) ? result.remainingMissing.map(String) : [];
        output.className = "what-if-result";
        output.textContent = `当前匹配 ${baseline} 分，假设后 ${simulated} 分（${delta >= 0 ? "+" : ""}${delta}）${remaining.length ? `；仍缺少：${remaining.join("、")}` : "；岗位要求已覆盖"}`;
    };
    const mount = (html: string) => { root.insertAdjacentHTML("beforeend", html); wire(); };
    root.querySelector<HTMLElement>('[data-action="add-skill"]')?.addEventListener("click", () => mount(renderSkillDialog()));
    root.querySelector<HTMLElement>("[data-what-if]")?.addEventListener("click", () => {
        const selectedSkill = root.querySelector<HTMLElement>("[data-generate-skill-path]")?.dataset.generateSkillPath ?? "";
        if (ui.onOpenJobWhatIf) ui.onOpenJobWhatIf(selectedSkill);
        else report(tasks, "假设分析", "failed", "假设分析需要目标岗位和角色档案，请到“岗位匹配”选择岗位后点击“试算技能影响”。");
    });
    const filterSkills = () => {
        const query = root.querySelector<HTMLInputElement>("[data-skill-search]")?.value.trim().toLowerCase() ?? "";
        const category = root.querySelector<HTMLSelectElement>("[data-skill-category]")?.value ?? "";
        let visible = 0;
        root.querySelectorAll<HTMLElement>("[data-skill-row]").forEach(row => { const show = (!query || (row.dataset.searchText ?? "").includes(query)) && (!category || row.dataset.category === category); row.hidden = !show; if (show) visible += 1; });
        const count = root.querySelector<HTMLElement>("[data-skill-count]"); if (count) count.textContent = `${visible} 项`;
        const empty = root.querySelector<HTMLElement>("[data-skill-empty]"); if (empty) empty.hidden = visible !== 0;
        ui.onSkillFilterChange?.({ query, category });
    };
    root.querySelector<HTMLInputElement>("[data-skill-search]")?.addEventListener("input", event => { event.stopImmediatePropagation(); filterSkills(); });
    root.querySelectorAll<HTMLElement>("[data-edit-skill]").forEach(node => node.addEventListener("click", async () => { const skill = (await source.load()).skills.find(x => x.id === node.dataset.editSkill); if (skill)
        mount(renderSkillDialog(skill)); }));
    root.querySelectorAll<HTMLElement>("[data-delete-skill]").forEach(node => node.addEventListener("click", async () => { try {
        await source.deleteCustomSkill(node.dataset.deleteSkill ?? "");
        await reload();
    }
    catch (error) {
        report(tasks, "删除技能", "failed", (error as Error).message);
    } }));
    root.querySelectorAll<HTMLElement>("[data-learning-jump]").forEach(node => node.addEventListener("click", () => {
        const pathId = node.dataset.learningJump ?? "";
        const target = [...root.querySelectorAll<HTMLElement>("[data-learning-path]")].find(path => path.dataset.learningPath === pathId);
        if (!target) return;
        target.tabIndex = -1;
        target.scrollIntoView({ behavior: "smooth", block: "start" });
        target.focus({ preventScroll: true });
    }));
    root.querySelectorAll<HTMLElement>("[data-delete-learning-path]").forEach(node => node.addEventListener("click", async () => {
        if (!confirm("确认删除这条学习路径？路径步骤会被删除，已经整理出的经历仍会保留。")) return;
        node.setAttribute("aria-busy", "true");
        try {
            await source.deleteLearningPath(node.dataset.deleteLearningPath ?? "", Number(node.dataset.version ?? 1));
            report(tasks, "删除学习路径", "completed", "学习路径已删除，已生成的经历不受影响。");
            await reload();
        } catch (error) {
            node.removeAttribute("aria-busy");
            report(tasks, "删除学习路径", "failed", (error as Error).message);
        }
    }));
    root.querySelectorAll<HTMLSelectElement>("[data-learning-status]").forEach(node => node.addEventListener("change", async () => { const id = node.dataset.learningStatus ?? "", item = (await source.load()).learning.find(x => x.id === id), completionNote = node.value === "completed" ? prompt("请填写学习完成说明（必填）")?.trim() : undefined; if (node.value === "completed" && !completionNote) {
        report(tasks, "更新学习进度", "failed", "完成学习时必须填写完成说明。");
        return;
    } try {
        await source.updateLearning(id, node.value, item?.version ?? 1, completionNote);
        await reload();
    }
    catch (error) {
        report(tasks, "更新学习进度", "failed", (error as Error).message);
    } }));
    root.querySelectorAll<HTMLElement>("[data-complete-learning]").forEach(node => node.addEventListener("click", () => { const row = node.closest<HTMLElement>("[data-learning-row]"); const title = row?.querySelector("strong")?.textContent?.trim() || "学习成果"; root.insertAdjacentHTML("beforeend", `<div class="modal-backdrop" data-learning-draft><form class="onboarding"><h1>编辑经历草稿</h1><p>本步骤只生成待确认草稿，不会进入正式经历。</p><label>标题<input name="title" required value="${title.replace(/&/g, "&amp;").replace(/"/g, "&quot;")}"></label><label>组织/来源<input name="organization" required value="学习计划"></label><label>经历说明<textarea name="rawDescription" required>完成 ${title.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</textarea></label><div class="dialog-actions"><button type="button" class="ghost" data-cancel-learning-draft>取消</button><button class="primary">生成待确认草稿</button></div></form></div>`); const modal = root.querySelector<HTMLElement>("[data-learning-draft]")!; modal.querySelector("[data-cancel-learning-draft]")?.addEventListener("click", () => modal.remove()); modal.querySelector("form")?.addEventListener("submit", async (event) => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement, v = new FormData(form), submit = form.querySelector<HTMLButtonElement>('button[type="submit"],button.primary'); if (submit)
        submit.disabled = true; try {
        const draft = await source.completeLearning(node.dataset.completeLearning ?? "", { title: String(v.get("title")), organization: String(v.get("organization")), rawDescription: String(v.get("rawDescription")) });
        showPendingDraft(modal, draft);
    }
    catch (error) {
        if (submit)
            submit.disabled = false;
        report(tasks, "生成经历草稿", "failed", (error as Error).message);
    } }); }));
    function showPendingDraft(modal: HTMLElement, draft: LearningExperienceDraft) { const sourceTitle = (draft.sourceSnapshot?.title ?? draft.sourceLearningTitle).replace(/&/g, "&amp;").replace(/</g, "&lt;"); modal.innerHTML = `<section class="onboarding" role="dialog" aria-modal="true"><p class="eyebrow">待确认</p><h1>${draft.title.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</h1><p>${draft.rawDescription.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</p><p class="helper">来源快照：${sourceTitle}。原学习路径删除后仍保留审计快照。</p><p class="helper">当前仍是草稿，确认后才进入正式经历；放弃会标记为 discarded，不用于简历或岗位匹配。</p><div class="dialog-actions"><button class="ghost" data-discard-learning-experience>放弃草稿</button><button class="primary" data-confirm-learning-experience>确认加入经历</button></div></section>`; const resolve = async (status: "confirmed" | "discarded") => { modal.querySelectorAll<HTMLButtonElement>("button").forEach(x => x.disabled = true); try {
        await source.resolveLearningExperience(draft, status);
        report(tasks, status === "confirmed" ? "确认经历" : "放弃经历草稿", "completed", status === "confirmed" ? "经历已确认并加入正式经历库。" : "草稿已放弃，来源快照保留用于审计。");
        modal.remove();
        await reload();
    }
    catch (error) {
        modal.querySelectorAll<HTMLButtonElement>("button").forEach(x => x.disabled = false);
        report(tasks, "处理经历草稿", "failed", (error as Error).message);
    } }; modal.querySelector("[data-confirm-learning-experience]")?.addEventListener("click", () => void resolve("confirmed")); modal.querySelector("[data-discard-learning-experience]")?.addEventListener("click", () => void resolve("discarded")); }
    root.querySelectorAll<HTMLElement>("[data-skill-detail]").forEach(node => node.addEventListener("click", () => ui.onSelectSkill?.(node.dataset.skillDetail ?? "", node)));
    root.querySelector<HTMLElement>("[data-close-skill-detail]")?.addEventListener("click", () => ui.onCloseSkill?.());
    root.querySelector<HTMLElement>("[data-generate-skill-path]")?.addEventListener("click", event => ui.onGenerateLearning?.((event.currentTarget as HTMLElement).dataset.generateSkillPath ?? ""));
    const skillDialog = root.querySelector<HTMLElement>("[data-skill-dialog].is-open");
    if (skillDialog) {
        skillDialog.addEventListener("click", event => { if (event.target === skillDialog) ui.onCloseSkill?.(); });
        skillDialog.addEventListener("keydown", event => { if (event.key === "Escape") { event.preventDefault(); ui.onCloseSkill?.(); } });
    }
    root.querySelector<HTMLSelectElement>("[data-skill-category]")?.addEventListener("change", event => { event.stopImmediatePropagation(); filterSkills(); });
    filterSkills();
    root.querySelector<HTMLSelectElement>("[data-learning-source]")?.addEventListener("change", event => root.querySelectorAll<HTMLElement>("[data-learning-row]").forEach(row => row.hidden = Boolean((event.currentTarget as HTMLSelectElement).value && row.dataset.source !== (event.currentTarget as HTMLSelectElement).value)));
    root.querySelector<HTMLSelectElement>("[data-job-filter]")?.addEventListener("change", event => root.querySelectorAll<HTMLElement>("[data-job-row]").forEach(row => row.hidden = Boolean((event.currentTarget as HTMLSelectElement).value && row.dataset.status !== (event.currentTarget as HTMLSelectElement).value)));
    root.querySelectorAll<HTMLElement>("[data-copy-resource]").forEach(copy => copy.addEventListener("click", event => { event.stopPropagation(); void (async () => { try { await navigator.clipboard.writeText(copy.dataset.copyResource ?? ""); toast("链接已复制。"); } catch (error) { report(tasks, "复制资源链接", "failed", (error as Error).message); } })(); }));
    root.querySelectorAll<HTMLElement>("[data-open-resource]").forEach(open => open.addEventListener("click", event => { event.stopPropagation(); void (async () => { try { await source.openResource(open.dataset.openResource ?? ""); toast("已安全打开资源。"); } catch (error) { report(tasks, "打开资源", "failed", (error as Error).message); } })(); }));
}
