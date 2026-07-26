use super::{DocumentError, DocumentExporter, DocumentFormat};
use crate::domain::resume::{ResumeEntry, ResumeRenderData, ResumeTemplate};

pub struct MarkdownExporter;

impl DocumentExporter for MarkdownExporter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Markdown
    }
    fn file_extension(&self) -> &'static str {
        "md"
    }
    fn export(
        &self,
        template: ResumeTemplate,
        data: &ResumeRenderData,
    ) -> Result<Vec<u8>, DocumentError> {
        data.validate()
            .map_err(|error| DocumentError::InvalidData(format!("{error:?}")))?;
        Ok(render(template, data).into_bytes())
    }
}

pub fn render(template: ResumeTemplate, data: &ResumeRenderData) -> String {
    let mut out = format!(
        "<!-- careercraft-template:{} -->\n# {}\n",
        template.id(),
        data.header.full_name.trim()
    );
    if !data.header.headline.trim().is_empty() {
        out.push_str(&format!("\n**{}**\n", data.header.headline.trim()));
    }
    let contacts = [
        data.header.email.as_deref(),
        data.header.phone.as_deref(),
        data.header.location.as_deref(),
    ]
    .into_iter()
    .flatten()
    .chain(data.header.links.iter().map(String::as_str))
    .collect::<Vec<_>>();
    if !contacts.is_empty() {
        out.push_str(&format!("\n{}\n", contacts.join(" · ")));
    }
    if let Some(summary) = &data.summary {
        section(&mut out, "Summary");
        out.push_str(summary.trim());
        out.push('\n');
    }
    entries(&mut out, "Experience", &data.experience);
    entries(&mut out, "Education", &data.education);
    if !data.skills.is_empty() {
        section(&mut out, "Skills");
        out.push_str(&data.skills.join(" · "));
        out.push('\n');
    }
    for (title, lines) in &data.extra_sections {
        section(&mut out, title);
        for line in lines {
            out.push_str(&format!("- {}\n", line.trim()));
        }
    }
    out
}

fn section(out: &mut String, title: &str) {
    out.push_str(&format!("\n## {title}\n\n"));
}
fn entries(out: &mut String, title: &str, items: &[ResumeEntry]) {
    if items.is_empty() {
        return;
    }
    section(out, title);
    for item in items {
        out.push_str(&format!("### {}", item.title.trim()));
        if let Some(org) = &item.organization {
            out.push_str(&format!(" — {}", org.trim()));
        }
        out.push('\n');
        if let Some(period) = &item.period {
            out.push_str(&format!("*{}*\n", period.trim()));
        }
        if let Some(summary) = &item.summary {
            out.push_str(&format!("{}\n", summary.trim()));
        }
        for achievement in &item.achievements {
            out.push_str(&format!("- {}\n", achievement.trim()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resume::ResumeHeader;
    #[test]
    fn markdown_is_utf8_and_records_template() {
        let data = ResumeRenderData {
            header: ResumeHeader {
                full_name: "张三".into(),
                headline: "产品经理".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let bytes = MarkdownExporter
            .export(ResumeTemplate::Modern, &data)
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("careercraft-template:modern"));
        assert!(text.contains("张三"));
    }
    fn complete_data() -> ResumeRenderData {
        let mut extra = std::collections::BTreeMap::new();
        extra.insert("Awards".into(), vec!["Winner".into()]);
        ResumeRenderData {
            header: ResumeHeader {
                full_name: "Ada".into(),
                headline: "Engineer".into(),
                email: Some("a@example.com".into()),
                phone: Some("123".into()),
                location: Some("London".into()),
                links: vec!["https://example.com".into()],
            },
            summary: Some("Summary".into()),
            experience: vec![ResumeEntry {
                source_experience_id: "e".into(),
                title: "Developer".into(),
                organization: Some("Org".into()),
                period: Some("2020-2024".into()),
                summary: Some("Built systems".into()),
                achievements: vec!["Improved quality".into()],
                skills: vec!["Rust".into()],
            }],
            education: vec![ResumeEntry {
                source_experience_id: "d".into(),
                title: "BSc".into(),
                ..Default::default()
            }],
            skills: vec!["Rust".into()],
            extra_sections: extra,
        }
    }
    #[test]
    fn all_templates_render_every_section() {
        for template in ResumeTemplate::ALL {
            let text = render(template, &complete_data());
            assert!(text.contains(template.id()));
            for expected in [
                "Ada",
                "Engineer",
                "a@example.com",
                "Summary",
                "Experience",
                "Developer",
                "Org",
                "2020-2024",
                "Improved quality",
                "Education",
                "Skills",
                "Awards",
                "Winner",
            ] {
                assert!(text.contains(expected), "missing {expected}");
            }
        }
    }
    #[test]
    fn exporter_rejects_missing_name() {
        let error = MarkdownExporter
            .export(ResumeTemplate::Classic, &ResumeRenderData::default())
            .unwrap_err();
        assert!(matches!(error, DocumentError::InvalidData(_)));
    }
}
