"""
CareerCraft Agent — LLM 路由器

统一对外 LLM 调用接口，支持流式输出、超时控制、故障降级。
Sprint 1 实现单模型通义千问集成，Sprint 4 新增多模型自动降级。
"""

from __future__ import annotations

import json
import logging
from typing import Any, AsyncIterator, Callable, Dict, List, Optional, TypeVar

import httpx

from src.config.settings import CareerCraftSettings, LLMProviderConfig, get_settings

logger = logging.getLogger(__name__)

F = TypeVar("F", bound=Callable[..., Any])


def retry_on_llm_error(max_retries: int = 3, backoff_base: float = 1.0):
    """
    重试装饰器：在 LLM 调用失败时自动重试，指数退避。

    仅对 transient 错误（超时、限流、HTTP 5xx）触发重试，
    配置错误（API Key 未配置等）不重试。
    """
    def decorator(fn: F) -> F:
        async def wrapper(*args: Any, **kwargs: Any) -> Any:
            last_error: Optional[Exception] = None
            for attempt in range(1, max_retries + 1):
                try:
                    return await fn(*args, **kwargs)
                except (LLMTimeoutError, LLMRateLimitError, httpx.HTTPStatusError) as e:
                    last_error = e
                    if isinstance(e, httpx.HTTPStatusError) and e.response.status_code < 500:
                        # 4xx 错误不重试
                        raise
                    wait = backoff_base * (2 ** (attempt - 1))
                    logger.warning(
                        "LLM 调用失败（第 %d/%d 次），%.1f 秒后重试: %s",
                        attempt, max_retries, wait, e
                    )
                    import asyncio
                    await asyncio.sleep(wait)
                except LLMError:
                    # 其他 LLMError 不重试
                    raise
            if last_error:
                raise last_error
            raise LLMError("重试次数耗尽")
        return wrapper  # type: ignore[return-value]
    return decorator


class LLMError(Exception):
    """LLM 调用异常基类"""

    def __init__(self, message: str, provider: str = "", model: str = "") -> None:
        super().__init__(message)
        self.provider = provider
        self.model = model


class LLMTimeoutError(LLMError):
    """LLM 超时"""


class LLMRateLimitError(LLMError):
    """LLM 限流"""


class LLMContentError(LLMError):
    """LLM 返回内容异常"""


