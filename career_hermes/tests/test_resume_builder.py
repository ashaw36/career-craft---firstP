"""Tests for Resume Builder"""
from __future__ import annotations

import asyncio
import pytest
from datetime import date
from unittest.mock import AsyncMock, patch, MagicMock

from src.services.resume_builder import ResumeBuilder


class TestResumeBuilder:
    """简历生成引擎测试"""

    @pytest.fixture
    def mock_persona(self):
        p = MagicMock()
        p.name = "AI产品经理"
        p.identity_statement = "专注于AI产品的策略与落地"
        p.career_narrative = ""
        p.tone_style = "business_insight"
        p.max_experiences = 5
        return p

    @pytest.fixture
    def mock_weighted_exp(self):
        """模拟带权重的经历"""
        rew = MagicMock()
        rew.relevance_score = 0.85
        rew.experience = MagicMock()
        rew.experience.title = "产品经理"
        rew.experience.organization = "美团"
        rew.experience.type = "work"
        rew.experience.start_date = date(2022, 1, 1)
        rew.experience.end_date = date(2023, 6, 1)
        rew.experience.structured_achievements = ["DAU提升30%"]
        rew.experience.skills_demonstrated = ["SQL", "Python"]
        rew.experience.metrics = ["30%"]
        rew.experience.raw_description = "负责增长"
        return rew

    @pytest.mark.asyncio
    async def test_build_context(self, mock_persona, mock_weighted_exp):
        """构建渲染上下文"""
        builder = ResumeBuilder(persona_id="test-id")
        builder._persona = mock_persona
        builder._experiences = [mock_weighted_exp]

        ctx = await builder._build_context()
        assert ctx["name"] == "AI产品经理"
        assert ctx["identity_statement"] == "专注于AI产品的策略与落地"
        assert len(ctx["experiences"]) == 1
        assert ctx["experiences"][0]["title"] == "产品经理"
        assert ctx["experiences"][0]["relevance_score"] == 0.85

    def test_format_period_both_dates(self):
        """格式化起止时间"""
        result = ResumeBuilder._format_period(
            date(2022, 1, 1), date(2023, 6, 1)
        )
        assert result == "2022.01 — 2023.06"

    def test_format_period_ongoing(self):
        """至今"""
        result = ResumeBuilder._format_period(date(2022, 1, 1), None)
        assert "2022.01" in result
        assert "至今" in result

    def test_format_period_none(self):
        """无日期"""
        result = ResumeBuilder._format_period(None, None)
        assert result == "? — 至今"


class TestResumeBuilderIntegration:
    """简历生成引擎集成测试 — 验证真实数据库场景下经历被正确填充"""

    @pytest.fixture(scope="session", autouse=True)
    def db_setup(self):
        from src.models.database import init_db, close_db
        asyncio.run(init_db())
        yield
        asyncio.run(close_db())

    @pytest.fixture
    def clean_db(self):
        yield
        from src.models.database import AsyncSessionLocal
        from sqlalchemy import delete
        from src.models.entities import Experience, Persona, RoleExperienceWeight
        async def _clean() -> None:
            async with AsyncSessionLocal() as session:
                await session.execute(delete(RoleExperienceWeight))
                await session.execute(delete(Experience))
                await session.execute(delete(Persona))
                await session.commit()
        asyncio.run(_clean())

    @pytest.mark.asyncio
    async def test_render_includes_experience_content(self, clean_db):
        """端到端：创建角色+经历 → 简历渲染 → 验证经历内容存在"""
        from src.models.database import AsyncSessionLocal
        from src.models.entities import Experience, Persona
        from datetime import date

        async with AsyncSessionLocal() as session:
            # 1. 创建角色
            persona = Persona(
                user_id="default",
                name="测试角色",
                tone_style="business_insight",
                capability_weights={"产品规划": 0.9, "数据分析": 0.8},
                max_experiences=5,
            )
            session.add(persona)

            # 2. 创建经历（confirmed 状态）
            exp = Experience(
                user_id="default",
                type="work",
                title="高级产品经理",
                organization="测试科技",
                start_date=date(2020, 7, 1),
                end_date=date(2023, 6, 1),
                raw_description="负责供应链数字化平台的产品规划与落地，实现成本降低20%。",
                structured_achievements=["DAU提升40%", "订单处理效率提升3倍"],
                skills_demonstrated=["产品规划", "数据分析", "供应链管理"],
                status="confirmed",
            )
            session.add(exp)
            await session.commit()
            await session.refresh(persona)
            await session.refresh(exp)

        # 3. 生成简历
        builder = ResumeBuilder(persona_id=persona.id)
        await builder.prepare()
        assert len(builder._experiences) > 0, "prepare() 应该加载到经历"

        # 绕过 LLM 身份声明生成
        builder._persona.identity_statement = "测试身份声明"
        md = await builder.render(template_name="modern")

        # 4. 验证
        assert "高级产品经理" in md
        assert "测试科技" in md
        assert "供应链数字化" in md  # 经历描述内容必须被填充
        assert "DAU提升40%" in md
        assert "产品规划" in md
        assert "2020.07" in md

    @pytest.mark.asyncio
    async def test_prepare_fallback_when_no_weights(self, clean_db):
        """当角色 capability_weights 为空时，fallback 仍能加载经历"""
        from src.models.database import AsyncSessionLocal
        from src.models.entities import Experience, Persona
        from datetime import date

        async with AsyncSessionLocal() as session:
            persona = Persona(
                user_id="default",
                name="无权重角色",
                tone_style="business_insight",
                capability_weights={},
                max_experiences=5,
            )
            session.add(persona)
            exp = Experience(
                user_id="default",
                type="work",
                title="无权重经历",
                organization="某公司",
                start_date=date(2021, 1, 1),
                raw_description="这是一段必须被填充的经历描述。",
                status="confirmed",
            )
            session.add(exp)
            await session.commit()
            await session.refresh(persona)
            await session.refresh(exp)

        builder = ResumeBuilder(persona_id=persona.id)
        await builder.prepare()
        assert len(builder._experiences) > 0, "即使无权重也应该通过 fallback 加载经历"

        builder._persona.identity_statement = "测试"
        md = await builder.render(template_name="modern")

        assert "无权重经历" in md
        assert "这是一段必须被填充的经历描述" in md

    @pytest.mark.asyncio
    async def test_prepare_fallback_when_experience_draft(self, clean_db):
        """当经历为 draft 状态时，简历不应包含该经历"""
        from src.models.database import AsyncSessionLocal
        from src.models.entities import Experience, Persona
        from datetime import date

        async with AsyncSessionLocal() as session:
            persona = Persona(
                user_id="default",
                name="测试角色",
                tone_style="business_insight",
                capability_weights={"test": 1.0},
                max_experiences=5,
            )
            session.add(persona)
            exp = Experience(
                user_id="default",
                type="work",
                title="Draft经历",
                organization="某公司",
                start_date=date(2021, 1, 1),
                raw_description="这是draft经历。",
                status="draft",
            )
            session.add(exp)
            await session.commit()
            await session.refresh(persona)

        builder = ResumeBuilder(persona_id=persona.id)
        await builder.prepare()
        md = await builder.render(template_name="modern")

        assert "Draft经历" not in md  # draft 经历不应出现在简历中
