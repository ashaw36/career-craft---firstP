# Document export implementation

Scope: CC-FR-006/007/018. The implementation contains no Python, Qt, browser automation, bundled Chromium, remote script, or CDN.

## Shipping decision

Markdown export is deterministic UTF-8. PDF export is a dependency-free Rust implementation using a PDF Type0/CIDFontType2 font, `Identity-H` encoding, an explicit ToUnicode CMap, and an embedded TrueType `FontFile2` stream.

The exporter discovers an installed Windows CJK font at runtime in this order:

1. DengXian (`Deng.ttf`)
2. SimHei (`simhei.ttf`)
3. Noto Sans SC (`NotoSansSC-VF.ttf`)
4. SimSun Bold (`simsunb.ttf`)

It reads the TrueType cmap and horizontal metrics tables, maps every Unicode character to a glyph, and checks the OS/2 `fsType` restricted-embedding bit before selecting a font. Missing fonts, restricted embedding, unsupported collections, malformed tables, or missing glyphs return a visible export error. Chinese text is never silently replaced or emitted as mojibake.

## Size and licensing

- Installer/application binary delta: effectively zero; there is no font asset and no new crate.
- Generated PDF size: typically 10–18 MB because the installed font is embedded in full. This is an accepted first production implementation; font subsetting is a later optimization behind the same `DocumentExporter` port.
- License handling: the exporter respects the font's machine-readable restricted-embedding flag. It does not redistribute or install the system font. Windows images without a permitted candidate receive an actionable error.

This avoids adding a 5–20 MB CJK font to every installer while keeping exported PDFs portable. Release testing must cover clean supported Windows 10/11 images, not only developer machines.

## Command behavior

- `generateResume`: loads the persona and confirmed experiences from SQLite, aggregates common render data, supports all five stable template IDs, renders Markdown, and creates an immutable in-session version.
- `chatRefineResume`: returns a confirmation-required proposal first. Only `confirm=true` with reviewed wording creates a child version. `undo`, `restore`, `撤销`, or `恢复` creates a restored version without overwriting history.
- `exportResumePDF`: exports the latest version, or aggregates current data when no version exists, and returns RFC 4648 base64 plus a filename.
- Versions are persisted as immutable structured snapshots in SQLite migration 0003. They survive process restarts, retain parent links and A/B diff data, and are transactionally capped at the newest five versions per persona.

## DOCX

DOCX export is not required by CC-FR-006. The replaceable port keeps it possible. If product scope adds it, evaluate `docx-rs` and measure its ZIP/XML dependency and binary delta; do not add an Office runtime.

## Verification

`cargo test --offline` passes 44 tests, including Chinese PDF export through an installed system font, all five templates, version limit, confirmation/undo domain behavior, Markdown UTF-8, and base64 encoding.
