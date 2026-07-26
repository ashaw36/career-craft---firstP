# CareerCraft Agent — 架构设计文档 v1.0

> Phase 3 产出 | 子代球：架构师代球

## 1. 模块划分与边界

| 层级 | 模块 | 职责 | 输入 | 输出 | 依赖 |
|------|------|------|------|------|------|
| **数据层** | `database.py` | SQLAlchemy 2.0 async engine、session factory、WAL 配置 | 连接字符串 | Session / Engine | 无 |
| | `models/` | 7 张核心 ORM 表 + 关系定义 | 迁移脚本 | 表结构 | database |
| | `migrations/` | Alembic 版本控制 | 模型变更 | SQL 迁移文件 | models |
| **服务层** | `ExperienceManager` | 经历 CRUD、草稿生命周期、时间冲突检测、技能标签提取、**批量导入解析** | 原始文本 / 字段 / draft_id / **文件路径** | Experience 对象、冲突报告、**导入记录** | Database, LLMRouter |
|| `PersonaEngine` | 角色 CRUD、Fit Score 计算、简历数据组装、经历重述 | persona_id, experience_list | 排序经历 + reframed_text + score 矩阵 | ExperienceManager, LLMRouter, SkillAnalyzer |
|| **`JobMatcher`** | **JD 解析、多维度匹配度计算**（技能匹配×50 + 经验匹配×25 + 文本相似度×15 + 其他×10）、**Gap 分析、状态追踪** | JD 文本/URL, persona_id | JobMatch 记录、match%、skill gaps、**score_breakdown** | LLMRouter, SkillAnalyzer, PersonaEngine |
|| `ResumeBuilder` | Jinja2 模板渲染、Markdown→PDF、对话式调优增量更新 | 筛选后的经历、模板 ID、调优指令 | Markdown, PDF 路径, diff | PersonaEngine, ExperienceManager |
|| `SkillAnalyzer` | 技能图谱 CRUD、别名标准化、预置节点加载、雷达图数据集 | 原始技能名 / 技能列表 | SkillNode 树、5 维聚合数据、标准名 | Database, LLMRouter(预留) |
|| **`ImportParser`** *(S7-8 新增)* | **Markdown/文本/JSON/PDF/Word 导入解析**、**LLM 自动分析非结构化文件** | 原始文本 / 文件内容 | ExperienceDraft 列表 | LLMRouter |
| | `LLMRouter` | 多模型配置、路由、故障降级、流式输出、Token 追踪 | messages, model_key, stream flag | 文本流/完整文本、latency | httpx, Settings |
|| **表示层** | `WebViewWindow` | WebView 主窗口、QWebEngineView + DevTools 调试 | HTML URL | 渲染后的 Web 页面 | `bridge.py` |
|| | `bridge.py` | QWebChannel Python 桥接（23个 API 端点）| JS 调用 | JSON 响应 | `api_handler.py` |
|| | `api_handler.py` | 同步 API 适配层（异步 Service → 同步 Bridge）| Bridge 请求 | Service 结果 | 所有 Service |
|| | `ui-prototype.html` | Linear 深色风格 HTML 原型（8 页面 + JS 桥接）| 用户交互 | 渲染的 DOM | `qwebchannel.js` |
| **基础设施** | `config.py` | Pydantic Settings、环境变量、默认配置 | `.env` / YAML | Settings 实例 | 无 |
| | `security.py` | API Key 加解密、数据库加密、密钥派生 | 明文 / 密文 | 安全存储对象 | cryptography, keyring |
| | `backup.py` | 定时备份、崩溃恢复、保留策略 | 数据库文件 | `.db.backup` | 无 |

## 2. 核心流程时序图

### 流程 A：对话式经历录入
```text
User → GUI(ExperienceView): 粘贴原始经历文本，点击"智能提取"
GUI → ExperienceManager.create_draft(raw_text)
ExperienceManager → LLMRouter: structured_extract(prompt+text, schema=ExperienceJSON)
LLMRouter --> ExperienceManager: 结构化字段(组织/职位/时间/成就/技能)
ExperienceManager: 基础校验(日期解析、必填项检查)
ExperienceManager → DB: INSERT experiences(status='draft')
ExperienceManager --> GUI: 返回 draft_id + 结构化表单
GUI: 展示可编辑表单，高亮置信度低字段
User → GUI: 修改/确认
GUI → ExperienceManager.confirm_experience(draft_id, user_edits)
ExperienceManager → DB: SELECT 现有经历(时间区间)
ExperienceManager: 冲突检测(起止时间重叠)
ExperienceManager → DB: UPDATE status='confirmed', 保存 skill_tags
ExperienceManager → SkillAnalyzer: normalize_skills(raw_skills)
SkillAnalyzer --> ExperienceManager: 标准技能节点 ID 列表
ExperienceManager → DB: 关联经历-技能
ExperienceManager --> GUI: 保存成功 + 刷新时间线
```

