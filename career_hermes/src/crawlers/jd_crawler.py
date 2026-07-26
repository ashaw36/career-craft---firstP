"""
CareerCraft Agent — JD 爬虫存根

由于环境限制（无真实 API Key / 反爬虫策略），当前仅提供模拟爬虫接口。
后续可替换为 Playwright 真实爬取实现。
"""

from __future__ import annotations

import logging
from typing import Dict, List, Optional

logger = logging.getLogger(__name__)


class JDCrawlerError(Exception):
    """JD 爬虫异常"""


class JDCrawler:
    """
    岗位描述爬虫

    使用示例：
        crawler = JDCrawler()
        jd_text = await crawler.fetch_jd("https://example.com/job/123")
    """

    # 模拟 JD 数据库
    _MOCK_JDS: Dict[str, str] = {
        "default": """
高级产品经理

公司：示例科技
地点：深圳
年限：3-5年

岗位描述：
负责 B 端 AI 产品的规划与落地，协调研发、运营、采购多部门配合。

要求：
- 4年以上产品经理经验
- 熟悉 Python、SQL、产品规划
- 有供应链/企业服务背景优先
- 微服务架构了解为加分项
""",
        "tencent": """
后端开发工程师

公司：腾讯
地点：广州
年限：5年以上

岗位描述：
负责核心后端服务设计与开发，高并发系统架构。

要求：
- 精通 Go / Java / C++
- 熟悉 Kubernetes、Docker、gRPC
- 有大规模分布式系统经验
- 微服务架构必备
""",
    }

    def __init__(self) -> None:
        self._headless: bool = True
        self._stealth: bool = True

    async def fetch_jd(self, url: str) -> str:
        """
        爬取岗位描述文本

        Args:
            url: 岗位详情页 URL

        Returns:
            岗位描述文本
        """
        logger.info("[MOCK] 爬取 JD: %s", url)
        # 根据 URL 关键词返回不同模拟数据
        url_lower = url.lower()
        if "tencent" in url_lower or "腾讯" in url_lower:
            return self._MOCK_JDS["tencent"].strip()
        return self._MOCK_JDS["default"].strip()

    async def search_jobs(self, keyword: str, city: Optional[str] = None) -> List[Dict[str, str]]:
        """
        搜索岗位（模拟）

        Returns:
            岗位列表，每项包含 title、company、url 等
        """
        logger.info("[MOCK] 搜索岗位: keyword=%s, city=%s", keyword, city)
        return [
            {
                "title": f"高级{keyword}",
                "company": "示例科技",
                "city": city or "深圳",
                "url": "https://example.com/job/1",
                "salary": "30-50K",
            },
            {
                "title": f"资深{keyword}",
                "company": "示例科技",
                "city": city or "广州",
                "url": "https://example.com/job/2",
                "salary": "40-60K",
            },
        ]
