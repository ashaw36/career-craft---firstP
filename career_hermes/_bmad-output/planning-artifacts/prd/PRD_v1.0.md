# CareerCraft Agent — PRD 需求文档 v1.0

> **For Hermes:** 本 PRD 供编码代理（Codex/Claude Code）执行使用。任何不清晰之处，代理应终止并提问。  
> **对应分析:** `../phase1/01-market-research.md`, `02-tech-research.md`, `03-requirements-analysis.md`

---

## 1. 产品定位与价值主张

**CareerCraft Agent** 是一个**角色档案驱动的个人职业智能体**，运行于本地桌面。它将你的职业经历视为**可组装的数据资产**，通过「角色档案」这一核心抽象，实现一套经历 → 多角色 → 多份简历 → 岗位匹配 → 技能 Gap 分析 → 学习路径推荐 → 能力进化追踪的完整闭环。

**一句话定位:**  
> "CareerCraft 是你职业生涯的 Git 仓库 —— 一套经历，无限角色，智能进化。"

---

## 2. 锁定决策（Decision Lock）

| 编号 | 决策 | 选择 | 不可逆理由 |
|------|------|------|-----------|
| **D1** | 产品形态 | B. 桌面应用（PySide6 GUI → **WebView 方案**）| 个人自用工具，数据本地化优先；Sprint 9-10 删除原生 GUI，仅保留 WebView 降低维护成本 |
|| **D2** | 目标用户 | A. 个人自用 | MVP 只服务单一用户，架构预留多用户扩展口 |
|| **D3** | 数据来源 | 简历自动生成(A) + 岗位爬虫(B) + 学习素材实时搜索(B) | 经历库从0构建，岗位依赖爬虫（Boss/猎聘），学习素材基于技能Gap动态检索 |
|| **D4** | LLM 策略 | 通义千问为主，支持多模型切换 | 通义千问 API 已确认可用，成本低、中文效果好 |
|| **D5** | 角色引擎 | 规则+Prompt 混合方案（非纯端到端大模型）| 个人工具需可解释性和可控性，规则引擎保留用户干预能力 |
|| **D6** | Sprint 11 范围 | **B. PDF 真实导出 + Fit Score 手动覆盖 + 经历重述编辑** | 补齐 P0/P1 前端缺口的

---

## 3. 功能需求

### 3.1 P0 — MVP 核心（必须可用）

#### FR-P0-01 对话式经历录入
- **状态: ✅ 已实现** (Sprint 2)
- **描述:** 用户以自然语言描述一段工作/项目经历，系统调用 LLM 将其结构化为字段（公司、岗位、时间、成就点、技能标签），展示给用户确认/修正。
- **验收标准 (AC):**
  - AC-001: 支持中文/英文自然语言输入 ✅
  - AC-002: 结构化结果必须包含：组织名称、职位/项目名、起止时间、核心成就（≥1 条）、技能标签（≥1 个） ✅
  - AC-003: 用户可以在确认前编辑任何字段 ✅
  - AC-004: 用户可以拒绝重来 ✅
  - AC-005: 原始描述始终保留，不被 LLM 覆盖 ✅

#### FR-P0-02 经历库基础管理
- **状态: ✅ 已实现** (Sprint 2)
- **描述:** 对已录入的经历进行增删改查，以时间线形式展示。
- **验收标准:**
  - AC-006: 支持经历的删除和手动编辑 ✅
  - AC-007: 时间线视图按时间倒序排列 ✅
  - AC-008: 重叠时间段自动检测并警告 ✅

#### FR-P0-03 角色档案创建与切换
- **状态: ✅ 已实现** (Sprint 2)
- **描述:** 用户可以创建多个角色档案，每个角色包含身份定位、目标岗位、能力侧重。系统支持在角色间快速切换。
- **验收标准:**
  - AC-009: 支持创建≥4个角色档案 ✅
  - AC-010: 每个角色必须包含：角色名称、身份定位语句、目标岗位类型、能力侧重配置 ✅
  - AC-011: 角色切换时，全局界面等待 ≤3 秒 ✅
  - AC-012: 删除角色不影响底层经历库 ✅

#### FR-P0-04 经历-角色适配（Fit Score）
- **状态: ✅ 已实现** (Sprint 2, 11)
- **描述:** 系统根据角色的能力侧重和目标岗位画像，为每条经历计算适配度分数（0–1.0）。
- **验收标准:**
  - AC-013: 适配度计算基于关键词匹配 + 角色权重简单规则 ✅
  - AC-014: 同一条经历在不同角色下可以有不同分数 ✅
  - AC-015: 用户可手动覆盖自动计算的分数 ✅ (Sprint 11: 角色页滑块 0-100 + "已手动调整"标记)