### 流程 B：简历一键生成
```text
User → GUI(ResumeView): 选括 Persona，点击"生成简历"
GUI → PersonaEngine.generate_resume(persona_id)
PersonaEngine → DB: 加载 Persona 配置 + 角色权重
PersonaEngine → ExperienceManager: 查询 confirmed 经历(含技能)
ExperienceManager --> PersonaEngine: List[Experience]
PersonaEngine: 规则计算 Fit Score(关键词匹配度 × 角色能力权重)
PersonaEngine: 筛选(score ≥ threshold) → 按时间倒序排序
loop 每个入选经历
    PersonaEngine → LLMRouter: reframe_experience(original_desc, persona_prompt, tone)
    LLMRouter --> PersonaEngine: reframed_summary
    PersonaEngine: 缓存到 role_experience_weights (不覆盖原始描述)
end
PersonaEngine --> ResumeBuilder: assemble(resume_data=[经历+重述+分数])
ResumeBuilder → DB: 加载 Jinja2 模板(按模板 ID)
ResumeBuilder: 渲染 Markdown(中文字体/排版处理)
ResumeBuilder: Markdown → PDF (asyncio.to_thread + weasyprint)
ResumeBuilder --> GUI: 返回 PDF 路径 + Markdown 原文
GUI: 左侧预览 Markdown，右侧提供"下载 PDF"
```

### 流程 C：岗位智能匹配
```text
User → GUI(JobMatchView): 粘贴 JD 文本 / 输入招聘 URL
GUI → JobMatcher.analyze_job(input, persona_id)
alt 输入为 URL
    JobMatcher → Playwright: new_context(stealth=True) → goto(url) → extract_text()
    Playwright --> JobMatcher: raw_html/text
    JobMatcher: 清洗正文(去除导航/页脚)
else 输入为文本
    JobMatcher: 直接使用
end
JobMatcher → LLMRouter: parse_jd(text) → 提取 structured_skills, requirements, years
LLMRouter --> JobMatcher: JSON(技能/要求/职责)
JobMatcher → SkillAnalyzer: normalize_skills(jd_skills)
SkillAnalyzer --> JobMatcher: 标准技能节点列表
JobMatcher → PersonaEngine: get_persona_skills(persona_id)
PersonaEngine --> JobMatcher: 角色当前技能集合
JobMatcher: **匹配度 = 技能匹配(基础×40 + 等级加成×10) + 经验匹配(年限满足×15 + 时间衰减×10) + 文本相似度(TF-IDF余弦×15) + 其他(学历+地点×10)**
JobMatcher: Gap = JD要求 - Persona现有技能
JobMatcher → DB: INSERT job_descs + job_matches (含 score_breakdown)
JobMatcher --> GUI: match_score%, matched[], missing[], breakdown{}
GUI → SkillAnalyzer: get_radar_data(persona_skills, jd_skills, dimensions≥5)
SkillAnalyzer --> GUI: 雷达图数据集
GUI: 渲染 PyQtGraph/ECharts 雷达图，展示匹配/缺失列表
```

## 3. 状态机设计

### 3.1 经历状态机
```text
[draft] --用户确认--> [confirmed]
[draft] --用户放弃--> [discarded]
[confirmed] --编辑--> [confirmed] (version += 1, 保留历史)
[confirmed] --删除--> [archived]
[archived] --恢复--> [confirmed]
```

### 3.2 岗位追踪状态机
```text
[new] --标记感兴趣--> [interested]
[new] --已投递--> [applied]
[interested] --已投递--> [applied]
[applied] --收到面试--> [interviewing]
[interviewing] --拿到offer--> [offered]
[interviewing] --流程结束--> [rejected]
[applied] --长期无回应--> [ghosted] (手动标记)
[offered] --接受--> [accepted]
[offered] --拒绝--> [declined]
```

### 3.3 简历渲染任务状态
```text
[pending] --开始渲染--> [rendering]
[rendering] --成功--> [completed]
[rendering] --失败--> [failed]
[failed] --用户重试--> [rendering]
```

## 4. 错误处理策略

