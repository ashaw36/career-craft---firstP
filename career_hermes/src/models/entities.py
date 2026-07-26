"""
CareerCraft Agent — 核心数据模型定义

SQLAlchemy 2.0 ORM 实体，支持异步操作。
严格遵循 PRD v1.0 第 5 节数据模型设计。
Sprint 4 扩充 JobDesc / JobMatch 字段以支持 JD 解析与匹配服务。
"""

from __future__ import annotations

import uuid
from datetime import date, datetime
from typing import List, Optional

from sqlalchemy import Float, JSON, Boolean, Date, DateTime, ForeignKey, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column, relationship
from sqlalchemy.sql import func

from src.models.database import Base


def generate_uuid() -> str:
    return str(uuid.uuid4())


class Experience(Base):
    __tablename__ = "experiences"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    user_id: Mapped[str] = mapped_column(String(36), nullable=False, default="default")
    type: Mapped[str] = mapped_column(
        String(20), nullable=False
    )  # work, project, education, certification
    title: Mapped[str] = mapped_column(String(200), nullable=False)
    organization: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    start_date: Mapped[Optional[date]] = mapped_column(Date, nullable=True)
    end_date: Mapped[Optional[date]] = mapped_column(Date, nullable=True)
    raw_description: Mapped[str] = mapped_column(Text, nullable=False)
    structured_achievements: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    skills_demonstrated: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    metrics: Mapped[Optional[List[dict]]] = mapped_column(JSON, nullable=True)
    status: Mapped[str] = mapped_column(
        String(20), nullable=False, default="draft"
    )  # draft, confirmed, discarded, archived
    version: Mapped[int] = mapped_column(Integer, nullable=False, default=1)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    role_weights: Mapped[List["RoleExperienceWeight"]] = relationship(
        "RoleExperienceWeight", back_populates="experience", cascade="all, delete-orphan"
    )

    def __repr__(self) -> str:
        return f"<Experience(id={self.id}, title={self.title}, type={self.type})>"


class Persona(Base):
    __tablename__ = "personas"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    user_id: Mapped[str] = mapped_column(String(36), nullable=False, default="default")
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    is_default: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    identity_statement: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    career_narrative: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    tone_style: Mapped[Optional[str]] = mapped_column(
        String(50), nullable=True
    )  # data_driven, business_insight, technical_deep
    capability_weights: Mapped[Optional[dict]] = mapped_column(JSON, nullable=True)
    target_job_profiles: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    max_experiences: Mapped[int] = mapped_column(Integer, nullable=False, default=5)
    preferred_model: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    role_weights: Mapped[List["RoleExperienceWeight"]] = relationship(
        "RoleExperienceWeight", back_populates="persona", cascade="all, delete-orphan"
    )
    job_matches: Mapped[List["JobMatch"]] = relationship(
        "JobMatch", back_populates="persona", cascade="all, delete-orphan"
    )
    learning_paths: Mapped[List["LearningPath"]] = relationship(
        "LearningPath", back_populates="persona", cascade="all, delete-orphan"
    )

    def __repr__(self) -> str:
        return f"<Persona(id={self.id}, name={self.name})>"


class RoleExperienceWeight(Base):
    __tablename__ = "role_experience_weights"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    persona_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("personas.id", ondelete="CASCADE"), nullable=False
    )
    experience_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("experiences.id", ondelete="CASCADE"), nullable=False
    )
    relevance_score: Mapped[float] = mapped_column(
        Float, nullable=False, default=0.0
    )  # 0.0 ~ 1.0
    reframed_summary: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    highlighted_skills: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    user_overridden: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    persona: Mapped["Persona"] = relationship("Persona", back_populates="role_weights")
    experience: Mapped["Experience"] = relationship("Experience", back_populates="role_weights")

    def __repr__(self) -> str:
        return f"<RoleExperienceWeight(p={self.persona_id}, e={self.experience_id}, score={self.relevance_score})>"


