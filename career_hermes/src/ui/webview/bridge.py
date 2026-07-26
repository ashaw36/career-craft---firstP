"""
CareerCraft Agent — QWebChannel Python Bridge

通过 QWebChannel 向 JS 暴露后端 API，所有方法返回 JSON 字符串以便 JS 解析。
"""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, Optional

from PySide6.QtCore import QObject, QUrl, Slot
from PySide6.QtGui import QDesktopServices

from src.ui.webview.api_handler import CareerAPI

logger = logging.getLogger(__name__)


class CareerBridge(QObject):
    """
    QWebChannel 桥接对象

    JS 侧通过 window.pybridge 访问：
        window.pybridge.getExperiences((result) => { console.log(result); });
    """

    def __init__(self, parent: Optional[QObject] = None) -> None:
        super().__init__(parent)
        self._api = CareerAPI()

    def _ok(self, data: Any) -> str:
        return json.dumps({"success": True, "data": data}, ensure_ascii=False)

    def _err(self, message: str) -> str:
        return json.dumps({"success": False, "error": message}, ensure_ascii=False)

    # ─── 经历 ───

    @Slot(result=str)
    def getExperiences(self) -> str:
        try:
            data = self._api.get_experiences()
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getExperiences error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def saveExperience(self, data_json: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(data_json)
            result = self._api.save_experience(data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge saveExperience error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def deleteExperience(self, exp_id: str) -> str:
        try:
            result = self._api.delete_experience(exp_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge deleteExperience error: {e}")
            return self._err(str(e))

    # ——— 角色 ———

    @Slot(result=str)
    def getPersonas(self) -> str:
        try:
            data = self._api.get_personas()
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getPersonas error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def getPersonaById(self, persona_id: str) -> str:
        try:
            data = self._api.get_persona_by_id(persona_id)
            if data is None:
                return self._err("角色不存在")
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getPersonaById error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def createPersona(self, data_json: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(data_json)
            result = self._api.create_persona(data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge createPersona error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def updatePersona(self, persona_id: str, data_json: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(data_json)
            result = self._api.update_persona(persona_id, data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge updatePersona error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def deletePersona(self, persona_id: str) -> str:
        try:
            result = self._api.delete_persona(persona_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge deletePersona error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def getExperiencesWithFitScore(self, persona_id: str) -> str:
        try:
            result = self._api.get_experiences_with_fit_score(persona_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge getExperiencesWithFitScore error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def updateFitScore(self, json_str: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(json_str)
            result = self._api.update_fit_score(data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge updateFitScore error: {e}")
            return self._err(str(e))

    # ─── 简历 ───

    @Slot(str, str, result=str)
    def generateResume(self, persona_id: str, template: str = "modern") -> str:
        try:
            result = self._api.generate_resume(persona_id, template_name=template)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge generateResume error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def exportResumePDF(self, persona_id: str) -> str:
        try:
            result = self._api.export_resume_pdf(persona_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge exportResumePDF error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def chatRefineResume(self, persona_id: str, instruction: str) -> str:
        try:
            result = self._api.chat_refine_resume(persona_id, instruction)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge chatRefineResume error: {e}")
            return self._err(str(e))

    @Slot(result=str)
    def getSettings(self) -> str:
        try:
            data = self._api.get_settings()
            return json.dumps(data, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge getSettings error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def saveSettings(self, settings_json: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(settings_json)
            result = self._api.save_settings(data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge saveSettings error: {e}")
            return self._err(str(e))

    @Slot(result=str)
    def testLLMConnection(self) -> str:
        try:
            result = self._api.test_llm_connection()
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge testLLMConnection error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def importExperiences(self, format: str, content_json: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(content_json)
            result = self._api.import_experiences(format, data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge importExperiences error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def importFile(self, file_name: str, base64_content: str) -> str:
        """导入 PDF/Word 文件，base64 编码"""
        try:
            result = self._api.import_file(file_name, base64_content)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge importFile error: {e}")
            return self._err(str(e))

    # ─── 岗位匹配 ───

    @Slot(str, result=str)
    def parseJD(self, jd_text: str) -> str:
        try:
            result = self._api.parse_jd(jd_text)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge parseJD error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def matchJob(self, job_desc_id: str, persona_id: str) -> str:
        try:
            result = self._api.match_job(job_desc_id, persona_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge matchJob error: {e}")
            return self._err(str(e))

    @Slot(result=str)
    def listJobs(self) -> str:
        try:
            data = self._api.list_jobs()
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge listJobs error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def deleteJob(self, job_desc_id: str) -> str:
        try:
            result = self._api.delete_job(job_desc_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge deleteJob error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def getJobMatches(self, job_desc_id: str) -> str:
        try:
            result = self._api.get_job_matches(job_desc_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge getJobMatches error: {e}")
            return self._err(str(e))

    @Slot(str, str, result=str)
    def updateMatchStatus(self, match_id: str, status: str) -> str:
        try:
            result = self._api.update_match_status(match_id, status)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge updateMatchStatus error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def reframeResume(self, match_id: str) -> str:
        try:
            result = self._api.reframe_resume(match_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge reframeResume error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def getReframeResults(self, match_id: str) -> str:
        try:
            result = self._api.get_reframe_results(match_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge getReframeResults error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def updateReframe(self, json_str: str) -> str:
        try:
            data: Dict[str, Any] = json.loads(json_str)
            result = self._api.update_reframe(data)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge updateReframe error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def resetReframe(self, reframe_id: str) -> str:
        try:
            result = self._api.reset_reframe(reframe_id)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge resetReframe error: {e}")
            return self._err(str(e))

    # ─── 学习路径 ───

    @Slot(str, result=str)
    def getLearningPath(self, skill: str) -> str:
        try:
            data = self._api.get_learning_path(skill)
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getLearningPath error: {e}")
            return self._err(str(e))

    @Slot(result=str)
    def getLearningPathsBySource(self) -> str:
        try:
            data = self._api.get_learning_paths_by_source()
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getLearningPathsBySource error: {e}")
            return self._err(str(e))

    # ─── 技能图谱 ───

    @Slot(result=str)
    def getSkillGraph(self) -> str:
        try:
            result = self._api.get_skill_graph()
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge getSkillGraph error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def getSkillResources(self, skill_id: str) -> str:
        try:
            data = self._api.get_skill_resources(skill_id)
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getSkillResources error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def searchSkills(self, query: str) -> str:
        try:
            result = self._api.search_skills(query)
            return json.dumps(result, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge searchSkills error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def openExternalUrl(self, url: str) -> str:
        try:
            opened = QDesktopServices.openUrl(QUrl(url))
            return json.dumps({"success": bool(opened)}, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge openExternalUrl error: {e}")
            return self._err(str(e))

    @Slot(str, result=str)
    def copyToClipboard(self, text: str) -> str:
        try:
            from PySide6.QtWidgets import QApplication
            app = QApplication.instance()
            if app:
                app.clipboard().setText(text)
            return json.dumps({"success": True}, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Bridge copyToClipboard error: {e}")
            return self._err(str(e))

    # ─── 统计 ───

    @Slot(result=str)
    def getStats(self) -> str:
        try:
            data = self._api.get_stats()
            return self._ok(data)
        except Exception as e:
            logger.error(f"Bridge getStats error: {e}")
            return self._err(str(e))
