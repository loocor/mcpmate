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
				The MCPMate Inspector is the live capability workbench for MCP servers. Use
				it to compare native and proxy behavior, run controlled tool and prompt
				requests, read resources, and capture response or event evidence without
				leaving the console.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="Inspector tool call panel against server capabilities"
			/>

			<H2>Features</H2>
			<Ul>
				<Li>
					<strong>Native and proxy channels:</strong> Compare raw server behavior
					with MCPMate-managed exposure
				</Li>
				<Li>
					<strong>Schema-aware inputs:</strong> Generate forms from capability
					metadata or switch to raw JSON
				</Li>
				<Li>
					<strong>Response and event review:</strong> Examine final output,
					progress, logs, errors, and cancellation separately
				</Li>
				<Li>
					<strong>Capability reads:</strong> Validate tools, prompts, resources,
					and resource templates from the same drawer workflow
				</Li>
				<Li>
					<strong>Error diagnostics:</strong> Quickly identify and troubleshoot
					request failures
				</Li>
			</Ul>

			<H2>Use cases</H2>
			<Ul>
				<Li>Debugging server configuration issues.</Li>
				<Li>Comparing native server output with profile-scoped proxy output.</Li>
				<Li>Validating capability payloads before enabling to all clients.</Li>
				<Li>Troubleshooting client-server communication problems.</Li>
			</Ul>

			<H2>Suggested workflow</H2>
			<Ul>
				<Li>Start from server details to identify the target server/capability.</Li>
				<Li>Run controlled calls in Inspector and review response and event output separately.</Li>
				<Li>Cross-check timestamps with Audit Logs for full operation context.</Li>
			</Ul>
		</DocLayout>
	);
}
