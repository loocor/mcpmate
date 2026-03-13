import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function UniImport() {
	return (
		<DocLayout
			meta={{
				title: "Uni-Import",
				description:
					"Easy configuration through drag-and-drop or paste - supports JSON/TOML, mcpb coming soon",
			}}
		>
			<P>
				Uni-Import is MCPMate's flexible configuration import system. Whether
				you have a JSON file, TOML configuration, or text snippet, you can
				import it into MCPMate through simple drag-and-drop or paste operations.
			</P>

			<h2>Supported Formats</h2>
			<ul>
				<li>
					<strong>JSON:</strong> Standard MCP configuration format
				</li>
				<li>
					<strong>TOML:</strong> Alternative configuration format
				</li>
				<li>
					<strong>MCPB:</strong> Coming soon - MCPMate's bundle format for
					sharing complete setups
				</li>
			</ul>

			<h2>Import Methods</h2>
			<ul>
				<li>
					<strong>Drag & Drop:</strong> Simply drag configuration files into
					MCPMate
				</li>
				<li>
					<strong>Paste:</strong> Copy configuration text and paste it into the
					import dialog
				</li>
				<li>
					<strong>File Browser:</strong> Traditional file selection dialog
				</li>
			</ul>

			<h2>Smart Parsing</h2>
			<P>
				Uni-Import automatically detects the format of your configuration and
				validates it before import. If there are any issues, MCPMate provides
				clear error messages and suggestions for fixing them.
			</P>

			<h2>Use Cases</h2>
			<ul>
				<li>Importing shared team configurations</li>
				<li>Migrating from other MCP tools</li>
				<li>Quick setup from documentation examples</li>
				<li>Restoring from backups</li>
			</ul>

			<P>Content coming soon with Uni-Import examples and supported schemas.</P>
		</DocLayout>
	);
}
