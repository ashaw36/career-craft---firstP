//! Resume commands (CC-FR-006/007/018). AI suggestions are never applied unless
//! the caller explicitly sends `confirm: true`; generated versions are immutable.

use crate::{
    application::{
        ports::{ExperienceRepository, PersonaRepository},
        resumes::{choose_template, select_work_experiences},
    },
    domain::{
        entities::ExperienceType,
        resume::{
            diff, ResumeEntry, ResumeHeader, ResumeRenderData, ResumeTemplate, ResumeVersion,
        },
    },
    error::{AppError, Envelope},
    infra::{
        documents::{DocumentExporter, MarkdownExporter, SystemFontPdfExporter},
        repositories::{
            SqliteExperienceRepository, SqlitePersonaRepository, SqliteResumeVersionRepository,
        },
    },
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
struct TuneProposal {
    base_id: String,
    summary: String,
    content_hash: String,
    expires: Instant,
    used: bool,
}
static PROPOSALS: OnceLock<Mutex<HashMap<String, TuneProposal>>> = OnceLock::new();
fn issue_proposal(base_id: &str, summary: String) -> (String, String) {
    let proposal_id = id();
    let hash = content_hash(summary.as_bytes());
    PROPOSALS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            proposal_id.clone(),
            TuneProposal {
                base_id: base_id.into(),
                summary,
                content_hash: hash.clone(),
                expires: Instant::now() + Duration::from_secs(600),
                used: false,
            },
        );
    (proposal_id, hash)
}
fn consume_proposal(id: &str, base: &str, hash: &str) -> Result<String, AppError> {
    let mut proposals = PROPOSALS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let proposal = proposals
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound("tuning proposal".into()))?;
    if proposal.used {
        return Err(AppError::Conflict("tuning proposal already used".into()));
    }
    if proposal.expires <= Instant::now() {
        return Err(AppError::Conflict("tuning proposal expired".into()));
    }
    if proposal.base_id != base || proposal.content_hash != hash {
        return Err(AppError::Conflict(
            "tuning proposal does not match base or content hash".into(),
        ));
    }
    proposal.used = true;
    Ok(proposal.summary.clone())
}
const TUNING_PROMPT_VERSION: &str = "resume-tuning-v2";
fn strip_json_fence(value: &str) -> &str {
    let trimmed = value.trim();
    if let Some(rest) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
    {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}
fn format_refinement_preview(payload: &Value, base: &ResumeRenderData) -> String {
    let mut out = String::new();
    if let Some(summary) = payload.get("summary").and_then(Value::as_str) {
        out.push_str("【职业摘要】\n");
        out.push_str(summary.trim());
        out.push_str("\n\n");
    } else if let Some(summary) = &base.summary {
        out.push_str("【职业摘要】\n");
        out.push_str(summary);
        out.push_str("\n\n");
    }
    let updates = payload
        .get("experiences")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for entry in &base.experience {
        out.push_str(&format!("【{}】\n", entry.title));
        if let Some(org) = &entry.organization {
            out.push_str(org);
            out.push('\n');
        }
        let patch = updates.iter().find(|item| {
            item.get("sourceExperienceId").and_then(Value::as_str)
                == Some(entry.source_experience_id.as_str())
        });
        let achievements = patch
            .and_then(|item| item.get("achievements"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|rows| !rows.is_empty())
            .unwrap_or_else(|| entry.achievements.clone());
        for line in achievements {
            out.push_str("- ");
            out.push_str(&line);
            out.push('\n');
        }
        out.push('\n');
    }
    out.trim().to_owned()
}
fn apply_experience_refinement(
    data: &mut ResumeRenderData,
    payload: &Value,
) -> Result<(), AppError> {
    if let Some(summary) = payload.get("summary").and_then(Value::as_str) {
        let summary = summary.trim();
        data.summary = if summary.is_empty() {
            None
        } else {
            Some(summary.into())
        };
    }
    let Some(items) = payload.get("experiences").and_then(Value::as_array) else {
        return Ok(());
    };
    for item in items {
        let id = item
            .get("sourceExperienceId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if id.is_empty() {
            continue;
        }
        let Some(entry) = data
            .experience
            .iter_mut()
            .find(|entry| entry.source_experience_id == id)
        else {
            continue;
        };
        if let Some(summary) = item.get("summary").and_then(Value::as_str) {
            let summary = summary.trim();
            entry.summary = if summary.is_empty() {
                None
            } else {
                Some(summary.into())
            };
        }
        if let Some(rows) = item.get("achievements").and_then(Value::as_array) {
            let achievements = rows
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !achievements.is_empty() {
                entry.achievements = achievements;
            }
        }
    }
    data.normalize();
    data.validate().map_err(resume_error)?;
    Ok(())
}
fn parse_refinement_payload(raw: &str, base: &ResumeRenderData) -> Result<Value, AppError> {
    let json = strip_json_fence(raw);
    let value: Value = serde_json::from_str(json).map_err(|_| {
        AppError::Unavailable("LLM refinement must be valid JSON for experiences".into())
    })?;
    if !value.is_object() {
        return Err(AppError::Unavailable(
            "LLM refinement JSON must be an object".into(),
        ));
    }
    if let Some(items) = value.get("experiences").and_then(Value::as_array) {
        for item in items {
            let id = item
                .get("sourceExperienceId")
                .and_then(Value::as_str)
                .unwrap_or("");
            if id.is_empty()
                || !base
                    .experience
                    .iter()
                    .any(|entry| entry.source_experience_id == id)
            {
                return Err(AppError::Unavailable(
                    "LLM refinement referenced an unknown experience".into(),
                ));
            }
        }
    }
    Ok(value)
}
fn parse_tuning_instruction_type(value: &str) -> Result<&'static str, AppError> {
    match value {
        "leadership" => Ok("leadership"),
        "metrics" => Ok("metrics"),
        "concise" => Ok("concise"),
        "technical_depth" => Ok("technical_depth"),
        "job_alignment" => Ok("job_alignment"),
        "general" => Ok("general"),
        _ => Err(AppError::Validation("invalid instructionType".into())),
    }
}
fn tuning_guidance(kind: &str) -> &'static str {
    match kind {
        "leadership" => {
            "突出已有事实中的主导权、跨团队协作、带队与决策影响；不要编造新事实。"
        }
        "metrics" => "优先保留已有可量化结果与数字；禁止编造或外推任何指标。",
        "concise" => "删减重复表达并压缩措辞，同时保留全部实质事实。",
        "technical_depth" => {
            "突出已有技术复杂度、架构、方法与工程决策；不要新增未出现过的技能。"
        }
        "job_alignment" => {
            "围绕目标岗位重排并强调已有证据；不要声称不具备的资质。"
        }
        "general" => "Improve clarity and emphasis in English while preserving all facts.",
        _ => "在保留全部事实的前提下提升清晰度与侧重点。",
    }
}
fn id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_nanos())
        .unwrap_or(0);
    format!(
        "resume-{}-{nanos}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}
fn connection() -> Result<Connection, AppError> {
    crate::infra::db::open_runtime_connection()
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| AppError::Validation(format!("{key} is required")))
}
fn outcome(value: Result<Value, AppError>) -> Envelope<Value> {
    value.into()
}
fn resume_error(error: crate::domain::resume::ResumeError) -> AppError {
    AppError::Validation(format!("{error:?}"))
}
struct BuiltResume {
    data: ResumeRenderData,
    selected_experience_ids: Vec<String>,
    fit_scores: Vec<(String, f64)>,
}
fn content_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3)
    }
    format!("fnv1a64:{hash:016x}")
}

