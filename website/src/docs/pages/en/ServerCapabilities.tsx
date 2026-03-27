import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ServerCapabilities() {
	return (
			<DocLayout
				meta={{
					title: "Server Capabilities",
					description: "Review the normalized capability inventory before moving into live debug workflows",
				}}
			>
				<P>
					Capability tabs answer two questions: what the server says it can do, and what
					MCPMate is currently able to read from it. Use them for validation, not just
					browsing names.
				</P>

				<Callout type="info" title="This page is for inventory; Inspector is for live proof">
					When you need to execute a call, compare proxy and native behavior, or watch
					live request events, move to the dedicated <strong>Inspector</strong> guide
					under the same Servers section.
				</Callout>

			<section id="tools">
				<H2>Tools</H2>
				<P>Use the Tools tab to verify callable actions and compare them with what profiles later expose.</P>
			</section>

			<section id="prompts">
				<H2>Prompts</H2>
				<P>Use Prompts to confirm reusable instruction assets before assigning the server to writing or analysis workflows.</P>
			</section>

			<section id="resources">
				<H2>Resources</H2>
				<P>Use Resources to inspect non-tool artifacts and confirm that the server is returning readable payloads.</P>
			</section>

			<section id="templates">
				<H2>Resource Templates</H2>
				<P>Use Templates when the server advertises structured resource entry points that downstream clients depend on.</P>
			</section>

			<H2>When to leave this page and open Inspector</H2>
			<Ul>
				<Li>When capability counts look wrong after an import or edit.</Li>
				<Li>When you need raw responses instead of the normalized UI summary.</Li>
				<Li>When comparing proxy and native channels during debugging.</Li>
			</Ul>

			<Callout type="info" title="Debug view is best for evidence">
				The Inspector is the shortest path from a suspicious UI symptom to the raw
				response that caused it. Use it before assuming the backend or market data is
				wrong.
			</Callout>
		</DocLayout>
	);
}
