import { useMemo } from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";

const howToSteps = [
	{
		name: "Get the desktop installer",
		text: "Start on GitHub Releases and download the installer for your platform. macOS builds are the most stable today; Windows and Linux installers are available too, but those platforms are still catching up.",
	},
	{
		name: "Launch MCPMate",
		text: "Open the app and let it start the bundled local proxy. You’ll get the dashboard plus the REST API on port 8080 and the MCP endpoint on port 8000.",
	},
	{
		name: "Bring in MCP servers",
		text: "Browse the built-in Market, import JSON/TOML snippets, or pull server settings from an existing client.",
	},
	{
		name: "Shape a profile",
		text: "Open the Default profile, add the servers you want, and turn tools, prompts, and resources on or off for the workflow you’re building.",
	},
	{
		name: "Roll it out to clients",
		text: "On the Clients page, confirm your editor is detected, choose Hosted, Unify, or Transparent mode, then apply the profile and verify it inside your editor.",
	},
];

export default function Quickstart() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "How to set up MCPMate",
				description:
					"Step-by-step guide to get MCPMate running from GitHub Releases first, then expand with servers, profiles, and client rollout.",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout
			meta={{ title: "Quick Start", description: "Install, configure, and run MCPMate" }}
		>
			<SchemaOrg schema={howTo} />
			<P>
				The fastest way to start today is the desktop installer on GitHub
				Releases. Once MCPMate is running, you can bring in servers, shape
				profiles, and roll the setup into your editor in a few deliberate
				steps.
			</P>

			<H2>Start with the desktop app</H2>
			<Callout type="info" title="Fastest path today">
				The quickest way to get MCPMate running today is the official desktop
				installer on GitHub Releases:
				https://github.com/loocor/mcpmate/releases
			</Callout>
			<Ul>
				<Li>Choose the installer for your platform from the Releases page.</Li>
				<Li>
					macOS installers are the most stable today. Windows and Linux
					installers are available too, but some features may still be
					incomplete or unstable while those platforms catch up.
				</Li>
				<Li>
					Launch MCPMate after installation. The desktop app packages the
					dashboard and local proxy together, so you can start operating from
					one place.
				</Li>
			</Ul>

			<H3>Build from source when you want full control</H3>
			<Ul>
				<Li>Install Rust 1.75+ and Node.js 18+ (or Bun) on your system.</Li>
				<Li>
					Clone the repository: <code>git clone https://github.com/loocor/mcpmate.git</code>
				</Li>
				<Li>
					Navigate to backend: <code>cd mcpmate/backend</code>
				</Li>
				<Li>
					Build and run: <code>cargo run --release</code>
				</Li>
				<Li>
					The proxy starts with REST API on port 8080 and MCP endpoint on port
					8000.
				</Li>
			</Ul>

			<H3>Run the dashboard from source</H3>
			<Ul>
				<Li>
					Navigate to dashboard: <code>cd mcpmate/board</code>
				</Li>
				<Li>Install dependencies: <code>bun install</code></Li>
				<Li>Start dev server: <code>bun run dev</code></Li>
				<Li>Open http://localhost:5173 to access the management dashboard.</Li>
			</Ul>

			<H2>Pick your shell: web or desktop</H2>
			<P>
				The same Board UI can run in two shells. Choose the one that matches
				how you want to operate the proxy.
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
					the dashboard with the local proxy. Official installers are
					published on GitHub Releases. The sidebar <strong>Account</strong>{" "}
					entry supports optional GitHub sign-in on macOS for future
					cloud-backed features; the in-app <strong>Documentation</strong>{" "}
					button opens guides on <code>mcp.umate.ai</code> for the page you are
					viewing.
				</Li>
			</Ul>

			<H2>Run core and UI separately</H2>
			<P>
				If you want MCPMate to live on another machine, or you simply prefer a
				split deployment, you can decouple the core services from the UI shell.
			</P>
			<Ul>
				<Li>
					Run backend on the target host and expose the REST/MCP ports you plan
					to use.
				</Li>
				<Li>
					Connect the dashboard shell (web or desktop) to that backend instead
					of running an all-in-one local bundle.
				</Li>
				<Li>
					Use the Settings → System section to verify API/MCP ports and copy
					relaunch commands when endpoints change.
				</Li>
			</Ul>

			<H2>Bring servers into MCPMate</H2>
			<P>
				Choose the import path that matches where your server definitions
				already live.
			</P>
			<H3>Browse the built-in Market</H3>
			<Ul>
				<Li>Open <strong>Market</strong> from the left sidebar.</Li>
				<Li>
					Search or filter for a server and select <strong>Install</strong> to
					add it to your workspace.
				</Li>
			</Ul>
			<H3>Drag in external bundles</H3>
			<Ul>
				<Li>
					From <strong>Servers</strong>, choose <strong>Add</strong> and drop MCP
					bundles or JSON/TOML snippets into the window.
				</Li>
				<Li>
					Review the preview, then confirm the import to create the server
					entry.
				</Li>
			</Ul>
			<H3>Import from an existing client</H3>
			<Ul>
				<Li>Open <strong>Clients</strong> and pick a detected client.</Li>
				<Li>
					Use the <strong>Import from client</strong> action to bring existing
					MCP configuration into MCPMate.
				</Li>
			</Ul>

			<H2>Shape profiles around real tasks</H2>
			<P>
				Profiles decide which servers and capabilities are exposed to your
				clients. MCPMate ships with a <strong>Default</strong> profile, and you
				can create more when different workflows need different surfaces.
			</P>
			<Ul>
				<Li>
					Go to <strong>Profiles</strong> and open the <strong>Default</strong>{" "}
					profile.
				</Li>
				<Li>
					Add the servers you installed, then enable or disable the tools,
					prompts, and resources that fit the job.
				</Li>
				<Li>
					Use <strong>New Profile</strong> to build additional presets (for
					example, Writing or Data Exploration) and tailor the enabled
					capabilities.
				</Li>
			</Ul>

			<H2>Roll profiles into clients</H2>
			<Ul>
				<Li>
					In <strong>Clients</strong>, ensure your editor appears as
					<strong> Detected</strong>. If it does not, reinstall the client or
					review the path.
				</Li>
				<Li>
					If the client should allow MCPMate to write its own MCP configuration,
					confirm in the New / Edit drawer that it points to a real writable
					local MCP config file. MCPMate validates that path before using it as
					a write target.
				</Li>
				<Li>
					Set the client to <strong>Hosted</strong> mode for dashboard-managed
					profile switching. Use <strong>Unify</strong> mode when you want
					session-local built-in control instead. <strong>Transparent</strong>{" "}
					mode only writes configuration files and cannot switch profiles live.
				</Li>
				<Li>
					Select the profile you prepared and apply it. Then launch your editor
					and trigger an MCP command to confirm the tools appear.
				</Li>
			</Ul>

			<H2>If something fails at runtime</H2>
			<Ul>
				<Li>
					If servers fail to start or tools return errors, open the
					<strong> Runtime</strong> page.
				</Li>
				<Li>
					Use <strong>Install / Repair</strong> to provision required runtimes
					like uv and Bun, and clear caches from the same page if you suspect
					stale data.
				</Li>
			</Ul>

			<H2>Trace changes with Audit Logs</H2>
			<Ul>
				<Li>
					Open the <strong>Audit Logs</strong> page to review profile, client,
					and server operations.
				</Li>
				<Li>
					Filter by action type and time range to understand what changed and
					when.
				</Li>
			</Ul>

			<H2>Stay current and contribute</H2>
			<P>
				If you run the desktop app, check GitHub Releases for the latest
				installers and release notes. If you run MCPMate from source, pull the
				latest changes from GitHub and rebuild. If you hit an issue or want to
				improve the project, open an issue or send a pull request.
			</P>
		</DocLayout>
	);
}