#### FR-P0-05 简历一键生成
- **状态: ✅ 已实现** (Sprint 3, 11)
- **描述:** 基于当前激活角色，自动筛选经历、排序、调整描述，输出一份简历。
- **验收标准:**
  - AC-016: 简历自动排除与角色冲突的经历 ✅
  - AC-017: 经历按 Fit Score 降序排列 ✅
  - AC-018: 支持导出 Markdown 和 PDF 格式 ✅ (Sprint 11: PDF 真实导出接入 `pdf_exporter.py`)
  - AC-019: 生成时间 ≤30 秒（单模型情况下） ✅

#### FR-P0-06 单模型 LLM 集成
- **状态: ⚠️ 部分实现** (Sprint 1, 4)
- **描述:** 接入通义千问 API，实现端到端的对话、经历结构化、简历生成。
- **验收标准:**
  - AC-020: 支持通义千问 qwen-max / qwen-plus 模型 ✅
  - AC-021: API 调用失败时显示友好错误信息 ✅
  - AC-022: 支持流式输出（逐字显示） ⚠️ 未实现（当前非流式）

#### FR-P0-07 本地数据持久化
- **状态: ⚠️ 部分实现** (Sprint 1)
- **描述:** 所有用户数据存储在本地 SQLite 数据库，应用关闭后数据不丢失。
- **验收标准:**
  - AC-023: 数据库文件存储在用户主目录（`~/.careercraft/`） ✅
  - AC-024: 支持数据库迁移/备份功能 ⚠️ 未实现
  - AC-025: 应用崩溃后数据完整性不受损害 ✅（WAL 模式）

### 3.2 P1 — 核心体验（完整闭环）

#### FR-P1-01 多模型 LLM 切换
- **状态: ⚠️ 部分实现** (Sprint 4)
- **描述:** 在设置中配置多个 LLM 供应商（通义千问、OpenAI、Claude），系统根据配置自动降级。
- **验收标准:**
  - AC-026: 支持≥3个供应商配置 ✅
  - AC-027: 主模型不可用时，自动切换到备用模型 ✅
  - AC-028: 支持按角色设置默认模型 ⚠️ 字段 `preferred_model` 存在但未使用

#### FR-P1-02 对话式简历调优
- **状态: ⚠️ 部分实现** (Sprint 9-10)
- **描述:** 用户可以通过自然语言指令调整已生成的简历，如"强化领导经验"、"增加数据"。
- **验收标准:**
  - AC-029: 支持≥5种常见调优指令 ✅
  - AC-030: 调优结果在原简历上增量更新，可恢复 ⚠️ 可恢复未实现
  - AC-031: 每次调优后用户必须确认 ✅

#### FR-P1-03 JD 粘贴与匹配分析
- **状态: ✅ 已实现** (Sprint 4-5)
- **描述:** 用户粘贴岗位 JD 文本，系统解析关键要求并与当前角色能力进行匹配度计算。
- **验收标准:**
  - AC-032: 支持中英文 JD 解析 ✅
  - AC-033: 输出整体匹配度百分比（0–100%） ✅
  - AC-034: 列出匹配的技能和不匹配的技能 ✅

#### FR-P1-04 技能 Gap 可视化
- **状态: ✅ 已实现** (Sprint 9-10)
- **描述:** 将角色当前能力与目标岗位要求进行对比，以雷达图形式展示。
- **验收标准:**
  - AC-035: 支持≥5个维度的雷达图 ✅
  - AC-036: 点击某个技能可展示详情 ✅

#### FR-P1-05 经历重述引擎
- **状态: ✅ 已实现** (Sprint 4, 7-8, 11)
- **描述:** 根据 JD 自动重述经历，用户可编辑/重置，不覆盖原始。
- **验收标准:**
  - AC-037: 自动重述经历 ✅
  - AC-038: 用户可编辑/重置重述结果 ✅ (Sprint 11: 岗位匹配详情页添加编辑/textarea/重置按钮)
  - AC-039: 重述不覆盖原始经历数据 ✅

