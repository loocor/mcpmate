import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfileDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "Profile Detail Overview",
				description: "Read profile state, activation rules, and counters before making changes",
			}}
		>
			<P>
				The overview tab on <code>/profiles/:profileId</code> is the safest place to
				understand what a profile controls before you touch individual servers or
				capabilities. It summarizes status, type, multi-select behavior, priority,
				and quick actions such as refresh, edit, default, enable, disable, or delete.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="Profile detail overview and counters"
			/>

			<H2>What to confirm first</H2>
			<Ul>
				<Li><strong>Status</strong> tells you whether the profile currently contributes to the merged runtime.</Li>
				<Li><strong>Type</strong> tells you whether the profile is shared, host-app specific, or another special workflow bucket.</Li>
				<Li><strong>Priority</strong> matters when multiple active profiles overlap and you need predictable resolution.</Li>
			</Ul>

			<H2>Why the counters matter</H2>
			<Ul>
				<Li>The counters let you estimate impact before opening deeper tabs.</Li>
				<Li>They also help explain why a client sees more or fewer capabilities after a rollout.</Li>
				<Li>Clicking the cards is the quickest way to jump into the relevant tab when troubleshooting.</Li>
			</Ul>

			<H3>Quick actions</H3>
			<P>
				Refresh re-pulls the detail data, Edit reopens the profile form, and the
				activation or delete buttons change what the runtime exposes. Use them from
				the overview when you need a controlled change with full context.
			</P>

			<Callout type="warning" title="Default and anchor rules are intentional">
				Some profiles cannot be disabled or deleted because they protect a baseline
				capability set. If a button is unavailable, treat that as policy rather than
				a UI error and review the profile role first.
			</Callout>

			<H2>Common questions</H2>
			<Ul>
				<Li><strong>Why can&apos;t I disable this profile?</strong> Default-anchor protections prevent removing required fallback coverage.</Li>
				<Li><strong>Why do counts look larger than one tab?</strong> The overview summarizes the whole profile, not just the currently visible category.</Li>
			</Ul>
		</DocLayout>
	);
}
