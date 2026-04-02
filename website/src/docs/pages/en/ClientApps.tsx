import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

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

			<DocScreenshot
				lightSrc="/screenshot/clients-light.png"
				darkSrc="/screenshot/clients-dark.png"
				alt="Clients grid with detection and managed toggles"
			/>

			<H2>Guide map</H2>
			<Ul>
				<Li>
					<strong>Detail Overview</strong> explains status badges, detection,
					docs links, transport badges, and current-server cards.
				</Li>
				<Li>
					<strong>Configuration</strong> explains Unify, Hosted, and Transparent mode,
					capability source, apply flows, and import preview.
				</Li>
				<Li>
					<strong>Backups</strong> focuses on retention, rollback, bulk delete,
					and recovery guidance.
				</Li>
			</Ul>

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
				Click a card to open <code>/clients/:identifier</code>. The detail layout
				uses three tabs:
			</P>

			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="Client detail overview with config path and current servers"
			/>

			<Ul>
				<Li>
					<strong>Overview</strong> &mdash; detection status, managed mode,
					profile apply actions, and shortcuts (for example open the
					client&apos;s MCP config folder).
				</Li>
				<Li>
					<strong>Configuration</strong> &mdash; live view of MCP servers MCPMate
					would write for this client, import-from-client flows, and Unify /
					Hosted / Transparent mode guidance.
				</Li>
				<Li>
					<strong>Backups</strong> &mdash; rotating snapshots created when you
					apply profiles or imports. Restore a snapshot to roll back, delete
					selected backups, or refresh the list after a successful apply.
				</Li>
			</Ul>
			<P>
				Backup retention limits and default filters come from{" "}
				<strong>Settings → Client Defaults</strong>, so tune those before large
				rollouts.
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
