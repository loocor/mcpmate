import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientConfiguration() {
	return (
		<DocLayout
			meta={{
				title: "Client Configuration",
				description: "Choose how MCPMate writes and sources client capability configuration",
			}}
		>
			<P>
				The Configuration tab is where you decide how a client should consume profile
				state. It combines management mode, capability source, apply actions, and
				import preview so you can control both the desired future state and the
				current file on disk.
			</P>

			<H2>Key choices</H2>
			<Ul>
				<Li><strong>Unify mode</strong> is best when you want session-local control through builtin MCP and UCAN tools without maintaining a dashboard-side client working set.</Li>
				<Li><strong>Hosted mode</strong> is best when you want MCPMate features such as live switching and finer policy control.</Li>
				<Li><strong>Transparent mode</strong> is best when you must write explicit server config into the client and accept fewer MCPMate-side controls.</Li>
				<Li><strong>Capability source</strong> determines whether Hosted or Transparent workflows follow active profiles, selected shared profiles, or a client-specific custom profile.</Li>
			</Ul>

			<H2>What each mode really means</H2>
			<Ul>
				<Li><strong>Unify</strong> starts with builtin MCP tools only, uses session-local builtin tooling to browse capabilities from globally enabled servers, and resets when the session ends.</Li>
				<Li><strong>Hosted</strong> gives the client one MCPMate-managed endpoint, so MCPMate can keep policy, profile switching, and visibility logic in the middle.</Li>
				<Li><strong>Transparent</strong> writes enabled servers directly into the client&apos;s own MCP configuration for compatibility or special-case control.</Li>
			</Ul>

			<H2>Source selection applies to Hosted and Transparent workflows</H2>
			<Ul>
				<Li><strong>Unify</strong> does not use dashboard profile selection here. Use the builtin UCAN tools during the current session to browse and call capabilities from globally enabled servers.</Li>
				<Li><strong>Activated</strong> follows the globally active profile set.</Li>
				<Li><strong>Profiles</strong> lets one client follow selected shared profiles even when the global active set is different.</Li>
				<Li><strong>Customize</strong> creates or reuses a client-specific custom profile.</Li>
			</Ul>

			<H2>Recommended workflow</H2>
			<Ul>
				<Li>Choose Unify when you want session-scoped builtin control, Hosted when you want durable managed rollout, or Transparent when you need direct client config output.</Li>
				<Li>Select the capability source only when you are using Hosted or Transparent.</Li>
				<Li>Preview or import when you need to compare existing client config before overwriting it.</Li>
				<Li>Apply only after the Overview tab shows the client is detected and reachable.</Li>
			</Ul>

			<Callout type="warning" title="Transparent mode changes the trade-off">
				Transparent mode writes server config directly into the client, which is useful
				for compatibility but reduces what MCPMate can control at capability level.
			</Callout>

			<Callout type="info" title="Why hosted mode feels more powerful">
				Hosted mode is the path that keeps MCPMate&apos;s built-in profile and client tools,
				client-aware visibility logic, and richer policy decisions in play. Transparent
				mode is intentionally simpler: it favors direct config output over runtime-aware
				control.
			</Callout>
		</DocLayout>
	);
}
