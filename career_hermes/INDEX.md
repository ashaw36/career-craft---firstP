# CareerCraft Agent — 项目单点入口

> 基于 BMad 框架开发 | 个人职业智能体 | 角色档案驱动
> 当前阶段：**Sprint 13 完成 — LLM 配置自定义化** — P0✅ P1✅ P2✅ P3✅ | 149 tests passed | 多 Provider 管理 | 自定义 base_url + 模型名 | 应用偏好已删除

## 🔒 已锁定决策（Decision Lock）

| 决策 | 选择 | 不可逆理由 |
|------|------|-----------|
| **D1 — 产品形态** | B. 桌面应用（PySide6 GUI）| 个人自用工具，数据本地化优先，无需服务端部署 |
| **D2 — 目标用户** | A. 个人自用 | MVP 只服务单一用户，架构预留多用户扩展口 |
| **D3 — 数据来源** | 简历A + 岗位爬虫B + 学习素材B | 经历库从0构建，岗位依赖Playwright爬虫，学习素材基于Gap动态检索 |
| **D4 — LLM策略** | 通义千问为主，支持多模型切换 | 通义千问API已确认可用，成本低、中文效果好 |
| **D5 — 角色引擎** | 规则+Prompt混合方案 | 个人工具需可解释性和可控性，规则引擎保留用户干预能力 |

## 🔒 新锁定决策（Sprint 7-8 阶段）

| 决策 | 状态 | 内容 |
|------|------|------|
| **D2 — 核心差异化** | ✅ 已锁定 | 聚焦"匹配度量化 + 提升路径"，不做简历工具 |
| **D3 — 数据飞轮** | ✅ 已锁定 | MVP 纯本地，V1.0 加入可选云同步 |
| **D4 — 简历模板** | ✅ 已锁定 | Phase 2 引入模板配置系统：选择+预览+自定义 |
| **D1 — 目标用户** | ⏸️ 暂不锁定 | 当前专注自用，后续视反馈决定 |

## 📋 项目全景

CareerCraft Agent 是一个**角色档案驱动的个人职业智能体**，运行于本地桌面。

```
个人经历录入 → 角色档案配置 → 简历自动生成 → 岗位智能匹配 → 技能 Gap 分析 → 学习路径推荐 → 能力提升追踪
```

### 核心特性
- **多角色档案**：同一套经历库，切换 AI PM / 销售 / 架构师等角色，生成对应简历和岗位匹配
- **对话式调整**：目标岗位、角色侧重均可通过自然语言动态调整
- **多模型切换**：支持通义千问、OpenAI、Claude 等多种 LLM 后端，自动降级
- **本地优先**：数据存储在本地 SQLite，API Key 加密保存，隐私可控

## 📅 开发进度

| 阶段 | 状态 | 交付物 |
|------|------|--------|
| **Phase 1 — 分析** | ✅ 已完成 | 市场调研、技术预研、需求分析 |
| **Phase 2 — 规划** | ✅ 已完成 | PRD v1.0（含P0/P1/P2、验收标准、用户旅程） |
| **Phase 3 — 方案** | ✅ 已完成 | 架构设计 v1.0、Epic/Story 拆分、就绪检查 |
|| **Sprint 1 (Week 1-2)** | ✅ 已完成 | 项目骨架、数据库ORM、LLM路由、安全存储 |
|| **Sprint 2 (Week 3-4)** | ✅ 已完成 | 经历管理服务、角色引擎、Fit Score 计算 |
|| **Sprint 3 (Week 5-6)** | ✅ 已完成 | 简历生成引擎（Jinja2+Fit Score排序）、对话引擎（意图识别） |
|| **Sprint 4 (Week 7-8)** | ✅ 已完成 | JD解析服务、经历重述引擎、多模型容错降级、JobMatcher |
|| **Sprint 5 (Week 9-10)** | ✅ 已完成 | 岗位匹配算法、Gap分析、学习路径推荐（learning_recommender） |
|| **Sprint 6 (Week 11-12)** | ✅ 已完成 | GUI完善（经历/角色/简历/岗位页面）、Polish、测试补齐 |
|||| **Sprint 7-8 (Week 15-18)** | ✅ 已完成 | 岗位匹配增强+JD修饰、WebView全页面动态化、e2e测试补齐、BMAD文档更新 |
|||| **Sprint 9-10 (Week 19-22)** | ✅ 已完成 | P0✅ P1✅ P2✅ P3✅ | 126测试通过 |
||||| **Sprint 11 (Week 23-24)** | ✅ 已完成 | PDF导出 + Fit Score覆盖 + 重述编辑重置 | 149测试通过 |
|||||| **Sprint 12 (Week 25-26)** | ✅ 已完成 | 51技能节点资源补充 + 学习路径真实链接 + 来源分类 + WebView外部跳转 | 149测试通过 |
||||||| **Sprint 13 (Week 27)** | ✅ 已完成 | LLM配置自定义化 + SecureStorage链路打通 + 数据库Schema自动修复 + 学习路径错误处理改进 | 149测试通过 |

