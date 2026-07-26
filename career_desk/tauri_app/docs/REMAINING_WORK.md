# CareerCraft 剩余工作与准出清单

更新时间：2026-07-22。本文只记录仍需完成或仍缺证据的事项。功能存在、单元测试通过和公开发布准出是三个不同层级。

## 当前已验证基线

- Rust/Tauri：`cargo test --all-features --offline`，171 passed，0 failed。
- 前端：23 files、114 tests，0 failed；生产构建通过。
- 安全静态门禁：`scripts/w8-security-gate.mjs`，12/12。
- 生产隔离：release 不包含 WDIO capability/plugin/devtools；隔离脚本通过。
- W7 后端性能与强杀恢复：7/7；不能替代真实桌面性能。

## P0：内部实现收口

- [x] W9 经历历史与 portable backup 修复后的全量回归及独立架构复审 Go（169/169）。
- [ ] 为每次门禁运行保存带时间、提交 SHA、平台和命令的机器可读报告；不能只在 Markdown 写测试数。
- [ ] 保持 63 个 command 契约、Rust/Vitest、安全、构建和生产隔离全部为绿；任一失败立即阻断。

## W6：真实桌面功能验收（公开发布阻断）

- [ ] 在真实 Tauri/WebView2 中跑通 `tests/desktop-e2e/journeys.json` 的 12 条旅程并产生 JUnit、截图/日志和 selector-ready 证据。
- [ ] 当前官方外置 `tauri-driver` 路径在 WebView2 150 报 `DevToolsActivePort file doesn't exist`；嵌入式 WDIO 能创建 session/window，但首次 DOM/execute 超时。此项为已复现阻塞，不得写成通过。
- [ ] 验证 Credential Manager、原生文件选择器、系统浏览器外链、真实 PDF/DOCX 文件及重启后持久化。

## W7：真实桌面性能（公开发布阻断）

- [x] 后端固定输入门禁：冷启动数据库、1000 经历、角色切换、JD、预览、CRUD、WAL 强杀恢复共 7/7。
- [ ] W6 提供同一二进制至少 5 次 selector-ready 样本后，运行 `scripts/w7-desktop-gate.ps1`。
- [ ] 取得冷启动 P50/P95 和 CareerCraft + WebView2 renderer/GPU 完整进程树空闲/峰值内存；窗口句柄或 `Responding` 不算页面就绪。

## W8：签名、更新与 Windows 发布（公开发布阻断）

- [ ] 配置自有 HTTPS updater endpoint、Minisign 密钥、受保护 release secrets。
- [ ] 配置可信 Authenticode 证书与时间戳；验证应用和安装器签名、发布 provenance。
- [ ] 提供上一个独立验证的已签名安装包，完成真实升级失败后的应用与数据库回滚。
- [ ] 在干净 Win10/11 普通用户 VM 验证一次安装、图标启动、覆盖升级、卸载、数据保留。
- [ ] 验证中文路径、离线、WebView2 缺失/损坏、SmartScreen、Defender。

## 安全与人工签核

- [ ] 在真实 Windows 用户上下文验证 API Key 仅进入 Credential Manager，响应、日志、数据库均不回显明文。
- [ ] 对五套简历的中文 PDF、Markdown 和真实 DOCX/PDF 导入做视觉/内容人工签核并归档证据。
- [ ] 发布说明明确：业务 SQLite 与 portable backup 当前不加密；哈希仅证明完整性，不证明来源真实性。

## 明确延期，不阻断当前冻结范围

- Gap 雷达图已由 matched/missing 技能和技能详情取代。
- 主题切换、模板字体/颜色、自定义模板与收藏。
- 面试准备、模拟面试、薪资洞察。
- 社区与模板市场。

延期项不得在 UI 中伪装为可用功能；重新纳入时必须新增需求 ID、验收标准和测试。

## 准出规则

内部 Preview 要求：171 Rust、114 frontend、security 12/12、build/isolation 全绿，W9 独立复审 Go；当前已满足。

公开发布额外要求：W6 12/12 真实桌面旅程、W7 桌面性能、W8 签名更新/回滚、Win10/11 VM 矩阵和系统级安全/文件签核全部通过。在此之前公开发布结论固定为 **No-Go**。
