"""
CareerCraft Agent — Skill Graph Service

管理预置技能图谱的加载、查询与关联分析
"""

from __future__ import annotations

import json
import os
from typing import Any, Dict, List, Optional, Set


def _default_graph_path() -> str:
    """获取 skill_graph.json 默认路径"""
    base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base, "data", "skill_graph.json")


class SkillGraph:
    """
    技能图谱管理器

    提供节点查询、分类筛选、前置技能链与关联技能推荐等能力
    """

    def __init__(self, graph_path: Optional[str] = None) -> None:
        self._graph_path = graph_path or _default_graph_path()
        self._nodes: List[Dict[str, Any]] = []
        self._index: Dict[str, Dict[str, Any]] = {}
        self._children: Dict[str, List[str]] = {}
        self.load()

    def load(self) -> None:
        """加载 JSON 图谱并构建索引"""
        with open(self._graph_path, "r", encoding="utf-8") as f:
            self._nodes = json.load(f)

        self._index = {}
        self._children = {}
        for node in self._nodes:
            skill_id = node.get("id", "")
            if skill_id:
                self._index[skill_id] = node
                for pre in node.get("prerequisites", []):
                    self._children.setdefault(pre, []).append(skill_id)

    def get_node(self, skill_id: str) -> Optional[Dict[str, Any]]:
        """获取单个节点"""
        return self._index.get(skill_id)

    def get_resources(self, skill_id: str) -> List[Dict[str, Any]]:
        """获取指定技能节点的学习资源列表"""
        node = self.get_node(skill_id)
        if not node:
            return []
        resources = node.get("resources", [])
        if not isinstance(resources, list):
            return []
        return [dict(resource) for resource in resources if isinstance(resource, dict)]

    def search(self, query: str) -> List[Dict[str, Any]]:
        """按名称/别名搜索（不区分大小写）"""
        q = query.strip().lower()
        if not q:
            return []
        results: List[Dict[str, Any]] = []
        for node in self._nodes:
            if q in node.get("name", "").lower():
                results.append(node)
                continue
            aliases = node.get("aliases", [])
            if any(q in alias.lower() for alias in aliases):
                results.append(node)
        return results

    def get_by_category(self, category: str) -> List[Dict[str, Any]]:
        """按分类获取节点"""
        return [node for node in self._nodes if node.get("category") == category]

    def get_prerequisites(self, skill_id: str) -> List[Dict[str, Any]]:
        """获取指定技能的前置技能链（包含直接与间接前置）"""
        visited: Set[str] = set()
        chain: List[Dict[str, Any]] = []

        def _dfs(sid: str) -> None:
            if sid in visited:
                return
            visited.add(sid)
            node = self._index.get(sid)
            if node is None:
                return
            for pre in node.get("prerequisites", []):
                _dfs(pre)
            chain.append(node)

        start = self._index.get(skill_id)
        if start is None:
            return []
        for pre in start.get("prerequisites", []):
            _dfs(pre)
        return chain

    def get_related(self, skill_id: str, depth: int = 1) -> List[Dict[str, Any]]:
        """获取指定技能的关联技能（前置 + 后置，在 depth 层内）"""
        start = self._index.get(skill_id)
        if start is None:
            return []

        related_ids: Set[str] = set()
        current_layer: Set[str] = {skill_id}

        for _ in range(depth):
            next_layer: Set[str] = set()
            for sid in current_layer:
                node = self._index.get(sid)
                if node is None:
                    continue
                for pre in node.get("prerequisites", []):
                    if pre != skill_id and pre not in related_ids:
                        related_ids.add(pre)
                        next_layer.add(pre)
                for child in self._children.get(sid, []):
                    if child != skill_id and child not in related_ids:
                        related_ids.add(child)
                        next_layer.add(child)
            current_layer = next_layer

        return [self._index[sid] for sid in related_ids if sid in self._index]

    def all_nodes(self) -> List[Dict[str, Any]]:
        """返回所有节点"""
        return list(self._nodes)
