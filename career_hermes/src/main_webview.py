"""
CareerCraft Agent — WebView 版入口

启动 PySide6 + QWebEngineView 混合框架，加载 HTML 原型并注入 Python Bridge。
"""

from __future__ import annotations

import logging
import sys
from typing import List

from PySide6.QtCore import Qt
from PySide6.QtWidgets import QApplication

from src.models.database import init_db
from src.ui.webview.webview_window import CareerWebWindow

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s: %(message)s")
logger = logging.getLogger(__name__)


def main(argv: List[str] = sys.argv) -> int:
    """主入口"""
    # 高 DPI 适配
    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )

    app = QApplication(argv)
    app.setApplicationName("CareerCraft Agent")
    app.setApplicationVersion("0.1.0")

    # 初始化数据库（包含 schema 修复）
    import asyncio
    try:
        asyncio.run(init_db())
        logger.info("数据库初始化完成")
    except Exception as e:
        logger.warning(f"数据库初始化异常，应用将继续运行: {e}")

    window = CareerWebWindow()
    window.show()

    logger.info("CareerCraft Agent WebView 已启动")
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
