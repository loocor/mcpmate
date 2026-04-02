import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function Prompts() {
	return (
		<DocLayout
			meta={{ title: "Prompts", description: "Prompt management and overrides" }}
		>
			<P>
				Prompts are reusable instruction assets exposed by MCP servers. MCPMate
				lets you control prompt availability through profile-level activation.
			</P>

			<H2>How prompt visibility is controlled</H2>
			<Ul>
				<Li>Enable or disable servers and capabilities in a profile.</Li>
				<Li>Apply that profile to clients in Hosted mode for live switching; Unify uses session-local builtin tools instead of dashboard-side profile switching.</Li>
				<Li>Validate prompt exposure from server detail and Inspector views.</Li>
			</Ul>

			<H2>Best practices</H2>
			<Ul>
				<Li>Use separate profiles for writing, coding, and analysis contexts.</Li>
				<Li>Keep shared prompts in default anchor profiles when required.</Li>
				<Li>Use Audit Logs to trace prompt-related profile changes.</Li>
			</Ul>
		</DocLayout>
	);
}