## 📁 核心文件清单

### 稳定知识树 `docs/`
| 路径 | 说明 |
|-------|------|
| `_bmad-output/planning-artifacts/prd/PRD_v1.0.md` | 需求文档 |
| `_bmad-output/planning-artifacts/architecture/ARCH_v1.0.md` | 架构设计 |
| `_bmad-output/planning-artifacts/epics/EPICS_v1.0.md` | Epic/Story 拆分 |
| `_bmad-output/planning-artifacts/phase1/` | Phase 1 三份调研报告 |

### 源代码 `src/`
| 路径 | 行数 | 说明 |
|-------|------|------|
| `src/models/entities.py` | 254 | 7张核心ORM表（已扩展JobDesc/JobMatch/LearningPath） |
| `src/models/database.py` | 70 | 异步SQLite引擎、WAL模式 |
| `src/config/settings.py` | 154 | Pydantic Settings + YAML配置 |
| `src/llm/router.py` | 300+ | LLM路由器，多模型自动降级 + 重试装饰器（指数退避） |
| `src/llm/prompts/*.py` | 183 | LLM Prompt模板（job_parsing、retelling、job_matching） |
| `src/services/experience_manager.py` | 248 | 经历CRUD、对话式录入、冲突检测 |
| `src/services/persona_engine.py` | 240 | 角色CRUD、Fit Score计算 |
| `src/services/resume_builder.py` | 166 | 简历渲染引擎（Jinja2模板） |
| `src/services/conversation_engine.py` | 100 | 自然语言意图识别 |
| `src/services/job_parser.py` | 236 | JD解析服务（模板+LLM回退） |
| `src/services/retelling_engine.py` | 247 | 经历重述引擎（归纳+扩展两模式） |
| `src/services/job_matcher.py` | 376 | 岗位匹配器（规则算法：技能60%、经验30%、其他10%） |
| `src/services/learning_recommender.py` | 294 | 学习路径推荐（LLM回退+本地模板库） |
| `src/services/pdf_exporter.py` | 242 | PDF简历导出服务（fpdf2） |
| `src/services/import_parser.py` | 372 | 经历批量导入解析器（Markdown/文本/JSON） |
|| `src/services/jd_reframe_engine.py` | 331 | JD经历修饰引擎（为岗位匹配生成经历修饰版本） |
|| `src/services/skill_graph.py` | 200+ | 技能图谱管理（50节点加载、搜索、关联分析） |
|| `src/data/skill_graph.json` | 50节点 | 预置技能图谱：产品(15)+技术(15)+管理(10)+行业(10) |
|| `src/crawlers/base.py` | 180+ | 爬虫基类（Playwright封装+UA轮换+Cookie复用+Stealth） |
| `src/crawlers/boss_zhipin.py` | 141 | Boss直聘爬虫 |
| `src/crawlers/jd_crawler.py` | 80 | JD爬虫存根（Mock模式） |
||| `src/ui/webview/bridge.py` | 280 | QWebChannel Python桥接（23个API端点） |
||| `src/ui/webview/webview_window.py` | 110 | WebView主窗口（QWebEngineView + DevTools） |
||| `src/ui/webview/api_handler.py` | 610 | 同步API适配层（异步Service → 同步Bridge） |
|| `src/ui/webview/__init__.py` | 6 | WebView模块导出 |
|| `src/main_webview.py` | 35 | 应用入口（WebView） |
|| `src/utils/security.py` | 186 | API Key加密存储（keyring/Fernet） |
|| `prototype/ui-prototype.html` | 1,600 | Linear深色风格HTML原型（8页面 + JS桥接） |
|| `prototype/qwebchannel.js` | 456 | Qt WebChannel JS库 |

|**总代码量：~4,300 行（不含测试）**

