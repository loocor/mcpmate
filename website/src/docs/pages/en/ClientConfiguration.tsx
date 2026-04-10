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
				<Li><strong>Unify mode</strong> is best when you want session-local control through builtin MCP and UCAN tools. Unify direct exposure is configured here and only applies to Unify sessions.</Li>
				<Li><strong>Hosted mode</strong> is best when you want MCPMate features such as live switching and finer policy control.</Li>
				<Li><strong>Transparent mode</strong> is best when you must write explicit server config into the client and accept fewer MCPMate-side controls.</Li>
				<Li><strong>Capability source</strong> determines whether Hosted or Transparent workflows follow active profiles, selected shared profiles, or a client-specific custom profile.</Li>
			</Ul>

			<H2>Governance and write eligibility are separate</H2>
			<Ul>
				<Li><strong>Allow / Deny</strong> decides whether a client may enter MCPMate&apos;s allowed capability circle. This is a safety control, not a configuration editor lock.</Li>
				<Li>You can still adjust management mode, capability source, and client metadata while a client is denied or suspended.</Li>
				<Li>Actually writing a client&apos;s own MCP config file is a different gate: Hosted, Unify, and Transparent only apply to disk after the client has a verified writable local config target.</Li>
				<Li>Saving governance state alone never creates a new client config file or upgrades an unverified path into a trusted write target.</Li>
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

			<H2>Unify direct exposure (Unify-only)</H2>
			<P>
				Direct exposure is scoped to Unify. Hosted and Transparent behavior is unchanged.
			</P>

			<Ul>
				<Li><strong>All Proxy</strong> (default): all enabled servers, including direct-eligible ones, stay brokered through the builtin UCAN tools.</Li>
				<Li><strong>Server Direct</strong>: directly expose all capabilities from selected servers that are marked eligible for Unify direct exposure.</Li>
				<Li><strong>Capability-Level Direct</strong> (advanced): directly expose selected tools only. In v1 this is tools-only, prompts/resources/templates remain brokered.</Li>
			</Ul>

			<P>
				Capability-Level Direct now opens a dedicated client-scoped editor page instead of reusing the Profiles route. This keeps the navigation state accurate while preserving the same bulk-editing workflow for tools.
			</P>

			<Callout type="warning" title="Mixed routing warning">
				Capability Level can split a workflow between brokered and direct tool calls.
				If an upstream server expects stateful sequences, mixed routing may cause unexpected results.
				MCPMate shows a warning for this case but does not automatically block or resolve it.
			</Callout>

			<H2>Recommended workflow</H2>
			<Ul>
				<Li>Use the New / Edit drawer to declare whether the client truly has a local MCP config file, then let MCPMate verify that path before using it as a write target.</Li>
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