#### FR-P1-06 简历模板多样化
- **状态: ✅ 已实现** (Sprint 5, 7)
- **描述:** 提供多套简历模板供用户选择。
- **验收标准:**
  - AC-040: 提供≥3套模板（技术简洁 / 产品故事 / 综合型） ✅ 实际 5 套（modern/classic/minimal/tech/外企）
  - AC-041: 模板可切换，切换后简历内容自动重新渲染 ✅

#### FR-P1-07 技能图谱基础
- **状态: ⚠️ 部分实现** (Sprint 9-10)
- **描述:** 建立预置技能节点库，支持层级关系和别名。
- **验收标准:**
  - AC-042: 预置≥50 个常见技能节点 ✅ 实际 51 节点
  - AC-043: 支持用户自定义添加技能 ⚠️ 未实现
  - AC-044: 支持技能别名匹配（如 "Python3" → "Python"） ✅

#### FR-P1-08 岗位保存与追踪
- **状态: ✅ 已实现** (Sprint 5)
- **描述:** 用户可保存感兴趣的岗位信息，记录投递/面试状态。
- **验收标准:**
  - AC-045: 支持保存粘贴的 JD 原文 ✅
  - AC-046: 支持标记状态：感兴趣 / 已投递 / 面试中 / 已拒绝 / 已接受 ✅

### 3.3 P2 — 增强智能（差异化）

#### FR-P2-01 智能学习路径推荐
- **状态: ✅ 已实现** (Sprint 5, 9-10, 12)
- **描述:** 基于 Gap 分析结果，推荐公开学习资源（课程、书籍、项目）。Sprint 12 补充 51 个技能节点真实学习资源，优先从 skill_graph 读取带真实链接的资源。
- **验收标准:**
  - AC-047: 每个 Gap 技能提供≥3条学习建议 ✅
  - AC-048: 学习资源包含标题、类型、时长估算、链接 ✅
  - AC-048-b: 链接为真实可访问的公开资源（GitHub/bilibili/官方文档等） ✅ (Sprint 12)
  - AC-048-c: 学习路径支持按来源分类（skill_graph / jd_gap / manual） ✅ (Sprint 12)

#### FR-P2-02 学习进度追踪
- **状态: ❌ 未实现**
- **描述:** 用户可标记学习项状态，完成后可更新到经历库。
- **验收标准:**
  - AC-049: 支持状态：未开始 / 学习中 / 已完成 ❌
  - AC-050: 已完成的学习项可一键添加到经历库 ❌

#### FR-P2-03 "假设分析"模拟
- **状态: ❌ 未实现**
- **描述:** 用户可运行"如果我掌握某技能，匹配度会提升多少"的模拟。
- **验收标准:**
  - AC-051: 支持添加/移除假设技能 ❌
  - AC-052: 实时更新模拟匹配度 ❌

#### FR-P2-04 简历 A/B 测试
- **状态: ❌ 未实现**
- **描述:** 支持同一角色下保存多个简历版本，并对比。
- **验收标准:**
  - AC-053: 支持≥5 个版本 ❌
  - AC-054: 版本间可对比差异 ❌

---

## 4. 用户旅程

### 旅程 1：首次使用（冷启动）
```
1. 用户打开应用 → 看到空经历库和默认"通用"角色
2. 点击"录入经历" → 用自然语言描述一段工作
3. LLM 结构化展示 → 用户确认/修正 → 保存
4. 重复 2-3 次，录入 3-5 条经历
5. 创建第一个专业角色（如"我是 AI PM"）
6. 系统自动计算经历与角色的适配度
7. 点击"生成简历" → 查看 Markdown 预览 → 导出 PDF
```

### 旅程 2：岗位匹配 + 简历修饰
```
1. 用户切换到目标角色（如 AI PM）
2. 粘贴一份目标岗位的 JD 文本
3. 系统解析 JD 并计算与角色的匹配度
4. 展示匹配的技能和不匹配的技能
5. 查看 Gap 可视化雷达图
6. 点击「✏️ 修饰简历以匹配此岗位」→ 系统自动为每条经历生成 JD 导向版本
7. 审阅修饰策略说明和修饰后的经历描述，确认后可用于简历生成
8. 保存岗位到追踪列表
```

### 旅程 3：角色切换
```
1. 用户已有 AI PM 角色，现在创建"销售经理"角色
2. 为新角色配置能力侧重（强调客户成功案例、成单能力）
3. 系统自动从同一经历库中重新计算适配度
4. 生成销售经理角色的简历（强调不同的成就点）
5. 对比两个角色的简历差异
```

