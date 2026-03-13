import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

export default function GranularControls() {
	return (
		<DocLayout
			meta={{
				title: "Granular Controls",
				description:
					"Toggle individual capabilities on/off - enable or disable as needed",
			}}
		>
			<P>
				MCPMate provides fine-grained control over every aspect of your MCP
				servers. Instead of an all-or-nothing approach, you can selectively
				enable or disable individual tools, prompts, and resources within each
				server.
			</P>

			<h2>Control Levels</h2>
			<ul>
				<li>
					<strong>Server Level:</strong> Enable or disable entire servers
				</li>
				<li>
					<strong>Capability Level:</strong> Toggle tools, prompts, and resources
					independently
				</li>
				<li>
					<strong>Individual Item Level:</strong> Control specific tools or
					prompts within a server
				</li>
			</ul>

			<h2>Use Cases</h2>
			<ul>
				<li>
					<strong>Security:</strong> Disable potentially dangerous operations
				</li>
				<li>
					<strong>Performance:</strong> Reduce overhead by disabling unused
					features
				</li>
				<li>
					<strong>Focus:</strong> Hide irrelevant tools to reduce clutter
				</li>
				<li>
					<strong>Testing:</strong> Gradually enable features during development
				</li>
			</ul>

			<h2>Benefits</h2>
			<P>
				Granular controls give you precise command over what functionality is
				available to your clients. This is especially valuable in team
				environments where different users may need different levels of access.
			</P>

			<P>Content coming soon with detailed control interface documentation.</P>
		</DocLayout>
	);
}
