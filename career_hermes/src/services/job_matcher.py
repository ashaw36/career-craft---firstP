"""
CareerCraft Agent — 岗位匹配服务

核心职责：比较角色档案 + 经历 vs JD，计算综合匹配度分数（0-100），
分析匹配/缺失技能，存入 JobMatch 模型。
Sprint 4 核心服务之三。

打分算法（升级版）：
1. 技能匹配（50分）：基础匹配×40 + 等级加成×10
2. 经验匹配（25分）：年限满足度×15 + 时间衰减加权×10
3. 文本相似度（15分）：简历文本 vs JD 描述的 TF-IDF 余弦相似度
4. 其他（10分）：学历 + 地点
"""

from __future__ import annotations

import json
import logging
import math
import re
from collections import Counter
from datetime import date
from typing import Any, Dict, List, Optional, Tuple

from sqlalchemy import select
from sqlalchemy.orm import selectinload

from src.llm.router import LLMError, LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import Experience, JobDesc, JobMatch, Persona

logger = logging.getLogger(__name__)


class JobMatchError(Exception):
    """岗位匹配异常基类"""

    def __init__(self, message: str, persona_id: str = "", job_desc_id: str = "") -> None:
        super().__init__(message)
        self.persona_id = persona_id
        self.job_desc_id = job_desc_id


