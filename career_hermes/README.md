# CareerCraft Agent

> 角色档案驱动的个人职业智能体 — 经历录入 → 角色配置 → 简历生成 → 岗位匹配 → 学习推荐

## 功能特性

- **多角色档案**：同一套经历库，切换 AI PM / 销售 / 架构师等角色，自动生成对应简历
- **对话式录入**：自然语言描述经历，LLM 自动结构化为标准格式
- **Fit Score 智能筛选**：基于角色关键词匹配，自动排序经历相关度
- **多模型 LLM 支持**：通义千问为主，支持 OpenAI / Claude 切换，自动降级
- **本地优先**：SQLite 本地存储，API Key 加密保护，隐私可控

## 快速开始

### 环境要求
- Python 3.11+
- 通义千问 API Key（可选，用于 LLM 功能）

### 安装

```bash
cd /mnt/d/workplace_for_hermes/career-agent
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
```

### 配置 API Key

第一次启动时会自动生成配置文件：
```bash
python -m src.main
```

或手动编辑 `~/.careercraft/config.yaml`：
```yaml
llm_providers:
  - name: tongyi
    api_key: "sk-your-key-here"
    base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1"
    default_model: "qwen-max"
    enabled: true
default_llm_provider: "tongyi"
```

### 运行

```bash
python -m src.main
```

## 项目结构

```
career-agent/
├── src/
│   ├── main.py                 # 应用入口
│   ├── config/                 # 配置管理
│   ├── models/                 # 数据库 ORM
│   ├── llm/                    # LLM 路由器
│   ├── services/               # 业务服务层
│   │   ├── experience_manager.py
│   │   ├── persona_engine.py
│   │   ├── resume_builder.py
│   │   └── conversation_engine.py
│   ├── ui/                     # PySide6 GUI
│   └── utils/                  # 工具函数
├── tests/                    # 测试用例
├── _bmad-output/            # BMAD 过程产物
└── docs/                    # 架构文档
```

## 开发进度

| 阶段 | 状态 |
|------|------|
| Sprint 1 — 骨架 + LLM 连通 | ✅ 完成 |
| Sprint 2 — 经历管理 + 角色基础 | ✅ 完成 |
| Sprint 3 — 智能录入 + 简历渲染 | ✅ 核心完成 |
| Sprint 4 — 经历重述 + 多模型 + JD 解析 | ⏳ 待启动 |
| Sprint 5 — 岗位匹配 + Gap + 学习推荐 | ⏳ 待启动 |
| Sprint 6 — Polish + 测试 + 文档 | ⏳ 待启动 |

## 技术栈

- **GUI**: PySide6
- **数据库**: SQLite + SQLAlchemy 2.0 (async) + aiosqlite
- **LLM**: 通义千问 / OpenAI 兼容 API
- **模板**: Jinja2
- **配置**: Pydantic Settings + YAML
- **安全**: keyring / Fernet 加密

## License

MIT
