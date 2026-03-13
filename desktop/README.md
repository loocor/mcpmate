# MCPMate

A native Swift & SwiftUI macOS application for managing MCP (Model Context Protocol) servers with system tray integration.

## ✨ Features

- **🖥️ Native macOS App**: Built with Swift 5.0 and SwiftUI
- **📋 System Tray Menu**: Complete menu bar integration with status indicators
- **🔄 Server Management**: Real-time monitoring and control of MCP servers
- **⚡ Quick Actions**: Start, stop, and refresh servers from the menu bar
- **🎯 Modern UI**: Clean, minimal interface with SwiftUI components
- **📊 Status Monitoring**: Live connection status and server count display
- **🔒 App Sandbox**: Secure sandboxed environment
- **🎨 Material Design**: Uses system materials and SF Symbols

## 🎯 Design Philosophy

### **"The best tool should be invisible"**
MCPMate follows the core design philosophy: an excellent tool should blend seamlessly into the system, making it feel like a native feature rather than an external application.

### **Core principles**
- **🌊 Seamless integration**：Blend seamlessly into the macOS system, following Human Interface Guidelines
- **🧠 Minimal cognitive load**：Users don't need to "switch mental modes," and the operation is intuitive and natural
- **⚡ Native experience**：Use system fonts, colors, and icons, and support platform-specific interactions
- **👻 Low presence**：The tool is powerful but has a low presence, like breathing naturally

### **Technical implementation philosophy**
- **🏗️ Multi-platform native strategy**：Choose native development over cross-platform solutions to achieve true platform integration
- **🔗 Decoupled architecture**：Backend Rust + frontend native UI, laying the groundwork for future multi-platform expansion
- **🎨 Platform consistency**：Maintain consistent functionality but adapt to the design and interaction habits of each platform

## 🏗️ System Tray Menu Features

### Status Display
- Real-time connection status indicator
- Active server count display
- Connection health monitoring

### Server Management
- Start/Stop individual servers
- Start/Stop all servers at once
- Real-time server status updates
- Port information display

### Quick Actions
- Refresh servers (⌘R)
- Open configuration (⌘,)
- Toggle main window visibility
- Launch at login option

### System Integration
- Background operation support
- Native macOS menu bar integration
- Keyboard shortcuts support
- About and Help sections

## 📋 Requirements

- macOS 13.0 or later
- Xcode 14.0 or later
- Swift 5.0

## 🚀 Getting Started

1. Open `MCPMate.xcodeproj` in Xcode
2. Select Mac as target device
3. Press ⌘R to build and run the application
4. **Menu Bar**: Look for the network icon in your menu bar
5. **Main Window**: Click "Show Window" from menu or launch normally

## 📁 Project Structure

```
MCPMate/
├── MCPMate.xcodeproj/          # Xcode project file
└── MCPMate/                    # Source code
    ├── MCPMate.swift           # App entry point with background support
    ├── MenuBarManager.swift    # System tray menu management
    ├── ContentView.swift       # Main window interface
    ├── MCPMate.entitlements    # App permissions
    ├── Assets.xcassets/        # App assets
    └── Preview Content/        # Preview assets
```

## 🔧 Technical Implementation

### MCPMate.swift
- Main application entry point
- Background operation support (`NSApp.setActivationPolicy(.accessory)`)
- Integration with MenuBarManager
- Window management and lifecycle

### MenuBarManager.swift
- System status bar item management
- Dynamic menu generation with server status
- Action handlers for all menu operations
- Real-time status updates and menu refresh

### ContentView.swift
- Main application window interface
- Server list display with status indicators
- Interactive server controls
- Integration with menu bar manager for status updates

## ⚙️ Configuration

- **Bundle Identifier**: `io.mcpmate.app`
- **Deployment Target**: macOS 13.0
- **Architectures**: Apple Silicon (arm64) and Intel (x86_64) — shipped as separate builds (no universal binary)
- **Swift Version**: 5.0

## 🎯 Usage Scenarios

### Menu Bar Only Mode
- Launch the app to run in background
- Access all functionality via menu bar icon
- Minimal system resource usage
- Perfect for continuous server monitoring

### Full Window Mode
- Open main window for detailed server management
- Interactive server controls and status display
- Visual server configuration and monitoring
- Ideal for setup and configuration tasks

## 🛠️ Development Notes

- The app supports both dock and menu-bar-only operation modes
- System tray integration follows macOS Human Interface Guidelines
- All menu actions are implemented with proper target-action patterns
- Real-time updates use SwiftUI's reactive data binding

## 📝 Future Enhancements

- [ ] Real MCP server integration
- [ ] Configuration file management
- [ ] Server logs viewing
- [ ] Custom server configurations
- [ ] Auto-start server management
- [ ] System notifications for server events

---

Built with ❤️ using Swift & SwiftUI for efficient MCP server management on macOS.