### 旅程 4：能力提升
```
1. 用户在 Gap 分析中发现缺少 "K8s 部署经验"
2. 系统推荐学习资源（课程、文档、练习项目）
3. 用户标记为"学习中"
4. 完成学习后录入一个新项目经历
5. 系统自动更新技能图谱，匹配度提升
```

---

## 5. 数据模型

### 5.1 实体关系

```
User ──1:N──▶ Experience
User ──1:N──▶ Persona
Persona ──1:N──▶ RoleExperienceWeight
Persona ──1:N──▶ JobMatch
Persona ──1:N──▶ LearningPath
JobDesc ──1:N──▶ JobMatch
Experience ──N:M──▶ SkillNode
SkillNode ──自引用（parent_id）
```

### 5.2 核心表结构

**表: `experiences`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| user_id | TEXT | NOT NULL | 单用户场景下固定值 |
| type | TEXT | NOT NULL | work, project, education, certification |
| title | TEXT | NOT NULL | 职位或项目名 |
| organization | TEXT | | 公司/机构名 |
| start_date | DATE | | |
| end_date | DATE | | NULL=至今 |
| raw_description | TEXT | NOT NULL | 用户原始输入 |
| structured_achievements | JSON | | ["成就点1", ...] |
| skills_demonstrated | JSON | | ["skill_id_1", ...] |
| metrics | JSON | | [{name, value, unit}, ...] |
| created_at | DATETIME | DEFAULT now | |

**表: `personas`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| user_id | TEXT | NOT NULL | |
| name | TEXT | NOT NULL | e.g. "AI产品经理-平台方向" |
| is_default | BOOLEAN | DEFAULT FALSE | |
| identity_statement | TEXT | | 一句话定位 |
| career_narrative | TEXT | | 职业故事线 |
| tone_style | TEXT | | data_driven / business_insight / technical_deep |
| capability_weights | JSON | | {skill_id: weight, ...} |
| target_job_profiles | JSON | | ["意向岗位1", ...] |
| max_experiences | INT | DEFAULT 5 | 简历最多展示条数 |
| preferred_model | TEXT | | 默认使用的LLM模型 |

**表: `role_experience_weights`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| persona_id | TEXT | FK → personas | |
| experience_id | TEXT | FK → experiences | |
| relevance_score | REAL | DEFAULT 0.0 | 0.0~1.0 |
| reframed_summary | TEXT | | 针对角色的重述 |
| highlighted_skills | JSON | | [强调的技能ID列表] |
| user_overridden | BOOLEAN | DEFAULT FALSE | 用户是否手动调整过 |

**表: `skill_nodes`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| name | TEXT | NOT NULL | e.g. "Python" |
| category | TEXT | | technical / business / soft_skill / domain / tool |
| description | TEXT | | |
| parent_id | TEXT | FK → skill_nodes | 支持层级 |
| aliases | JSON | | ["Python3", "Py"] |
| vector_embedding | BLOB | | 可选，用于语义匹配 |

**表: `job_descs`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| raw_text | TEXT | NOT NULL | JD原文 |
| title | TEXT | | 岗位名称 |
| company | TEXT | | 公司名 |
| parsed_skills | JSON | | 解析出的技能需求 |
| source | TEXT | | manual / crawler_boss / crawler_liepin |

**表: `job_matches`**
|| 字段 | 类型 | 约束 | 说明 |
||------|------|------|------|
|| id | TEXT (UUID) | PK | |
|| persona_id | TEXT | FK | |
|| job_desc_id | TEXT | FK | |
|| match_score | INT | | 0~100 |
|| matched_skills | JSON | | [匹配的技能] |
|| missing_skills | JSON | | [缺失的技能] |
|| score_breakdown | JSON | | 分项得分报告 |
|| created_at | DATETIME | | |

**表: `job_match_experience_reframes`** *(Sprint 7-8 新增，JD导向经历修饰)*
|| 字段 | 类型 | 约束 | 说明 |
||------|------|------|------|
|| id | TEXT (UUID) | PK | |
|| job_match_id | TEXT | FK → job_matches (CASCADE) | 关联岗位匹配 |
|| experience_id | TEXT | FK → experiences (CASCADE) | 关联原始经历 |
|| original_summary | TEXT | NOT NULL | 原始经历摘要 |
|| reframed_summary | TEXT | NOT NULL | 修饰后的摘要 |
|| reframing_strategy | TEXT | | 修饰策略说明（供用户参考） |
|| created_at | DATETIME | DEFAULT now | |

