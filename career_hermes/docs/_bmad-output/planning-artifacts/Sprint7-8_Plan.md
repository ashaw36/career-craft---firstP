# CareerCraft Agent — Sprint 7-8 产品方案与开发计划

> 阶段代号：**真实闭环**（Real Data Loop）
> 北极星指标：**端到端完成一次"经历→简历→岗位匹配→Gap分析→学习推荐"全链路，且LLM输出质量可接受**

---

## 🔒 三大锁定决策（Decision Lock）

| 决策 | 选择 | 不可逆理由 |
|------|------|-----------|
| **D6 — 经历冷启动** | 支持 Markdown/JSON/文本批量导入，保留手动录入作为兜底 | 用户已有简历/笔记，从零手敲经历体验极差，批量导入是激活关键 |
| **D7 — 岗位获取** | Boss直聘优先（Playwright爬虫），手动粘贴保留 | Boss直聘岗位量最大、JD结构化最好，MVP只做一个平台 |
| **D8 — LLM验证策略** | 真实LLM为主链路，Mock模式作为网络异常fallback | 只有真实LLM才能验证Prompt质量和输出可用性，Mock仅用于开发调试 |

---

## 🎯 阶段目标

**Sprint 7（Week 13-14）：数据接入 + 端到端验证**
- 真实LLM接入，跑通完整链路
- 经历批量导入（解决冷启动）
- 修复端到端过程中暴露的集成bug

**Sprint 8（Week 15-16）：岗位爬虫 + 体验Polish**
- Boss直聘岗位自动抓取
- 岗位列表自动刷新
- 打包发布 v0.1-alpha

---

## 🗺️ 用户旅程（今晚验证路径）

```
┌─────────────────────────────────────────────────────────────────┐
│  Step 1: 经历导入（5分钟）                                       │
│  → 选择 "从Markdown导入"，粘贴现有简历/笔记                       │
│  → 系统自动解析为工作/项目/教育经历，进入草稿态                   │
├─────────────────────────────────────────────────────────────────┤
│  Step 2: 角色配置（2分钟）                                       │
│  → 创建 "AI产品经理" 角色，设置目标岗位关键词                     │
│  → 系统自动计算 Fit Score                                       │
├─────────────────────────────────────────────────────────────────┤
│  Step 3: 简历生成（1分钟）                                       │
│  → 选择角色 + modern模板 → 生成Markdown → 导出PDF               │
│  → PDF排版正确，中文无乱码                                      │
├─────────────────────────────────────────────────────────────────┤
│  Step 4: 岗位匹配（3分钟）                                       │
│  → 粘贴一个真实JD → 解析 → 匹配 → 查看匹配详情                   │
│  → 匹配度>60%时显示"高匹配"，<40%显示"需提升"                    │
├─────────────────────────────────────────────────────────────────┤
│  Step 5: Gap分析 + 学习路径（2分钟）                             │
│  → 系统自动识别技能Gap → 推荐学习资源（课程/文档/项目）          │
│  → 资源可点击跳转或标记"已学习"                                  │
└─────────────────────────────────────────────────────────────────┘
```

**验证标准：全程在GUI完成，无需终端操作，总耗时<15分钟**

---

## 📋 任务拆分

### Sprint 7 — 数据接入 + 端到端验证

#### 7.1 真实LLM接入验证（今晚做）
| 任务 | 文件 | 验收标准 |
|------|------|---------|
| 配置通义千问API Key | `settings.yaml` | `pytest tests/test_router.py -v` 通过真实请求 |
| 验证JD解析质量 | `src/services/job_parser.py` | 粘贴3个真实JD，解析准确率>80% |
| 验证经历重述质量 | `src/services/retelling_engine.py` | 重述后经历通顺、不丢失关键信息 |
| 验证简历生成质量 | `src/services/resume_builder.py` | 生成简历结构完整，无模板渲染错误 |
| 验证岗位匹配合理性 | `src/services/job_matcher.py` | 匹配分数与直觉相符，高匹配确实高 |

