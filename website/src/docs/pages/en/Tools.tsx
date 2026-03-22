import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Tools() {
	return (
		<DocLayout
			meta={{ title: "Tools", description: "Use and govern MCP tools in the dashboard" }}
		>
			<P>
				MCP <strong>tools</strong> are callable capabilities advertised by each MCP
				server. In the Board UI they are not a top-level sidebar section; you work
				with them inside <strong>Profiles</strong> and <strong>Servers</strong> so
				visibility stays tied to the servers you trust.
			</P>

			<Callout type="info" title="Where to click in Board">
				Open a server at <code>/servers/:serverId</code>, then use the{" "}
				<strong>Capabilities</strong> area and the <strong>Tools</strong> tab to
				list names, descriptions, and enablement. At the profile level, the same
				tool keys appear with toggles so you can expose only what each client
				needs.
			</Callout>

			<H2>Enablement layers</H2>
			<Ul>
				<Li>
					<strong>Server</strong> &mdash; turning a server off removes its tools
					from every profile until it is enabled again.
				</Li>
				<Li>
					<strong>Profile</strong> &mdash; per-tool switches inside an active
					profile let you narrow the merged surface without uninstalling the
					server.
				</Li>
				<Li>
					<strong>Client</strong> &mdash; managed clients receive the merged set
					from active profiles; transparent mode only reflects what you wrote to
					disk.
				</Li>
			</Ul>

			<H2>Discovery and debugging</H2>
			<H3>Capability cache</H3>
			<P>
				Tool metadata is cached for performance. If a server ships an updated
				manifest, reset the capability cache from the <strong>Runtime</strong>{" "}
				page or restart the proxy so lists match what the Inspector CLI sees.
			</P>

			<H3>Raw JSON</H3>
			<P>
				Enable <strong>Show Raw Capability JSON</strong> under Settings →
				Developer when you need to compare the proxy response with what the Board
				rendered.
			</P>

			<P>
				For step-by-step UI flows, read the <strong>Servers</strong> and{" "}
				<strong>Profiles</strong> guides in this documentation; they mirror the
				Board routes where tools are edited.
			</P>
		</DocLayout>
	);
}