**表: `learning_paths`**
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| persona_id | TEXT | FK | |
| target_gap | TEXT | | 目标补充的能力 |
| items | JSON | | [{resource, status, progress}, ...] |

**表: `uploaded_files`** *(Sprint 7-8 新增)*
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT (UUID) | PK | |
| user_id | TEXT | NOT NULL | |
| filename | TEXT | NOT NULL | 原始文件名 |
| file_type | TEXT | NOT NULL | Markdown/纯文本/JSON/PDF/Word |
| content_preview | TEXT | | 前500字符预览 |
| extracted_count | INT | DEFAULT 0 | LLM成功提取的经历条数 |
| status | TEXT | DEFAULT 'processed' | processed / failed |
| created_at | DATETIME | DEFAULT now | |

### 5.3 索引设计
- `experiences(user_id, end_date DESC)` — 时间线查询
- `role_experience_weights(persona_id, relevance_score DESC)` — 角色经历排序
- `skill_nodes(name)` — 技能查找
- `job_matches(persona_id, created_at DESC)` — 匹配历史

---

## 6. API 设计（内部层）

本项目为单体桌面应用，API 为内部模块间调用接口，非对外 HTTP。

### 6.1 ExperienceManager
```python
class ExperienceManager:
    async def create_from_text(self, raw_text: str) -> Experience:
        """自然语言 → LLM结构化 → 保存，返回待确认的结构化结果"""
    
    async def confirm_and_save(self, draft: ExperienceDraft) -> Experience:
        """用户确认后正式保存"""
    
    async def list_by_user(self, user_id: str) -> list[Experience]:
        """时间线排序列表"""
    
    async def update(self, exp_id: str, **fields) -> Experience:
    
    async def delete(self, exp_id: str) -> None:
```

### 6.2 PersonaEngine
```python
class PersonaEngine:
    async def create(self, name: str, identity: str, targets: list[str]) -> Persona:
    
    async def calculate_fit_scores(self, persona_id: str) -> list[RoleExperienceWeight]:
        """为所有经历计算适配度"""
    
    async def generate_resume(self, persona_id: str, template: str) -> Resume:
        """生成简历文档"""
    
    async def reframe_experience(self, persona_id: str, exp_id: str) -> str:
        """重新描述单条经历"""
```

### 6.3 JobMatcher *(Sprint 7-8 更新: 打分算法升级)*
```python
class JobMatcher:
    async def parse_jd(self, raw_text: str) -> JobDesc:
        """解析JD文本"""
    
    async def calculate_match(self, persona_id: str, job_desc_id: str) -> JobMatch:
        """计算匹配度，打分策略:
        - 技能匹配 50分(基础40+等级10)：精通×1.0/熟悉×0.6/了解×0.3/入门×0.1
        - 经验匹配 25分(年限15+衰减10)：3年内×1.0/3-5年×0.8/5年以上×0.6
        - 文本相似度 15分：简化TF-IDF+余弦相似度
        - 其他 10分：地点/年限要求/学历等
        - 返回 score_breakdown JSON 字段
        """
    
    async def delete_match(self, match_id: str) -> int:
        """删除岗位匹配记录，联级删除关联的修饰记录"""
    
    async def analyze_gap(self, match_id: str) -> GapAnalysis:
        """生成Gap分析"""
```

### 6.6 JDReframeEngine *(Sprint 7-8 新增，JD导向经历修饰)*
```python
class JDReframeEngine:
    async def reframe_experiences_for_job(
        self, match_id: str, force_refresh: bool = False
    ) -> list[JobMatchExperienceReframe]:
        """根据岗位JD要求，对角色关联的经历进行针对性修饰重写
        
        流程:
        1. 加载 JobMatch → 获取 persona_id + job_desc_id
        2. 加载角色经历（按 relevance_score 排序，限制8条）
        3. 对每条经历构建 JD 导向 Prompt（角色风格 + JD要求 + 原始经历）
        4. LLM 返回 JSON: reframed_summary + reframing_strategy
        5. 保存到 job_match_experience_reframes 表
        6. 支持缓存（force_refresh 可强制刷新）
        
        修饰原则:
        - 保留事实真实性，不编造不存在的事实
        - 突出与JD要求匹配的技能和经验
        - 使用更专业、更有影响力的表达方式
        - 不超过200字
        - 根据角色 tone_style 调整语气
        """
    
    async def get_reframed_experiences(self, match_id: str) -> list[JobMatchExperienceReframe]:
        """获取已修饰的经历列表"""
    
    async def delete_reframes(self, match_id: str) -> int:
        """删除指定岗位的所有修饰记录"""
    
    def _extract_json(self, text: str) -> dict:
        """从LLM返回中提取JSON，支持 markdown代码块/直接JSON/正则提取/回退"""
```

