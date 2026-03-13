import React from "react";
import DocLayout from "../../layout/DocLayout";
import { P } from "../../components/Headings";

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

			<h2>How It Works</h2>
			<P>
				Create multiple profiles in MCPMate, each with its own set of servers
				and settings. Switch between profiles with a single click, and all
				connected clients automatically update to use the new configuration.
			</P>

			<h2>Example Scenarios</h2>
			<ul>
				<li>
					<strong>Development Profile:</strong> Code assistance, file operations,
					Git integration
				</li>
				<li>
					<strong>Writing Profile:</strong> Grammar checking, research tools,
					citation management
				</li>
				<li>
					<strong>Analysis Profile:</strong> Data processing, visualization,
					statistical tools
				</li>
				<li>
					<strong>Team Profile:</strong> Shared resources and collaboration tools
				</li>
			</ul>

			<h2>Benefits</h2>
			<ul>
				<li>No need to manually reconfigure clients</li>
				<li>Reduce cognitive load by focusing only on relevant tools</li>
				<li>Optimize performance by loading only what you need</li>
				<li>Create specialized workflows for different tasks</li>
			</ul>

			<P>Content coming soon with profile management best practices.</P>
		</DocLayout>
	);
}
