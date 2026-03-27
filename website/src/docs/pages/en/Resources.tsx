import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function Resources() {
	return (
		<DocLayout
			meta={{ title: "Resources", description: "Shared resources surfaced by servers" }}
		>
			<P>
				Resources are non-tool artifacts exposed by MCP servers, such as files,
				documents, references, or generated outputs that clients can read.
			</P>

			<H2>Where to manage resources</H2>
			<Ul>
				<Li>Open a server detail page and switch to the Resources tab.</Li>
				<Li>Use Profiles to control whether that server is active for clients.</Li>
				<Li>Use the Inspector to validate resource visibility and payload shape.</Li>
			</Ul>

			<H2>Operational guidance</H2>
			<Ul>
				<Li>Keep resource-heavy servers scoped to specific profiles.</Li>
				<Li>Use clear server names so operators can identify resource owners.</Li>
				<Li>Review Audit Logs after profile changes affecting resource access.</Li>
			</Ul>
		</DocLayout>
	);
}
