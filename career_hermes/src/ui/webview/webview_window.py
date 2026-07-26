"""
CareerCraft Agent — WebView 主窗口

PySide6 QWebEngineView 容器，加载本地 HTML 原型，
注入 QWebChannel 桥接以实现 Python ↔ JS 通信。
"""

from __future__ import annotations

from pathlib import Path
from typing import List, Optional

from PySide6.QtCore import QUrl
from PySide6.QtGui import QDesktopServices
from PySide6.QtWebChannel import QWebChannel
from PySide6.QtWebEngineCore import QWebEnginePage, QWebEngineSettings
from PySide6.QtWebEngineWidgets import QWebEngineView
from PySide6.QtWidgets import QMainWindow

from src.ui.webview.bridge import CareerBridge


class ExternalLinkPage(QWebEnginePage):
    """Open clicked external links in the system browser."""

    def acceptNavigationRequest(
        self,
        url: QUrl,
        navigation_type: QWebEnginePage.NavigationType,
        is_main_frame: bool,
    ) -> bool:
        if (
            navigation_type
            == QWebEnginePage.NavigationType.NavigationTypeLinkClicked
            and url.scheme() in ("http", "https")
        ):
            QDesktopServices.openUrl(url)
            return False
        return super().acceptNavigationRequest(url, navigation_type, is_main_frame)


class CareerWebWindow(QMainWindow):
    """
    CareerCraft Agent WebView 主窗口

    特性：
    - 加载本地 HTML 原型 (prototype/ui-prototype.html)
    - 注入 QWebChannel 供 JS 调用 Python API
    - F12 开启 DevTools
    - 窗口大小 1280x800，深色标题栏
    """

    def __init__(self, parent: Optional[QMainWindow] = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("CareerCraft Agent")
        self.resize(1280, 800)

        # 创建 WebEngineView
        self.web_view = QWebEngineView(self)
        self.web_page = ExternalLinkPage(self.web_view)
        self.web_view.setPage(self.web_page)
        self.setCentralWidget(self.web_view)

        # 配置 WebEngine 设置
        settings = self.web_view.settings()
        dev_extras = getattr(
            QWebEngineSettings.WebAttribute, "DeveloperExtrasEnabled", None
        )
        if dev_extras is not None:
            settings.setAttribute(dev_extras, True)
        settings.setAttribute(
            QWebEngineSettings.WebAttribute.LocalStorageEnabled, True
        )
        settings.setAttribute(
            QWebEngineSettings.WebAttribute.JavascriptEnabled, True
        )

        # 创建并注册 QWebChannel + Bridge
        self.bridge = CareerBridge(self)
        self.channel = QWebChannel(self.web_page)
        self.channel.registerObject("pybridge", self.bridge)
        self.web_page.setWebChannel(self.channel)

        # 加载本地 HTML 原型
        html_path = self._resolve_html_path()
        self.web_view.load(QUrl.fromLocalFile(html_path))

    def _resolve_html_path(self) -> str:
        """解析 HTML 文件绝对路径"""
        source_root = Path(__file__).resolve().parents[3]
        candidates: List[Path] = [
            source_root / "prototype" / "ui-prototype.html",
            Path.cwd() / "prototype" / "ui-prototype.html",
        ]

        for path in candidates:
            if path.is_file():
                return str(path)
        return str(candidates[0])

    def keyPressEvent(self, event) -> None:
        """F12 打开 DevTools"""
        from PySide6.QtCore import Qt
        if event.key() == Qt.Key.Key_F12:
            self.web_view.page().setDevToolsPage(QWebEnginePage(self.web_view))
            self.web_view.triggerPageAction(QWebEngineView.WebAction.InspectElement)
        else:
            super().keyPressEvent(event)
