import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Quickstart() {
	return (
		<DocLayout
			meta={{ title: "Quick Start", description: "Build, configure, and run MCPMate" }}
		>
			<P>
				This guide walks you through building MCPMate from source, adding
				servers, preparing profiles, and applying them inside your MCP clients.
			</P>

			<H2>Build from source</H2>
			<Callout type="info" title="Open Source">
				MCPMate is open source under the MIT license. Clone the repo at github.com/loocor/mcpmate
			</Callout>
			<Ul>
				<Li>Install Rust 1.75+ and Node.js 18+ (or Bun) on your system.</Li>
				<Li>Clone the repository: <code>git clone https://github.com/loocor/mcpmate.git</code></Li>
				<Li>Navigate to backend: <code>cd mcpmate/backend</code></Li>
				<Li>Build and run: <code>cargo run --release</code></Li>
				<Li>The proxy starts with REST API on port 8080 and MCP endpoint on port 8000.</Li>
			</Ul>

			<H3>Run the Dashboard</H3>
			<Ul>
				<Li>Navigate to board: <code>cd mcpmate/board</code></Li>
				<Li>Install dependencies: <code>bun install</code></Li>
				<Li>Start dev server: <code>bun run dev</code></Li>
				<Li>Open http://localhost:5173 to access the management dashboard.</Li>
			</Ul>

			<H2>Web dashboard vs desktop app</H2>
			<P>
				The same Board UI ships in two shells. Pick whichever matches how you run
				the proxy.
			</P>
			<Ul>
				<Li>
					<strong>Browser + dev proxy</strong> &mdash; Vite serves the UI; API
					calls go to <code>http://127.0.0.1:8080</code> (or your overridden
					base). Best for contributors iterating on frontend or backend
					separately.
				</Li>
				<Li>
					<strong>Tauri desktop (macOS, Windows, Linux)</strong> &mdash; bundles
					the dashboard with the local proxy. The sidebar <strong>Account</strong>{" "}
					entry supports optional GitHub sign-in on macOS for future cloud-backed
					features; the in-app <strong>Documentation</strong> button opens guides
					on <code>mcp.umate.ai</code> for the page you are viewing.
				</Li>
			</Ul>

			<H2>Run in separated mode (core server + UI)</H2>
			<P>
				You can decouple MCPMate core services from the UI shell for remote or
				multi-machine operations.
			</P>
			<Ul>
				<Li>
					Run backend on the target host and expose the REST/MCP ports you plan
					to use.
				</Li>
				<Li>
					Connect the dashboard shell (web or desktop) to that backend instead of
					running an all-in-one local bundle.
				</Li>
				<Li>
					Use the Settings → System section to verify API/MCP ports and copy
					relaunch commands when endpoints change.
				</Li>
			</Ul>

			<H2>Install MCP servers</H2>
			<P>Pick the approach that matches the services you want to use.</P>
			<H3>Browse the built-in marketplace</H3>
			<Ul>
				<Li>Open <strong>Market</strong> from the left sidebar.</Li>
				<Li>Search or filter for a server and select <strong>Install</strong> to add it to your workspace.</Li>
			</Ul>
			<H3>Drag and drop external bundles</H3>
			<Ul>
				<Li>From <strong>Servers</strong>, choose <strong>Add</strong> and drop MCP bundles or JSON/TOML snippets into the window.</Li>
				<Li>Review the preview, then confirm the import to create the server entry.</Li>
			</Ul>
			<H3>Import servers from an existing client</H3>
			<Ul>
				<Li>Open <strong>Clients</strong> and pick a detected client.</Li>
				<Li>Use the <strong>Import from client</strong> action to bring existing MCP configuration into MCPMate.</Li>
			</Ul>

			<H2>Organize profiles</H2>
			<P>
				Profiles decide which servers and capabilities are exposed to your clients. MCPMate ships with a
				<strong> Default</strong> profile, and you can create more for specific scenarios.
			</P>
			<Ul>
				<Li>Go to <strong>Profiles</strong> and open the <strong>Default</strong> profile.</Li>
				<Li>Add the servers you installed, enable or disable the tools, prompts, and resources you need.</Li>
				<Li>
					Use <strong>New Profile</strong> to build additional presets (for example, Writing or Data
					Exploration) and tailor the enabled capabilities.
				</Li>
			</Ul>

			<H2>Apply profiles inside clients</H2>
			<Ul>
				<Li>
					In <strong>Clients</strong>, ensure your editor appears as <strong>Detected</strong>. If it is not, reinstall the client or review the path.
				</Li>
				<Li>
					If the client should allow MCPMate to write its own MCP configuration, confirm in the New / Edit drawer that it points to a real writable local MCP config file. MCPMate validates that path before using it as a write target.
				</Li>
				<Li>
					Set the client to <strong>Hosted</strong> mode to enable in-place profile switching from MCPMate.
					 Use <strong>Unify</strong> mode when you want session-local builtin control instead of dashboard-managed profile switching.
					 <strong>Transparent</strong> mode only writes configuration files and cannot toggle profiles live.
				</Li>
				<Li>Select the profile you prepared and apply it. Launch your editor and trigger an MCP command to confirm the tools appear.</Li>
			</Ul>

			<H2>Troubleshooting runtime</H2>
			<Ul>
				<Li>
					If servers fail to start or tools return errors, open the <strong>Runtime</strong> page and use
					<strong> Install / Repair</strong> to provision required runtimes (uv, Bun).
				</Li>
				<Li>Clear caches from the same page if you suspect stale data.</Li>
			</Ul>

			<H2>Review audit logs</H2>
			<Ul>
				<Li>
					Open the <strong>Audit Logs</strong> page to review profile/client/server
					operations and security-relevant actions.
				</Li>
				<Li>
					Filter by action type and time range, then paginate with cursor-based
					loading for long-running environments.
				</Li>
			</Ul>

			<H2>Updates and contributions</H2>
			<P>
				Pull the latest changes from GitHub to get new features and bug fixes.
				If you encounter issues or have suggestions, please open an issue or submit a pull request.
			</P>
		</DocLayout>
	);
}
