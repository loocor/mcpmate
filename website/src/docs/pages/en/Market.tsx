import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";
import DiscoveryPortalList from "../../components/DiscoveryPortalList";

export default function MarketEN() {
	return (
		<DocLayout
			meta={{
				title: "Market",
				description: "Browse and manage community servers",
			}}
		>
			<P>
				The Market connects MCPMate to the official MCP Registry. From here you
				can browse listings, preview metadata, hide noisy entries, and send
				candidates straight into the install wizard.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="MCP Market with server listings and search"
			/>

			<H2>Registry & data</H2>
			<Ul>
				<Li>
					The Market lists the official MCP Registry. Search (with debounced
					input) and sorting (Recent, Alphabetical) run client-side against
					cached pages while the app streams additional pages on demand.
				</Li>
				<Li>
					To import server snippets from arbitrary websites, use the{" "}
					<strong>MCPMate Discovery Browser Extension</strong> (
					<code>extension/browser</code>) which opens{" "}
					<code>mcpmate://import/server</code> on the desktop app.
				</Li>
				<Li>
					Use Settings → <strong>MCP Market</strong> to choose the default
					market, manage hidden entries, and review browser extension shortcuts.
				</Li>
			</Ul>

			<H2 id="discovery-portals">Discovery portals</H2>
			<P>
				The Discovery Portal catalog highlights MCP directories and community
				entry points that pair well with MCPMate imports. The list below is
				loaded from the same Public Discovery API used by the browser extension,
				so the website, extension, and desktop import flow share one catalog
				source.
			</P>
			<DiscoveryPortalList
				copy={{
					loading: "Loading Discovery portals...",
					error: "Discovery portal data is updating. Visit the browser extension portal tab for the same catalog workflow.",
					empty: "Discovery portal data is being prepared.",
					visit: "Open portal",
					source: "Source",
				}}
			/>

			<H2>Installing from the market</H2>
			<H3>Preview drawer</H3>
			<P>
				Select a server card to open the preview drawer. It shows description,
				capability counts, transport types, environment variables, and bundled
				icons. A secondary button launches the Uni-Import wizard with the server
				pre-filled so you can tweak aliases before saving.
			</P>

			<H3>OAuth-enabled upstream servers</H3>
			<P>
				For upstream Streamable HTTP servers that require OAuth, the install
				wizard can prepare authorization metadata and open the provider login
				popup. After approval, MCPMate receives the callback, closes the popup,
				and continues in the same install flow.
			</P>

			<H3>Hide or blacklist items</H3>
			<P>
				Use the &ldquo;Hide&rdquo; action to move entries into your local market
				blacklist. Hidden servers disappear from the grid but remain recoverable
				from Settings → MCP Market should you need them later.
			</P>

			<H2>Blacklist</H2>
			<P>
				Manage hidden registry entries under Settings → MCP Market. Restoring an
				entry returns it to the grid.
			</P>

			<Callout type="info" title="Relationship with Servers page">
				Every installation flows through the same <strong>Server Install
					Wizard</strong> used for drag-and-drop imports. Anything you add from the
				Market immediately appears in the Servers list, where you can review the
				configuration, enable it globally, and then add it to the right profiles.
			</Callout>
		</DocLayout>
	);
}
