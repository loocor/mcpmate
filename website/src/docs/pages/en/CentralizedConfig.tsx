import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function CentralizedConfig() {
	return (
		<DocLayout
			meta={{
				title: "Centralized Configuration",
				description:
					"Configure once, use everywhere across all your MCP clients",
			}}
		>
			<P>
				One of MCPMate's core features is centralized configuration management.
				Instead of maintaining separate configurations for each MCP client, you
				can configure your servers once in MCPMate and have them automatically
				available across all connected clients.
			</P>

			<H2>Benefits</H2>
			<Ul>
				<Li>
					<strong>Single Source of Truth:</strong> All your MCP server
					configurations are managed in one place
				</Li>
				<Li>
					<strong>No Duplication:</strong> Eliminate the need to copy settings
					across different clients
				</Li>
				<Li>
					<strong>Consistent Experience:</strong> Ensure all clients use the same
					server configurations
				</Li>
				<Li>
					<strong>Easy Updates:</strong> Change configurations once and have them
					apply everywhere
				</Li>
			</Ul>

			<H2>How it works</H2>
			<P>
				MCPMate acts as a central hub for all your MCP servers. When you
				configure a server in MCPMate, it becomes available to all connected
				clients automatically. This eliminates the traditional workflow of
				manually editing configuration files for each client application.
			</P>

			<H2>Use cases</H2>
			<Ul>
				<Li>Using the same servers across Claude Desktop, Cursor, and VS Code.</Li>
				<Li>Managing team-wide server configurations.</Li>
				<Li>Onboarding new clients without reconfiguration.</Li>
			</Ul>

			<H2>With separated deployment mode</H2>
			<P>
				When core services run independently from the UI shell, centralized
				configuration remains the same operating model. The UI edits config, the
				core service persists it, and clients consume it through managed workflows.
			</P>
		</DocLayout>
	);
}
