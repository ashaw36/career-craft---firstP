# Golden fixture protocol

Applies to DB-GOLD-001, API-CONTRACT-001..033, CC-FR-001..018 and CC-FR-023.

Fixtures are deterministic, synthetic and sanitized. Never copy API keys, names, employers, URLs, resumes or free text from a real user database. Dates use ISO-8601, UUIDs are fixed, JSON keys are sorted before hashing, and timestamps are normalized to UTC without fractional seconds.

Generation procedure:

1. Create a temporary SQLite database by executing `contracts/db/legacy_schema_v1.sql` with `PRAGMA foreign_keys=ON`.
2. Insert `legacy_rows.json` in dependency order. Null and empty JSON cases must both remain represented.
3. Record `sqlite_master` SQL, `PRAGMA table_info`, `foreign_key_list`, indexes, row counts, `foreign_key_check`, `integrity_check`, and canonical row hashes.
4. Open the fixture with the Rust migration build, back it up, migrate in one transaction, then compare all legacy columns and normalized values exactly.
5. For each command in `contracts/commands/v1/commands.json`, store success, domain-error and schema-invalid vectors. Errors compare stable codes, never stack traces.
6. Regeneration requires review because fixtures are frozen compatibility assets.

`test_cases.json` is the machine-readable acceptance inventory. Binary PDF/DOCX corpora belong to the document work package and must not be synthesized from private data.
