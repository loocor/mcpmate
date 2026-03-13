# MCPBoard

MCPBoard is the web-based management dashboard for MCPMate, providing a comprehensive graphical interface for managing Model Context Protocol (MCP) servers, clients, profiles, and runtime environments.

## Overview

MCPBoard serves as the frontend application for the MCPMate ecosystem, offering an intuitive and feature-rich interface to interact with the MCPMate backend services. It enables users to manage MCP servers, configure client connections, organize profiles for different scenarios, monitor system health, and discover new MCP servers from integrated marketplaces.

## Features

### Dashboard
- **System Overview**: Real-time monitoring of system metrics including CPU, memory usage, and uptime
- **Metrics Visualization**: Historical charts for resource consumption tracking
- **Quick Stats**: Overview of servers, clients, profiles, and runtime status
- **Activity Monitoring**: Live updates on system activity and health status

### Server Management
- **Server List**: View all configured MCP servers with their status, type, and instance counts
- **Server Details**: Comprehensive server information including:
  - Configuration details (command, arguments, environment variables)
  - Active instances with health status
  - Capabilities overview (tools, resources, prompts, resource templates)
- **Instance Management**: Monitor and manage individual server instances:
  - View instance health and connection status
  - Disconnect, reconnect, or reset instances
  - Cancel initialization operations

### Client Management
- **Client List**: Browse all detected and configured MCP clients
- **Client Details**: Manage client-specific configurations:
  - Apply profiles to clients
  - Configure backup policies
  - Restore and manage backups
  - View client status and capabilities

### Profile Management
- **Profile Organization**: Create and manage profiles (formerly ConfigSuits) for different scenarios
- **Profile Presets**: Access predefined profile templates for common use cases
- **Profile Details**: Configure which servers and tools are active in each profile
- **Dynamic Switching**: Enable or disable profiles without restarting services

### Runtime Management
- **Runtime Installation**: Install and manage runtime environments (Node.js, uv/Python, Bun.js)
- **Runtime Status**: Check installed runtimes and their versions
- **Environment Integration**: Automatic environment variable configuration for seamless MCP server usage

### MCP Market Integration
- **Market Discovery**: Browse and discover MCP servers from integrated marketplaces (mcpmarket.cn, mcp.so)
- **Server Installation**: One-click installation of servers from market listings
- **Market Proxy**: Seamless integration with remote market portals through built-in proxy middleware
- **Search and Filter**: Find servers by category, tags, and keywords

### Settings
- **General Settings**: Configure language, theme, and application preferences
- **Market Settings**: Manage market blacklist and portal preferences
- **About**: View application version, licenses, and component information

## Technology Stack

- **Framework**: React 18 with TypeScript
- **Build Tool**: Vite
- **UI Components**: shadcn/ui + Radix UI primitives
- **Styling**: Tailwind CSS
- **State Management**: TanStack Query for server state, Zustand for client state
- **Routing**: React Router v6
- **Internationalization**: react-i18next (supports English, Simplified Chinese, Japanese)
- **Charts**: Recharts for data visualization
- **Forms**: React Hook Form with Zod validation

## Getting Started

### Prerequisites

- Node.js 18+ or Bun
- MCPMate backend running at `http://localhost:8080`

### Installation

```bash
# Install dependencies
bun install
# or
npm install
```

### Development

```bash
# Start development server
bun run dev
# or
npm run dev
```

The development server will start on `http://localhost:5173` (or 5174 if 5173 is occupied). The Vite dev server automatically proxies:
- `/api/*` → `http://localhost:8080/api/*`
- `/ws` → `ws://localhost:8080/ws`
- `/market-proxy/*` → Remote market portals (mcpmarket.cn, mcp.so)

### Building for Production

```bash
# Build for production
bun run build
# or
npm run build
```

The production build will be output to the `dist/` directory.

### Preview Production Build

```bash
# Preview production build
bun run preview
# or
npm run preview
```

## Project Structure

