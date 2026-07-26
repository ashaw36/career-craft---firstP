"""
CareerCraft Agent — 简历生成端到端测试

测试链路：创建角色 → 创建经历 → 简历渲染 → 验证输出
"""

from __future__ import annotations

import asyncio
from datetime import date
from typing import Any, Dict, List, Optional

import pytest

from src.models.database import init_db, close_db
from src.models.entities import Experience, Persona
from src.services.experience_manager import ExperienceManager, ExperienceDraft
from src.services.persona_engine import PersonaEngine
from src.services.resume_builder import ResumeBuilder


@pytest.fixture(scope="session", autouse=True)
def db_setup() -> None:
    """测试级数据库初始化"""
    asyncio.run(init_db())
    yield
    asyncio.run(close_db())


@pytest.fixture
def clean_db() -> None:
    """每个测试后清理数据，确保测试隔离"""
    yield
    from src.models.database import AsyncSessionLocal
    from sqlalchemy import delete
    from src.models.entities import (
        Experience,
        Persona,
        RoleExperienceWeight,
        JobDesc,
        JobMatch,
    )

    async def _clean() -> None:
        async with AsyncSessionLocal() as session:
            await session.execute(delete(RoleExperienceWeight))
            await session.execute(delete(JobMatch))
            await session.execute(delete(JobDesc))
            await session.execute(delete(Experience))
            await session.execute(delete(Persona))
            await session.commit()

    asyncio.run(_clean())


class TestResumeE2E:
    """简历生成端到端测试类"""

    @pytest.mark.asyncio
    async def test_generate_resume_with_experiences(self, clean_db: Any) -> None:
        """正常流程：创建角色和多条 confirmed 经历，验证简历 Markdown 包含经历内容"""
        persona_engine = PersonaEngine()
        exp_manager = ExperienceManager()

        # 1. 创建角色
        # 使用空 capability_weights 触发 fallback，确保所有 confirmed 经历都被加载
        persona = await persona_engine.create(
            name="测试产品经理",
            identity_statement="专注于AI产品的策略与落地",
            capability_weights={},
            tone_style="business_insight",
            max_experiences=5,
        )

        # 2. 创建经历（confirmed 状态）
        draft1 = ExperienceDraft(
            raw_text="负责核心产品规划和迭代，主导3个重点项目",
            extracted={
                "title": "高级产品经理",
                "type": "work",
                "organization": "测试科技有限公司",
                "start_date": "2020-07-01",
                "end_date": "2021-12-01",
                "raw_description": "负责核心产品规划和迭代，主导3个重点项目",
                "structured_achievements": ["提升DAU 40%", "月活突破500万"],
                "skills_demonstrated": ["产品规划", "数据分析", "项目管理"],
            },
        )
        exp1 = await exp_manager.confirm_and_save(draft1)
        assert exp1.status == "confirmed"

        draft2 = ExperienceDraft(
            raw_text="搭建企业级AI Agent平台，支持RAG和多轮对话",
            extracted={
                "title": "AI Agent平台项目",
                "type": "project",
                "organization": "测试科技有限公司",
                "start_date": "2022-01-01",
                "end_date": None,
                "raw_description": "搭建企业级AI Agent平台，支持RAG和多轮对话",
                "structured_achievements": ["系统QPS达到2000+", "服务10+内部业务线"],
                "skills_demonstrated": ["Python", "FastAPI", "LangChain"],
            },
        )
        exp2 = await exp_manager.confirm_and_save(draft2)
        assert exp2.status == "confirmed"

        draft3 = ExperienceDraft(
            raw_text="负责后端系统架构设计，支撑百万级用户",
            extracted={
                "title": "后端工程师",
                "type": "work",
                "organization": "某初创公司",
                "start_date": "2018-03-01",
                "end_date": "2020-06-01",
                "raw_description": "负责后端系统架构设计，支撑百万级用户",
                "structured_achievements": ["系统稳定性达到99.99%"],
                "skills_demonstrated": ["Java", "微服务", "高并发"],
            },
        )
        exp3 = await exp_manager.confirm_and_save(draft3)
        assert exp3.status == "confirmed"

        # 3. 创建 ResumeBuilder，调用 prepare() + render()
        builder = ResumeBuilder(persona_id=persona.id)
        await builder.prepare()
        assert len(builder._experiences) >= 2, "prepare() 应加载至少2条经历"

        # 绕过 LLM 身份声明生成，避免调用外部 LLM
        builder._persona.identity_statement = "测试身份声明"
        md = await builder.render(template_name="modern")

        # 4. 验证输出 Markdown 包含经历内容
        # 验证第一条工作经历
        assert "高级产品经理" in md
        assert "测试科技有限公司" in md
        assert "负责核心产品规划和迭代" in md
        assert "提升DAU 40%" in md
        assert "产品规划" in md
        assert "2020.07" in md

        # 验证第二条项目经历
        assert "AI Agent平台项目" in md
        assert "搭建企业级AI Agent平台" in md
        assert "系统QPS达到2000+" in md
        assert "2022.01" in md

        # 验证第三条工作经历
        assert "后端工程师" in md
        assert "某初创公司" in md
        assert "负责后端系统架构设计" in md
        assert "系统稳定性达到99.99%" in md

    @pytest.mark.asyncio
    async def test_generate_resume_excludes_draft(self, clean_db: Any) -> None:
        """draft 状态经历不应出现在简历中"""
        persona_engine = PersonaEngine()
        exp_manager = ExperienceManager()

        # 1. 创建角色
        persona = await persona_engine.create(
            name="测试角色",
            identity_statement="测试身份声明",
            capability_weights={"测试": 1.0},
            tone_style="business_insight",
            max_experiences=5,
        )

        # 2. 创建 confirmed 经历
        confirmed_draft = ExperienceDraft(
            raw_text="已确认的经历描述",
            extracted={
                "title": "Confirmed经历",
                "type": "work",
                "organization": "确认公司",
                "start_date": "2021-01-01",
                "end_date": "2022-12-01",
                "raw_description": "这是一段已确认的经历描述。",
                "structured_achievements": ["成就1"],
                "skills_demonstrated": ["技能1"],
            },
        )
        await exp_manager.confirm_and_save(confirmed_draft)

        # 3. 直接创建 draft 状态经历（不通过 confirm_and_save）
        from src.models.database import AsyncSessionLocal

        async with AsyncSessionLocal() as session:
            draft_exp = Experience(
                user_id="default",
                type="work",
                title="Draft经历",
                organization="草稿公司",
                start_date=date(2021, 1, 1),
                end_date=date(2022, 12, 1),
                raw_description="这是草稿经历，不应出现在简历中。",
                status="draft",
            )
            session.add(draft_exp)
            await session.commit()

        # 4. 生成简历
        builder = ResumeBuilder(persona_id=persona.id)
        await builder.prepare()
        builder._persona.identity_statement = "测试身份声明"
        md = await builder.render(template_name="modern")

        # 5. 验证：confirmed 经历存在，draft 经历不存在
        assert "Confirmed经历" in md
        assert "确认公司" in md

        assert "Draft经历" not in md
        assert "草稿公司" not in md
        assert "这是草稿经历" not in md