fn build(persona_id: &str) -> Result<BuiltResume, AppError> {
    let connection = connection()?;
    let personas = SqlitePersonaRepository::new(&connection);
    let persona = personas
        .get(persona_id)
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("persona".into()))?;
    let experience_repo = SqliteExperienceRepository::new(&connection);
    let experiences = experience_repo
        .list_confirmed(&persona.user_id)
        .map_err(AppError::from)?;
    let existing = personas.get_weights(persona_id).map_err(AppError::from)?;
    let weights = experiences
        .iter()
        .map(|exp| {
            existing
                .iter()
                .find(|w| w.experience_id == exp.id && w.user_overridden)
                .cloned()
                .or_else(|| {
                    existing
                        .iter()
                        .find(|w| w.experience_id == exp.id)
                        .cloned()
                })
                .unwrap_or_else(|| crate::domain::entities::RoleExperienceWeight {
                    id: String::new(),
                    persona_id: persona.id.clone(),
                    experience_id: exp.id.clone(),
                    relevance_score: crate::domain::fit_score::calculate(&persona, exp),
                    reframed_summary: None,
                    highlighted_skills: vec![],
                    user_overridden: false,
                })
        })
        .collect::<Vec<_>>();
    let score_of = |id: &str| {
        weights
            .iter()
            .find(|w| w.experience_id == id)
            .map(|w| w.relevance_score)
            .unwrap_or(0.0)
    };
    let selected_work =
        select_work_experiences(&experiences, &weights, persona.max_experiences as usize);
    let mut selected = selected_work;
    let mut selected_education = experiences
        .into_iter()
        .filter(|item| {
            matches!(
                item.kind,
                ExperienceType::Education | ExperienceType::Certification
            )
        })
        .collect::<Vec<_>>();
    selected_education.sort_by(|a, b| {
        b.start_date
            .cmp(&a.start_date)
            .then_with(|| a.id.cmp(&b.id))
    });
    selected.extend(selected_education);
    let selected_experience_ids = selected.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
    let fit_scores = weights
        .iter()
        .map(|w| (w.experience_id.clone(), w.relevance_score * 100.0))
        .collect::<Vec<_>>();
    let mut skills = Vec::new();
    let mut work = Vec::new();
    let mut education = Vec::new();
    for item in selected {
        let score = score_of(&item.id);
        skills.extend(item.skills_demonstrated.clone());
        let period = match (&item.start_date, &item.end_date) {
            (Some(start), Some(end)) => Some(format!("{start} – {end}")),
            (Some(start), None) => Some(format!("{start} – Present")),
            _ => None,
        };
        let achievements = if matches!(
            item.kind,
            ExperienceType::Education | ExperienceType::Certification
        ) {
            item.structured_achievements.clone()
        } else {
            crate::application::resumes::allocate_achievements_by_weight(
                &item.structured_achievements,
                score,
            )
        };
        let summary = if matches!(
            item.kind,
            ExperienceType::Education | ExperienceType::Certification
        ) {
            if item.raw_description.trim().is_empty() {
                None
            } else {
                Some(item.raw_description)
            }
        } else if score >= 0.5 && !item.raw_description.trim().is_empty() {
            Some(item.raw_description)
        } else if achievements.is_empty() && !item.raw_description.trim().is_empty() {
            Some(item.raw_description.chars().take(120).collect())
        } else {
            None
        };
        let entry = ResumeEntry {
            source_experience_id: item.id,
            title: item.title,
            organization: item.organization,
            period,
            summary,
            achievements,
            skills: item.skills_demonstrated,
        };
        if matches!(
            item.kind,
            ExperienceType::Education | ExperienceType::Certification
        ) {
            education.push(entry);
        } else {
            work.push(entry);
        }
    }
    let mut data = ResumeRenderData {
        header: ResumeHeader {
            full_name: persona.name,
            headline: persona.identity_statement.unwrap_or_default(),
            ..Default::default()
        },
        summary: persona.career_narrative,
        experience: work,
        education,
        skills,
        ..Default::default()
    };
    data.normalize();
    data.validate().map_err(resume_error)?;
    Ok(BuiltResume {
        data,
        selected_experience_ids,
        fit_scores,
    })
}

