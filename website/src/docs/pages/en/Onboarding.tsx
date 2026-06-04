import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Onboarding() {
	return (
		<DocLayout
			meta={{
				title: "Onboarding",
				description:
					"Use MCPMate onboarding to detect clients, import existing servers, and start from curated Discovery presets.",
			}}
		>
			<P>
				Onboarding guides the first MCPMate session from local discovery to a
				usable workspace. It detects compatible clients, reviews existing MCP
				server configurations, and can load starter entries from MCPMate Public
				Discovery when a local setup is still empty.
			</P>

			<H2>Flow overview</H2>
			<Ul>
				<Li>Detect installed AI clients and their MCP configuration targets.</Li>
				<Li>Review servers found in existing local client configuration files.</Li>
				<Li>Select starter client presets and server entries from Public Discovery.</Li>
				<Li>Import selected servers into MCPMate and place them in the starter profile.</Li>
				<Li>Continue to Clients, Servers, Market, or Profiles for deeper setup.</Li>
			</Ul>

			<H2>Client discovery</H2>
			<P>
				The client step combines local detection with MCPMate Discovery presets.
				Detected apps show where MCPMate can read or write configuration. Preset
				entries help you add supported clients and prepare a client record before
				connecting the app.
			</P>

			<H2>Server selection</H2>
			<P>
				The server step can import entries found in local client files and can also
				show curated starter servers from Public Discovery. Selected servers are
				grouped into an import request so MCPMate can normalize the configuration
				and keep the result in the local server library.
			</P>

			<H3>What happens after selection</H3>
			<Ul>
				<Li>MCPMate saves the selected server definitions into the local workspace.</Li>
				<Li>Imported servers become available from the Servers page.</Li>
				<Li>Profiles can use those servers to shape which capabilities each client sees.</Li>
				<Li>Client setup can continue with Hosted, Unify, or Transparent rollout choices.</Li>
			</Ul>

			<Callout type="info" title="Discovery-backed starter data">
				Public Discovery gives onboarding a ready-to-use starting point for common
				clients and server entries. The same Admin-owned discovery data also powers
				the browser extension catalog tabs.
			</Callout>
		</DocLayout>
	);
}
