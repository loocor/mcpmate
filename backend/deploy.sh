#!/bin/bash

# MCPMate Deployment Package Creator
# Creates standalone deployment packages for different platforms

set -e

echo "🚀 MCPMate Deployment Builder"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --release              Build in release mode (optimized)"
    echo "  --target <TARGET>      Cross-compile for specific target"
    echo "  --platform <PLATFORM>  Create platform-specific package"
    echo "  --help                 Show this help message"
    echo ""
    echo "Supported targets:"
    echo "  x86_64-unknown-linux-gnu     Linux x64"
    echo "  aarch64-unknown-linux-gnu    Linux ARM64"
    echo "  x86_64-pc-windows-gnullvm    Windows x64"
    echo "  aarch64-pc-windows-gnullvm   Windows ARM64"
    echo "  x86_64-apple-darwin          macOS x64"
    echo "  aarch64-apple-darwin         macOS ARM64 (Apple Silicon)"
    echo ""
    echo "Examples:"
    echo "  $0                           Build for current platform (debug)"
    echo "  $0 --release                 Build for current platform (release)"
    echo "  $0 --platform windows        Build for Windows x64"
    echo "  $0 --target aarch64-apple-darwin --release"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "board" ]; then
    print_error "This script must be run from the MCPMate root directory"
    exit 1
fi

# Parse command line arguments
BUILD_MODE="debug"
TARGET=""
PLATFORM=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Convert platform shortcuts to targets
case "$PLATFORM" in
    linux|linux-x64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
    linux-arm64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
    windows|win64)
        TARGET="x86_64-pc-windows-gnullvm"
        ;;
    windows-arm64)
        TARGET="aarch64-pc-windows-gnullvm"
        ;;
    macos|macos-x64)
        TARGET="x86_64-apple-darwin"
        ;;
    macos-arm64)
        TARGET="aarch64-apple-darwin"
        ;;
    "")
        # No platform specified, use current
        ;;
    *)
        print_error "Unknown platform: $PLATFORM"
        show_usage
        exit 1
        ;;
esac

# Determine binary name and extension
BINARY_NAME="mcpmate"
if [[ "$TARGET" == *"windows"* ]]; then
    BINARY_NAME="mcpmate.exe"
fi

# Determine output directory name
if [ -n "$TARGET" ]; then
    DIST_DIR="dist/${TARGET}"
    if [ "$BUILD_MODE" = "release" ]; then
        DIST_DIR="${DIST_DIR}-release"
    fi
else
    DIST_DIR="dist/local"
    if [ "$BUILD_MODE" = "release" ]; then
        DIST_DIR="${DIST_DIR}-release"
    fi
fi

# Determine binary path
if [ -n "$TARGET" ]; then
    BINARY_PATH="target/${TARGET}/${BUILD_MODE}/${BINARY_NAME}"
else
    BINARY_PATH="target/${BUILD_MODE}/${BINARY_NAME}"
fi

print_status "Configuration:"
print_status "  Build mode: $BUILD_MODE"
print_status "  Target: ${TARGET:-current platform}"
print_status "  Output: $DIST_DIR"
print_status "  Binary: $BINARY_PATH"
echo ""

# Step 1: Build frontend
print_status "Building frontend..."
cd board

if [ ! -f "package.json" ]; then
    print_error "package.json not found in board directory"
    exit 1
fi

# Detect package manager
if command -v yarn &> /dev/null && [ -f "yarn.lock" ]; then
    PACKAGE_MANAGER="yarn"
elif command -v npm &> /dev/null; then
    PACKAGE_MANAGER="npm"
else
    print_error "Neither npm nor yarn found. Please install Node.js and npm/yarn."
    exit 1
fi

# Install and build
$PACKAGE_MANAGER install
$PACKAGE_MANAGER run build

if [ ! -f "dist/index.html" ]; then
    print_error "Frontend build failed - dist/index.html not found"
    exit 1
fi

cd ..

# Step 2: Build backend using platform-specific script
print_status "Building backend..."

SCRIPT_DIR="script"

# Resolve platform script from target/platform
PLATFORM_SCRIPT=""
if [ -n "$PLATFORM" ]; then
    case "$PLATFORM" in
        linux|linux-x64|linux-arm64)
            PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh"
            ;;
        windows|win64|windows-arm64)
            PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh"
            ;;
        macos|macos-x64|macos-arm64|darwin)
            PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh"
            ;;
        *)
            print_error "Unknown platform: $PLATFORM"
            show_usage
            exit 1
            ;;
    esac
elif [ -n "$TARGET" ]; then
    case "$TARGET" in
        *unknown-linux-gnu*) PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh" ;;
        *pc-windows-*)       PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh" ;;
        *apple-darwin*)      PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh" ;;
        *)
            print_error "Cannot determine platform for target: $TARGET"
            exit 1
            ;;
    esac
else
    # Auto-detect current host
    case "$(uname -s)" in
        Linux)  PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh" ;;
        Darwin) PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh" ;;
        CYGWIN*|MINGW*|MSYS*) PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh" ;;
        *) print_error "Unsupported host platform: $(uname -s)"; exit 1 ;;
    esac
fi

if [ ! -f "$PLATFORM_SCRIPT" ]; then
    print_error "Build script not found: $PLATFORM_SCRIPT"
    exit 1
fi

chmod +x "$PLATFORM_SCRIPT"

if [ -n "$TARGET" ]; then
    "$PLATFORM_SCRIPT" "$BUILD_MODE" "$TARGET"
else
    "$PLATFORM_SCRIPT" "$BUILD_MODE"