fn render_version(version: &ResumeVersion) -> Result<Value, AppError> {
    let markdown = MarkdownExporter
        .export(version.template, &version.data)
        .map_err(|error| AppError::Unavailable(format!("resume render failed: {error:?}")))?;
    let difference = if let Some(parent_id) = &version.parent_id {
        let c = connection()?;
        SqliteResumeVersionRepository::new(&c).get(parent_id)?.map(|parent|{let d=diff(&parent,version);json!({"headerChanged":d.header_changed,"summaryChanged":d.summary_changed,"experienceAdded":d.experience_added,"experienceRemoved":d.experience_removed,"experienceChanged":d.experience_changed,"skillsAdded":d.skills_added,"skillsRemoved":d.skills_removed})})
    } else {
        None
    };
    let hash = content_hash(&markdown);
    Ok(
        json!({"versionId":version.id,"personaId":version.persona_id,"revision":version.revision,"parentId":version.parent_id,"diffFromParent":difference,
        "template":version.template.id(),"markdown":String::from_utf8(markdown).map_err(|_|AppError::Internal)?,"contentHash":hash}),
    )
}
fn save(version: ResumeVersion) -> Result<ResumeVersion, AppError> {
    let c = connection()?;
    SqliteResumeVersionRepository::new(&c).save(&version)
}
fn latest(persona_id: &str) -> Result<Option<ResumeVersion>, AppError> {
    let c = connection()?;
    SqliteResumeVersionRepository::new(&c).latest(persona_id)
}
fn get_version(id: &str) -> Result<Option<ResumeVersion>, AppError> {
    let c = connection()?;
    SqliteResumeVersionRepository::new(&c).get(id)
}

pub fn generate_resume(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let template = choose_template(
            payload
                .get("template")
                .and_then(Value::as_str)
                .unwrap_or("modern"),
        )
        .map_err(resume_error)?;
        let built = build(persona_id)?;
        let version = save(ResumeVersion {
            id: id(),
            persona_id: persona_id.into(),
            label: "Generated resume".into(),
            template,
            revision: 0,
            data: built.data,
            parent_id: None,
            created_at: "local".into(),
        })?;
        render_version(&version)
    })())
}

pub fn preview_resume(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let template = choose_template(
            payload
                .get("template")
                .and_then(Value::as_str)
                .unwrap_or("modern"),
        )
        .map_err(resume_error)?;
        let built = build(persona_id)?;
        let markdown = MarkdownExporter
            .export(template, &built.data)
            .map_err(|e| AppError::Unavailable(format!("resume render failed: {e:?}")))?;
        let hash = content_hash(&markdown);
        Ok(
            json!({"personaId":persona_id,"template":template.id(),"markdown":String::from_utf8(markdown).map_err(|_|AppError::Internal)?,"contentHash":hash,"selectedExperienceIds":built.selected_experience_ids,"fitScores":built.fit_scores.into_iter().map(|(experience_id,score)|json!({"experienceId":experience_id,"score":score})).collect::<Vec<_>>(),"warnings":[]}),
        )
    })())
}

