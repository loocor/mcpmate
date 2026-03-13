import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function APIDocsEN() {
	return (
		<DocLayout
			meta={{
				title: "API Docs",
				description: "REST and MCP API references",
			}}
		>
			<P>
				MCPMate exposes interactive documentation for its REST endpoints at{" "}
				<code>http://127.0.0.1:8080/docs</code> (served by the backend proxy).
				This page explains how to surface the shortcut in the dashboard and what
				you can do once you are there.
			</P>

			<H2>Enabling the shortcut</H2>
			<Ul>
				<Li>
					Navigate to <strong>Settings → Developer</strong> and flip on{" "}
					<strong>Show API Docs Menu</strong>. The sidebar will reveal an{" "}
					<em>API Docs</em> link near Settings.
				</Li>
				<Li>
					If you changed the backend ports (Settings → Developer → Runtime Ports),
					update the API base URL so the shortcut targets the correct host.
				</Li>
				<Li>
					Clicking the link opens the docs in a new tab. You can also visit the
					URL directly if you prefer bookmarking it.
				</Li>
			</Ul>

			<H2>Inside the documentation</H2>
			<H3>REST operations</H3>
			<P>
				The OpenAPI view lists endpoints grouped by area (system, servers,
				config suits, runtime, marketplace, etc.). Expand an operation to inspect
				request/response schemas and issue test calls using the Try It button.
			</P>

			<H3>MCP & SSE endpoints</H3>
			<P>
				The documentation also references the MCP transport endpoints (HTTP SSE
				and WebSocket bridges). Use these summaries to verify headers, payload
				keys, and auth expectations when writing client integrations.
			</P>

			<H2>Best practices</H2>
			<Ul>
				<Li>
					Keep the dashboard open while experimenting with the API so you can
					observe visible changes (e.g., server installs, profile toggles).
				</Li>
				<Li>
					When testing destructive operations, capture the request in your notes
					or scripts so others can reproduce the scenario.
				</Li>
				<Li>
					Use the <strong>Authorize</strong> button if authentication becomes
					required in future releases—the docs tool reuses the same cookies or
					tokens as regular API calls.
				</Li>
			</Ul>

			<Callout type="warning" title="Proxy must be running">
				The docs endpoint only appears when the backend is live. If the page does
				not load, confirm <code>cargo run -p app-mcpmate</code> (or your deployed
				instance) is serving <code>/docs</code> and that your browser can reach
				the configured API port.
			</Callout>
		</DocLayout>
	);
}
