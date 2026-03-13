import { H2, H3, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";
import Callout from "../../components/Callout";

export default function Dashboard() {
	return (
		<DocLayout
			meta={{
				title: "Dashboard",
				description: "Overview of system status and key panels",
			}}
		>
			<P>
				The dashboard is the first screen you land on inside the MCPMate
				operational console. It combines uptime signals, profile/server/client
				counts, and time-series resource telemetry so you can decide what to
				investigate next before diving into the management views.
			</P>

			<Callout type="info" title="Refresh cadence">
				System status, profile, server, and client summaries poll the proxy every
				30 seconds. The metrics chart samples CPU and memory usage every 10
				seconds and retains the last 60 points in local storage so short reloads
				do not wipe the context.
			</Callout>

			<H2>Status overview cards</H2>
			<Ul>
				<Li>
					<strong>System Status</strong> displays uptime, kernel version, and the
					live health badge. Click the card to jump straight into the Runtime
					page for deeper repair actions.
				</Li>
				<Li>
					<strong>Profiles</strong> surfaces the total suits and how many are
					currently active. It respects the same data ordering as the Profiles
					page (default anchor first, then active suits) so discrepancies are
					easy to spot.
				</Li>
				<Li>
					<strong>Servers</strong> highlights how many servers exist, the subset
					enabled, and how many are currently connected. Follow the link to open
					the detailed server list with Uni-Import and per-instance controls.
				</Li>
				<Li>
					<strong>Clients</strong> shows detected and managed desktop editors.
					This reflects the same detection logic used on the Clients page, so it
					is a quick pulse check before pushing profile updates.
				</Li>
			</Ul>

			<H2>Resource metrics timeline</H2>
			<P>
				The bottom chart tracks four data series: MCPMate CPU %, MCPMate memory
				%, host CPU %, and host memory %. Hover to compare exact values or switch
				to dark mode to see the adaptive grid styling. When no samples are
				collected yet (for example, immediately after launch), the chart displays
				an empty state message instead of an empty axis.
			</P>

			<H3>Interpreting the lines</H3>
			<Ul>
				<Li>
					Use MCPMate CPU spikes to catch runaway servers or heavy inspection
					sessions.
				</Li>
				<Li>
					Compare MCPMate and host memory to estimate overhead versus total
					system pressure.
				</Li>
				<Li>
					The tooltip adds the live megabyte count next to memory percentages
					for quick conversion.
				</Li>
			</Ul>

			<H2>Suggested health checks</H2>
			<Ul>
				<Li>
					Confirm <strong>Status</strong> shows <em>running</em> with the expected
					version before enabling new profiles.
				</Li>
				<Li>
					Look for sudden drops in connected servers or managed clients to
					determine whether a restart disrupted components.
				</Li>
				<Li>
					Scan the metrics trend during load testing or Inspector loops to ensure
					CPU stays below your target threshold.
				</Li>
			</Ul>

			<Callout type="warning" title="If data appears frozen">
				Check that the dashboard is allowed to reach{" "}
				<code>http://127.0.0.1:8080</code>. If you paused the backend process,
				the cards retain the last known values until the next successful poll.
				Restarting the proxy or refreshing the page after the proxy is running
				will restore live updates.
			</Callout>
		</DocLayout>
	);
}
