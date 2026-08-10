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

			<H3>Validate transport before runtime use</H3>
			<P>
				Server definitions persist an explicit <strong>stdio</strong> or <strong>HTTP</strong>
				transport. MCPMate validates that definition before it reaches connection,
				OAuth, capability, client-export, or Inspector paths.
			</P>
			<P>
				An invalid or unrecognized legacy transport remains visible here so you can
				repair it, but it cannot be used by the runtime until the definition is valid.
				Open <strong>Edit</strong> and save a valid stdio or HTTP definition before
				retrying connection or capability discovery.
			</P>

			<Callout type="warning" title="Refresh is not the same as enable">
				Refreshing capabilities re-pulls metadata. Enabling or disabling the server
				changes runtime availability. Use the right action for the problem you are
				actually solving.
			</Callout>
		</DocLayout>
	);
}
