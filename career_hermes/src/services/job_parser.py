"""
CareerCraft Agent — JD 解析服务

核心职责：接收岗位描述文本，使用 LLM 提取关键信息，存入 JobDesc 模型。
Sprint 4 核心服务之一。
"""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional

from sqlalchemy import select

from src.llm.prompts.job_parsing import build_job_parsing_prompt
from src.llm.router import LLMError, LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import JobDesc

logger = logging.getLogger(__name__)


class JobParsingError(Exception):
    """JD 解析异常基类"""

    def __init__(self, message: str, raw_text: str = "") -> None:
        super().__init__(message)
        self.raw_text = raw_text


class JobParser:
    """
    岗位描述解析器

    使用流程：
        parser = JobParser()
        job_desc = await parser.parse_and_save(raw_text="...")
        # 或先解析再确认
        parsed = await parser.parse(raw_text="...")
        job_desc = await parser.save(parsed, source="manual")
    """

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    async def parse(self, raw_text: str) -> Dict[str, Any]:
        """
        将原始 JD 文本解析为结构化字典。

        Args:
            raw_text: 原始岗位描述文本

        Returns:
            包含岗位字段的字典，如 title、company、parsed_skills 等

        Raises:
            JobParsingError: LLM 返回异常或 JSON 解析失败时抛出
        """
        if not raw_text or not raw_text.strip():
            raise JobParsingError("JD 原文为空", raw_text=raw_text)

        prompt = build_job_parsing_prompt(raw_text)

        try:
            response = await self.llm.chat(
                messages=[{"role": "user", "content": prompt}],
                json_mode=True,
                temperature=0.3,
            )
        except LLMError as e:
            logger.error("LLM 调用失败: %s", e)
            raise JobParsingError(f"LLM 调用失败: {e}", raw_text=raw_text) from e

        if not isinstance(response, str):
            raise JobParsingError(
                "LLM 返回不是文本类型，可能是流式输出",
                raw_text=raw_text,
            )

        return self._extract_json(response, raw_text)

    async def parse_and_save(
        self,
        raw_text: str,
        source: str = "manual",
        url: Optional[str] = None,
    ) -> JobDesc:
        """
        一步式解析并保存 JD。

        Args:
            raw_text: 原始岗位描述
            source: 来源，如 manual / crawler_boss / crawler_liepin
            url: 岗位链接

        Returns:
            已持久化的 JobDesc 实体
        """
        parsed = await self.parse(raw_text)
        return await self.save(parsed, source=source, url=url, raw_text=raw_text)

    async def save(
        self,
        parsed: Dict[str, Any],
        source: str = "manual",
        url: Optional[str] = None,
        raw_text: str = "",
    ) -> JobDesc:
        """
        将解析结果持久化为 JobDesc 实体。

        Args:
            parsed: LLM 解析后的结构化字典
            source: 来源标识
            url: 岗位链接
            raw_text: 原始文本（如果 parsed 中无 raw_text 则使用此参数）

        Returns:
            已保存的 JobDesc 实体
        """
        job = JobDesc(
            raw_text=parsed.get("raw_text") or raw_text,
            title=parsed.get("title"),
            company=parsed.get("company"),
            years_of_experience=parsed.get("years_of_experience"),
            salary_range=parsed.get("salary_range"),
            location=parsed.get("location"),
            job_type=parsed.get("job_type"),
            education_requirement=parsed.get("education_requirement"),
            responsibilities=parsed.get("responsibilities"),
            benefits=parsed.get("benefits"),
            url=url,
            parsed_skills=parsed.get("parsed_skills"),
            source=source,
        )

        async with AsyncSessionLocal() as session:
            session.add(job)
            await session.commit()
            await session.refresh(job)
            logger.info("JobDesc 已保存: id=%s title=%s", job.id, job.title)
            return job

    async def get_by_id(self, job_id: str) -> Optional[JobDesc]:
        """根据 ID 获取 JD"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobDesc).where(JobDesc.id == job_id)
            )
            return result.scalar_one_or_none()

    async def list_all(
        self,
        source_filter: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[JobDesc]:
        """
        列出 JD 列表，支持来源过滤和分页。
        """
        async with AsyncSessionLocal() as session:
            stmt = select(JobDesc).order_by(JobDesc.created_at.desc())
            if source_filter:
                stmt = stmt.where(JobDesc.source == source_filter)
            stmt = stmt.limit(limit).offset(offset)
            result = await session.execute(stmt)
            return list(result.scalars().all())

    async def update(self, job_id: str, **fields: Any) -> Optional[JobDesc]:
        """更新 JD 字段"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobDesc).where(JobDesc.id == job_id)
            )
            job = result.scalar_one_or_none()
            if not job:
                return None

            for key, value in fields.items():
                if hasattr(job, key):
                    setattr(job, key, value)

            await session.commit()
            await session.refresh(job)
            logger.info("JobDesc 已更新: id=%s", job_id)
            return job

    async def delete(self, job_id: str) -> bool:
        """删除 JD（硬删除）"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(JobDesc).where(JobDesc.id == job_id)
            )
            job = result.scalar_one_or_none()
            if not job:
                return False
            await session.delete(job)
            await session.commit()
            logger.info("JobDesc 已删除: id=%s", job_id)
            return True

    @staticmethod
    def _extract_json(response: str, raw_text: str) -> Dict[str, Any]:
        """
        从 LLM 响应中提取 JSON。

        先尝试直接解析，失败时尝试从 markdown 代码块中提取。
        """
        text = response.strip() if isinstance(response, str) else ""

        # 直接解析
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

        raise JobParsingError(
            f"LLM 返回不是有效 JSON: {text[:200]}",
            raw_text=raw_text,
        )