#### 7.2 经历批量导入服务 *(2026-07-18 完成)*

| 任务 | 文件 | 说明 | 状态 |
|------|------|------|------|
| 创建导入解析器 | `src/services/import_parser.py` | 支持Markdown/纯文本/JSON三种格式 | ✅ 已完成 |
| 解析工作经历 | `import_parser._parse_work()` | 识别公司、职位、时间段、描述 | ✅ 已完成 |
| 解析项目经历 | `import_parser._parse_project()` | 识别项目名称、技术栈、成果 | ✅ 已完成 |
| 解析教育背景 | `import_parser._parse_education()` | 识别学校、专业、学历、时间 | ✅ 已完成 |
| **文件导入Tab + LLM分析** | `experience_page.py` | **新增"文件"Tab，支持PDF/Word上传**，LLM自动分析提取经历 | ✅ 已完成 |
| **上传留痕** | `entities.py` | **新增 `uploaded_files` 模型**，保存文件名、类型、预览、提取数量、状态 | ✅ 已完成 |
| 冲突检测 | `experience_manager.py` | 导入时检测与现有经历的时间重叠 | 🔷 待实现 |

#### 7.3 岗位匹配打分优化 *(2026-07-18 完成)*

| 任务 | 文件 | 说明 | 状态 |
|------|------|------|------|
| **技能等级权重** | `job_matcher.py` | **匹配技能根据`capability_weights`解析等级**（精通×1.0/熟悉×0.6/了解×0.3） | ✅ 已完成 |
| **经验时间衰减** | `job_matcher.py` | **3年内×1.0，3-5年×0.8，5年以上×0.6**，加权年限替代原始年限 | ✅ 已完成 |
| **TF-IDF文本相似度** | `job_matcher.py` | **简化TF-IDF+余弦相似度**，简历文本 vs JD描述，15分 | ✅ 已完成 |
| 分项得分报告 | `job_matcher.py` | `score_breakdown` JSON字段，分项展示技能/经验/文本/其他 | ✅ 已完成 |

#### 7.4 事件循环改造 *(2026-07-18 完成)*

| 任务 | 文件 | 说明 | 状态 |
|------|------|------|------|
| **qasync统一事件循环** | `main.py` | **移除`asyncio.run()`+阻塞式`QApplication.exec()`**，改用`qasync.QEventLoop(app)` | ✅ 已完成 |

#### 7.5 端到端Bug修复（验证中发现）
- [x] 导入对话框中文引号转义导致语法错误（已修复，使用「」替换"" 在 f-string 中）
- [ ] 待填：根据今晚验证结果记录

### Sprint 8 — 岗位爬虫 + 体验Polish

#### 8.1 Boss直聘爬虫
| 任务 | 文件 | 说明 |
|------|------|------|
| Playwright爬虫基类 | `src/crawlers/base.py` | 封装浏览器启动、页面等待、反检测 |
| Boss直聘爬虫 | `src/crawlers/boss_zhipin.py` | 搜索关键词→列表页→详情页→解析JD |
| 爬虫调度器 | `src/crawlers/scheduler.py` | 定时抓取、去重、存储到JobDesc |
| UI爬虫配置 | `job_match_page.py` | 新增"自动抓取"面板：关键词+城市+频率 |

#### 8.2 体验Polish
| 任务 | 文件 | 说明 |
|------|------|------|
| 加载状态指示 | 全局 | 所有异步操作显示加载动画/进度条 |
| 错误友好提示 | 全局 | LLM超时/限流时给出具体建议（如"请检查网络"） |
| 首次启动引导 | `main_window.py` | 无数据时显示"快速开始"向导 |

