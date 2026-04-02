import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function Profile() {
	return (
		<DocLayout
			meta={{
				title: "Profiles",
				description: "Reusable presets of servers, tools and prompts",
			}}
		>
			<P>
				Profiles let you bundle MCP servers, tools, resources, and prompts into
				named presets. The Profiles page provides a searchable catalog, quick
				stats, and activation toggles that sync directly with the core service.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="Profiles list with stats and default profile card"
			/>

			<H2>Guide map</H2>
			<Ul>
				<Li>
					<strong>Preset Templates</strong> explains the read-only preset route and
					when to clone instead of editing in place.
				</Li>
				<Li>
					<strong>Detail Overview</strong> focuses on the overview tab of
					<code>/profiles/:profileId</code>, including activation, default rules,
					and counters.
				</Li>
				<Li>
					<strong>Capability Tabs</strong> covers the server, tool, prompt,
					resource, and template tabs where exposure is tuned for each profile.
				</Li>
			</Ul>

			<Callout type="info" title="Default anchor profiles">
				Profiles tagged with the <code>default_anchor</code> role are pinned to the
				top of the list and cannot be disabled. They act as the fallback profile
				that guarantees core capabilities stay online.
			</Callout>

			<H2>Stats cards & toolbar</H2>
			<Ul>
				<Li>
					Review the four stats cards: active profiles, enabled servers, enabled
					tools, and ready instances. These values aggregate across all active
					profiles.
				</Li>
				<Li>
					The toolbar supports search (name/description), sorting (alphabetical
					or active first), and a grid/list toggle. Changing the view updates the
					global default stored in Settings so the same preference loads next time.
				</Li>
				<Li>
					Use the refresh icon to force a server poll and the plus icon to open
					the creation drawer without leaving the page.
				</Li>
			</Ul>

			<H2>Creating and editing profiles</H2>
			<H3>New profile drawer</H3>
			<P>
				Click <strong>New Profile</strong> to launch the side drawer. The form
				supports friendly display names, descriptions, and optional profile types
				(e.g., writer, coder). If you arrive via a preset shortcut (such as
				<code>?type=writer</code>), the drawer preselects the appropriate template.
			</P>

			<H3>Detail pages</H3>
			<P>
				Selecting a profile card navigates to <code>/profiles/:profileId</code>,
				where you inspect servers, tools, prompts, and resources assigned to that
				profile. The detail view exposes per-capability toggles and activity logs
				while preserving a breadcrumb back to the catalog.
			</P>
			<P>
				Built-in templates open under <code>/profiles/presets/:presetId</code>.
				They behave like read-optimized starting points: review the bundled
				servers and capabilities, then clone or customize into a full profile when
				you need editable copies.
			</P>

			<H2>Why shared profiles matter</H2>
			<P>
				Shared profiles are the reusable layer that clients can point at directly.
				Instead of reopening a complex configuration screen and toggling servers,
				tools, prompts, and resources one by one, you can prepare named working
				modes such as writing, frontend development, or research and then switch the
				whole bundle as one decision.
			</P>
			<Ul>
				<Li>They reduce repeated UI work when the same capability mix is needed across multiple clients.</Li>
				<Li>They narrow the visible capability surface to what the current task actually needs.</Li>
				<Li>They make it easier to keep expensive tools, prompts, and resources out of sessions that do not need them.</Li>
			</Ul>

			<H2>Shared profiles in client workflows</H2>
			<P>
				Profiles are the reusable working sets that client workflows can point at.
				In Hosted rollouts they support durable switching, and in broader client
				management flows they define which servers and capabilities should be
				visible without rebuilding the same setup over and over.
			</P>
			<Callout type="info" title="Why this matters operationally">
				The value is consistency: prepare a reusable working set once, then reuse
				it across clients instead of re-tuning servers, tools, prompts, and
				resources one by one.
			</Callout>

			<H2>Activation workflow</H2>
			<Ul>
				<Li>
					Every card and list item includes a toggle switch for enabling or
					pausing the profile. MCPMate immediately calls the activation/deactivation
					endpoints and surfaces toast notifications confirming the change.
				</Li>
				<Li>
					When multiple profiles are active, their enabled servers, tools, and
					resources are merged at runtime. The stats cards keep you aware of the
					total footprint.
				</Li>
				<Li>
					Default anchor profiles display a disabled toggle to prevent accidental
					removal of baseline configurations.
				</Li>
			</Ul>

			<Callout type="warning" title="Troubleshooting activation">
				If a toggle appears stuck, open the Runtime page to confirm the proxy is
				healthy, then reload the Profiles page with the refresh button. You can
				also verify profile status via API Docs to ensure the backend persisted
				the change.
			</Callout>
		</DocLayout>
	);
}
