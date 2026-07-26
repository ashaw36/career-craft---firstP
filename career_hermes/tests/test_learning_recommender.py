"""
CareerCraft Agent — LearningRecommender 单元测试

测试覆盖：
1. 本地模板推荐
2. LLM 回退逻辑
3. 空技能列表
"""

from __future__ import annotations

from typing import Any, Dict, List

import pytest

from src.services.learning_recommender import LearningRecommender
from src.llm.router import LLMError


@pytest.fixture
def recommender() -> LearningRecommender:
    return LearningRecommender()


class TestRecommendByTemplate:
    def test_known_skill(self, recommender: LearningRecommender) -> None:
        items = recommender._recommend_by_template(["Kubernetes"])
        assert len(items) >= 1
        assert items[0]["type"] in ("course", "article", "book", "project")
        assert "title" in items[0]
        assert items[0]["status"] == "pending"

    def test_multiple_known_skills(self, recommender: LearningRecommender) -> None:
        items = recommender._recommend_by_template(["Docker", "gRPC"])
        assert len(items) >= 2
        titles = [i["title"] for i in items]
        assert len(titles) == len(set(titles))  # 去重

    def test_unknown_skill_fallback(self, recommender: LearningRecommender) -> None:
        items = recommender._recommend_by_template(["SomeExoticTech123"])
        assert len(items) == 1
        assert "SomeExoticTech123" in items[0]["title"]

    def test_empty_skills(self, recommender: LearningRecommender) -> None:
        items = recommender._recommend_by_template([])
        assert items == []


class TestRecommendForGap:
    @pytest.mark.asyncio
    async def test_empty_skills(self, recommender: LearningRecommender) -> None:
        items = await recommender.recommend_for_gap("p1", [])
        assert items == []

    @pytest.mark.asyncio
    async def test_with_mock_llm(self, recommender: LearningRecommender) -> None:
        # 由于无真实 API Key，此测试验证 LLM 失败时回退到模板
        items = await recommender.recommend_for_gap("p1", ["Python"])
        assert len(items) >= 1
        assert all("status" in i for i in items)


class TestResourceTemplates:
    def test_template_keys_lowercase(self, recommender: LearningRecommender) -> None:
        for key in recommender._RESOURCE_TEMPLATES:
            assert key == key.lower()

    def test_template_structure(self, recommender: LearningRecommender) -> None:
        for key, templates in recommender._RESOURCE_TEMPLATES.items():
            for t in templates:
                assert "title" in t
                assert "type" in t
                assert "estimated_hours" in t
                assert "url" in t

    def test_url_field_in_recommendations(self, recommender: LearningRecommender) -> None:
        items = recommender._recommend_by_template(["Docker", "UnknownSkill"])
        assert len(items) >= 1
        for item in items:
            assert "url" in item
            assert isinstance(item["url"], str)
