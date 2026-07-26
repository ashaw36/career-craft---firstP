"""
CareerCraft Agent — 简历生成引擎

核心职责：根据角色档案筛选经历 → 排序 → 渲染模板 → 导出 Markdown/PDF。
"""

from __future__ import annotations

import logging
from datetime import date
from pathlib import Path
from typing import Any, Dict, List, Optional

from jinja2 import Environment, FileSystemLoader, select_autoescape

from src.llm.router import LLMRouter
from src.models.entities import Experience, Persona, RoleExperienceWeight
from src.services.persona_engine import PersonaEngine

logger = logging.getLogger(__name__)

class ResumeBuilder:
    """
    简历生成引擎

    使用流程：
        builder = ResumeBuilder(persona_id="xxx")
        md = await builder.render(template_name="modern")
        builder.export_to_file(md, "/path/to/resume.md")
    """

    def __init__(self, persona_id: str, llm_router: Optional[LLMRouter] = None) -> None:
        self.persona_id = persona_id
        self.llm = llm_router or LLMRouter()
        self.persona_engine = PersonaEngine()
        self._persona: Optional[Persona] = None
        self._experiences: List[RoleExperienceWeight] = []

    async def prepare(self) -> "ResumeBuilder":
        """
        预处理：加载角色和筛选经历
        """
        self._persona = await self.persona_engine.get_by_id(self.persona_id)
        if not self._persona:
            raise ValueError(f"角色不存在: {self.persona_id}")

        # 计算并获取按 Fit Score 排序的经历
        await self.persona_engine.calculate_fit_scores(self.persona_id)
        min_score = getattr(self._persona, "min_relevance_score", None) or 0.05
        self._experiences = await self.persona_engine.get_weighted_experiences(
            self.persona_id,
            min_score=min_score,
            limit=self._persona.max_experiences,
        )

        # Fallback: 若过滤后为空，降级 min_score 直到有结果
        if not self._experiences:
            logger.warning(
                "简历生成: min_score=%.2f 过滤后经历为空，尝试降级阈值", min_score
            )
            for fallback_score in (0.0,):
                self._experiences = await self.persona_engine.get_weighted_experiences(
                    self.persona_id,
                    min_score=fallback_score,
                    limit=self._persona.max_experiences,
                )
                if self._experiences:
                    logger.info(
                        "简历生成: fallback min_score=%.2f 后获取 %d 条经历",
                        fallback_score,
                        len(self._experiences),
                    )
                    break

        # 最终 Fallback: 若仍为空，直接加载所有 confirmed 经历
        if not self._experiences:
            logger.warning("简历生成: RoleExperienceWeight 为空，直接加载 confirmed 经历")
            from src.models.database import AsyncSessionLocal
            from src.models.entities import Experience
            from sqlalchemy import select
            async with AsyncSessionLocal() as session:
                result = await session.execute(
                    select(Experience).where(
                        Experience.user_id == self._persona.user_id,
                        Experience.status == "confirmed",
                    ).order_by(Experience.start_date.desc())
                    .limit(self._persona.max_experiences)
                )
                exps = list(result.scalars().all())
                # 包装为 RoleExperienceWeight 的兼容结构
                for exp in exps:
                    self._experiences.append(self._SimpleWeight(exp))
                logger.info("简历生成: 直接加载 %d 条 confirmed 经历", len(exps))

        return self

    class _SimpleWeight:
        """Fallback 用的简化权重包装，兼容 RoleExperienceWeight 接口"""
        def __init__(self, experience: Experience) -> None:
            self.experience = experience
            self.relevance_score = 0.5
            self.reframed_summary = None
            self.highlighted_skills = None

    async def render(
        self,
        template_name: str = "modern",
        format: str = "markdown",
    ) -> str:
        """
        渲染简历

        Args:
            template_name: 模板名称（存放于 src/ui/templates/resume/）
            format: 输出格式，目前支持 "markdown"

        Returns:
            渲染后的文本
        """
        if not self._persona:
            await self.prepare()

        # 构建渲染上下文
        ctx = await self._build_context()

        # 加载模板
        template_dir = Path(__file__).parent.parent / "ui" / "templates" / "resume"
        env = Environment(
            loader=FileSystemLoader(str(template_dir)),
            autoescape=select_autoescape(["html", "xml"]),
        )
        template = env.get_template(f"{template_name}.md.j2")

        return template.render(**ctx)

    async def _build_context(self) -> Dict[str, Any]:
        """构建 Jinja2 渲染上下文"""
        persona = self._persona
        assert persona is not None

        # 身份声明
        identity = persona.identity_statement or await self._generate_identity()

        # 职业叙事
        narrative = persona.career_narrative or ""

        # 经历列表（按类型分组）
        work_exps = []
        project_exps = []
        education_exps = []
        for rew in self._experiences:
            exp = rew.experience
            if exp is None:
                logger.warning("简历生成: RoleExperienceWeight 缺少 experience 对象，跳过")
                continue
            # 优先使用重述，fallback 到原始描述
            description = rew.reframed_summary or exp.raw_description or ""
            entry = {
                "title": exp.title,
                "organization": exp.organization,
                "type": exp.type,
                "period": self._format_period(exp.start_date, exp.end_date),
                "description": description,
                "achievements": exp.structured_achievements or [],
                "skills": rew.highlighted_skills or exp.skills_demonstrated or [],
                "metrics": exp.metrics or [],
                "relevance_score": rew.relevance_score,
            }
            if exp.type == "work":
                work_exps.append(entry)
            elif exp.type == "project":
                project_exps.append(entry)
            elif exp.type == "education":
                education_exps.append(entry)
            else:
                work_exps.append(entry)

        return {
            "name": persona.name,
            "identity_statement": identity,
            "career_narrative": narrative,
            "tone_style": persona.tone_style,
            "capability_weights": persona.capability_weights or {},
            "target_job_profiles": persona.target_job_profiles or [],
            "experiences": work_exps + project_exps + education_exps,
            "work_experiences": work_exps,
            "project_experiences": project_exps,
            "education_experiences": education_exps,
            "generated_at": date.today().isoformat(),
        }

    async def _generate_identity(self) -> str:
        """
        使用 LLM 生成身份声明
        """
        if not self._experiences:
            return ""

        # 抽取关键信息
        exp_summaries = []
        for rew in self._experiences:
            exp = rew.experience
            exp_summaries.append(f"- {exp.title} @ {exp.organization}: {exp.raw_description[:100]}")

        prompt = f"""你是一个职业简历撰写专家。根据以下经历，生成一段1-2句的身份声明（Identity Statement）。

经历：
{chr(10).join(exp_summaries)}

角色侧重：{self._persona.tone_style if self._persona else 'business_insight'}

要求：
- 简洁有力，突出核心竞争力
- 不超过80个中文字符
- 不要套话

请直接返回身份声明文本，不要其他内容。
"""
        response = await self.llm.chat(
            messages=[{"role": "user", "content": prompt}],
            temperature=0.5,
        )
        return response.strip() if isinstance(response, str) else ""

    def export_to_file(self, content: str, filepath: str) -> Path:
        """导出到文件

        安全校验：解析相对路径，防止路径穿越。
        """
        path = Path(filepath).resolve()
        if ".." in path.parts:
            raise ValueError("文件路径不安全，包含非法的 .. 组件")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    @staticmethod
    def _format_period(start: Optional[date], end: Optional[date]) -> str:
        """格式化时间区间"""
        start_str = start.strftime("%Y.%m") if start else "?"
        end_str = end.strftime("%Y.%m") if end else "至今"
        return f"{start_str} — {end_str}"
