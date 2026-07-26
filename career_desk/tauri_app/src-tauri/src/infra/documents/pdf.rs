use super::markdown::render as render_markdown;
use super::{DocumentError, DocumentExporter, DocumentFormat};
use crate::domain::resume::{ResumeRenderData, ResumeTemplate};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Production PDF exporter that discovers a redistributable Windows CJK TrueType
/// font at runtime and embeds it into the generated document. The application
/// package itself carries no font payload.
pub struct SystemFontPdfExporter;
pub type MinimalPdfExporter = SystemFontPdfExporter;

impl DocumentExporter for SystemFontPdfExporter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Pdf
    }
    fn file_extension(&self) -> &'static str {
        "pdf"
    }
    fn export(
        &self,
        template: ResumeTemplate,
        data: &ResumeRenderData,
    ) -> Result<Vec<u8>, DocumentError> {
        data.validate()
            .map_err(|error| DocumentError::InvalidData(format!("{error:?}")))?;
        let font_path = discover_cjk_font().ok_or_else(|| {
            DocumentError::Render(
                "No usable CJK TrueType font was found. Install DengXian, SimHei, or Noto Sans SC."
                    .into(),
            )
        })?;
        let font_bytes = fs::read(&font_path)
            .map_err(|error| DocumentError::Render(format!("cannot read system font: {error}")))?;
        render_unicode_pdf(&render_markdown(template, data), &font_bytes)
    }
}

pub fn discover_cjk_font() -> Option<PathBuf> {
    let root = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let fonts = root.join("Fonts");
    ["Deng.ttf", "simhei.ttf", "NotoSansSC-VF.ttf", "simsunb.ttf"]
        .into_iter()
        .map(|name| fonts.join(name))
        .find(|path| path.is_file() && font_allows_embedding(path))
}

fn font_allows_embedding(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let Ok(font) = TrueTypeFont::parse(&bytes) else {
        return false;
    };
    // OS/2 fsType bit 1 means Restricted License Embedding.
    font.table(b"OS/2")
        .and_then(|table| be_u16(table, 8))
        .map(|flags| flags & 0x0002 == 0)
        .unwrap_or(true)
}

