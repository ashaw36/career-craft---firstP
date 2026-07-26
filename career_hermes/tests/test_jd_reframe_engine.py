"""
CareerCraft Agent — JDReframeEngine 单元测试

测试覆盖：
1. 单条经历修饰
2. 完整 JobMatch 修饰流程
3. 缓存读取
4. 删除修饰记录
5. JSON 提取鲁棒性
"""

from __future__ import annotations

import asyncio
import json
from datetime import date
from typing import Any, Dict, List

import pytest

from src.models.database import init_db, close_db
from src.models.entities import (
    Experience,
    JobDesc,
    JobMatch,
    JobMatchExperienceReframe,
    Persona,
    RoleExperienceWeight,
)
from src.services.jd_reframe_engine import JDReframeEngine


class FakeLLM:
    """模拟 LLM，返回预定义的修饰 JSON"""

    def __init__(self, response_data: Dict[str, Any] = None) -> None:
        self.response_data = response_data or {
            "reframed_summary": "作为高级产品经理，我主导了供应链数字化平台的架构设计与落地，通过数据驱动的决策优化供应链效率，实现成本降低20%。",
            "reframing_strategy": "突出供应链数字化能力，强调成本优化和系统架构经验",
        }

    async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
        return json.dumps(self.response_data, ensure_ascii=False)


@pytest.fixture(scope="session", autouse=True)
def db_setup() -> None:
    """测试级数据库初始化"""
    asyncio.run(init_db())
    yield
    asyncio.run(close_db())


@pytest.fixture
def clean_db() -> None:
    """每个测试后清理数据"""
    yield
    from src.models.database import AsyncSessionLocal
    from sqlalchemy import delete
    async def _clean() -> None:
        async with AsyncSessionLocal() as session:
            await session.execute(delete(JobMatchExperienceReframe))
            await session.execute(delete(JobMatch))
            await session.execute(delete(JobDesc))
            await session.execute(delete(RoleExperienceWeight))
            await session.execute(delete(Experience))
            await session.execute(delete(Persona))
            await session.commit()
    asyncio.run(_clean())


async def _create_test_data() -> tuple[str, str, str, str]:
    """创建测试需要的完整数据链

    返回: (experience_id, persona_id, job_desc_id, match_id)
    """
    from src.models.database import AsyncSessionLocal
    async with AsyncSessionLocal() as session:
        exp = Experience(
            user_id="default",
            type="work",
            title="高级产品经理",
            organization="测试科技有限公司",
            start_date=date(2020, 7, 1),
            end_date=date(2023, 6, 1),
            raw_description="负责产品规划和迭代，主导供应链系统重构",
            structured_achievements=["提升DAU 40%", "订单处理效率提升3倍"],
            skills_demonstrated=["产品规划", "数据分析", "供应链管理"],
            status="confirmed",
        )
        session.add(exp)
        await session.flush()

        persona = Persona(
            user_id="default",
            name="产品经理-AI方向",
            tone_style="business_insight",
            capability_weights={"产品规划": 0.9, "数据分析": 0.8, "供应链管理": 0.7},
            max_experiences=5,
        )
        session.add(persona)
        await session.flush()

        rew = RoleExperienceWeight(
            persona_id=persona.id,
            experience_id=exp.id,
            relevance_score=0.85,
            reframed_summary="高级产品经理，主导供应链数字化",
        )
        session.add(rew)

        job_desc = JobDesc(
            raw_text="招聘高级产品经理，负责供应链数字化平台...",
            title="高级产品经理-供应链方向",
            company="目标科技公司",
            location="深圳",
            parsed_skills=["产品规划", "供应链管理", "数据分析", "系统架构"],
            responsibilities=["负责供应链数字化平台规划", "优化采购成本"],
            years_of_experience="3-5年",
        )
        session.add(job_desc)
        await session.flush()

        match = JobMatch(
            persona_id=persona.id,
            job_desc_id=job_desc.id,
            match_score=75,
            matched_skills=["产品规划", "数据分析"],
            missing_skills=["系统架构"],
            score_breakdown={"skill": 40, "experience": 20, "text_similarity": 10, "other": 5},
            tracking_status="new",
        )
        session.add(match)
        await session.commit()

        return exp.id, persona.id, job_desc.id, match.id


