import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerInspector() {
	return (
		<DocLayout
			meta={{
				title: "Server Inspector",
				description: "Use the debug workbench and Inspector drawer to verify live MCP behavior",
			}}
		>
			<P>
				The practical Inspector workflow lives on <code>/servers/:serverId</code> when
				you switch the page into <code>view=debug</code>. It is where you compare
				proxy and native behavior, fetch live capability lists, open the Inspector
				drawer, and capture request or event evidence without leaving the server
				context.
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="Inspector debug view and tool-call drawer"
			/>

			<H2>Where Inspector starts</H2>
			<Ul>
				<Li>From the Servers list, the Inspect button opens a server directly in debug mode.</Li>
				<Li>From Profile detail, each server row exposes Browse and Inspect hover actions.</Li>
				<Li>The current route keeps the server context, so you can move between capability tabs without losing the target server.</Li>
			</Ul>

			<H2>Choose the right channel first</H2>
			<P>
				Proxy mode is best when you want to verify what active profiles expose through
				the managed runtime. Native mode is best when you need to bypass profile
				activation and ask what the server can do on its own.
			</P>

			<Callout type="warning" title="Proxy mode can be unavailable by design">
				If the server is not enabled in any active profile, the debug view falls back
				to native mode and explains why. Treat that as an exposure-state clue, not as
				a broken Inspector.
			</Callout>

			<section id="tools">
				<H2>Tools</H2>
				<P>
					Use the Tools tab to list live tool definitions, then open the drawer to run
					controlled tool calls with a timeout, schema-driven inputs, raw JSON
					overrides, event streaming, and cancellation.
				</P>
			</section>

			<section id="prompts">
				<H2>Prompts</H2>
				<P>
					Use Prompts to list what the server currently advertises, then open the
					drawer to issue prompt-get requests with generated argument forms or raw
					JSON.
				</P>
			</section>

			<section id="resources">
				<H2>Resources</H2>
				<P>
					Use Resources when you need to read a concrete URI and validate the returned
					payload instead of trusting the summary row alone.
				</P>
			</section>

			<section id="templates">
				<H2>Resource Templates</H2>
				<P>
					Templates are useful when resource URIs are parameterized. The drawer can
					derive placeholders, prefill mock values, and generate the final URI before
					reading it.
				</P>
			</section>

			<H2>What the drawer adds</H2>
			<Ul>
				<Li>Schema-derived forms for tool and prompt arguments when metadata is available.</Li>
				<Li>Raw JSON mode when you need exact request bodies instead of generated fields.</Li>
				<Li>Live events for tool calls, including started, progress, log, result, error, and cancelled states.</Li>
				<Li>Copy and clear actions so outputs can be reused in bug reports or comparisons.</Li>
			</Ul>

			<H3>When to use Inspector instead of browse mode</H3>
			<P>
				Choose browse mode when you only need the normalized capability inventory.
				Choose Inspector when you need to prove what the live endpoint returns, compare
				channels, or run a single request with controlled inputs.
			</P>

			<H2>Common troubleshooting questions</H2>
			<Ul>
				<Li><strong>Why do I only see native mode?</strong> The server is probably not enabled in any active profile, so proxy mode has nothing to route through.</Li>
				<Li><strong>Why does a tool call stay running?</strong> Use cancel in the drawer, then inspect the event stream to see whether the server emitted progress or log messages before stalling.</Li>
				<Li><strong>Why does the drawer show generated fields for one item but raw JSON for another?</strong> The Inspector adapts to the schema metadata each capability actually exposes.</Li>
			</Ul>
		</DocLayout>
	);
}
