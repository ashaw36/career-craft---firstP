"""
CareerCraft Agent — 经历结构化提取 Prompt

用户自然语言 → LLM 结构化 → 用户确认
"""

from __future__ import annotations

EXPERIENCE_EXTRACTION_PROMPT = """你是一个职业经历分析助手。请将用户的自然语言描述解构化为JSON格式。

## 输入
用户提供的原始经历描述：
```
{raw_text}
```

## 输出要求
请返回严格的JSON对象，包含以下字段：
- `title`: 职位名称或项目名称（必填）
- `organization`: 公司/组织名称（如果有）
- `type`: 经历类型，可选值："work"、"project"、"education"、"certification"
- `start_date`: 开始日期，格式 YYYY-MM-DD 或 null
- `end_date`: 结束日期，格式 YYYY-MM-DD 或 null（null表示至今）
- `structured_achievements`: 字符串数组，每条是一个具体、可量化的成就点（至少1条）
- `skills_demonstrated`: 字符串数组，这段经历中体现的技能标签（至少1个）
- `metrics`: 对象数组，每个对象含 name, value, unit（可选）

## 重要约束
1. 事实不可变：不能编造不存在的信息，时间、公司名必须严格来自原文
2. 如果日期不明确，返回 null，不能胡猜
3. 技能标签要精练，避免过于泛化（如用"Python"而非"编程"）
4. 中英文输入均支持

## 输出
请只返回JSON，不要任何其他解释。
"""


def build_extraction_prompt(raw_text: str) -> str:
    return EXPERIENCE_EXTRACTION_PROMPT.format(raw_text=raw_text)