class LLMRouter:
    """
    LLM 路由器

    使用示例：
        router = LLMRouter()
        # 非流式
        response = await router.chat(messages=[...])
        # 流式
        async for chunk in router.chat(messages=[...], stream=True):
            print(chunk, end="")
    """

    def __init__(self, settings: Optional[CareerCraftSettings] = None, mock: bool = False) -> None:
        self.settings = settings or get_settings()
        self._client: Optional[httpx.AsyncClient] = None
        self._provider: Optional[LLMProviderConfig] = None
        self._mock = mock

    def enable_mock(self) -> None:
        """启用 Mock 模式，不调用真实 LLM，返回简单模拟响应"""
        self._mock = True

    def disable_mock(self) -> None:
        """关闭 Mock 模式"""
        self._mock = False

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None:
            self._client = httpx.AsyncClient(
                timeout=httpx.Timeout(30.0, connect=10.0),
                limits=httpx.Limits(max_connections=10, max_keepalive_connections=5),
            )
        return self._client

    def _resolve_provider(self, provider_name: Optional[str] = None) -> LLMProviderConfig:
        """解析当前使用的 LLM 供应商配置"""
        if provider_name:
            for p in self.settings.llm_providers:
                if p.name == provider_name and p.enabled:
                    return p
            raise LLMError(f"未找到启用的 LLM 供应商: {provider_name}")

        if not self._provider:
            default_name = self.settings.default_llm_provider
            for p in self.settings.llm_providers:
                if p.name == default_name and p.enabled:
                    self._provider = p
                    break
            if not self._provider:
                raise LLMError(f"未找到启用的 LLM 供应商: {default_name}")
        return self._provider

    def _get_all_enabled_providers(self) -> List[LLMProviderConfig]:
        """获取所有启用的供应商，默认供应商排在第一位"""
        enabled = [p for p in self.settings.llm_providers if p.enabled]
        default_name = self.settings.default_llm_provider
        # 将默认供应商移到最前面
        enabled.sort(key=lambda p: (p.name != default_name, p.name))
        return enabled

    async def chat(
        self,
        messages: List[Dict[str, str]],
        model: Optional[str] = None,
        temperature: float = 0.7,
        stream: bool = False,
        json_mode: bool = False,
    ) -> str | AsyncIterator[str]:
        """
        统一对话接口（支持多模型自动降级）

        Args:
            messages: OpenAI 格式消息列表，e.g. [{"role": "user", "content": "..."}]
            model: 指定模型，None 时使用供应商默认模型
            temperature: 采样温度
            stream: 是否流式输出
            json_mode: 是否强制返回 JSON

        Returns:
            非流式时返回完整字符串；流式时返回 AsyncIterator[str]
        """
        if self._mock:
            return self._mock_response(messages, json_mode)

        providers = self._get_all_enabled_providers()
        if not providers:
            raise LLMError("没有启用的 LLM 供应商")

        last_error: Optional[Exception] = None

        for provider in providers:
            try:
                return await self._chat_single(
                    provider=provider,
                    messages=messages,
                    model=model,
                    temperature=temperature,
                    stream=stream,
                    json_mode=json_mode,
                )
            except (LLMTimeoutError, LLMRateLimitError, httpx.HTTPStatusError) as e:
                last_error = e
                logger.warning(
                    "LLM 供应商 %s 调用失败，尝试下一个: %s",
                    provider.name,
                    e,
                )
                continue
            except LLMError as e:
                # 其他 LLMError 不降级（如 API Key 未配置）
                if provider.name == providers[-1].name:
                    raise
                last_error = e
                logger.warning(
                    "LLM 供应商 %s 错误: %s",
                    provider.name,
                    e,
                )
                continue

        # 所有供应商都失败
        if last_error:
            raise last_error
        raise LLMError("所有启用的 LLM 供应商均调用失败")

    @retry_on_llm_error(max_retries=3, backoff_base=1.0)
    async def _chat_single(
        self,
        provider: LLMProviderConfig,
        messages: List[Dict[str, str]],
        model: Optional[str],
        temperature: float,
        stream: bool,
        json_mode: bool,
    ) -> str | AsyncIterator[str]:
        """
        单供应商对话（原 chat() 的核心逻辑）
        """
        use_model = model or provider.default_model

        if not provider.api_key:
            raise LLMError("API Key 未配置", provider=provider.name, model=use_model)

        headers = {
            "Authorization": f"Bearer {provider.api_key}",
            "Content-Type": "application/json",
        }

        payload: Dict[str, Any] = {
            "model": use_model,
            "messages": messages,
            "temperature": temperature,
            "stream": stream,
        }

        if json_mode:
            payload["response_format"] = {"type": "json_object"}

        base_url = provider.base_url or "https://api.openai.com/v1"
        url = f"{base_url.rstrip('/')}/chat/completions"

        client = await self._get_client()

        try:
            if stream:
                return self._stream_chat(
                    client, url, headers, payload, provider.name, use_model
                )
            else:
                return await self._complete_chat(
                    client, url, headers, payload, provider.name, use_model
                )
        except httpx.TimeoutException as e:
            raise LLMTimeoutError(
                f"LLM 请求超时: {e}", provider=provider.name, model=use_model
            )
        except httpx.HTTPStatusError as e:
            if e.response.status_code == 429:
                raise LLMRateLimitError(
                    "LLM 限流，请稍后重试", provider=provider.name, model=use_model
                )
            raise LLMError(
                f"LLM HTTP 错误 {e.response.status_code}: {e.response.text}",
                provider=provider.name,
                model=use_model,
            )
        except Exception as e:
            raise LLMError(
                f"LLM 调用失败: {e}", provider=provider.name, model=use_model
            )

    async def _complete_chat(
        self,
        client: httpx.AsyncClient,
        url: str,
        headers: Dict[str, str],
        payload: Dict[str, Any],
        provider_name: str,
        model: str,
    ) -> str:
        """非流式对话"""
        response = await client.post(url, headers=headers, json=payload)
        response.raise_for_status()
        data = response.json()

        if "choices" not in data or not data["choices"]:
            raise LLMContentError("LLM 返回格式异常，缺少 choices", provider_name, model)

        content = data["choices"][0].get("message", {}).get("content", "")
        return content.strip()

    async def _stream_chat(
        self,
        client: httpx.AsyncClient,
        url: str,
        headers: Dict[str, str],
        payload: Dict[str, Any],
        provider_name: str,
        model: str,
    ) -> AsyncIterator[str]:
        """流式对话，逐字返回"""
        async with client.stream("POST", url, headers=headers, json=payload) as response:
            response.raise_for_status()
            async for line in response.aiter_lines():
                line = line.strip()
                if not line or not line.startswith("data: "):
                    continue
                data_str = line[6:]  # 去掉 "data: "
                if data_str == "[DONE]":
                    break
                try:
                    data = json.loads(data_str)
                    delta = (
                        data.get("choices", [{}])[0]
                        .get("delta", {})
                        .get("content", "")
                    )
                    if delta:
                        yield delta
                except json.JSONDecodeError:
                    continue

    async def close(self) -> None:
        """关闭 HTTP 客户端"""
        if self._client:
            await self._client.aclose()
            self._client = None

    def _mock_response(
        self, messages: List[Dict[str, str]], json_mode: bool
    ) -> str:
        """
        Mock 响应生成器，用于开发测试无需真实 API Key。

        根据用户消息内容返回简单模拟结果：
        - 如果消息包含 JSON/JSON 数组 关键词，返回简单 JSON
        - 否则返回简短文本确认
        """
        user_content = ""
        for m in messages:
            if m.get("role") == "user":
                user_content = m.get("content", "")
                break

        if json_mode or "json" in user_content.lower():
            # 根据 prompt 内容返回合适结构
            if "学习" in user_content or "资源" in user_content or "learning" in user_content.lower():
                return '[{"type": "course", "title": "Mock 学习资源", "source": "Mock", "estimated_hours": 10, "priority": 1}]'
            # 经历提取 / 岗位解析 需要 dict
            return '{"title": "Mock 标题", "organization": "MockCorp", "start_date": "2024-01-01", "end_date": "2024-12-31", "type": "work", "structured_achievements": ["成果1"], "skills_demonstrated": ["Python", "SQL"], "metrics": [{"metric": "效率", "value": "+40%"}]}'

        if "岗位" in user_content or "jd" in user_content.lower() or "job" in user_content.lower():
            return '{"title": "Mock 岗位", "company": "MockCorp", "parsed_skills": ["Python", "SQL"], "location": "北京"}'

        if "重述" in user_content or "retell" in user_content.lower() or "narrative" in user_content.lower():
            return "这是一段重述后的模拟经历摘要，突出了用户在目标岗位上的匹配能力。"

        return "这是 Mock 模式的自动响应，用于开发测试。"