pub fn chat_refine_resume(payload: Option<Value>) -> Envelope<Value> {
    chat_refine_resume_with_cancel(payload, &crate::infra::llm::CancellationToken::default())
}
pub(crate) fn chat_refine_resume_with_cancel(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
) -> Envelope<Value> {
    chat_refine_resume_with_generator(payload, cancel, |persona, prompt, cancel| {
        super::generate_for_persona_with_tokens(persona, prompt, 4096, cancel)
    })
}
fn chat_refine_resume_with_generator<F>(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
    generator: F,
) -> Envelope<Value>
where
    F: Fn(Option<&str>, String, &crate::infra::llm::CancellationToken) -> Result<String, AppError>,
{
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let instruction = required(&payload, "instruction")?;
        let base = latest(persona_id)?
            .ok_or_else(|| AppError::NotFound("generate a resume before refining it".into()))?;
        if matches!(
            instruction.trim().to_lowercase().as_str(),
            "undo" | "restore" | "撤销" | "恢复"
        ) {
            let target = base
                .parent_id
                .as_ref()
                .and_then(|parent| get_version(parent).ok().flatten())
                .ok_or_else(|| AppError::NotFound("no previous resume version".into()))?;
            let restored = save(ResumeVersion {
                id: id(),
                revision: 0,
                parent_id: Some(base.id),
                label: "Restored resume".into(),
                created_at: "local".into(),
                ..target
            })?;
            return render_version(&restored);
        }
        if matches!(instruction.trim().to_lowercase().as_str(), "redo" | "重做") {
            let target = base
                .parent_id
                .as_ref()
                .and_then(|parent| get_version(parent).ok().flatten())
                .ok_or_else(|| AppError::NotFound("no resume version to redo".into()))?;
            let redone = save(ResumeVersion {
                id: id(),
                revision: 0,
                parent_id: Some(base.id),
                label: "Redone resume".into(),
                created_at: "local".into(),
                ..target
            })?;
            return render_version(&redone);
        }
        let confirm = payload
            .get("confirm")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if confirm {
            let base_version_id = required(&payload, "baseVersionId")?;
            if base.id != base_version_id {
                return Err(AppError::Conflict(
                    "resume changed after refinement preview; generate a new preview".into(),
                ));
            }
            let stored = consume_proposal(
                required(&payload, "proposalId")?,
                base_version_id,
                required(&payload, "contentHash")?,
            )?;
            let refinement = parse_refinement_payload(&stored, &base.data)?;
            let mut data = base.data.clone();
            apply_experience_refinement(&mut data, &refinement)?;
            let tuned = save(ResumeVersion {
                id: id(),
                persona_id: base.persona_id,
                label: "Refined resume".into(),
                template: base.template,
                revision: 0,
                data,
                parent_id: Some(base.id),
                created_at: "local".into(),
            })?;
            return render_version(&tuned);
        }
        let experience_catalog = base
            .data
            .experience
            .iter()
            .map(|entry| {
                json!({
                    "sourceExperienceId": entry.source_experience_id,
                    "title": entry.title,
                    "organization": entry.organization,
                    "achievements": entry.achievements,
                    "summary": entry.summary,
                })
            })
            .collect::<Vec<_>>();
        let instruction_type =
            parse_tuning_instruction_type(required(&payload, "instructionType")?)?;
        let guidance = tuning_guidance(instruction_type);
        let prompt = format!(
            "Prompt-Version:{TUNING_PROMPT_VERSION}\nInstruction-Type:{instruction_type}\nType-Guidance:{guidance}\n你只改写简历经历要点的语气与侧重点。\n只返回一个 JSON 对象，键为：summary (string|null)、experiences (数组，元素为 {{sourceExperienceId, achievements:string[], summary?:string|null}})。\n规则：保持 sourceExperienceId 不变；不编造事实、数字、技能、公司、职位或日期；只改写已有 achievements；无变化的经历可省略；不要 markdown 代码块。\n语言：除 instructionType=general 外，summary 与 achievements 必须使用中文输出。\n当前经历 JSON：\n{}\n\n用户要求：\n{instruction}",
            serde_json::to_string(&experience_catalog).unwrap_or_else(|_| "[]".into())
        );
        let raw = generator(Some(persona_id), prompt, cancel)?;
        if raw.trim().is_empty() {
            return Err(AppError::Unavailable(
                "LLM returned an empty refinement".into(),
            ));
        }
        let refinement = parse_refinement_payload(&raw, &base.data)?;
        let preview_text = format_refinement_preview(&refinement, &base.data);
        if preview_text.trim().is_empty() {
            return Err(AppError::Unavailable(
                "LLM refinement produced no experience updates".into(),
            ));
        }
        let stored = serde_json::to_string(&refinement).map_err(|_| AppError::Internal)?;
        let (proposal_id, hash) = issue_proposal(&base.id, stored);
        Ok(
            json!({"requiresConfirmation":true,"instruction":instruction,"instructionType":instruction_type,"baseVersionId":base.id,"refinedSummary":preview_text,"refinement":refinement,"proposalId":proposal_id,"contentHash":hash,"promptVersion":TUNING_PROMPT_VERSION,"cachePolicy":"no-store"}),
        )
    })())
}

