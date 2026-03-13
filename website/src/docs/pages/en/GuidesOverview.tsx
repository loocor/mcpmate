import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function GuidesOverview() {
	return (
		<DocLayout
			meta={{
				title: "Guides Overview",
				description: "Learn how to use MCPMate effectively",
			}}
		>
			<P>
				This section mirrors the navigation inside the MCPMate dashboard web
				app (the <code>board/</code> project). Each guide explains the intent of
				that screen, the data sources it reads from the proxy, and the common
				workflows we exercise during validation.
			</P>

			<Callout type="info" title="How these guides map to the product">
				The Dashboard, Profiles, Clients, Servers, Market, Runtime, API Docs,
				and Settings articles follow the exact order of the sidebar links in the
				app. When you explore a page in this documentation, keep the dashboard
				open in another tab so you can try the steps in real time.
			</Callout>

			<H2>Primary navigation pillars</H2>
			<Ul>
				<Li>
					<strong>Dashboard</strong> &mdash; live health, active profile counts,
					and CPU / memory timelines refreshed every 10 seconds.
				</Li>
				<Li>
					<strong>Profiles</strong> &mdash; curate reusable suit bundles with
					granular enable toggles, statistics, and quick activation buttons.
				</Li>
				<Li>
					<strong>Clients</strong> &mdash; track detected editors, toggle managed
					mode, and audit MCP configuration paths.
				</Li>
				<Li>
					<strong>Servers</strong> &mdash; inspect capability summaries, enable or
					disable instances, and import new servers with Uni-Import or manual
					forms.
				</Li>
				<Li>
					<strong>Market</strong> &mdash; browse the official registry, connect to
					custom portals, preview metadata, and install entries straight into the
					server list.
				</Li>
				<Li>
					<strong>Runtime</strong> &mdash; monitor bundled runtimes (uv, Bun),
					reset caches, and view capability cache statistics.
				</Li>
				<Li>
					<strong>API Docs</strong> &mdash; open the generated REST and MCP schema
					reference once the proxy is running locally.
				</Li>
				<Li>
					<strong>Settings</strong> &mdash; adjust appearance, localization,
					default list views, developer options, and marketplace preferences.
				</Li>
			</Ul>

			<H2>Recommended learning flow</H2>
			<H3>1. Observe the current state</H3>
			<P>
				Start with the Dashboard and Runtime pages to confirm the proxy is
				healthy, which profiles are active, and how host resources behave under
				load. The metrics chart and status cards highlight issues quickly.
			</P>

			<H3>2. Configure what runs</H3>
			<P>
				Move on to Profiles, Servers, and Clients. These screens share the
				toolbar for search, sorting, list/grid toggles, and filters, so once you
				learn one you can operate the others. Use the drawers and detail pages to
				update metadata without losing your place.
			</P>

			<H3>3. Extend with new capabilities</H3>
			<P>
				Explore the Market to discover additional MCP servers, then confirm the
				runtime and capability caches are primed. Finish in Settings to lock in
				default views, marketplace portals, and the API Docs shortcut.
			</P>

			<H2>Shared UI patterns to notice</H2>
			<Ul>
				<Li>
					<strong>Stats cards</strong> &mdash; every management screen opens with
					a four-card summary (total, active/enabled, connectivity, instances).
				</Li>
				<Li>
					<strong>Page toolbar</strong> &mdash; consistent search, sort, and view
					switches with optional filters (e.g., client detected/managed modes).
				</Li>
				<Li>
					<strong>Action drawers</strong> &mdash; creation flows (new profile,
					server install wizard) slide in from the side so the context list stays
					visible.
				</Li>
				<Li>
					<strong>Inspect helpers</strong> &mdash; debug toggles reveal raw JSON,
					log extracts, or install payloads whenever deeper troubleshooting is
					required.
				</Li>
			</Ul>

			<Callout type="warning" title="Prerequisites">
				Run the backend proxy locally (<code>cargo run -p app-mcpmate</code> from
				<code>backend/</code>) or connect to an existing deployment before
				following these guides. Several cards and charts will remain blank if the
				API endpoints at <code>http://127.0.0.1:8080</code> are unreachable.
			</Callout>
		</DocLayout>
	);
}
