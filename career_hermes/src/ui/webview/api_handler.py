"""
CareerCraft Agent — WebView API 同步适配层

将异步 Service 封装为同步 API，供 QWebChannel Bridge 调用。
在后台线程中运行 async 代码，避免与 Qt 主事件循环冲突。
"""

from __future__ import annotations

import asyncio
import logging
import os
from concurrent.futures import TimeoutError as FutureTimeoutError
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Dict, List, Optional

from sqlalchemy import func, select

from src.models.database import AsyncSessionLocal
from src.models.entities import JobMatch, LearningPath
from src.services.experience_manager import ExperienceManager, TimeConflictError
from src.services.job_matcher import JobMatcher
from src.services.learning_recommender import LearningRecommender
from src.services.persona_engine import PersonaEngine
from src.services.resume_builder import ResumeBuilder
from src.services.skill_graph import SkillGraph
from src.utils.security import SecureStorage

logger = logging.getLogger(__name__)


class AsyncRunnerTimeoutError(TimeoutError):
    """Raised when a synchronous webview API wait exceeds its timeout."""


class AsyncRunner:
    """在独立线程中运行 async 协程，返回同步结果"""

    _executor: Optional[ThreadPoolExecutor] = None
    _default_timeout: float = float(os.getenv("CC_ASYNC_RUNNER_TIMEOUT_SECONDS", "60"))

    @classmethod
    def _get_executor(cls) -> ThreadPoolExecutor:
        if cls._executor is None:
            cls._executor = ThreadPoolExecutor(max_workers=2, thread_name_prefix="career_async")
        return cls._executor

    @classmethod
    def run(cls, coro: Any, timeout: Optional[float] = None) -> Any:
        """提交协程到后台线程执行，阻塞等待结果"""
        def _run() -> Any:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                return loop.run_until_complete(coro)
            finally:
                loop.close()

        future = cls._get_executor().submit(_run)
        effective_timeout = timeout if timeout is not None else cls._default_timeout
        try:
            return future.result(timeout=effective_timeout)
        except FutureTimeoutError as exc:
            future.cancel()
            raise AsyncRunnerTimeoutError(
                f"操作超时：后台任务在 {effective_timeout:.0f} 秒内未完成，请稍后重试或检查 LLM/网络配置。"
            ) from exc


