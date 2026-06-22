import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function BrowserExtension() {
	return (
		<DocLayout
			meta={{
				title: "Browser Extension",
				description:
					"Use the MCPMate browser extension to browse curated discovery entries and send MCP snippets into the desktop import flow.",
			}}
		>
			<P>
				The MCPMate Chrome and Edge extension keeps discovery close to the web pages
				where MCP servers are usually found. The toolbar popup shows curated
				Portals, Servers, and Clients from MCPMate Public Discovery, while the
				content script can send detected MCP snippets, GitHub MCP catalog entries,
				and Cursor.directory entries into the desktop app.
			</P>

			<H2>Discovery tabs</H2>
			<Ul>
				<Li>
					<strong>Portals</strong> lists useful MCP discovery destinations and
					community resources.
				</Li>
				<Li>
					<strong>Servers</strong> lists curated server entries published through
					MCPMate Admin.
				</Li>
				<Li>
					<strong>Clients</strong> lists compatible AI apps and client presets that
					can help with setup.
				</Li>
			</Ul>

			<H2>Snippet handoff</H2>
			<P>
				When a page contains a likely MCP server configuration block, the extension
				adds an <strong>Add to MCPMate</strong> action. Clicking it opens the
				desktop app through <code>mcpmate://import/server</code> with the snippet
				text, inferred format, and source URL. MCPMate then continues through the
				same Uni-Import preview and validation flow used by the Servers page.
			</P>
			<Ul>
				<Li>
					On <strong>GitHub MCP</strong> catalog pages, the extension adds an
					Install in MCPMate option to server install menus.
				</Li>
				<Li>
					On <strong>Cursor.directory</strong> pages, merged stdio command strings
					are normalized before the draft reaches the desktop import flow.
				</Li>
			</Ul>

			<H2>Catalog loading</H2>
			<Ul>
				<Li>Discovery entries come from the MCPMate Public Discovery API.</Li>
				<Li>Server and client tabs load paginated entries as the popup scrolls.</Li>
				<Li>The popup caches discovery responses locally for faster repeat opens.</Li>
				<Li>Language follows the browser language on first open and can be changed in popup settings.</Li>
			</Ul>

			<H2>Install links</H2>
			<Ul>
				<Li>
					Chrome Web Store:{" "}
					<a
						href="https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
				<Li>
					Microsoft Edge Add-ons:{" "}
					<a
						href="https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
			</Ul>

			<H2>How it fits with MCPMate</H2>
			<H3>From web discovery to local control</H3>
			<P>
				The extension is the web-side entry point. MCPMate desktop remains the place
				where imported servers are previewed, validated, stored, enabled, and added
				to Profiles or client rollout flows.
			</P>

			<Callout type="info" title="Same import path">
				Extension capture, drag-and-drop, paste, and Market installs all converge on
				the Server Install Wizard so every server can be reviewed before it becomes
				part of your local workspace.
			</Callout>
		</DocLayout>
	);
}
