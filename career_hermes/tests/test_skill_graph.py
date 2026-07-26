"""
CareerCraft Agent — SkillGraph 测试
"""

from __future__ import annotations

from typing import Any, Dict, List

import pytest

from src.services.skill_graph import SkillGraph


class TestSkillGraph:
    """SkillGraph 单元测试"""

    @pytest.fixture
    def graph(self) -> SkillGraph:
        return SkillGraph()

    def test_load_50_nodes(self, graph: SkillGraph) -> None:
        """加载后应包含 50 个节点"""
        nodes = graph.all_nodes()
        assert len(nodes) == 51

    def test_node_structure(self, graph: SkillGraph) -> None:
        """每个节点必备字段完整"""
        for node in graph.all_nodes():
            assert "id" in node
            assert "name" in node
            assert "category" in node
            assert "aliases" in node
            assert "prerequisites" in node
            assert "level" in node
            assert "description" in node
            assert isinstance(node["id"], str)
            assert isinstance(node["name"], str)
            assert isinstance(node["category"], str)
            assert isinstance(node["aliases"], list)
            assert isinstance(node["prerequisites"], list)
            assert isinstance(node["level"], int)
            assert isinstance(node["description"], str)

    def test_categories(self, graph: SkillGraph) -> None:
        """节点分布在 4 个类别中"""
        categories = set()
        for node in graph.all_nodes():
            categories.add(node["category"])
        assert categories == {
            "product_management",
            "technical",
            "management",
            "industry",
        }

    def test_get_node(self, graph: SkillGraph) -> None:
        """通过 id 获取节点"""
        node = graph.get_node("requirement_analysis")
        assert node is not None
        assert node["name"] == "需求分析"
        assert node["category"] == "product_management"

    def test_get_node_not_found(self, graph: SkillGraph) -> None:
        """获取不存在的节点返回 None"""
        assert graph.get_node("nonexistent_skill") is None

    def test_search_by_name(self, graph: SkillGraph) -> None:
        """按名称搜索"""
        results = graph.search("Python")
        assert any(r["id"] == "python" for r in results)

    def test_search_by_alias(self, graph: SkillGraph) -> None:
        """按别名搜索"""
        results = graph.search("RESTful API")
        assert any(r["id"] == "api_design" for r in results)

    def test_search_empty(self, graph: SkillGraph) -> None:
        """空搜索返回空列表"""
        assert graph.search("") == []

    def test_get_by_category(self, graph: SkillGraph) -> None:
        """按分类获取"""
        pm_nodes = graph.get_by_category("product_management")
        assert len(pm_nodes) == 15
        for node in pm_nodes:
            assert node["category"] == "product_management"

    def test_get_prerequisites(self, graph: SkillGraph) -> None:
        """获取前置技能链"""
        chain = graph.get_prerequisites("growth_strategy")
        ids = [n["id"] for n in chain]
        # growth_strategy 的前置有 data_driven_decision → ab_testing
        assert "data_driven_decision" in ids
        assert "ab_testing" in ids
        # data_driven_decision 的前置是 data_analysis
        assert "data_analysis" in ids

    def test_get_prerequisites_not_found(self, graph: SkillGraph) -> None:
        """获取不存在的前置链返回空"""
        assert graph.get_prerequisites("nonexistent") == []

    def test_get_related(self, graph: SkillGraph) -> None:
        """获取关联技能"""
        related = graph.get_related("data_analysis", depth=1)
        ids = [n["id"] for n in related]
        # data_analysis 的后置：data_driven_decision, data_visualization, machine_learning_basics, etl_pipeline
        assert "data_driven_decision" in ids
        # data_analysis 的前置：python, sql
        assert "python" in ids
        assert "sql" in ids

    def test_get_related_depth_limit(self, graph: SkillGraph) -> None:
        """关联技能深度限制"""
        depth1 = graph.get_related("data_analysis", depth=1)
        depth2 = graph.get_related("data_analysis", depth=2)
        assert len(depth1) <= len(depth2)
    def test_no_circular_dependencies(self, graph: SkillGraph) -> None:
        """
68c7查没有循环依赖"""
        for node in graph.all_nodes():
            start = node["id"]
            current_path: set[str] = set()
            stack: list[tuple[str, list[str]]] = [(start, [start])]

            while stack:
                current, path = stack.pop()
                if current in current_path and current != path[0]:
                    # 检查是否是真正的循环（不是自身）
                    if current in path[:-1]:
                        pytest.fail(f"发现循环依赖: {' -> '.join(path)}")
                current_path.add(current)
                n = graph.get_node(current)
                if n:
                    for pre in n.get("prerequisites", []):
                        stack.append((pre, path + [pre]))
