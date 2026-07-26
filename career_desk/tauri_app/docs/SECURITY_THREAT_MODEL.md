# CareerCraft desktop threat model

Scope: the signed Windows desktop application, local SQLite data, imported documents, user-triggered URLs, LLM provider traffic, updater metadata, and credentials. Browser-only development servers and CI signing infrastructure are outside the runtime trust boundary.

| Asset / boundary | Threat | Control and evidence | Residual / dependency |
|---|---|---|---|
| Provider API keys | plaintext database/config/log disclosure | Windows Credential Manager only; database stores opaque target; response exposes `hasKey`, never key; migration and redaction tests | Real Credential Manager E2E requires Windows release validation |
| Tauri IPC | arbitrary native invocation | compile-time invoke handler plus minimal `core:default` capability; security gate compares registered commands to versioned manifest | New commands require manifest review |
| WebView content | remote script/XSS/devtools | production `devtools:false`; CSP default/script self, no CDN/eval; bundled frontend scan rejects remote scripts | W6 desktop CSP evidence remains release evidence |
| URL collection | SSRF, DNS rebinding, credential URL | user-triggered command; HTTP(S)-only parsing, credential rejection, resolved private/link-local/metadata rejection, no redirects, timeout/size cap; manual fallback | Public sites may block automation |
| External links | arbitrary shell/open | validated URL is bound to an expiring single-use in-memory token | OS browser behavior requires E2E |
| Local database | corruption / rollback / disclosure | WAL, transactions, migrations with pre-migration backup and restore tests, retention; W7 forced-process crash gate | Database is not encrypted; see ADR below |
| AI content | prompt/content leakage and fabricated facts | minimum operation-specific prompt, no prompt logging/cache body, preview/confirmation protocols | User chooses provider and accepts its privacy policy |
| Update channel | malicious manifest/package, interrupted apply | HTTPS metadata, pinned public-key signature verification delegated to Tauri updater, state machine rejects bad metadata and records rollback | Production endpoint, Minisign private key custody and Authenticode certificate are external release dependencies |

## Database encryption ADR

Decision: SQLite/SQLCipher encryption is **not implemented in this release**. API keys remain outside SQLite, but resumes, experiences and job data are plaintext within the current user's application-data ACL. This is an explicit residual confidentiality risk for a compromised Windows account, administrator, malware, or unencrypted disk.

Rationale: silently claiming encryption or inventing a machine-derived key would provide weak protection and complicate backup recovery. A production encryption change requires SQLCipher distribution review, a recovery-key/user-secret design, encrypted backup migration, performance measurements, and destructive-loss UX. Until that project is approved, release documentation must disclose local plaintext business data and recommend BitLocker/device protection. High-sensitivity enterprise deployment may treat this as a release blocker.

## Logging rule

Production diagnostics must contain error families and operation IDs only. Prompts, resumes, imported content, authorization headers, API keys, passwords, update private keys and URL credentials are forbidden. Central redaction is defense in depth, not permission to log sensitive input.
