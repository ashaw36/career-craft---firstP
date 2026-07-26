# ADR-001：Tauri 2 + Rust 本地核心

## 决策

生产版采用 Tauri 2、系统 WebView2、Vite/TypeScript 前端和 Rust 分层单体。生产产物不得包含 Python、Qt 或 Playwright Chromium。

```text
frontend feature modules
  → generated typed client
  → Tauri command / task events
  → application use cases
  → domain
  → infra: SQLite / LLM / Credential Manager / documents / crawler
```

## 数据

继续使用 `~/.careercraft/career.db`。首次启动先文件锁与备份，再完整性检查、事务迁移、二次检查；失败进入只读恢复页。数据库实体不得直接暴露到前端。

## 技术候选

- Tauri 2、tokio、serde、thiserror、uuid、chrono、validator。
- rusqlite + WAL + 编号 SQL migration。
- reqwest + 流式响应 + CancellationToken。
- specta/tauri-specta；不稳定则 JSON Schema codegen。
- minijinja、zip/quick-xml、PDF 候选库；PDF 输出优先 WebView2 print-to-PDF。
- keyring crate 调用 Windows Credential Manager。
- tracing 滚动脱敏日志和 crash marker。

## Go/No-Go

全面迁移前必须通过：旧库无损升级、经历→角色→简历→JD 垂直切片、中文文档语料、Win10/11 安装发布四项 PoC。Rust 文档能力不达标则评审 .NET NativeAOT 文档模块或整体 C# 备选。