class JobMatcher:
    """
    岗位匹配器

    使用流程：
        matcher = JobMatcher()
        match = await matcher.match(persona_id="xxx", job_desc_id="yyy")
        report = await matcher.get_match_report(match.id)
    """

    # 技能等级权重映射
    SKILL_LEVEL_WEIGHTS = {
        "精通": 1.0,
        "master": 1.0,
        "expert": 1.0,
        "熟悉": 0.6,
        "proficient": 0.6,
        "advanced": 0.6,
        "了解": 0.3,
        "familiar": 0.3,
        "intermediate": 0.4,
        "入门": 0.1,
        "beginner": 0.1,
        "novice": 0.1,
    }

    # 允许的投递状态
    _VALID_STATUSES = {
        "new",
        "interested",
        "applied",
        "interviewing",
        "offered",
        "rejected",
        "ghosted",
        "accepted",
        "declined",
    }

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    async def match(self, persona_id: str, job_desc_id: str) -> JobMatch:
        """
        计算匹配度并保存 JobMatch。

        升级算法：
        1. 技能匹配（50分）：基础匹配 + 等级加成
        2. 经验匹配（25分）：年限满足度 + 时间衰减
        3. 文本相似度（15分）：TF-IDF 余弦相似度
        4. 其他（10分）：学历 + 地点
        """
        async with AsyncSessionLocal() as session:
            # 加载角色
            persona_result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = persona_result.scalar_one_or_none()
            if not persona:
                raise JobMatchError(f"角色不存在: {persona_id}", persona_id=persona_id)

            # 加载 JD
            job_result = await session.execute(
                select(JobDesc).where(JobDesc.id == job_desc_id)
            )
            job_desc = job_result.scalar_one_or_none()
            if not job_desc:
                raise JobMatchError(
                    f"岗位描述不存在: {job_desc_id}", job_desc_id=job_desc_id
                )

            # 加载角色的所有确认经历
            exp_result = await session.execute(
                select(Experience).where(
                    Experience.user_id == persona.user_id,
                    Experience.status == "confirmed",
                )
            )
            experiences = list(exp_result.scalars().all())

            # 从角色的 capability_weights 提取技能列表
            persona_skills = list((persona.capability_weights or {}).keys())
            job_skills = await self._extract_required_skills(job_desc, persona_skills)

            # 计算各项分数
            matched, missing, skill_score = self._calculate_skill_match(
                persona_skills, job_skills, persona.capability_weights or {}
            )
            exp_score = self._calculate_experience_match(experiences, job_desc)
            text_score = self._calculate_text_similarity(experiences, job_desc)
            other_score = self._calculate_other_match(persona, job_desc)

            total = min(100.0, skill_score + exp_score + text_score + other_score)

            breakdown: Dict[str, Any] = {
                "skill": round(skill_score, 2),
                "experience": round(exp_score, 2),
                "text_similarity": round(text_score, 2),
                "other": round(other_score, 2),
                "required_skills": job_skills,
            }

            # 检查是否已有匹配记录，有则更新
            existing_result = await session.execute(
                select(JobMatch).where(
                    JobMatch.persona_id == persona_id,
                    JobMatch.job_desc_id == job_desc_id,
                )
            )
            existing = existing_result.scalar_one_or_none()

            if existing:
                existing.match_score = int(total)
                existing.matched_skills = matched
                existing.missing_skills = missing
                existing.score_breakdown = breakdown
                await session.commit()
                loaded_existing = await self._load_match_with_relationships(session, existing.id)
                logger.info(
                    "JobMatch 已更新: id=%s score=%s",
                    existing.id,
                    existing.match_score,
                )
                return loaded_existing or existing

            match = JobMatch(
                persona_id=persona_id,
                job_desc_id=job_desc_id,
                match_score=int(total),
                matched_skills=matched,
                missing_skills=missing,
                score_breakdown=breakdown,
                ai_analysis="",
                tracking_status="new",
            )
            session.add(match)
            await session.commit()
            loaded_match = await self._load_match_with_relationships(session, match.id)
            logger.info(
                "JobMatch 已创建: id=%s score=%s",
                match.id,
                match.match_score,
            )
            return loaded_match or match

    @staticmethod
    async def _load_match_with_relationships(session: Any, match_id: str) -> Optional[JobMatch]:
        result = await session.execute(
            select(JobMatch)
            .options(selectinload(JobMatch.persona), selectinload(JobMatch.job_desc))
            .where(JobMatch.id == match_id)
        )
        return result.scalar_one_or_none()

    async def _extract_required_skills(
        self, job_desc: JobDesc, persona_skills: List[str]
    ) -> List[str]:
        """
        Extract JD skill requirements with LLM first, then fall back to parsed skills
        and explicit capability keyword hits in the raw JD text.
        """
        raw_text = job_desc.raw_text or ""
        parsed_skills = self._normalize_skill_list(job_desc.parsed_skills or [])
        llm_skills: List[str] = []

        if raw_text.strip():
            prompt = self._build_skill_extraction_prompt(raw_text)
            try:
                response = await self.llm.chat(
                    messages=[{"role": "user", "content": prompt}],
                    json_mode=True,
                    temperature=0.2,
                )
                if isinstance(response, str):
                    llm_skills = self._parse_skill_extraction_response(response)
            except (LLMError, json.JSONDecodeError, TypeError, ValueError) as exc:
                logger.warning("JD 技能需求 LLM 抽取失败，使用降级逻辑: %s", exc)

        text_hits = self._extract_persona_skill_hits(raw_text, persona_skills)
        return self._merge_skills(llm_skills, parsed_skills, text_hits)

    @staticmethod
    def _build_skill_extraction_prompt(raw_text: str) -> str:
        return f"""你是一位招聘 JD 分析助手。请从以下岗位描述中提取岗位明确要求或强相关的技能、工具、方法论、业务领域能力。

要求：
1. 只输出 JSON 对象，不要解释。
2. 字段 `required_skills` 必须是字符串数组。
3. 技能名称保持简洁，例如 Python、SQL、产品规划、供应链管理。
4. 不要输出学历、城市、薪资、福利、软性泛词。

岗位描述：
{raw_text[:4000]}
"""

    @staticmethod
    def _parse_skill_extraction_response(response: str) -> List[str]:
        text = response.strip()
        try:
            if "```json" in text:
                text = text.split("```json", 1)[1].split("```", 1)[0].strip()
            elif "```" in text:
                text = text.split("```", 1)[1].split("```", 1)[0].strip()
        except (IndexError, ValueError):
            # markdown 代码块不完整，尝试直接解析
            pass

        data = json.loads(text)
        if isinstance(data, dict):
            raw_skills = (
                data.get("required_skills")
                or data.get("skills")
                or data.get("parsed_skills")
                or []
            )
        elif isinstance(data, list):
            raw_skills = data
        else:
            raw_skills = []
        return JobMatcher._normalize_skill_list(raw_skills)

    @staticmethod
    def _normalize_skill_list(raw_skills: Any) -> List[str]:
        skills: List[str] = []
        if not isinstance(raw_skills, list):
            return skills
        for item in raw_skills:
            if isinstance(item, str):
                value = item.strip()
            elif isinstance(item, dict):
                value = str(item.get("name") or item.get("skill") or "").strip()
            else:
                value = str(item).strip()
            if value:
                skills.append(value)
        return JobMatcher._merge_skills(skills)

    @staticmethod
    def _extract_persona_skill_hits(raw_text: str, persona_skills: List[str]) -> List[str]:
        text = (raw_text or "").lower()
        hits: List[str] = []
        for skill in persona_skills:
            normalized = str(skill or "").strip()
            if normalized and normalized.lower() in text:
                hits.append(normalized)
        return hits

    @staticmethod
    def _merge_skills(*skill_groups: List[str]) -> List[str]:
        merged: List[str] = []
        seen = set()
        for group in skill_groups:
            for skill in group or []:
                value = str(skill or "").strip()
                key = value.lower()
                if value and key not in seen:
                    seen.add(key)
                    merged.append(value)
        return merged

    def _calculate_skill_match(
        self,
        persona_skills: List[str],
        job_skills: List[str],
        capability_weights: Optional[Dict[str, Any]] = None,
    ) -> Tuple[List[str], List[str], float]:
        """
        计算技能匹配度（0-50分）。

        基础匹配（0-40分）：JD 技能在 Persona 技能中的比例
        等级加成（0-10分）：匹配技能的等级权重加成
        """
        if not job_skills:
            return [], [], 25.0  # JD 无技能要求，给中等分

        _weights = capability_weights or {}
        persona_skills_lower = [s.lower().strip() for s in persona_skills]
        matched = []
        missing = []
        level_bonus = 0.0

        for skill in job_skills:
            skill_lower = skill.lower().strip()
            if skill_lower in persona_skills_lower:
                matched.append(skill)
                # 等级加成
                level_bonus += self._get_skill_level_weight(skill_lower, _weights)
            else:
                # 子串宽松匹配
                found = any(
                    skill_lower in ps or ps in skill_lower
                    for ps in persona_skills_lower
                )
                if found:
                    matched.append(skill)
                    level_bonus += self._get_skill_level_weight(skill_lower, _weights) * 0.7
                else:
                    missing.append(skill)

        # 基础分：匹配比例 × 40
        base_score = (len(matched) / len(job_skills)) * 40.0

        # 等级加成：平均等级权重 × 10，最高 10 分
        if matched:
            level_score = min(10.0, (level_bonus / len(matched)) * 10.0)
        else:
            level_score = 0.0

        return matched, missing, base_score + level_score

    def _get_skill_level_weight(self, skill_lower: str, capability_weights: Dict[str, Any]) -> float:
        """获取技能等级权重"""
        # 先从 capability_weights 中查找
        for raw_skill, weight in capability_weights.items():
            if skill_lower in raw_skill.lower() or raw_skill.lower() in skill_lower:
                # 尝试解析等级
                raw_str = str(raw_skill).lower()
                for level, w in self.SKILL_LEVEL_WEIGHTS.items():
                    if level in raw_str:
                        return w
                # 无等级标注，用数值权重映射（0.0-1.0）
                if isinstance(weight, (int, float)):
                    return min(1.0, max(0.0, float(weight)))
                return 0.5
        return 0.5  # 默认中等

    def _calculate_experience_match(
        self, experiences: List[Experience], job_desc: JobDesc
    ) -> float:
        """
        计算经验匹配度（0-25分）。

        年限满足度（0-15分）：根据经历总年限是否满足 JD 要求
        时间衰减加权（0-10分）：近期经历权重更高
        """
        required_years = JobMatcher._parse_years_requirement(
            job_desc.years_of_experience
        )

        if not experiences:
            return 0.0

        # 计算带有时间衰减的总年限
        total_months = 0
        weighted_months = 0
        today = date.today()

        for exp in experiences:
            start = exp.start_date
            end = exp.end_date or today
            if not start or end < start:
                continue
            months = (end.year - start.year) * 12 + (end.month - start.month)
            months = max(0, months)
            total_months += months

            # 时间衰减：根据经历结束时间距今天的年数
            years_ago = (today.year - end.year) + (today.month - end.month) / 12.0
            if years_ago <= 3:
                decay = 1.0
            elif years_ago <= 5:
                decay = 0.8
            else:
                decay = 0.6
            weighted_months += months * decay

        total_years = total_months / 12.0
        weighted_years = weighted_months / 12.0

        # 年限满足度（0-15分）：用加权年限与要求年限比较
        if required_years <= 0:
            year_score = 15.0
        elif weighted_years >= required_years:
            year_score = 15.0
        else:
            year_score = (weighted_years / required_years) * 15.0

        # 时间衰减奖励（0-10分）：加权年限 / 原始年限 的比例
        # 如果全部经历都是近期的，比例接近1.0，得满分
        # 如果很多老旧经历，比例降低
        if total_years > 0:
            recency_ratio = weighted_years / total_years
            recency_score = recency_ratio * 10.0
        else:
            recency_score = 0.0

        return min(25.0, year_score + recency_score)

    def _calculate_text_similarity(
        self, experiences: List[Experience], job_desc: JobDesc
    ) -> float:
        """
        计算文本相似度（0-15分）。

        简化 TF-IDF + 余弦相似度：
        - 将所有经历描述拼接为"简历文本"
        - 与 JD 描述计算词频向量的余弦相似度
        """
        if not job_desc.raw_text:
            return 7.5  # 无 JD 文本，给中等分

        # 拼接经历文本
        resume_parts = []
        for exp in experiences:
            resume_parts.append(exp.title or "")
            resume_parts.append(exp.raw_description or "")
            if exp.structured_achievements:
                resume_parts.extend(exp.structured_achievements)
        resume_text = "\n".join(resume_parts)

        if not resume_text.strip():
            return 0.0

        similarity = self._cosine_similarity(resume_text, job_desc.raw_text)
        return similarity * 15.0

    @staticmethod
    def _tokenize(text: str) -> List[str]:
        """简化分词：提取中文字符和英文单词"""
        # 中文字符
        chinese_chars = re.findall(r"[\u4e00-\u9fff]+", text)
        # 英文单词
        english_words = re.findall(r"[a-zA-Z]{2,}", text.lower())
        # 中文按字分割
        chinese_words = []
        for segment in chinese_chars:
            chinese_words.extend(list(segment))
        return english_words + chinese_words

    @classmethod
    def _cosine_similarity(cls, text1: str, text2: str) -> float:
        """计算两段文本的余弦相似度（0.0 ~ 1.0）"""
        words1 = cls._tokenize(text1)
        words2 = cls._tokenize(text2)

        if not words1 or not words2:
            return 0.0

        freq1 = Counter(words1)
        freq2 = Counter(words2)

        all_words = set(freq1.keys()) | set(freq2.keys())

        vec1 = [freq1.get(w, 0) for w in all_words]
        vec2 = [freq2.get(w, 0) for w in all_words]

        dot_product = sum(a * b for a, b in zip(vec1, vec2))
        norm1 = math.sqrt(sum(a * a for a in vec1))
        norm2 = math.sqrt(sum(a * a for vec in [vec2] for a in vec))

        if norm1 == 0 or norm2 == 0:
            return 0.0

        return dot_product / (norm1 * norm2)

    @staticmethod
    def _calculate_other_match(persona: Persona, job_desc: JobDesc) -> float:
        """
        其他匹配度（0-10分）：学历 + 地点。
        """
        score = 0.0

        # 学历匹配（5分）
        edu_requirement = job_desc.education_requirement or ""
        if edu_requirement:
            score += 3.0
        else:
            score += 5.0

        # 地点匹配（5分）
        if job_desc.location and persona.target_job_profiles:
            target_str = " ".join(persona.target_job_profiles).lower()
            if job_desc.location.lower() in target_str:
                score += 5.0
            else:
                score += 2.5
        else:
            score += 5.0

        return min(10.0, score)

    @staticmethod
    def _parse_years_requirement(years_str: Optional[str]) -> float:
        """
        解析工作年限字符串，返回最小要求年数。
        """
        if not years_str:
            return 0.0

        text = years_str.strip()
        numbers = re.findall(r"\d+", text)
        if not numbers:
            return 0.0

        nums = [int(n) for n in numbers]

        if len(nums) >= 2:
            return float(min(nums))

        return float(nums[0])

    @staticmethod
    def _calculate_total_years(experiences: List[Experience]) -> float:
        """计算经历总年限（去重叠）。"""
        total_months = 0
        today = date.today()

        for exp in experiences:
            start = exp.start_date
            end = exp.end_date or today
            if not start:
                continue
            if end < start:
                continue
            months = (end.year - start.year) * 12 + (end.month - start.month)
            total_months += max(0, months)

        return total_months / 12.0

    async def get_match_report(self, match_id: str) -> str:
        """获取匹配报告。"""
        async with AsyncSessionLocal() as session:
            match = await session.get(JobMatch, match_id)
            if not match:
                return f"未找到匹配记录: {match_id}"

            lines = [
                "## 岗位匹配报告",
                "",
                f"- 匹配 ID: {match.id}",
                f"- 角色 ID: {match.persona_id}",
                f"- 岗位 ID: {match.job_desc_id}",
                f"- **综合匹配度: {match.match_score}/100**",
                "",
                "### 分项得分",
                f"- 技能匹配: {match.score_breakdown.get('skill', 0)} / 50",
                f"- 经验匹配: {match.score_breakdown.get('experience', 0)} / 25",
                f"- 文本相似度: {match.score_breakdown.get('text_similarity', 0)} / 15",
                f"- 其他匹配: {match.score_breakdown.get('other', 0)} / 10",
                "",
                f"### 匹配技能 ({len(match.matched_skills or [])} 个)",
            ]
            for skill in match.matched_skills or []:
                lines.append(f"- ✅ {skill}")

            lines.extend([
                "",
                f"### 缺失技能 ({len(match.missing_skills or [])} 个)",
            ])
            for skill in match.missing_skills or []:
                lines.append(f"- ❌ {skill}")

            lines.extend([
                "",
                "### 投递状态",
                f"{match.tracking_status}",
            ])

            if match.ai_analysis:
                lines.extend(["", "### AI 分析", f"{match.ai_analysis}"])

            return "\n".join(lines)

    async def list_matches_by_job(self, job_desc_id: str) -> List[JobMatch]:
        """列出某个岗位的所有匹配记录，按匹配度倒序。"""
        async with AsyncSessionLocal() as session:
            stmt = (
                select(JobMatch)
                .options(selectinload(JobMatch.persona), selectinload(JobMatch.job_desc))
                .where(JobMatch.job_desc_id == job_desc_id)
                .order_by(JobMatch.match_score.desc())
            )
            result = await session.execute(stmt)
            return list(result.scalars().all())

    async def list_matches(self, persona_id: str) -> List[JobMatch]:
        """列出角色的所有匹配记录，按匹配度倒序。"""
        async with AsyncSessionLocal() as session:
            stmt = (
                select(JobMatch)
                .options(selectinload(JobMatch.persona), selectinload(JobMatch.job_desc))
                .where(JobMatch.persona_id == persona_id)
                .order_by(JobMatch.match_score.desc())
            )
            result = await session.execute(stmt)
            return list(result.scalars().all())

    async def update_tracking_status(
        self, match_id: str, status: str
    ) -> Optional[JobMatch]:
        """更新投递状态。"""
        if status not in self._VALID_STATUSES:
            logger.warning("无效的投递状态: %s", status)
            return None

        async with AsyncSessionLocal() as session:
            match = await session.get(JobMatch, match_id)
            if not match:
                return None

            match.tracking_status = status
            await session.commit()
            await session.refresh(match)
            logger.info("JobMatch 状态已更新: id=%s status=%s", match_id, status)
            return match

    async def delete_match(self, match_id: str) -> bool:
        """删除匹配记录（硬删除）"""
        async with AsyncSessionLocal() as session:
            match = await session.get(JobMatch, match_id)
            if not match:
                return False
            await session.delete(match)
            await session.commit()
            logger.info("JobMatch 已删除: id=%s", match_id)
            return True
