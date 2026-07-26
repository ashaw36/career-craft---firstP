# Windows release and rollback

## Release gate

Only a draft GitHub Release may be created automatically. Promotion to a public release requires all required Windows CI jobs to pass and a human to verify the Win10/Win11 matrix results. The optional desktop WebDriver probe is currently an upstream diagnostic: WebView2 150 creates the embedded session but DOM commands time out. It is not recorded as passed and does not replace manual desktop acceptance.

Required repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: updater signing private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password.
- `WINDOWS_CERTIFICATE_BASE64`: base64-encoded PFX for Authenticode signing.
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX password.

The workflow fails before packaging if any signing secret is absent. It imports the PFX into the current-user certificate store, generates a release-only Tauri config with its thumbprint, and removes the certificate after the job. The release overlay enables updater artifacts; ordinary local and CI builds keep `createUpdaterArtifacts=false`. Production uses only the `default` capability and has `devtools=false`; WDIO capability and the E2E bridge exist only in the E2E overlay/build mode.

The release workflow is tag-only (`v*`) and uses the protected `production-release` environment. Its third-party actions currently use their official major-version tags; repository governance must use Dependabot for GitHub Actions and review upstream release notes before updating those refs. Commit-SHA pinning should be applied once the repository owner approves the exact upstream commits; no unverified SHA is recorded here.

## Promotion checklist

1. Confirm the tag and Tauri/package versions match.
2. Download the workflow evidence artifact and verify `SHA256SUMS.txt` against the NSIS installer.
3. Inspect `careercraft.cdx.json` and the GitHub build-provenance attestation.
4. Confirm the NSIS installer, updater bundle, `.sig`, and `latest.json` are present in the draft release.
5. Run `scripts/windows_vm_matrix.ps1` on clean Windows 10 and Windows 11 VMs. Attach both `result.json` files and logs to the release evidence.
6. Install, launch, create data, close/reopen, uninstall, reinstall, and confirm user data behavior follows the product policy.
7. Publish only after product owner approval.

## Rollback

1. Mark the faulty release as a prerelease or remove it from “latest”; do not delete evidence.
2. Restore `latest.json` to the last approved signed version and verify its referenced bundle and signature still exist.
3. Publish a new patch version when possible. Never reuse a tag or overwrite a signed artifact in place.
4. For urgent rollback, direct users to the prior NSIS installer and record whether database migrations are backward compatible. If compatibility is unknown, require a CareerCraft backup before downgrade.
5. Record incident tag, hashes, affected versions, database schema version, decision owner, and recovery validation in the release notes.

Rollback is complete only after a clean Win10/Win11 install and a preserved-user-data restart have been observed. Results must be attached; do not infer them from CI.
