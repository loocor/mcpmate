import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

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

			<H2>Control levels</H2>
			<Ul>
				<Li>
					<strong>Server Level:</strong> Enable or disable entire servers
				</Li>
				<Li>
					<strong>Capability Level:</strong> Toggle tools, prompts, and resources
					independently
				</Li>
				<Li>
					<strong>Individual Item Level:</strong> Control specific tools or
					prompts within a server
				</Li>
			</Ul>

			<H2>Use cases</H2>
			<Ul>
				<Li>
					<strong>Security:</strong> Disable potentially dangerous operations
				</Li>
				<Li>
					<strong>Performance:</strong> Reduce overhead by disabling unused
					features
				</Li>
				<Li>
					<strong>Focus:</strong> Hide irrelevant tools to reduce clutter
				</Li>
				<Li>
					<strong>Testing:</strong> Gradually enable features during development
				</Li>
			</Ul>

			<H2>Benefits</H2>
			<P>
				Granular controls give you precise command over what functionality is
				available to your clients. This is especially valuable in team
				environments where different users may need different levels of access.
			</P>

			<H2>Recommended rollout pattern</H2>
			<Ul>
				<Li>Enable new capabilities in a limited profile first.</Li>
				<Li>Validate via Inspector, then apply profile to more clients.</Li>
				<Li>Use Audit Logs to confirm exactly when toggles changed.</Li>
			</Ul>
		</DocLayout>
	);
}
