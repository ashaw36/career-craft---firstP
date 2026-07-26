# Sprint 13 — LLM 配置自定义化

## 背景

当前设置页的 LLM 配置过于简际：
- 模型名下拉框只有3个固定选项（gpt-4o / claude-3-5-sonnet / qwen-turbo），无法输入其他模型
- 不支持自定义 `base_url`，无法接入 OpenAI 兼容 API 代理
- 后端 `save_settings` 只保存了单个 `preferred_model` 字符串，与实际使用的 `llm_providers` 数组配置脱节
- "应用偏好"卡片包含主题切换、默认经历数等未实现功能，占据空间且无实际价值

## 锁定决策

- **D1:** 设置页改为完整 Provider 编辑器：多 Provider 列表、切换编辑、添加/删除；模型名改为自由输入；支持自定义 `base_url`
- **D2:** 删除"应用偏好"卡片，设置页只保留 LLM 配置
- **D3:** API Key 不回显，留空则保留原值；保存时清空 `_settings` 缓存保证即时生效

## 任务拆解

| 编号 | 任务 | 文件 | 状态 |
|------|------|------|------|
| 13.1 | 后端：新增 `get_settings()` 方法，返回脱敏后的 Provider 列表 | `api_handler.py` | ✅ |
| 13.2 | 后端：重写 `save_settings()` 方法，全量写入 `llm_providers` + `default_llm_provider` | `api_handler.py` | ✅ |
| 13.3 | Bridge：新增 `getSettings` Slot | `bridge.py` | ✅ |
| 13.4 | 前端：删除"应用偏好"卡片，单列布局 | `ui-prototype.html` | ✅ |
| 13.5 | 前端：改造 LLM 配置卡片为 Provider 编辑器 | `ui-prototype.html` | ✅ |
| 13.6 | 前端：新增 `loadSettings()` 回显 + `addNewProvider()` / `removeCurrentProvider()` / `syncCurrentProviderFromForm()` | `ui-prototype.html` | ✅ |
| 13.7 | 前端：重写 `saveSettings()` 提交完整 Provider 数据 | `ui-prototype.html` | ✅ |

## 验收标准

1. 设置页不再显示"应用偏好"卡片
2. LLM 配置卡片支持：多 Provider 列表、切换编辑、添加/删除 Provider
3. Base URL 可自由输入，模型名可自由输入（text input）
4. API Key 输入框 placeholder 为"留空则保留原值"，不回显已保存的 key
5. 保存后配置写入 `~/.careercraft/config.yaml` 的 `llm_providers` 数组
6. 页面加载时能回显当前配置
7. `.venv/bin/pytest tests/ -q` 全部通过（149 个测试）

## 完成记录

- 完成日期：2026-07-19
- 提交 hash：`63bf2a6`
- 测试结果：149 passed in 17.57s

## Patch 2026-07-20 — SecureStorage 链路打通

### 问题

- `save_settings` 存储 API Key 时，SecureStorage 在无 keyring/cryptography 环境下抛出 `RuntimeError`，后端 try/except 只打 warning 后继续，导致：
  1. YAML 中的明文 Key 被清空
  2. SecureStorage 未存入
  3. 前端误报"保存成功"
- `load_settings()` 从未从 SecureStorage 注入 Key，Router 始终读不到 Key

### 修复

| 文件 | 变更 |
|-------|------|
| `src/utils/security.py` | `store_api_key` 增加明文 fallback（当 keyring 和 cryptography 都不可用时）；`retrieve_api_key` 先检测明文再尝试解密；全部方法兼容明文文件路径 |
| `src/config/settings.py` | `load_settings()` 在构建 Settings 后，遍历 providers 从 SecureStorage 注入 `api_key` |
| `src/ui/webview/api_handler.py` | `save_settings` 中 API Key 存储失败时立即返回 `{"success": false, "error": ...}`，不再静默吞异常 |
| `tests/test_security.py` | 更新为明文 fallback 场景 |

- 测试：**149 passed** 全部通过

## Patch 2026-07-20 — 数据库 Schema 自动修复 + 学习路径错误处理改进

### 问题

- 用户报告：技能图谱中点击"生成学习路径"没有任何反应，其他入口（简历页缺失技能、学习路径页直接生成）同样无反应
- 根因：Sprint 12 中 `LearningPath` 模型新增 `source_type` 字段，但用户本地数据库（`~/.careercraft/career.db`）是旧 schema，缺少该列
- `get_learning_path` 调用 `create_learning_path` 保存时插入失败，`sqlite3.OperationalError: table learning_paths has no column named source_type`
- 异常被 `api_handler.get_learning_path` 的 try/except 吞掉，静默返回 `[]`，前端显示"暂无学习资源"，用户无法知道发生了什么
- `main_webview.py` 启动时未调用 `init_db()`，数据库创建/修复被延迟到首次服务访问，无法保证启动时就已完成 schema 更新

### 修复

| 文件 | 变更 |
|-------|------|
| `src/models/database.py` | 新增 `SCHEMA_MIGRATIONS` 配置 + `_migrate_schema()` 自动检测并添加缺失列；`init_db()` 在 `create_all` 后自动执行 schema 修复 |
| `src/main_webview.py` | 启动时调用 `asyncio.run(init_db())`，确保数据库初始化和 schema 修复在第一个窗口显示前完成 |
| `src/ui/webview/api_handler.py` | `get_learning_path` 去除宽泛 try/except，无角色时抛出 `ValueError`；异常由 bridge 层捕获并返回 `_err`，前端显示具体错误提示 |
| `tests/ui/webview/test_bridge.py` | `test_get_learning_path_with_skill` 先创建一个角色再测试，避免因无角色而失败 |

- 测试：**149 passed** 全部通过
