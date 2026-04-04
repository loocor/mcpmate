import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function UniImport() {
	return (
		<DocLayout
			meta={{
				title: "Uni-Import",
				description:
					"Easy configuration through drag-and-drop or paste with normalized transport handling",
			}}
		>
			<P>
				Uni-Import is MCPMate's flexible configuration import system. Whether
				you have a JSON file, TOML configuration, or text snippet, you can
				import it into MCPMate through simple drag-and-drop or paste operations.
			</P>

			<H2>Supported formats</H2>
			<Ul>
				<Li>
					<strong>JSON:</strong> Standard MCP configuration format
				</Li>
				<Li>
					<strong>TOML:</strong> Alternative configuration format
				</Li>
				<Li>
					<strong>Snippet text:</strong> Direct paste from docs, chat, or team
					wikis
				</Li>
			</Ul>

			<H2>Import methods</H2>
			<Ul>
				<Li>
					<strong>Drag & Drop:</strong> Simply drag configuration files into
					MCPMate
				</Li>
				<Li>
					<strong>Paste:</strong> Copy configuration text and paste it into the
					import dialog
				</Li>
				<Li>
					<strong>File Browser:</strong> Traditional file selection dialog
				</Li>
			</Ul>

			<H2>Smart parsing</H2>
			<P>
				Uni-Import automatically detects the format of your configuration and
				validates it before import. If there are any issues, MCPMate provides
				clear error messages and suggestions for fixing them. Legacy SSE-style
				input is accepted and normalized to Streamable HTTP during persistence.
			</P>

			<H2>OAuth for upstream HTTP servers</H2>
			<P>
				When an imported Streamable HTTP server requires OAuth, Uni-Import keeps
				auth setup in the first step and starts a callback-based authorization
				flow. MCPMate prepares metadata automatically, opens the provider login,
				and continues to preview/import after authorization completes.
			</P>

			<H2>Use cases</H2>
			<Ul>
				<Li>Importing shared team configurations.</Li>
				<Li>Migrating from other MCP tools.</Li>
				<Li>Quick setup from documentation examples.</Li>
				<Li>Restoring from backups.</Li>
			</Ul>
		</DocLayout>
	);
}
