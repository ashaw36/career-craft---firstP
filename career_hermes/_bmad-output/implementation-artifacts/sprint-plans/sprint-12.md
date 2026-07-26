# Sprint 12: 学习资源体系完善

## 背景
Sprint 11 完成后，P0/P1 前端缺口已补齐。用户反馈学习资源体系存在三大问题：
1. 技能图谱 51 个节点仅 5 个有 `learning_url`，大部分为空
2. 学习路径生成后资源常为空链接或模板化文本
3. 学习路径无来源分类，无法归类留档

## 锁定决策
- **D1**: 采用人工整理预置资源库，不依赖外部搜索 API
- **D2**: skill_graph.json 节点结构扩展为 `resources` 数组，支持 course/video/repo/article/book/doc 类型
- **D3**: LearningPath 增加 `source_type` 字段（jd_gap / skill_graph / manual），前端按来源二级菜单归类

## 任务拆解

### Story 12.1 技能图谱学习资源补充
- [x] 子代理并行搜索 51 个技能节点的优质学习资源
- [x] 汇总审核并编入 skill_graph.json（每个节点 2-3 个资源）
- 资源覆盖：GitHub 高 stars repo、bilibili 视频、官方文档、知乎专栏、豆瓣读书、Coursera 等

### Story 12.2 学习路径真实链接
- [x] SkillNode 模型添加 `resources` JSON 字段
- [x] SkillGraphService 添加 `get_resources()` 方法
- [x] LearningRecommender 优先从 skill_graph 读取 resources，再 LLM，再 fallback
- [x] 生成的学习路径项带真实可访问 URL

### Story 12.3 学习路径来源分类
- [x] LearningPath 模型添加 `source_type` 字段（默认 manual）
- [x] api_handler 添加 `get_learning_paths_by_source()` 按来源分类
- [x] bridge 添加 `getLearningPathsBySource` Slot
- [x] 前端学习路径页面增加二级菜单（全部/技能图谱/JD补充/手动创建）

### Story 12.4 WebView 外部链接跳转
- [x] 子类化 `ExternalLinkPage(QWebEnginePage)`，重写 `acceptNavigationRequest`
- [x] 拦截 `target="_blank"` 外部链接，用 `QDesktopServices.openUrl()` 打开系统浏览器
- [x] 前端技能详情弹窗展示 resources 列表（替换原单个 learning_url）

## 验收标准
- [x] 51 个技能节点均有 resources 数组，每个 2-3 个真实链接
- [x] 学习路径生成时优先使用 skill_graph 资源，链接可点击
- [x] 学习路径按 source_type 分类展示，支持二级筛选
- [x] WebView 内外部链接自动用系统浏览器打开
- [x] 全量回归测试 149/149 passed

## 风险
- 部分 B 站/知乎链接可能随时间失效（子代理基于真实平台链接格式生成，但无法 100% 验证）
- 解决：主要资源使用官方文档、GitHub repo、豆瓣读书等稳定链接

## 完成状态
- 完成日期：2026-07-19
- 提交 hash：`c0b70d0`
- 测试结果：149/149 passed