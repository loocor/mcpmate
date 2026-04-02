import DocLayout from "../../layout/DocLayout";
import { H3, P } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "Market Install Flow",
				description:
					"How registry cards flow into MCPMate's install wizard",
			}}
		>
			<P>
				This flow explains what happens after you choose a registry card in the
				Market. MCPMate opens the same install wizard used by manual imports so
				you can review transport, normalize the manifest, and decide where the
				server belongs before saving.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="Inline marketplace browsing the official MCP registry"
			/>

			<h2>What the flow gives you</h2>
			<ul>
				<li>
					<strong>Registry-to-install handoff:</strong> Start from Market cards and continue in the guided install wizard
				</li>
				<li>
					<strong>Normalized preview:</strong> Review transport and manifest details before saving
				</li>
				<li>
					<strong>Controlled rollout:</strong> Add the server first, then decide which profiles should expose it
				</li>
				<li>
					<strong>Consistent import path:</strong> Market installs and drag-and-drop imports share the same downstream flow
				</li>
			</ul>

			<h2>Where it fits</h2>
			<P>
				Use Market when you want to browse the official registry inside MCPMate,
				and use this install flow when you are ready to inspect the server details
				before it lands in Servers.
			</P>

			<h2>Benefits</h2>
			<P>
				Instead of jumping between registry pages, snippets, and local config
				files, MCPMate keeps discovery and installation in one guided path.
			</P>

			<H3>Add MCP Server wizard</H3>
			<P>
				Installing from a registry card opens the guided flow: configure
				transport, preview the normalized manifest, save the server, then add it
				to the desired profiles from the Servers or Profiles pages.
			</P>
			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="Add MCP Server stepper with core configuration form"
			/>
		</DocLayout>
	);
}