### 6.4 ImportParser *(Sprint 7-8 新增)*
```python
class ImportParser:
    async def parse_markdown(self, text: str) -> list[ExperienceDraft]:
        """Markdown格式 → 结构化经历草稿列表"""
    
    async def parse_text(self, text: str) -> list[ExperienceDraft]:
        """纯文本格式 → 结构化经历草稿列表"""
    
    async def parse_json(self, text: str) -> list[ExperienceDraft]:
        """JSON格式 → 结构化经历草稿列表"""
    
    async def analyze_file_with_llm(self, content: str, file_type: str) -> list[ExperienceDraft]:
        """PDF/Word等非结构化文件 → LLM自动分析提取经历
        
        鲁棒性处理:
        - LLM返回可能包含 markdown 代码块(\`\`\`json) → 自动提取 JSON 部分
        - 正则提取使用非贪婪匹配(\[.*?\]) 避免跨数组
        - 保留原始换行字符，不替换为空格
        - 对 LLM返回的每个条目进行校验: title 为 None/空 时跳过
        - 支持中英文字段名(title/标题, organization/公司, skills/技能等)
        
        日期解析支持格式:
        - YYYY-MM-DD, YYYY-MM, YYYY.MM, YYYY/MM, YYYY年MM月
        """
    
    class ImportParserError(Exception):
        """解析失败时抛出，包含具体错误信息"""
```

### 6.5 LLMRouter
```python
class LLMRouter:
    async def chat(self, messages: list[Message], model: str | None = None, 
                   stream: bool = False) -> str | AsyncIterator[str]:
        """统一对话接口，model=None 时使用默认模型"""
    
    def get_available_models(self) -> list[str]:
```

---

## 7. 架构与技术栈

### 7.1 运行时架构 *(Sprint 7-8 更新: qasync统一事件循环)*

```
┌───────────────────────────────────────────────────────────┐
│  PySide6 GUI 层 (主线程, qasync QEventLoop)              │
│  ┌───────────────┬───────────────┬───────────────┐   │
│  │ 对话面板       │ 经历管理器    │ 简历预览器    │   │
│  └───────────────┴───────────────┴───────────────┘   │
└───────────────┼───────────────────────────────┘
               │  Qt Signals / Slots + asyncio await
┌───────────────┴───────────────────────────────┐
│  服务层 (统一asyncio事件循环, 无需QThread)           │
│  ┌───────────────┬───────────────┬───────────────┐ │
│  │ ExperienceManager │ PersonaEngine    │ JobMatcher      │ │
│  └───────────────┴───────────────┴───────────────┘ │
│  ┌───────────────┬───────────────┬───────────────┐ │
│  │ ResumeBuilder     │ ImportParser     │ LLMRouter       │ │
│  └───────────────┴───────────────┴───────────────┘ │
└──────────────────┼──────────────────────────────┘
               │  aiosqlite (SQLite 异步驱动)
┌───────────────┴───────────────────────────────┐
│  持久化层                                             │
│  ┌─────────────────────────────────────────┐ │
│  │  ~/.careercraft/career.db  (SQLite 单文件)          │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘

                    ↑ 外部调用（HTTPS）
┌─────────────────────────────────────────────────────┐
│  外部服务                                              │
│  ┌───────────────┬───────────────┬───────────────┐ │
│  │ 通义千问 API       │ 猎聘/Boss (爬虫)  │ Web搜索API    │ │
│  └───────────────┴───────────────┴───────────────┘ │
└─────────────────────────────────────────────────────┘
```

**事件循环方案演进:**
- **旧方案 (Sprint 1-6):** `asyncio.run()` 启动异步初始化 → 完成后阻塞式启动 `QApplication.exec()`。GUI内部使用 `QThreadPool` 跑异步任务，通过 `pyqtSignal` 回调。
- **新方案 (Sprint 7-8):** `qasync.QEventLoop(app)` 统一桥接 Qt 与 asyncio。主线程直接 `await` 异步服务调用，无需 `QThread` 包装。窗口关闭时事件循环自动退出。

