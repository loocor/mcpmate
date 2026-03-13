import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function CentralizedConfig() {
	return (
		<DocLayout
			meta={{
				title: "Centralized Configuration",
				description:
					"Configure once, use everywhere across all your MCP clients",
			}}
		>
			<P>
				One of MCPMate's core features is centralized configuration management.
				Instead of maintaining separate configurations for each MCP client, you
				can configure your servers once in MCPMate and have them automatically
				available across all connected clients.
			</P>

			<h2>Benefits</h2>
			<ul>
				<li>
					<strong>Single Source of Truth:</strong> All your MCP server
					configurations are managed in one place
				</li>
				<li>
					<strong>No Duplication:</strong> Eliminate the need to copy settings
					across different clients
				</li>
				<li>
					<strong>Consistent Experience:</strong> Ensure all clients use the same
					server configurations
				</li>
				<li>
					<strong>Easy Updates:</strong> Change configurations once and have them
					apply everywhere
				</li>
			</ul>

			<h2>How It Works</h2>
			<P>
				MCPMate acts as a central hub for all your MCP servers. When you
				configure a server in MCPMate, it becomes available to all connected
				clients automatically. This eliminates the traditional workflow of
				manually editing configuration files for each client application.
			</P>

			<h2>Use Cases</h2>
			<ul>
				<li>Using the same servers across Claude Desktop, Cursor, and VS Code</li>
				<li>Managing team-wide server configurations</li>
				<li>Quickly onboarding new clients without reconfiguration</li>
			</ul>

			<P>Content coming soon with detailed configuration examples.</P>
		</DocLayout>
	);
}
