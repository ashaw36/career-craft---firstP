# CareerCraft 功能验收报告

复验更新：2026-07-22。文件名保留首次验收日期，内容以本次复验为准。

## 结论

当前版本的冻结功能在 Rust 领域层、Tauri command 契约和前端编排层已基本闭环，适合作为内部 Preview 候选；**尚不满足公开发布准出，结论为 No-Go**。

原因是 W6 真实 WebView2 E2E 尚未跑通，连带阻塞 W7 真实桌面启动/内存证据；W8 仍缺生产签名 updater、可信旧版本回滚链和干净 Win10/11 VM 验证。单元测试数量不能替代这些证据。

## 当前自动化基线

| 门禁 | 最新结果 | 证据范围 |
|---|---:|---|
| Rust/Tauri 全特性 | 171 passed | `src-tauri/src` 单元/集成测试 |
| 前端 | 23 files、114 tests | `tests/frontend`，jsdom/fixture/mock |
| 前端生产构建 | 通过 | TypeScript + Vite |
| 安全静态门禁 | 12/12 | `scripts/w8-security-gate.mjs` |
| 生产隔离 | 通过 | release 无 WDIO capability/plugin/devtools |
| W7 后端性能/恢复 | 7/7 | `target/w7-gate/*.json`、JUnit |
| W6 真实桌面旅程 | 未通过 | 外置 driver 无 session；嵌入式首次 DOM/execute 超时 |
| Win10/11 发布矩阵 | 未执行完成 | 缺签名、更新端点、VM 与真实回滚链 |

以上测试数是当前工作树的门禁基线；最终报告仍须附提交 SHA、时间、平台及原始输出。

## 24 项功能验收

状态：已证实＝实现和自动测试具备；部分＝内部实现存在但系统/桌面证据不足；已取代＝后续产品决策替换。

| ID | 状态 | 判断 |
|---|---|---|
| CC-FR-001 | 已证实 | 中英结构化、可编辑草稿、取消、原文精确保留、最近三次 revision/CAS restore 有测试；W9 最终复审前不作为公开准出证据。 |
| CC-FR-002 | 已证实 | CRUD、倒序、严格日期重叠、生命周期和并发冲突有测试。 |
| CC-FR-003 | 已证实 | TXT/MD/JSON/PDF/DOCX 与损坏/超限输入有 Rust 测试；真实文件人工签核待 W6。 |
| CC-FR-004 | 已证实 | Persona CRUD、删除不删经历、切换编排和后端性能有测试。 |
| CC-FR-005 | 已证实 | 结构化技能/年限/行业/学历证据、override/reset、稳定排序有测试。 |
| CC-FR-006 | 已证实 | 五模板、无副作用 preview、Markdown/PDF、指定历史 version 导出有测试；安装版文件签核待完成。 |
| CC-FR-007 | 已证实 | 五类结构化调优、预览确认、服务端 proposal/hash、undo/redo 有测试。 |
| CC-FR-008 | 已证实 | 多 Provider、角色首选、401/429/5xx/超时、retry/fallback、事件流、取消与 7 日缓存有测试。 |
| CC-FR-009 | 已证实 | 中英 JD、原文和结构化要求有测试。 |
| CC-FR-010 | 已证实 | 0–100 及 50/25/15/10 breakdown、证据来源有测试。 |
| CC-FR-011 | 已证实 | 岗位状态 CAS、append-only audit、历史展示有测试。 |
| CC-FR-012 | 已证实 | JD 重述、原文隔离、编辑/重置/重生成有测试。 |
| CC-FR-013 | 已取代 | 雷达图由 matched/missing 和技能详情取代，不作为当前缺陷。 |
| CC-FR-014 | 已证实 | 51 节点、别名/依赖、搜索、资源和自定义 CRUD 有测试。 |
| CC-FR-015 | 已证实 | 每个内置技能至少三条去重 HTTP(S) 资源、来源筛选和路径持久化有测试。 |
| CC-FR-016 | 已证实 | 学习状态、并发版本、完成说明和一次性事务转经历有测试。 |
| CC-FR-017 | 已证实 | 假设技能增删、delta、无持久副作用和前端竞态保护有测试。 |
| CC-FR-018 | 已证实 | 至少五版本、diff、restore、父版本和重启持久化有测试。 |
| CC-FR-019 | 部分 | 首次引导 jsdom 测试通过；干净安装首次启动未证实。 |
| CC-FR-020 | 部分 | 用户主动 URL、SSRF 防护、失败手贴降级有测试；登录辅助和真实站点稳定性未证实。 |
| CC-FR-021 | 部分 | Provider CRUD、Key 不回显和 Credential Manager adapter 有测试；真实 Windows 凭据系统 E2E 缺失。 |
| CC-FR-022 | 部分 | 单次限时外链 token 和复制/提示编排有测试；系统浏览器行为缺真实桌面证据。 |
| CC-FR-023 | 已证实 | legacy golden 迁移、备份、失败恢复、WAL 强杀和 portable 校验/恢复测试具备；W9 最终复审待完成。 |
| CC-FR-024 | 部分 | NSIS、本机安装、Updater adapter/恢复状态机及 release workflow 已实现；生产签名、真实更新回滚和 VM 矩阵未完成。 |

## 准出判定

- 内部 Preview：Go；W9 独立复审及最新全量基线均已通过。
- 公开发布：W6、W7 桌面部分、W8 外部发布条件全部通过前固定 No-Go。
- 明确延期：主题/自定义模板、面试与薪资、社区/市场；不得宣称已交付。
