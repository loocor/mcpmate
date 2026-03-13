import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function Inspector() {
	return (
		<DocLayout
			meta={{
				title: "Inspector",
				description:
					"Deep insights into server status, logs, and diagnostics without leaving the console",
			}}
		>
			<P>
				The MCPMate Inspector provides a powerful interface for monitoring and
				debugging your MCP servers. Get real-time insights into server behavior,
				examine logs, and diagnose issues—all from within the MCPMate console.
			</P>

			<h2>Features</h2>
			<ul>
				<li>
					<strong>Real-time Monitoring:</strong> Watch server activity as it
					happens
				</li>
				<li>
					<strong>Log Viewer:</strong> Browse and search through server logs
				</li>
				<li>
					<strong>Request/Response Inspector:</strong> Examine MCP protocol
					messages in detail
				</li>
				<li>
					<strong>Performance Metrics:</strong> Track response times and resource
					usage
				</li>
				<li>
					<strong>Error Diagnostics:</strong> Quickly identify and troubleshoot
					issues
				</li>
			</ul>

			<h2>Use Cases</h2>
			<ul>
				<li>Debugging server configuration issues</li>
				<li>Monitoring server performance in production</li>
				<li>Understanding MCP protocol interactions</li>
				<li>Troubleshooting client-server communication problems</li>
			</ul>

			<P>
				Content coming soon with detailed inspector interface documentation.
			</P>
		</DocLayout>
	);
}