class SkillNode(Base):
    __tablename__ = "skill_nodes"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    name: Mapped[str] = mapped_column(String(100), nullable=False, unique=True)
    category: Mapped[Optional[str]] = mapped_column(
        String(50), nullable=True
    )  # technical, business, soft_skill, domain, tool
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    parent_id: Mapped[Optional[str]] = mapped_column(
        String(36), ForeignKey("skill_nodes.id", ondelete="SET NULL"), nullable=True
    )
    aliases: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    resources: Mapped[Optional[List[dict]]] = mapped_column(JSON, nullable=True)
    vector_embedding: Mapped[Optional[bytes]] = mapped_column(String(36), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    # 自引用关系
    parent: Mapped[Optional["SkillNode"]] = relationship(
        "SkillNode", remote_side="SkillNode.id", backref="children"
    )

    def __repr__(self) -> str:
        return f"<SkillNode(id={self.id}, name={self.name})>"


class JobDesc(Base):
    __tablename__ = "job_descs"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    raw_text: Mapped[str] = mapped_column(Text, nullable=False)
    title: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    company: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    # Sprint 4 新增字段
    years_of_experience: Mapped[Optional[str]] = mapped_column(String(50), nullable=True)
    salary_range: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    location: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    job_type: Mapped[Optional[str]] = mapped_column(
        String(50), nullable=True
    )  # full_time, part_time, contract, intern
    education_requirement: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    responsibilities: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    benefits: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    url: Mapped[Optional[str]] = mapped_column(String(500), nullable=True)
    parsed_skills: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    source: Mapped[Optional[str]] = mapped_column(
        String(50), nullable=True
    )  # manual, crawler_boss, crawler_liepin
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    matches: Mapped[List["JobMatch"]] = relationship(
        "JobMatch", back_populates="job_desc", cascade="all, delete-orphan"
    )

    def __repr__(self) -> str:
        return f"<JobDesc(id={self.id}, title={self.title})>"


class JobMatch(Base):
    __tablename__ = "job_matches"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    persona_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("personas.id", ondelete="CASCADE"), nullable=False
    )
    job_desc_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("job_descs.id", ondelete="CASCADE"), nullable=False
    )
    match_score: Mapped[int] = mapped_column(Integer, nullable=False, default=0)  # 0 ~ 100
    matched_skills: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    missing_skills: Mapped[Optional[List[str]]] = mapped_column(JSON, nullable=True)
    # Sprint 4 新增字段
    score_breakdown: Mapped[Optional[dict]] = mapped_column(JSON, nullable=True)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    ai_analysis: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    tracking_status: Mapped[str] = mapped_column(
        String(20), nullable=False, default="new"
    )  # new, interested, applied, interviewing, offered, rejected, ghosted, accepted, declined
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    persona: Mapped["Persona"] = relationship("Persona", back_populates="job_matches")
    job_desc: Mapped["JobDesc"] = relationship("JobDesc", back_populates="matches")

    def __repr__(self) -> str:
        return f"<JobMatch(id={self.id}, score={self.match_score}, status={self.tracking_status})>"


class JobMatchExperienceReframe(Base):
    __tablename__ = "job_match_experience_reframes"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    job_match_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("job_matches.id", ondelete="CASCADE"), nullable=False
    )
    experience_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("experiences.id", ondelete="CASCADE"), nullable=False
    )
    original_summary: Mapped[str] = mapped_column(Text, nullable=False)
    reframed_summary: Mapped[str] = mapped_column(Text, nullable=False)
    reframing_strategy: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    # 关系
    job_match: Mapped["JobMatch"] = relationship("JobMatch", backref="experience_reframes")
    experience: Mapped["Experience"] = relationship("Experience")

    def __repr__(self) -> str:
        return f"<JobMatchExperienceReframe(m={self.job_match_id}, e={self.experience_id})>"


class LearningPath(Base):
    __tablename__ = "learning_paths"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    persona_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("personas.id", ondelete="CASCADE"), nullable=False
    )
    target_gap: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    items: Mapped[Optional[List[dict]]] = mapped_column(JSON, nullable=True)
    source_type: Mapped[Optional[str]] = mapped_column(
        String(20), nullable=True, default="manual"
    )
    status: Mapped[str] = mapped_column(
        String(20), nullable=False, default="active"
    )  # active, completed, archived
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )

    # 关系
    persona: Mapped["Persona"] = relationship("Persona", back_populates="learning_paths")

    def __repr__(self) -> str:
        return f"<LearningPath(id={self.id}, target={self.target_gap})>"


class UploadedFile(Base):
    __tablename__ = "uploaded_files"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=generate_uuid)
    user_id: Mapped[str] = mapped_column(String(36), nullable=False, default="default")
    filename: Mapped[str] = mapped_column(String(255), nullable=False)
    file_type: Mapped[str] = mapped_column(String(50), nullable=False)
    content_preview: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    extracted_count: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    status: Mapped[str] = mapped_column(
        String(20), nullable=False, default="processed"
    )  # processed, failed
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
