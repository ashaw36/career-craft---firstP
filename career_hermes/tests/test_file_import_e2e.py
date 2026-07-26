"""
CareerCraft Agent — 文件导入端到端测试

测试链路：文件读取 → LLM分析 → JSON解析 → 入库留痕
"""

from __future__ import annotations

import asyncio
import json
import tempfile
from pathlib import Path
from typing import Any, Dict, List

import pytest

from src.models.database import init_db, close_db
from src.models.entities import UploadedFile
from src.services.experience_manager import ExperienceManager
from src.services.import_parser import ImportParser, ImportParserError


class FakeLLM:
    """模拟 LLM，返回预定义的结构化经历 JSON"""

    def __init__(self, response_data: Any = None) -> None:
        self.response_data = response_data or [
            {
                "title": "高级产品经理",
                "type": "work",
                "organization": "测试科技有限公司",
                "start_date": "2020-07",
                "end_date": "2021-12",
                "raw_description": "负责核心产品规划和迭代，主导3个重点项目",
                "structured_achievements": [
                    "提升DAU 40%，月活突破500万",
                    "主导供应链系统重构，订单处理效率提升3倍"
                ],
                "skills_demonstrated": ["产品规划", "数据分析", "项目管理"],
                "metrics": [
                    {"metric": "DAU提升", "value": "40%", "unit": "百分比"},
                    {"metric": "月活", "value": "500万", "unit": "用户"}
                ]
            },
            {
                "title": "AI Agent平台项目",
                "type": "project",
                "organization": "测试科技有限公司",
                "start_date": "2022-01",
                "end_date": None,
                "raw_description": "搭建企业级AI Agent平台，支持RAG和多轮对话",
                "structured_achievements": [
                    "系统QPS达到2000+",
                    "服务10+内部业务线"
                ],
                "skills_demonstrated": ["Python", "FastAPI", "LangChain"],
                "metrics": [
                    {"metric": "QPS", "value": "2000+", "unit": "次/秒"},
                    {"metric": "业务线", "value": "10+", "unit": "条"}
                ]
            }
        ]

    async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
        # 验证 prompt 中包含了文件内容
        prompt = messages[0].get("content", "")
        assert "测试简历内容" in prompt or "file" in prompt.lower()
        return json.dumps(self.response_data, ensure_ascii=False)


@pytest.fixture(scope="session", autouse=True)
def db_setup() -> None:
    """测试级数据库初始化"""
    asyncio.run(init_db())
    yield
    asyncio.run(close_db())


@pytest.fixture
def clean_db() -> None:
    """每个测试后清理经历数据"""
    yield
    # 清理测试数据
    from src.models.database import AsyncSessionLocal
    from sqlalchemy import delete
    from src.models.entities import Experience
    async def _clean() -> None:
        async with AsyncSessionLocal() as session:
            await session.execute(delete(Experience))
            await session.execute(delete(UploadedFile))
            await session.commit()
    asyncio.run(_clean())


class TestFileImportE2E:
    @pytest.mark.asyncio
    async def test_analyze_file_with_llm_success(self, clean_db: Any) -> None:
        """测试 LLM 文件分析正常流程"""
        fake_llm = FakeLLM()
        parser = ImportParser(llm_router=fake_llm)  # type: ignore[arg-type]

        drafts = await parser.analyze_file_with_llm(
            "这是一份测试简历内容，包含产品经理工作经历",
            file_type="项目总结报告"
        )

        assert len(drafts) == 2

        # 第一条：工作经历
        d1 = drafts[0]
        assert d1.title == "高级产品经理"
        assert d1.extracted["type"] == "work"
        assert d1.extracted["organization"] == "测试科技有限公司"
        assert d1.extracted["start_date"] == "2020-07-01"  # YYYY-MM 解析
        assert d1.extracted["end_date"] == "2021-12-01"
        assert len(d1.extracted["structured_achievements"]) == 2
        assert "产品规划" in d1.extracted["skills_demonstrated"]

        # 第二条：项目经历（至今）
        d2 = drafts[1]
        assert d2.title == "AI Agent平台项目"
        assert d2.extracted["type"] == "project"
        assert d2.extracted["end_date"] is None  # 至今
        metrics = d2.extracted["metrics"]
        assert len(metrics) == 2
        assert metrics[0]["metric"] == "QPS"

    @pytest.mark.asyncio
    async def test_analyze_file_llm_returns_markdown_json(self, clean_db: Any) -> None:
        """测试 LLM 返回 markdown 代码块包裹的 JSON"""
        class MarkdownLLM:
            async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
                return (
                    "这是分析结果\n\n"
                    "```json\n"
                    '[{"title": "测试经历", "type": "work", "organization": "ABC公司"}]\n'
                    "```\n"
                    "希望对您有帮助"
                )

        parser = ImportParser(llm_router=MarkdownLLM())  # type: ignore[arg-type]
        drafts = await parser.analyze_file_with_llm("测试内容")

        assert len(drafts) == 1
        assert drafts[0].title == "测试经历"
        assert drafts[0].extracted["organization"] == "ABC公司"

    @pytest.mark.asyncio
    async def test_analyze_file_null_title_skipped(self, clean_db: Any) -> None:
        """测试 LLM 返回 title 为 null 时正确跳过"""
        class NullTitleLLM:
            async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
                return json.dumps([
                    {"title": None, "type": "work"},
                    {"title": "有效经历", "type": "project"}
                ], ensure_ascii=False)

        parser = ImportParser(llm_router=NullTitleLLM())  # type: ignore[arg-type]
        drafts = await parser.analyze_file_with_llm("测试内容")

        assert len(drafts) == 1
        assert drafts[0].title == "有效经历"

    @pytest.mark.asyncio
    async def test_file_import_confirm_and_save(self, clean_db: Any) -> None:
        """测试完整链路：文件分析 → 确认保存 → 数据库留痕"""
        fake_llm = FakeLLM()
        parser = ImportParser(llm_router=fake_llm)  # type: ignore[arg-type]
        manager = ExperienceManager()

        # 1. LLM 分析文件
        drafts = await parser.analyze_file_with_llm("测试简历内容")
        assert len(drafts) == 2

        # 2. 确认保存
        saved_exps = []
        for draft in drafts:
            exp = await manager.confirm_and_save(draft)
            saved_exps.append(exp)

        assert len(saved_exps) == 2
        assert saved_exps[0].title == "高级产品经理"
        assert saved_exps[0].status == "confirmed"

        # 3. 验证可查询
        exps = await manager.list_by_user(status_filter="confirmed")
        assert len(exps) == 2

    @pytest.mark.asyncio
    async def test_import_parser_error_handling(self, clean_db: Any) -> None:
        """测试 LLM 返回无效内容时抛出异常"""
        class BadLLM:
            async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
                return "这不是 JSON，也没有数组"

        parser = ImportParser(llm_router=BadLLM())  # type: ignore[arg-type]
        with pytest.raises(ImportParserError):
            await parser.analyze_file_with_llm("测试内容")


class TestFileSuffixCaseInsensitive:
    """测试文件后缀大小写不敏感"""

    def test_path_suffix_lower(self) -> None:
        """验证 Path.suffix.lower() 行为"""
        assert Path("/path/to/file.PDF").suffix.lower() == ".pdf"
        assert Path("/path/to/file.Docx").suffix.lower() == ".docx"
        assert Path("/path/to/file.MD").suffix.lower() == ".md"
        assert Path("/path/to/file.JSON").suffix.lower() == ".json"
