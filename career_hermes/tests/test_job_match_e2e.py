"""
CareerCraft Agent — 岗位匹配+修饰端到端测试

测试覆盖完整链路：
1. 创建角色 + 经历
2. 用 FakeLLM 解析 JD（模拟 JobParser.parse_and_save）
3. 执行 JobMatcher.match() 生成匹配记录
4. 执行 JDReframeEngine.reframe_experiences_for_job() 生成修饰
5. 验证修饰结果已存档到数据库
"""

from __future__ import annotations

import asyncio
import json
from datetime import date
from typing import Any, Dict, List, Optional

import pytest

from src.models.database import init_db, close_db
from src.models.entities import (
    Experience,
    JobDesc,
    JobMatch,
    JobMatchExperienceReframe,
    Persona,
)
from src.services.experience_manager import ExperienceManager, ExperienceDraft
from src.services.job_matcher import JobMatcher
from src.services.job_parser import JobParser
from src.services.jd_reframe_engine import JDReframeEngine
from src.services.persona_engine import PersonaEngine


class FakeLLM:
    """模拟 LLM，支持 JD 解析和经历修饰两种场景"""

    def __init__(self, response_data: Optional[Dict[str, Any]] = None) -> None:
        self.response_data = response_data

    async def chat(self, messages: List[Dict[str, str]], **kwargs: Any) -> str:
        prompt = messages[0].get("content", "")

        # 经历修饰场景：prompt 包含更特异的修饰标记（优先判断）
        if (
            "resume optimizer" in prompt
            or "Original Experience" in prompt
            or "Reframing Rules" in prompt
        ):
            reframe_data = self.response_data or {
                "reframed_summary": "作为高级产品经理，我主导了供应链数字化平台的架构设计与落地，通过数据驱动的决策优化供应链效率，实现成本降低20%。",
                "reframing_strategy": "突出供应链数字化能力，强调成本优化和系统架构经验",
            }
            return json.dumps(reframe_data, ensure_ascii=False)

        # JD 解析场景：prompt 包含 "岗位" 或 "Job Description" 等关键词
        if "岗位" in prompt or "Job Description" in prompt or "职位" in prompt:
            jd_data = self.response_data or {
                "title": "高级产品经理-供应链方向",
                "company": "目标科技有限公司",
                "location": "深圳",
                "years_of_experience": "3-5年",
                "parsed_skills": ["产品规划", "供应链管理", "数据分析", "系统架构"],
                "responsibilities": [
                    "负责供应链数字化平台的产品规划与落地",
                    "优化采购成本，提升供应链效率",
                ],
                "education_requirement": "本科及以上",
                "job_type": "full_time",
            }
            return json.dumps(jd_data, ensure_ascii=False)

        # 默认返回修饰数据
        reframe_data = self.response_data or {
            "reframed_summary": "作为高级产品经理，我主导了供应链数字化平台的架构设计与落地，通过数据驱动的决策优化供应链效率，实现成本降低20%。",
            "reframing_strategy": "突出供应链数字化能力，强调成本优化和系统架构经验",
        }
        return json.dumps(reframe_data, ensure_ascii=False)


@pytest.fixture(scope="session", autouse=True)
def db_setup() -> None:
    """测试级数据库初始化"""
    asyncio.run(init_db())
    yield
    asyncio.run(close_db())


@pytest.fixture
def clean_db() -> None:
    """每个测试后清理所有相关数据"""
    yield
    from src.models.database import AsyncSessionLocal
    from sqlalchemy import delete

    async def _clean() -> None:
        async with AsyncSessionLocal() as session:
            await session.execute(delete(JobMatchExperienceReframe))
            await session.execute(delete(JobMatch))
            await session.execute(delete(JobDesc))
            await session.execute(delete(Experience))
            await session.execute(delete(Persona))
            await session.commit()

    asyncio.run(_clean())


