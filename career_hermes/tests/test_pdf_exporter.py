"""
CareerCraft Agent — PDF 导出单元测试
"""

from __future__ import annotations

import pytest

from src.services.pdf_exporter import PDFExporter, PDFExporterError, FPDF_AVAILABLE


class TestPDFExporter:
    def test_fpdf_availability(self) -> None:
        # fpdf2 可能未安装，但模块导入应正常
        assert isinstance(FPDF_AVAILABLE, bool)

    @pytest.mark.asyncio
    async def test_export_without_fpdf_raises(self) -> None:
        if FPDF_AVAILABLE:
            pytest.skip("fpdf2 已安装，跳过此测试")
        exporter = PDFExporter()
        with pytest.raises(PDFExporterError, match="fpdf2 未安装"):
            await exporter.export_resume(None, [])  # type: ignore[arg-type]

    def test_format_period(self) -> None:
        from datetime import date
        result = PDFExporter._format_period(date(2020, 1, 1), date(2023, 6, 1))
        assert result == "2020.01 - 2023.06"

    def test_format_period_ongoing(self) -> None:
        from datetime import date
        result = PDFExporter._format_period(date(2022, 3, 1), None)
        assert result == "2022.03 - 至今"

    def test_format_period_none(self) -> None:
        result = PDFExporter._format_period(None, None)
        assert result == "? - 至今"
