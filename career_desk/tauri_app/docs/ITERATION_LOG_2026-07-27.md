# CareerCraft 迭代记录 — 2026-07-27

批次：`UX-EXP-SETTINGS-001`  
范围：`tauri_app`  
方法：BMAD（需求 → 根因 → 方案确认 → 实现 → 准出记录）

## 决策确认

| 项 | 决策 |
|---|---|
| 修改记录 | 方案 A：下线产品面（UI + 前端 API + 2 个 command），保留 DB 静默 `append_experience_revision` 与 migration |
| 按钮间距 | `2ch`（由原 4ch 收紧） |
| 编辑弹窗 | 复用可滚动完整字段「添加/编辑经历」对话框；删除旧内联编辑页 |
| 草稿行 | 保留确认 / 丢弃 |
| 服务商 API Key | 新建必填；保留保存时 DNS/HTTPS 校验 |
| UI-4 | 暂不补充 |

## 实现摘要

1. **经历列表**：去掉「修改记录」；标题行右侧 `编辑`/`删除`（草稿保留确认/丢弃），`gap: 2ch`。
2. **搜索/类型筛选**：`data-experience-search` + `data-experience-kind` +「筛选」按钮；修复 `.content-row{display:flex}` 覆盖 `[hidden]` 导致筛选无效。
3. **经历弹窗**：统一 `form-dialog`（可滚动），字段完整；编辑走 `updateExperience`，并扩展后端 `ExperiencePatch` 支持用户改写原文/技能等（AI enrichment 仍不经此路径改原文）。
4. **服务商**：`load()` 回传 `baseUrl`；新建 Key 必填；弹窗内联中文错误；测试连接改用非流式 probe，并补充中文失败提示。
5. **契约**：command 数 **63 → 61**（移除 `get_experience_revisions` / `restore_experience_revision`）。

## 热修（同日用户复测）

| Bug | 根因 | 修复 |
|---|---|---|
| 点「编辑」任务中心报「删除经历失败」 | `app.ts` 把 `[data-edit-experience]` 误绑到 `deleteExperience` | 删除选择器仅保留 `[data-delete-experience]` |
| 搜索无效 | author CSS `display:flex` 压过 UA `[hidden]` | `[hidden]` / `.is-filtered-out { display:none !important }`；增加「筛选」按钮 |
| 编辑/删除间距 | 用户要求收紧 | `gap: 1ch` |
| 测试连接失败 | probe 走 stream + max_tokens=1，常空文本误判失败；错误英文难懂 | 非流式 probe + 中文错误映射 |
| AI 整理报「不符合 v3 契约」 | 后端序列化 `experienceType`，前端契约要 `type`；且空数组被前后端硬拒 | `#[serde(rename="type")]`；允许空列表；前端容错规范化 |
## 简历页改版（同日）

决策：版本管理用弹窗；主界面只展示纸面；生成可无预览（B1）；产品面取消导出 PDF，仅保留 Markdown；对话调优弹窗重做布局。

实现：`pages.ts` 简历页、`resume.ts` / `resume-extra.ts`、相关 CSS；前端移除 `exportResumePdf` 调用链。后端 `export_resume_pdf` command 暂保留（契约 61），产品入口已下线。

## 简历页收口（同日续）

1. 切换角色自动载入该角色最新纸面（无版本则实时预览）。
2. 对话调优改为按经历改写成就要点（`resume-tuning-v2` JSON），确认后写入新版本。
3. 去掉模板切换 UI；去掉底部撤销/重做调优（版本管理保留）。
4. 角色列表去掉 Fit Score / 重置自动评分展示；「调整经历权重」保留。AI 按定位陈述推荐权重待确认后再做。

## 调优超时 / 版本精简 / 角色 AI 权重（同日）

1. LLM HTTP 超时 30s → **180s**；简历调优 `max_tokens` → **4096**；前端区分 TIMEOUT / JSON 失败提示。
2. 版本管理去掉「比较最近两版」「恢复所选」，仅点选载入预览。
3. 保存角色后触发 `recommend_persona_weights`，弹窗确认经历权重后再写入。

## 岗位匹配页改版（同日）

拍板：A1 左右分栏 / B1 摘要分数+弹窗分项 / C1 补强链学习 / D1 移除 stub、详情试算 what-if。

实现：`pages.ts` jobs 纸面布局、`jobs.ts` 选中/证据/学习跳转/试算、`data-source` 保留 `jobDescId`+`rawText`+`matchId`；去掉列表堆叠与 disabled 假设分析。

## 热修：生成 / 调优纸面 / 导出 MD（同日）

| Bug | 根因 | 修复 |
|---|---|---|
| 生成并保存无反应 | `allocate_achievements_by_weight` 在要点数=1 时 `clamp(2,1)` panic，后台线程崩 | 改为 `target.min(n)`，单要点不再崩溃 |
| 导出 Markdown 失败 | `write_text_file` 误用仅允许 `.zip/.ccbackup` 的 `portable_path` | 新增 `export_text_path` 允许 `.md/.markdown/.txt` |
| 调优确认后纸面不更新 | 仅 `reload`+任务上报重绘，返回的 markdown 未直接刷纸面；任务 `change` 整页重绘易冲掉 | 确认/生成后用返回 markdown `paintResumePaper`；`load()` 已 ready 时不再闪 loading |
| 权重详细度 | 生成路径已按权重裁剪要点（此前 panic 导致根本进不去） | 保留并加固；状态文案提示「按经历权重」 |


## 准出清单

- [ ] 经历列表无「修改记录」；标题行编辑/删除间距约 2ch
- [ ] 编辑弹窗字段完整、可滚动，保存后内容生效（含具体经历）；点编辑不再误报删除失败
- [ ] 添加经历长表单可滚到底并提交
- [ ] 搜索 + 类型筛选 +「筛选」按钮组合正确
- [ ] 新建服务商：无 Key 被拦；合法 HTTPS 可保存；错误中文可读；测试连接可读中文失败原因
- [ ] 前端测试 / 契约门禁 command=61
- [ ] Rust 相关测试（含 ExperiencePatch 扩展、probe）通过

## 需求追踪变更

- `CC-FR-001` 中「最近三次 revision / UI 恢复」→ **产品下线（方案 A）**；静默审计仍写入。
- 新增本批次 UX 项见 `REQUIREMENTS_TRACEABILITY.md`。
