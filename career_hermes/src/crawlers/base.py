"""
CareerCraft Agent — 爬虫基类

封装 Playwright 浏览器启动、页面等待、反检测。
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import random
from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


try:
    from playwright.async_api import async_playwright, Browser, BrowserContext, Page
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False
    logger.warning("playwright 未安装，爬虫功能不可用")

try:
    from playwright_stealth import Stealth
    PLAYWRIGHT_STEALTH_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_STEALTH_AVAILABLE = False
    logger.warning("playwright-stealth 未安装，Stealth 模式不可用")


class CrawlerError(Exception):
    """爬虫异常"""


# 桌面浏览器 User-Agent 池
USER_AGENT_POOL: List[str] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
]


class BaseCrawler(ABC):
    """
    爬虫基类

    子类实现 search() 方法即可。
    """

    def __init__(
        self,
        headless: bool = True,
        slow_mo: int = 100,
        delay_range: Tuple[int, int] = (2, 5),
        cookie_path: Optional[str] = None,
    ) -> None:
        self.headless = headless
        self.slow_mo = slow_mo
        self.delay_range = delay_range
        self.cookie_path = cookie_path
        self._browser: Optional[Browser] = None
        self._context: Optional[BrowserContext] = None
        self._page: Optional[Page] = None
        self._playwright: Optional[Any] = None

    def _pick_user_agent(self) -> str:
        """随机选择一个 User-Agent"""
        return random.choice(USER_AGENT_POOL)

    async def _random_delay(self) -> None:
        """在 delay_range 范围内随机延迟"""
        delay = random.uniform(self.delay_range[0], self.delay_range[1])
        await asyncio.sleep(delay)

    async def _load_cookies(self) -> None:
        """从本地文件加载 cookies 到当前 context"""
        if self.cookie_path and self._context and os.path.exists(self.cookie_path):
            try:
                with open(self.cookie_path, "r", encoding="utf-8") as f:
                    cookies = json.load(f)
                await self._context.add_cookies(cookies)
                logger.info("已加载 cookies: %s", self.cookie_path)
            except Exception as e:
                logger.warning("加载 cookies 失败: %s", e)

    async def save_cookies(self) -> None:
        """将当前 context 的 cookies 保存到本地文件"""
        if self.cookie_path and self._context:
            try:
                cookies = await self._context.cookies()
                os.makedirs(os.path.dirname(self.cookie_path), exist_ok=True)
                with open(self.cookie_path, "w", encoding="utf-8") as f:
                    json.dump(cookies, f, ensure_ascii=False, indent=2)
                logger.info("已保存 cookies: %s", self.cookie_path)
            except Exception as e:
                logger.warning("保存 cookies 失败: %s", e)

    async def _launch(self) -> Page:
        """启动浏览器"""
        if not PLAYWRIGHT_AVAILABLE:
            raise CrawlerError("playwright 未安装，请运行: pip install playwright && playwright install")

        if self._browser is None:
            self._playwright = await async_playwright().start()
            self._browser = await self._playwright.chromium.launch(
                headless=self.headless,
                slow_mo=self.slow_mo,
            )

            user_agent = self._pick_user_agent()
            extra_headers: Dict[str, str] = {
                "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
                "Referer": "https://www.zhipin.com/",
            }

            self._context = await self._browser.new_context(
                user_agent=user_agent,
                viewport={"width": 1920, "height": 1080},
                locale="zh-CN",
                timezone_id="Asia/Shanghai",
                extra_http_headers=extra_headers,
            )
            self._page = await self._context.new_page()

            # 加载 cookies
            await self._load_cookies()

            # 启用 Stealth 模式
            if PLAYWRIGHT_STEALTH_AVAILABLE:
                stealth = Stealth()
                await stealth.apply_stealth_async(self._page)
                logger.debug("Stealth 模式已启用")

        return self._page

    async def _safe_goto(self, url: str, wait_until: str = "networkidle") -> None:
        """安全访问 URL，带随机延迟"""
        page = await self._launch()
        await self._random_delay()
        await page.goto(url, wait_until=wait_until)
        await asyncio.sleep(random.uniform(0.5, 1.5))

    async def _random_scroll(self) -> None:
        """随机滚动页面模拟人类行为"""
        if self._page:
            for _ in range(random.randint(2, 5)):
                await self._page.mouse.wheel(0, random.randint(300, 800))
                await asyncio.sleep(random.uniform(0.3, 1.0))

    async def close(self) -> None:
        """关闭浏览器"""
        await self.save_cookies()
        if self._context:
            await self._context.close()
            self._context = None
        if self._browser:
            await self._browser.close()
            self._browser = None
        if self._playwright:
            await self._playwright.stop()
            self._playwright = None
        self._page = None

    @abstractmethod
    async def search(self, keyword: str, city: Optional[str] = None, limit: int = 10) -> List[Dict[str, Any]]:
        """
        搜索岗位，子类实现

        Returns:
            岗位列表，每个岗位为 dict：{
                "title": str,
                "company": str,
                "location": str,
                "salary": str,
                "url": str,
                "raw_text": str,  # 完整 JD 文本
            }
        """
        raise NotImplementedError
