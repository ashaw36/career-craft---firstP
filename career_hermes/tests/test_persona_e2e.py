"""
CareerCraft Agent — 角色端到端测试

测试链路：创建角色 → 创建经历 → 计算 Fit Score → 验证角色切换效果
"""

from __future__ import annotations

import asyncio
from typing import Any, Dict, List

import pytest

from src.models.database import init_db, close_db
from src.models.entities import Persona, Experience, RoleExperienceWeight
from src.services.persona_engine import PersonaEngine
from src.services.experience_manager import ExperienceManager, ExperienceDraft


@pytest.fixture(scope="session", autouse=True)
def db_setup() -> None:
    """测试级数据库初始化"""
    asyncio.run(init_db())
    yield
    asyncio.run(close_db())


@pytest.fixture
def clean_db() -> None:
    """每个测试前后清理所有数据，确保隔离"""
    from src.models.database import AsyncSessionLocal
    from sqlalchemy import delete

    async def _clean() -> None:
        async with AsyncSessionLocal() as session:
            await session.execute(delete(RoleExperienceWeight))
            await session.execute(delete(Experience))
            await session.execute(delete(Persona))
            await session.commit()

    asyncio.run(_clean())
    yield
    asyncio.run(_clean())


class TestPersonaE2E:
    @pytest.mark.asyncio
    async def test_fit_score_differs_by_persona(self, clean_db: Any) -> None:
        """不同角色下同一经历的 Fit Score 不同"""
        engine = PersonaEngine()
        manager = ExperienceManager()

        # 1. 创建两个能力权重分布完全不同的角色
        persona_tech = await engine.create(
            name="技术型角色",
            identity_statement="专注技术深度",
            capability_weights={"Python": 1.0, "后端开发": 0.5},
        )
        persona_product = await engine.create(
            name="产品型角色",
            identity_statement="专注产品规划",
            capability_weights={"产品规划": 1.0, "数据分析": 0.5},
        )

        # 2. 创建两条带有不同技能标签的经历
        draft_tech = ExperienceDraft(
            raw_text="负责后端API开发，使用Python和FastAPI构建高并发服务",
            extracted={
                "title": "后端工程师",
                "type": "work",
                "organization": "科技公司",
                "skills_demonstrated": ["Python", "FastAPI"],
                "structured_achievements": ["系统QPS达到2000+"],
            },
        )
        draft_product = ExperienceDraft(
            raw_text="负责产品规划和数据分析，驱动用户增长策略",
            extracted={
                "title": "产品经理",
                "type": "work",
                "organization": "互联网公司",
                "skills_demonstrated": ["产品规划", "数据分析"],
                "structured_achievements": ["DAU提升40%"],
            },
        )

        exp_tech = await manager.confirm_and_save(draft_tech)
        exp_product = await manager.confirm_and_save(draft_product)

        # 3. 为每个角色计算 Fit Score
        weights_tech_persona = await engine.calculate_fit_scores(persona_tech.id)
        weights_product_persona = await engine.calculate_fit_scores(persona_product.id)

        # 4. 验证同一经历在不同角色下分数不同
        def _to_map(weights: List[RoleExperienceWeight]) -> Dict[str, float]:
            return {w.experience_id: w.relevance_score for w in weights}

        map_tech = _to_map(weights_tech_persona)
        map_product = _to_map(weights_product_persona)

        # 技术经历在技术型角色下分数应显著高于产品型角色
        assert map_tech[exp_tech.id] > map_product[exp_tech.id], (
            "技术经历在技术型角色下 Fit Score 应更高"
        )
        # 产品经历在产品型角色下分数应显著高于技术型角色
        assert map_product[exp_product.id] > map_tech[exp_product.id], (
            "产品经历在产品型角色下 Fit Score 应更高"
        )

    @pytest.mark.asyncio
    async def test_role_switch_reorders_experiences(self, clean_db: Any) -> None:
        """切换角色后经历排序发生变化"""
        engine = PersonaEngine()
        manager = ExperienceManager()

        # 1. 创建角色：技术型重视 Python，产品型重视产品规划
        persona_tech = await engine.create(
            name="技术型角色",
            identity_statement="专注技术深度",
            capability_weights={"Python": 1.0},
        )
        persona_product = await engine.create(
            name="产品型角色",
            identity_statement="专注产品规划",
            capability_weights={"产品规划": 1.0},
        )

        # 2. 创建两条技能标签互异的经历
        draft_tech = ExperienceDraft(
            raw_text="使用Python进行后端开发，优化系统性能",
            extracted={
                "title": "后端工程师",
                "type": "work",
                "organization": "A公司",
                "skills_demonstrated": ["Python"],
            },
        )
        draft_product = ExperienceDraft(
            raw_text="负责产品规划和迭代，协调多方资源推进项目",
            extracted={
                "title": "产品经理",
                "type": "work",
                "organization": "B公司",
                "skills_demonstrated": ["产品规划"],
            },
        )

        exp_tech = await manager.confirm_and_save(draft_tech)
        exp_product = await manager.confirm_and_save(draft_product)

        # 3. 计算 Fit Score
        await engine.calculate_fit_scores(persona_tech.id)
        await engine.calculate_fit_scores(persona_product.id)

        # 4. 获取按 Fit Score 排序的经历列表
        ordered_tech = await engine.get_weighted_experiences(persona_tech.id)
        ordered_product = await engine.get_weighted_experiences(persona_product.id)

        tech_ids = [w.experience_id for w in ordered_tech]
        product_ids = [w.experience_id for w in ordered_product]

        # 验证技术角色下技术经历排在首位，产品角色下产品经历排在首位
        assert tech_ids[0] == exp_tech.id, "技术角色下技术经历应排在首位"
        assert product_ids[0] == exp_product.id, "产品角色下产品经历应排在首位"

        # 因为只有两条经历，排序应互为反转
        assert tech_ids == list(reversed(product_ids)), (
            "切换角色后经历顺序应完全反转"
        )
