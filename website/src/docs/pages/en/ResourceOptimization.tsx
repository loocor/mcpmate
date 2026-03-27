import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function ResourceOptimization() {
	return (
		<DocLayout
			meta={{
				title: "Resource Optimization",
				description:
					"Intelligently manage server resources to reduce system overhead and improve performance",
			}}
		>
			<P>
				MCPMate intelligently manages server resources to minimize system
				overhead while maximizing performance. Through smart pooling and
				lifecycle management, MCPMate ensures your MCP servers run efficiently.
			</P>

			<H2>Key capabilities</H2>
			<Ul>
				<Li>
					<strong>Connection Pooling:</strong> Share server instances across
					multiple clients
				</Li>
				<Li>
					<strong>Automatic Lifecycle Management:</strong> Start servers
					on-demand and stop them when not in use
				</Li>
				<Li>
					<strong>Memory Optimization:</strong> Reduce memory footprint through
					intelligent resource sharing
				</Li>
				<Li>
					<strong>Performance Monitoring:</strong> Track resource usage and
					identify optimization opportunities
				</Li>
			</Ul>

			<H2>Benefits</H2>
			<P>
				Instead of running multiple instances of the same server for different
				clients, MCPMate can share a single instance across all your
				applications, dramatically reducing CPU and memory usage.
			</P>

			<H2>Operational checks</H2>
			<Ul>
				<Li>Use Dashboard metrics to spot sustained CPU/memory growth.</Li>
				<Li>Review Runtime page cache and runtime health before scaling clients.</Li>
				<Li>Track major optimization changes in Audit Logs for later comparison.</Li>
			</Ul>
		</DocLayout>
	);
}
