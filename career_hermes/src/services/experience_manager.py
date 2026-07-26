"""
CareerCraft Agent — 经历管理服务

核心职责：经历CRUD、对话式录入、时间冲突检测、草稿生命周期。
"""

from __future__ import annotations

import json
from datetime import date
from typing import Any, Dict, List, Optional, cast

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from src.llm.prompts.experience_extraction import build_extraction_prompt
from src.llm.router import LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import Experience


class TimeConflictError(Exception):
    """经历时间冲突异常。

    属性:
        conflicts: 与当前经历时间重叠的 Experience 列表
    """

    def __init__(self, message: str, conflicts: List[Experience]) -> None:
        super().__init__(message)
        self.conflicts = conflicts


class ExperienceDraft:
    """经历草稿，待用户确认"""

    def __init__(self, raw_text: str, extracted: Dict[str, Any]) -> None:
        self.raw_text = raw_text
        self.extracted = extracted
        self.user_edits: Optional[Dict[str, Any]] = None

    @property
    def title(self) -> str:
        return str(self.extracted.get("title", ""))

    @property
    def skills(self) -> List[str]:
        return list(self.extracted.get("skills_demonstrated", []) or [])

    @property
    def period(self) -> str:
        s = self.extracted.get("start_date", "")
        e = self.extracted.get("end_date", "")
        if s and e:
            return f"{s} ~ {e}"
        return str(s or e or "")

    def to_dict(self) -> Dict[str, Any]:
        data = dict(self.extracted)
        data["raw_description"] = self.raw_text
        if self.user_edits:
            data.update(self.user_edits)
        return data


class ExperienceManager:
    """经历管理器"""

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    async def create_draft(self, raw_text: str) -> ExperienceDraft:
        """
        对话式录入：原始文本 → LLM结构化提取 → 草稿
        """
        prompt = build_extraction_prompt(raw_text)
        response = cast(str, await self.llm.chat(
            messages=[{"role": "user", "content": prompt}],
            json_mode=True,
        ))

        try:
            extracted = json.loads(response)
        except json.JSONDecodeError:
            # 尝试从 markdown 代码块中提取
            if "```json" in response:
                json_str = response.split("```json")[1].split("```")[0].strip()
                extracted = json.loads(json_str)
            elif "```" in response:
                json_str = response.split("```")[1].split("```")[0].strip()
                extracted = json.loads(json_str)
            else:
                raise ValueError(f"LLM 返回不是有效JSON: {response[:200]}")

        # 基础校验
        if not extracted.get("title"):
            raise ValueError("结构化结果缺少必填字段: title")

        return ExperienceDraft(raw_text=raw_text, extracted=extracted)

    async def confirm_and_save(
        self,
        draft: ExperienceDraft,
        user_id: str = "default",
    ) -> Experience:
        """
        用户确认后正式保存经历
        """
        data = draft.to_dict()

        # 日期解析
        start_date = self._parse_date(data.get("start_date"))
        end_date = self._parse_date(data.get("end_date"))

        async with AsyncSessionLocal() as session:
            # 冲突检测
            conflicts = await self._check_time_conflicts(
                session, user_id, start_date, end_date
            )
            if conflicts:
                conflict_titles = ", ".join([e.title for e in conflicts])
                raise TimeConflictError(
                    f"时间冲突: 与已有经历重叠 ({conflict_titles})",
                    conflicts,
                )

            exp = Experience(
                user_id=user_id,
                type=data.get("type", "work"),
                title=data["title"],
                organization=data.get("organization"),
                start_date=start_date,
                end_date=end_date,
                raw_description=data["raw_description"],
                structured_achievements=data.get("structured_achievements"),
                skills_demonstrated=data.get("skills_demonstrated"),
                metrics=data.get("metrics"),
                status="confirmed",
            )
            session.add(exp)
            await session.commit()
            await session.refresh(exp)
            return exp

    async def list_by_user(
        self,
        user_id: str = "default",
        status_filter: Optional[str] = None,
    ) -> List[Experience]:
        """
        按时间倒序列出经历
        """
        async with AsyncSessionLocal() as session:
            stmt = (
                select(Experience)
                .where(Experience.user_id == user_id)
                .order_by(Experience.end_date.desc().nullsfirst())
            )
            if status_filter:
                stmt = stmt.where(Experience.status == status_filter)
            result = await session.execute(stmt)
            return list(result.scalars().all())

    async def get_by_id(self, exp_id: str) -> Optional[Experience]:
        """根据ID获取经历"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Experience).where(Experience.id == exp_id)
            )
            return result.scalar_one_or_none()

    async def update(self, exp_id: str, **fields: Any) -> Optional[Experience]:
        """更新经历。

        若更新了日期字段，自动检测时间冲突。
        """
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Experience).where(Experience.id == exp_id)
            )
            exp = result.scalar_one_or_none()
            if not exp:
                return None

            for key, value in fields.items():
                if hasattr(exp, key):
                    setattr(exp, key, value)

            # 如果更新了日期，检测冲突
            new_start = fields.get("start_date", exp.start_date)
            new_end = fields.get("end_date", exp.end_date)
            if "start_date" in fields or "end_date" in fields:
                conflicts = await self._check_time_conflicts(
                    session,
                    exp.user_id,
                    new_start,
                    new_end,
                    exclude_id=exp_id,
                )
                if conflicts:
                    conflict_titles = ", ".join([e.title for e in conflicts])
                    raise TimeConflictError(
                        f"时间冲突: 与已有经历重叠 ({conflict_titles})",
                        conflicts,
                    )

            await session.commit()
            await session.refresh(exp)
            return exp

    async def delete(self, exp_id: str) -> bool:
        """删除经历（软删除，改为 archived 状态）"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Experience).where(Experience.id == exp_id)
            )
            exp = result.scalar_one_or_none()
            if not exp:
                return False
            exp.status = "archived"
            await session.commit()
            return True

    async def hard_delete(self, exp_id: str) -> bool:
        """硬删除（尚未确认的草稿可硬删除）"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(Experience).where(Experience.id == exp_id)
            )
            exp = result.scalar_one_or_none()
            if not exp:
                return False
            await session.delete(exp)
            await session.commit()
            return True

    async def _check_time_conflicts(
        self,
        session: AsyncSession,
        user_id: str,
        start: Optional[date],
        end: Optional[date],
        exclude_id: Optional[str] = None,
    ) -> List[Experience]:
        """
        检测时间区间重叠
        """
        if not start:
            return []

        stmt = select(Experience).where(
            Experience.user_id == user_id,
            Experience.status.in_(["confirmed", "draft"]),
        )

        if exclude_id:
            stmt = stmt.where(Experience.id != exclude_id)

        # 重叠条件：新经历的开始在现有经历期间内，或结束在现有经历期间内，或覆盖现有经历
        # 简化判断：新经历的开始日期 ≤ 现有经历的结束日期 且 新经历的结束日期 ≥ 现有经历的开始日期
        result = await session.execute(stmt)
        all_exps = list(result.scalars().all())

        conflicts = []
        for exp in all_exps:
            if not exp.start_date:
                continue
            exp_end = exp.end_date or date.today()
            new_end = end or date.today()

            if start <= exp_end and new_end >= exp.start_date:
                conflicts.append(exp)

        return conflicts

    @staticmethod
    def _parse_date(date_str: Optional[str]) -> Optional[date]:
        """解析日期字符串"""
        if not date_str:
            return None
        try:
            return date.fromisoformat(date_str)
        except (ValueError, TypeError):
            return None
