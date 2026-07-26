"""Tests for LLM Router"""
from __future__ import annotations

import pytest
from unittest.mock import AsyncMock, patch, MagicMock
import httpx

from src.llm.router import LLMRouter, LLMError, LLMTimeoutError


class TestLLMRouter:
    """LLM 路由器测试"""

    @pytest.fixture
    def mock_settings(self):
        """模拟配置"""
        settings = MagicMock()
        settings.default_llm_provider = "test"
        provider = MagicMock()
        provider.name = "test"
        provider.enabled = True
        provider.api_key = "sk-test"
        provider.base_url = "https://api.test.com"
        provider.default_model = "gpt-test"
        settings.llm_providers = [provider]
        return settings

    @pytest.mark.asyncio
    async def test_chat_success(self, mock_settings):
        """正常对话流程"""
        router = LLMRouter(settings=mock_settings)
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "choices": [{"message": {"content": "Hello"}}]
        }
        mock_response.raise_for_status = MagicMock()

        mock_client = AsyncMock()
        mock_client.post.return_value = mock_response

        with patch.object(router, "_get_client", return_value=mock_client):
            result = await router.chat(messages=[{"role": "user", "content": "hi"}])
            assert result == "Hello"

    @pytest.mark.asyncio
    async def test_chat_no_api_key(self, mock_settings):
        """缺少 API Key 时抛出异常"""
        mock_settings.llm_providers[0].api_key = ""
        router = LLMRouter(settings=mock_settings)
        with pytest.raises(LLMError):
            await router.chat(messages=[{"role": "user", "content": "hi"}])

    @pytest.mark.asyncio
    async def test_stream_chat(self, mock_settings):
        """流式输出测试"""
        router = LLMRouter(settings=mock_settings)

        async def _aiter_lines():
            yield 'data: {"choices": [{"delta": {"content": "Hi"}}]}'
            yield "data: [DONE]"

        mock_stream = MagicMock()
        mock_stream.__aenter__ = AsyncMock(return_value=mock_stream)
        mock_stream.__aexit__ = AsyncMock(return_value=None)
        mock_stream.aiter_lines = MagicMock(return_value=_aiter_lines())

        mock_client = AsyncMock()
        mock_client.stream = MagicMock(return_value=mock_stream)

        with patch.object(router, "_get_client", return_value=mock_client):
            chunks = []
            async for chunk in await router.chat(
                messages=[{"role": "user", "content": "hi"}], stream=True
            ):
                chunks.append(chunk)
            assert len(chunks) == 1
            assert chunks[0] == "Hi"
