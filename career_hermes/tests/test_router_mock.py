"""
CareerCraft Agent — LLM Router Mock 模式测试
"""

from __future__ import annotations

import pytest

from src.llm.router import LLMRouter


class TestMockMode:
    @pytest.mark.asyncio
    async def test_mock_returns_text(self) -> None:
        router = LLMRouter(mock=True)
        response = await router.chat(
            messages=[{"role": "user", "content": "hello"}]
        )
        assert isinstance(response, str)
        assert len(response) > 0

    @pytest.mark.asyncio
    async def test_mock_json_mode(self) -> None:
        router = LLMRouter(mock=True)
        response = await router.chat(
            messages=[{"role": "user", "content": "parse this"}],
            json_mode=True,
        )
        assert isinstance(response, str)
        assert "[" in response or "{" in response

    @pytest.mark.asyncio
    async def test_mock_job_parsing(self) -> None:
        router = LLMRouter(mock=True)
        response = await router.chat(
            messages=[{"role": "user", "content": "parse job description"}],
            json_mode=True,
        )
        assert "Mock" in response

    @pytest.mark.asyncio
    async def test_mock_retelling(self) -> None:
        router = LLMRouter(mock=True)
        response = await router.chat(
            messages=[{"role": "user", "content": "retell experience"}],
        )
        assert "重述" in response or "Mock" in response

    def test_enable_disable_mock(self) -> None:
        router = LLMRouter()
        assert not router._mock
        router.enable_mock()
        assert router._mock
        router.disable_mock()
        assert not router._mock