fi

if [ ! -f "$BINARY_PATH" ]; then
    print_error "Backend build failed - binary not found at $BINARY_PATH"
    exit 1
fi

# Step 3: Create deployment package
print_status "Creating deployment package..."

# Remove existing deployment directory
rm -rf "$DIST_DIR"

# Create deployment structure
mkdir -p "$DIST_DIR"

# Copy binary
cp "$BINARY_PATH" "$DIST_DIR/"

# Create platform-specific README
cat > "$DIST_DIR/README.md" << EOF
# MCPMate Deployment Package

This is a standalone deployment package for MCPMate.

## Package Information

- **Build Mode**: $BUILD_MODE
- **Target Platform**: ${TARGET:-current platform}
- **Binary**: $BINARY_NAME

## How to run:

1. Double-click the \`$BINARY_NAME\` executable (or run \`./mcpmate\` in terminal)
2. API endpoints will be available at http://localhost:8080

## What's included:

- \`$BINARY_NAME\` - The MCPMate server binary

## Requirements:

- No additional dependencies needed
- The server will serve both the API and web interface on port 8080
- Browser will open automatically when started

## Troubleshooting:

If API requests fail, verify:
1. Port 8080 is not being used by another application
2. Check the console output for any error messages
EOF

# Create platform-specific launcher scripts
if [[ "$TARGET" == *"windows"* ]] || [[ "$BINARY_NAME" == *.exe ]]; then
    # Windows batch file
    cat > "$DIST_DIR/start.bat" << 'EOF'
@echo off
REM Change to the directory containing this script
cd /d "%~dp0"

echo Starting MCPMate...
echo Web interface will be available at: http://localhost:8080
echo Browser should open automatically
echo Press Ctrl+C to stop

REM Start the server in background
start /B mcpmate.exe

REM Wait a moment for server to start
timeout /t 3 /nobreak >nul

REM Open browser
start http://localhost:8080

REM Keep the window open and wait for user input
echo.
echo MCPMate is running. Press any key to stop...
pause >nul

REM Kill the server process
taskkill /F /IM mcpmate.exe >nul 2>&1
EOF

    cat > "$DIST_DIR/start.sh" << 'EOF'
#!/bin/bash
# Change to the directory containing this script
cd "$(dirname "$0")"

echo "Starting MCPMate..."
echo "Web interface will be available at: http://localhost:8080"
echo "Browser should open automatically"
echo "Press Ctrl+C to stop"

# Start the server in background
./mcpmate.exe &
PROXY_PID=$!

# Wait a moment for server to start
sleep 3

# Open browser (try different commands based on platform)
if command -v start >/dev/null 2>&1; then
    start http://localhost:8080
elif command -v open >/dev/null 2>&1; then
    open http://localhost:8080
elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open http://localhost:8080
else
    echo "Please open http://localhost:8080 in your browser"
fi

echo
echo "MCPMate is running. Press any key to stop..."
read -n 1 -s

# Kill the server process
kill $PROXY_PID 2>/dev/null
EOF
else
    # Unix-like systems
    cat > "$DIST_DIR/start.sh" << 'EOF'
#!/bin/bash
echo "Starting MCPMate..."
echo "Web interface will be available at: http://localhost:8080"
echo "Browser should open automatically"
echo "Press Ctrl+C to stop"

# Function to open browser
open_browser() {
    if command -v xdg-open > /dev/null; then
        xdg-open http://localhost:8080
    elif command -v open > /dev/null; then
        open http://localhost:8080
    else
        echo "Could not detect how to open browser. Please manually open: http://localhost:8080"
    fi
}

# Function to handle cleanup
cleanup() {
    echo ""
    echo "Shutting down MCPMate..."
    if [ ! -z "$PROXY_PID" ]; then
        kill $PROXY_PID 2>/dev/null
    fi
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Start server in background
./mcpmate &
PROXY_PID=$!

# Wait for server to start
sleep 3

# Open browser
open_browser

# Wait for server process
wait $PROXY_PID
EOF

    cat > "$DIST_DIR/start.bat" << 'EOF'
@echo off
echo This package is built for Unix-like systems
echo Please use start.sh instead
pause
EOF
fi

chmod +x "$DIST_DIR/start.sh"

# Create a simple version info file
cat > "$DIST_DIR/VERSION" << EOF
MCPMate Deployment Package
Build Date: $(date)
Build Mode: $BUILD_MODE
Target: ${TARGET:-current platform}
Binary: $BINARY_NAME
EOF

print_success "Deployment package created successfully!"
echo ""
print_status "Package location: ./$DIST_DIR/"
print_status "Package contents:"
ls -la "$DIST_DIR/"
echo ""
print_status "To test the deployment:"
print_status "  cd $DIST_DIR"
if [[ "$TARGET" == *"windows"* ]] || [[ "$BINARY_NAME" == *.exe ]]; then
    print_status "  ./mcpmate.exe (or double-click mcpmate.exe)"
    print_status "  Or use: start.bat"
else
    print_status "  ./mcpmate (or double-click mcpmate)"
    print_status "  Or use: ./start.sh"
fi
print_status "  Browser should open automatically to http://localhost:8080"
echo ""

# Show package size
if command -v du &> /dev/null; then
    PACKAGE_SIZE=$(du -sh "$DIST_DIR" | cut -f1)
    print_status "Total package size: $PACKAGE_SIZE"
fi

# Show available packages
echo ""
print_status "All available packages:"
if [ -d "dist" ]; then
    find dist -maxdepth 1 -type d -not -name "dist" | sort
fi
