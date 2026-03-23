import { P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function ProtocolBridging() {
	return (
		<DocLayout
			meta={{
				title: "Protocol Bridging",
				description:
					"Connect stdio-based clients to Streamable HTTP services without client modification",
			}}
		>
			<P>
				MCPMate's protocol bridging capability allows stdio-based MCP clients to
				connect to Streamable HTTP based services without any
				modification to the client code. This enables greater flexibility in how
				you deploy and use MCP servers.
			</P>

			<h2>How It Works</h2>
			<P>
				MCPMate acts as a transparent bridge between different transport
				protocols. When a stdio-based client connects to MCPMate, it can
				communicate with Streamable HTTP servers as if they were native stdio
				servers. The protocol translation happens seamlessly in the background.
				Legacy SSE-style configurations are accepted at import boundaries and
				normalized to Streamable HTTP.
			</P>

			<h2>Use Cases</h2>
			<ul>
				<li>
					<strong>Remote Server Access:</strong> Connect local clients to
					cloud-hosted MCP servers
				</li>
				<li>
					<strong>Hybrid Deployments:</strong> Mix local and remote servers in
					the same workflow
				</li>
				<li>
					<strong>Legacy Client Support:</strong> Use modern SSE servers with
					older stdio-only clients
				</li>
				<li>
					<strong>Service Migration:</strong> Gradually migrate from stdio to
					Streamable HTTP without client disruption
				</li>
			</ul>

			<h2>Benefits</h2>
			<ul>
				<li>No client code changes required</li>
				<li>Unified interface for all transport types</li>
				<li>Enables flexible deployment architectures</li>
				<li>Future-proof your MCP infrastructure</li>
			</ul>

			<P>Content coming soon with protocol bridging configuration examples.</P>
		</DocLayout>
	);
}
