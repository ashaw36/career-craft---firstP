"""
CareerCraft Agent — 角色引擎服务

核心职责：角色CRUD、Fit Score计算、经历筛选排序。
"""

from __future__ import annotations

import re
from typing import Any, Dict, List, Optional

from sqlalchemy import select
from sqlalchemy.orm import selectinload

from src.models.database import AsyncSessionLocal
from src.models.entities import Experience, Persona, RoleExperienceWeight


class PersonaEngine:
    """角色引擎"""

    async def create(
        self,
        name: str,
        identity_statement: Optional[str] = None,
        career_narrative: Optional[str] = None,
        tone_style: str = "business_insight",
        capability_weights: Optional[Dict[str, float]] = None,
        target_job_profiles: Optional[List[str]] = None,
        max_experiences: int = 5,
        user_id: str = "default",
    ) -> Persona:
        """创建角色档案"""
        persona = Persona(
            user_id=user_id,
            name=name,
            identity_statement=identity_statement,
            career_narrative=career_narrative,
            tone_style=tone_style,
            capability_weights=capability_weights or {},
            target_job_profiles=target_job_profiles or [],
            max_experiences=max_experiences,
        )
        async with AsyncSessionLocal() as session:
            session.add(persona)
            await session.commit()
            await session.refresh(persona)
        return persona

    async def get_by_id(self, persona_id: str) -> Optional[Persona]:
        """获取角色"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            return result.scalar_one_or_none()

    async def list_by_user(self, user_id: str = "default") -> List[Persona]:
        """列出所有角色"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Persona)
                .where(Persona.user_id == user_id)
                .order_by(Persona.created_at.desc())
            )
            return list(result.scalars().all())

    async def update(self, persona_id: str, **fields: Any) -> Optional[Persona]:
        """更新角色"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = result.scalar_one_or_none()
            if not persona:
                return None
            for key, value in fields.items():
                if hasattr(persona, key):
                    setattr(persona, key, value)
            await session.commit()
            await session.refresh(persona)
            return persona

    async def delete(self, persona_id: str) -> bool:
        """删除角色"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = result.scalar_one_or_none()
            if not persona:
                return False
            await session.delete(persona)
            await session.commit()
            return True

    async def calculate_fit_scores(self, persona_id: str) -> List[RoleExperienceWeight]:
        """
        计算角色与所有经历的适配度

        算法：
        1. 对每条经历，提取其关键词（技能、职位、成就）
        2. 与角色的 capability_weights 进行匹配
        3. relevance_score = 匹配权重和 / 总权重和
        """
        async with AsyncSessionLocal() as session:
            # 加载角色
            persona_result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = persona_result.scalar_one_or_none()
            if not persona:
                return []

            # 加载所有确认状态的经历
            exp_result = await session.execute(
                select(Experience).where(
                    Experience.user_id == persona.user_id,
                    Experience.status == "confirmed",
                )
            )
            experiences = list(exp_result.scalars().all())

            # 获取现有的权重记录（避免重复计算用户已覆盖的）
            existing_result = await session.execute(
                select(RoleExperienceWeight).where(
                    RoleExperienceWeight.persona_id == persona_id
                )
            )
            existing_weights = {
                w.experience_id: w for w in existing_result.scalars().all()
            }

            weights = []
            capability_weights = persona.capability_weights or {}
            total_weight = sum(capability_weights.values()) or 1.0

            for exp in experiences:
                # 提取经历关键词
                exp_keywords = self._extract_keywords(exp)

                # 计算匹配度
                matched_weight = 0.0
                for skill, weight in capability_weights.items():
                    if self._keyword_match(skill, exp_keywords):
                        matched_weight += weight

                score = min(1.0, matched_weight / total_weight)

                # 检查是否已有记录
                if exp.id in existing_weights:
                    rew = existing_weights[exp.id]
                    if not rew.user_overridden:
                        rew.relevance_score = score
                else:
                    rew = RoleExperienceWeight(
                        persona_id=persona_id,
                        experience_id=exp.id,
                        relevance_score=score,
                    )
                    session.add(rew)

                weights.append(rew)

            await session.commit()
            return weights

    async def get_weighted_experiences(
        self,
        persona_id: str,
        min_score: float = 0.0,
        limit: Optional[int] = None,
    ) -> List[RoleExperienceWeight]:
        """
        获取按 Fit Score 排序的经历列表
        """
        async with AsyncSessionLocal() as session:
            stmt = (
                select(RoleExperienceWeight)
                .options(selectinload(RoleExperienceWeight.experience))
                .where(
                    RoleExperienceWeight.persona_id == persona_id,
                    RoleExperienceWeight.relevance_score >= min_score,
                )
                .order_by(RoleExperienceWeight.relevance_score.desc())
            )
            if limit:
                stmt = stmt.limit(limit)
            result = await session.execute(stmt)
            return list(result.scalars().all())

    @staticmethod
    def _extract_keywords(exp: Experience) -> List[str]:
        """
        从经历中提取关键词集合
        """
        keywords = []

        # 技能
        if exp.skills_demonstrated:
            keywords.extend(exp.skills_demonstrated)

        # 职位和公司
        keywords.append(exp.title)
        if exp.organization:
            keywords.append(exp.organization)

        # 成就描述
        if exp.structured_achievements:
            for achievement in exp.structured_achievements:
                keywords.extend(PersonaEngine._tokenize(achievement))

        # 原始描述
        keywords.extend(PersonaEngine._tokenize(exp.raw_description))

        # 去重并转小写
        return list(set(k.lower().strip() for k in keywords if k))

    @staticmethod
    def _tokenize(text: str) -> List[str]:
        """
        简单的分词：提取英文单词和中文词组
        """
        if not text:
            return []
        # 提取英文单词
        english_words = re.findall(r"[a-zA-Z]+", text)
        # 提取中文词组（连续中文字符）
        chinese_words = re.findall(r"[\u4e00-\u9fff]+", text)
        return english_words + chinese_words

    @staticmethod
    def _keyword_match(skill: str, keywords: List[str]) -> bool:
        """
        检查技能是否在关键词列表中
        """
        skill_lower = skill.lower().strip()
        for kw in keywords:
            if skill_lower in kw or kw in skill_lower:
                return True
        return False

    async def update_fit_score(
        self,
        persona_id: str,
        experience_id: str,
        score: float,
    ) -> Optional[RoleExperienceWeight]:
        """
        手动更新角色-经历的 Fit Score。

        如果记录已存在则更新 score 并设置 user_overridden=True，
        否则创建新记录。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(RoleExperienceWeight).where(
                    RoleExperienceWeight.persona_id == persona_id,
                    RoleExperienceWeight.experience_id == experience_id,
                )
            )
            rew = result.scalar_one_or_none()
            if rew:
                rew.relevance_score = max(0.0, min(1.0, score))
                rew.user_overridden = True
            else:
                rew = RoleExperienceWeight(
                    persona_id=persona_id,
                    experience_id=experience_id,
                    relevance_score=max(0.0, min(1.0, score)),
                    user_overridden=True,
                )
                session.add(rew)
            await session.commit()
            await session.refresh(rew)
            return rew
