import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Quickstart() {
	return (
		<DocLayout
			meta={{ title: "Quick Start", description: "Install, configure, and apply MCPMate" }}
		>
			<P>
				This guide walks you through installing the desktop preview of MCPMate on macOS, adding
				servers, preparing profiles, and applying them inside your MCP clients.
			</P>

			<H2>Download and install</H2>
			<Callout type="info" title="Preview availability">
				MCPMate is currently offered as a macOS preview build. Windows and Linux packages are not
				available yet.
			</Callout>
			<Ul>
				<Li>Visit the MCPMate website and choose <strong>Download</strong>.</Li>
				<Li>Open the downloaded DMG and drag <strong>MCPMate</strong> into <strong>Applications</strong>.</Li>
				<Li>Launch MCPMate. macOS may prompt you to confirm opening an app from an identified developer.</Li>
				<Li>On first launch, MCPMate scans for supported clients such as Cursor, Claude Desktop, and Zed.</Li>
			</Ul>

			<H2>Install MCP servers</H2>
			<P>Pick the approach that matches the services you want to use.</P>
			<H3>Browse the built-in marketplace</H3>
			<Ul>
				<Li>Open <strong>Market</strong> from the left sidebar.</Li>
				<Li>Search or filter for a server and select <strong>Install</strong> to add it to your workspace.</Li>
			</Ul>
			<H3>Drag and drop external bundles</H3>
			<Ul>
				<Li>From <strong>Servers</strong>, choose <strong>Add</strong> and drop MCP bundles or JSON/TOML snippets into the window.</Li>
				<Li>Review the preview, then confirm the import to create the server entry.</Li>
			</Ul>
			<H3>Import servers from an existing client</H3>
			<Ul>
				<Li>Open <strong>Clients</strong> and pick a detected client.</Li>
				<Li>Use the <strong>Import from client</strong> action to bring existing MCP configuration into MCPMate.</Li>
			</Ul>

			<H2>Organize profiles</H2>
			<P>
				Profiles decide which servers and capabilities are exposed to your clients. MCPMate ships with a
				<strong> Default</strong> profile, and you can create more for specific scenarios.
			</P>
			<Ul>
				<Li>Go to <strong>Profiles</strong> and open the <strong>Default</strong> profile.</Li>
				<Li>Add the servers you installed, enable or disable the tools, prompts, and resources you need.</Li>
				<Li>
					Use <strong>New Profile</strong> to build additional presets (for example, Writing or Data
					Exploration) and tailor the enabled capabilities.
				</Li>
			</Ul>

			<H2>Apply profiles inside clients</H2>
			<Ul>
				<Li>
					In <strong>Clients</strong>, ensure your editor appears as <strong>Detected</strong>. If it is not, reinstall the client or review the path.
				</Li>
				<Li>
					Set the client to <strong>Hosted</strong> mode to enable in-place profile switching from MCPMate.
					Transparent mode only writes configuration files and cannot toggle profiles live.
				</Li>
				<Li>Select the profile you prepared and apply it. Launch your editor and trigger an MCP command to confirm the tools appear.</Li>
			</Ul>

			<H2>Troubleshooting runtime</H2>
			<Ul>
				<Li>
					If servers fail to start or tools return errors, open the <strong>Runtime</strong> page and use
					<strong> Install / Repair</strong> to provision required runtimes (uv, Bun).
				</Li>
				<Li>Clear caches from the same page if you suspect stale data.</Li>
			</Ul>

			<H2>Preview limitations and updates</H2>
			<P>
				This preview build is still evolving. Review the <strong>Changelog</strong> page for the latest features,
				bug fixes, and known issues before adopting new releases. If you encounter problems not listed there,
				please let us know so we can include them in upcoming fixes.
			</P>
		</DocLayout>
	);
}
