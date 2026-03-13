# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCPMate desktop is a native macOS application built with Swift/SwiftUI that provides a graphical interface for managing MCP (Model Context Protocol) servers. It's part of the larger MCPMate ecosystem that includes a Rust backend, React web dashboard, and cross-platform support.

## Architecture & Key Components

### Core Architecture
- **Backend Integration**: Uses Rust FFI (Foreign Function Interface) via `libmcpmate.dylib`
- **SwiftUI Native UI**: Modern declarative interface following macOS Human Interface Guidelines
- **Operation Modes**: Three distinct modes (Auto/Hosted/Transparent) for different runtime behaviors
- **Service Management**: Real-time backend lifecycle management with progress monitoring

### Key Files & Directories
```
desktop/macos/
├── mcpmate/                    # Swift source code
│   ├── MCPMate.swift          # App entry point
│   ├── Shared/MCPMateFFI.swift # FFI bindings & configuration
│   ├── Common/                # Shared utilities & services
│   │   ├── Services/
│   │   │   ├── ServiceManagerFFI.swift    # Backend lifecycle management
│   │   │   └── OperationModeManager.swift # Mode switching logic
│   │   └── Models/            # Data models for API communication
│   ├── Pages/                 # Main application views
│   │   ├── Dashboard/         # System overview & metrics
│   │   ├── ClientApps/        # MCP client management
│   │   └── ConfigSuits/       # Configuration management
│   └── scripts/               # Build automation
```

## Development Commands

### Quick Start
```bash
# One-click complete build & release
./desktop/macos/scripts/release.sh

# Development build (backend + desktop)
./desktop/macos/scripts/01-build-backend.sh
./desktop/macos/scripts/02-prepare-desktop.sh
./desktop/macos/scripts/03-build-desktop.sh
```

### Build Options
```bash
# Clean build
./release.sh --clean

# Skip specific steps
./release.sh --skip-backend --skip-dmg

# Development mode
open desktop/macos/mcpmate.xcodeproj
```

### Individual Scripts
- `01-build-backend.sh` - Builds Rust backend with FFI
- `02-prepare-desktop.sh` - Copies backend artifacts to macOS project
- `03-build-desktop.sh` - Builds macOS app with Xcode
- `04-create-dmg.sh` - Creates distributable DMG installer

## FFI Integration

### Backend Communication
- **Library**: `libmcpmate.dylib` (built from Rust backend)
- **Configuration**: JSON-based communication via C FFI
- **Ports**: Configurable API (8080) and MCP (8000) ports via UserDefaults
- **Startup Modes**: Minimal (API-only), Default (with config suites), Custom (specific suites)

### Key FFI Functions
```swift
// Service lifecycle
MCPMateEngine.start(with: PortConfig)
MCPMateEngine.startMinimal(apiPort: UInt16)
MCPMateEngine.startDefault(apiPort:mcpPort:)
MCPMateEngine.startWithSuites(apiPort:mcpPort:suites:)

// Status monitoring
MCPMateEngine.getStartupProgress() -> StartupProgress
MCPMateEngine.getServiceInfo() -> ServiceInfo
```

## Configuration System

### UserDefaults Keys
- `MCPMate.apiPort` - API server port (default: 8080)
- `MCPMate.mcpPort` - MCP proxy port (default: 8000)
- `MCPMate.operationMode` - Current operation mode
- `MCPMate.hasCustomConfig` - Flag for custom configuration

### Operation Modes
- **Auto**: Load active config suites, sync based on client preferences
- **Hosted**: Force hosted mode for all clients
- **Transparent**: Stop upstream services, force transparent mode

## Build Artifacts

### Backend Outputs
- `libmcpmate.dylib` - FFI dynamic library
- `bridge` - HTTP to stdio bridge
- `mcpmate` - CLI tool

### Desktop Outputs
- `MCPMate.app` - macOS application bundle
- `MCPMate-Installer-YYYY.MM.DD.dmg` - Distributable installer

## Development Workflow

### Standard Process
1. **Backend Changes**: Modify Rust backend in `backend/` directory
2. **Build Backend**: Run `01-build-backend.sh` with FFI features
3. **Update Desktop**: Run `02-prepare-desktop.sh` to sync artifacts
4. **Test Changes**: Build and run from Xcode or use `03-build-desktop.sh`
5. **Release**: Use `release.sh` for complete build pipeline

### Debugging
- **Port Conflicts**: Check with `lsof -i :8080` or `lsof -i :8000`
- **FFI Issues**: Verify `libmcpmate.dylib` exists in app bundle
- **Backend Logs**: Check console output in Dashboard page
- **Build Issues**: Use `--clean` flag for fresh builds

## Integration Points

### API Endpoints
- `GET /api/system/status` - Service health check
- `GET /api/system/info` - System information
- Configuration management via RESTful APIs

### Environment Setup
- **PATH**: Scripts ensure Homebrew paths are included
- **Runtime Dependencies**: Automatic detection of `uvx`, `npx` commands
- **Port Validation**: Built-in port availability checking

## Testing & Validation

### Manual Testing
1. Launch app from Xcode or built `.app`
2. Verify backend starts (check Dashboard for connection status)
3. Test operation mode switching
4. Validate configuration changes persist
5. Test port configuration changes

### Automated Testing
- Unit tests: `mcpmateTests` target
- UI tests: `mcpmateUITests` target
- Integration tests via Xcode build system