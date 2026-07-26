# Tauri/Rust 重构开发任务清单

任何开发代理开始前必须阅读 `REQUIREMENTS_TRACEABILITY.md`、本文件和 `TEST_PLAN.md`。共享接口变更由主代理批准。

## WP0 行为冻结与治理

- [ ] 建立旧 SQLite schema/fixture/脱敏黄金库。
- [ ] 固化 33 个旧 Bridge 行为为版本化 command DTO/JSON Schema。
- [ ] 建立 PDF/DOCX/Markdown 黄金样本目录和预期输出。
- [ ] 固化 5 套模板、51 节点技能图谱和核心 Prompt。
- [ ] 建立需求 ID → 新模块 → 测试 ID 的 CI 检查。

## WP1 平台骨架

- [ ] 创建 Tauri 2 + Vite + TypeScript 独立工程。
- [ ] Rust 分层单体：domain/application/infra/interface/bootstrap。
- [ ] typed command codegen，前端禁止散落原始 `invoke()`。
- [ ] AppError/envelope、任务事件与 CancellationToken。
- [ ] CSP、capability 白名单、日志脱敏、panic/crash marker。

## WP2 数据与迁移

- [ ] 用 `rusqlite` 映射现有 9 表和索引/外键。
- [ ] 编号 SQL migration 和 `schema_migrations`。
- [ ] 启动前锁、备份、integrity/FK check、事务升级、失败恢复。
- [ ] Repository 层与领域 DTO 解耦。
- [ ] 新表：resume_versions、custom_skills、task/recovery metadata（按设计确认）。

## WP3 经历、角色与 Fit

- [ ] 经历 CRUD、冲突、草稿确认/拒绝、原文不可变。
- [ ] 文本/JSON/Markdown 导入。
- [ ] Persona CRUD、角色切换、删除不级联经历。
- [ ] Fit Score 基线、解释、人工覆盖/重置和排序。

## WP4 LLM 与设置

- [ ] OpenAI-compatible Provider、通义兼容与 fallback。
- [ ] 超时、429/5xx 退避、流式、取消、错误归一化。
- [ ] Prompt 原样迁移与版本标识。
- [ ] 角色首选模型生效。
- [ ] Credential Manager；旧明文 Key 安全迁移后删除。
- [ ] AI 数据发送告知与最小化。

## WP5 技能与学习

- [ ] 51 节点、别名/关联/前置、搜索和资源。
- [ ] 自定义技能 CRUD。
- [ ] Gap 路径生成和来源分类。
- [ ] 学习状态、进度、完成转经历。
- [ ] 资源坏链与离线降级。

## WP6 文档 PoC 与实现

- [ ] DOCX 段落/标题/列表/表格/页眉解析。
- [ ] PDF 中文、多栏、表格、损坏/加密检测。
- [ ] 文件大小/MIME/zip bomb/path traversal 防护。
- [ ] 5 模板 Markdown/HTML 渲染。
- [ ] WebView2 print-to-PDF 中文字体、分页与长文本。
- [ ] Rust 不达标时触发 .NET NativeAOT 文档模块回退评审。

## WP7 简历与版本

- [ ] 按 Fit Score 生成、预览、Markdown/PDF。
- [ ] 对话式调优、确认、撤销/恢复。
- [ ] 每角色至少 5 个版本。
- [ ] 版本差异比较与恢复。

## WP8 JD、匹配与重述

- [ ] JD 解析与原文保存。
- [ ] 50/25/15/10 分项算法等价。
- [ ] matched/missing skills 与状态跟踪。
- [ ] 重述事实隔离、编辑/重置/重生成。
- [ ] 假设技能增删与实时提升模拟。

## WP9 采集

- [ ] 普通 URL 的 HTTP 获取、SSRF/allowlist/限速。
- [ ] 登录站点 WebView2 用户登录辅助 PoC。
- [ ] DOM 变化、验证码、断网时手动粘贴降级。
- [ ] 不内置 Playwright Chromium。

## WP10 前端

- [ ] 按功能域迁移八个页面与首次引导。
- [ ] Notion 风格 DESIGN.md 视觉规范。
- [ ] 状态管理、草稿保持、键盘与可访问性。
- [ ] 长任务进度/取消/失败/恢复。
- [ ] 页面不得直接接触数据库或底层命令。

## WP11 发布

- [ ] Windows Setup.exe/NSIS 或 MSI，安装器处理 WebView2 prerequisite。
- [ ] updater 签名、灰度、失败回滚、schema 兼容窗口。
- [ ] 普通用户、中文路径、离线和无开发环境运行。
- [ ] 卸载时明确保留或删除用户数据。
- [ ] 正式代码签名方案。

## 并行所有权

- 平台/DB 代理：`src-tauri` bootstrap、infra/db、migration、typed command。
- 业务代理：domain/application、LLM、matching、skills/learning。
- 前端代理：`src` 前端页面、typed client、状态与 UI 测试。
- 文档代理在 WP1 完成后单独负责 documents corpus/实现。
- 主代理独占共享 schema、Cargo 根配置、合并与架构决策。