| 层级 | 策略 | 实现细节 |
|------|------|----------|
|| **GUI 层** | 全局免底 + 异步信号 | `sys.excepthook` 捕获未处理异常 → 写日志 + `QMessageBox.critical`；所有 `async` 任务通过 `pyqtSignal` 返回 `(result, error)` 元组，禁止后台异常直接崩溃主循环 |
|| **文件导入层** | 多层次错误处理 + 用户友好提示 | PDF/Word解析失败(`ImportError`) → 显示具体安装命令；LLM解析失败(`ImportParserError`) → 显示"内容格式异常或网络问题"；文件后缀大小写不敏感 → `path.suffix.lower()`统一处理；无效条目自动跳过，不阻断整体导入 |
|| **服务层** | 自定义异常树 + 降级 | 基类 `CareerCraftError`，子类：`ValidationError`(400)、`LLMError`(503)、`ScraperError`(502)、`DatabaseError`(500)、`RenderError`(500)、**`ImportParserError`(解析失败)** |
| **LLMRouter** | 指数退避重试 + 多供应商降级 | 单模型失败重试 3 次(backoff: 1s, 2s, 4s)；主模型超时 → 自动切 Fallback；最终失败返回 `LLM_ALL_FAILED` |
| **爬虫层** | 超时隔离 + 手动降级 | Playwright 页面超时 10s，浏览器 context 级隔离，抓取失败时提示用户粘贴文本 |
| **数据层** | 事务回滚 + WAL 恢复 | SQLAlchemy `begin()` 包裹写入，异常自动 `rollback`；SQLite 启用 WAL 模式 |
| **崩溃保护** | 定时快照 | 每 5 分钟或关键操作后自动 `VACUUM INTO` 备份；保留最近 10 个 `.backup` 文件 |

## 5. 安全设计

| 维度 | 方案 |
|------|------|
| **API Key 存储** | 优先使用系统 keyring；不可用时回退至本地文件，采用 **Fernet (AES-128-CBC + HMAC)** 加密，密钥由 `PBKDF2HMAC(machine-id + 用户主密码)` 派生。首次启动强制设置主密码。 |
| **数据库加密** | 第一层：系统级全盘加密；第二层：应用级对敏感字段(原始经历描述、JD 原文)做 AES-256-GCM 列级加密；第三层(未来)：SQLCipher。 |
| **LLM 幻觉防御** | 1) 结构化强制：JSON Schema + Pydantic 校验；2) 不可变源数据：关键字段禁止 LLM 改写；3) 版本隔离：`reframed_summary` 独立存储；4) 人工确认环；5) 溯源标注。 |
| **爬虫合规** | 仅抓取用户主动提供的单个 URL；Playwright stealth 降低反爬触发；不存储完整网页快照；内置 2s 延迟。 |

## 6. 性能考量

| 方面 | 策略 |
|------|------|
|| **异步架构** | GUI 主线程保持 60fps；所有服务调用通过 `asyncio.create_task` + Signal 回调返回；数据库 `aiosqlite` 连接池 size=5；HTTP `httpx.AsyncClient` 全局复用连接池 | **Sprint 9-10: WebView + QWebChannel 同步桥接**，Bridge 层通过 `AsyncRunner.run()` 将异步 Service 转换为同步 JSON 返回给 JS。无需 `QThread` 或 qasync。 |
|| **缓存策略** | LLM 缓存：prompt+text 的 SHA256 为 key，结果缓存 7 天；重述缓存：`(experience_id + persona_id + prompt_version)` → 缓存；模板缓存：Jinja2 默认编译缓存；技能映射缓存：LRU(1000) | 增加 `ImportParser` 缓存：文件分析结果按 `(content_hash + prompt_version)` 缓存 1天 |
| **大数据量/渲染** | 经历库按年份分页(LIMIT 50)；PDF 生成投递到 `QThreadPool` 或 `asyncio.to_thread`；Playwright browser 懒加载，空闲 5 分钟自动关闭 |
| **数据库性能** | WAL 模式支持读写并发；`experiences(start_date, end_date)` 联合索引；`skill_nodes(name, aliases)` 索引 |

## 7. 扩展性设计

| 层面 | 预留方案 |
|------|----------|
| **数据层** | 所有核心表预留 `user_id: str` 字段，默认值 `"default"`；多用户时通过文件隔离：`/data/<user_id>/careercraft.db` |
| **服务层** | 服务类采用依赖注入模式，`__init__(self, db_session, user_context)`；后续扩展无需重构业务逻辑 |
| **GUI 层** | 主窗口预留登录/切换入口；当前自动以 `default` 登录 |
| **后端化** | 服务层与 GUI 层通过清晰接口分离；未来可无痛抽取为 FastAPI 后端，PySide6 前端仅保留 HTTP client + UI |
