"""
CareerCraft Agent — JD 解析 Prompt

岗位描述文本 → LLM 结构化提取 → JobDesc 数据模型
"""

from __future__ import annotations

JOB_PARSING_PROMPT = """你是一个招聘信息提取专家。请从以下岗位描述中提取关键信息，返回严格的JSON对象。

## 输入岗位描述
```
{raw_text}
```

## 输出字段
- `title`: 岗位名称（如"高级Python后端工程师"）
- `company`: 公司名称
- `years_of_experience`: 经验年限要求（如"3-5年"、"5年以上"）
- `salary_range`: 薪资区间（如"25k-40k"、"15薪×15"）
- `location`: 工作地点
- `job_type`: 岗位类型，仅允许以下值：full_time, part_time, contract, intern
- `education_requirement`: 学历要求（如"本科及以上"、"硕士"）
- `responsibilities`: 岗位职责列表（字符串数组）
- `benefits`: 福利待遇列表（字符串数组）
- `parsed_skills`: 要求技能标签列表（字符串数组）

## 重要约束
1. 如果某个字段在原文中没有明确提及，返回 `null`，不能胡猜
2. 技能标签要精练，避免过于泛化（如用"Python"而非"编程"）
3. 岗位职责列表尽量保持原文表述，但可以简化过于周边的描述
4. 如果岗位描述中同时包含多个岗位，请主要提取第一个岗位的信息

## 输出
请只返回JSON，不要任何其他解释。
"""


def build_job_parsing_prompt(raw_text: str) -> str:
    return JOB_PARSING_PROMPT.format(raw_text=raw_text)
