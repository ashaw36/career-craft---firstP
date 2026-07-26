"""
CareerCraft Agent — 经历重述引擎

核心职责：根据角色档案和目标岗位，使用 LLM 将经历重新叙事，突出与目标岗位的匹配度。
Sprint 4 核心服务之二。
"""

from __future__ import annotations

import logging
from typing import List, Optional

from sqlalchemy import select

from src.llm.prompts.retelling import build_retelling_prompt
from src.llm.router import LLMError, LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import (
    Experience,
    JobDesc,
    Persona,
    RoleExperienceWeight,
)

logger = logging.getLogger(__name__)


class RetellingError(Exception):
    """经历重述异常基类"""

    def __init__(self, message: str, experience_id: str = "") -> None:
        super().__init__(message)
        self.experience_id = experience_id


class RetellingEngine:
    """
    经历重述引擎

    使用流程：
        engine = RetellingEngine()
        # 单条经历重述
        reframed = await engine.retell_experience(
            experience_id="xxx",
            persona_id="yyy",
            job_desc_id="zzz",
        )
        # 批量重述（针对某个岗位的所有相关经历）
        results = await engine.retell_for_job(
            persona_id="yyy",
            job_desc_id="zzz",
        )
    """

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    async def retell_experience(
        self,
        experience_id: str,
        persona_id: str,
        job_desc_id: str,
    ) -> str:
        """
        对单条经历进行重述。

        Args:
            experience_id: 经历 ID
            persona_id: 角色 ID
            job_desc_id: 目标岗位 ID

        Returns:
            重述后的文本

        Raises:
            RetellingError: 实体不存在或 LLM 调用失败时抛出
        """
        async with AsyncSessionLocal() as session:
            # 加载实体
            exp_result = await session.execute(
                select(Experience).where(Experience.id == experience_id)
            )
            experience = exp_result.scalar_one_or_none()
            if not experience:
                raise RetellingError(
                    f"经历不存在: {experience_id}", experience_id=experience_id
                )

            persona_result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = persona_result.scalar_one_or_none()
            if not persona:
                raise RetellingError(
                    f"角色不存在: {persona_id}", experience_id=experience_id
                )

            job_result = await session.execute(
                select(JobDesc).where(JobDesc.id == job_desc_id)
            )
            job_desc = job_result.scalar_one_or_none()
            if not job_desc:
                raise RetellingError(
                    f"岗位描述不存在: {job_desc_id}", experience_id=experience_id
                )

            # 构建 prompt
            prompt = build_retelling_prompt(
                persona_name=persona.name,
                identity_statement=persona.identity_statement or "",
                tone_style=persona.tone_style or "business_insight",
                job_title=job_desc.title or "",
                job_company=job_desc.company or "",
                job_skills=", ".join(job_desc.parsed_skills or []),
                exp_type=experience.type,
                exp_title=experience.title,
                exp_organization=experience.organization or "",
                exp_description=experience.raw_description,
                exp_achievements="\n".join(
                    f"- {a}" for a in (experience.structured_achievements or [])
                ),
                exp_skills=", ".join(experience.skills_demonstrated or []),
            )

            # 调用 LLM
            try:
                response = await self.llm.chat(
                    messages=[{"role": "user", "content": prompt}],
                    temperature=0.6,
                )
            except LLMError as e:
                logger.error(
                    "经历重述 LLM 调用失败: exp=%s error=%s", experience_id, e
                )
                raise RetellingError(
                    f"LLM 调用失败: {e}", experience_id=experience_id
                ) from e

            if not isinstance(response, str):
                raise RetellingError(
                    "LLM 返回不是文本类型",
                    experience_id=experience_id,
                )

            reframed_text = response.strip()

            # 更新或创建 RoleExperienceWeight 记录
            weight_result = await session.execute(
                select(RoleExperienceWeight).where(
                    RoleExperienceWeight.persona_id == persona_id,
                    RoleExperienceWeight.experience_id == experience_id,
                )
            )
            weight = weight_result.scalar_one_or_none()
            if weight:
                weight.reframed_summary = reframed_text
            else:
                weight = RoleExperienceWeight(
                    persona_id=persona_id,
                    experience_id=experience_id,
                    reframed_summary=reframed_text,
                )
                session.add(weight)

            await session.commit()
            logger.info(
                "经历重述已保存: exp=%s persona=%s job=%s",
                experience_id,
                persona_id,
                job_desc_id,
            )
            return reframed_text

    async def retell_for_job(
        self,
        persona_id: str,
        job_desc_id: str,
        min_relevance: float = 0.0,
        limit: Optional[int] = None,
    ) -> List[RoleExperienceWeight]:
        """
        针对目标岗位，批量重述角色下的所有相关经历。

        Args:
            persona_id: 角色 ID
            job_desc_id: 目标岗位 ID
            min_relevance: 最低相关度阈值，仅重述 relevance_score >= 阈值的经历
            limit: 最大重述数量

        Returns:
            重述后的 RoleExperienceWeight 列表
        """
        async with AsyncSessionLocal() as session:
            stmt = (
                select(RoleExperienceWeight)
                .where(
                    RoleExperienceWeight.persona_id == persona_id,
                    RoleExperienceWeight.relevance_score >= min_relevance,
                )
                .order_by(RoleExperienceWeight.relevance_score.desc())
            )
            if limit:
                stmt = stmt.limit(limit)

            result = await session.execute(stmt)
            weights = list(result.scalars().all())

        # 在会话外逐条重述，避免会话过长持有
        reframed_weights = []
        for weight in weights:
            try:
                await self.retell_experience(
                    experience_id=weight.experience_id,
                    persona_id=persona_id,
                    job_desc_id=job_desc_id,
                )
                reframed_weights.append(weight)
            except RetellingError as e:
                logger.warning(
                    "经历重述跳过: exp=%s error=%s",
                    weight.experience_id,
                    e,
                )
                continue

        return reframed_weights

    async def get_reframed_summary(
        self,
        persona_id: str,
        experience_id: str,
    ) -> Optional[str]:
        """
        获取某条经历已重述的摘要。

        Returns:
            重述摘要文本，如果未重述则返回 None
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(RoleExperienceWeight).where(
                    RoleExperienceWeight.persona_id == persona_id,
                    RoleExperienceWeight.experience_id == experience_id,
                )
            )
            weight = result.scalar_one_or_none()
            return weight.reframed_summary if weight else None
