"""
CareerCraft Agent — 应用配置管理

Pydantic Settings 实现，支持环境变量 + YAML 配置文件。
"""

from __future__ import annotations

from pathlib import Path
from typing import List, Optional

import yaml
from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

# 配置目录：~/.careercraft/
CONFIG_DIR = Path.home() / ".careercraft"
CONFIG_FILE = CONFIG_DIR / "config.yaml"


class LLMProviderConfig(BaseSettings):
    """单个 LLM 供应商配置"""

    name: str
    api_key: str = ""
    base_url: Optional[str] = None
    default_model: str = "qwen-max"
    fallback_models: List[str] = Field(default_factory=list)
    timeout: int = 30
    max_retries: int = 3
    enabled: bool = True


class CareerCraftSettings(BaseSettings):
    """全局应用配置"""

    model_config = SettingsConfigDict(
        env_prefix="CC_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # 应用基础
    app_name: str = "CareerCraft Agent"
    app_version: str = "0.1.0"
    debug: bool = False
    log_level: str = "INFO"

    # 数据库
    db_path: str = str(Path.home() / ".careercraft" / "career.db")
    db_backup_interval_minutes: int = 5
    db_backup_retention_count: int = 10

    # 简历导出
    export_dir: str = str(Path.home() / "Documents" / "CareerCraft")

    # LLM 配置（从 YAML 加载后覆盖）
    llm_providers: List[LLMProviderConfig] = Field(default_factory=list)
    default_llm_provider: str = "tongyi"

    # 爬虫配置
    crawler_headless: bool = True
    crawler_stealth: bool = True
    crawler_request_delay_ms: int = 2000
    crawler_timeout_ms: int = 10000

    # 简历生成
    resume_max_experiences: int = 5
    resume_fit_score_threshold: float = 0.3

    @field_validator("log_level")
    @classmethod
    def validate_log_level(cls, v: str) -> str:
        allowed = {"DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}
        upper = v.upper()
        if upper not in allowed:
            raise ValueError(f"log_level 必须为 {allowed}")
        return upper


def _load_yaml_config() -> dict:
    """从 ~/.careercraft/config.yaml 加载配置"""
    if not CONFIG_FILE.exists():
        return {}
    with open(CONFIG_FILE, "r", encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


def _save_yaml_config(data: dict) -> None:
    """保存配置到 ~/.careercraft/config.yaml"""
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    with open(CONFIG_FILE, "w", encoding="utf-8") as f:
        yaml.dump(data, f, allow_unicode=True, sort_keys=False)


def create_default_config() -> None:
    """生成默认配置文件"""
    default_data = {
        "debug": False,
        "log_level": "INFO",
        "llm_providers": [
            {
                "name": "tongyi",
                "api_key": "",
                "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "default_model": "qwen-max",
                "fallback_models": ["qwen-plus", "qwen-turbo"],
                "timeout": 30,
                "max_retries": 3,
                "enabled": True,
            },
            {
                "name": "openai",
                "api_key": "",
                "base_url": None,
                "default_model": "gpt-4o",
                "fallback_models": ["gpt-4o-mini"],
                "timeout": 30,
                "max_retries": 3,
                "enabled": False,
            },
        ],
        "default_llm_provider": "tongyi",
        "crawler_headless": True,
        "crawler_stealth": True,
        "crawler_request_delay_ms": 2000,
        "resume_max_experiences": 5,
        "resume_fit_score_threshold": 0.3,
    }
    _save_yaml_config(default_data)


def load_settings() -> CareerCraftSettings:
    """
    加载应用配置：先读取环境变量，再从 YAML 文件加载并合并。
    如果配置文件不存在，自动创建默认配置。
    加载完成后从 SecureStorage 注入 API Key。"""
    if not CONFIG_FILE.exists():
        create_default_config()

    yaml_data = _load_yaml_config()
    settings = CareerCraftSettings(**yaml_data)

    # 从 SecureStorage 注入 API Key（YAML 中不存储明文 Key）
    from src.utils.security import SecureStorage
    for provider in settings.llm_providers:
        key = SecureStorage.retrieve_api_key(provider.name)
        if key:
            provider.api_key = key

    return settings


# 全局配置实例（延迟初始化）
_settings: Optional[CareerCraftSettings] = None


def get_settings() -> CareerCraftSettings:
    global _settings
    if _settings is None:
        _settings = load_settings()
    return _settings
