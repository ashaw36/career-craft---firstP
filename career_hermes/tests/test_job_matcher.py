"""
CareerCraft Agent — JobMatcher 单元测试

测试覆盖：
1. 技能匹配算法
2. 经验年限解析
3. 其他匹配度
4. 投递状态更新
"""

from __future__ import annotations

from datetime import date
from typing import Any, Dict, List

import pytest
from pytest_asyncio import fixture as async_fixture

from src.services.job_matcher import JobMatcher
from src.models.entities import Experience, JobDesc, Persona


@pytest.fixture
def matcher() -> JobMatcher:
    return JobMatcher()


class TestSkillMatch:
    def test_perfect_match(self, matcher: JobMatcher) -> None:
        persona = ["Python", "Docker", "Kubernetes"]
        job = ["Python", "Docker"]
        matched, missing, score = matcher._calculate_skill_match(persona, job)
        assert matched == ["Python", "Docker"]
        assert missing == []
        # 基础分 40 + 默认等级加成 5 = 45
        assert abs(score - 45.0) < 0.1

    def test_partial_match(self, matcher: JobMatcher) -> None:
        persona = ["Python", "Docker"]
        job = ["Python", "Docker", "Kubernetes"]
        matched, missing, score = matcher._calculate_skill_match(persona, job)
        assert matched == ["Python", "Docker"]
        assert missing == ["Kubernetes"]
        # 基础分 (2/3)*40 ≈ 26.67 + 等级加成 5 ≈ 31.67
        assert abs(score - 31.67) < 0.1

    def test_no_match(self, matcher: JobMatcher) -> None:
        persona = ["Java"]
        job = ["Python"]
        matched, missing, score = matcher._calculate_skill_match(persona, job)
        assert matched == []
        assert missing == ["Python"]
        assert score == 0.0

    def test_empty_job_skills(self, matcher: JobMatcher) -> None:
        matched, missing, score = matcher._calculate_skill_match(["Python"], [])
        assert matched == []
        assert missing == []
        assert score == 25.0

    def test_case_insensitive(self, matcher: JobMatcher) -> None:
        persona = ["python", "docker"]
        job = ["Python", "DOCKER"]
        matched, missing, score = matcher._calculate_skill_match(persona, job)
        assert len(matched) == 2
        assert abs(score - 45.0) < 0.1

    def test_substring_match(self, matcher: JobMatcher) -> None:
        persona = ["kubernetes"]
        job = ["k8s"]
        matched, missing, score = matcher._calculate_skill_match(persona, job)
        # k8s 不在 kubernetes 中，但是 substring 逻辑不会匹配
        # 因为 "k8s" 不在 "kubernetes" 中，且 "kubernetes" 不在 "k8s" 中
        assert missing == ["k8s"]


class TestParseYearsRequirement:
    def test_range(self, matcher: JobMatcher) -> None:
        assert matcher._parse_years_requirement("3-5年") == 3

    def test_above(self, matcher: JobMatcher) -> None:
        assert matcher._parse_years_requirement("5年以上") == 5

    def test_plus(self, matcher: JobMatcher) -> None:
        assert matcher._parse_years_requirement("3年+") == 3

    def test_single_number(self, matcher: JobMatcher) -> None:
        assert matcher._parse_years_requirement("2") == 2

    def test_empty(self, matcher: JobMatcher) -> None:
        assert matcher._parse_years_requirement("") == 0.0
        assert matcher._parse_years_requirement(None) == 0.0


class TestCalculateTotalYears:
    def test_single_experience(self, matcher: JobMatcher) -> None:
        exp = Experience(
            id="e1",
            user_id="u1",
            status="confirmed",
            start_date=date(2020, 1, 1),
            end_date=date(2023, 1, 1),
        )
        years = matcher._calculate_total_years([exp])
        assert abs(years - 3.0) < 0.1

    def test_ongoing_experience(self, matcher: JobMatcher) -> None:
        exp = Experience(
            id="e1",
            user_id="u1",
            status="confirmed",
            start_date=date(2022, 1, 1),
            end_date=None,
        )
        years = matcher._calculate_total_years([exp])
        assert years > 0  # 至少有几年

    def test_no_experiences(self, matcher: JobMatcher) -> None:
        assert matcher._calculate_total_years([]) == 0.0


class TestOtherMatch:
    def test_no_requirements(self, matcher: JobMatcher) -> None:
        persona = Persona(id="p1", user_id="u1", name="测试")
        job = JobDesc(id="j1", raw_text="test")
        score = matcher._calculate_other_match(persona, job)
        assert score == 10.0  # 无学历无地点要求 = 满分

    def test_location_match(self, matcher: JobMatcher) -> None:
        persona = Persona(
            id="p1", user_id="u1", name="测试",
            target_job_profiles=["北京 产品经理"]
        )
        job = JobDesc(id="j1", raw_text="test", location="北京")
        score = matcher._calculate_other_match(persona, job)
        # 无学历要求 = 5分，地点匹配 = 5分，总计 10 分
        assert score == 10.0


class TestValidStatuses:
    def test_valid_statuses(self, matcher: JobMatcher) -> None:
        valid = ["new", "interested", "applied", "interviewing", "offered",
                 "rejected", "ghosted", "accepted", "declined"]
        for s in valid:
            assert s in matcher._VALID_STATUSES

    def test_invalid_status(self, matcher: JobMatcher) -> None:
        assert "invalid" not in matcher._VALID_STATUSES
