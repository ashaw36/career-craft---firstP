use flate2::read::DeflateDecoder;
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub enum ImportError {
    Invalid(String),
    Unsupported(String),
    Corrupt(String),
}
pub fn extract(name: &str, bytes: &[u8]) -> Result<String, ImportError> {
    if bytes.len() > 20 * 1024 * 1024 {
        return Err(ImportError::Invalid("file exceeds 20 MiB".into()));
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "txt" | "md" | "json" => String::from_utf8(bytes.to_vec())
            .map_err(|_| ImportError::Invalid("text must be UTF-8".into())),
        "docx" => docx(bytes),
        "pdf" => pdf(bytes),
        _ => Err(ImportError::Unsupported(
            "supported formats: txt, md, json, docx, pdf".into(),
        )),
    }
}
fn docx(bytes: &[u8]) -> Result<String, ImportError> {
    const MAX_ENTRIES: usize = 256;
    const MAX_ENTRY: usize = 8 * 1024 * 1024;
    const MAX_TOTAL: usize = 16 * 1024 * 1024;
    const MAX_RATIO: usize = 100;
    const MAX_TEXT: usize = 4 * 1024 * 1024;
    let mut at = 0;
    let mut entries = 0;
    let mut total_expanded = 0usize;
    while at + 30 <= bytes.len() {
        if &bytes[at..at + 4] != b"PK\x03\x04" {
            at += 1;
            continue;
        }
        let method = u16::from_le_bytes([bytes[at + 8], bytes[at + 9]]);
        entries += 1;
        if entries > MAX_ENTRIES {
            return Err(ImportError::Invalid("DOCX has too many entries".into()));
        }
        let size = u32::from_le_bytes(bytes[at + 18..at + 22].try_into().unwrap()) as usize;
        let expanded = u32::from_le_bytes(bytes[at + 22..at + 26].try_into().unwrap()) as usize;
        total_expanded = total_expanded.saturating_add(expanded);
        if total_expanded > MAX_TOTAL {
            return Err(ImportError::Invalid(
                "DOCX total extraction exceeds limit".into(),
            ));
        }
        if expanded > MAX_ENTRY || (size > 0 && expanded / size.max(1) > MAX_RATIO) {
            return Err(ImportError::Invalid(
                "DOCX entry exceeds extraction limits".into(),
            ));
        }
        let n = u16::from_le_bytes([bytes[at + 26], bytes[at + 27]]) as usize;
        let x = u16::from_le_bytes([bytes[at + 28], bytes[at + 29]]) as usize;
        let start = at + 30 + n + x;
        if start + size > bytes.len() {
            return Err(ImportError::Corrupt("DOCX entry out of bounds".into()));
        }
        let name = std::str::from_utf8(&bytes[at + 30..at + 30 + n]).unwrap_or("");
        if name == "word/document.xml" {
            let raw = match method {
                0 => bytes[start..start + size].to_vec(),
                8 => {
                    let mut out = Vec::new();
                    DeflateDecoder::new(&bytes[start..start + size])
                        .take((MAX_ENTRY + 1) as u64)
                        .read_to_end(&mut out)
                        .map_err(|_| ImportError::Corrupt("DOCX deflate stream invalid".into()))?;
                    if out.len() > MAX_ENTRY {
                        return Err(ImportError::Invalid(
                            "DOCX entry exceeds extraction limits".into(),
                        ));
                    }
                    out
                }
                _ => {
                    return Err(ImportError::Unsupported(
                        "DOCX compression unsupported".into(),
                    ))
                }
            };
            let xml = String::from_utf8(raw)
                .map_err(|_| ImportError::Corrupt("DOCX XML is not UTF-8".into()))?;
            if xml.len() > MAX_TEXT {
                return Err(ImportError::Invalid("DOCX XML text exceeds limit".into()));
            }
            let text = xml.replace("</w:p>", "\n").replace("</w:tr>", "\n");
            let mut out = String::new();
            let mut tag = false;
            for c in text.chars() {
                match c {
                    '<' => tag = true,
                    '>' => tag = false,
                    _ if !tag => out.push(c),
                    _ => {}
                }
            }
            return Ok(out
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .trim()
                .into());
        }
        at = start + size;
    }
    Err(ImportError::Corrupt("word/document.xml missing".into()))
}
fn pdf(bytes: &[u8]) -> Result<String, ImportError> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(ImportError::Corrupt("invalid PDF header".into()));
    }
    let raw = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != ')' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                s.push(chars[i]);
                i += 1;
            }
            let tail: String = chars.iter().skip(i + 1).take(8).collect();
            if tail.contains("Tj") || tail.contains("TJ") {
                out.push(s);
            }
        }
        i += 1;
    }
    if out.is_empty() {
        Err(ImportError::Unsupported(
            "PDF contains no extractable text; scanned PDFs require OCR, which is not bundled"
                .into(),
        ))
    } else {
        Ok(out.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_round_trip() {
        assert_eq!(extract("a.md", "中文".as_bytes()).unwrap(), "中文");
    }
    #[test]
    fn scanned_pdf_is_explicit() {
        assert!(
            matches!(extract("a.pdf",b"%PDF-1.7 no text"),Err(ImportError::Unsupported(v))if v.contains("OCR"))
        );
    }
    #[test]
    fn rejects_docx_entry_with_excessive_expansion_claim() {
        let mut bytes = vec![0u8; 30];
        bytes[0..4].copy_from_slice(b"PK\x03\x04");
        bytes[22..26].copy_from_slice(&(9u32 * 1024 * 1024).to_le_bytes());
        assert!(
            matches!(extract("bomb.docx",&bytes),Err(ImportError::Invalid(v)) if v.contains("limits"))
        );
    }
    fn stored_docx(name: &str, data: &[u8], method: u16) -> Vec<u8> {
        let mut b = vec![0u8; 30];
        b[0..4].copy_from_slice(b"PK\x03\x04");
        b[8..10].copy_from_slice(&method.to_le_bytes());
        b[18..22].copy_from_slice(&(data.len() as u32).to_le_bytes());
        b[22..26].copy_from_slice(&(data.len() as u32).to_le_bytes());
        b[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
        b.extend_from_slice(name.as_bytes());
        b.extend_from_slice(data);
        b
    }
    #[test]
    fn accepts_txt_markdown_and_json() {
        for (name, body) in [
            ("a.txt", "plain"),
            ("a.MD", "# title"),
            ("a.json", "{\"ok\":true}"),
        ] {
            assert_eq!(extract(name, body.as_bytes()).unwrap(), body)
        }
    }
    #[test]
    fn rejects_unknown_invalid_utf8_and_oversize() {
        assert!(matches!(
            extract("a.csv", b"x"),
            Err(ImportError::Unsupported(_))
        ));
        assert!(matches!(
            extract("a.txt", &[0xff]),
            Err(ImportError::Invalid(_))
        ));
        assert!(matches!(
            extract("a.txt", &vec![0; 20 * 1024 * 1024 + 1]),
            Err(ImportError::Invalid(_))
        ));
    }
    #[test]
    fn extracts_stored_docx_xml_and_entities() {
        let xml=b"<w:document><w:p><w:t>A &amp; B</w:t></w:p><w:p><w:t>&lt;C&gt;</w:t></w:p></w:document>";
        assert_eq!(
            extract("a.docx", &stored_docx("word/document.xml", xml, 0)).unwrap(),
            "A & B\n<C>"
        );
    }
    #[test]
    fn rejects_corrupt_and_unsupported_docx() {
        assert!(matches!(
            extract("a.docx", b"not zip"),
            Err(ImportError::Corrupt(_))
        ));
        assert!(matches!(
            extract("a.docx", &stored_docx("word/document.xml", b"x", 99)),
            Err(ImportError::Unsupported(_))
        ));
        let mut truncated = stored_docx("word/document.xml", b"hello", 0);
        truncated.pop();
        assert!(matches!(
            extract("a.docx", &truncated),
            Err(ImportError::Corrupt(_))
        ));
    }
    #[test]
    fn extracts_pdf_literals_and_rejects_bad_header() {
        assert_eq!(
            extract("a.pdf", b"%PDF-1.7\n(Hello\\) world) Tj").unwrap(),
            "Hello) world"
        );
        assert!(matches!(
            extract("a.pdf", b"no pdf"),
            Err(ImportError::Corrupt(_))
        ));
    }
    #[test]
    fn extracts_deflated_docx_and_rejects_bad_stream() {
        use flate2::{write::DeflateEncoder, Compression};
        use std::io::Write;
        let xml = b"<w:p><w:t>Compressed</w:t></w:p>";
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut bytes = stored_docx("word/document.xml", &compressed, 8);
        bytes[22..26].copy_from_slice(&(xml.len() as u32).to_le_bytes());
        assert_eq!(extract("a.docx", &bytes).unwrap(), "Compressed");
        assert!(matches!(
            extract(
                "a.docx",
                &stored_docx("word/document.xml", &[0xff, 0xff, 0xff], 8)
            ),
            Err(ImportError::Corrupt(_))
        ));
    }
    #[test]
    fn docx_entry_count_and_total_expansion_are_limited() {
        let entry = stored_docx("other", b"", 0);
        let many = entry.repeat(257);
        assert!(
            matches!(extract("a.docx",&many),Err(ImportError::Invalid(v))if v.contains("many"))
        );
        let mut total = Vec::new();
        for _ in 0..17 {
            let mut item = stored_docx("other", b"", 0);
            item[22..26].copy_from_slice(&(1024u32 * 1024).to_le_bytes());
            total.extend(item);
        }
        assert!(
            matches!(extract("a.docx",&total),Err(ImportError::Invalid(v))if v.contains("total"))
        );
    }
    #[test]
    fn docx_skips_prefix_and_rejects_non_utf8_xml() {
        let mut prefixed = b"junk".to_vec();
        prefixed.extend(stored_docx("word/document.xml", b"<w:t>ok</w:t>", 0));
        assert_eq!(extract("a.docx", &prefixed).unwrap(), "ok");
        assert!(matches!(
            extract("a.docx", &stored_docx("word/document.xml", &[0xff], 0)),
            Err(ImportError::Corrupt(_))
        ));
    }
}