fn render_unicode_pdf(text: &str, font_bytes: &[u8]) -> Result<Vec<u8>, DocumentError> {
    let font = TrueTypeFont::parse(font_bytes)?;
    let lines: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with("<!--"))
        .take(48)
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .replace("**", "")
                .replace('*', "")
        })
        .collect();
    let mut mappings = BTreeMap::<u16, char>::new();
    let mut widths = BTreeMap::<u16, u16>::new();
    let mut content = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let mut hex = String::new();
        for ch in line.chars() {
            let glyph = font.glyph_id(ch).ok_or_else(|| {
                DocumentError::Render(format!("system font lacks glyph U+{:04X}", ch as u32))
            })?;
            mappings.entry(glyph).or_insert(ch);
            widths.entry(glyph).or_insert(font.width_1000(glyph));
            hex.push_str(&format!("{glyph:04X}"));
        }
        let y = 790_i32 - (index as i32 * 15);
        content.extend_from_slice(format!("BT /F0 10 Tf 46 {y} Td <{hex}> Tj ET\n").as_bytes());
    }
    let to_unicode = to_unicode_cmap(&mappings);
    let width_spec = widths
        .iter()
        .map(|(gid, width)| format!("{gid} [{width}]"))
        .collect::<Vec<_>>()
        .join(" ");
    let font_name = "CareerCraftSystemCJK";
    let objects = vec![
        plain("<< /Type /Catalog /Pages 2 0 R >>"),
        plain("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        plain("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F0 5 0 R >> >> /Contents 4 0 R >>"),
        stream(&content, ""),
        plain(&format!("<< /Type /Font /Subtype /Type0 /BaseFont /{font_name} /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>")),
        plain(&format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{font_name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 1000 /W [{width_spec}] >>")),
        plain(&format!("<< /Type /FontDescriptor /FontName /{font_name} /Flags 4 /FontBBox [-1000 -1000 3000 3000] /ItalicAngle 0 /Ascent 900 /Descent -250 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>")),
        stream(font_bytes, &format!("/Length1 {}", font_bytes.len())),
        stream(to_unicode.as_bytes(), ""),
    ];
    Ok(build_pdf(objects))
}

fn plain(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}
fn stream(bytes: &[u8], extra: &str) -> Vec<u8> {
    let mut value = format!("<< /Length {} {extra} >>\nstream\n", bytes.len()).into_bytes();
    value.extend_from_slice(bytes);
    value.extend_from_slice(b"\nendstream");
    value
}
fn build_pdf(objects: Vec<Vec<u8>>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn to_unicode_cmap(mappings: &BTreeMap<u16, char>) -> String {
    let mut body = String::new();
    for (glyph, ch) in mappings {
        let unicode = if (*ch as u32) <= 0xFFFF {
            format!("{:04X}", *ch as u32)
        } else {
            let value = *ch as u32 - 0x10000;
            format!(
                "{:04X}{:04X}",
                0xD800 + (value >> 10),
                0xDC00 + (value & 0x3FF)
            )
        };
        body.push_str(&format!("<{glyph:04X}> <{unicode}>\n"));
    }
    format!("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /CareerCraft-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n{} beginbfchar\n{}endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend", mappings.len(), body)
}

struct TrueTypeFont<'a> {
    bytes: &'a [u8],
    tables: BTreeMap<[u8; 4], (usize, usize)>,
}
impl<'a> TrueTypeFont<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, DocumentError> {
        if bytes.len() < 12 || &bytes[..4] == b"ttcf" {
            return Err(DocumentError::Render(
                "font must be a standalone TrueType/OpenType file".into(),
            ));
        }
        let count = be_u16(bytes, 4)
            .ok_or_else(|| DocumentError::Render("invalid font header".into()))?
            as usize;
        let mut tables = BTreeMap::new();
        for index in 0..count {
            let pos = 12 + index * 16;
            let tag: [u8; 4] = bytes
                .get(pos..pos + 4)
                .and_then(|value| value.try_into().ok())
                .ok_or_else(|| DocumentError::Render("invalid font directory".into()))?;
            let offset = be_u32(bytes, pos + 8).unwrap_or(0) as usize;
            let length = be_u32(bytes, pos + 12).unwrap_or(0) as usize;
            if offset
                .checked_add(length)
                .filter(|end| *end <= bytes.len())
                .is_none()
            {
                return Err(DocumentError::Render("font table is out of bounds".into()));
            }
            tables.insert(tag, (offset, length));
        }
        Ok(Self { bytes, tables })
    }
    fn table(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        let (offset, length) = *self.tables.get(tag)?;
        self.bytes.get(offset..offset + length)
    }
    fn glyph_id(&self, ch: char) -> Option<u16> {
        let cmap = self.table(b"cmap")?;
        let count = be_u16(cmap, 2)? as usize;
        let mut format4 = None;
        for index in 0..count {
            let pos = 4 + index * 8;
            let offset = be_u32(cmap, pos + 4)? as usize;
            match be_u16(cmap, offset)? {
                12 => {
                    if let Some(glyph) = cmap12(cmap.get(offset..)?, ch as u32) {
                        return Some(glyph);
                    }
                }
                4 => format4 = cmap.get(offset..),
                _ => {}
            }
        }
        format4.and_then(|table| cmap4(table, ch as u32))
    }
    fn width_1000(&self, glyph: u16) -> u16 {
        let units = self
            .table(b"head")
            .and_then(|t| be_u16(t, 18))
            .unwrap_or(1000)
            .max(1) as u32;
        let metrics = self
            .table(b"hhea")
            .and_then(|t| be_u16(t, 34))
            .unwrap_or(1)
            .max(1);
        let index = glyph.min(metrics - 1) as usize;
        let width = self
            .table(b"hmtx")
            .and_then(|t| be_u16(t, index * 4))
            .unwrap_or(units as u16) as u32;
        ((width * 1000) / units).min(u16::MAX as u32) as u16
    }
}

