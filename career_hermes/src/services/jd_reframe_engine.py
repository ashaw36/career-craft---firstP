"""
CareerCraft Agent — JD 导向经历修饰引擎

核心职责：根据岗位 JD 要求，对候选人的经历进行针对性修饰重写，
使其更突出与 JD 匹配的技能和经验，并将结果存档关联到对应岗位。
"""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional

from sqlalchemy import select
from sqlalchemy.orm import selectinload

from src.llm.router import LLMError, LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import (
    Experience,
    JobDesc,
    JobMatch,
    JobMatchExperienceReframe,
    Persona,
    RoleExperienceWeight,
)

logger = logging.getLogger(__name__)


class JDReframeError(Exception):
    """JD 修饰异常基类"""

    def __init__(self, message: str, match_id: str = "") -> None:
        super().__init__(message)
        self.match_id = match_id


class JDReframeEngine:
    """
    JD 导向经历修饰引擎

    使用流程：
        engine = JDReframeEngine()
        reframes = await engine.reframe_experiences_for_job(match_id="xxx")
        # 或获取已修饰的经历
        reframes = await engine.get_reframed_experiences(match_id="xxx")
    """

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    async def reframe_experiences_for_job(
        self,
        match_id: str,
        force_refresh: bool = False,
    ) -> List[JobMatchExperienceReframe]:
        """
        为某个 JobMatch 下的所有相关经历生成修饰版本。

        Args:
            match_id: JobMatch 记录 ID
            force_refresh: 是否强制重新修饰（即使已有缓存）

        Returns:
            修饰后的经历列表
        """
        async with AsyncSessionLocal() as session:
            # 1. 加载 JobMatch 及关联数据
            match_result = await session.execute(
                select(JobMatch)
                .options(selectinload(JobMatch.persona), selectinload(JobMatch.job_desc))
                .where(JobMatch.id == match_id)
            )
            match = match_result.scalar_one_or_none()
            if not match:
                raise JDReframeError(f"匹配记录不存在: {match_id}", match_id=match_id)

            persona = match.persona
            job_desc = match.job_desc
            if not persona or not job_desc:
                raise JDReframeError("匹配记录缺少角色或 JD 信息", match_id=match_id)

            # 2. 检查是否已有缓存
            if not force_refresh:
                existing = await session.execute(
                    select(JobMatchExperienceReframe)
                    .options(selectinload(JobMatchExperienceReframe.experience))
                    .where(
                        JobMatchExperienceReframe.job_match_id == match_id
                    )
                )
                cached = list(existing.scalars().all())
                if cached:
                    logger.info("使用已缓存的修饰经历: match_id=%s, count=%d", match_id, len(cached))
                    return cached

            # 3. 加载角色的经历（通过 RoleExperienceWeight 排序）
            stmt = (
                select(RoleExperienceWeight)
                .options(selectinload(RoleExperienceWeight.experience))
                .where(
                    RoleExperienceWeight.persona_id == persona.id,
                    RoleExperienceWeight.relevance_score >= 0.1,
                )
                .order_by(RoleExperienceWeight.relevance_score.desc())
                .limit(8)  # 最多修饰 8 条经历
            )
            rew_result = await session.execute(stmt)
            role_weights = list(rew_result.scalars().all())

            if not role_weights:
                logger.warning("角色无可修饰经历: persona_id=%s", persona.id)
                return []

            # 4. 对每条经历进行修饰
            reframes: List[JobMatchExperienceReframe] = []
            for rew in role_weights:
                exp = rew.experience
                if not exp:
                    continue

                try:
                    original = rew.reframed_summary or exp.raw_description or ""
                    reframe = await self._reframe_single_experience(
                        experience=exp,
                        job_desc=job_desc,
                        persona=persona,
                        match_id=match_id,
                        original_summary=original,
                    )
                    reframes.append(reframe)
                except Exception as exc:
                    logger.error("修饰经历失败: exp_id=%s error=%s", exp.id, exc)
                    continue

            # 5. 批量保存
            session.add_all(reframes)
            await session.commit()
            reframe_ids = [r.id for r in reframes]
            refreshed_result = await session.execute(
                select(JobMatchExperienceReframe)
                .options(selectinload(JobMatchExperienceReframe.experience))
                .where(JobMatchExperienceReframe.id.in_(reframe_ids))
                .order_by(JobMatchExperienceReframe.created_at.desc())
            )
            loaded_reframes = list(refreshed_result.scalars().all())
            logger.info(
                "JD 经历修饰完成: match_id=%s, count=%d", match_id, len(reframes)
            )
            return loaded_reframes

    async def _reframe_single_experience(
        self,
        experience: Experience,
        job_desc: JobDesc,
        persona: Persona,
        match_id: str,
        original_summary: str = "",
    ) -> JobMatchExperienceReframe:
        """
        使用 LLM 对单条经历进行 JD 导向修饰。
        """
        if not original_summary:
            original_summary = experience.raw_description or ""

        prompt = self._build_reframe_prompt(
            experience=experience,
            job_desc=job_desc,
            persona=persona,
            original_summary=original_summary,
        )

        try:
            response = await self.llm.chat(
                messages=[{"role": "user", "content": prompt}],
                json_mode=True,
                temperature=0.4,
            )
        except LLMError as exc:
            raise JDReframeError(f"LLM 调用失败: {exc}", match_id=match_id) from exc

        parsed = self._extract_json(response)
        reframed_summary = parsed.get("reframed_summary", original_summary)
        strategy = parsed.get("reframing_strategy", "")

        return JobMatchExperienceReframe(
            job_match_id=match_id,
            experience_id=experience.id,
            original_summary=original_summary,
            reframed_summary=reframed_summary,
            reframing_strategy=strategy,
        )

    @staticmethod
    def _build_reframe_prompt(
        experience: Experience,
        job_desc: JobDesc,
        persona: Persona,
        original_summary: str,
    ) -> str:
        """构建经历修饰的 LLM Prompt。"""
        achievements_str = ""
        if experience.structured_achievements:
            achievements_str = "\n".join(f"- {a}" for a in experience.structured_achievements)

        skills_str = ""
        if experience.skills_demonstrated:
            skills_str = "、".join(experience.skills_demonstrated)

        jd_skills_str = ""
        if job_desc.parsed_skills:
            jd_skills_str = "、".join(job_desc.parsed_skills)

        jd_responsibilities_str = ""
        if job_desc.responsibilities:
            jd_responsibilities_str = "\n".join(f"- {r}" for r in job_desc.responsibilities)

        tone = persona.tone_style or "business_insight"
        tone_mapping = {
            "business_insight": "商业洞察力风格，突出业务成果和战略思维",
            "technical_deep": "技术深度风格，突出架构设计和技术难点解决",
            "data_driven": "数据驱动风格，突出量化成果和 A/B 测试经验",
        }
        tone_desc = tone_mapping.get(tone, "专业、简洁的职场表达风格")

        period = ""
        if experience.start_date:
            start = experience.start_date.strftime("%Y.%m")
            end = experience.end_date.strftime("%Y.%m") if experience.end_date else "至今"
            period = f"{start} — {end}"

        return f"""You are an expert resume optimizer. Your task is to rewrite a candidate's experience description to better align with a specific job description (JD), while maintaining complete factual accuracy.

## Original Experience
- Title: {experience.title}
- Organization: {experience.organization or "N/A"}
- Period: {period}
- Original Description: {original_summary}
- Key Achievements:
{achievements_str or "- (none provided)"}
- Skills: {skills_str or "N/A"}

## Target Job Description
- Title: {job_desc.title or "N/A"}
- Company: {job_desc.company or "N/A"}
- Required Skills: {jd_skills_str or "N/A"}
- Key Responsibilities:
{jd_responsibilities_str or "- (none provided)"}
- Full JD: {job_desc.raw_text[:800]}

## Persona Style
{tone_desc}

## Reframing Rules
1. **Factual accuracy**: Never invent facts, metrics, or projects that did not exist.
2. **Keyword alignment**: Naturally incorporate JD-relevant keywords and skills into the description.
3. **Impact focus**: Use STAR-like structure (Situation, Task, Action, Result) where possible.
4. **Length**: Keep the rewritten description within 150-250 Chinese characters (or 80-150 English words).
5. **Style**: Match the persona style specified above.
6. **Highlight fit**: Emphasize aspects of the experience that directly address the JD's requirements.

## Output Format
Return a JSON object with exactly these keys:
{{
    "reframed_summary": "The rewritten experience description...",
    "reframing_strategy": "Brief strategy note, e.g. 突出数据分析能力，强调与JD匹配的用户增长经验"
}}"""

    @staticmethod
    def _extract_json(response: Any) -> Dict[str, Any]:
        """从 LLM 响应中提取 JSON。"""
        text = response.strip() if isinstance(response, str) else ""

        try:
            return json.loads(text)
        except json.JSONDecodeError:
            pass

        # 尝试从 markdown 代码块提取
        if "```json" in text:
            try:
                json_str = text.split("```json")[1].split("```")[0].strip()
                return json.loads(json_str)
            except (IndexError, json.JSONDecodeError):
                pass

        if "```" in text:
            try:
                json_str = text.split("```")[1].split("```")[0].strip()
                return json.loads(json_str)
            except (IndexError, json.JSONDecodeError):
                pass

        # 尝试正则提取 JSON 对象
        import re

        match = re.search(r"\{.*\}", text, re.DOTALL)
        if match:
            try:
                return json.loads(match.group())
            except json.JSONDecodeError:
                pass

        logger.warning("无法解析 LLM 响应为 JSON: %s", text[:200])
        return {"reframed_summary": text, "reframing_strategy": "原文返回"}

    async def get_reframed_experiences(
        self, match_id: str
    ) -> List[JobMatchExperienceReframe]:
        """
        获取某个 JobMatch 下已修饰的经历列表。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobMatchExperienceReframe)
                .options(
                    selectinload(JobMatchExperienceReframe.job_match),
                    selectinload(JobMatchExperienceReframe.experience),
                )
                .where(JobMatchExperienceReframe.job_match_id == match_id)
                .order_by(JobMatchExperienceReframe.created_at.desc())
            )
            return list(result.scalars().all())

    async def delete_reframes(self, match_id: str) -> int:
        """
        删除某个 JobMatch 下的所有修饰记录。
        返回删除的记录数量。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobMatchExperienceReframe).where(
                    JobMatchExperienceReframe.job_match_id == match_id
                )
            )
            reframes = result.scalars().all()
            count = 0
            for r in reframes:
                await session.delete(r)
                count += 1
            await session.commit()
            logger.info("删除修饰记录: match_id=%s, count=%d", match_id, count)
            return count

    async def update_reframe(
        self,
        reframe_id: str,
        reframed_summary: str,
    ) -> Optional[JobMatchExperienceReframe]:
        """
        手动更新单条重述的 reframed_summary。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobMatchExperienceReframe)
                .options(selectinload(JobMatchExperienceReframe.experience))
                .where(JobMatchExperienceReframe.id == reframe_id)
            )
            reframe = result.scalar_one_or_none()
            if not reframe:
                return None
            reframe.reframed_summary = reframed_summary
            await session.commit()
            refreshed = await session.execute(
                select(JobMatchExperienceReframe)
                .options(selectinload(JobMatchExperienceReframe.experience))
                .where(JobMatchExperienceReframe.id == reframe_id)
            )
            return refreshed.scalar_one_or_none()

    async def reset_reframe(self, reframe_id: str) -> bool:
        """
        删除单条重述记录，下次会自动重新走 LLM 生成。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobMatchExperienceReframe).where(
                    JobMatchExperienceReframe.id == reframe_id
                )
            )
            reframe = result.scalar_one_or_none()
            if not reframe:
                return False
            await session.delete(reframe)
            await session.commit()
            return True
