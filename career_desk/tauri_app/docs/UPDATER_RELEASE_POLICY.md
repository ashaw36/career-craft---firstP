# Signed updater release policy

The checked-in application does not contain a signing private key and does not claim a production updater is active. `src-tauri/updater.production.example.json` is the reviewed production overlay contract; `updater.test.json` is disabled and loopback-only.

Production activation requires all of the following external release inputs:

1. An owned HTTPS update endpoint and immutable signed manifest retention.
2. A generated Minisign keypair: only the public key is pinned in the production Tauri updater configuration. The private key is held in the release secret store and supplied only to the signing job.
3. An Authenticode certificate and timestamp service configured outside this repository.
4. A test-only build with the updater adapter wired to the `UpdateMachine` policy; bad signature/hash/URL tests must fail before staging.
5. Install/apply/relaunch verification and a retained previous installer. Apply failure invokes the explicit rollback path; schema compatibility must permit the previous application version to reopen the database.

Until those inputs exist, `createUpdaterArtifacts` remains false in the active `tauri.conf.json`. Replacing placeholders without signed end-to-end evidence is a release-blocking error, not completion.
