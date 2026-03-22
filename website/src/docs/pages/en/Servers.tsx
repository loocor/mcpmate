import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function Servers() {
	return (
		<DocLayout
			meta={{
				title: "Servers",
				description: "Manage and connect MCP servers",
			}}
		>
			<P>
				The Servers page is the heart of MCPMate: it lists every registered MCP
				server, shows connection health, and provides install/import flows. Use
				it to enable or pause capabilities without touching configuration files
				by hand.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/servers-light.png"
				darkSrc="/screenshot/servers-dark.png"
				alt="Servers grid with transport badges and status"
			/>

			<H2>Stats cards & toolbar</H2>
			<Ul>
				<Li>
					Top-level cards count total servers, how many are enabled, currently
					connected, and the overall instance total. Keep an eye on the instance
					count when servers expose multiple transports.
				</Li>
				<Li>
					The toolbar delivers search (name and description), sorting (name or
					enable status), and a reusable grid/list view toggle tied to your
					global default view.
				</Li>
				<Li>
					Action buttons include a refresh icon (forces a server resync) and an
					Add button that doubles as a drag-and-drop surface for Uni-Import.
				</Li>
			</Ul>

			<H2>Reviewing server cards</H2>
			<P>
				Each card shows capability counts (tools, prompts, resources, resource
				templates), transport badges, and a live status indicator. The switch on
				the bottom-right enables or disables the server across all profiles.
			</P>
			<Ul>
				<Li>
					Click the card to open <code>/servers/:serverId</code> for deep
					inspection: overview and instances on the first tab, then nested{" "}
					<strong>Capabilities</strong> tabs for Tools, Resources, Prompts, and
					Resource templates (the top-level sidebar no longer lists Tools as its
					own page).
				</Li>
				<Li>
					Instance-specific URLs use{" "}
					<code>/servers/:serverId/instances/:instanceId</code> when you need to
					bookmark a single transport.
				</Li>
				<Li>
					If debug tooling is enabled in Settings, an extra button appears to
					open the Inspect view (inline or in a new tab depending on the
					preference).
				</Li>
				<Li>
					Error states trigger a blinking status badge so failing servers stand
					out immediately.
				</Li>
			</Ul>

			<DocScreenshot
				lightSrc="/screenshot/server-detail-light.png"
				darkSrc="/screenshot/server-detail-dark.png"
				alt="Server detail overview with instances and capability tabs"
			/>

			<H2>Adding and editing servers</H2>
			<H3>Uni-Import pipeline</H3>
			<P>
				Drop MCP bundles (<code>.mcpb</code>), JSON snippets, URLs, or raw text
				onto the Add button to trigger the server install wizard. MCPMate parses
				the payload, normalizes transports, and lets you preview the resulting
				config before committing it.
			</P>

			<H3>Manual form & edit drawer</H3>
			<P>
				Click <strong>Add Server</strong> to open the manual entry form. For
				existing entries, select a card and choose the edit action inside the
				detail page to update metadata, secrets, or per-instance settings without
				restarting the proxy.
			</P>

			<Callout type="info" title="Debugging failed loads">
				If the server list fails to load, enable the Inspect toggle (Settings →
				Developer) and use the debug button. It reveals raw API responses,
				error messages, and the data MCPMate attempted to render. This is the
				fastest way to validate new backend endpoints during development.
			</Callout>

			<H2>Recommended checklist</H2>
			<Ul>
				<Li>Verify new installs appear with the expected capability counts.</Li>
				<Li>
					Confirm the instance total matches what the Inspector CLI reports for
					the same server.
				</Li>
				<Li>
					Test enable/disable toggles while monitoring the Runtime logs for
					errors.
				</Li>
			</Ul>
		</DocLayout>
	);
}
