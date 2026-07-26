"""
CareerCraft Agent — Boss直聘爬虫

Playwright 实现，支持搜索关键词 + 城市，抓取列表页和详情页。
"""

from __future__ import annotations

import logging
from typing import Any, Dict, List, Optional, Tuple

from src.crawlers.base import BaseCrawler, CrawlerError

logger = logging.getLogger(__name__)


class BossZhipinCrawler(BaseCrawler):
    """
    Boss直聘爬虫

    使用示例：
        crawler = BossZhipinCrawler()
        jobs = await crawler.search("产品经理", city="深圳", limit=10)
        await crawler.close()
    """

    BASE_URL = "https://www.zhipin.com"

    # 城市代码映射（部分常用城市）
    CITY_CODES = {
        "北京": "101010100",
        "上海": "101020100",
        "广州": "101280100",
        "深圳": "101280600",
        "杭州": "101210100",
        "成都": "101270100",
        "武汉": "101200100",
        "西安": "101110100",
        "南京": "101190100",
        "苏州": "101190400",
    }

    def __init__(
        self,
        headless: bool = True,
        delay_range: Tuple[int, int] = (2, 5),
        cookie_path: Optional[str] = None,
    ) -> None:
        super().__init__(
            headless=headless,
            delay_range=delay_range,
            cookie_path=cookie_path,
        )

    async def search(
        self,
        keyword: str,
        city: Optional[str] = None,
        limit: int = 10,
    ) -> List[Dict[str, Any]]:
        """
        在 Boss直聘搜索岗位

        Args:
            keyword: 搜索关键词
            city: 城市名称（可选）
            limit: 最多返回岗位数量

        Returns:
            岗位列表
        """
        city_code = self.CITY_CODES.get(city, "")
        city_param = f"&city={city_code}" if city_code else ""
        search_url = (
            f"{self.BASE_URL}/web/geek/job?query={keyword}{city_param}&page=1"
        )

        try:
            page = await self._launch()
            await self._safe_goto(search_url)
            await self._random_scroll()

            # 等待岗位列表加载
            await page.wait_for_selector(".job-card-wrapper", timeout=15000)

            jobs: List[Dict[str, Any]] = []
            cards = await page.query_selector_all(".job-card-wrapper")

            for card in cards[:limit]:
                try:
                    job = await self._parse_card(card)
                    if job:
                        jobs.append(job)
                except Exception as e:
                    logger.warning("解析岗位卡片失败: %s", e)
                    continue

            logger.info("Boss直聘搜索完成: %s 关键词, %d 条结果", keyword, len(jobs))
            return jobs

        except Exception as e:
            raise CrawlerError(f"Boss直聘搜索失败: {e}")

    async def _parse_card(self, card: Any) -> Optional[Dict[str, Any]]:
        """解析单个岗位卡片"""
        try:
            title_el = await card.query_selector(".job-name")
            title = await title_el.inner_text() if title_el else ""

            company_el = await card.query_selector(".company-name")
            company = await company_el.inner_text() if company_el else ""

            location_el = await card.query_selector(".job-area")
            location = await location_el.inner_text() if location_el else ""

            salary_el = await card.query_selector(".salary")
            salary = await salary_el.inner_text() if salary_el else ""

            # 获取详情页链接
            link_el = await card.query_selector("a")
            href = await link_el.get_attribute("href") if link_el else ""
            url = f"{self.BASE_URL}{href}" if href and not href.startswith("http") else href

            return {
                "title": title.strip(),
                "company": company.strip(),
                "location": location.strip(),
                "salary": salary.strip(),
                "url": url,
                "raw_text": f"{title}\n{company}\n{location}\n{salary}",
                "source": "crawler_boss",
            }
        except Exception as e:
            logger.warning("解析卡片元素失败: %s", e)
            return None

    async def fetch_detail(self, url: str) -> str:
        """获取岗位详情页的 JD 文本"""
        try:
            page = await self._launch()
            await self._safe_goto(url)
            await self._random_scroll()

            # 等待 JD 内容加载
            await page.wait_for_selector(".job-sec-text", timeout=15000)

            jd_el = await page.query_selector(".job-sec-text")
            if jd_el:
                return await jd_el.inner_text()
            return ""
        except Exception as e:
            logger.warning("获取详情页失败: %s", e)
            return ""