class TestReframeSingleExperience:
    def test_reframe_single_experience(self, clean_db: Any) -> None:
        """测试单条经历修饰"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            from src.models.database import AsyncSessionLocal
            async with AsyncSessionLocal() as session:
                exp = await session.get(Experience, exp_id)
                persona = await session.get(Persona, persona_id)
                job_desc = await session.get(JobDesc, job_desc_id)

                reframe = await engine._reframe_single_experience(
                    experience=exp,
                    job_desc=job_desc,
                    persona=persona,
                    match_id=match_id,
                    original_summary=exp.raw_description or "",
                )

                assert reframe.job_match_id == match_id
                assert reframe.experience_id == exp_id
                assert reframe.original_summary == (exp.raw_description or "")
                assert "供应链" in reframe.reframed_summary
                assert reframe.reframing_strategy != ""

        asyncio.run(_test())


class TestReframeExperiencesForJob:
    def test_reframe_experiences_for_job(self, clean_db: Any) -> None:
        """测试完整的 JobMatch 修饰流程"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            reframes = await engine.reframe_experiences_for_job(match_id)

            assert len(reframes) == 1
            assert reframes[0].job_match_id == match_id
            assert reframes[0].experience_id == exp_id
            assert "供应链" in reframes[0].reframed_summary

            from src.models.database import AsyncSessionLocal
            from sqlalchemy import select
            async with AsyncSessionLocal() as session:
                result = await session.execute(
                    select(JobMatchExperienceReframe).where(
                        JobMatchExperienceReframe.job_match_id == match_id
                    )
                )
                db_reframes = list(result.scalars().all())
                assert len(db_reframes) == 1

        asyncio.run(_test())

    def test_reframe_cache_hit(self, clean_db: Any) -> None:
        """测试缓存命中：第二次调用直接返回缓存"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            reframes1 = await engine.reframe_experiences_for_job(match_id)
            assert len(reframes1) == 1

            reframes2 = await engine.reframe_experiences_for_job(match_id)
            assert len(reframes2) == 1
            assert reframes2[0].id == reframes1[0].id

        asyncio.run(_test())

    def test_reframe_force_refresh(self, clean_db: Any) -> None:
        """测试强制刷新缓存"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            reframes1 = await engine.reframe_experiences_for_job(match_id)
            old_id = reframes1[0].id

            reframes2 = await engine.reframe_experiences_for_job(match_id, force_refresh=True)
            assert len(reframes2) == 1
            assert reframes2[0].job_match_id == match_id

        asyncio.run(_test())


class TestGetAndDelete:
    def test_get_reframed_experiences(self, clean_db: Any) -> None:
        """测试获取已修饰的经历列表"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            await engine.reframe_experiences_for_job(match_id)

            reframes = await engine.get_reframed_experiences(match_id)
            assert len(reframes) == 1
            assert reframes[0].experience_id == exp_id

        asyncio.run(_test())

    def test_delete_reframes(self, clean_db: Any) -> None:
        """测试删除修饰记录"""
        exp_id, persona_id, job_desc_id, match_id = asyncio.run(_create_test_data())

        fake_llm = FakeLLM()
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]

        async def _test() -> None:
            await engine.reframe_experiences_for_job(match_id)

            count = await engine.delete_reframes(match_id)
            assert count == 1

            reframes = await engine.get_reframed_experiences(match_id)
            assert len(reframes) == 0

        asyncio.run(_test())


class TestExtractJSON:
    """测试 JSON 提取鲁棒性"""

    def test_direct_json(self) -> None:
        engine = JDReframeEngine()
        result = engine._extract_json('{"reframed_summary": "test", "reframing_strategy": "s"}')
        assert result["reframed_summary"] == "test"

    def test_markdown_json_block(self) -> None:
        engine = JDReframeEngine()
        text = '```json\n{"reframed_summary": "md test", "reframing_strategy": "s"}\n```'
        result = engine._extract_json(text)
        assert result["reframed_summary"] == "md test"

    def test_plain_markdown_block(self) -> None:
        engine = JDReframeEngine()
        text = '```\n{"reframed_summary": "plain test"}\n```'
        result = engine._extract_json(text)
        assert result["reframed_summary"] == "plain test"

    def test_invalid_fallback(self) -> None:
        engine = JDReframeEngine()
        result = engine._extract_json("这是一段无效文本")
        assert "reframed_summary" in result
        assert result["reframed_summary"] == "这是一段无效文本"

    def test_regex_extraction(self) -> None:
        engine = JDReframeEngine()
        text = '修饰结果如下\n{"reframed_summary": "regex test", "reframing_strategy": "s"}\n希望有帮助'
        result = engine._extract_json(text)
        assert result["reframed_summary"] == "regex test"
