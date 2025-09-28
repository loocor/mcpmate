# MCPMate Desktop Release Guide

This note captures the agreed flow for producing and shipping the Tauri desktop build. It replaces the
old “embed board/dist into the backend binary” workflow and documents the assets and scripts that now
drive the release process.

## 1. Prerequisites

| Component | Version / Notes |
| --- | --- |
| Node.js + npm | Same toolchain used by the board workspace |
| Rust toolchain | Stable 1.79+ with the required targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, optional `aarch64-pc-windows-msvc`) |
| Tauri CLI | `cargo install tauri-cli@2` |
| Optional | `tauri signer` (installed together with Tauri CLI) for update signatures |

The desktop release now depends on the Tauri shell; the backend crate never bundles the board UI directly.

## 2. Local Build Workflow (macOS Universal)

```bash
# From repo root
cd backend/tauri

# 1. Build dashboard assets
npm --prefix ../../board run build

# 2. Build universal macOS bundle (Release)
CI=true cargo tauri build \
  --target universal-apple-darwin \
  --bundles dmg

# Result:
#   target/universal-apple-darwin/release/bundle/macos/MCPMate.app
#   target/universal-apple-darwin/release/bundle/dmg/MCPMate_<version>_universal.dmg
```

Notes:

- `CI=true` skips the Finder AppleScript that fails on macOS 26 because of tightened permissions.
- The DMG already contains the refreshed icon and the embedded MCPMate backend. No extra static files
  are copied from the backend project anymore.

## 3. Windows Bundles

Run the steps on a Windows build agent (or cross-compile with the appropriate MSVC toolchains installed):

```powershell
cd backend\tauri
npm --prefix ..\..\board run build

cargo tauri build --release `
  --target x86_64-pc-windows-msvc `
  --bundles msi

# Optional: ARM64
cargo tauri build --release `
  --target aarch64-pc-windows-msvc `
  --bundles msi
```

Artifacts are written to `target/<triple>/release/bundle/{msi,nsis}/`.

## 4. Automatic Update Pipeline

The updater is enabled via `tauri.conf.json > plugins.updater`. A release requires:

1. **Desktop bundles** for each platform (see above).
2. **Ed25519 signatures** per bundle (`tauri signer sign --private-key signing-key.pem <bundle>`).
3. **Update manifest JSON** that references the bundle URLs and signatures.
4. Hosting the manifest + bundles over HTTPS (CDN/object storage is sufficient).

Scripts in `script/` help automate these steps:

- `script/build-tauri-release.sh` &mdash; orchestrates the board build and Tauri bundling for selectable targets.
- `script/generate-update-manifest.sh` &mdash; creates an updater manifest JSON from command-line inputs.

Integration tip: run the scripts from CI after tagging a release. Upload the produced bundles to the
configured CDN, generate the manifest, then publish both assets. The desktop client will discover the
new version automatically on next launch.

## 5. Release Checklist

1. Update `package.json` and `tauri.conf.json` versions.
2. Run the build script(s) for the desired platforms.
3. Sign each bundle and capture the signature output.
4. Generate the update manifest with the new version, download URLs, and signatures.
5. Upload bundles + manifest to the CDN.
6. (macOS) Codesign & notarize the `.app`/`.dmg` if distributing beyond internal testing.
7. Smoke-test the released DMG/MSI.

## 6. References

- [Tauri Updater Guide](https://tauri.app/v2/guides/distribution/updater/)
- [Tauri Signing](https://tauri.app/v2/guides/distribution/signing/)
- Project scripts under `backend/tauri/script/`