#### 8.3 打包发布
| 任务 | 文件 | 验收标准 |
|------|------|---------|
| 运行build.py | `build.py` | 成功生成 `dist/CareerCraftAgent/` |
| 测试exe启动 | — | 双击exe正常启动，无终端弹窗 |
| 测试数据持久化 | — | 关闭重启后数据不丢失 |

---

## 🧪 验收标准（Definition of Done）

### Sprint 7 DoD
- [ ] 真实LLM跑通完整链路（经历→简历→匹配→Gap→推荐），输出质量可接受
- [ ] 经历批量导入支持Markdown/文本/JSON，准确率>70%
- [ ] 单元测试覆盖率不下降（保持63个通过）
- [ ] 新增集成测试：端到端链路至少1个

### Sprint 8 DoD
- [ ] Boss直聘爬虫可稳定抓取10+岗位（含JD解析）
- [ ] 打包后的exe在Windows上独立运行
- [ ] 所有异步操作有加载状态反馈
- [ ] v0.1-alpha 标签发布

---

## ⚠️ 风险与对策

| 风险 | 概率 | 影响 | 对策 |
|------|------|------|------|
| 通义千问API Key无效/额度不足 | 中 | 高 | 今晚先验证Key有效性；准备OpenAI备用Key |
| Boss直聘反爬升级 | 高 | 中 | Playwright模拟真人操作（随机延迟、UA伪装）；反爬严重时退回手动粘贴 |
| 经历导入解析准确率不足 | 中 | 中 | MVP只处理结构化较好的Markdown；复杂格式提示用户手动修正 |
| PyInstaller打包体积过大 | 中 | 低 | 使用UPX压缩；只打包必要依赖 |

---

## 📅 今晚验证Checklist

```
□ 1. 确认settings.yaml中LLM配置正确（base_url + api_key）
□ 2. 运行 python -m src.main 启动GUI
□ 3. 创建1个角色（AI产品经理）
□ 4. 手动录入2-3条真实经历
□ 5. 生成简历 → 检查Markdown质量 → 导出PDF检查排版
□ 6. 粘贴1个真实JD → 解析 → 匹配 → 查看详情
□ 7. 检查Gap分析结果是否合理
□ 8. 检查学习路径推荐是否有用
□ 9. 记录所有异常/bug/体验问题
□ 10. 截图关键页面（用于Notion备份）
```

---

## 📝 变更日志

- **2026-07-17** — Sprint 7-8 计划制定：确定"真实闭环"阶段目标，锁定3个决策，拆分14项任务
- **2026-07-18** — **重大更新：**
  - ✅ 经历导入增加"文件"Tab，支持PDF/Word上传 + LLM自动分析 + 上传留痕(`uploaded_files`表)
  - ✅ 岗位匹配打分算法升级：技能等级权重 + 时间衰减加权 + TF-IDF余弦相似度 + 分项报告
  - ✅ 异步架构改造：qasync统一事件循环，移除QThread混合方案
  - ✅ 单元测试 74 passed / 0 failed
  - ✅ GitHub 推送: `9dcea79`
- **2026-07-18 晚间** — **Bug修复与测试完善：**
  - 🔧 `严重` `main.py`: `aboutToQuit`信号为无参数信号，`future.set_result`缺参报错导致应用闪退 → 用`lambda: future.set_result(None)`包装
  - 🔧 `严重` `experience_page.py`: 文件后缀判断使用`endswith`大小写敏感 → Windows上`.PDF`/`.Docx`被误判为文本 → 改用`path.suffix.lower()`
  - 🔧 `中等` `experience_page.py`: PyMuPDF导入名错误(`import pymupdf`) → 正确为`import fitz`
  - 🔧 `中等` `import_parser.py`: JSON提取正则贪婪匹配可能跨数组、换行被替换破坏JSON → 改用非贪婪`'.*?]'`并保留换行
  - 🔧 `中等` `import_parser.py`: LLM返回`"title": null`时`item.get("title", "")`返回`None`而非`""` → 改用`item.get("title") or ""`并显式跳过无标题条目
  - 🔧 `中等` `import_parser.py`: 日期解析遗漏`YYYY-MM`格式(LLM常见输出) → 添加`"%Y-%m"`格式支持
  - 🔧 `轻微` `experience_page.py`: 文件分析失败时错误信息过于笼统("文件分析失败"五字) → 根据异常类型给出具体提示(LLM解析失败/缺少依赖/其他错误)
  - ✅ 新增`tests/test_file_import_e2e.py`：6个端到端测试覆盖文件分析全链路(LLM正常返回/markdown代码块/null标题跳过/完整入库/错误处理/大小写后缀)
  - ✅ 测试总数: 80 passed / 0 failed (原74→80)
  - ✅ GitHub 推送: `59a1847` + `b93310c` + `575da02` + `2b4a022` + `be72d7d`
