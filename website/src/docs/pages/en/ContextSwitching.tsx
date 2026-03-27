import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function ContextSwitching() {
	return (
		<DocLayout
			meta={{
				title: "Seamless Context Switching",
				description:
					"Switch between different work scenarios with instant configuration changes",
			}}
		>
			<P>
				MCPMate enables seamless context switching through its profile system.
				Instantly switch between different sets of servers and configurations to
				match your current work scenario—whether you're coding, writing,
				researching, or collaborating.
			</P>

			<H2>How it works</H2>
			<P>
				Create multiple profiles in MCPMate, each with its own set of servers
				and settings. Switch between profiles with a single click, and all
				connected clients automatically update to use the new configuration.
			</P>

			<H2>Example scenarios</H2>
			<Ul>
				<Li>
					<strong>Development Profile:</strong> Code assistance, file operations,
					Git integration
				</Li>
				<Li>
					<strong>Writing Profile:</strong> Grammar checking, research tools,
					citation management
				</Li>
				<Li>
					<strong>Analysis Profile:</strong> Data processing, visualization,
					statistical tools
				</Li>
				<Li>
					<strong>Team Profile:</strong> Shared resources and collaboration tools
				</Li>
			</Ul>

			<H2>Operational tips</H2>
			<Ul>
				<Li>Keep one default anchor profile always active as your safe baseline.</Li>
				<Li>Use Hosted mode when you need instant profile switching in clients.</Li>
				<Li>For separated deployments, verify API connectivity before switching.</Li>
				<Li>After major changes, verify effects in Audit Logs and client pages.</Li>
			</Ul>

			<P>
				A practical sequence is: prepare profiles in advance, switch by task, and
				review Audit Logs for traceability when multiple operators share one core
				service.
			</P>
		</DocLayout>
	);
}
