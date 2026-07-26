"""
CareerCraft Agent — 经历批量导入解析器

支持 Markdown/纯文本/JSON 三种格式，自动解析为 work/project/education 经历实体。
"""

from __future__ import annotations

import json
import logging
import re
from dataclasses import dataclass
from datetime import date, datetime
from typing import Any, Dict, List, Optional

from src.llm.prompts.file_analysis import build_file_analysis_prompt
from src.llm.router import LLMRouter
from src.services.experience_manager import ExperienceDraft

logger = logging.getLogger(__name__)


@dataclass
class ParsedExperience:
    """解析后的经历中间结果"""

    title: str
    exp_type: str
    organization: Optional[str] = None
    start_date: Optional[date] = None
    end_date: Optional[date] = None
    raw_description: str = ""
    skills_demonstrated: Optional[List[str]] = None
    structured_achievements: Optional[List[str]] = None
    metrics: Optional[List[Dict[str, str]]] = None


class ImportParserError(Exception):
    pass


class ImportParser:
    """
    经历批量导入解析器

    使用示例：
        parser = ImportParser()
        drafts = await parser.parse_markdown(md_text)
        drafts = await parser.parse_text(text)
        drafts = await parser.parse_json(json_text)
        drafts = await parser.analyze_file_with_llm(file_content, file_type="项目总结")
    """

    def __init__(self, llm_router: Optional[LLMRouter] = None) -> None:
        self.llm = llm_router or LLMRouter()

    # 常见时间格式正则
    DATE_PATTERNS = [
        # 2020.01 - 2023.06
        r"(\d{4})[\.\-/](\d{1,2})\s*[-—~]\s*(\d{4})[\.\-/](\d{1,2})",
        # 2020.01 - 至今
        r"(\d{4})[\.\-/](\d{1,2})\s*[-—~]\s*(至今|现在|present|now)",
        # 2020年7月 - 2023年6月
        r"(\d{4})年(\d{1,2})月\s*[-—~]\s*(\d{4})年(\d{1,2})月",
        # 2020年7月 - 至今
        r"(\d{4})年(\d{1,2})月\s*[-—~]\s*(至今|现在|present)",
    ]

    async def parse_markdown(self, text: str) -> List[ExperienceDraft]:
        """
        解析 Markdown 格式的经历文本。

        支持以下结构：
        ## 工作经历
        ### 公司名 | 职位名
        2020.01 - 2023.06

        - 描述或成就
        - 描述或成就

        ## 项目经历
        ### 项目名
        ...
        """
        experiences: List[ParsedExperience] = []
        lines = text.splitlines()
        i = 0
        current_section = ""

        while i < len(lines):
            line = lines[i].strip()

            # 识别一级标题 ## 工作经历 / ## 项目经历 / ## 教育背景
            if line.startswith("## "):
                section_title = line[3:].strip().lower()
                if any(k in section_title for k in ["工作", "work", "职位", "career"]):
                    current_section = "work"
                elif any(k in section_title for k in ["项目", "project"]):
                    current_section = "project"
                elif any(k in section_title for k in ["教育", "education", "学历", "学校"]):
                    current_section = "education"
                else:
                    current_section = ""
                i += 1
                continue

            # 识别三级标题 ### 公司名 | 职位
            if line.startswith("### "):
                header = line[4:].strip()
                # 尝试分离公司和职位（通过 | 或 @ 或 在某某 分隔）
                org, title = self._split_header(header)

                # 收集该经历的所有行
                exp_lines: List[str] = []
                i += 1
                while i < len(lines):
                    next_line = lines[i].strip()
                    if next_line.startswith("## ") or next_line.startswith("### "):
                        break
                    exp_lines.append(lines[i])
                    i += 1

                parsed = self._parse_experience_block(
                    exp_lines, title=title, organization=org, exp_type=current_section
                )
                if parsed:
                    experiences.append(parsed)
                continue

            i += 1

        # 如果没有找到标准的 Markdown 结构，尝试按空行分块解析
        if not experiences:
            experiences = self._parse_plain_blocks(text)

        return [self._to_draft(e) for e in experiences]

    async def parse_text(self, text: str) -> List[ExperienceDraft]:
        """解析纯文本格式的经历。"""
        # 纯文本直接使用按块解析
        experiences = self._parse_plain_blocks(text)
        return [self._to_draft(e) for e in experiences]

    async def parse_json(self, text: str) -> List[ExperienceDraft]:
        """解析 JSON 格式的经历数组。"""
        try:
            data = json.loads(text)
        except json.JSONDecodeError as e:
            raise ImportParserError(f"JSON 解析失败: {e}")

        if not isinstance(data, list):
            data = [data]

        experiences: List[ParsedExperience] = []
        for item in data:
            if not isinstance(item, dict):
                continue
            exp = self._dict_to_experience(item)
            if exp:
                experiences.append(exp)

        return [self._to_draft(e) for e in experiences]

    @classmethod
    def _split_header(cls, header: str) -> tuple:
        """将标题分离为 (组织, 职位/标题)"""
        separators = [" | ", " @ ", " 在 ", " - ", " — ", "\t"]
        for sep in separators:
            if sep in header:
                parts = header.split(sep, 1)
                return parts[0].strip(), parts[1].strip()
        # 默认整个标题作为职位，组织为空
        return None, header

    @classmethod
    def _parse_experience_block(
        cls,
        lines: List[str],
        title: str,
        organization: Optional[str],
        exp_type: str,
    ) -> Optional[ParsedExperience]:
        """解析经历块的详细内容"""
        if not title:
            return None

        text = "\n".join(lines)
        start_date, end_date = cls._extract_dates(text)
        description, achievements = cls._extract_description_and_achievements(lines)
        skills = cls._extract_skills(text)

        # 如果类型为空，尝试自动判断
        if not exp_type:
            exp_type = cls._infer_type(title, organization, text)

        return ParsedExperience(
            title=title,
            exp_type=exp_type,
            organization=organization,
            start_date=start_date,
            end_date=end_date,
            raw_description=description,
            skills_demonstrated=skills if skills else None,
            structured_achievements=achievements if achievements else None,
        )

    @classmethod
    def _parse_plain_blocks(cls, text: str) -> List[ParsedExperience]:
        """按空行分块解析经历"""
        blocks = [b.strip() for b in text.split("\n\n") if b.strip()]
        experiences: List[ParsedExperience] = []

        for block in blocks:
            lines = block.splitlines()
            if not lines:
                continue

            # 第一行作为标题
            header = lines[0].strip()
            org, title = cls._split_header(header)

            parsed = cls._parse_experience_block(
                lines[1:], title=title, organization=org, exp_type=""
            )
            if parsed and parsed.title:
                experiences.append(parsed)

        return experiences

    @classmethod
    def _extract_dates(cls, text: str) -> tuple:
        """从文本中提取日期范围"""
        for pattern in cls.DATE_PATTERNS:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                groups = match.groups()
                try:
                    start_year = int(groups[0])
                    start_month = int(groups[1])
                    start_date = date(start_year, start_month, 1)

                    if len(groups) >= 4 and groups[2].isdigit():
                        end_year = int(groups[2])
                        end_month = int(groups[3])
                        end_date = date(end_year, end_month, 1)
                    elif len(groups) >= 3 and groups[2].lower() in (
                        "至今", "现在", "present", "now"
                    ):
                        end_date = None
                    else:
                        end_date = None

                    return start_date, end_date
                except (ValueError, IndexError):
                    continue
        return None, None

    @classmethod
    def _extract_description_and_achievements(
        cls, lines: List[str]
    ) -> tuple:
        """提取描述和成就列表"""
        achievements: List[str] = []
        description_lines: List[str] = []

        for line in lines:
            stripped = line.strip()
            if not stripped:
                continue
            # 识别列表项作为成就
            if re.match(r"^[-*+•\d+\.\)]\s+", stripped):
                achievements.append(re.sub(r"^[-*+•\d+\.\)]\s+", "", stripped))
            else:
                description_lines.append(line)

        description = "\n".join(description_lines).strip()
        return description, achievements if achievements else None

    @classmethod
    def _extract_skills(cls, text: str) -> List[str]:
        """提取技能关键词（简单规则）"""
        # 常见技能关键词库（可扩展）
        skill_keywords = [
            "python", "java", "go", "rust", "c++", "javascript", "typescript",
            "sql", "mysql", "postgresql", "mongodb", "redis", "elasticsearch",
            "aws", "azure", "gcp", "docker", "kubernetes", "terraform",
            "react", "vue", "angular", "django", "flask", "fastapi", "spring",
            "machine learning", "deep learning", "nlp", "computer vision",
            "data analysis", "data science", "a/b testing", "product management",
            "agile", "scrum", "jira", "confluence", "figma", "sketch",
            "产品规划", "数据分析", "用户研究", "项目管理",
            "机器学习", "深度学习", "自然语言处理",
            "linux", "git", "ci/cd", "github actions", "jenkins",
        ]

        found_skills: List[str] = []
        text_lower = text.lower()
        for skill in skill_keywords:
            if skill.lower() in text_lower:
                found_skills.append(skill)
        return found_skills

    @classmethod
    def _infer_type(
        cls, title: str, organization: Optional[str], text: str
    ) -> str:
        """根据内容推断经历类型"""
        text_lower = text.lower()
        title_lower = title.lower()

        if any(k in title_lower for k in ["硕士", "博士", "本科", "学士", "mba", "master", "bachelor", "phd"]):
            return "education"
        if any(k in text_lower for k in ["毕业", "学校", "专业", "学历", "university", "college", "degree"]):
            return "education"
        if any(k in title_lower for k in ["项目", "project", "平台", "系统"]):
            return "project"
        # 默认工作经历
        return "work"

    @classmethod
    def _dict_to_experience(cls, data: Dict[str, Any]) -> Optional[ParsedExperience]:
        """将字典转换为 ParsedExperience"""
        title = data.get("title") or data.get("标题") or data.get("职位")
        if not title:
            return None

        exp_type = data.get("type") or data.get("类型") or "work"
        org = data.get("organization") or data.get("公司") or data.get("组织")
        desc = data.get("raw_description") or data.get("描述") or data.get("description") or ""
        skills = data.get("skills_demonstrated") or data.get("技能") or data.get("skills")
        achievements = data.get("structured_achievements") or data.get("成就") or data.get("achievements")

        start_date = cls._parse_date(data.get("start_date") or data.get("开始日期"))
        end_date = cls._parse_date(data.get("end_date") or data.get("结束日期"))

        return ParsedExperience(
            title=title,
            exp_type=exp_type,
            organization=org,
            start_date=start_date,
            end_date=end_date,
            raw_description=desc,
            skills_demonstrated=skills if skills else None,
            structured_achievements=achievements if achievements else None,
        )

    @classmethod
    def _parse_date(cls, value: Any) -> Optional[date]:
        """解析日期字符串

        支持格式：YYYY-MM-DD、YYYY-MM、YYYY.MM、YYYY/MM、YYYY年MM月
        """
        if not value:
            return None
        if isinstance(value, date):
            return value
        if isinstance(value, str):
            for fmt in ["%Y-%m-%d", "%Y-%m", "%Y.%m", "%Y/%m", "%Y年%m月"]:
                try:
                    if fmt == "%Y-%m-%d":
                        # 先尝试完整 ISO 日期；YYYY-MM 不是标准 ISO 日期，会被后续格式捕获
                        if len(value) == 10:
                            return date.fromisoformat(value)
                        continue
                    return datetime.strptime(value, fmt).date()
                except (ValueError, ImportError):
                    continue
        return None

    @classmethod
    def _to_draft(cls, parsed: ParsedExperience) -> ExperienceDraft:
        """将解析结果转换为 ExperienceDraft"""
        return ExperienceDraft(
            raw_text=parsed.raw_description or parsed.title,
            extracted={
                "title": parsed.title,
                "organization": parsed.organization,
                "type": parsed.exp_type,
                "start_date": parsed.start_date.isoformat() if parsed.start_date else None,
                "end_date": parsed.end_date.isoformat() if parsed.end_date else None,
                "raw_description": parsed.raw_description,
                "skills_demonstrated": parsed.skills_demonstrated,
                "structured_achievements": parsed.structured_achievements,
                "metrics": parsed.metrics,
            },
        )

    @staticmethod
    def extract_text_from_pdf(file_bytes: bytes) -> str:
        """从 PDF 文件提取文本。"""
        import fitz

        doc = fitz.open(stream=file_bytes, filetype="pdf")
        try:
            text_parts: List[str] = []
            for page in doc:
                text_parts.append(page.get_text())
            return "\n".join(text_parts)
        finally:
            doc.close()

    @staticmethod
    def extract_text_from_word(file_bytes: bytes) -> str:
        """从 Word 文件提取文本。"""
        import io

        from docx import Document

        doc = Document(io.BytesIO(file_bytes))
        text_parts: List[str] = []
        for para in doc.paragraphs:
            if para.text.strip():
                text_parts.append(para.text)
        return "\n".join(text_parts)

    async def import_file(self, file_name: str, file_bytes: bytes) -> List[ExperienceDraft]:
        """
        导入 PDF/Word/文本文件，提取全文后交给 LLM 分析。

        Args:
            file_name: 原始文件名（用于判断扩展名）
            file_bytes: 文件二进制内容

        Returns:
            ExperienceDraft 列表
        """
        ext = file_name.lower().rsplit(".", 1)[-1] if "." in file_name else ""
        if ext == "pdf":
            text = self.extract_text_from_pdf(file_bytes)
        elif ext in ("doc", "docx"):
            text = self.extract_text_from_word(file_bytes)
        else:
            text = file_bytes.decode("utf-8", errors="ignore")

        if not text.strip():
            raise ImportParserError(f"无法从 {file_name} 提取文本")

        logger.info("从 %s 提取文本: %d 字符", file_name, len(text))
        return await self.analyze_file_with_llm(text, file_type=file_name)

    async def analyze_file_with_llm(
        self, file_content: str, file_type: str = "未知"
    ) -> List[ExperienceDraft]:
        """
        使用 LLM 分析文件内容，自动提取结构化经历。

        Args:
            file_content: 文件全文内容
            file_type: 文件类型描述

        Returns:
            ExperienceDraft 列表
        """
        prompt = build_file_analysis_prompt(file_content, file_type)
        try:
            response = await self.llm.chat(
                messages=[{"role": "user", "content": prompt}],
                temperature=0.3,
            )
        except Exception as e:
            logger.error("文件分析 LLM 调用失败: %s", e)
            raise ImportParserError(f"LLM 分析失败: {e}") from e

        if not isinstance(response, str):
            raise ImportParserError("LLM 返回类型异常")

        # 提取 JSON 代码块
        json_str = response.strip()
        if "```json" in json_str:
            json_str = json_str.split("```json")[1].split("```")[0].strip()
        elif "```" in json_str:
            json_str = json_str.split("```")[1].split("```")[0].strip()

        try:
            items = json.loads(json_str)
        except json.JSONDecodeError as e:
            logger.warning("文件分析 JSON 解析失败，尝试整体解析: %s", e)
            # 尝试从响应中提取第一个 JSON 数组（非贪婪匹配，保留换行）
            match = re.search(r"\[.*?\]", response, re.DOTALL)
            if not match:
                raise ImportParserError(f"无法解析 LLM 返回: {e}")
            try:
                items = json.loads(match.group(0))
            except json.JSONDecodeError as e2:
                raise ImportParserError(f"无法解析 LLM 返回: {e2}")

        if not isinstance(items, list):
            items = [items]

        experiences: List[ParsedExperience] = []
        for item in items:
            if not isinstance(item, dict):
                continue
            # 处理 metrics 格式
            metrics_raw = item.get("metrics") or []
            metrics = None
            if metrics_raw and isinstance(metrics_raw, list):
                metrics = [
                    {"metric": str(m.get("metric", "")), "value": str(m.get("value", "")), "unit": str(m.get("unit", ""))}
                    for m in metrics_raw if isinstance(m, dict)
                ]

            title = item.get("title") or item.get("标题") or ""
            if not title:
                logger.warning("跳过无标题的经历条目")
                continue

            exp = ParsedExperience(
                title=title,
                exp_type=item.get("type", "work") or item.get("类型") or "work",
                organization=item.get("organization") or item.get("company") or item.get("公司"),
                start_date=self._parse_date(item.get("start_date") or item.get("开始日期")),
                end_date=self._parse_date(item.get("end_date") or item.get("结束日期")),
                raw_description=item.get("raw_description") or item.get("description") or item.get("描述") or "",
                skills_demonstrated=item.get("skills_demonstrated") or item.get("skills") or item.get("技能"),
                structured_achievements=item.get("structured_achievements") or item.get("achievements") or item.get("成就"),
                metrics=metrics if metrics else None,
            )
            experiences.append(exp)

        logger.info("文件分析完成，提取经历: %d 条", len(experiences))
        return [self._to_draft(e) for e in experiences]
