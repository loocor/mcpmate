import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "Auto Discovery & Import",
				description:
					"Discover local MCP configurations and Discovery presets for faster setup.",
			}}
		>
			<P>
				MCPMate combines local configuration scanning with the Public Discovery
				catalog. Local discovery finds MCP settings already present on your
				machine, while Discovery presets provide curated client and server
				starting points for new setups.
			</P>

			<H2>Local discovery</H2>
			<P>
				MCPMate scans common configuration locations used by popular MCP
				clients:
			</P>
			<Ul>
				<Li>Claude Desktop configuration files</Li>
				<Li>VS Code MCP extension settings</Li>
				<Li>Cursor MCP configurations</Li>
				<Li>Other standard MCP client setups, including user-defined clients</Li>
			</Ul>

			<H2>Discovery presets</H2>
			<P>
				Public Discovery powers the preset entries shown during first run, inside
				client add/edit drawers, and in the browser extension. These entries
				include identifiers, display names, links, icons, and import metadata so
				MCPMate can create a cleaner draft before you review it.
			</P>
			<Ul>
				<Li>Client presets help add supported AI apps with known MCP config targets.</Li>
				<Li>Server entries provide import-ready metadata for the server wizard.</Li>
				<Li>Portal entries connect Market documentation and the browser extension portal tab.</Li>
			</Ul>

			<H2>Import process</H2>
			<Ul>
				<Li>MCPMate scans for existing local configurations.</Li>
				<Li>The import view lists discovered servers and Discovery-backed drafts for review.</Li>
				<Li>You select what to import and target profile placement.</Li>
				<Li>Imported entries are normalized and stored in MCPMate.</Li>
			</Ul>

			<H2>Benefits</H2>
			<Ul>
				<Li>
					<strong>Quick Onboarding:</strong> Get started with MCPMate immediately
				</Li>
				<Li>
					<strong>Guided Setup:</strong> Start from detected local state or
					Discovery-backed presets
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
