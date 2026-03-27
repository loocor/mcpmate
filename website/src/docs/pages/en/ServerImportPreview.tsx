import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerImportPreview() {
	return (
		<DocLayout
			meta={{
				title: "Server Import & Preview",
				description: "Use Uni-Import to ingest messy server snippets and verify real capabilities before installation",
			}}
		>
			<P>
				The Add Server flow is more than a blank form. MCPMate&apos;s Uni-Import lets you
				drop, paste, or load server snippets in different formats, normalizes them
				into a clean draft, and then runs a preview plus import validation before the
				server is actually installed.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="Server import wizard with configuration preview"
			/>

			<H2>Where this flow starts</H2>
			<Ul>
				<Li>The Add button on the Servers page opens the install wizard.</Li>
				<Li>The same button also acts as a drag-and-drop target for Uni-Import.</Li>
				<Li>After the browser extension is approved, compatible MCP snippets on the web can be captured and sent into MCPMate with one click.</Li>
				<Li>The wizard follows three steps: Configuration, Preview, and Import &amp; Profile.</Li>
			</Ul>

			<H2>Browser extension capture</H2>
			<P>
				MCPMate&apos;s Chrome and Edge extension is designed as an upstream entry into the
				same Uni-Import flow. It scans web pages for likely MCP configuration blocks,
				adds an <strong>Add to MCPMate</strong> action, and opens the desktop app
				through the <code>mcpmate://import/server</code> deep link with the snippet
				text, inferred format, and source page URL.
			</P>

			<Callout type="info" title="Why the extension matters">
				This removes one more manual step from installation. Instead of copying a code
				block, cleaning it, and pasting it into the Add Server drawer, users can send
				a detected MCP snippet straight into MCPMate and let Uni-Import continue from
				there.
			</Callout>

			<H2>What Uni-Import accepts</H2>
			<Ul>
				<Li>Plain text pasted from blogs, docs, registries, or chat snippets.</Li>
				<Li>JSON and JSON5 fragments, including fenced code blocks and top-level property lists.</Li>
				<Li>TOML snippets, including extracted sections and compact key-value windows.</Li>
				<Li>Bundle files such as <code>.mcpb</code> and <code>.dxt</code>.</Li>
			</Ul>

			<H2>Why dirty input still works</H2>
			<P>
				The normalizer does not rely on one exact syntax. It accepts multiple input
				shapes such as JSON, JSON5-style payloads, TOML snippets, and MCP bundles,
				then converts them into a consistent draft the install wizard can review.
			</P>

			<Callout type="info" title="This is why pasted snippets feel forgiving">
				Uni-Import is designed for real-world copy and paste. The goal is not to make
				the user pre-clean every snippet, but to salvage importable server structure
				from noisy text whenever the intent is still recognizable.
			</Callout>

			<H2>What Preview gives you before install</H2>
			<Ul>
				<Li>A real capability preview rather than only a config echo.</Li>
				<Li>Per-server summaries for tools, resources, prompts, and templates.</Li>
				<Li>Expandable details so you can inspect capability names and descriptions.</Li>
				<Li>Visibility into preview errors while still allowing you to decide whether to continue.</Li>
			</Ul>

			<H3>Why this matters</H3>
			<P>
				Preview makes the install step more transparent. Instead of trusting a raw
				configuration snippet, you can see what the target server appears to expose
				and catch mismatches before it becomes part of your working environment.
			</P>

			<H2>The final validation step</H2>
			<P>
				When you move to the last step, MCPMate performs a dry-run import. This tells
				you how many servers are ready to import, which ones would be skipped because
				they already exist, and whether any items fail validation before the real
				import button becomes available.
			</P>

			<H2>Recommended workflow</H2>
			<Ul>
				<Li>When the extension becomes available, prefer one-click capture from compatible documentation pages or registries.</Li>
				<Li>Drop or paste the snippet first; only switch to manual edits when the normalized draft needs correction.</Li>
				<Li>Read the Preview step for capability shape, not just install success.</Li>
				<Li>Use the validation step to catch duplicates or broken entries before importing.</Li>
				<Li>After import, continue to Profiles if the server should participate in managed exposure.</Li>
			</Ul>

			<H2>Common questions</H2>
			<Ul>
				<Li><strong>What does the browser extension actually send?</strong> It sends the snippet text, inferred format, and source URL into the same desktop import flow for traceable ingestion.</Li>
				<Li><strong>Why did MCPMate accept a messy snippet?</strong> The normalizer intentionally extracts importable structure from noisy text when possible.</Li>
				<Li><strong>What if preview shows issues?</strong> Treat preview as an early warning step. Review the reported problem and then use the final validation step to confirm whether import can proceed safely.</Li>
				<Li><strong>Why is Import disabled on the last step?</strong> Dry-run validation likely found no importable servers or encountered a blocking error.</Li>
			</Ul>
		</DocLayout>
	);
}