class TestJobMatchE2E:
    """岗位匹配+修饰端到端测试"""

    @pytest.mark.asyncio
    async def test_full_match_and_reframe_pipeline(self, clean_db: Any) -> None:
        """测试完整流程：创建角色/经历 → 解析JD → 匹配 → 修饰"""
        # 1. 创建角色
        persona_engine = PersonaEngine()
        persona = await persona_engine.create(
            name="产品经理-AI方向",
            tone_style="business_insight",
            capability_weights={
                "产品规划": 0.9,
                "数据分析": 0.8,
                "供应链管理": 0.7,
                "系统架构": 0.6,
            },
            target_job_profiles=["深圳 产品经理"],
            max_experiences=5,
            user_id="default",
        )
        assert persona.id is not None

        # 2. 创建经历（直接使用 ExperienceManager 保存草稿）
        draft = ExperienceDraft(
            raw_text="负责核心产品规划和迭代，主导供应链系统重构",
            extracted={
                "title": "高级产品经理",
                "type": "work",
                "organization": "测试科技有限公司",
                "start_date": "2020-07-01",
                "end_date": "2023-06-01",
                "structured_achievements": [
                    "提升DAU 40%，月活突破500万",
                    "主导供应链系统重构，订单处理效率提升3倍",
                ],
                "skills_demonstrated": ["产品规划", "数据分析", "供应链管理"],
            },
        )
        exp_manager = ExperienceManager()
        experience = await exp_manager.confirm_and_save(draft, user_id="default")
        assert experience.id is not None
        assert experience.status == "confirmed"

        # 2.1 计算角色与经历的适配度，生成 RoleExperienceWeight
        weights = await persona_engine.calculate_fit_scores(persona.id)
        assert len(weights) >= 1
        # 确保计算了该经历的权重（分数可能为0，但必须有记录）
        assert weights[0].experience_id == experience.id

        # 3. 用 FakeLLM 解析 JD
        fake_llm = FakeLLM()
        parser = JobParser(llm_router=fake_llm)  # type: ignore[arg-type]
        jd_text = (
            "招聘高级产品经理-供应链方向\n"
            "负责供应链数字化平台的产品规划与落地\n"
            "要求：3-5年经验，熟悉产品规划、供应链管理、数据分析\n"
            "地点：深圳"
        )
        job_desc = await parser.parse_and_save(jd_text, source="manual")
        assert job_desc.id is not None
        assert job_desc.title == "高级产品经理-供应链方向"
        assert "产品规划" in (job_desc.parsed_skills or [])

        # 4. 执行匹配
        matcher = JobMatcher()
        match = await matcher.match(persona_id=persona.id, job_desc_id=job_desc.id)
        assert match.id is not None
        assert match.persona_id == persona.id
        assert match.job_desc_id == job_desc.id
        assert match.match_score >= 0
        assert match.matched_skills is not None
        assert "产品规划" in match.matched_skills
        assert match.tracking_status == "new"

        # 5. 执行修饰
        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]
        reframes = await engine.reframe_experiences_for_job(match.id)
        assert len(reframes) >= 1
        reframe = reframes[0]
        assert reframe.job_match_id == match.id
        assert reframe.experience_id == experience.id
        assert reframe.reframed_summary != ""
        assert reframe.reframing_strategy != ""

    @pytest.mark.asyncio
    async def test_reframe_results_persisted(self, clean_db: Any) -> None:
        """测试修饰结果存档并可查询"""
        # 准备基础数据
        persona_engine = PersonaEngine()
        persona = await persona_engine.create(
            name="技术负责人",
            tone_style="technical_deep",
            capability_weights={
                "Python": 0.9,
                "FastAPI": 0.8,
                "Kubernetes": 0.7,
            },
            target_job_profiles=["北京 后端开发"],
            max_experiences=5,
            user_id="default",
        )

        draft = ExperienceDraft(
            raw_text="搭建企业级AI Agent平台，支持RAG和多轮对话",
            extracted={
                "title": "后端技术负责人",
                "type": "work",
                "organization": "创新科技有限公司",
                "start_date": "2022-01-01",
                "end_date": None,
                "structured_achievements": [
                    "系统QPS达到2000+",
                    "服务10+内部业务线",
                ],
                "skills_demonstrated": ["Python", "FastAPI", "Kubernetes"],
            },
        )
        exp_manager = ExperienceManager()
        experience = await exp_manager.confirm_and_save(draft, user_id="default")

        await persona_engine.calculate_fit_scores(persona.id)

        fake_llm = FakeLLM()
        parser = JobParser(llm_router=fake_llm)  # type: ignore[arg-type]
        job_desc = await parser.parse_and_save(
            "招聘后端技术负责人，要求Python、FastAPI、Kubernetes经验",
            source="manual",
        )

        matcher = JobMatcher()
        match = await matcher.match(persona_id=persona.id, job_desc_id=job_desc.id)

        engine = JDReframeEngine(llm_router=fake_llm)  # type: ignore[arg-type]
        reframes = await engine.reframe_experiences_for_job(match.id)
        assert len(reframes) == 1

        # 验证数据库中可查询
        from src.models.database import AsyncSessionLocal
        from sqlalchemy import select

        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobMatchExperienceReframe).where(
                    JobMatchExperienceReframe.job_match_id == match.id
                )
            )
            db_reframes = list(result.scalars().all())
            assert len(db_reframes) == 1
            db_reframe = db_reframes[0]
            assert db_reframe.experience_id == experience.id
            assert db_reframe.reframed_summary == reframes[0].reframed_summary
            assert db_reframe.reframing_strategy == reframes[0].reframing_strategy

        # 验证通过引擎接口也可查询
        queried = await engine.get_reframed_experiences(match.id)
        assert len(queried) == 1
        assert queried[0].id == reframes[0].id
