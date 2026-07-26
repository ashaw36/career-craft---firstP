# CareerCraft Tauri 重构状态

更新时间：2026-07-22。

## 当前判定

- 内部实现：冻结的核心功能已基本完成，当前自动化基线为 Rust 171、frontend 23 files/114 tests、security 12/12，生产 build/isolation 通过。
- 内部 Preview：**Go**。W9 经历历史/portable backup 修复已通过全量门禁和独立架构复审。
- 公开发布：**No-Go**。W6 真实桌面 E2E、W7 桌面性能、W8 签名更新/回滚和 Win10/11 VM 矩阵尚未完成。

## 已完成并有自动化证据

- Tauri/Rust 本地架构、SQLite migration/WAL/失败恢复、63 command 契约。
- 经历、角色、Fit、五模板简历、调优/版本、JD/匹配/重述、技能/学习/What-if、岗位审计。
- LLM Provider 路由、retry/fallback、token 事件、取消、7 日缓存和安全错误分类。
- 结构化行业/学历、经历 revision、portable backup 基线、Updater adapter/recovery 基线。
- API Key 不回显、SSRF/URL 防护、CSP/capability/command 白名单、生产 WDIO 隔离。
- W7 后端固定输入性能与强杀恢复 7/7。

## 尚未准出

### W6 真实桌面

12 条旅程脚本已存在，但没有真实通过证据。外置 `tauri-driver` 在当前 WebView2 150 无法建立 session；嵌入式 WDIO 建立 session/window 后首次 DOM/execute 超时。jsdom/mock 不得替代此门禁。

### W7 桌面性能

后端门禁已通过。真实冷启动 P50/P95 和完整 CareerCraft/WebView2 进程树内存必须使用 W6 的 selector-ready 样本；当前被 W6 阻塞。

### W8 公开发布

仍缺自有 HTTPS updater endpoint、Minisign 私钥托管、可信 Authenticode/时间戳、独立验证的上一已签名包、真实失败回滚和干净 Win10/11 普通用户 VM 矩阵。SmartScreen、Defender、中文路径、离线及 WebView2 异常也未完成。

## 最新门禁表

| 门禁 | 状态 |
|---|---|
| Rust all-features | 171 passed |
| Frontend | 23 files、114 tests passed |
| Security gate | 12/12 |
| TypeScript/Vite build | 通过 |
| Production WDIO/devtools isolation | 通过 |
| W7 backend | 7/7 |
| W9 独立复审 | Go |
| W6 real desktop | 阻塞/未通过 |
| W7 desktop startup/memory | 被 W6 阻塞 |
| Signed updater + rollback | 未完成 |
| Win10/11 VM release matrix | 未完成 |

## 安全披露

API Key 设计为仅存 Windows Credential Manager。业务 SQLite 与 portable backup 当前不加密；SHA-256 仅用于完整性校验，不证明数据包来源。真实 Credential Manager 行为仍需 W6/VM 证据。

## 明确延期

Gap 雷达图已被 matched/missing 技能详情取代。主题/字体颜色/自定义模板、面试与薪资、社区与模板市场不属于当前冻结准出范围。

详细逐条证据见 `docs/REQUIREMENTS_TRACEABILITY.md`；剩余门禁见 `docs/REMAINING_WORK.md`。
