import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function RuntimeEN() {
	return (
		<DocLayout
			meta={{
				title: "Runtime",
				description: "Runtime controls and health",
			}}
		>
			<P>
				The Runtime screen exposes the embedded environments MCPMate manages for
				MCP servers (currently <strong>uv</strong> and <strong>Bun</strong>). Use
				it to confirm installations, clear caches, and reset capability state
				when testing new transports or server upgrades.
			</P>

			<H2>Runtime status cards</H2>
			<Ul>
				<Li>
					Each runtime card shows availability, version, install folder, last
					status message, cache size, package count, and last modification time.
				</Li>
				<Li>
					The <strong>Install / Repair</strong> button runs the installer with{" "}
					<code>verbose=true</code> so you can see detailed logs in the backend
					console. Use this after clearing caches or when a runtime fails health
					checks.
				</Li>
				<Li>
					The <strong>Reset Cache</strong> button wipes downloaded packages for
					that runtime only. MCPMate will rehydrate the cache on the next server
					request.
				</Li>
			</Ul>

			<H2>Capability cache controls</H2>
			<P>
				The bottom card summarizes the capability cache database (path, size,
				last cleanup) and counts for servers, tools, resources, prompts, and
				resource templates. It also tracks hit/miss metrics with a ratio to help
				evaluate cache efficiency.
			</P>
			<Ul>
				<Li>
					Use <strong>Reset Capabilities</strong> when you change server
					manifests or want Inspector to refetch tool metadata.
				</Li>
				<Li>
					Clearing the cache invalidates signatures immediately; subsequent
					requests repopulate entries with the latest data from the proxy.
				</Li>
			</Ul>

			<H2>Recommended maintenance flow</H2>
			<H3>Before importing new servers</H3>
			<P>
				Verify both runtimes show <em>running</em> with sensible versions. If the
				status badge is <em>stopped</em>, run Install / Repair first.
			</P>

			<H3>After heavy profile edits</H3>
			<P>
				Clear the capability cache to avoid stale prompts or resource templates.
				Once the cache repopulates, rerun the Inspector checklist to confirm list
				responses stay under the 5 second target.
			</P>

			<Callout type="warning" title="Cache resets remove downloaded packages">
				Resetting uv or Bun caches deletes the virtual environment contents. Any
				subsequent server call will reinstall dependencies, which may take time.
				Schedule resets during maintenance windows or before running automated
				tests rather than while end users are connected.
			</Callout>
		</DocLayout>
	);
}
