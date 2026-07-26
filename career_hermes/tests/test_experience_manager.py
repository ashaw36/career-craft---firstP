"""Tests for Experience Manager"""
from __future__ import annotations

import pytest
from datetime import date
from unittest.mock import AsyncMock, patch, MagicMock

from src.services.experience_manager import ExperienceManager, ExperienceDraft


class TestExperienceDraft:
    """经历草稿测试"""

    def test_to_dict_basic(self):
        """草稿转字典"""
        draft = ExperienceDraft(
            raw_text="我在美团做PM",
            extracted={"title": "产品经理", "organization": "美团"},
        )
        data = draft.to_dict()
        assert data["title"] == "产品经理"
        assert data["raw_description"] == "我在美团做PM"

    def test_to_dict_with_user_edits(self):
        """用户编辑覆盖"""
        draft = ExperienceDraft(
            raw_text="我在美团做PM",
            extracted={"title": "产品经理"},
        )
        draft.user_edits = {"title": "高级产品经理"}
        data = draft.to_dict()
        assert data["title"] == "高级产品经理"


class TestExperienceManager:
    """经历管理器测试"""

    @pytest.fixture
    def manager(self):
        mock_llm = AsyncMock()
        return ExperienceManager(llm_router=mock_llm)

    @pytest.mark.asyncio
    async def test_create_draft_success(self, manager):
        """成功创建草稿"""
        manager.llm.chat.return_value = '''{"title": "产品经理", "organization": "美团", "type": "work"}'''
        draft = await manager.create_draft("我在美团做产品经理，负责增长")
        assert draft.extracted["title"] == "产品经理"
        assert draft.extracted["organization"] == "美团"

    @pytest.mark.asyncio
    async def test_create_draft_json_in_markdown(self, manager):
        """LLM 返回 markdown 代码块"""
        manager.llm.chat.return_value = '''```json\n{"title": "工程师", "type": "work"}\n```'''
        draft = await manager.create_draft("我在字节做工程师")
        assert draft.extracted["title"] == "工程师"

    @pytest.mark.asyncio
    async def test_create_draft_missing_title(self, manager):
        """缺少必填字段时抛出异常"""
        manager.llm.chat.return_value = '''{"organization": "美团"}'''
        with pytest.raises(ValueError, match="title"):
            await manager.create_draft("一段没有标题的描述")

    def test_parse_date_iso(self):
        """ISO 日期解析"""
        result = ExperienceManager._parse_date("2023-06-01")
        assert result == date(2023, 6, 1)

    def test_parse_date_invalid(self):
        """无效日期返回 None"""
        result = ExperienceManager._parse_date("not-a-date")
        assert result is None

    def test_parse_date_empty(self):
        """空字符串返回 None"""
        result = ExperienceManager._parse_date("")
        assert result is None
