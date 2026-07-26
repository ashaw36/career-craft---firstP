# Sprint 11 计划 — P0/P1 补漏：PDF导出 + Fit Score覆盖 + 经历重述编辑

> **锁定日期:** 2026-07-19  
> **决策:** D6-B — 补齐 P0/P1 前端缺口，不做 P2 差异化  
> **代码量预估:** ~300 行（Python + HTML/JS）  
> **测试预估:** 新增 6-10 个测试  
> **状态: ✅ 已完成** | 完成日期: 2026-07-19 | 提交: `f867e11` | 测试: 149/149 passed

---

## 一背景

Sprint 9-10 完成后，P0/P1 后端功能均已实现，但前端存在 **3 个关键缺口**：

1. **PDF 导出是占位** — `api_handler.export_resume_pdf()` 返回 Markdown，未调用 `pdf_exporter.py`
2. **Fit Score 无前端覆盖入口** — `user_overridden` 字段存在但无滑块/输入框
3. **经历重述无编辑/重置入口** — `jd_reframe_engine.py` 完整但前端无面板

本 Sprint 目标是将这 3 个"半成品"变为"成品"。

---

## 二任务分解

### Story 11.1 — PDF 真实导出 ✅

**State:** 后端 `pdf_exporter.py` 存在且完整，`api_handler.py` 为占位  
**Gap:** Bridge 未接入 PDF 生成逻辑，前端下载的是 Markdown 伪装文件

**任务:**

| # | 任务 | 文件 | 状态 |
|---|------|------|------|
| 11.1.1 | 更新 `api_handler.export_resume_pdf()` | `src/ui/webview/api_handler.py` | ✅ 调用 `PDFExporter.export_resume()` 生成 PDF bytes，返回 base64 编码 |
| 11.1.2 | 更新 `bridge.exportResumePDF()` | `src/ui/webview/bridge.py` | ✅ 传递 base64 给 JS，保持接口不变 |
| 11.1.3 | 更新前端 `exportResumePDF()` | `prototype/ui-prototype.html` | ✅ base64→Blob，触发真实 PDF 下载 |
| 11.1.4 | 补充测试 | `tests/test_pdf_exporter.py` / `test_bridge.py` | ✅ 通过 |

**验收标准:**
- [x] 点击"导出 PDF"按钮下载的是 `.pdf` 文件，可用浏览器打开
- [x] PDF 中包含正确的中文内容（`fpdf2` 已处理中文字体）
- [x] 无 `fpdf2` 时下降为 Markdown 下载，并提示安装命令

---

### Story 11.2 — Fit Score 手动覆盖 ✅

**State:** `user_overridden` 字段存在，`persona_engine.py` 计算时检查该字段  
**Gap:** 前端无调整 relevance_score 的入口

**任务:**

| # | 任务 | 文件 | 状态 |
|---|------|------|------|
| 11.2.1 | 新增 Bridge API `updateFitScore` | `src/ui/webview/bridge.py` + `api_handler.py` | ✅ 更新 `role_experience_weights` 并设置 `user_overridden=True` |
| 11.2.2 | 前端增加 Fit Score 滑块 | `prototype/ui-prototype.html` | ✅ 角色页展示 Fit Score，支持 0-100 滑块调整 |
| 11.2.3 | 显示覆盖状态 | `prototype/ui-prototype.html` | ✅ "已手动调整"标记 |
| 11.2.4 | 补充测试 | `tests/test_persona_engine.py` | ✅ 通过 |

**验收标准:**
- [x] 用户可在前端看到每条经历的 Fit Score
- [x] 拖动滑块后保存，下次生成简历时按新分数排序
- [x] "重置"后恢复自动计算值

---

### Story 11.3 — 经历重述编辑/重置 ✅

**State:** `jd_reframe_engine.py` 完整，`job_match_experience_reframes` 表存在  
**Gap:** 前端无编辑/重置入口，只能查看自动生成的重述

**任务:**

| # | 任务 | 文件 | 状态 |
|---|------|------|------|
| 11.3.1 | 新增 Bridge API `updateReframe` / `resetReframe` | `src/ui/webview/bridge.py` + `api_handler.py` | ✅ 更新重述文本 / 删除重述记录恢复原始 |
| 11.3.2 | 前端增加重述编辑面板 | `prototype/ui-prototype.html` | ✅ 岗位匹配详情页展示 `reframed_summary` + `reframing_strategy`，支持编辑 |
| 11.3.3 | 前端增加重置功能 | `prototype/ui-prototype.html` | ✅ "重置为自动生成"按钮 |
| 11.3.4 | 补充测试 | `tests/test_jd_reframe_engine.py` | ✅ 通过 |

**验收标准:**
- [x] 在岗位匹配详情页可看到每条经历的自动重述和策略说明
- [x] 点击编辑后可修改 `reframed_summary`，保存后生效
- [x] 点击重置后删除重述记录，下次生成时重新走 LLM

---

## 三时间线（实际）

| 天 | 任务 | 产出 |
|-----|------|--------|
| D1 | Story 11.1: PDF 导出 | Codex 执行 → 代码审查 → 23测试通过 |
| D2 | Story 11.2: Fit Score 覆盖 | Codex 执行 → 代码审查 → 全量149通过 |
| D3 | Story 11.3: 重述编辑/重置 | Codex 执行 → 代码审查 → 全量149通过 |
| D4 | BMAD 同步 + git commit push | PRD/INDEX/Sprint Plan 更新 → `f867e11` |

---

## 四风险（事后复盘）

| 风险 | 级别 | 实际结果 |
|------|------|----------|
| `fpdf2` 字体缺失导致乱码 | 中 | 未触发，已打包中文字体 |
| Fit Score 前后端数据不一致 | 低 | 未触发，API 返回完整数据 |
| 重述编辑后 LLM 缓存不刷新 | 低 | 未触发，重置时删除记录自然走 LLM |

---

## 五代码统计

| 指标 | 数值 |
|------|------|
| 修改文件数 | 6 |
| 新增代码行 | 387 |
| 删除代码行 | 27 |
| 净增代码行 | +360 |
| 测试通过率 | 149/149 (100%) |
| 回归测试耗时 | ~16s |

---

*Sprint 11 计划 — 锁定日期: 2026-07-19 — ✅ 已完成*
