"""Tests for Persona Engine"""
from __future__ import annotations

import pytest

from src.services.persona_engine import PersonaEngine
from src.models.entities import Experience


class TestPersonaEngine:
    """角色引擎测试"""

    @pytest.fixture
    def engine(self):
        return PersonaEngine()

    def test_extract_keywords(self, engine):
        """关键词提取"""
        exp = Experience(
            title="产品经理",
            organization="美团",
            raw_description="负责用户增长，使用SQL和Python分析数据",
            skills_demonstrated=["SQL", "Python", "增长策略"],
            structured_achievements=["DAU提升30%"],
        )
        keywords = PersonaEngine._extract_keywords(exp)
        assert "产品经理" in keywords
        assert "美团" in keywords
        assert "sql" in keywords
        assert "python" in keywords
        assert "增长策略" in keywords

    def test_tokenize_mixed(self, engine):
        """中英混合分词"""
        result = PersonaEngine._tokenize("负责AI算法和Python开发")
        assert "AI" in result
        assert "Python" in result
        assert "负责" in result  # 中文连续字符

    def test_tokenize_empty(self, engine):
        """空文本分词"""
        result = PersonaEngine._tokenize("")
        assert result == []

    def test_keyword_match_exact(self, engine):
        """精确匹配"""
        assert PersonaEngine._keyword_match("Python", ["python", "sql"]) is True

    def test_keyword_match_substring(self, engine):
        """子串匹配"""
        assert PersonaEngine._keyword_match("Python开发", ["python"]) is True

    def test_keyword_match_no_match(self, engine):
        """不匹配"""
        assert PersonaEngine._keyword_match("Java", ["python", "sql"]) is False