```
board/
├── src/
│   ├── App.tsx                 # Main app component with routing
│   ├── components/             # Reusable UI components
│   │   ├── layout/            # Layout components (header, sidebar)
│   │   └── ui/                # shadcn/ui component wrappers
│   ├── pages/                 # Feature pages
│   │   ├── dashboard/         # Dashboard page
│   │   ├── servers/           # Server management pages
│   │   ├── clients/           # Client management pages
│   │   ├── profile/           # Profile management pages
│   │   ├── runtime/           # Runtime management page
│   │   ├── market/            # MCP Market integration
│   │   └── settings/          # Settings page
│   ├── lib/
│   │   ├── api.ts             # Centralized API client
│   │   ├── types.ts           # TypeScript type definitions
│   │   ├── i18n/              # Internationalization setup
│   │   └── utils.ts            # Utility functions
│   └── hooks/                 # Custom React hooks
├── docs/
│   └── openapi.json           # OpenAPI schema (source of truth)
├── vite.config.ts            # Vite configuration with proxy setup
└── package.json
```

## API Integration

MCPBoard communicates with the MCPMate backend through RESTful APIs. The API client is centralized in `src/lib/api.ts` and aligns with the OpenAPI specification in `docs/openapi.json`.

### Key API Endpoints

- **Servers**: `/api/mcp/servers/*`
- **Clients**: `/api/mcp/clients/*`
- **Profiles**: `/api/mcp/profile/*`
- **Runtime**: `/api/runtime/*`
- **System**: `/api/system/*`

### WebSocket Support

The application supports WebSocket connections at `/ws` for real-time updates. In development, this is proxied through Vite. In desktop builds (Tauri), the UI connects directly to `ws://127.0.0.1:8080/ws`.

## Internationalization

MCPBoard supports multiple languages:
- English (`en`)
- Simplified Chinese (`zh-CN`)
- Japanese (`ja-JP`)

Language detection follows this priority:
1. User preference stored in `localStorage`
2. Browser navigator language
3. HTML tag language attribute

Users can manually switch languages in Settings → General → Language.

### Translation Guidelines

- All user-facing strings must be internationalized
- Use nested object structure in translation files (not dot-notation keys)
- Include `i18n.language` in React hook dependencies when using translations
- Load page translations before first render using `usePageTranslations()`

## Theming

MCPBoard supports three theme modes:
- **Light**: Light color scheme
- **Dark**: Dark color scheme
- **System**: Follows OS `prefers-color-scheme`

Theme preference is stored in `localStorage.mcp_theme` and applied early to avoid flash of unstyled content (FOUC).

## Desktop Application Support

MCPBoard is designed to work both as a web application and as a desktop application through Tauri integration. When running in Tauri:
- Custom URI scheme `mcpmate://localhost/market-proxy/*` is used for market portals
- API base URL can be configured at runtime via Settings
- Native system integration features are available

## Development Guidelines

### Code Style
- TypeScript with strict type checking
- Functional React components with hooks
- Prefer composition over inheritance
- Keep components small and focused (< 300 lines when possible)

### Data Fetching
- Use TanStack Query for all server state
- Isolate API calls in `src/lib/api.ts`
- Use query invalidation for cache updates
- Prefer event-driven refresh via WebSocket when available

### Component Organization
- Extract reusable components to `src/components/`
- Page-specific components stay in `src/pages/{page}/`
- Use shadcn/ui components for consistency
- Follow Radix UI patterns for accessibility

## Testing

Manual validation is the primary testing approach:
1. Start the backend at `http://localhost:8080`
2. Run `bun run dev` to start the development server
3. Verify pages load correctly and API calls succeed
4. Test in all supported languages
5. Verify theme switching works correctly

## Contributing

Before contributing, please review:
- [Repository Guidelines](./AGENTS.md) for development workflows
- [Backend README](../backend/README.md) for API documentation
- OpenAPI schema in `docs/openapi.json` for API contract

## License

See the main MCPMate repository for license information.

## Related Projects

- [MCPMate Backend](../backend/): Core proxy and management services
- [MCPMate SDK](../sdk/): Rust SDK for MCP protocol
- [MCPMate Desktop](../desktop/): Native desktop applications
