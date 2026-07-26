"""
CareerCraft Agent — 异步数据库引擎与会话管理

SQLAlchemy 2.0 async + aiosqlite，启用 WAL 模式保证性能和崩溃恢复能力。
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import AsyncGenerator, Dict, List, Optional

from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import declarative_base

logger = logging.getLogger(__name__)

# 数据库文件路径：~/.careercraft/career.db
DEFAULT_DB_DIR = Path.home() / ".careercraft"
DEFAULT_DB_PATH = DEFAULT_DB_DIR / "career.db"

# 确保目录存在
DEFAULT_DB_DIR.mkdir(parents=True, exist_ok=True)

# 异步引擎，启用 WAL 模式
engine = create_async_engine(
    f"sqlite+aiosqlite:///{DEFAULT_DB_PATH}",
    echo=False,
    connect_args={"check_same_thread": False},
    # WAL 模式通过 PRAGMA 在初始化时设置
)

# 异步会话工厂
AsyncSessionLocal = async_sessionmaker(
    bind=engine,
    class_=AsyncSession,
    expire_on_commit=False,
    autoflush=False,
)

# 声明性基类
Base = declarative_base()

# Schema 修复配置：表名 -> 缺失时需添加的列
# 格式：{"column_name": "SQL_TYPE [DEFAULT ...]"}
SCHEMA_MIGRATIONS: Dict[str, Dict[str, str]] = {
    "learning_paths": {
        "source_type": "VARCHAR(20) DEFAULT 'manual'",
    },
}


async def _migrate_schema() -> None:
    """检测并修复 SQLite schema 缺失的列。
    用于应用升级时自动补全旧数据库中新增字段。"""
    from sqlalchemy import text

    async with engine.begin() as conn:
        for table_name, columns in SCHEMA_MIGRATIONS.items():
            # 查询表是否存在
            result = await conn.execute(
                text(f"SELECT name FROM sqlite_master WHERE type='table' AND name='{table_name}'")
            )
            if result.scalar() is None:
                continue  # 表尚未创建，等 create_all 处理

            # 查询现有列
            result = await conn.execute(text(f"PRAGMA table_info({table_name})"))
            existing_cols = {row[1] for row in result.all()}

            for col_name, col_def in columns.items():
                if col_name not in existing_cols:
                    logger.info(f"Schema 修复: 表 {table_name} 添加列 {col_name}")
                    await conn.execute(
                        text(f"ALTER TABLE {table_name} ADD COLUMN {col_name} {col_def}")
                    )


async def init_db() -> None:
    """初始化数据库：创建所有表结构并执行 schema 修复。
    应用启动时调用一次。"""
    async with engine.begin() as conn:
        # 启用 WAL 模式以支持读写并发
        await conn.exec_driver_sql("PRAGMA journal_mode=WAL")
        await conn.exec_driver_sql("PRAGMA foreign_keys=ON")
        await conn.run_sync(Base.metadata.create_all)
    # 在 create_all 之后执行 schema 修复，处理旧数据库缺失列
    await _migrate_schema()


async def get_session() -> AsyncGenerator[AsyncSession, None]:
    """异步会话依赖注入器。
    用于 FastAPI 或其他异步框架的会话管理。"""
    async with AsyncSessionLocal() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise
        finally:
            await session.close()


async def close_db() -> None:
    """关闭数据库引擎，释放连接池。
    应用退出时调用。"""
    await engine.dispose()
