import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "Server Detail Overview",
				description: "Inspect server health, instance state, and lifecycle actions",
			}}
		>
			<P>
				The browse view on <code>/servers/:serverId</code> is where you decide whether
				a server is healthy enough to keep in rotation. It combines state, transport,
				instance information, and lifecycle actions such as enable, disable, edit,
				or delete.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/server-detail-light.png"
				darkSrc="/screenshot/server-detail-dark.png"
				alt="Server detail overview"
			/>

			<H2>What to confirm first</H2>
			<Ul>
				<Li>Connection state and whether the status is transitional or stable.</Li>
				<Li>Instance count, especially when the same server exposes multiple transports.</Li>
				<Li>Whether an edit or restart would affect clients that already depend on it.</Li>
			</Ul>

			<H3>Why the overview comes before capability tabs</H3>
			<P>
				If the server itself is unhealthy, capability lists are secondary symptoms.
				Stabilize the lifecycle first, then move into capability review or debug mode.
			</P>

			<Callout type="warning" title="Refresh is not the same as enable">
				Refreshing capabilities re-pulls metadata. Enabling or disabling the server
				changes runtime availability. Use the right action for the problem you are
				actually solving.
			</Callout>
		</DocLayout>
	);
}
