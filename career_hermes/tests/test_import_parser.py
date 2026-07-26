"""
CareerCraft Agent — 经历批量导入解析器测试
"""

from __future__ import annotations

import pytest

from src.services.import_parser import ImportParser, ImportParserError


class TestImportParser:
    @pytest.mark.asyncio
    async def test_parse_markdown_basic(self) -> None:
        md = """
## 工作经历

### 某某科技 | 高级产品经理
2020.01 - 2023.06

- 负责 AI 产品规划与落地
- 带领 5 人团队

## 项目经历

### 智能客服平台
2021.03 - 2022.12

- 设计并实现智能客服系统
- 提升用户满意度 30%
"""
        parser = ImportParser()
        drafts = await parser.parse_markdown(md)
        assert len(drafts) >= 2
        titles = [d.extracted.get("title", "") for d in drafts]
        assert "高级产品经理" in titles
        assert "智能客服平台" in titles

    @pytest.mark.asyncio
    async def test_parse_markdown_dates(self) -> None:
        md = """
## 工作经历

### 测试公司 | 工程师
2020.01 - 至今

- 开发后端系统
"""
        parser = ImportParser()
        drafts = await parser.parse_markdown(md)
        assert len(drafts) == 1
        d = drafts[0]
        assert d.extracted["title"] == "工程师"
        assert d.extracted["organization"] == "测试公司"
        assert d.extracted["start_date"] == "2020-01-01"
        assert d.extracted["end_date"] is None

    @pytest.mark.asyncio
    async def test_parse_text_blocks(self) -> None:
        text = """
测试公司 | 产品经理
2020.03 - 2023.06

- 负责产品规划
- 带领团队增长

另一家公司 | 运营专员
2019.01 - 2020.02

- 负责用户运营
"""
        parser = ImportParser()
        drafts = await parser.parse_text(text)
        assert len(drafts) >= 1

    @pytest.mark.asyncio
    async def test_parse_json_array(self) -> None:
        json_text = """[
            {"title": "工程师", "type": "work", "organization": "A公司", "start_date": "2020-01-01"},
            {"title": "产品经理", "type": "work", "organization": "B公司"}
        ]"""
        parser = ImportParser()
        drafts = await parser.parse_json(json_text)
        assert len(drafts) == 2
        assert drafts[0].extracted["title"] == "工程师"
        assert drafts[1].extracted["title"] == "产品经理"

    @pytest.mark.asyncio
    async def test_parse_json_single_object(self) -> None:
        json_text = '{"title": "某某", "type": "project"}'
        parser = ImportParser()
        drafts = await parser.parse_json(json_text)
        assert len(drafts) == 1
        assert drafts[0].extracted["type"] == "project"

    @pytest.mark.asyncio
    async def test_parse_json_invalid(self) -> None:
        with pytest.raises(ImportParserError):
            parser = ImportParser()
            await parser.parse_json("not json")

    def test_extract_dates(self) -> None:
        from datetime import date as dt
        text = "在此工作 2020.03 - 2023.06"
        start, end = ImportParser._extract_dates(text)
        assert start == dt(2020, 3, 1)
        assert end == dt(2023, 6, 1)

    def test_extract_dates_ongoing(self) -> None:
        text = "2020.03 - 至今"
        start, end = ImportParser._extract_dates(text)
        from datetime import date as dt
        assert start == dt(2020, 3, 1)
        assert end is None

    def test_extract_dates_none(self) -> None:
        start, end = ImportParser._extract_dates("没有日期")
        assert start is None
        assert end is None

    def test_split_header(self) -> None:
        assert ImportParser._split_header("公司 | 职位") == ("公司", "职位")
        assert ImportParser._split_header("职位 @ 公司") == ("职位", "公司")
        assert ImportParser._split_header("单一标题") == (None, "单一标题")

    def test_infer_type(self) -> None:
        assert ImportParser._infer_type("软件工程师", "某公司", "开发系统") == "work"
        assert ImportParser._infer_type("某项目", None, "开发平台") == "project"
        assert ImportParser._infer_type("某本科", "某大学", "毕业") == "education"