class CareerAPI:
    """
    同步 API 封装，直接供 Bridge 调用
    服务实例延迟初始化，避免导入时阻塞。
    """

    def __init__(self) -> None:
        self._exp_mgr: Optional[ExperienceManager] = None
        self._persona_eng: Optional[PersonaEngine] = None
        self._job_matcher: Optional[JobMatcher] = None
        self._learner: Optional[LearningRecommender] = None
        self._job_parser: Optional[Any] = None
        self._jd_reframe: Optional[Any] = None
        self._skill_graph: Optional[SkillGraph] = None

    @property
    def exp_mgr(self) -> ExperienceManager:
        if self._exp_mgr is None:
            self._exp_mgr = ExperienceManager()
        return self._exp_mgr

    @property
    def persona_eng(self) -> PersonaEngine:
        if self._persona_eng is None:
            self._persona_eng = PersonaEngine()
        return self._persona_eng

    @property
    def job_matcher(self) -> JobMatcher:
        if self._job_matcher is None:
            self._job_matcher = JobMatcher()
        return self._job_matcher

    @property
    def learner(self) -> LearningRecommender:
        if self._learner is None:
            self._learner = LearningRecommender()
        return self._learner

    @property
    def job_parser(self) -> Any:
        if self._job_parser is None:
            from src.services.job_parser import JobParser
            self._job_parser = JobParser()
        return self._job_parser

    @property
    def jd_reframe(self) -> Any:
        if self._jd_reframe is None:
            from src.services.jd_reframe_engine import JDReframeEngine
            self._jd_reframe = JDReframeEngine()
        return self._jd_reframe

    @property
    def skill_graph(self) -> SkillGraph:
        if self._skill_graph is None:
            self._skill_graph = SkillGraph()
        return self._skill_graph

    # ─── 经历 ───

    def get_experiences(self) -> List[Dict[str, Any]]:
        """获取经历列表"""
        try:
            exps = AsyncRunner.run(self.exp_mgr.list_by_user())
            return [self._exp_to_dict(e) for e in exps]
        except Exception as e:
            logger.error(f"get_experiences error: {e}")
            return []

    def save_experience(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """保存经历（绕过 draft 流程，直接保存）"""
        try:
            exp_id = str(data.get("id") or "").strip()
            fields: Dict[str, Any] = {
                "title": data.get("title", ""),
                "organization": data.get("organization") or data.get("company", ""),
                "type": data.get("type", "work"),
                "start_date": self.exp_mgr._parse_date(data.get("start_date")),
                "end_date": self.exp_mgr._parse_date(data.get("end_date")),
                "raw_description": data.get("raw_description") or data.get("description", ""),
                "skills_demonstrated": data.get("skills_demonstrated") or data.get("skills", []),
                "structured_achievements": data.get("structured_achievements")
                or data.get("achievements"),
            }
            if exp_id:
                result = AsyncRunner.run(self.exp_mgr.update(exp_id, **fields))
                if result is None:
                    return {"success": False, "error": f"经历不存在或无法更新: {exp_id}"}
                return {"success": True, "id": str(result.id)}

            from src.services.experience_manager import ExperienceDraft
            draft = ExperienceDraft(
                raw_text=fields["raw_description"],
                extracted={
                    "title": fields["title"],
                    "start_date": data.get("start_date", ""),
                    "end_date": data.get("end_date", ""),
                    "skills_demonstrated": fields["skills_demonstrated"],
                    "structured_achievements": fields["structured_achievements"],
                    "organization": fields["organization"],
                    "type": fields["type"],
                },
            )
            result = AsyncRunner.run(self.exp_mgr.confirm_and_save(draft))
            return {"success": True, "id": str(result.id) if hasattr(result, "id") else ""}
        except TimeConflictError as e:
            logger.warning(f"save_experience time conflict: {e}")
            conflicts = [
                {
                    "id": str(c.id),
                    "title": c.title,
                    "organization": getattr(c, "organization", ""),
                    "start_date": str(c.start_date) if c.start_date else "",
                    "end_date": str(c.end_date) if c.end_date else "至今",
                }
                for c in e.conflicts
            ]
            return {
                "success": False,
                "error_type": "TIME_CONFLICT",
                "error": str(e),
                "conflicts": conflicts,
            }
        except Exception as e:
            logger.error(f"save_experience error: {e}")
            return {"success": False, "error": str(e)}

    @staticmethod
    def _exp_to_dict(exp: Any) -> Dict[str, Any]:
        return {
            "id": str(exp.id) if hasattr(exp, "id") else "",
            "title": getattr(exp, "title", ""),
            "description": getattr(exp, "raw_description", ""),
            "company": getattr(exp, "organization", ""),
            "role": getattr(exp, "title", ""),
            "type": getattr(exp, "type", "work"),
            "start_date": str(getattr(exp, "start_date", "")),
            "end_date": str(getattr(exp, "end_date", "")),
            "skills": list(getattr(exp, "skills_demonstrated", []) or []),
            "achievements": list(getattr(exp, "structured_achievements", []) or []),
            "metrics": list(getattr(exp, "metrics", []) or []),
            "status": getattr(exp, "status", "draft"),
            "version": getattr(exp, "version", 1),
        }

    # ─── 角色 ───

    def get_personas(self) -> List[Dict[str, Any]]:
        """获取角色列表"""
        try:
            personas = AsyncRunner.run(self.persona_eng.list_by_user())
            return [self._persona_to_dict(p) for p in personas]
        except Exception as e:
            logger.error(f"get_personas error: {e}")
            return []

    def get_experiences_with_fit_score(self, persona_id: str) -> Dict[str, Any]:
        """获取经历列表及其在指定角色下的 Fit Score"""
        try:
            weights = AsyncRunner.run(
                self.persona_eng.get_weighted_experiences(persona_id, min_score=0.0)
            )
            items = []
            for w in weights:
                exp = getattr(w, "experience", None)
                if exp:
                    items.append({
                        "experience_id": str(w.experience_id),
                        "title": getattr(exp, "title", ""),
                        "relevance_score": round(float(w.relevance_score or 0) * 100),
                        "user_overridden": bool(w.user_overridden),
                    })
            return {"success": True, "data": items}
        except Exception as e:
            logger.error(f"get_experiences_with_fit_score error: {e}")
            return {"success": False, "error": str(e)}

    def update_fit_score(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """手动更新 Fit Score"""
        try:
            persona_id = str(data.get("persona_id", ""))
            experience_id = str(data.get("experience_id", ""))
            score = float(data.get("score", 0)) / 100.0  # 前端传 0-100
            if not persona_id or not experience_id:
                return {"success": False, "error": "缺少 persona_id 或 experience_id"}
            rew = AsyncRunner.run(
                self.persona_eng.update_fit_score(persona_id, experience_id, score)
            )
            if rew:
                return {
                    "success": True,
                    "data": {
                        "experience_id": str(rew.experience_id),
                        "relevance_score": round(float(rew.relevance_score) * 100),
                        "user_overridden": bool(rew.user_overridden),
                    },
                }
            return {"success": False, "error": "更新失败"}
        except Exception as e:
            logger.error(f"update_fit_score error: {e}")
            return {"success": False, "error": str(e)}

    @staticmethod
    def _persona_to_dict(p: Any) -> Dict[str, Any]:
        return {
            "id": str(p.id) if hasattr(p, "id") else "",
            "name": getattr(p, "name", ""),
            "identity_statement": getattr(p, "identity_statement", ""),
            "career_narrative": getattr(p, "career_narrative", ""),
            "tone_style": getattr(p, "tone_style", ""),
            "capability_weights": dict(getattr(p, "capability_weights", {}) or {}),
            "target_job_profiles": list(getattr(p, "target_job_profiles", []) or []),
            "max_experiences": getattr(p, "max_experiences", 5),
            "preferred_model": getattr(p, "preferred_model", ""),
            "is_default": getattr(p, "is_default", False),
        }

    # ─── 简历 ───

    def generate_resume(self, persona_id: str, template_name: str = "modern") -> Dict[str, Any]:
        """生成简历，并返回技能覆盖分析"""
        try:
            builder = ResumeBuilder(persona_id=persona_id)
            AsyncRunner.run(builder.prepare())
            md = AsyncRunner.run(builder.render(template_name=template_name))

            # 计算技能覆盖情况
            covered_skills: List[str] = []
            missing_skills: List[str] = []
            try:
                persona = AsyncRunner.run(self.persona_eng.get_by_id(persona_id))
                if persona and getattr(persona, "capability_weights", None):
                    capability_skills = list(getattr(persona, "capability_weights", {}).keys())
                    # 从 builder 已选经历中汇总技能
                    exp_skills: set = set()
                    for rew in getattr(builder, "_experiences", []) or []:
                        exp = getattr(rew, "experience", None)
                        if exp:
                            exp_skills.update(
                                list(getattr(exp, "skills_demonstrated", []) or [])
                            )
                    covered_skills = [s for s in capability_skills if s in exp_skills]
                    missing_skills = [s for s in capability_skills if s not in exp_skills]
            except Exception as e:
                logger.warning(f"generate_resume 技能覆盖计算失败: {e}")

            return {
                "success": True,
                "markdown": md,
                "covered_skills": covered_skills,
                "missing_skills": missing_skills,
            }
        except Exception as e:
            logger.error(f"generate_resume error: {e}")
            return {"success": False, "error": str(e)}

    # ─── 岗位匹配 ───

    def parse_jd(self, jd_text: str) -> Dict[str, Any]:
        """解析 JD 并保存，返回岗位信息"""
        try:
            jd = AsyncRunner.run(self.job_parser.parse_and_save(jd_text, source="manual"))
            return {"success": True, "data": self._job_to_dict(jd)}
        except Exception as e:
            logger.error(f"parse_jd error: {e}")
            return {"success": False, "error": str(e)}

    def match_job(self, job_desc_id: str, persona_id: str) -> Dict[str, Any]:
        """为指定岗位和角色执行匹配"""
        try:
            match = AsyncRunner.run(
                self.job_matcher.match(persona_id=persona_id, job_desc_id=job_desc_id)
            )
            if match:
                return {"success": True, "data": self._match_to_dict(match)}
            return {"success": False, "error": "匹配未生成结果"}
        except Exception as e:
            logger.error(f"match_job error: {e}")
            return {"success": False, "error": str(e)}

    @staticmethod
    def _match_to_dict(m: Any, job_title: str = "", persona_name: str = "") -> Dict[str, Any]:
        breakdown = getattr(m, "score_breakdown", {}) or {}
        matched_skills = list(getattr(m, "matched_skills", []) or [])
        missing_skills = list(getattr(m, "missing_skills", []) or [])
        try:
            job_desc = getattr(m, "job_desc", None)
        except Exception:
            job_desc = None
        try:
            persona = getattr(m, "persona", None)
        except Exception:
            persona = None
        parsed_skills = []
        job_description = ""
        if job_desc is not None:
            try:
                parsed_skills = list(getattr(job_desc, "parsed_skills", []) or [])
            except Exception:
                parsed_skills = []
            try:
                job_description = (
                    getattr(job_desc, "raw_text", "")
                    or getattr(job_desc, "raw_description", "")
                    or getattr(job_desc, "description", "")
                    or ""
                )
            except Exception:
                job_description = ""
        required_skills = list(breakdown.get("required_skills", []) or [])
        if not required_skills:
            required_skills = parsed_skills or matched_skills + missing_skills
        try:
            resolved_job_title = job_title or getattr(job_desc, "title", "") or ""
        except Exception:
            resolved_job_title = job_title or ""
        try:
            resolved_persona_name = persona_name or getattr(persona, "name", "") or ""
        except Exception:
            resolved_persona_name = persona_name or ""
        return {
            "id": str(m.id) if hasattr(m, "id") else "",
            "persona_id": str(getattr(m, "persona_id", "")) or "",
            "job_desc_id": str(getattr(m, "job_desc_id", "")) or "",
            "score": getattr(m, "match_score", 0),
            "skill_score": breakdown.get("skill", 0),
            "exp_score": breakdown.get("experience", 0),
            "score_breakdown": dict(breakdown),
            "matched_skills": matched_skills,
            "missing_skills": missing_skills,
            "required_skills": required_skills,
            "skill_gaps": missing_skills,
            "tracking_status": getattr(m, "tracking_status", "new"),
            "status": getattr(m, "tracking_status", "new"),
            "notes": getattr(m, "notes", ""),
            "ai_analysis": getattr(m, "ai_analysis", ""),
            "match_reason": getattr(m, "ai_analysis", "") or getattr(m, "notes", ""),
            "skill_matches": matched_skills,
            "strengths": [],
            "job_title": resolved_job_title,
            "persona_name": resolved_persona_name,
            "job_description": job_description,
        }

    def list_jobs(self) -> List[Dict[str, Any]]:
        """列出所有岗位"""
        try:
            jobs = AsyncRunner.run(self.job_parser.list_all(limit=100))
            return [self._job_to_dict(j) for j in jobs]
        except Exception as e:
            logger.error(f"list_jobs error: {e}")
            return []

    def delete_job(self, job_desc_id: str) -> Dict[str, Any]:
        """删除岗位及关联的匹配、修饰记录"""
        try:
            # 先删除关联的修饰记录和匹配记录
            personas = self.get_personas()
            for p in personas:
                persona_id = p.get("id", "")
                matches = AsyncRunner.run(
                    self.job_matcher.list_matches(persona_id)
                )
                for m in matches:
                    if str(getattr(m, "job_desc_id", "")) == job_desc_id:
                        match_id = str(m.id)
                        AsyncRunner.run(self.jd_reframe.delete_reframes(match_id))
                        AsyncRunner.run(self.job_matcher.delete_match(match_id))
            deleted = AsyncRunner.run(self.job_parser.delete(job_desc_id))
            return {"success": deleted}
        except Exception as e:
            logger.error(f"delete_job error: {e}")
            return {"success": False, "error": str(e)}

    def get_job_matches(self, job_desc_id: str) -> Dict[str, Any]:
        """获取某个岗位的所有匹配记录（含关联信息）"""
        try:
            matches = AsyncRunner.run(
                self.job_matcher.list_matches_by_job(job_desc_id)
            )
            return {
                "success": True,
                "data": [self._match_to_dict(m) for m in matches],
            }
        except Exception as e:
            logger.error(f"get_job_matches error: {e}")
            return {"success": False, "error": str(e), "data": []}

    def update_match_status(self, match_id: str, status: str) -> Dict[str, Any]:
        """更新岗位匹配投递状态"""
        try:
            result = AsyncRunner.run(
                self.job_matcher.update_tracking_status(match_id, status)
            )
            return {"success": result is not None}
        except Exception as e:
            logger.error(f"update_match_status error: {e}")
            return {"success": False, "error": str(e)}

    def reframe_resume(self, match_id: str) -> Dict[str, Any]:
        """为某个岗位匹配生成 JD 修饰经历"""
        try:
            reframes = AsyncRunner.run(
                self.jd_reframe.reframe_experiences_for_job(match_id)
            )
            return {
                "success": True,
                "count": len(reframes),
                "reframes": [self._reframe_to_dict(r) for r in reframes],
            }
        except Exception as e:
            logger.error(f"reframe_resume error: {e}")
            return {"success": False, "error": str(e)}

    def get_reframe_results(self, match_id: str) -> Dict[str, Any]:
        """获取已缓存的修饰结果"""
        try:
            reframes = AsyncRunner.run(
                self.jd_reframe.get_reframed_experiences(match_id)
            )
            data = [self._reframe_to_dict(r) for r in reframes]
            return {
                "success": True,
                "count": len(reframes),
                "reframes": data,
                "data": data,
            }
        except Exception as e:
            logger.error(f"get_reframe_results error: {e}")
            return {"success": False, "error": str(e)}

    def update_reframe(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """手动更新单条重述内容"""
        try:
            reframe_id = str(data.get("reframe_id", ""))
            reframed_summary = str(data.get("reframed_summary", ""))
            if not reframe_id:
                return {"success": False, "error": "缺少 reframe_id"}
            reframe = AsyncRunner.run(
                self.jd_reframe.update_reframe(reframe_id, reframed_summary)
            )
            if reframe:
                return {
                    "success": True,
                    "data": self._reframe_to_dict(reframe),
                }
            return {"success": False, "error": "重述记录不存在"}
        except Exception as e:
            logger.error(f"update_reframe error: {e}")
            return {"success": False, "error": str(e)}

    def reset_reframe(self, reframe_id: str) -> Dict[str, Any]:
        """重置单条重述，删除记录后下次重新生成"""
        try:
            if not reframe_id:
                return {"success": False, "error": "缺少 reframe_id"}
            ok = AsyncRunner.run(self.jd_reframe.reset_reframe(reframe_id))
            return {"success": ok, "data": {"deleted": ok}}
        except Exception as e:
            logger.error(f"reset_reframe error: {e}")
            return {"success": False, "error": str(e)}

    @staticmethod
    def _job_to_dict(j: Any) -> Dict[str, Any]:
        return {
            "id": str(j.id) if hasattr(j, "id") else "",
            "title": getattr(j, "title", "") or "",
            "company": getattr(j, "company", "") or "",
            "location": getattr(j, "location", "") or "",
            "parsed_skills": list(getattr(j, "parsed_skills", []) or []),
            "skills": list(getattr(j, "parsed_skills", []) or []),
            "responsibilities": list(getattr(j, "responsibilities", []) or []),
            "raw_text": getattr(j, "raw_text", "") or "",
            "years_of_experience": getattr(j, "years_of_experience", "") or "",
            "salary_range": getattr(j, "salary_range", "") or "",
            "created_at": str(getattr(j, "created_at", "")),
        }

    @staticmethod
    def _reframe_to_dict(r: Any) -> Dict[str, Any]:
        try:
            experience = getattr(r, "experience", None)
        except Exception:
            experience = None
        try:
            experience_title = getattr(experience, "title", "") or ""
        except Exception:
            experience_title = ""
        return {
            "id": str(r.id) if hasattr(r, "id") else "",
            "job_match_id": str(getattr(r, "job_match_id", "")) or "",
            "experience_id": str(getattr(r, "experience_id", "")) or "",
            "original_summary": getattr(r, "original_summary", "") or "",
            "reframed_summary": getattr(r, "reframed_summary", "") or "",
            "reframed_content": getattr(r, "reframed_summary", "") or "",
            "reframing_strategy": getattr(r, "reframing_strategy", "") or "",
            "target_capability": getattr(r, "reframing_strategy", "") or "",
            "experience_title": experience_title,
            "created_at": str(getattr(r, "created_at", "")),
        }

    # ——— 学习路径 ———

    def get_learning_path(self, skill: str) -> List[Dict[str, Any]]:
        """获取学习路径，字段名统一为 duration，确保每个资源包含 url

        异常时直接抛出，由 bridge 层捕获并返回错误 JSON，确保前端能看到具体错误提示。
        """
        personas = self.get_personas()
        if not personas:
            raise ValueError("尚未创建角色，请先在“角色管理”中创建一个角色后再生成学习路径。")
        persona_id = personas[0].get("id", "")
        items = AsyncRunner.run(
            self.learner.recommend_for_gap(
                persona_id=persona_id,
                missing_skills=[skill],
            )
        )
        if items:
            AsyncRunner.run(
                self.learner.create_learning_path(
                    persona_id=persona_id,
                    target_gap=skill,
                    items=items,
                    source_type="skill_graph",
                )
            )
        result = []
        for item in (items or []):
            if not isinstance(item, dict):
                continue
            normalized = dict(item)
            # 兼容 estimated_hours / duration
            if "estimated_hours" in normalized and "duration" not in normalized:
                normalized["duration"] = str(normalized.pop("estimated_hours")) + " 小时"
            # 确保 url 字段存在（从 link 降级复制）
            if "url" not in normalized and "link" in normalized:
                normalized["url"] = normalized.pop("link")
            if "url" not in normalized:
                normalized["url"] = ""
            result.append(normalized)
        return result

    def get_learning_paths_by_source(
        self, persona_id: str = ""
    ) -> Dict[str, List[Dict[str, Any]]]:
        """按 source_type 分类返回学习路径"""
        try:
            target_persona_id = persona_id
            if not target_persona_id:
                personas = self.get_personas()
                target_persona_id = personas[0].get("id", "") if personas else ""
            if not target_persona_id:
                return {"jd_gap": [], "skill_graph": [], "manual": [], "other": []}
            return AsyncRunner.run(self._get_learning_paths_by_source(target_persona_id))
        except Exception as e:
            logger.error(f"get_learning_paths_by_source error: {e}")
            return {"jd_gap": [], "skill_graph": [], "manual": [], "other": []}

    @staticmethod
    async def _get_learning_paths_by_source(
        persona_id: str,
    ) -> Dict[str, List[Dict[str, Any]]]:
        grouped: Dict[str, List[Dict[str, Any]]] = {
            "jd_gap": [],
            "skill_graph": [],
            "manual": [],
            "other": [],
        }
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(LearningPath)
                .where(LearningPath.persona_id == persona_id)
                .order_by(LearningPath.created_at.desc())
            )
            for path in result.scalars().all():
                source_type = getattr(path, "source_type", None) or "manual"
                key = source_type if source_type in grouped else "other"
                grouped[key].append(CareerAPI._learning_path_to_dict(path))
        return grouped

    @staticmethod
    def _learning_path_to_dict(path: Any) -> Dict[str, Any]:
        return {
            "id": str(path.id) if hasattr(path, "id") else "",
            "persona_id": str(getattr(path, "persona_id", "")) or "",
            "target_gap": getattr(path, "target_gap", "") or "",
            "items": list(getattr(path, "items", []) or []),
            "source_type": getattr(path, "source_type", None) or "manual",
            "status": getattr(path, "status", "active") or "active",
            "created_at": str(getattr(path, "created_at", "")),
            "updated_at": str(getattr(path, "updated_at", "")),
        }

    # ——— 经历增删 ———

    def delete_experience(self, exp_id: str) -> Dict[str, Any]:
        """删除经历（软删除）"""
        try:
            AsyncRunner.run(self.exp_mgr.delete(exp_id))
            return {"success": True}
        except Exception as e:
            logger.error(f"delete_experience error: {e}")
            return {"success": False, "error": str(e)}

    # ——— 角色 CRUD ———

    def get_persona_by_id(self, persona_id: str) -> Optional[Dict[str, Any]]:
        """获取单个角色"""
        try:
            p = AsyncRunner.run(self.persona_eng.get_by_id(persona_id))
            if p is None:
                return None
            return self._persona_to_dict(p)
        except Exception as e:
            logger.error(f"get_persona_by_id error: {e}")
            return None

    def create_persona(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """创建角色"""
        try:
            p = AsyncRunner.run(
                self.persona_eng.create(
                    name=data.get("name", ""),
                    identity_statement=data.get("identity_statement", ""),
                    capability_weights=data.get("capability_weights", {}),
                    tone_style=data.get("tone_style", "business_insight"),
                    target_job_profiles=data.get("target_job_profiles", []),
                    max_experiences=data.get("max_experiences", 5),
                    user_id=data.get("user_id", "default"),
                )
            )
            return {"success": True, "id": str(p.id), "data": self._persona_to_dict(p)}
        except Exception as e:
            logger.error(f"create_persona error: {e}")
            return {"success": False, "error": str(e)}

    def update_persona(self, persona_id: str, data: Dict[str, Any]) -> Dict[str, Any]:
        """更新角色"""
        try:
            fields: Dict[str, Any] = {}
            for key in [
                "name",
                "identity_statement",
                "career_narrative",
                "tone_style",
                "capability_weights",
                "target_job_profiles",
                "max_experiences",
                "preferred_model",
            ]:
                if key in data:
                    fields[key] = data[key]
            p = AsyncRunner.run(self.persona_eng.update(persona_id, **fields))
            if p is None:
                return {"success": False, "error": f"角色不存在: {persona_id}"}
            return {"success": True, "id": str(p.id), "data": self._persona_to_dict(p)}
        except Exception as e:
            logger.error(f"update_persona error: {e}")
            return {"success": False, "error": str(e)}

    def delete_persona(self, persona_id: str) -> Dict[str, Any]:
        """删除角色"""
        try:
            AsyncRunner.run(self.persona_eng.delete(persona_id))
            return {"success": True}
        except Exception as e:
            logger.error(f"delete_persona error: {e}")
            return {"success": False, "error": str(e)}

    def chat_refine_resume(self, persona_id: str, instruction: str) -> Dict[str, Any]:
        """对话式简历调优：基于当前简历 + 用户指令生成修改版本"""
        try:
            if not persona_id or not instruction:
                return {"success": False, "error": "角色ID和调优指令不能为空"}

            # 1. 获取当前简历
            builder = ResumeBuilder(persona_id=persona_id)
            AsyncRunner.run(builder.prepare())
            current_resume = AsyncRunner.run(builder.render(template_name="modern"))

            # 2. 调用 LLM 生成修改版本
            from src.llm.router import LLMRouter
            from src.config.settings import get_settings

            settings = get_settings()
            router = LLMRouter(settings=settings)

            prompt = (
                f"你是一位资深简历顾问。以下是用户的当前简历：\n\n"
                f"{current_resume}\n\n"
                f"用户的调优指令：{instruction}\n\n"
                f"请根据指令修改简历，保持事实准确，只调整表达方式和侧重点。"
                f"返回修改后的完整简历 Markdown。"
            )

            response = AsyncRunner.run(
                router.chat(messages=[{"role": "user", "content": prompt}])
            )
            refined = response if isinstance(response, str) else str(response)

            return {
                "success": True,
                "data": {
                    "original": current_resume,
                    "refined": refined,
                    "instruction": instruction,
                },
            }
        except Exception as e:
            logger.error(f"chat_refine_resume error: {e}")
            return {"success": False, "error": str(e)}

    # ─── 技能图谱 ───

    def get_skill_graph(self) -> Dict[str, Any]:
        """返回完整技能图谱"""
        try:
            nodes = self.skill_graph.all_nodes()
            return {"success": True, "data": nodes}
        except Exception as e:
            logger.error(f"get_skill_graph error: {e}")
            return {"success": False, "error": str(e)}

    def search_skills(self, query: str) -> Dict[str, Any]:
        """搜索技能"""
        try:
            results = self.skill_graph.search(query)
            return {"success": True, "data": results}
        except Exception as e:
            logger.error(f"search_skills error: {e}")
            return {"success": False, "error": str(e)}

    def get_skill_resources(self, skill_id: str) -> List[Dict[str, Any]]:
        """获取技能节点的学习资源"""
        try:
            resources = self.skill_graph.get_resources(skill_id)
            return [dict(resource) for resource in resources]
        except Exception as e:
            logger.error(f"get_skill_resources error: {e}")
            return []

    # ─── 统计 ───

    def export_resume_pdf(self, persona_id: str) -> Dict[str, Any]:
        """生成简历 PDF。先尝试调用 PDFExporter 生成真实 PDF，如果 fpdf2 未安装则降级为 Markdown 下载。"""
        try:
            import base64

            from src.services.pdf_exporter import PDFExporter, PDFExporterError
            from src.services.resume_builder import ResumeBuilder

            # 获取 Persona
            persona = AsyncRunner.run(self.persona_eng.get_by_id(persona_id))
            if not persona:
                return {"success": False, "error": "角色不存在"}

            # 通过 ResumeBuilder 获取经历列表
            builder = ResumeBuilder(persona_id=persona_id)
            AsyncRunner.run(builder.prepare())
            experiences: List[Any] = []
            for rew in getattr(builder, "_experiences", []) or []:
                exp = getattr(rew, "experience", None)
                if exp:
                    experiences.append(exp)

            # 生成 PDF
            exporter = PDFExporter()
            pdf_bytes = AsyncRunner.run(exporter.export_resume(persona, experiences))
            encoded = base64.b64encode(pdf_bytes).decode("utf-8")

            return {
                "success": True,
                "data": {
                    "pdf_base64": encoded,
                    "filename": f"简历_{persona.name}.pdf",
                },
            }
        except PDFExporterError:
            # fpdf2 未安装，降级为 Markdown
            md_result = self.generate_resume(persona_id)
            return {
                "success": False,
                "error": "fpdf2 未安装，PDF 导出不可用。请运行: pip install fpdf2",
                "fallback_markdown": md_result.get("markdown", "") if isinstance(md_result, dict) else "",
            }
        except Exception as e:
            logger.error(f"export_resume_pdf error: {e}")
            return {"success": False, "error": str(e)}

    def import_experiences(self, format: str, data: Dict[str, Any]) -> Dict[str, Any]:
        """批量导入经历（文本/Markdown/JSON）"""
        try:
            from src.services.import_parser import ImportParser

            parser = ImportParser()
            content = data.get("content", "")

            if format == "markdown":
                drafts = AsyncRunner.run(parser.parse_markdown(content))
            elif format == "json":
                drafts = AsyncRunner.run(parser.parse_json(content))
            else:
                drafts = AsyncRunner.run(parser.parse_text(content))

            count = 0
            for draft in drafts:
                try:
                    result = AsyncRunner.run(self.exp_mgr.confirm_and_save(draft))
                    if result:
                        count += 1
                except Exception as e:
                    logger.warning(f"导入经历失败: {e}")
                    continue

            return {"success": True, "count": count}
        except Exception as e:
            logger.error(f"import_experiences error: {e}")
            return {"success": False, "error": str(e)}

    def import_file(self, file_name: str, base64_content: str) -> Dict[str, Any]:
        """导入 PDF/Word 文件，提取文本后由 LLM 解析为结构化经历并自动保存"""
        try:
            import base64

            from src.services.import_parser import ImportParser

            file_bytes = base64.b64decode(base64_content)
            parser = ImportParser()
            drafts = AsyncRunner.run(parser.import_file(file_name, file_bytes))

            count = 0
            for draft in drafts:
                try:
                    result = AsyncRunner.run(self.exp_mgr.confirm_and_save(draft))
                    if result:
                        count += 1
                except Exception as e:
                    logger.warning(f"导入经历失败: {e}")
                    continue

            return {
                "success": True,
                "count": count,
                "message": f"成功从 {file_name} 导入 {count} 条经历",
            }
        except Exception as e:
            logger.error(f"import_file error: {e}")
            return {"success": False, "error": str(e)}

    def get_settings(self) -> Dict[str, Any]:
        """读取设置：返回 LLM Provider 配置，不返回真实 API Key"""
        try:
            from src.config.settings import CONFIG_FILE, _load_yaml_config, create_default_config

            if not CONFIG_FILE.exists():
                create_default_config()
            config = _load_yaml_config()
            providers = []
            for provider in config.get("llm_providers", []):
                name = provider.get("name", "")
                providers.append({
                    "name": name,
                    "base_url": provider.get("base_url"),
                    "default_model": provider.get("default_model", ""),
                    "enabled": provider.get("enabled", True),
                    "has_key": SecureStorage.has_api_key(name),
                })

            return {
                "success": True,
                "data": {
                    "llm_providers": providers,
                    "default_llm_provider": config.get("default_llm_provider", ""),
                },
            }
        except Exception as e:
            logger.error(f"get_settings error: {e}")
            return {"success": False, "error": str(e)}

    def save_settings(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """保存设置：安全存储 API Key，保存 LLM Provider 配置"""
        try:
            from src.config import settings as settings_module

            if "providers" not in data:
                logger.info("忽略旧版设置保存请求")
                return {"success": True, "message": "配置已保存"}

            config = settings_module._load_yaml_config()
            providers = []

            for item in data.get("providers", []):
                name = item.get("name", "").strip()
                if not name:
                    continue

                api_key = item.get("api_key", "").strip()
                if api_key:
                    success = SecureStorage.store_api_key(name, api_key)
                    if not success:
                        return {"success": False, "error": f"Provider '{name}' 的 API Key 存储失败"}

                base_url = item.get("base_url")
                if isinstance(base_url, str):
                    base_url = base_url.strip() or None

                providers.append({
                    "name": name,
                    "base_url": base_url,
                    "default_model": item.get("default_model", "").strip(),
                    "enabled": bool(item.get("enabled", True)),
                })

            config["llm_providers"] = providers
            config["default_llm_provider"] = data.get("default_provider", "")
            settings_module._save_yaml_config(config)
            settings_module._settings = None

            logger.info("保存 LLM Provider 设置")
            return {"success": True, "message": "配置已保存"}
        except Exception as e:
            logger.error(f"save_settings error: {e}")
            return {"success": False, "error": str(e)}

    def test_llm_connection(self) -> Dict[str, Any]:
        """测试 LLM 连接"""
        try:
            from src.llm.router import LLMRouter
            from src.config.settings import get_settings

            settings = get_settings()
            router = LLMRouter(settings=settings)

            response = AsyncRunner.run(
                router.chat(messages=[{"role": "user", "content": "你好"}])
            )

            return {"success": True, "connected": True, "message": "连接成功"}
        except Exception as e:
            logger.error(f"test_llm_connection error: {e}")
            return {"success": True, "connected": False, "message": f"连接失败: {str(e)}"}

    # ─── 统计 ───

    def get_stats(self) -> Dict[str, Any]:
        """获取欢迎页统计"""
        try:
            exps = self.get_experiences()
            personas = self.get_personas()
            counts = AsyncRunner.run(self._get_db_counts())
            return {
                "experiencesCount": len(exps),
                "personasCount": len(personas),
                "jobMatches": counts["jobMatches"],
                "learningPaths": counts["learningPaths"],
            }
        except Exception as e:
            logger.error(f"get_stats error: {e}")
            return {
                "experiencesCount": 0,
                "personasCount": 0,
                "jobMatches": 0,
                "learningPaths": 0,
            }

    @staticmethod
    async def _get_db_counts() -> Dict[str, int]:
        async with AsyncSessionLocal() as session:
            job_matches = await session.scalar(select(func.count()).select_from(JobMatch))
            learning_paths = await session.scalar(select(func.count()).select_from(LearningPath))
            return {
                "jobMatches": int(job_matches or 0),
                "learningPaths": int(learning_paths or 0),
            }