### 测试 `tests/`
| 路径 | 说明 |
|------|------|
| `tests/conftest.py` | pytest-asyncio配置 + 内存数据库fixture |
| `tests/test_experience_manager.py` | 经历CRUD、冲突检测测试 |
| `tests/test_persona_engine.py` | 角色引擎、Fit Score测试 |
| `tests/test_resume_builder.py` | 简历渲染上下文测试 |
| `tests/test_security.py` | API Key安全存储测试 |
| `tests/test_router.py` | LLM Router（mock httpx）测试 |
| `tests/test_job_matcher.py` | 岗位匹配算法、状态更新测试 |
| `tests/test_learning_recommender.py` | 学习路径推荐测试 |
| `tests/test_pdf_exporter.py` | PDF导出服务测试 |
| `tests/test_router_mock.py` | LLM Router Mock 模式测试 |
| `tests/test_import_parser.py` | 经历批量导入解析测试 |
|| `tests/ui/webview/test_bridge.py` | WebView Bridge API 测试（17个） |
||| `tests/test_skill_graph.py` | 技能图谱管理测试 |

||**测试总数：149 个用例，全部通过**

|**总代码量：~4,300 行 Python + ~2,000 行 HTML/JS（不含测试）**

| Commit | 说明 |
|--------|------|
| `d3cfa85` | Sprint 1: 项目骨架 + 数据库ORM + LLM路由 + 安全存储 |
| `8b4e6b8` | Sprint 2: 经历管理服务 + 角色引擎 + Fit Score 计算 |
| `62bae86` | docs: 更新 INDEX.md 项目状态，Sprint 1-2 完成 |
| `837104b` | feat(sprint3): 简历生成引擎 + 对话引擎 + Jinja2模板 |
| `6f3f234` | docs: 添加 README.md 快速启动指南 |
| `a949267` | feat: Sprint 4-6 完成 — 岗位匹配、学习路径、GUI完善、53测试通过 |
| `ba175a7` | feat: 优化三大限制 + 打包 — 行点击修复、Mock LLM、PDF导出、PyInstaller脚本、63测试通过 |
| `10e6c7a` | docs: 产品功能路线图 v1.0 + INDEX更新 — 锁定差异化/模板/云同步决策 |
|| `3dd4441` | feat(Sprint7): 经历批量导入 + 爬虫框架 + 模板选择 — 74测试通过 |
|| `387183c` | feat(Sprint8): 岗位匹配增强+JD修饰+WebView动态化 — 109测试通过 |
||| `e736919` | feat(Sprint9-10 P0+P1): 冲突检测+WebView全页面动态化+加载状态/错误提示 — 109测试通过 |
||| `fc9a18c` | feat(Sprint9-10 P2): 首次启动引导+PyInstaller打包+技能Gap雷达图 — 109测试通过 |
||| `2006091` | feat(Sprint9-10 P1-3): 对话式简历调优 — 109测试通过 |
||| `3e604ee` | feat(crawler): Boss直聘爬虫稳定化(P2-2) — UA轮换+Cookie复用+Stealth — 109测试通过 |
||| `d51a439` | P3: 技能图谱预置50节点 — 126测试通过 |

## 📋 Notion 映射

