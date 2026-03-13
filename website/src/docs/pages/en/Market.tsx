import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function MarketEN() {
	return (
		<DocLayout
			meta={{
				title: "Market",
				description: "Browse and manage community servers",
			}}
		>
			<P>
				The Market connects MCPMate to curated registries of MCP servers. From
				here you can browse official listings, add your own portals, preview
				metadata, and send candidates straight into the install wizard.
			</P>

			<H2>Tabs & data sources</H2>
			<Ul>
				<Li>
					<strong>Official</strong> tab lists the MCPMate registry. Search (with
					debounced input) and sorting (Recent, Popular, Alphabetical) run
					client-side against cached pages while the app streams additional pages
					on demand.
				</Li>
				<Li>
					<strong>Portal</strong> tabs render inside an iframe so third-party
					portals can expose their own UI. Add new portals from Settings →
					Marketplace. Switching languages forces a soft refresh to request
					translated catalogues when available.
				</Li>
				<Li>
					Remote connectors surface under <em>Remote</em> options; they represent
					predefined endpoints (Git repos, zip bundles, etc.) that can be
					imported with one click.
				</Li>
			</Ul>

			<H2>Installing from the market</H2>
			<H3>Preview drawer</H3>
			<P>
				Select a server card to open the preview drawer. It shows description,
				capability counts, transport types, environment variables, and bundled
				icons. A secondary button launches the Uni-Import wizard with the server
				pre-filled so you can tweak aliases before saving.
			</P>

			<H3>Hide or blacklist items</H3>
			<P>
				Use the &ldquo;Hide&rdquo; action to move entries into your local market
				blacklist. Hidden servers disappear from the grid but remain recoverable
				from Settings → Marketplace should you need them later.
			</P>

			<H2>Portal management tips</H2>
			<Ul>
				<Li>
					Set a <strong>Default Market</strong> in Settings so the dashboard
					opens your preferred portal on load.
				</Li>
				<Li>
					Use the <em>Open Portal</em> button (top-right of the tabs) to launch
					the portal in a new window while keeping the dashboard view available
					for installation.
				</Li>
				<Li>
					When a portal requires authentication headers, define them in Settings
					so the iframe and install pipeline receive the same credentials.
				</Li>
			</Ul>

			<Callout type="info" title="Relationship with Servers page">
				Every installation flows through the same <strong>Server Install
				Wizard</strong> used for drag-and-drop imports. Anything you add from the
				Market immediately appears in the Servers list, where you can enable it
				per profile and monitor connectivity.
			</Callout>
		</DocLayout>
	);
}
