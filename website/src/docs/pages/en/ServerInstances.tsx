import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";

export default function ServerInstances() {
	return (
		<DocLayout
			meta={{
				title: "Server Instances",
				description: "Investigate one concrete server transport instance at a time",
			}}
		>
			<P>
				Instance routes are useful when a single transport is unhealthy even though the
				server card still looks broadly available. Treat this page as the per-instance
				troubleshooting view rather than a general capability browser.
			</P>

			<H2>Typical situations</H2>
			<Ul>
				<Li>A multi-transport server has one failing connection but others remain healthy.</Li>
				<Li>You need a bookmarkable page for a specific runtime instance during QA.</Li>
				<Li>You are correlating one instance with runtime logs or audit events.</Li>
			</Ul>

			<H2>How to use it well</H2>
			<Ul>
				<Li>Start from the server overview so you know whether the issue is global or local.</Li>
				<Li>Use instance pages to isolate transport-specific behavior before restarting the whole server.</Li>
				<Li>Return to the capability tabs only after the instance state is stable again.</Li>
			</Ul>
		</DocLayout>
	);
}