pub fn list_resume_versions(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let c = connection()?;
        let repo = SqliteResumeVersionRepository::new(&c);
        let items = repo
            .list(persona_id)?
            .iter()
            .map(render_version)
            .collect::<Result<Vec<_>, _>>()?;
        let count = items.len();
        Ok(json!({"items":items,"count":count}))
    })())
}

pub fn diff_resume_versions(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let left = get_version(required(&payload, "leftVersionId")?)?
            .ok_or_else(|| AppError::NotFound("left resume version".into()))?;
        let right = get_version(required(&payload, "rightVersionId")?)?
            .ok_or_else(|| AppError::NotFound("right resume version".into()))?;
        if left.persona_id != right.persona_id {
            return Err(AppError::Validation(
                "resume versions must belong to the same persona".into(),
            ));
        }
        let d = diff(&left, &right);
        Ok(
            json!({"leftVersionId":left.id,"rightVersionId":right.id,"headerChanged":d.header_changed,"summaryChanged":d.summary_changed,"experienceAdded":d.experience_added,"experienceRemoved":d.experience_removed,"experienceChanged":d.experience_changed,"skillsAdded":d.skills_added,"skillsRemoved":d.skills_removed}),
        )
    })())
}

pub fn restore_resume_version(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let source = get_version(required(&payload, "versionId")?)?
            .ok_or_else(|| AppError::NotFound("resume version".into()))?;
        if source.persona_id != persona_id {
            return Err(AppError::Validation(
                "resume version does not belong to persona".into(),
            ));
        }
        let parent = latest(&source.persona_id)?.map(|v| v.id);
        let restored = save(ResumeVersion {
            id: id(),
            persona_id: source.persona_id,
            label: format!("{} (restored)", source.label),
            template: source.template,
            revision: 0,
            data: source.data,
            parent_id: parent,
            created_at: "local".into(),
        })?;
        render_version(&restored)
    })())
}

