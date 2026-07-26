"""
CareerCraft Agent — JobMatch 匹配分析 Prompt

比较角色档案 + 经历 vs JD，计算综合匹配度分数。
"""

from __future__ import annotations

JOB_MATCHING_PROMPT = """你是一位职业规划师，擅长将求职者的背景与岗位要求进行系统性匹配分析。
请对以下求职者档案和岗位描述进行深度匹配分析，返回严格的JSON对象。

## 角色档案
- 角色名称：{persona_name}
- 身份声明：{identity_statement}
- 技能权重：{capability_weights}

## 求职者经历摘要
{experiences_summary}

## 目标岗位
- 岗位名称：{job_title}
- 公司：{job_company}
- 岗位要求技能：{job_skills}
- 岗位职责：{job_responsibilities}
- 工作地点：{job_location}
- 经验年限要求：{job_years}
- 学历要求：{job_education}

## 输出字段
- `match_score`: 整体匹配分数，整数 0 ~ 100
- `matched_skills`: 匹配成功的技能列表
- `missing_skills`: 缺失的技能列表
- `score_breakdown`: 分项得分对象，必含以下键：
  - `skills_match`: 技能匹配得分 0~100
  - `experience_relevance`: 经历相关性得分 0~100
  - `role_fit`: 角色定位契合度得分 0~100
- `ai_analysis`: 一段简要的AI分析文本，说明求职者优势、差距和建议

## 评分原则
1. skills_match：按匹配技能占岗位要求技能的比例评分
2. experience_relevance：按经历与岗位职责的相关度评分
3. role_fit：按角色定位与岗位文化、层级的契合度评分
4. match_score = round(skills_match * 0.4 + experience_relevance * 0.4 + role_fit * 0.2)

## 输出
请只返回JSON，不要任何其他解释。
"""


def build_job_matching_prompt(
    persona_name: str,
    identity_statement: str,
    capability_weights: str,
    experiences_summary: str,
    job_title: str,
    job_company: str,
    job_skills: str,
    job_responsibilities: str,
    job_location: str,
    job_years: str,
    job_education: str,
) -> str:
    return JOB_MATCHING_PROMPT.format(
        persona_name=persona_name,
        identity_statement=identity_statement or "",
        capability_weights=capability_weights or "",
        experiences_summary=experiences_summary or "",
        job_title=job_title or "",
        job_company=job_company or "",
        job_skills=job_skills or "",
        job_responsibilities=job_responsibilities or "",
        job_location=job_location or "",
        job_years=job_years or "",
        job_education=job_education or "",
    )
