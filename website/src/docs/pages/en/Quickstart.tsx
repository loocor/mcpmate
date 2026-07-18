import { useMemo } from "react";
import { Link } from "react-router-dom";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";
import Callout from "../../components/Callout";
import CommunityLinks from "../../components/CommunityLinks";
import CopyableInlineCode from "../../components/CopyableInlineCode";
import DesktopDownloadList from "../../components/DesktopDownloadList";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

const howToSteps = [
	{
		name: "Install and launch MCPMate",
		text: "Download the desktop package for your platform, install it, and open MCPMate.",
	},
	{
		name: "Complete onboarding",
		text: "Let MCPMate detect your existing clients and servers, then confirm the setup you want to keep.",
	},
	{
		name: "Add your first server",
		text: "Choose a server from Market or import an existing MCP configuration and review it before installation.",
	},
	{
		name: "Connect a client",
		text: "Use the Default profile to expose the server to a detected AI client.",
	},
	{
		name: "Verify the connection",
		text: "Open the client and run one simple MCP action to confirm that the server capability is available.",
	},
];

export default function Quickstart() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "How to get started with MCPMate",
				description:
					"Install MCPMate, complete onboarding, add one MCP server, and make it available in an AI client.",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout
			meta={{
				title: "Quick Start",
				description: "Go from installation to your first working MCP server in a few steps.",
			}}
		>
			<SchemaOrg schema={howTo} />
			<P>
				This guide follows one short path: install MCPMate, complete the first-run
				setup, add one server, and confirm that it works in your AI client.
			</P>

			<H2>Start with the desktop app</H2>
			<P>
				Choose the installer for your operating system and processor. These links use
				MCPMate&apos;s tracked download service and resolve to the current release assets.
			</P>
			<DesktopDownloadList locale="en" />
			<Callout type="info" title="Prefer Homebrew?">
				macOS and Linux users can install MCPMate with{" "}
				<CopyableInlineCode
					copyLabel="Copy command"
					copiedLabel="Copied"
					errorLabel="Copy failed"
				>
					brew install --cask loocor/tap/mcpmate@beta
				</CopyableInlineCode>
				. See the{" "}
				<Link className="font-medium underline" to="/docs/en/installation">
					Installation guide
				</Link>{" "}
				for supported systems, updates, and uninstall steps.
			</Callout>

			<H2>Complete onboarding</H2>
			<Ul>
				<Li>Open MCPMate after installation and continue through the welcome flow.</Li>
				<Li>Review the AI clients and MCP Servers detected on your machine.</Li>
				<Li>
					Keep the detected setup you want to use, or choose a starter Server if this is
					your first MCP setup.
				</Li>
			</Ul>

			<H2>Add your first Server</H2>
			<Ul>
				<Li>
					Open <strong>Market</strong> and choose a Server, or open <strong>Servers</strong>{" "}
					to import a configuration you already have.
				</Li>
				<Li>Review the detected command, arguments, and required values.</Li>
				<Li>Run the preview check, then confirm the installation.</Li>
			</Ul>

			<H2>Connect it to your client</H2>
			<Ul>
				<Li>
					Open <strong>Profiles</strong>, select <strong>Default</strong>, and make sure your
					new Server is included.
				</Li>
				<Li>
					Open <strong>Clients</strong>, choose a detected AI app, and apply the Default
					profile using the setup recommended by MCPMate.
				</Li>
			</Ul>

			<H2>Verify your first capability</H2>
			<P>
				Open or restart the connected AI client, then ask it to perform one simple
				action provided by the Server. When the client can see and call that capability,
				your first MCPMate setup is complete.
			</P>

			<H2>Join the community</H2>
			<P>Get help, share what you are building, or tell us what should improve next.</P>
			<CommunityLinks locale="en" />
		</DocLayout>
	);
}
