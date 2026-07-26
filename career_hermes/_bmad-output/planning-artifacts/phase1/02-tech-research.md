# CareerCraft Agent — 技术预研与架构选型

> Phase 1 产出 | 子代球：架构师调研代球

## 一、最终推荐技术栈

| 层级 | 技术 | 版本/说明 |
|------|------|----------|
| **GUI** | PySide6 | Qt6，Python原生绑定 |
| **爬虫** | Playwright + stealth | 持久化context，限速采集 |
| **LLM路由** | 自研Router | httpx异步，YAML配置，自动降级 |
| **数据存储** | SQLite + sqlite-vec | 主数据+语义检索一体 |
| **打包** | PyInstaller / nuitka | Windows单exe输出 |
| **配置管理** | Pydantic Settings | 环境变量+本地YAML |

## 二、GUI 框架对比

| 方案 | 优点 | 缺点 | 推荐度 |
|------|------|------|--------|
| PySide6 | Python原生；功能完备；打包成熟 | 包体积大(~50MB+) | ⭐⭐⭐⭐⭐ |
| Tauri | 包体极小(~3MB)；前端自由 | Rust学习曲线陡；与Python集成需桥接 | ⭐⭐⭐ |
| Electron | 生态最成熟 | 包体巨大(~150MB+) | ⭐⭐ |
| Flet | Python写UI极快 | 尚处beta；自定义能力弱 | ⭐⭐⭐⭐ |

## 三、岗位爬虫方案

| 站点 | 反爬难度 | 法律风险 | 推荐策略 |
|------|----------|----------|----------|
| Boss直聘 | 高（滑块验证、IP限频） | 中 | Playwright+stealth+限速 |
| 猎聘网 | 中高 | 中 | Playwright或逆向接口 |
| LinkedIn | 极高 | **高**（CFAA风险） | **不建议爬取** |

**降级方案**：若爬虫失效，降级为浏览器书签脚本（用户手动复制JD，应用解析）。

## 四、LLM 多模型路由方案

**推荐：自研轻量统一层**（基于 httpx + pydantic）

```python
class LLMProvider(ABC):
    @abstractmethod
    async def chat(self, messages, model, temperature, stream=False) -> AsyncIterator[str] | str: ...
    @property
    def is_available(self) -> bool: ...

class Router:
    def __init__(self, providers: list[LLMProvider], fallback_chain: list[str]):
        self.fallback_chain = fallback_chain
    
    async def chat(self, **kwargs):
        for model in self.resolve(kwargs.pop("model")):
            provider = self.providers[model]
            if not provider.is_available: continue
            try:
                return await provider.chat(model=model, **kwargs)
            except (RateLimit, Timeout):
                continue
        raise NoProviderAvailable
```

## 五、架构草图

```
┌───────────────────────────────────────────────────────────┐
│  PySide6 GUI (主线程)                        │
│  ┌─────────┬─────────┬─────────┬─────────┐       │
│  │ 对话界面  │ 简历编辑器│ 岗位浏览器│ 学习看板  │       │
│  └─────────┴─────────┴─────────┴─────────┘       │
└───────────────┼───────────────────────────────┘
               │  Qt Signals / Slots
┌───────────────┴───────────────────────────────┐
│  Python 业务层 (异步线程池)                   │
│  ┌─────────────┬─────────────┬───────────────────┐ │
│  │ResumeBuilder│JobCrawler    │LLMRouter    │ │
│  │(Jinja2→PDF)│(Playwright)  │(httpx async)│ │
│  └─────────────┴─────────────┴───────────────────┘ │
│  ┌─────────────┬─────────────────────────┐ │
│  │SkillAnalyzer│ExperienceManager│PersonaEngine    │ │
│  │(向量比对)   │(经历CRUD)     │(角色适配)     │ │
│  └─────────────┴─────────────────────────┘ │
└──────────────────┼──────────────────────────────┘
               │  aiosqlite / sqlite-vec
┌───────────────┴───────────────────────────────┐
│  SQLite 单文件数据库 (~/.careercraft/)        │
│  ┌───────────────┬───────────────────┐         │
│  │ career.db      │ career.vec       │         │
│  │ (关系型主数据)  │ (向量索引，可选)  │         │
│  └───────────────┴───────────────────┘         │
└─────────────────────────────────────────────────────┘
```
