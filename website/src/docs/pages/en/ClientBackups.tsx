import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientBackups() {
	return (
		<DocLayout
			meta={{
				title: "Client Backups",
				description: "Protect and restore client configuration safely",
			}}
		>
			<P>
				The Backups tab is your safety net for client configuration changes. Use it
				before large apply operations, after imports, or whenever an editor starts
				behaving differently from the intended profile assignment.
			</P>

			<H2>When backups matter most</H2>
			<Ul>
				<Li>Before switching a client from transparent to hosted mode.</Li>
				<Li>After importing configuration from an existing client file.</Li>
				<Li>When a rollout needs a fast rollback without reconstructing edits by hand.</Li>
			</Ul>

			<H2>How to use the tab</H2>
			<Ul>
				<Li>Refresh after apply operations so the latest snapshot is visible.</Li>
				<Li>Restore one snapshot when a client needs immediate recovery.</Li>
				<Li>Use bulk delete only after you verify retention policy and recovery needs.</Li>
			</Ul>

			<Callout type="info" title="Retention policy starts in Settings">
				The backup list reflects the strategy and limits configured under Settings →
				Client Defaults. Tune that policy before large migrations so this tab remains
				useful instead of noisy.
			</Callout>
		</DocLayout>
	);
}
