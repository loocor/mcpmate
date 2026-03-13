import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Profile() {
	return (
		<DocLayout
			meta={{
				title: "Profiles",
				description: "Reusable presets of servers, tools and prompts",
			}}
		>
			<P>
				Profiles (internally called <em>suits</em>) let you bundle MCP servers,
				tools, resources, and prompts into named presets. The Profiles page in
				the dashboard provides a searchable catalog of every suit, quick stats,
				and activation toggles that sync directly with the proxy.
			</P>

			<Callout type="info" title="Default anchor suits">
				Suits tagged with the <code>default_anchor</code> role are pinned to the
				top of the list and cannot be disabled. They act as the fallback profile
				that guarantees core capabilities stay online.
			</Callout>

			<H2>Stats cards & toolbar</H2>
			<Ul>
				<Li>
					Review the four stats cards: active suits, enabled servers, enabled
					tools, and ready instances. These values aggregate across all active
					suits by calling the proxy&apos;s <code>/config/suits</code> APIs.
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

			<H2>Creating and editing suits</H2>
			<H3>New suit drawer</H3>
			<P>
				Click <strong>New Profile</strong> to launch the side drawer. The form
				supports friendly display names, descriptions, and optional suit types
				(e.g., writer, coder). If you arrive via a preset shortcut (such as
				<code>?type=writer</code>), the drawer preselects the appropriate template.
			</P>

			<H3>Detail pages</H3>
			<P>
				Selecting a suit card navigates to <code>/profiles/:id</code>, where you
				can inspect servers, tools, prompts, and resources assigned to that suit.
				The detail view exposes per-capability toggles and activity logs while
				preserving a breadcrumb back to the catalog.
			</P>

			<H2>Activation workflow</H2>
			<Ul>
				<Li>
					Every card and list item includes a toggle switch for enabling or
					pausing the suit. MCPMate immediately calls the activation/deactivation
					endpoints and surfaces toast notifications confirming the change.
				</Li>
				<Li>
					When multiple suits are active, their enabled servers, tools, and
					resources are merged at runtime. The stats cards keep you aware of the
					total footprint.
				</Li>
				<Li>
					Default anchor suits display a disabled toggle to prevent accidental
					removal of baseline configurations.
				</Li>
			</Ul>

			<Callout type="warning" title="Troubleshooting activation">
				If a toggle appears stuck, open the Runtime page to confirm the proxy is
				healthy, then reload the Profiles page with the refresh button. You can
				also inspect the suit via <code>/config/suits/:id</code> using the API
				Docs link to ensure the backend persisted the change.
			</Callout>
		</DocLayout>
	);
}