- **2026-07-19** — **简历生成修复 + JD导向经历修饰 + 岗位删除**：
  - ❗ `修复` `modern.md.j2`: 模板缺少 `description` 字段渲染，导致经历内容未填充 → 添加 `{% if exp.description %}` 块
  - ⭐ `新增` `JobMatchExperienceReframe` 模型: `job_match_id` + `experience_id` + `original_summary` + `reframed_summary` + `reframing_strategy` + `created_at`
  - ⭐ `新增` `src/services/jd_reframe_engine.py`: JDReframeEngine 服务，核心方法 `reframe_experiences_for_job(match_id)`，策略:
    - 加载角色经历（按 relevance_score 排序，限制8条）
    - 对每条经历构建 JD 导向 Prompt（含角色风格、JD要求、原始经历）
    - LLM 返回 JSON `reframed_summary` + `reframing_strategy`
    - 自动存档到 `job_match_experience_reframes` 表
    - 支持缓存命中（force_refresh 可强制刷新）
  - ⭐ `新增` `job_match_page.py` UI: 匹配详情区增加「✏️ 修饰简历以匹配此岗位」按钮 + 修饰结果展示区
  - ⭐ `新增` 岗位删除功能: JD 列表操作列增加「删除」按钮，删除时联级删除关联的修饰记录和匹配记录
  - ✅ 新增 `tests/test_jd_reframe_engine.py`: 11个测试覆盖单条修饰/完整流程/缓存命中/强制刷新/获取删除/JSON提取鲁棒性
  - ✅ 测试总数: 91 passed / 0 failed (原80→91)
- **2026-07-19 晚间** — **WebView桥接层 + 简历Bug修复 + 岗位匹配增强**
  - ❯ 修复 `resume_builder.py` 经历为空 Bug：`min_score` 0.15→0.0 + 跨线程 ORM 字段复制避免 detached 状态
  - ❯ 新增 `parseJD` + `matchJob(job_desc_id, persona_id)` 拆分流程，支持角色选择后匹配
  - ❯ 新增 `src/ui/webview/api_handler.py` 岗位/匹配/修饰 API：list_jobs、delete_job（级联删除）、get_job_matches、updateMatchStatus、reframeResume、getReframeResults
  - ❯ 新增 `src/ui/webview/bridge.py` 6个 Qt Slot：parseJD、matchJob、listJobs、deleteJob、getJobMatches、updateMatchStatus、reframeResume、getReframeResults
  - ❯ `job_matcher.py` 新增 `list_matches_by_job()` 按岗位查询匹配
  - ❯ HTML原型 `ui-prototype.html` 岗位页完全动态化：JD粘贴区→解析匹配→动态卡片（分数环+删除）→匹配详情（状态更新）→修饰结果展示
  - ❯ 补齐 `test_resume_builder.py` 集成测试(3个) + `test_bridge.py` 签名适配 + 新增 e2e 测试：test_resume_e2e(2)、test_persona_e2e(2)、test_job_match_e2e(2)
  - ❯ 测试总数：**109 passed** (原91→109) | 代码量：~5,400 行
