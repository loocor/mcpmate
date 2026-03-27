import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ClientDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "Client Detail Overview",
				description: "Review client state, integration readiness, and current server exposure",
			}}
		>
			<P>
				The Overview tab on <code>/clients/:identifier</code> tells you whether a
				client is detected, managed, and ready to receive MCPMate-controlled
				configuration. It is the right place to inspect transport support, product
				docs links, and the current server set before you apply configuration.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="Client detail overview"
			/>

			<H2>What this page is for</H2>
			<Ul>
				<Li>Confirm the client identity and whether MCPMate can currently manage it.</Li>
				<Li>Check supported transports before changing how the client connects.</Li>
				<Li>Review the current servers extracted from the client&apos;s effective config.</Li>
			</Ul>

			<H3>High-value actions</H3>
			<P>
				Use <strong>Refresh</strong> after installing or moving a client so MCPMate can
				rescan detection state. Use the managed toggle when you are ready for MCPMate
				to own the client configuration lifecycle.
			</P>

			<Callout type="info" title="Docs links on the overview are product specific">
				The Docs and Homepage links shown here come from the client metadata itself.
				They are useful when you need vendor-specific setup notes alongside MCPMate&apos;s
				own guidance.
			</Callout>

			<H2>Common questions</H2>
			<Ul>
				<Li><strong>Why does a client show as undetected?</strong> The app may not be installed in its expected path, or the backend lacks permission to scan for it.</Li>
				<Li><strong>Why do current servers differ from active profiles?</strong> Current servers reflect the client&apos;s present config, not just the desired target state.</Li>
			</Ul>
		</DocLayout>
	);
}
