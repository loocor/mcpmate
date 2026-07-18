import Callout from "../../components/Callout";
import DesktopDownloadList from "../../components/DesktopDownloadList";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

const installCommand = "brew install --cask loocor/tap/mcpmate@beta";
const upgradeCommand = "brew upgrade --cask loocor/tap/mcpmate@beta";
const uninstallCommand = "brew uninstall --cask loocor/tap/mcpmate@beta";

function Command({ children }: { children: string }) {
	return (
		<pre className="not-prose overflow-x-auto rounded-lg border border-brand-border bg-brand-elevated p-3 text-sm text-brand-foreground">
			<code>{children}</code>
		</pre>
	);
}

export default function Installation() {
	return (
		<DocLayout
			meta={{
				title: "Installation",
				description: "Install, update, and uninstall MCPMate on macOS, Windows, and Linux.",
			}}
		>
			<P>
				Use an official desktop installer on macOS, Windows, or Linux. Homebrew is
				also available for macOS and Linux when you prefer a command-line package
				workflow.
			</P>

			<H2 id="supported-systems">Supported systems</H2>
			<Ul>
				<Li>macOS on ARM64 (Apple Silicon) and x64 (Intel).</Li>
				<Li>Windows on ARM64 and x64.</Li>
				<Li>Linux on ARM64 and x64.</Li>
				<Li>Homebrew on Linux requires Homebrew 5.1.12 or later.</Li>
			</Ul>

			<H2 id="install">Install</H2>
			<H3 id="desktop-install">Desktop installer</H3>
			<P>
				Choose the package that matches your operating system and architecture. The
				links below use MCPMate&apos;s tracked download service and resolve to the
				artifact from the current release.
			</P>
			<DesktopDownloadList locale="en" />
			<Ul>
				<Li>On macOS, open the DMG and move MCPMate to Applications.</Li>
				<Li>On Windows, run the MSI installer and follow the setup prompts.</Li>
				<Li>On Linux, install the DEB package with your system package manager.</Li>
			</Ul>

			<H3 id="homebrew">Homebrew</H3>
			<P>
				Use the fully qualified cask name. It installs the current Homebrew Beta
				channel without a separate <code>brew tap</code> step.
			</P>
			<Command>{installCommand}</Command>

			<H2 id="upgrade">Update</H2>
			<H3 id="desktop-upgrade">Desktop installer</H3>
			<P>
				Quit MCPMate, download the current package for the same platform and
				architecture, and run it over the existing installation. Your MCPMate user
				state remains in place.
			</P>
			<H3 id="homebrew-upgrade">Homebrew</H3>
			<Command>{upgradeCommand}</Command>

			<H2 id="uninstall">Uninstall</H2>
			<P>
				Quit MCPMate and allow its service to stop normally before uninstalling.
			</P>
			<H3 id="desktop-uninstall">Desktop installer</H3>
			<Ul>
				<Li>On macOS, move MCPMate from Applications to the Trash.</Li>
				<Li>On Windows, remove MCPMate from Settings &gt; Apps &gt; Installed apps.</Li>
				<Li>On Linux, remove MCPMate with the package manager used to install the DEB.</Li>
			</Ul>
			<H3 id="homebrew-uninstall">Homebrew</H3>
			<Command>{uninstallCommand}</Command>
			<Callout type="info" title="User state is preserved">
				Uninstalling the application preserves <code>~/.mcpmate</code>, including
				configuration, databases, and logs. Homebrew does not clean separately
				running background service state.
			</Callout>
			<P>
				There is currently no <code>mcpmate service uninstall</code> command. Stop
				the service normally before removing the application.
			</P>
		</DocLayout>
	);
}
