import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Settings() {
	return (
		<DocLayout
			meta={{
				title: "Settings",
				description: "Preferences and configuration",
			}}
		>
			<P>
				The Settings screen centralizes every configurable aspect of the
				dashboard experience. Tabs split controls into logical groups so you can
				tune appearance, default behaviors, marketplace sources, developer
				toggles, and backend connectivity without leaving the app.
			</P>

			<H2>Tab overview</H2>
			<Ul>
				<Li>
					<strong>General</strong> &mdash; set the default list/grid view,
					pick the app mode (Express vs. Expert), and choose the dashboard
					language (work-in-progress).
				</Li>
				<Li>
					<strong>Appearance</strong> &mdash; switch between light/dark/system
					theme, control the macOS tray/dock behavior when running via the Tauri
					shell, and keep the UI synchronized with system preferences.
				</Li>
				<Li>
					<strong>Server Controls</strong> &mdash; decide whether enable/disable
					actions propagate to managed clients and whether new installs auto-join
					the default profile.
				</Li>
				<Li>
					<strong>Client Defaults</strong> &mdash; configure hosted vs.
					transparent modes, default visibility filters, backup strategies, and
					backup limits for editor configs.
				</Li>
				<Li>
					<strong>MCP Market</strong> &mdash; choose the default portal, toggle
					the blacklist, search hidden servers, and restore items to the market.
				</Li>
				<Li>
					<strong>Developer</strong> &mdash; enable server inspection buttons,
					open inspect views in new windows, reveal API Docs, and display raw JSON
					payloads or default headers.
				</Li>
				<Li>
					<strong>System</strong> &mdash; adjust API/MCP ports, copy launch
					commands, and stop the currently running backend.
				</Li>
				<Li>
					<strong>About & Licenses</strong> (visible when license data loads)
					&mdash; browse aggregated open-source notices pulled from the backend.
				</Li>
			</Ul>

			<H2>Key workflows</H2>
			<H3>Adjust default layouts</H3>
			<P>
				Use General → Default View to pick list or grid. The selection immediately
				applies to Profiles, Clients, and Servers, matching the toolbar toggle
				behaviour described in their guides.
			</P>

			<H3>Coordinate client rollouts</H3>
			<P>
				Under Client Defaults, set how new editors should behave (Hosted vs.
				Transparent), what filter the Clients page should use on load, and how
				many configuration backups to keep. These values feed the store consumed
				by the Clients page toolbar.
			</P>

			<H3>Manage marketplace curation</H3>
			<P>
				The MCP Market tab lets you mark portals as default, enable or disable the
				blacklist, and recover hidden entries. The search and sort inputs help you
				find older blacklist entries when QA revisits a server.
			</P>

			<H3>Tweak runtime connectivity</H3>
			<P>
				In the System tab, set custom API/MCP ports, copy ready-to-run commands
				(for both <code>cargo run</code> and the release binary), and shut down
				the currently attached backend process before relaunching it with new
				ports.
			</P>

			<Callout type="warning" title="Restart required after port changes">
				When you update API or MCP ports, restart the backend with the copied
				command before refreshing the dashboard. Otherwise, the UI will continue
				to point at the old port and fail to fetch data.
			</Callout>

			<H2>Developer-focused options</H2>
			<Ul>
				<Li>
					Enable <strong>Server Inspection</strong> to surface debug buttons on
					the Servers page.
				</Li>
				<Li>
					Turn on <strong>Open Inspect In New Window</strong> when you want deep
					debug sessions without losing list context.
				</Li>
				<Li>
					Give QA teams access to <strong>Show Raw Capability JSON</strong> to
					verify payloads exposed through Uni-Import and server details.
				</Li>
			</Ul>

			<Callout type="info" title="Desktop-specific toggles">
				Menu bar and dock controls only appear when MCPMate runs inside the Tauri
				shell. In the web-only preview they stay hidden, so adjust them on the
				macOS build when validating the desktop app bundle.
			</Callout>
		</DocLayout>
	);
}