pub fn export_resume_pdf(payload: Option<Value>) -> Envelope<Value> {
    outcome((|| {
        let payload = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&payload, "personaId")?;
        let requested = payload.get("versionId").and_then(Value::as_str);
        let version = if let Some(version_id) = requested {
            let value = get_version(version_id)?
                .ok_or_else(|| AppError::NotFound("resume version".into()))?;
            if value.persona_id != persona_id {
                return Err(AppError::Validation(
                    "resume version does not belong to persona".into(),
                ));
            }
            Some(value)
        } else {
            latest(persona_id)?
        };
        let version = match version {
            Some(value) => value,
            None => ResumeVersion {
                id: id(),
                persona_id: persona_id.into(),
                label: "Export".into(),
                template: ResumeTemplate::Modern,
                revision: 1,
                data: build(persona_id)?.data,
                parent_id: None,
                created_at: "local".into(),
            },
        };
        let bytes = SystemFontPdfExporter
            .export(version.template, &version.data)
            .map_err(|error| AppError::Unavailable(format!("PDF export failed: {error:?}")))?;
        let markdown = MarkdownExporter
            .export(version.template, &version.data)
            .map_err(|e| AppError::Unavailable(format!("resume render failed: {e:?}")))?;
        let hash = content_hash(&markdown);
        Ok(
            json!({"pdfBase64":base64(&bytes),"filename":format!("resume-{}.pdf",persona_id),"versionId":version.id,"template":version.template.id(),"contentHash":hash}),
        )
    })())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (chunk[0] as u32) << 16
            | (chunk.get(1).copied().unwrap_or(0) as u32) << 8
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn base64_is_rfc4648_compatible() {
        assert_eq!(base64(b"PDF"), "UERG");
        assert_eq!(base64(b"PDF!"), "UERGIQ==");
    }
    #[test]
    fn unknown_template_is_rejected() {
        let value = serde_json::to_value(generate_resume(Some(
            json!({"personaId":"p","template":"unknown"}),
        )))
        .unwrap();
        assert_eq!(value["success"], false);
    }
    #[test]
    fn v2_resume_commands_reject_missing_payload() {
        for result in [
            list_resume_versions(None),
            diff_resume_versions(None),
            restore_resume_version(None),
        ] {
            let value = serde_json::to_value(result).unwrap();
            assert_eq!(value["success"], false);
            assert_eq!(value["error"]["code"], "VALIDATION");
        }
    }
    #[test]
    fn v2_contract_registers_resume_version_commands() {
        let value: Value =
            serde_json::from_str(include_str!("../../../contracts/commands/v2/commands.json"))
                .unwrap();
        let names = value["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[0].as_str().unwrap())
            .collect::<Vec<_>>();
        for name in [
            "listResumeVersions",
            "diffResumeVersions",
            "restoreResumeVersion",
        ] {
            assert!(names.contains(&name));
        }
    }
    #[test]
    fn refinement_preview_confirm_and_stale_protocol() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("refine.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','Persona',1,'{}','[]',5)",[]).unwrap();
        let repo = SqliteResumeVersionRepository::new(&c);
        repo.save(&ResumeVersion {
            id: "base".into(),
            persona_id: "p".into(),
            label: "Base".into(),
            template: ResumeTemplate::Modern,
            revision: 0,
            data: ResumeRenderData {
                header: ResumeHeader {
                    full_name: "Test User".into(),
                    ..Default::default()
                },
                summary: Some("Original summary".into()),
                experience: vec![ResumeEntry {
                    source_experience_id: "e1".into(),
                    title: "Engineer".into(),
                    organization: Some("Acme".into()),
                    period: Some("2024".into()),
                    summary: None,
                    achievements: vec!["Built a dashboard".into()],
                    skills: vec!["Rust".into()],
                }],
                ..Default::default()
            },
            parent_id: None,
            created_at: "now".into(),
        })
        .unwrap();
        drop(c);
        let calls = Cell::new(0);
        let cancel = crate::infra::llm::CancellationToken::default();
        let preview = serde_json::to_value(chat_refine_resume_with_generator(
            Some(json!({"personaId":"p","instructionType":"concise","instruction":"more concise"})),
            &cancel,
            |_, _, _| {
                calls.set(calls.get() + 1);
                Ok(r#"{"summary":"Concise summary","experiences":[{"sourceExperienceId":"e1","achievements":["Shipped a dashboard"]}]}"#.into())
            },
        ))
        .unwrap();
        assert_eq!(preview["success"], true);
        assert_eq!(preview["data"]["requiresConfirmation"], true);
        assert!(preview["data"]["refinedSummary"]
            .as_str()
            .unwrap()
            .contains("Shipped a dashboard"));
        assert_eq!(calls.get(), 1);
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            SqliteResumeVersionRepository::new(&c)
                .list("p")
                .unwrap()
                .len(),
            1
        );
        drop(c);
        let proposal_id = preview["data"]["proposalId"].as_str().unwrap();
        let proposal_hash = preview["data"]["contentHash"].as_str().unwrap();
        let committed=serde_json::to_value(chat_refine_resume_with_generator(Some(json!({"personaId":"p","instruction":"more concise","confirm":true,"baseVersionId":"base","proposalId":proposal_id,"contentHash":proposal_hash,"refinedSummary":"tampered client text"})),&cancel,|_,_,_|panic!("confirm must not call provider"))).unwrap();
        assert_eq!(committed["success"], true);
        let c = Connection::open(&path).unwrap();
        let repo = SqliteResumeVersionRepository::new(&c);
        assert_eq!(repo.list("p").unwrap().len(), 2);
        assert_eq!(
            repo.latest("p").unwrap().unwrap().data.summary.as_deref(),
            Some("Concise summary")
        );
        assert_eq!(
            repo.latest("p").unwrap().unwrap().data.experience[0].achievements,
            vec!["Shipped a dashboard".to_string()]
        );
        drop(c);
        let stale=serde_json::to_value(chat_refine_resume_with_generator(Some(json!({"personaId":"p","instruction":"again","confirm":true,"baseVersionId":"base","proposalId":proposal_id,"contentHash":proposal_hash})),&cancel,|_,_,_|panic!("stale confirm must not call provider"))).unwrap();
        assert_eq!(stale["success"], false);
        assert_eq!(stale["error"]["code"], "CONFLICT");
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            SqliteResumeVersionRepository::new(&c)
                .list("p")
                .unwrap()
                .len(),
            2
        );
        drop(c);
        let undone = serde_json::to_value(chat_refine_resume_with_generator(
            Some(json!({"personaId":"p","instruction":"undo"})),
            &cancel,
            |_, _, _| panic!("undo must not call provider"),
        ))
        .unwrap();
        assert_eq!(undone["success"], true);
        let redone = serde_json::to_value(chat_refine_resume_with_generator(
            Some(json!({"personaId":"p","instruction":"redo"})),
            &cancel,
            |_, _, _| panic!("redo must not call provider"),
        ))
        .unwrap();
        assert_eq!(redone["success"], true);
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            SqliteResumeVersionRepository::new(&c)
                .list("p")
                .unwrap()
                .len(),
            4
        );
        let redone_version = SqliteResumeVersionRepository::new(&c)
            .latest("p")
            .unwrap()
            .unwrap();
        assert_eq!(
            redone_version.data.summary.as_deref(),
            Some("Concise summary")
        );
        assert_eq!(
            redone_version.data.experience[0].achievements,
            vec!["Shipped a dashboard".to_string()]
        );
        assert_eq!(redone_version.data.header.full_name, "Test User");
        drop(c);
        let invalid = serde_json::to_value(chat_refine_resume(None)).unwrap();
        assert_eq!(invalid["error"]["code"], "VALIDATION");
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn accepts_only_explicit_tuning_instruction_types() {
        for kind in [
            "leadership",
            "metrics",
            "concise",
            "technical_depth",
            "job_alignment",
            "general",
        ] {
            assert_eq!(parse_tuning_instruction_type(kind).unwrap(), kind)
        }
        assert!(parse_tuning_instruction_type("more concise").is_err());
        assert!(parse_tuning_instruction_type("精简").is_err())
    }
    #[test]
    fn proposal_is_bound_single_use_tamper_proof_and_expires() {
        let (summary_id, hash) = issue_proposal("base", "server summary".into());
        assert!(consume_proposal(&summary_id, "other", &hash).is_err());
        assert!(consume_proposal(&summary_id, "base", "tampered").is_err());
        assert_eq!(
            consume_proposal(&summary_id, "base", &hash).unwrap(),
            "server summary"
        );
        assert!(consume_proposal(&summary_id, "base", &hash).is_err());
        let (expired_id, expired_hash) = issue_proposal("base", "expired".into());
        PROPOSALS
            .get()
            .unwrap()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_mut(&expired_id)
            .unwrap()
            .expires = Instant::now() - Duration::from_secs(1);
        assert!(consume_proposal(&expired_id, "base", &expired_hash).is_err())
    }
    #[test]
    fn five_tuning_types_have_distinct_prompt_guidance() {
        for kind in [
            "leadership",
            "metrics",
            "concise",
            "technical_depth",
            "job_alignment",
        ] {
            let guidance = tuning_guidance(kind);
            assert!(!guidance.is_empty());
            assert!(
                guidance.contains("已有")
                    || guidance.contains("保留")
                    || guidance.contains("事实")
                    || guidance.contains("existing")
                    || guidance.contains("preserving")
            )
        }
    }
    #[test]
    fn resume_commands_generate_list_diff_restore_and_export() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("matrix.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,identity_statement,career_narrative,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','Ada',1,'Engineer','Builder','{}','[]',5)",[]).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('q','q','Other',0,'{}','[]',5)",[]).unwrap();
        c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,start_date,end_date) VALUES('e','u','work','Developer','Built systems','[\"Improved quality\"]','[\"Rust\"]','[]','confirmed',1,'2020-01-01','2024-01-01')",[]).unwrap();
        c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,start_date) VALUES('edu','u','education','BSc','','[]','[]','[]','confirmed',1,'2016-01-01')",[]).unwrap();
        drop(c);
        let first = serde_json::to_value(generate_resume(Some(
            json!({"personaId":"p","template":"classic"}),
        )))
        .unwrap();
        assert_eq!(first["success"], true, "{first:?}");
        let first_id = first["data"]["versionId"].as_str().unwrap().to_owned();
        let second = serde_json::to_value(generate_resume(Some(
            json!({"personaId":"p","template":"technical"}),
        )))
        .unwrap();
        assert_eq!(second["success"], true, "{second:?}");
        let second_id = second["data"]["versionId"].as_str().unwrap().to_owned();
        let list =
            serde_json::to_value(list_resume_versions(Some(json!({"personaId":"p"})))).unwrap();
        assert_eq!(list["data"]["count"], 2, "{list:?}");
        let compared = serde_json::to_value(diff_resume_versions(Some(
            json!({"leftVersionId":first_id,"rightVersionId":second_id}),
        )))
        .unwrap();
        assert_eq!(compared["success"], true, "{compared:?}");
        let restored = serde_json::to_value(restore_resume_version(Some(
            json!({"personaId":"p","versionId":first["data"]["versionId"]}),
        )))
        .unwrap();
        assert_eq!(restored["success"], true, "{restored:?}");
        let exported =
            serde_json::to_value(export_resume_pdf(Some(json!({"personaId":"p"})))).unwrap();
        assert_eq!(exported["success"], true);
        assert!(exported["data"]["pdfBase64"]
            .as_str()
            .unwrap()
            .starts_with("JVBER"));
        let fallback =
            serde_json::to_value(export_resume_pdf(Some(json!({"personaId":"q"})))).unwrap();
        assert_eq!(fallback["success"], true);
        let c = Connection::open(&path).unwrap();
        let q = SqliteResumeVersionRepository::new(&c)
            .save(&ResumeVersion {
                id: "qv".into(),
                persona_id: "q".into(),
                label: "q".into(),
                template: ResumeTemplate::Modern,
                revision: 0,
                data: ResumeRenderData {
                    header: ResumeHeader {
                        full_name: "Other".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                parent_id: None,
                created_at: "now".into(),
            })
            .unwrap();
        drop(c);
        let cross = serde_json::to_value(diff_resume_versions(Some(
            json!({"leftVersionId":first["data"]["versionId"],"rightVersionId":q.id}),
        )))
        .unwrap();
        assert_eq!(cross["error"]["code"], "VALIDATION");
        let wrong = serde_json::to_value(restore_resume_version(Some(
            json!({"personaId":"q","versionId":first["data"]["versionId"]}),
        )))
        .unwrap();
        assert_eq!(wrong["error"]["code"], "VALIDATION");
        let missing = serde_json::to_value(diff_resume_versions(Some(
            json!({"leftVersionId":"missing","rightVersionId":"also"}),
        )))
        .unwrap();
        assert_eq!(missing["error"]["code"], "NOT_FOUND");
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn refinement_rejects_blank_provider_output_and_incomplete_confirmation() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("errors.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','Ada',1,'{}','[]',5)",[]).unwrap();
        SqliteResumeVersionRepository::new(&c)
            .save(&ResumeVersion {
                id: "base".into(),
                persona_id: "p".into(),
                label: "base".into(),
                template: ResumeTemplate::Modern,
                revision: 0,
                data: ResumeRenderData {
                    header: ResumeHeader {
                        full_name: "Ada".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                parent_id: None,
                created_at: "now".into(),
            })
            .unwrap();
        drop(c);
        let cancel = crate::infra::llm::CancellationToken::default();
        let blank = serde_json::to_value(chat_refine_resume_with_generator(
            Some(json!({"personaId":"p","instructionType":"general","instruction":"x"})),
            &cancel,
            |_, _, _| Ok(" ".into()),
        ))
        .unwrap();
        assert_eq!(blank["error"]["code"], "UNAVAILABLE");
        let missing = serde_json::to_value(chat_refine_resume_with_generator(
            Some(json!({"personaId":"p","instruction":"x","confirm":true})),
            &cancel,
            |_, _, _| panic!(),
        ))
        .unwrap();
        assert_eq!(missing["error"]["code"], "VALIDATION");
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn preview_is_read_only_all_templates_and_generate_has_same_hash() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preview.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','Ada',1,'{\"Rust\":1}','[]',2)",[]).unwrap();
        c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,industry_tags) VALUES('e','u','work','Engineer','Built','[]','[\"Rust\"]','[]','confirmed',1,'[]')",[]).unwrap();
        drop(c);
        for template in ResumeTemplate::ALL {
            let before = {
                let c = Connection::open(&path).unwrap();
                (
                    c.query_row("SELECT COUNT(*) FROM resume_versions", [], |r| {
                        r.get::<_, u32>(0)
                    })
                    .unwrap(),
                    c.query_row("SELECT COUNT(*) FROM role_experience_weights", [], |r| {
                        r.get::<_, u32>(0)
                    })
                    .unwrap(),
                )
            };
            let preview = serde_json::to_value(preview_resume(Some(
                json!({"personaId":"p","template":template.id()}),
            )))
            .unwrap();
            assert_eq!(preview["success"], true);
            let after = {
                let c = Connection::open(&path).unwrap();
                (
                    c.query_row("SELECT COUNT(*) FROM resume_versions", [], |r| {
                        r.get::<_, u32>(0)
                    })
                    .unwrap(),
                    c.query_row("SELECT COUNT(*) FROM role_experience_weights", [], |r| {
                        r.get::<_, u32>(0)
                    })
                    .unwrap(),
                )
            };
            assert_eq!(before, after);
            if template == ResumeTemplate::Classic {
                let generated = serde_json::to_value(generate_resume(Some(
                    json!({"personaId":"p","template":"classic"}),
                )))
                .unwrap();
                assert_eq!(
                    preview["data"]["contentHash"],
                    generated["data"]["contentHash"]
                )
            }
        }
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
    #[test]
    fn export_can_target_old_version_and_rejects_cross_persona_or_missing() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export-version.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        for (id, user) in [("p", "u"), ("q", "q")] {
            c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES(?1,?2,?1,0,'{}','[]',5)",rusqlite::params![id,user]).unwrap();
        }
        let repo = SqliteResumeVersionRepository::new(&c);
        let first = repo
            .save(&ResumeVersion {
                id: "old".into(),
                persona_id: "p".into(),
                label: "old".into(),
                template: ResumeTemplate::Classic,
                revision: 0,
                data: ResumeRenderData {
                    header: ResumeHeader {
                        full_name: "Old".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                parent_id: None,
                created_at: "1".into(),
            })
            .unwrap();
        repo.save(&ResumeVersion {
            id: "new".into(),
            persona_id: "p".into(),
            label: "new".into(),
            template: ResumeTemplate::Modern,
            revision: 0,
            data: ResumeRenderData {
                header: ResumeHeader {
                    full_name: "New".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            parent_id: None,
            created_at: "2".into(),
        })
        .unwrap();
        drop(c);
        let exact = serde_json::to_value(export_resume_pdf(Some(
            json!({"personaId":"p","versionId":first.id}),
        )))
        .unwrap();
        assert_eq!(exact["data"]["versionId"], "old");
        assert_eq!(exact["data"]["template"], "classic");
        let latest =
            serde_json::to_value(export_resume_pdf(Some(json!({"personaId":"p"})))).unwrap();
        assert_eq!(latest["data"]["versionId"], "new");
        let cross = serde_json::to_value(export_resume_pdf(Some(
            json!({"personaId":"q","versionId":"old"}),
        )))
        .unwrap();
        assert_eq!(cross["error"]["code"], "VALIDATION");
        let missing = serde_json::to_value(export_resume_pdf(Some(
            json!({"personaId":"p","versionId":"missing"}),
        )))
        .unwrap();
        assert_eq!(missing["error"]["code"], "NOT_FOUND");
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
}
