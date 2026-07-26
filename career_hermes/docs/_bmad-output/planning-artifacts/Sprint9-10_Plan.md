# CareerCraft Agent — Sprint 9-10 开发计划

> 阶段代号：**体验闭环**（Experience Loop）
> 北极星指标：**完成所有 P0-P3 待办项，WebView 前端完全可用，用户无需终端即可跑通完整链路**

---

## 🔒 锁定决策（Decision Lock）

| 决策 | 选择 | 不可逆理由 |
|------|------|-----------|
| **D9** — WebView 优先 | WebView 版为主，原生 PySide6 保留作为备份 | WebView 开发效率更高，UI 统一性更好，且已投入大量桥接层代码 |
| **D10** — 体验优于功能 | P0 重点放在用户可感知的体验缺失，而非新功能 | 当前核心功能已齐备，体验缺失（加载状态、错误提示、引导）是阻止用户正常使用的核心障碍 |
| **D11** — 打包为必项 | Sprint 10 必须完成 PyInstaller 打包 | 没有可分发的二进制文件，项目无法交付给用户 |

---

## 🎯 阶段目标

**Sprint 9（Week 17-18）：P0 + P1 完成**
- P0: 经历冲突检测 + WebView 剩余页面动态化
- P1: 全局加载状态 + 错误友好提示 + 对话式简历调优

**Sprint 10（Week 19-20）：P2 + P3 + 打包**
- P2: 首次启动引导 + Boss直聘爬虫稳定化 + PyInstaller 打包 + 技能Gap雷达图
- P3: 技能图谱预置50节点
- 发布 v0.1-alpha

---

## 📄 任务拆分

### P0 — 经历冲突检测 + WebView 剩余页面

| 序号 | 任务 | 文件 | 验收标准 | 预估工时 |
|------|------|------|---------|---------|
| P0-1 | 经历时间冲突检测 | `experience_manager.py` | 导入/创建经历时，检测与现有经历的起止时间重叠，重叠时弹窗警告 | 4h |
| P0-2 | WebView 经历页动态化 | `ui-prototype.html` + `bridge.py` | 经历列表通过 `getExperiences` 动态渲染，支持编辑/删除 | 6h |
| P0-3 | WebView 角色页动态化 | `ui-prototype.html` + `bridge.py` | 角色列表通过 `getPersonas` 动态渲染，支持切换/编辑 | 4h |
| P0-4 | WebView 简历页动态化 | `ui-prototype.html` + `bridge.py` | 选择角色→生成简历→Markdown预览→PDF下载 | 6h |
| P0-5 | WebView 设置页 | `ui-prototype.html` + `bridge.py` | API Key配置、模型选择、主题切换 | 4h |

### P1 — 体验优化

| 序号 | 任务 | 文件 | 验收标准 | 预估工时 |
|------|------|------|---------|---------|
| P1-1 | 全局加载状态指示 | `ui-prototype.html` | 所有异步操作（生成简历/匹配/修饰）显示 Loading 动画 | 4h |
| P1-2 | 错误友好提示 | `ui-prototype.html` + `bridge.py` | LLM超时/限流时展示具体建议，不显示原始异常 | 4h |
| P1-3 | 对话式简历调优 | `resume_builder.py` + `conversation_engine.py` | 输入"强调领导力"→LLM生成增量修改→展示 diff→用户确认 | 8h |

### P2 — 进阶功能

| 序号 | 任务 | 文件 | 验收标准 | 预估工时 |
|------|------|------|---------|---------|
| P2-1 | 首次启动引导 | `ui-prototype.html` | 无数据时显示"快速开始"向导：导入/录入/创建角色 | 6h |
| P2-2 | Boss直聘爬虫稳定化 | `crawlers/boss_zhipin.py` | 搜索关键词→列表页→详情页→解析JD，稳定抓取≥10条 | 8h |
| P2-3 | 技能Gap雷达图 | `ui-prototype.html` | 岗位匹配页展示≥5维雷达图（Persona vs JD） | 8h |
| P2-4 | PyInstaller 打包 | `build.py` + 配置 | 生成 `dist/CareerCraftAgent.exe`，双击启动无终端弹窗 | 6h |

### P3 — 技能图谱

| 序号 | 任务 | 文件 | 验收标准 | 预估工时 |
|------|------|------|---------|---------|
| P3-1 | 预置50技能节点 | `skill_analyzer.py` | 初始化时加载≥50个预置技能节点（技术/业务/软技能/领域/工具） | 4h |
| P3-2 | 技能别名匹配 | `skill_analyzer.py` | "ReactJS"→"React"、"Python3"→"Python" 自动标准化 | 4h |

---

## 📋 验收标准（Definition of Done）

### Sprint 9 DoD
- [ ] WebView 五页面全部动态化，无硬编码静态内容
- [ ] 经历导入/创建时检测时间冲突，重叠时给出明确警告
- [ ] 所有异步操作有加载状态反馈
- [ ] 对话式简历调优可用（至少3种调优指令）
- [ ] 测试覆盖率不下降（109 个通过）

### Sprint 10 DoD
- [ ] Boss直聘爬虫可稳定抓取≥10条岗位
- [ ] 技能Gap雷达图在岗位页展示
- [ ] 首次启动引导可用
- [ ] PyInstaller 打包成功，exe 可独立运行
- [ ] 技能图谱预置50节点加载
- [ ] 测试总数 ≥ 115 个，全部通过
- [ ] v0.1-alpha 标签发布

---

## ⚠️ 风险与对策

| 风险 | 概率 | 影响 | 对策 |
|------|------|------|------|
| Boss直聘反爬升级 | 高 | 中 | Playwright stealth + 随机延迟；严重时降级为手动粘贴 |
| PyInstaller 打包失败 | 中 | 高 | 提前在 Week 19 开始打包调试，预留一周缓冲 |
| WebView 前端跨域问题 | 中 | 中 | QWebChannel 确保同源；如有问题回退到原生 PySide6 UI |
| LLM 调优提示质量不稳定 | 中 | 中 | 多次迭代 Prompt；提供"撤销"功能 |
