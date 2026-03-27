import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ProfileCapabilities() {
	return (
		<DocLayout
			meta={{
				title: "Profile Capability Tabs",
				description: "Fine-tune which servers and capabilities each profile exposes",
			}}
		>
			<P>
				The capability tabs are where a profile becomes operational policy instead of
				just metadata. Use them to decide which servers are attached to the profile
				and which tools, prompts, resources, or templates should remain visible once
				the profile is active.
			</P>

			<section id="servers">
				<H2>Servers</H2>
				<P>
					Start here when a profile should gain or lose access to an entire server.
					This tab is the broadest control surface and usually the right first move
					before you fine-tune any individual capability.
				</P>
				<P>
					When server debugging is enabled, each server row also exposes Browse and
					Inspect actions on hover. Use Inspect when you need to jump straight into
					the related server&apos;s live debug workbench without losing the profile
					context that led you there.
				</P>
			</section>

			<section id="tools">
				<H2>Tools</H2>
				<P>
					Use the Tools tab when a server stays enabled but certain callable actions
					should be hidden for a specific workflow or client audience.
				</P>
			</section>

			<section id="prompts">
				<H2>Prompts</H2>
				<P>
					Use Prompts when you want shared instruction assets in one profile but not
					another, especially for writing, coding, and analysis workflows.
				</P>
			</section>

			<section id="resources">
				<H2>Resources</H2>
				<P>
					Use Resources when a server should stay available but read access needs to be
					narrowed to the profiles that truly need it.
				</P>
			</section>

			<section id="templates">
				<H2>Resource Templates</H2>
				<P>
					Templates usually matter when a server generates structured resource entry
					points. Keep them aligned with the same exposure rules as the underlying
					server to avoid confusing partial access.
				</P>
			</section>

			<H2>Shared operating pattern</H2>
			<Ul>
				<Li>Filter first, then use bulk actions for safe, repeatable changes.</Li>
				<Li>Prefer disabling in the profile before disabling a whole server globally.</Li>
				<Li>Re-check affected clients after bulk edits, because merged runtime exposure can change immediately.</Li>
			</Ul>

			<Callout type="warning" title="Capability toggles can affect server visibility">
				Profile-level capability changes are not isolated labels. They influence what
				clients can actually call, and in some cases they also change how useful a
				server remains in the merged runtime.
			</Callout>
		</DocLayout>
	);
}
