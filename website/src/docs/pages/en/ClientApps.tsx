import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientApps() {
	return (
		<DocLayout
			meta={{
				title: "Client Apps",
				description: "Apps that integrate with MCPMate",
			}}
		>
			<P>
				The Clients screen tracks desktop applications that can talk to MCPMate
				(Cursor, Claude Desktop, Zed, etc.). It combines automatic discovery,
				management toggles, and configuration hints so you can keep editor
				integration in sync with the proxy.
			</P>

			<H2>Stats & filters</H2>
			<Ul>
				<Li>
					The stats cards count discovered clients, how many were detected on
					disk, how many are under managed mode, and how many already have an MCP
					configuration file.
				</Li>
				<Li>
					The toolbar offers search (display name, identifier, description),
					sort options (alphabetical, detected, managed), and a grid/list toggle
					that shares the same default stored in Settings.
				</Li>
				<Li>
					Use the filter dropdown to quickly show <em>All</em>, only{" "}
					<em>Detected</em>, or only <em>Managed</em> clients. The choice feeds
					back into the store, so subsequent visits load the same view.
				</Li>
			</Ul>

			<H2>Managing integration state</H2>
			<H3>Detection badges</H3>
			<P>
				Each card/list row surfaces a green <strong>Detected</strong> badge when
				MCPMate located the client binary. If the client is missing, use the
				refresh icon to trigger a rescan (<code>/clients?force_refresh=true</code>
				), install the app, and reload.
			</P>

			<H3>Managed toggle</H3>
			<P>
				The switch at the bottom-right of each item enables or disables managed
				mode. When enabled, MCPMate keeps the client&apos;s configuration in sync
				with the active profile set. The toggle updates immediately and shows a
				toast on success or failure.
			</P>

			<H3>Client details</H3>
			<P>
				Click a card to open <code>/clients/:identifier</code> for a detailed
				view. You can inspect detected MCP servers, version metadata, download
				links, and quick actions such as opening the configuration folder.
			</P>

			<Callout type="warning" title="When a client stays undetected">
				Make sure the client is installed in the default location and that the
				proxy process has permission to scan your Applications directory. On
				macOS, you may need to grant &ldquo;Full Disk Access&rdquo; to the MCPMate
				service. After adjusting permissions, press <strong>Refresh</strong> to
				force a rescan.
			</Callout>
		</DocLayout>
	);
}
