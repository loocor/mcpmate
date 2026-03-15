import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "Inline Marketplace",
				description:
					"Built-in official MCP registry - discover servers without leaving the app",
			}}
		>
			<P>
			MCPMate includes an integrated marketplace that provides access to the
			official MCP registry. Discover, install, and
			configure new MCP servers without ever leaving the application.
			</P>

			<h2>Features</h2>
			<ul>
				<li>
					<strong>Unified Search:</strong> Search the official registry
				</li>
				<li>
					<strong>One-Click Install:</strong> Install servers directly from the
					marketplace
				</li>
				<li>
					<strong>Automatic Configuration:</strong> Servers are automatically
					added to your active profile
				</li>
				<li>
					<strong>Version Management:</strong> Update servers when new versions
					are available
				</li>
				<li>
					<strong>Ratings & Reviews:</strong> See community feedback before
					installing
				</li>
			</ul>

			<h2>Supported Registries</h2>
			<ul>
				<li>
					<strong>Official MCP Registry:</strong> Anthropic's official server
					collection
				</li>
			</ul>

			<h2>Benefits</h2>
			<P>
				Instead of manually searching GitHub or documentation sites, browsing
				installation instructions, and editing configuration files, the
				marketplace streamlines the entire process into a few clicks.
			</P>

			<P>Content coming soon with marketplace usage guide and screenshots.</P>
		</DocLayout>
	);
}
