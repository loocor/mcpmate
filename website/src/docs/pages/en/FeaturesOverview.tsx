import { P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function FeaturesOverview() {
	return (
		<DocLayout
			meta={{
				title: "Features Overview",
				description: "Explore MCPMate's powerful features",
			}}
		>
			<P>
				MCPMate provides a comprehensive set of features designed to make working
				with MCP servers easier, more efficient, and more powerful.
			</P>

			<h2>Core Features</h2>
			<P>
				Our feature set spans from centralized configuration and resource
				optimization to advanced tooling and seamless integrations. Each feature
				is designed with user experience and developer productivity in mind.
			</P>

			<h3>Configuration & Management</h3>
			<ul>
				<li>
					<strong>Centralized Configuration:</strong> Configure once, use
					everywhere across all your clients
				</li>
				<li>
					<strong>Seamless Context Switching:</strong> Instantly switch between
					different work scenarios
				</li>
				<li>
					<strong>Granular Controls:</strong> Fine-tune every capability with
					precise toggles
				</li>
				<li>
					<strong>Core Server + UI Separation:</strong> Run the backend core
					service independently and connect web/desktop UI shells as needed
				</li>
			</ul>

			<h3>Performance & Optimization</h3>
			<ul>
				<li>
					<strong>Resource Optimization:</strong> Intelligent server resource
					management for better performance
				</li>
				<li>
					<strong>Protocol Bridging:</strong> Connect stdio-based clients to
					Streamable HTTP services without modification
				</li>
			</ul>

			<h3>Developer Tools</h3>
			<ul>
				<li>
					<strong>Inspector:</strong> Deep insights into server status, logs, and
					diagnostics
				</li>
				<li>
					<strong>Auto Discovery & Import:</strong> Automatically detect and
					import existing configurations
				</li>
				<li>
					<strong>Uni-Import:</strong> Easy configuration through drag-and-drop
					or paste
				</li>
				<li>
					<strong>Audit Logs:</strong> Track management actions and MCP activity
					with filterable, cursor-paginated history
				</li>
			</ul>

			<h3>Ecosystem</h3>
			<ul>
				<li>
					<strong>Market Install Flow:</strong> Guided handoff from the official MCP Registry into MCPMate's install flow —
					discover and install servers without leaving the app
				</li>
			</ul>

			<P>
				Explore each feature in detail through the sections below to learn how
				MCPMate can enhance your MCP workflow.
			</P>
		</DocLayout>
	);
}
