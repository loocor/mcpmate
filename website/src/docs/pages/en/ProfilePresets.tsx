import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfilePresets() {
	return (
		<DocLayout
			meta={{
				title: "Profile Preset Templates",
				description: "Use built-in profile templates as guided starting points",
			}}
		>
			<P>
				Preset routes under <code>/profiles/presets/:presetId</code> are best used as
				decision aids. They help you compare a recommended bundle before you create a
				real profile that your team can activate, edit, and assign to clients.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="Profile presets list with template cards"
			/>

			<H2>When to use presets</H2>
			<Ul>
				<Li>When you want a fast starting point for a known workflow such as writing or coding.</Li>
				<Li>When you need to review bundled servers and capability scope before rollout.</Li>
				<Li>When onboarding a teammate who needs examples before making custom choices.</Li>
			</Ul>

			<H2>How to work from a preset</H2>
			<Ul>
				<Li>Open the preset route and review the included servers and capability mix.</Li>
				<Li>Create a new profile or clone into one when you need editable ownership.</Li>
				<Li>Move to the detail pages only after the new profile exists under your workspace.</Li>
			</Ul>

			<Callout type="info" title="Why presets stay separate from live profiles">
				Preset pages are optimized for comparison, not daily operations. Keeping them
				separate avoids accidental assumptions that a template is already active in the
				runtime.
			</Callout>

			<H2>Common questions</H2>
			<Ul>
				<Li><strong>Can I edit a preset directly?</strong> Treat presets as references; create or clone a profile for ongoing changes.</Li>
				<Li><strong>Why is nothing changing in clients?</strong> Presets do not affect clients until a real profile is activated or selected.</Li>
			</Ul>
		</DocLayout>
	);
}
