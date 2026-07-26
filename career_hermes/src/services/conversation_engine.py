"""
CareerCraft Agent — 对话引擎

支持自然语言调整简历、角色配置、经历录入。
核心职责：意图识别 → 参数提取 → 操作执行。
"""

from __future__ import annotations

import json
from typing import Any, Dict, Optional

from src.llm.router import LLMRouter
from src.services.experience_manager import ExperienceManager
from src.services.persona_engine import PersonaEngine


class ConversationAction:
    """对话操作结果"""

    def __init__(
        self,
        action_type: str,
        params: Dict[str, Any],
        explanation: str,
    ) -> None:
        self.action_type = action_type  # update_persona, add_experience, generate_resume, etc.
        self.params = params
        self.explanation = explanation


class ConversationEngine:
    """
    对话引擎

    使用示例：
        engine = ConversationEngine()
        action = await engine.process("把我的简历调成更数据驱动的语气")
        # action.action_type == "update_persona"
        # action.params == {"tone_style": "data_driven"}
    """

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()
        self.exp_mgr = ExperienceManager(llm_router=self.llm)
        self.persona_engine = PersonaEngine()

    async def process(self, user_input: str, context: Optional[Dict[str, Any]] = None) -> ConversationAction:
        """
        处理用户自然语言输入，返回结构化操作
        """
        system_prompt = """你是 CareerCraft Agent 的对话意图识别模块。请将用户的自然语言输入解析为结构化操作。

可能的 action_type：
- `update_persona`: 更新角色配置（如语气、身份声明、目标岗位）
- `add_experience`: 添加新经历
- `generate_resume`: 生成简历
- `switch_persona`: 切换角色
- `ask_clarification`: 需要用户确认或补充信息

请返回严格JSON格式：
{
  "action_type": "...",
  "params": {...},
  "explanation": "..."
}
"""
        ctx_str = json.dumps(context, ensure_ascii=False, default=str) if context else "{}"

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": f"当前上下文: {ctx_str}\n\n用户输入: {user_input}"},
        ]

        response = await self.llm.chat(messages=messages, json_mode=True, temperature=0.3)
        if not isinstance(response, str):
            raise RuntimeError("对话引擎未预期的流式返回")

        try:
            parsed = json.loads(response)
        except json.JSONDecodeError:
            # 尝试从 markdown 代码块提取
            parsed = self._extract_json_from_markdown(response)

        return ConversationAction(
            action_type=parsed.get("action_type", "ask_clarification"),
            params=parsed.get("params", {}),
            explanation=parsed.get("explanation", ""),
        )

    @staticmethod
    def _extract_json_from_markdown(text: str) -> Dict[str, Any]:
        """从 markdown JSON 代码块中提取 JSON"""
        import re
        pattern = r"```(?:json)?\s*\n?(.*?)\n?```"
        match = re.search(pattern, text, re.DOTALL)
        if match:
            return json.loads(match.group(1).strip())
        raise ValueError(f"无法解析JSON: {text[:200]}")
