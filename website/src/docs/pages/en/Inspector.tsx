import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

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
				examine logs, and diagnose issues all from within the MCPMate console.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="Inspector tool call panel against server capabilities"
			/>

			<H2>Features</H2>
			<Ul>
				<Li>
					<strong>Real-time Monitoring:</strong> Watch server activity as it
					happens
				</Li>
				<Li>
					<strong>Log Viewer:</strong> Browse and search through server logs
				</Li>
				<Li>
					<strong>Request/Response Inspector:</strong> Examine MCP protocol
					messages in detail
				</Li>
				<Li>
					<strong>Performance Metrics:</strong> Track response times and resource
					usage
				</Li>
				<Li>
					<strong>Error Diagnostics:</strong> Quickly identify and troubleshoot
					issues
				</Li>
			</Ul>

			<H2>Use cases</H2>
			<Ul>
				<Li>Debugging server configuration issues.</Li>
				<Li>Monitoring runtime behavior during rollout windows.</Li>
				<Li>Validating capability payloads before enabling to all clients.</Li>
				<Li>Troubleshooting client-server communication problems.</Li>
			</Ul>

			<H2>Suggested workflow</H2>
			<Ul>
				<Li>Start from server details to identify the target server/capability.</Li>
				<Li>Run controlled calls in Inspector and capture response metadata.</Li>
				<Li>Cross-check timestamps with Audit Logs for full operation context.</Li>
			</Ul>
		</DocLayout>
	);
}
