# MCPMate Desktop Dependencies Analysis: Windows/Linux/macOS Support

**Date:** April 14, 2026  
**Scope:** `desktop/src-tauri/Cargo.toml` and runtime code  
**Objective:** Identify platform-specific crates, Tauri plugins, sidecar assumptions, and blocking issues for Windows/Linux GitHub Actions builds.

---

## Executive Summary

MCPMate Desktop uses **Tauri 2** with **macOS-primary assumptions**. The codebase has **significant macOS-only features** (OAuth/Keychain, runtime environment setup, shell preferences) and **Windows-specific service level requirements**. Linux support is minimally guarded but untested. **No blocking dependencies prevent Windows/Linux packaging**, but several **runtime features degrade gracefully to stubs on non-macOS platforms**.

---

## 1. Dependencies Overview

### 1.1 Core Tauri & Plugins

**File:** `desktop/src-tauri/Cargo.toml` (lines 13, 19-22)

| Dependency | Version | Platform | Notes |
|---|---|---|---|
| `tauri` | 2 | All | Core framework; features: `tray-icon`, `wry`, `image-png` |
| `tauri-plugin-updater` | 2 | All | Auto-updater plugin (disabled by default in config) |
| `tauri-plugin-dialog` | 2 | All | Native dialogs |
| `tauri-plugin-opener` | 2 | All | External link/URL opening |
| `tauri-plugin-clipboard-manager` | 2 | All | Clipboard access |
| `tauri-plugin-deep-link` | 2 | All | `mcpmate://` URL scheme handling |

**Verdict:** All plugins are **cross-platform compatible**. No blockers here.

### 1.2 Backend Integration

**File:** `desktop/src-tauri/Cargo.toml` (line 18)

```toml
mcpmate = { path = "../../backend" }
```

Embeds the MCPMate backend as a workspace dependency. Backend analysis is separate; assumes no platform-blocking deps there.

### 1.3 macOS-Only Dependencies

**File:** `desktop/src-tauri/Cargo.toml` (lines 47-49)

```toml
[target.'cfg(target_os = "macos")'.dependencies]
keyring = { version = "3", features = ["apple-native"] }
plist = "1.1"
```

- **`keyring` (3.x, `apple-native` feature):** Secures macOS Keychain integration for OAuth JWT storage. Used by **`src/account.rs`**.
- **`plist` (1.1):** macOS property list parsing. Used by Tauri's macOS integration.

**Impact:** These crates **do not compile on Windows/Linux** but are guarded by `#[cfg(target_os = "macos")]` throughout the code. **No build failure expected** on other platforms.

---

## 2. Platform-Specific Code Analysis

### 2.1 Account Management (`src/account.rs`)

**Scope:** Lines 1–167; OAuth & JWT handling

| Function | macOS | Windows | Linux | Notes |
|---|---|---|---|---|
| `read_jwt()` | ✅ Keychain | ❌ Error | ❌ Error | Stored in macOS Keychain; Windows/Linux return error stub |
| `store_jwt()` | ✅ Keychain | ❌ Error | ❌ Error | Writes to Keychain; non-macOS return error |
| `delete_jwt()` | ✅ Keychain | ❌ Error | ❌ Error | Deletes from Keychain; non-macOS return error |
| `start_github_login()` | ✅ OAuth → browser | ❌ Stub error | ❌ Stub error | Opens browser; non-macOS: "Account linking is only available on macOS." |
| `get_status()` | ✅ Full details | ❌ Error | ❌ Error | Returns device ID & login status; non-macOS error |
| `logout()` | ✅ Clears JWT | ❌ Error | ❌ Error | Non-macOS: "Account logout is only available on macOS." |

**Guard locations:**
- Lines 31–42: `#[cfg(target_os = "macos")]` for `read_jwt()` + stub
- Lines 44–55: macOS `store_jwt()` / `delete_jwt()` + non-macOS stubs
- Lines 85–97: `start_github_login()` platform variants
- Lines 99–121: `get_status()` and `logout()` with platform guards

**Risk:** Account linking is **disabled on non-macOS**. Frontend must handle gracefully.

---

### 2.2 Core Service Management (`src/core_service.rs`)

**Scope:** Lines 1–481; service installation, start/stop, status