### 7.2 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| GUI | PySide6 (Qt6) | 主线程 GUI，工作线程处理耗时任务 |
| 异步框架 | asyncio + aiosqlite | 全栈异步，避免 UI 卡顿 |
| ORM | SQLAlchemy 2.0 (async) | 数据模型定义与查询 |
| HTTP 客户端 | httpx (async) | LLM API 调用 |
|| 爬虫 | Playwright + stealth | 岗位信息采集 |
|| 简历渲染 | Jinja2 + markdown + pdfkit/weasyprint | 模板渲染与导出 |
|| 配置 | Pydantic Settings | 环境变量 + YAML 文件 |
|| 打包 | PyInstaller | Windows 单 exe 输出 |
|| **事件循环桥接** | **qasync** | **Sprint 7-8 引入，统一 Qt 与 asyncio** |

### 7.3 项目目录结构

```
career-agent/
├── src/
│   ├── __init__.py
│   ├── main.py                 # 应用入口
│   ├── config/
│   │   ├── settings.py         # Pydantic Settings
│   │   └── llm_providers.yaml  # LLM 供应商配置
│   ├── models/
│   │   ├── database.py         # SQLAlchemy 引擎、会话
│   │   └── entities.py         # 所有实体定义
│   ├── services/
│   │   ├── experience_manager.py
│   │   ├── persona_engine.py
│   │   ├── job_matcher.py
│   │   ├── resume_builder.py
│   │   ├── skill_analyzer.py
│   │   └── learning_recommender.py
│   ├── llm/
│   │   ├── router.py           # LLM 路由器
│   │   ├── providers/          # 各供应商实现
│   │   └── prompts/            # Prompt 模板
│   ├── crawler/
│   │   ├── base.py
│   │   ├── boss_crawler.py
│   │   └── liepin_crawler.py
│   ├── ui/
│   │   ├── main_window.py      # 主窗口
│   │   ├── chat_panel.py       # 对话面板
│   │   ├── experience_view.py  # 经历管理视图
│   │   ├── persona_view.py     # 角色管理视图
│   │   ├── resume_preview.py   # 简历预览
│   │   └── job_match_view.py   # 岗位匹配视图
│   └── utils/
│       ├── validators.py
│       └── formatters.py
├── tests/
├── scripts/
├── requirements.txt
├── pyproject.toml
└── README.md
```

---

## 8. AI 引擎设计

### 8.1 LLM 使用场景

| 场景 | 模型优先级 | 说明 |
|------|-----------|------|
| 经历结构化 | qwen-max | 需高精度信息提取，中文优化 |
| 简历生成 | qwen-plus / gpt-4o | 需创意写作，可接受稍微便宜的模型 |
| JD 解析 | qwen-plus | 技能关键词提取 |
| 对话调优 | qwen-max | 需深度理解上下文 |
| Gap 分析 | qwen-plus | 结构化分析 |
| 学习推荐 | qwen-plus / 搜索API | 可结合搜索增强 |

### 8.2 Prompt 工程原则
- **事实层不可变：** 任何 Prompt 都必须包含 `raw_description` 字段，严禁 LLM 胡编事实
- **结构化输出：** 所有提取类任务强制输出 JSON，带 JSON Schema 验证
- **少示例：** 减少 few-shot 占用上下文，使用系统提示词优化

### 8.3 故障降级策略

```python
# 降级链
fallback_chain = [
    "qwen-max",       # 主力
    "qwen-plus",      # 成本低、速度快
    "gpt-4o",         # 国际备用
    "gpt-4o-mini",    # 极端情况
]
```

---

## 9. 部署架构

本项目为本地单机桌面应用，无需服务端部署。

**数据存储:**
- 数据库: `~/.careercraft/career.db`
- 配置文件: `~/.careercraft/config.yaml`
- 简历导出: `~/Documents/CareerCraft/`
- 日志: `~/.careercraft/logs/`

**打包交付:**
- PyInstaller 打包为单个 `CareerCraft.exe`
- 含 SQLite 实时复制备份功能
- 支持无网络状态下的本地操作（仅限经历管理和本地简历预览）

---

## 10. 开发里程碑

### Week 1 — 项目骨架与基础设施
| 天 | 任务 | P0 映射 | 产出 |
|----|------|---------|------|
| D1 | 搭建 PySide6 主窗口 + 页面导航 | — | main_window.py |
| D2 | 数据库设计 + SQLAlchemy 模型 | P0-7 | entities.py, database.py |
| D3 | Pydantic Settings + YAML 配置 | P0-7 | settings.py |
| D4 | LLM Router 框架 + 通义千问接入 | P0-6 | router.py, providers/ |
| D5 | 异步工作线程封装 + 测试 | — | 测试用例 |
| D6 | 对话面板 UI 设计 | P0-1 | chat_panel.py |
| D7 | Week 1 验收 | — | 可跑的空框架 |

