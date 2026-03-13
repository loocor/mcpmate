import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "Auto Discovery & Import",
				description:
					"Automatically detect and import existing configurations - no manual editing required",
			}}
		>
			<P>
				MCPMate can automatically discover existing MCP server configurations on
				your system and import them with a single click. This eliminates the
				tedious process of manually recreating your setup in a new tool.
			</P>

			<h2>How It Works</h2>
			<P>
				MCPMate scans common configuration locations used by popular MCP
				clients:
			</P>
			<ul>
				<li>Claude Desktop configuration files</li>
				<li>VS Code MCP extensions settings</li>
				<li>Cursor MCP configurations</li>
				<li>Other standard MCP client setups</li>
			</ul>

			<h2>Import Process</h2>
			<ol>
				<li>MCPMate automatically scans for existing configurations</li>
				<li>Displays discovered servers in the import interface</li>
				<li>You review and select which servers to import</li>
				<li>MCPMate imports the configurations into your active profile</li>
			</ol>

			<h2>Benefits</h2>
			<ul>
				<li>
					<strong>Quick Onboarding:</strong> Get started with MCPMate immediately
				</li>
				<li>
					<strong>No Manual Work:</strong> Avoid copying configuration details by
					hand
				</li>
				<li>
					<strong>Preserve Existing Setup:</strong> Your original configurations
					remain untouched
				</li>
				<li>
					<strong>Error Prevention:</strong> Reduce configuration mistakes from
					manual entry
				</li>
			</ul>

			<P>Content coming soon with auto-discovery walkthrough and screenshots.</P>
		</DocLayout>
	);
}