| Feature | macOS | Windows | Linux | Notes |
|---|---|---|---|---|
| Service level | `User` | `System` | `User` | Windows installs system-wide; Unix (macOS/Linux) use user-level services |
| Service manager | Native (launchd/systemd/SCM) | SCM (Windows Service Manager) | systemd/launchd | Via `service-manager` crate |
| Binary resolution | ✅ Multiple paths | ✅ `.exe` suffix | ✅ No suffix | `resolve_local_core_binary()` detects `EXE_SUFFIX` |

**Key code (lines 44–54):**
```rust
pub fn resolve_service_level() -> ServiceLevel {
    #[cfg(target_os = "windows")]
    {
        ServiceLevel::System
    }

    #[cfg(not(target_os = "windows"))]
    {
        ServiceLevel::User
    }
}
```

**Sidecar detection (lines 78–134):**
- **macOS:** Checks `MCPMate.app/Contents/MacOS/` (standard app bundle), then `Resources/`
- **Windows/Linux:** Checks workspace `target/` directories in debug mode

**Risk:** Windows service installation requires **admin privileges** (system-level). Linux requires systemd or compatible init. Tauri + `service-manager` should handle this, but **manual testing required**.

---

### 2.3 Runtime Environment (`src/runtime_env.rs`)

**Scope:** Lines 1–189; shell/PATH setup

| Feature | macOS | Windows | Linux | Notes |
|---|---|---|---|---|
| PATH customization | ✅ Custom bin/ + runtimes | ❌ (empty) | ❌ (empty) | Creates shims for `npx`, `python3` on macOS only |
| Bun/Node shims | ✅ Fallback to system | ❌ Skipped | ❌ Skipped | Injects Bun-first shims; non-macOS: no modification |
| HOME env var | ✅ Propagated | ❌ | ❌ | Ensures HOME is set for service spawning |

**Lines with guards (lines 36–57):**
```rust
fn desktop_runtime_environment() -> BTreeMap<String, String> {
    #[cfg(target_os = "macos")]
    {
        // Custom PATH + HOME setup
    }

    #[cfg(not(target_os = "macos"))]
    {
        BTreeMap::new()  // No custom environment
    }
}
```

**Risk:** **Low**. Windows/Linux simply skip custom PATH setup; services inherit system environment. May degrade perf if Bun/uv are not on system PATH, but not a failure.

---

### 2.4 Shell Management (`src/shell.rs`)

**Scope:** Lines 1–363; tray icon, menu bar, dock integration

| Feature | macOS | Windows | Linux | Notes |
|---|---|---|---|---|
| Tray icon template | ✅ Monochrome + template | ❌ Standard icon | ❌ Standard icon | macOS uses `icon_as_template=true` for light/dark tint |
| Dock icon visibility | ✅ Can hide dock | ❌ N/A | ❌ N/A | Menu bar accessory mode (lines 648–652 in lib.rs) |
| Menu bar icon mode | ✅ Dock/menu bar toggle | ❌ Fallback to static | ❌ Fallback to static | macOS-only preference; enforced constraints (lines 118–122) |

**Lines with guards:**
- Lines 19–24: `set_tray_icon_with_template()` calls `icon_as_template(true)` only on macOS
- Shell preferences enforce: if `!show_dock_icon`, `menu_bar_icon_mode` reset to `Runtime` (line 120)

**Activation policy (lib.rs lines 361, 1083–1087):**
```rust
#[cfg(target_os = "macos")]
{
    builder = builder
        .title_bar_style(tauri::TitleBarStyle::Transparent)
        .hidden_title(true);
}
```

**Risk:** **Minimal**. Menu bar visibility preference is macOS-only; Windows/Linux will show standard taskbar tray. UI must not expose dock-toggle on non-macOS.

---

### 2.5 Deep Link Handling (`src/deep_link.rs`)

**Scope:** `mcpmate://` URL scheme routing for OAuth callback + server import

**Platform support:** ✅ **All platforms**

Tauri's `tauri-plugin-deep-link` handles registration and routing uniformly. The `mcpmate://auth` and `mcpmate://import/server` flows work identically on all platforms. ✅ No platform guards needed.

---

## 3. Tauri Configuration (`tauri.conf.json`)

**File:** `desktop/src-tauri/tauri.conf.json` (lines 1–70)

### 3.1 Build Configuration

```json
"bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": ["icons/icon.icns"],
    "externalBin": [
        "../../backend/target/sidecars/bridge",
        "../../backend/target/sidecars/mcpmate-core"
    ],
    "macOS": { ... }
}
```

