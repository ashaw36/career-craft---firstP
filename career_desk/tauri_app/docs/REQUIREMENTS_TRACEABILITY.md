# CareerCraft 需求—实现—测试—证据追踪矩阵

更新：2026-07-27。状态只表示当前证据强度：`已证实`、`部分`、`已取代`、`已下线`。Rust/Vitest 为 Windows 本地自动化；`W6` 表示真实 Tauri/WebView2 E2E，当前未通过。

## 功能需求

| ID | 实现位置 | 实际测试名/文件 | 最近证据 | 状态 |
|---|---|---|---|---|
| CC-FR-001 | `application/experience_structuring.rs`、`repositories/sqlite.rs` | `fenced_chinese_golden_*`；`revision_history_is_append_only_*`（静默审计）；`w1-experience-structure.test.ts` | 2026-07-27：UI/restore command 下线（方案 A）；用户编辑可改写原文；AI enrichment 仍不改原文 | 已修订 |
| CC-UX-EXP-001 | `features/pages.ts`、`app.css` | 经历标题行操作区 `4ch`；无「修改记录」入口 | ITERATION_LOG_2026-07-27 | 进行中 |
| CC-UX-EXP-002 | `pages.ts` `renderExperienceDialog`、`app.ts` | 统一可滚动完整字段添加/编辑弹窗 | ITERATION_LOG_2026-07-27 | 进行中 |
| CC-UX-EXP-003 | `actions/experience.ts` | 搜索 + 类型本地筛选 | ITERATION_LOG_2026-07-27 | 进行中 |
| CC-UX-SET-001 | `data-source.ts`、`app.ts` `saveProvider` | baseUrl 回传；新建 Key 必填；中文错误 | ITERATION_LOG_2026-07-27 | 进行中 |
| CC-FR-001-HIST | 原 `get/restore_experience_revision` | 已从 command 契约移除（61 commands） | 方案 A | 已下线 |
| CC-FR-002 | `application/experiences.rs`、SQLite repository | `crud_validation_conflict_and_not_found`；`lifecycle_and_overlap_warnings_are_explicit`；`experience_command_closes_lifecycle_and_returns_overlap_warning` | Rust 169 | 已证实 |
| CC-FR-003 | `infra/documents/import.rs` | `accepts_txt_markdown_and_json`；`extracts_deflated_docx_and_rejects_bad_stream`；`extracts_pdf_literals_and_rejects_bad_header` | Rust 169；无人工视觉报告 | 已证实/系统签核部分 |
| CC-FR-004 | `application/personas.rs`、`repositories/sqlite.rs` | `create_update_validation_conflict_not_found`；`deleting_persona_does_not_delete_experience`；`p0-workflows.test.ts` | Rust 169；frontend 113 | 已证实 |
| CC-FR-005 | `domain/fit_score.rs`、`application/jobs.rs`、`application/resumes.rs` | `candidate_uses_skill_industry_education_and_merged_year_evidence`；`override_and_reset_are_persisted`；`fit_selection_honors_override_score_limit_and_stable_ties` | Rust 169 | 已证实 |
| CC-FR-006 | `domain/resume.rs`、`resume_commands.rs`、documents | `exposes_exactly_five_stable_templates`；`w3-resume-preview.test.ts`；`resume-extra.test.ts` | Rust 169；frontend 113；W6 未过 | 已证实/系统签核部分 |
| CC-FR-007 | `application/resumes.rs`、`resume_commands.rs` | `confirmed_tune_creates_child_and_preserves_base`；proposal/hash/undo tests；`w4-ai-gates.test.ts` | Rust 169；frontend 113 | 已证实 |
| CC-FR-008 | `llm_orchestration.rs`、`infra/llm`、`llm_cache.rs` | `retries_then_falls_back`；`persona_preference_is_first_without_duplicate`；`real_transport_timeout_invalid_sse_and_truncated_stream_are_explicit`；cache tests | Rust 169 | 已证实；真实公网非门禁证据 |
| CC-FR-009 | `application/jobs.rs`、commands | `local_parser_is_available_without_network`；`parser_exposes_conservative_job_terms`；`w5-contract-ui.test.ts` | Rust 169；frontend 113 | 已证实 |
| CC-FR-010 | `application/jobs.rs`、`domain/jobs.rs` | candidate evidence/breakdown tests；`get_job_matches_has_named_shape_evidence_and_stable_updated_order` | Rust 169 | 已证实 |
| CC-FR-011 | commands、migration 7 | `migration7_backfills_once_survives_restart_and_cascades_with_match`；`w5-migration8.test.ts` | Rust 169；frontend 113 | 已证实 |
| CC-FR-012 | reframe actions/commands | reframe edit/reset/regenerate Rust tests；`action-security.test.ts`、journey tests | Rust 169；frontend 113 | 已证实 |
| CC-FR-013 | `domain/skills.rs`、skills UI | `gaps_are_rows_sorted_by_gap_not_radar_dimensions` | Rust 169；产品决策 | 已取代：不做雷达图 |
| CC-FR-014 | `domain/skills.rs`、`infra/skills`、`application/skills.rs` | `frozen_catalog_has_exactly_51_valid_nodes`；`trusted_resource_lookup_accepts_only_bundled_https_urls`；`custom_skill_crud_and_duplicate_rules`；`w4-skills-learning-jobs.test.ts` | Rust 171；frontend 114 | 已证实；图谱详情支持复制/受控打开 |
| CC-FR-015 | `infra/skills`、learning commands | `resource_urls_are_normalized_and_deduplicated`；`bundled_learning_resource_gets_open_token_without_page_collection`；learning persistence tests；`w4-skills-learning-jobs.test.ts` | Rust 171；frontend 114 | 已证实；路径支持复制/受控打开 |
| CC-FR-016 | `domain/skills_learning.rs`、migration 8 | `completion_updates_progress_and_path`；`conversion_is_explicit_and_once_only`；`migration8_preserves_conversion_as_snapshot` | Rust 169 | 已证实 |
| CC-FR-017 | `domain/skills.rs`、skills-learning actions | `what_if_does_not_mutate_and_reports_delta`；前端 stale-response/race tests | Rust 169；frontend 113 | 已证实 |
| CC-FR-018 | `application/resumes.rs`、resume version repository | `rejects_sixth_version_for_same_persona`；`compare_restore_and_missing_paths`；`survives_restart`；`keeps_five` | Rust 169 | 已证实 |
| CC-FR-019 | onboarding frontend | `onboarding.test.ts`、`p0-workflows.test.ts` | frontend 113；W6 未过 | 部分 |
| CC-FR-020 | `infra/http/mod.rs`、job-url action | `rejects_all_private_host_forms_and_bad_schemes`；`invalid_collection_returns_explicit_unsupported_fallback`；URL frontend tests | Rust 169；W6/真实站点未过 | 部分 |
| CC-FR-021 | `infra/secrets`、settings commands/actions | `credential_target_migration_is_isolated_and_never_plaintext_config`；settings frontend tests | adapter 自动化；真实 Credential Manager 未验 | 部分 |
| CC-FR-022 | commands、external-link frontend | `external_url_token_is_bound_and_single_use`；`w4-skills-learning-jobs.test.ts` | Rust/frontend 绿；W6 未过 | 部分 |
| CC-FR-023 | `infra/db`、migrations、`portable_backup.rs` | `golden_legacy_rows_migrate_1_through_8_without_loss_and_restore`；migration 9/10；pending restore/failure recovery；portable tests | Rust 169；W7 7/7；W9 复审 Go | 已证实 |
| CC-FR-024 | Tauri config、updater/recovery、workflows | updater state/recovery tests；production isolation；NSIS 本机 smoke | security 12/12、build/isolation 绿；签名/VM 缺失 | 部分 |

