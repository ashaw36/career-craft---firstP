"""
CareerCraft Agent — PDF 简历导出服务

使用 fpdf2 生成专业排版的 PDF 简历。
"""

from __future__ import annotations

import logging
from datetime import date
from pathlib import Path
from typing import List, Optional

from src.models.entities import Experience, Persona

logger = logging.getLogger(__name__)


try:
    from fpdf import FPDF
    FPDF_AVAILABLE = True
except ImportError:
    FPDF_AVAILABLE = False
    logger.warning("fpdf2 未安装，PDF 导出功能不可用")


class PDFExporterError(Exception):
    """PDF 导出异常"""


class ResumePDF(FPDF if FPDF_AVAILABLE else object):
    """
    简历 PDF 生成器

    使用流程：
        exporter = PDFExporter()
        pdf_bytes = await exporter.export_resume(persona, experiences, output_path)
    """

    def __init__(self) -> None:
        if not FPDF_AVAILABLE:
            raise PDFExporterError("fpdf2 未安装，请运行: pip install fpdf2")
        super().__init__()
        self.set_auto_page_break(auto=True, margin=15)
        self.add_page()
        self.set_margins(15, 15, 15)

    def header_text(self, text: str, size: int = 12, bold: bool = False) -> None:
        """添加标题文本"""
        if bold:
            self.set_font("DejaVu", "B", size)
        else:
            self.set_font("DejaVu", "", size)
        self.cell(0, 8, text, ln=True)

    def body_text(self, text: str, size: int = 10) -> None:
        """添加正文文本"""
        self.set_font("DejaVu", "", size)
        self.multi_cell(0, 6, text)
        self.ln(2)

    def add_section_title(self, title: str) -> None:
        """添加分隔线标题"""
        self.ln(4)
        self.set_font("DejaVu", "B", 12)
        self.set_text_color(33, 37, 41)
        self.cell(0, 8, title, ln=True)
        self.set_draw_color(33, 37, 41)
        self.line(15, self.get_y(), 195, self.get_y())
        self.ln(4)
        self.set_text_color(0, 0, 0)

    def add_experience_block(
        self,
        title: str,
        organization: Optional[str],
        period: str,
        description: str,
        skills: Optional[List[str]] = None,
    ) -> None:
        """添加经历块"""
        self.set_font("DejaVu", "B", 10)
        self.cell(0, 6, title, ln=True)
        self.set_font("DejaVu", "", 9)
        org_text = organization or ""
        if org_text:
            self.cell(0, 5, f"{org_text}  |  {period}", ln=True)
        else:
            self.cell(0, 5, period, ln=True)
        self.set_font("DejaVu", "", 9)
        self.multi_cell(0, 5, description)
        if skills:
            self.set_font("DejaVu", "", 8)
            self.set_text_color(80, 80, 80)
            self.cell(0, 5, f"技能: {', '.join(skills)}", ln=True)
            self.set_text_color(0, 0, 0)
        self.ln(3)


class PDFExporter:
    """
    PDF 导出器

    使用示例：
        exporter = PDFExporter()
        pdf_bytes = await exporter.export_resume(persona, experiences)
        # 或保存到文件
        await exporter.save_resume(persona, experiences, Path("resume.pdf"))
    """

    async def export_resume(
        self,
        persona: Persona,
        experiences: List[Experience],
    ) -> bytes:
        """
        将角色 + 经历导出为 PDF 字节。
        """
        if not FPDF_AVAILABLE:
            raise PDFExporterError("fpdf2 未安装")

        pdf = ResumePDF()

        # 尝试加载中文字体
        font_added = self._add_chinese_font(pdf)
        if not font_added:
            logger.warning("未找到中文字体，PDF 可能显示乱码")

        # 标题：姓名 + 角色
        pdf.set_font("DejaVu" if font_added else "Helvetica", "B", 18)
        pdf.cell(0, 10, persona.name, ln=True, align="C")
        pdf.set_font("DejaVu" if font_added else "Helvetica", "", 11)
        pdf.cell(0, 6, persona.identity_statement or "", ln=True, align="C")
        pdf.ln(5)

        # 角色摘要
        if persona.career_narrative:
            pdf.add_section_title("个人摘要")
            pdf.body_text(persona.career_narrative)

        # 技能
        if persona.capability_weights:
            pdf.add_section_title("技能")
            skills_text = "  •  ".join(
                f"{k} ({int(v * 100)}%)"
                for k, v in persona.capability_weights.items()
            )
            pdf.body_text(skills_text)

        # 工作经历
        work_exps = [e for e in experiences if e.type == "work" and e.status == "confirmed"]
        if work_exps:
            pdf.add_section_title("工作经历")
            for exp in sorted(
                work_exps,
                key=lambda e: e.start_date or date.min,
                reverse=True,
            ):
                period = self._format_period(exp.start_date, exp.end_date)
                desc = exp.raw_description or ""
                if exp.structured_achievements:
                    desc += "\n\n主要成就:\n"
                    desc += "\n".join(f"• {a}" for a in exp.structured_achievements)
                pdf.add_experience_block(
                    title=exp.title,
                    organization=exp.organization,
                    period=period,
                    description=desc,
                    skills=exp.skills_demonstrated,
                )

        # 项目经历
        proj_exps = [e for e in experiences if e.type == "project" and e.status == "confirmed"]
        if proj_exps:
            pdf.add_section_title("项目经历")
            for exp in sorted(
                proj_exps,
                key=lambda e: e.start_date or date.min,
                reverse=True,
            ):
                period = self._format_period(exp.start_date, exp.end_date)
                desc = exp.raw_description or ""
                pdf.add_experience_block(
                    title=exp.title,
                    organization=exp.organization,
                    period=period,
                    description=desc,
                    skills=exp.skills_demonstrated,
                )

        # 教育背景
        edu_exps = [e for e in experiences if e.type == "education" and e.status == "confirmed"]
        if edu_exps:
            pdf.add_section_title("教育背景")
            for exp in edu_exps:
                period = self._format_period(exp.start_date, exp.end_date)
                pdf.header_text(exp.title, size=10, bold=True)
                pdf.body_text(f"{exp.organization or ''}  |  {period}")

        return pdf.output(dest="S")

    async def save_resume(
        self,
        persona: Persona,
        experiences: List[Experience],
        output_path: Path,
    ) -> Path:
        """
        导出并保存到文件。

        安全校验：解析相对路径，防止路径穿越。
        """
        resolved = output_path.resolve()
        if ".." in resolved.parts:
            raise ValueError("文件路径不安全，包含非法的 .. 组件")
        pdf_bytes = await self.export_resume(persona, experiences)
        resolved.parent.mkdir(parents=True, exist_ok=True)
        resolved.write_bytes(pdf_bytes)
        logger.info("简历 PDF 已保存: %s", resolved)
        return resolved

    @staticmethod
    def _format_period(start: Optional[date], end: Optional[date]) -> str:
        """格式化日期区间"""
        s = start.strftime("%Y.%m") if start else "?"
        e = end.strftime("%Y.%m") if end else "至今"
        return f"{s} - {e}"

    @staticmethod
    def _add_chinese_font(pdf: FPDF) -> bool:
        """
        尝试添加中文字体。首先尝试系统字体，失败则返回 False。
        """
        font_paths = [
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ]
        for fp in font_paths:
            if Path(fp).exists():
                try:
                    pdf.add_font("DejaVu", "", fp, uni=True)
                    pdf.add_font("DejaVu", "B", fp, uni=True)
                    return True
                except Exception:
                    continue
        return False
