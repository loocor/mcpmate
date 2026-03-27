import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocLayout from "../../layout/DocLayout";

export default function Logs() {
	return (
		<DocLayout
			meta={{ title: "Logs", description: "Diagnostics and audit logs" }}
		>
			<P>
				The Audit Logs page provides a unified timeline of MCPMate operations so
				you can trace who changed what, when it happened, and which object was
				affected.
			</P>

			<H2>What gets logged</H2>
			<Ul>
				<Li>Profile lifecycle actions: create, update, activate, deactivate.</Li>
				<Li>Client-side operations: apply profile, backup, restore, mode updates.</Li>
				<Li>Server operations: create/import/update/toggle plus capability refreshes.</Li>
				<Li>Security-relevant records surfaced by the audit subsystem.</Li>
			</Ul>

			<H2>How to use the page</H2>
			<H3>Filter and narrow scope</H3>
			<P>
				Start with action/category filters and time range selection to isolate the
				window you are investigating. This is useful when many operators share one
				control plane.
			</P>

			<H3>Load history with cursor pagination</H3>
			<P>
				Audit entries use cursor-based pagination for stable traversal in
				high-volume environments. Continue loading from the current cursor instead
				of relying on offset pages.
			</P>

			<H3>Correlate with UI actions</H3>
			<P>
				When debugging production-like incidents, compare timestamps between audit
				entries and notifications shown in Dashboard, Clients, and Servers pages
				to reconstruct the exact sequence.
			</P>

			<Callout type="info" title="Operational recommendation">
				Review audit logs after major profile rollouts, server imports, or client
				mode changes. This catches accidental toggles early and speeds up incident
				root-cause analysis.
			</Callout>
		</DocLayout>
	);
}