- 母页面：[hermes信息收集](https://www.notion.so/3691bfd6-d4a8-8075-b435-ec4385b4bb73)
- 备命名格式：`YYYY-MM-DD_标题.md`

## 🚀 快速启动

```bash
cd /mnt/d/workplace_for_hermes/career-agent
source .venv/bin/activate
python -m src.main_webview
```

运行测试：
```bash
pytest tests/ -v --tb=short
```

## 📝 变更日志

- **2026-07-19** — **Sprint 12 完成 — 学习资源体系完善** (`c0b70d0`)：
  - 子代理并行搜索 51 个技能节点优质学习资源（GitHub/bilibili/官方文档/知乎/豆瓣读书/Coursera），编入 `skill_graph.json`
  - SkillNode 模型新增 `resources` JSON 字段，替代原单个 `learning_url`
  - LearningPath 模型新增 `source_type` 字段（jd_gap / skill_graph / manual）
  - LearningRecommender 优先从 skill_graph 读取 resources 生成带真实链接的学习路径，再 LLM，再 fallback
  - 前端学习路径页面增加来源分类二级菜单（全部/技能图谱/JD补充/手动创建）+刷新列表
  - WebView 外部链接跳转：子类化 ExternalLinkPage(QWebEnginePage)，拦截 target="_blank" 用系统浏览器打开
  - 技能详情弹窗展示 resources 列表（标题/类型/来源/时长/链接）
  - 测试：**149 passed** 全部通过

- **2026-07-19** — **Sprint 13 完成 — LLM 配置自定义化** (`63bf2a6`)：
  - 设置页删除"应用偏好"卡片（主题/默认经历数等未实现功能）
  - LLM 配置卡片改造为完整 Provider 编辑器：多 Provider 列表、切换编辑、添加/删除 Provider
  - 支持自定义 Base URL 输入（任意 OpenAI 兼容 API 代理）
  - 模型名改为自由输入 text input（原下拉框只有3个固定选项，不够用）
  - API Key 不回显（安全），留空则保留原值
  - 页面加载时自动回显当前 config.yaml 配置
  - 后端 `save_settings` 重写：全量写入 `llm_providers` 数组 + `default_llm_provider`，清空 `_settings` 缓存保证即时生效

- **2026-07-19** — **Sprint 13 patch** (`04afed3`)：
  - 技能弹窗和学习路径页面的所有"访问资源"链接，从 `target="_blank"` 跳转改为**点击复制链接到剪贴板**
  - 新增 Bridge `copyToClipboard(text)` Slot，通过 `QApplication.clipboard().setText()` 写入系统剪贴板
  - 前端新增 `copyResourceUrl()` + `showCopiedToast()` 函数，复制成功后底部显示绿色 Toast 提示
  - 测试：**149 passed** 全部通过

- **2026-07-19** — **删除原生 PySide6 GUI，只保留 WebView**：
  - 删除文件：`src/main.py` (41行)、`src/ui/main_window.py` (243行)、`src/ui/pages/*` (共2,108行)
  - 原因：WebView 已完全覆盖 P0+P1 全部功能，原生 GUI 缺 P1 功能且维护成本高
  - 唯一入口：`python -m src.main_webview`
  - 测试：**131 个，全部通过** | Python 代码量: ~4,300 行

- **2026-07-19** — **前端 P0/P1 缺失功能补齐**（由 Codex 执行）：
  - 原状：后端+Bridge 126 测试通过，但 HTML 前端 7 处仅 UI 占位未接入 pybridge
  - 补齐项：
    1. 简历页「导出 Markdown/PDF」接入 `generateResume` / 新增 `exportResumePDF`
    2. 简历页「模板选择」下拉框（5 种模板），`generateResume` 支持 `template` 参数
    3. 设置页「保存配置/测试连接」接入，新增 `saveSettings` / `testLLMConnection` Slot
    4. 新增「技能图谱」独立页面（Sidebar 导航），调用 `getSkillGraph`/`searchSkills`
    5. 新增「学习路径」独立页面（Sidebar 导航），调用 `getLearningPath`
    6. 经历页「批量导入」面板，调用新增 `importExperiences` Slot
    7. 欢迎页 CTA 按钮补齐 `switchPage` 跳转
  - 修改文件：`prototype/ui-prototype.html` (+415行)、`bridge.py` (+49行)、`api_handler.py` (+68行)、`test_bridge.py` (+34行)
  - 测试：**131 个，全部通过** | 代码量: ~6,400 行

- **2026-07-19** — **Sprint 9-10 全部完成**并推送 GitHub `d51a439`：
  - ❯ P0-1: 经历时间冲突检测 — `TimeConflictError` + API 层返回 `error_type: TIME_CONFLICT` + 前端弹窗
  - ❯ P0-2~5: WebView 全页面动态化 — 经历/角色/简历/欢迎页全部接通 bridge API，支持点击切换、表单编辑、新建/删除
  - ❯ P1-1: 全局加载遮罩层 `showLoading`/`hideLoading`
  - ❯ P1-2: 友好错误提示 `showError(message, suggestion)` — 右上角浮窗
  - ❯ P1-3: 对话式简历调优 — `chatRefineResume` Slot + API + 前端输入框/历史/结果展示
  - ❯ P2-1: 首次启动引导 — 经历为空时欢迎页显示引导卡片 + CTA跳转
  - ❯ P2-2: Boss 直聘爬虫稳定化 — UA 轮换、随机延迟、Cookie 复用、Stealth 模式、请求头补全
  - ❯ P2-3: PyInstaller 打包重构 — onefile/onedir 开关、打包前测试检查、路径解析适配 `_internal`
  - ❯ P2-4: 技能 Gap 雷达图 — ECharts 雷达图可视化当前角色 vs 目标岗位技能覆盖
  - ❯ P3: 技能图谱预置 50 节点 — 产品(15)+技术(15)+管理(10)+行业(10)，`SkillGraph` 类 + Bridge API
  - 新增 bridge API: `deleteExperience`, `getPersonaById`, `createPersona`, `updatePersona`, `deletePersona`, `chatRefineResume`, `getSkillGraph`, `searchSkills`
  - 新增测试: `test_skill_graph.py` + bridge 拓展 = 17 个新用例
  - 测试: **126 个，全部通过** | 代码量: ~5,800 行

- **2026-07-19** — **P0+P1 完成**并推送 GitHub `e736919`...
  - ❯ P0-1: 经历时间冲突检测 — `TimeConflictError` + API 层返回 `error_type: TIME_CONFLICT` + 前端弹窗
  - ❯ P0-2~5: WebView 全页面动态化 — 经历/角色/简历/欢迎页全部接通 bridge API，点击切换、表单编辑、新建/删除
  - ❯ P1-1: 全局加载遮罩层 `showLoading`/`hideLoading`
  - ❯ P1-2: 友好错误提示 `showError(message, suggestion)` — 右上角浮窗
  - 新增 bridge API: `deleteExperience`、`getPersonaById`、`createPersona`、`updatePersona`、`deletePersona`
  - 测试: **109 个，全部通过** | 代码量: ~5,400 行

- **2026-07-19** — 岗位匹配增强 + WebView 前端动态化 + e2e测试补齐：
  - 修复 `resume_builder.py` 经历为空 Bug：`min_score` 从 0.15 降至 0.0，无权重时增加 fallback 逻辑；修复跨线程 ORM 对象 detached 状态引发的隐患
  - 新增 `parseJD` + `matchJob(job_desc_id, persona_id)` 拆分流程，支持选择角色后匹配
  - 新增岗位管理 API：`listJobs`、`deleteJob`、`getJobMatches`、`updateMatchStatus`、`reframeResume`、`getReframeResults`
  - HTML 原型 `ui-prototype.html` 岗位页完全动态化
  - 新增 e2e 测试 3 个文件：test_resume_e2e(2)、test_persona_e2e(2)、test_job_match_e2e(2)
  - BMAD 文档更新：Sprint7-8_Plan 补录 + 新建 Sprint9-10_Plan
  - 测试总数：109 个，全部通过 | 代码量：~5,400 行

- **2026-07-17** — Sprint 7 推进：
  - 新增 `import_parser.py`：支持 Markdown/文本/JSON 三种格式经历批量导入，解决冷启动问题
  - 新增 `crawlers/` 框架：BaseCrawler + BossZhipinCrawler（Playwright）
  - 更新 `experience_page.py`：添加批量导入UI（Tab切换+文件加载）
  - 更新 `resume_page.py`：模板选择扩展为 5 种（modern/classic/minimal/tech/外企）
  - 补 11 个单元测试，总计 **74 个测试全部通过**
  - 代码量：~4,800 行

- **2026-07-17** — 三大限制优化 + 打包基础：
  - 修复 `job_match_page` 行点击：通过 `_job_id_map` 映射表实现点击表格行查看匹配详情
  - 添加 LLM Mock 模式：`LLMRouter(mock=True)` 或 `enable_mock()`，开发测试无需真实 API Key
  - 新增 PDF 导出服务 `pdf_exporter.py` + UI "导出 PDF" 按钮（依赖 fpdf2）
  - 新增 PyInstaller 打包脚本 `build.py`
  - 补 10 个单元测试（Mock + PDF），总计 **63 个测试全部通过**

- **2026-07-17** — Sprint 1-6 全部完成：
  - Sprint 4: JD解析(job_parser)、经历重述(retelling_engine)、JobMatcher(规则匹配)、router多模型降级
  - Sprint 5: learning_recommender(学习路径推荐)
  - Sprint 6: GUI完善(经历/角色/简历/岗位页面)、测试齐全(53个通过)
  - 代码总量：~3,640 行，自驻 cron job 每30分钟检查

- **2026-07-19** — 前端交互全面审计 + 14项缺陷修复：
  - P0 修复：经历新建按钮补齐 `newExperience()`、经历保存重写支持全字段、岗位列表技能标签 `_job_to_dict` 映射
  - P1 修复：全局header移除"新建"按钮 + "Mock模式"改为"连接状态"、批量导入扩展PDF/Word accept(但提示暂不支持)、简历生成返回技能覆盖数据、设置 `save_settings` 实现真实保存(SecureStorage + config.yaml)、设置页应用偏好按钮解绑错误函数
  - P1 体验增强：技能图谱详情展示描述/前置技能/学习资源跳转、学习路径UI增加描述/链接/步骤、学习路径后端字段名统一 `duration`
  - P2 优化：经历列表排版 `text-overflow: ellipsis`、解析并匹配状态细化提示
  - 测试：**131 passed** 全部通过

- **2026-07-19** — 技能图谱与学习路径交互优化（已确认执行）：
  - 技能图谱页：点击卡片改为**居中模态弹窗**，前置/关联技能标签可点击跳转，增加「生成学习路径」按钮
  - 学习路径页：支持 URL 参数 `?skill=xxx&auto=1` 自动触发路径生成；资源卡片显示**可点击链接**（有 url 则直连，无则降级为 Bilibili 搜索）
  - 后端 `learning_recommender.py`：模板库补充 `url` 字段，LLM prompt 要求返回 `url`
  - 数据 `skill_graph.json`：高频技能补充 `learning_url`（用户研究、需求分析、Python、SQL、Docker、K8s 等）
  - 移除 `localStorage` 状态管理（WebView 不支持）
  - 测试：**149 passed** 全部通过

- **2026-07-19** — 文件导入自动分析测试 + E2E 验收：
  - 新增 `tests/test_import_parser_file.py` (16 测试)：PDF 文本提取、Word 文本提取、LLM 分析、import_file 完整链路、_to_draft 转换
  - 新增 `scripts/validate_import_e2e.py` 验收脚本：模拟项目总结上传 → 文本提取 → LLM分析 → 保存到库 → Bridge API 调用
  - 验收结果：字段完整性 100% 通过（title/type/raw_description/skills/organization/dates/achievements/metrics）
  - 测试：**148 passed** 全部通过

- **2026-07-19** — PDF/Word 解析 + Mock 清理：
  - 新增 `import_parser.extract_text_from_pdf()` 和 `extract_text_from_word()`，支持 PDF/Word 上传自动解析
  - 新增 Bridge API `importFile(file_name, base64_content)`，前端通过 `readAsDataURL` 传输 base64
  - 上传 PDF/Word 后 LLM 自动提取结构化经历并保存到经历库
  - 移除全局 header "Mock模式"/“连接状态"按钮，清理无用配置
  - 测试：**132 passed** 全部通过

- **2026-07-19** — **Sprint 11 完成 — P0/P1 补漏**：
  - Story 11.1: PDF 真实导出 (`pdf_exporter.py` 接入 Bridge，前端 base64→Blob 下载)
  - Story 11.2: Fit Score 手动覆盖 (角色页滑块 0-100 + `已手动调整`标记)
  - Story 11.3: 经历重述编辑/重置 (岗位详情页 textarea 编辑 + 删除重新走 LLM)
  - 149 测试通过，推送 `main` (`f867e11`)

- **2026-07-19** — 用户体验优兆修复:
  1. 角色创建/保存/删除后自动同步刷新简历页和岗位匹配页的角色选择器（新增 `refreshAllPersonaSelectors()`）
  2. 修复匹配并解析报错 `Parent instance <JobMatch> is not bound to a Session`：`_match_to_dict` relationship 访问加 try/except 降级，`list_matches` 补 `selectinload`
  3. 经历库明细行只展示项目名称，隐藏 role/company/date 副标题
  - 测试：**149 passed** 全部通过

- **2026-07-20** — **SecureStorage 链路修复 — Key 存储全链路打通**:
  1. `security.py` 增加明文 fallback：keyring 和 cryptography 都不可用时降级为明文文件存储
  2. `settings.py` `load_settings()` 加载后从 SecureStorage 注入 `api_key`，Router 无需改动即可读取
  3. `api_handler.py` `save_settings()` 严格校验：API Key 存储失败时返回 `{"success": false, "error": ...}`，不再静默吞掉
  4. 测试：`test_security.py` 更新为明文 fallback 场景
  - 测试：**149 passed** 全部通过
