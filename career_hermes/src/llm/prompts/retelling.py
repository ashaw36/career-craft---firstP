"""
CareerCraft Agent — 经历重述 Prompt

根据角色档案和目标岗位，使用 LLM 将经历重新叙事，突出匹配度。
"""

from __future__ import annotations

EXPERIENCE_RETELLING_PROMPT = """你是一位职业简历撰写专家。请根据角色定位和目标岗位，对以下经历进行重述，突出与目标岗位的匹配度。

## 角色定位
- 角色名称：{persona_name}
- 身份声明：{identity_statement}
- 叙事风格：{tone_style}

## 目标岗位
- 岗位名称：{job_title}
- 公司：{job_company}
- 岗位要求技能：{job_skills}

## 原始经历
- 经历类型：{exp_type}
- 标题：{exp_title}
- 组织：{exp_organization}
- 原始描述：{exp_description}
- 已结构化成就：{exp_achievements}
- 已知技能：{exp_skills}

## 重述要求
1. 保持事实准确，不编造不存在的信息
2. 突出与目标岗位要求技能相关的成就和贡献
3. 使用数据和量化结果增强说服力
4. 语言风格符合角色定位的 tone_style
5. 重述后的文本长度应该与原始描述接近，不要大幅扩充

## 输出
请直接返回重述后的文本，不要任何其他解释。
"""


def build_retelling_prompt(
    persona_name: str,
    identity_statement: str,
    tone_style: str,
    job_title: str,
    job_company: str,
    job_skills: str,
    exp_type: str,
    exp_title: str,
    exp_organization: str,
    exp_description: str,
    exp_achievements: str,
    exp_skills: str,
) -> str:
    return EXPERIENCE_RETELLING_PROMPT.format(
        persona_name=persona_name,
        identity_statement=identity_statement or "",
        tone_style=tone_style or "business_insight",
        job_title=job_title or "",
        job_company=job_company or "",
        job_skills=job_skills or "",
        exp_type=exp_type or "",
        exp_title=exp_title or "",
        exp_organization=exp_organization or "",
        exp_description=exp_description or "",
        exp_achievements=exp_achievements or "",
        exp_skills=exp_skills or "",
    )