## 非功能需求

| ID | 实际测试/门禁 | 最近证据 | 状态 |
|---|---|---|---|
| CC-NFR-001 性能 | `w7_gate.rs`；`w7-desktop-gate.ps1` | 后端 7/7；桌面脚本因无 selector-ready 证据拒绝通过 | 部分 |
| CC-NFR-002 长任务 | `task_lifecycle_is_queryable_and_cancelled`；`stream_events_are_queryable_and_indeterminate_progress_is_honest`；late-token test | Rust 169；frontend task tests | 已证实 |
| CC-NFR-003 WAL/恢复 | W7 `wal_forced_crash_atomic_recovery`；migration rollback/restore tests | W7 7/7；Rust 169 | 已证实 |
| CC-NFR-004 本地优先 | `local_parser_is_available_without_network`；W7 offline CRUD/preview | 后端通过；安装版断网重启未验 | 部分 |
| CC-NFR-005 AI 安全 | raw preservation、side-effect-free preview、proposal confirmation、redaction tests | Rust/frontend 绿 | 已证实 |
| CC-NFR-006 轻量签名安装 | `verify_production_isolation.ps1`、release workflow gates | 无 Python/Qt/Chromium 与隔离通过；签名 updater/VM 缺失 | 部分 |
| CC-NFR-007 中文/键盘/状态 | UTF-8 contract tests、frontend loading/error/escape tests | jsdom 绿；真实键盘/WebView/中文路径缺失 | 部分 |

## 安全与隐私需求

| ID | 实际测试/证据 | 最近证据 | 状态 |
|---|---|---|---|
| CC-SEC-001 | `credential_target_migration_is_isolated_and_never_plaintext_config`；`infra/secrets` | Rust 绿；真实 Credential Manager 未验 | 部分 |
| CC-SEC-002 | operation-specific prompt、cache/log redaction tests | Rust 绿；`SECURITY_THREAT_MODEL.md` | 已证实 |
| CC-SEC-003 | 明确“不加密 SQLite”的 ADR | `SECURITY_THREAT_MODEL.md` | 已证实，残余风险接受仍属发布决策 |
| CC-SEC-004 | migration backup/retention/restore、portable inspect/import | Rust 169；W7 7/7；W9 复审 Go | 已证实（系统级发布矩阵另行阻断） |
| CC-SEC-005 | HTTP(S)、凭据 URL、私网/metadata、redirect/size/timeout tests | Rust 绿 | 已证实 |
| CC-SEC-006 | command manifest、capability/CSP、单次 token、production isolation | security 12/12；build/isolation 绿；W6 CSP 证据缺失 | 部分 |

## 机器可核验规则

1. 自动化基线：Rust 171、frontend 23 files/114、security 12/12、build 和 production isolation 全绿。
2. 每次准出记录必须包含提交 SHA、UTC 时间、Windows/WebView2 版本、命令、退出码和报告路径。
3. W6 仅以真实进程的 JUnit/日志/截图及 selector-ready 证据为通过；jsdom/mock 不计入 12 条桌面旅程。
4. W7 桌面性能必须消费同一二进制的 W6 selector-ready 数据。
5. W8 必须有真实签名、更新/失败回滚与干净 Win10/11 VM 报告。缺任一项，公开发布固定 No-Go。

延期：主题/字体颜色/自定义模板、面试/薪资、社区/市场。重新纳入时必须新增独立需求与测试 ID。