fn cmap12(table: &[u8], codepoint: u32) -> Option<u16> {
    let count = be_u32(table, 12)? as usize;
    for index in 0..count {
        let pos = 16 + index * 12;
        let start = be_u32(table, pos)?;
        let end = be_u32(table, pos + 4)?;
        if codepoint >= start && codepoint <= end {
            return u16::try_from(be_u32(table, pos + 8)? + codepoint - start).ok();
        }
    }
    None
}
fn cmap4(table: &[u8], codepoint: u32) -> Option<u16> {
    let code = u16::try_from(codepoint).ok()?;
    let seg_count = be_u16(table, 6)? as usize / 2;
    let end_base = 14;
    let start_base = end_base + seg_count * 2 + 2;
    let delta_base = start_base + seg_count * 2;
    let range_base = delta_base + seg_count * 2;
    for index in 0..seg_count {
        let end = be_u16(table, end_base + index * 2)?;
        let start = be_u16(table, start_base + index * 2)?;
        if code < start || code > end {
            continue;
        }
        let delta = be_u16(table, delta_base + index * 2)?;
        let range = be_u16(table, range_base + index * 2)? as usize;
        if range == 0 {
            return Some(code.wrapping_add(delta));
        }
        let range_pos = range_base + index * 2;
        let glyph_pos = range_pos + range + (code - start) as usize * 2;
        let glyph = be_u16(table, glyph_pos)?;
        return Some(if glyph == 0 {
            0
        } else {
            glyph.wrapping_add(delta)
        });
    }
    None
}
fn be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resume::ResumeHeader;
    #[test]
    fn exports_chinese_with_embedded_system_font_when_available() {
        if discover_cjk_font().is_none() {
            return;
        }
        let data = ResumeRenderData {
            header: ResumeHeader {
                full_name: "张三".into(),
                headline: "产品经理".into(),
                ..Default::default()
            },
            skills: vec!["用户研究".into()],
            ..Default::default()
        };
        let bytes = SystemFontPdfExporter
            .export(ResumeTemplate::Modern, &data)
            .unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
        assert!(bytes.windows(11).any(|window| window == b"/Identity-H"));
        assert!(bytes.len() > 1_000_000);
    }
    #[test]
    fn all_five_templates_export() {
        if discover_cjk_font().is_none() {
            return;
        }
        let data = ResumeRenderData {
            header: ResumeHeader {
                full_name: "Ada".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        for template in ResumeTemplate::ALL {
            assert!(SystemFontPdfExporter.export(template, &data).is_ok());
        }
    }
    #[test]
    fn metadata_and_invalid_data_are_explicit() {
        let exporter = SystemFontPdfExporter;
        assert_eq!(exporter.format(), DocumentFormat::Pdf);
        assert_eq!(exporter.file_extension(), "pdf");
        assert!(matches!(
            exporter.export(ResumeTemplate::Classic, &ResumeRenderData::default()),
            Err(DocumentError::InvalidData(_))
        ));
    }
    #[test]
    fn pdf_builder_writes_xref_and_unicode_map() {
        let mut map = BTreeMap::new();
        map.insert(1, 'A');
        map.insert(2, '😀');
        let cmap = to_unicode_cmap(&map);
        assert!(cmap.contains("<0001> <0041>"));
        assert!(cmap.contains("D83DDE00"));
        let pdf = build_pdf(vec![plain("<< /Type /Catalog >>"), stream(b"hello", "")]);
        assert!(pdf.starts_with(b"%PDF-1.7"));
        assert!(pdf.windows(4).any(|v| v == b"xref"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }
    #[test]
    fn malformed_font_tables_are_rejected() {
        assert!(TrueTypeFont::parse(b"bad").is_err());
        assert!(TrueTypeFont::parse(b"ttcf\0\0\0\0\0\0\0\0").is_err());
        assert_eq!(be_u16(&[0, 2], 0), Some(2));
        assert_eq!(be_u32(&[0, 0, 0, 3], 0), Some(3));
        assert_eq!(be_u32(&[0], 0), None);
    }
    #[test]
    fn cmap_helpers_cover_format_12_and_format_4() {
        let mut twelve = vec![0u8; 28];
        twelve[12..16].copy_from_slice(&1u32.to_be_bytes());
        twelve[16..20].copy_from_slice(&65u32.to_be_bytes());
        twelve[20..24].copy_from_slice(&66u32.to_be_bytes());
        twelve[24..28].copy_from_slice(&10u32.to_be_bytes());
        assert_eq!(cmap12(&twelve, 65), Some(10));
        assert_eq!(cmap12(&twelve, 66), Some(11));
        assert_eq!(cmap12(&twelve, 70), None);
        let mut four = vec![0u8; 28];
        four[6..8].copy_from_slice(&2u16.to_be_bytes());
        four[14..16].copy_from_slice(&65u16.to_be_bytes());
        four[18..20].copy_from_slice(&65u16.to_be_bytes());
        four[20..22].copy_from_slice(&1u16.to_be_bytes());
        assert_eq!(cmap4(&four, 65), Some(66));
        assert_eq!(cmap4(&four, 64), None);
        assert_eq!(cmap4(&four, 0x1_0000), None);
        four[22..24].copy_from_slice(&2u16.to_be_bytes());
        four[24..26].copy_from_slice(&5u16.to_be_bytes());
        assert_eq!(cmap4(&four, 65), Some(6));
        four[24..26].copy_from_slice(&0u16.to_be_bytes());
        assert_eq!(cmap4(&four, 65), Some(0));
    }
    #[test]
    fn parses_minimal_table_directory_and_rejects_bounds() {
        let mut font = vec![0u8; 30];
        font[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());
        font[4..6].copy_from_slice(&1u16.to_be_bytes());
        font[12..16].copy_from_slice(b"test");
        font[20..24].copy_from_slice(&28u32.to_be_bytes());
        font[24..28].copy_from_slice(&2u32.to_be_bytes());
        font[28..30].copy_from_slice(&[1, 2]);
        let parsed = TrueTypeFont::parse(&font).unwrap();
        assert_eq!(parsed.table(b"test"), Some(&[1, 2][..]));
        font[20..24].copy_from_slice(&29u32.to_be_bytes());
        assert!(TrueTypeFont::parse(&font).is_err());
        assert!(!font_allows_embedding(std::path::Path::new(
            "definitely-missing-font.ttf"
        )));
    }
}
