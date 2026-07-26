//! Replaceable document exporters. No Python, Qt, browser automation, or CDN.
//! Requirement mapping: CC-FR-006.

pub mod import;
mod markdown;
mod pdf;

pub use markdown::MarkdownExporter;
pub use pdf::{MinimalPdfExporter, SystemFontPdfExporter};

use crate::domain::resume::{ResumeRenderData, ResumeTemplate};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentFormat {
    Markdown,
    Pdf,
    Docx,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentError {
    InvalidData(String),
    Unsupported(DocumentFormat),
    Render(String),
}

pub trait DocumentExporter: Send + Sync {
    fn format(&self) -> DocumentFormat;
    fn file_extension(&self) -> &'static str;
    fn export(
        &self,
        template: ResumeTemplate,
        data: &ResumeRenderData,
    ) -> Result<Vec<u8>, DocumentError>;
}

pub fn exporter_for<'a>(
    format: DocumentFormat,
    exporters: &'a [&dyn DocumentExporter],
) -> Result<&'a dyn DocumentExporter, DocumentError> {
    exporters
        .iter()
        .copied()
        .find(|exporter| exporter.format() == format)
        .ok_or(DocumentError::Unsupported(format))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_exporter_and_exposes_metadata() {
        let markdown = MarkdownExporter;
        let pdf = SystemFontPdfExporter;
        let exporters: [&dyn DocumentExporter; 2] = [&markdown, &pdf];
        let selected = exporter_for(DocumentFormat::Markdown, &exporters).unwrap();
        assert_eq!(selected.format(), DocumentFormat::Markdown);
        assert_eq!(selected.file_extension(), "md");
        assert_eq!(pdf.file_extension(), "pdf");
    }
    #[test]
    fn unsupported_format_is_explicit() {
        let markdown = MarkdownExporter;
        assert!(matches!(
            exporter_for(DocumentFormat::Docx, &[&markdown]),
            Err(DocumentError::Unsupported(DocumentFormat::Docx))
        ));
    }
}
