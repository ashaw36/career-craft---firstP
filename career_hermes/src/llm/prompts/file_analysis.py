"""
CareerCraft Agent — LLM Prompt: 文件材料分析提取经历

输入：项目文件、工作汇报、总结等原始材料
输出：结构化的经历列表（JSON 数组）
"""

from __future__ import annotations


def build_file_analysis_prompt(file_content: str, file_type: str = "未知") -> str:
    """
    构建文件分析 prompt。

    Args:
        file_content: 文件全文
        file_type: 文件类型描述（如"项目总结报告"、"工作汇报"等）
    """
    return f"""你是一位资深的人力资源顾问和职业发展专家。你的任务是从用户提供的工作材料中提取结构化经历。

## 输入材料类型
{file_type}

## 材料内容
```
{file_content[:8000]}
```

## 提取要求
请从材料中提取所有可量化的工作经历和项目经历，输出为严格JSON数组格式：

```json
[
  {{
    "title": "经历标题（如：XX项目负责/产品经理）",
    "type": "work 或 project 或 education",
    "organization": "公司/组织名称",
    "start_date": "开始日期，格式YYYY-MM-DD或YYYY-MM，如无法确定可为null",
    "end_date": "结束日期，格式同上，进行中可为null",
    "raw_description": "这段经历的详细描述，200-500字，保留关键细节",
    "structured_achievements": ["成就1：用数据量化", "成就2：用数据量化"],
    "skills_demonstrated": ["技能1", "技能2", "技能3"],
    "metrics": [
      {{"metric": "指标名称", "value": "数值", "unit": "单位"}}
    ]
  }}
]
```

## 注意事项
1. 必须提取至少1条经历，多则不限
2. 每条经历的 achievements 必须包含可量化数据（如"提升效率40%"、"负责100万GMV"）
3. skills_demonstrated 必须与岗位相关，不要泛泛而谈
4. metrics 数组尽量填充，没有确切数据时可写估算值并标注"预估"
5. 时间如无法精确到日，至少精确到月
6. 如果材料中包含多个项目/经历，请分别提取
7. 输出必须是合法JSON，不要有任何其他文字
"""