### Week 2 — 核心流程（经历录入 + 角色管理）
| 天 | 任务 | P0 映射 | 产出 |
|----|------|---------|------|
| D1 | 经历结构化 Prompt 设计 + 实现 | P0-1 | prompts/ |
| D2 | ExperienceManager 实现 | P0-1,2 | experience_manager.py |
| D3 | 经历管理视图 + 时间线 | P0-2 | experience_view.py |
| D4 | 角色档案数据模型 + CRUD | P0-3 | persona_engine.py |
| D5 | 角色管理视图 + 切换 | P0-3 | persona_view.py |
| D6 | Fit Score 计算引擎 | P0-4 | 规则引擎 |
| D7 | Week 2 验收 | — | 可录入经历、可切换角色 |

### Week 3 — 简历生成 + 岗位匹配
| 天 | 任务 | P0/P1 映射 | 产出 |
|----|------|------------|------|
| D1 | 简历模板设计（Markdown + CSS） | P0-5 | templates/ |
| D2 | ResumeBuilder 实现 | P0-5 | resume_builder.py |
| D3 | 简历预览视图 + 导出 | P0-5 | resume_preview.py |
| D4 | JD 解析 Prompt + JobMatcher | P1-3 | job_matcher.py |
| D5 | 岗位匹配视图 + Gap 初始展示 | P1-3,4 | job_match_view.py |
| D6 | 经历重述引擎（P1） | P1-5 | persona_engine.py 增强 |
| D7 | Week 3 验收 | — | 可生成简历、可粘贴JD匹配 |

### Week 4 — P1 完善 + 打包
| 天 | 任务 | P1 映射 | 产出 |
|----|------|---------|------|
| D1 | 多模型路由完善 | P1-1 | router.py 增强 |
| D2 | 对话式简历调优 | P1-2 | chat_panel.py 增强 |
| D3 | 技能图谱预置 + 管理 | P1-7 | skill_nodes 初始化 |
| D4 | 简历模板扩展（3套） | P1-6 | 新模板 |
| D5 | 岗位保存追踪 | P1-8 | 追踪功能 |
| D6 | PyInstaller 打包测试 | — | CareerCraft.exe |
| D7 | 整体验收 + Bug 修复 | — | MVP 发布 |

---

## 11. 测试验收标准

### 11.1 功能测试
- [ ] P0-1: 输入一段中文经历描述，3 秒内返回结构化结果
- [ ] P0-3: 创建 2 个角色，切换时无卡顿
- [ ] P0-5: 基于角色生成简历，PDF 可正常打开
- [ ] P1-3: 粘贴 JD，10 秒内返回匹配度和 Gap 列表

### 11.2 性能测试
- [ ] 简历生成耗时 ≤30s（含 LLM API 调用）
- [ ] 数据库查询 1000 条经历 ≤500ms
- [ ] UI 响应时间 ≤500ms（本地操作）

### 11.3 安全测试
- [ ] 数据库文件加密存储（可选）
- [ ] API Key 存储不明文暴露
- [ ] 所有 LLM 生成内容需用户确认

---

## 12. 编码代理执行摘要

**产品定义:** 本地桌面职业智能体，角色档案驱动，从经历录入到简历生成、岗位匹配、学习推荐的完整闭环。  

**技术栈:** PySide6 + SQLAlchemy 2.0 + httpx + Playwright + SQLite。  

**入口任务:** 先实现 `src/main.py` 主窗口框架和 `src/models/` 数据库层，确保项目能跑起来。  

**环境依赖:** Python 3.11+, PySide6, SQLAlchemy[asyncio], aiosqlite, httpx, pydantic, pydantic-settings, Jinja2, markdown, playwright。  

**风险提示:**  
- 所有中文输出必须直接使用 UTF-8，禁用 `\uXXXX` 转义  
- PEP 585 内置泛型禁用，必须使用 `typing.List`, `typing.Optional`, `typing.Dict`  
- 所有异步操作必须经过 Qt 信号槽与主线程通信，严禁在主线程执行阻塞IO  
- LLM 调用必须有超时和故障降级处理

---

*PRD v1.0 — 锁定日期: 2026-07-17*
