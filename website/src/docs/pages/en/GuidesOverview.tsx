import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function GuidesOverview() {
	return (
		<DocLayout
			meta={{
				title: "Guides Overview",
				description: "Learn how to use MCPMate effectively",
			}}
		>
			<P>
				This section mirrors the MCPMate operational dashboard (the{" "}
				<code>board/</code> React app shipped with the desktop bundle or run
				against a local proxy). Each guide explains that screen&apos;s purpose,
				the proxy APIs it uses, and workflows we rely on during validation.
			</P>

			<Callout type="info" title="How these guides map to the product">
				The Dashboard, Profiles, Clients, Servers, Market, Runtime, and Audit
				Logs articles match the main sidebar order. API Docs appears at the bottom
				only after you enable it in Settings → Developer. Account and Settings sit
				below that. Open the dashboard in another tab while reading so you can
				follow along.
			</Callout>

			<H2>Board layout: sidebar and header</H2>
			<H3>Sidebar</H3>
			<Ul>
				<Li>
					<strong>MAIN</strong> links, top to bottom: Dashboard, Profiles,
					Clients, Servers, Market, Runtime. There is no separate{" "}
					<strong>Tools</strong> entry; tools, prompts, resources, and templates
					are managed per server on the server detail page.
				</Li>
				<Li>
					At the bottom: optional <strong>API Docs</strong> (opens{" "}
					<code>http://127.0.0.1:8080/docs</code> when the proxy uses default
					ports), <strong>Account</strong> (desktop sign-in; see below), and{" "}
					<strong>Settings</strong>.
				</Li>
			</Ul>
			<H3>Top bar</H3>
			<Ul>
				<Li>
					Page title on primary routes, or a <strong>Back</strong> control on
					nested detail pages.
				</Li>
				<Li>
					<strong>Feedback</strong> opens a prefilled email to the team.
				</Li>
				<Li>
					<strong>Documentation</strong> opens the public guides on{" "}
					<code>mcp.umate.ai</code> in a new tab, choosing the article that best
					matches the screen you are on.
				</Li>
				<Li>
					<strong>Theme</strong> toggles light/dark; <strong>Notifications</strong>{" "}
					lists recent in-app toasts (success, warnings, errors) so you can review
					messages after they dismiss.
				</Li>
			</Ul>
			<H3>Account (desktop)</H3>
			<P>
				In the Tauri desktop app on macOS, <strong>Account</strong> can link a
				GitHub identity for upcoming cloud-backed features. The web dashboard
				shows the same entry point but sign-in is only available inside the
				packaged app; terms and privacy links point at{" "}
				<code>mcp.umate.ai</code>.
			</P>

			<H2>Primary navigation pillars</H2>
			<Ul>
				<Li>
					<strong>Dashboard</strong> &mdash; summary cards for system status,
					profiles, servers, and clients (polled about every 30 seconds), plus a
					CPU / memory chart sampled about every 10 seconds.
				</Li>
				<Li>
					<strong>Profiles</strong> &mdash; curate reusable profile bundles with
					granular enable toggles, statistics, and quick activation buttons.
				</Li>
				<Li>
					<strong>Clients</strong> &mdash; track detected editors, toggle managed
					mode, and audit MCP configuration paths.
				</Li>
				<Li>
					<strong>Servers</strong> &mdash; inspect capability summaries, enable or
					disable instances, and import new servers with Uni-Import or manual
					forms.
				</Li>
				<Li>
					<strong>Market</strong> &mdash; browse the official registry, connect to
					custom portals, preview metadata, and install entries straight into the
					server list.
				</Li>
				<Li>
					<strong>Runtime</strong> &mdash; monitor bundled runtimes (uv, Bun),
					reset caches, and view capability cache statistics.
				</Li>
				<Li>
					<strong>Audit Logs</strong> &mdash; inspect cursor-paginated operation
					history for profile/client/server changes and security-relevant events.
				</Li>
				<Li>
					<strong>API Docs</strong> &mdash; optional sidebar link to the
					proxy&apos;s interactive OpenAPI UI at <code>/docs</code> (default{" "}
					<code>http://127.0.0.1:8080/docs</code>).
				</Li>
				<Li>
					<strong>Settings</strong> &mdash; appearance, language (English,
					Chinese, Japanese), default list views, client and market defaults,
					developer toggles, and system ports.
				</Li>
			</Ul>

			<H2>Recommended learning flow</H2>
			<H3>1. Observe the current state</H3>
			<P>
				Start with the Dashboard and Runtime pages to confirm the proxy is
				healthy, which profiles are active, and how host resources behave under
				load. The metrics chart and status cards highlight issues quickly.
			</P>

			<H3>2. Configure what runs</H3>
			<P>
				Move on to Profiles, Servers, and Clients. These screens share the
				toolbar for search, sorting, list/grid toggles, and filters, so once you
				learn one you can operate the others. Use the drawers and detail pages to
				update metadata without losing your place.
			</P>

			<H3>3. Extend with new capabilities</H3>
			<P>
				Explore the Market to discover additional MCP servers, then confirm the
				runtime and capability caches are primed. Finish in Settings to lock in
				default views, marketplace portals, API base ports, and the API Docs
				shortcut. For separated deployments (core service + UI shell), this step
				is where you keep backend connectivity explicit.
			</P>

			<H2>Shared UI patterns to notice</H2>
			<Ul>
				<Li>
					<strong>Stats cards</strong> &mdash; every management screen opens with
					a four-card summary (total, active/enabled, connectivity, instances).
				</Li>
				<Li>
					<strong>Page toolbar</strong> &mdash; consistent search, sort, and view
					switches with optional filters (e.g., client detected/managed modes).
				</Li>
				<Li>
					<strong>Action drawers</strong> &mdash; creation flows (new profile,
					server install wizard) slide in from the side so the context list stays
					visible.
				</Li>
				<Li>
					<strong>Inspect helpers</strong> &mdash; debug toggles reveal raw JSON,
					log extracts, or install payloads whenever deeper troubleshooting is
					required.
				</Li>
			</Ul>

			<Callout type="warning" title="Prerequisites">
				Run the backend proxy locally (<code>cargo run -p app-mcpmate</code> from
				<code>backend/</code>) or connect to an existing deployment before
				following these guides. Several cards and charts will remain blank if the
				API endpoints at <code>http://127.0.0.1:8080</code> are unreachable.
			</Callout>
		</DocLayout>
	);
}
