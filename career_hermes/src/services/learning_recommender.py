"""
CareerCraft Agent — 学习路径推荐服务

核心职责：根据技能 Gap 生成学习路径，管理学习进度。
Sprint 5 核心服务。
"""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional

from sqlalchemy import select

from src.llm.router import LLMError, LLMRouter
from src.models.database import AsyncSessionLocal
from src.models.entities import LearningPath, Persona
from src.services.skill_graph import SkillGraph

logger = logging.getLogger(__name__)


class LearningRecommender:
    """
    学习路径推荐器

    使用流程：
        recommender = LearningRecommender()
        # 基于缺失技能生成推荐
        items = await recommender.recommend_for_gap(
            persona_id="xxx",
            missing_skills=["Kubernetes", "gRPC"],
        )
        # 保存为学习路径
        path = await recommender.create_learning_path(
            persona_id="xxx",
            target_gap="Kubernetes + gRPC",
            items=items,
        )
    """

    # 学习资源模板库（简化版，可扩展）
    _RESOURCE_TEMPLATES: Dict[str, List[Dict[str, Any]]] = {
        "kubernetes": [
            {"type": "course", "title": "Kubernetes 基础入门", "source": "KubeAcademy / 某课堂", "estimated_hours": 12, "url": "https://kubernetes.io/docs/tutorials/kubernetes-basics/"},
            {"type": "project", "title": "部署一个多服务应用到 K8s", "source": "个人项目", "estimated_hours": 20, "url": ""},
        ],
        "docker": [
            {"type": "course", "title": "Docker 实战", "source": "Docker 官方文档", "estimated_hours": 8, "url": "https://docs.docker.com/get-started/"},
        ],
        "grpc": [
            {"type": "article", "title": "gRPC 设计理念与实践", "source": "谷歌官方文档", "estimated_hours": 6, "url": "https://grpc.io/docs/what-is-grpc/introduction/"},
            {"type": "project", "title": "实现一个 gRPC 服务端与客户端", "source": "个人项目", "estimated_hours": 15, "url": ""},
        ],
        "python": [
            {"type": "course", "title": "Python 高级编程", "source": "官方文档 / 网易课堂", "estimated_hours": 20, "url": "https://docs.python.org/zh-cn/3/tutorial/"},
        ],
        "sql": [
            {"type": "course", "title": "SQL 性能优化", "source": "某课堂", "estimated_hours": 10, "url": "https://sqlbolt.com/"},
        ],
        "machine learning": [
            {"type": "book", "title": "机器学习实战", "source": "图书馆 / 亚马逊", "estimated_hours": 40, "url": "https://www.amazon.com/dp/1617290181"},
        ],
        "product management": [
            {"type": "course", "title": "产品经理成长计划", "source": "三节课", "estimated_hours": 16, "url": "https://www.sanjieke.cn/"},
        ],
    }

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()
        self.skill_graph = SkillGraph()

    async def recommend_for_gap(
        self,
        persona_id: str,
        missing_skills: List[str],
    ) -> List[Dict[str, Any]]:
        """
        根据缺失技能生成学习推荐列表。

        策略：先尝试技能图谱真实资源，再尝试 LLM，最后回退到本地模板库。
        """
        if not missing_skills:
            return []

        graph_items = self._recommend_by_skill_graph(missing_skills)
        if graph_items:
            return graph_items

        try:
            return await self._recommend_by_llm(persona_id, missing_skills)
        except LLMError as e:
            logger.warning("LLM 推荐失败，回退到本地模板: %s", e)
            return self._recommend_by_template(missing_skills)

    def _recommend_by_skill_graph(
        self, missing_skills: List[str]
    ) -> List[Dict[str, Any]]:
        """基于 skill_graph.json 中的真实资源生成推荐"""
        items: List[Dict[str, Any]] = []
        seen_titles: set[str] = set()
        priority = 1

        for skill in missing_skills:
            skill_text = skill.strip()
            if not skill_text:
                continue

            node = self.skill_graph.get_node(skill_text)
            if node is None:
                node = self.skill_graph.get_node(skill_text.lower())
            if node is None:
                matches = self.skill_graph.search(skill_text)
                node = matches[0] if matches else None
            if node is None:
                continue

            resources = node.get("resources", [])
            if not isinstance(resources, list):
                continue

            added_for_skill = 0
            for resource in resources:
                if not isinstance(resource, dict):
                    continue
                title = str(resource.get("title", "")).strip()
                if not title or title in seen_titles:
                    continue
                seen_titles.add(title)
                item = {
                    "type": resource.get("type", "course"),
                    "title": title,
                    "source": resource.get("source", "线上平台"),
                    "estimated_hours": resource.get("estimated_hours", 10),
                    "url": resource.get("url", ""),
                    "priority": priority,
                    "status": "pending",
                    "skill_id": node.get("id", ""),
                    "skill_name": node.get("name", skill_text),
                }
                items.append(item)
                added_for_skill += 1
                priority += 1
                if added_for_skill >= 2:
                    break

        return items

    async def _recommend_by_llm(
        self,
        persona_id: str,
        missing_skills: List[str],
    ) -> List[Dict[str, Any]]:
        """使用 LLM 生成学习路径"""
        async with AsyncSessionLocal() as session:
            persona_result = await session.execute(
                select(Persona).where(Persona.id == persona_id)
            )
            persona = persona_result.scalar_one_or_none()

        persona_name = persona.name if persona else "求职者"
        skills_str = ", ".join(missing_skills)

        prompt = f"""你是一位职业发展顾问。请为一位"{persona_name}"角色的用户
针对以下缺失技能，生成一份精简的学习路径推荐。

缺失技能: {skills_str}

请返回严格的 JSON 数组，每个元素包含：
- type: "course" | "article" | "book" | "project"
- title: 资源标题
- source: 推荐来源（平台名或书名）
- estimated_hours: 预估学习小时数（整数）
- priority: 优先级（1-5，1最高）
- url: 资源的访问链接（如果有真实链接则填写，否则留空字符串）

要求：
1. 每个技能至少推荐 1 个资源
2. 总课时控制在 100 小时以内
3. 以中文返回
4. 只返回 JSON 数组，不要其他解释
"""
        response = await self.llm.chat(
            messages=[{"role": "user", "content": prompt}],
            json_mode=True,
            temperature=0.5,
        )

        if not isinstance(response, str):
            raise LLMError("LLM 返回不是文本类型")

        try:
            items = json.loads(response)
            if not isinstance(items, list):
                raise ValueError("返回不是数组")
            # 添加状态字段并确保 url 存在
            for item in items:
                item["status"] = "pending"
                if "url" not in item:
                    item["url"] = ""
            return items
        except (json.JSONDecodeError, ValueError) as e:
            logger.warning("LLM 返回解析失败: %s", e)
            return self._recommend_by_template(missing_skills)

    def _recommend_by_template(
        self, missing_skills: List[str]
    ) -> List[Dict[str, Any]]:
        """基于本地模板库生成推荐"""
        items = []
        seen_titles = set()
        priority = 1

        for skill in missing_skills:
            skill_lower = skill.lower().strip()
            templates = self._RESOURCE_TEMPLATES.get(skill_lower, [])
            if not templates:
                # 通用推荐
                templates = [
                    {
                        "type": "course",
                        "title": f"{skill} 基础入门",
                        "source": "线上学习平台",
                        "estimated_hours": 10,
                        "url": "",
                    }
                ]

            for tmpl in templates:
                title = tmpl["title"]
                if title in seen_titles:
                    continue
                seen_titles.add(title)
                items.append(
                    {
                        "type": tmpl.get("type", "course"),
                        "title": title,
                        "source": tmpl.get("source", "线上平台"),
                        "estimated_hours": tmpl.get("estimated_hours", 10),
                        "priority": priority,
                        "status": "pending",
                        "url": tmpl.get("url", ""),
                    }
                )
            priority += 1

        return items

    async def create_learning_path(
        self,
        persona_id: str,
        target_gap: str,
        items: List[Dict[str, Any]],
        source_type: str = "manual",
    ) -> LearningPath:
        """
        创建学习路径并保存到数据库。
        """
        path = LearningPath(
            persona_id=persona_id,
            target_gap=target_gap,
            items=items,
            source_type=source_type,
            status="active",
        )
        async with AsyncSessionLocal() as session:
            session.add(path)
            await session.commit()
            await session.refresh(path)
            logger.info(
                "LearningPath 已创建: id=%s target=%s items=%d",
                path.id,
                target_gap,
                len(items),
            )
            return path

    async def get_active_paths(
        self, persona_id: str
    ) -> List[LearningPath]:
        """
        获取进行中的学习路径。
        """
        async with AsyncSessionLocal() as session:
            stmt = (
                select(LearningPath)
                .where(
                    LearningPath.persona_id == persona_id,
                    LearningPath.status == "active",
                )
                .order_by(LearningPath.created_at.desc())
            )
            result = await session.execute(stmt)
            return list(result.scalars().all())

    async def get_path_by_id(self, path_id: str) -> Optional[LearningPath]:
        """根据 ID 获取学习路径"""
        async with AsyncSessionLocal() as session:
            result = await session.execute(
                select(LearningPath).where(LearningPath.id == path_id)
            )
            return result.scalar_one_or_none()

    async def update_path_status(
        self, path_id: str, status: str
    ) -> Optional[LearningPath]:
        """
        更新学习路径状态（active / completed / archived）。
        """
        if status not in ("active", "completed", "archived"):
            logger.warning("无效的学习路径状态: %s", status)
            return None

        async with AsyncSessionLocal() as session:
            path = await session.get(LearningPath, path_id)
            if not path:
                return None
            path.status = status
            await session.commit()
            await session.refresh(path)
            logger.info("LearningPath 状态已更新: id=%s status=%s", path_id, status)
            return path

    async def update_item_status(
        self,
        path_id: str,
        item_index: int,
        new_status: str,
    ) -> Optional[LearningPath]:
        """
        更新学习路径中某个项目的状态。
        """
        if new_status not in ("pending", "in_progress", "completed", "skipped"):
            logger.warning("无效的项目状态: %s", new_status)
            return None

        async with AsyncSessionLocal() as session:
            path = await session.get(LearningPath, path_id)
            if not path or not path.items:
                return None
            if item_index < 0 or item_index >= len(path.items):
                return None

            path.items[item_index]["status"] = new_status
            await session.commit()
            await session.refresh(path)
            return path

    async def delete_path(self, path_id: str) -> bool:
        """删除学习路径（硬删除）〄"""
        async with AsyncSessionLocal() as session:
            path = await session.get(LearningPath, path_id)
            if not path:
                return False
            await session.delete(path)
            await session.commit()
            logger.info("LearningPath 已删除: id=%s", path_id)
            return True