**Issue:** 
- `targets: ["app", "dmg"]` is macOS-only bundle format. 
- `icon.icns` is macOS format.
- **Windows/Linux requires `msi`/`deb`/`rpm` targets respectively.**
- **Tauri build script (`build.rs`) does not conditionally adjust bundle types** — this is OK; `tauri build` auto-selects based on host OS.

**Bridge/Core sidecars:**
- Both binaries are copied during build (lines 43–44)
- `externalBin` directive tells Tauri where to find them at bundle time
- **Windows will look for `.exe` suffix automatically** ✅

---

## 4. Sidecar Binary Requirements

### 4.1 Bridge Sidecar (`bridge` binary)

**Built from:** `backend/src/bin/bridge.rs`  
**Purpose:** Connects stdio-mode MCP clients to HTTP proxy  
**Platform dependency:** **None detected** — uses standard Rust + Tokio

**Deployed via:** `tauri.conf.json` `externalBin` → copied from `backend/target/sidecars/bridge` or `bridge.exe` on Windows

### 4.2 Core Sidecar (`mcpmate-core`, alias `mcpmate`)

**Built from:** `backend/src/main.rs`  
**Purpose:** Main MCPMate daemon (API + MCP servers)  

**Binary resolution (core_service.rs, lines 78–134):**
```
Candidates (in order):
1. MacOS directory (for bundle): MCPMate.app/Contents/MacOS/mcpmate-core[-{target}]{.exe}
2. Resources directory: {resource_dir}/mcpmate-core[-{target}]{.exe}
3. Debug (workspace): backend/target/{target}/debug/mcpmate{.exe}
4. Sidecar directory: backend/target/sidecars/mcpmate-core[-{target}]{.exe}
```

**Windows/Linux:** Will use `.exe` suffix and sidecar directory paths. ✅ No blocker.

---

## 5. Cargo & Build Targets

**File:** `desktop/src-tauri/build.rs` (lines 112–131)

```rust
fn backend_build_context() -> BackendBuildContext {
    let target = env::var("TARGET").expect("TARGET");
    let exe_suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    // ...
}
```

**Detects Windows and applies `.exe` suffix correctly** ✅

---

## 6. Summary Table: Platform Support

| Capability | macOS | Windows | Linux | Notes |
|---|---|---|---|---|
| **Build** | ✅ | ✅ | ✅ | No blocking crate deps |
| **Package** | ✅ DMG | ✅ MSI | ✅ AppImage/deb | Tauri auto-selects |
| **Desktop Shell** | ✅ Full | ✅ Taskbar tray | ✅ Taskbar tray | macOS menus; others standard |
| **Service Install** | ✅ launchd (user) | ✅ SCM (system, needs admin) | ✅ systemd (user) | Windows needs UAC testing |
| **OAuth/Account** | ✅ Keychain JWT | ❌ Error stub | ❌ Error stub | Needs fallback on non-macOS |
| **Runtime Setup** | ✅ Custom PATH | ❌ System PATH | ❌ System PATH | Non-critical |
| **Deep Link** | ✅ mcpmate:// | ✅ mcpmate:// | ✅ mcpmate:// | Uniform across platforms |
| **Sidecar Exec** | ✅ | ✅ | ✅ | Binary detection works |

---

## 7. Identified Risks & Next Steps

### ✅ No Build Blockers

- All macOS-only dependencies are properly gated with `#[cfg(...)]`
- Service manager crate supports Windows, Linux, macOS
- All Tauri plugins are cross-platform
- Build script correctly applies `.exe` suffix on Windows

### 🟡 Non-Critical Issues

1. **Account features disabled on non-macOS** — Frontend must hide account settings
2. **Windows service requires admin** — Document in setup wizard
3. **Linux requires systemd** — Verify on target distros
4. **JWT storage on non-macOS** — Currently errors; could add file-based fallback for feature parity

### ✅ Ready for CI/CD

Windows and Linux builds should succeed immediately. Test sidecar binary discovery before release.

---

## Key Files to Review

| File | Purpose | Risk |
|---|---|---|
| `src/account.rs` | OAuth/Keychain | macOS-only; error stub on other platforms |
| `src/core_service.rs` | Service mgmt | Windows requires admin; Linux needs systemd |
| `src/runtime_env.rs` | PATH setup | macOS-specific; others skip gracefully |
| `src/shell.rs` | Tray/dock | macOS menus; others show standard taskbar |
| `build.rs` | Sidecar build | Correctly detects Windows `.exe` suffix |
| `tauri.conf.json` | Bundle config | `dmg` is macOS-only; Tauri auto-selects per OS |

