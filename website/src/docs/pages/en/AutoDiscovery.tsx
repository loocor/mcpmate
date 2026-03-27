import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

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

			<H2>How it works</H2>
			<P>
				MCPMate scans common configuration locations used by popular MCP
				clients:
			</P>
			<Ul>
				<Li>Claude Desktop configuration files</Li>
				<Li>VS Code MCP extension settings</Li>
				<Li>Cursor MCP configurations</Li>
				<Li>Other standard MCP client setups</Li>
			</Ul>

			<H2>Import process</H2>
			<Ul>
				<Li>MCPMate scans for existing configurations automatically.</Li>
				<Li>The import view lists discovered servers for review.</Li>
				<Li>You select what to import and target profile placement.</Li>
				<Li>Imported entries are normalized and stored in MCPMate.</Li>
			</Ul>

			<H2>Benefits</H2>
			<Ul>
				<Li>
					<strong>Quick Onboarding:</strong> Get started with MCPMate immediately
				</Li>
				<Li>
					<strong>No Manual Work:</strong> Avoid copying configuration details by
					hand
				</Li>
				<Li>
					<strong>Preserve Existing Setup:</strong> Your original configurations
					remain untouched
				</Li>
				<Li>
					<strong>Error Prevention:</strong> Reduce configuration mistakes from
					manual entry
				</Li>
			</Ul>
		</DocLayout>
	);
}
