import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function ResourceOptimization() {
	return (
		<DocLayout
			meta={{
				title: "资源优化",
				description: "智能管理服务器资源，减少系统开销并提高性能",
			}}
		>
			<P>
				MCPMate
				智能管理服务器资源，在最大化性能的同时最小化系统开销。通过智能池化和生命周期管理，MCPMate
				确保您的 MCP 服务器高效运行。
			</P>

			<H2>主要能力</H2>
			<Ul>
				<Li>
					<strong>连接池：</strong>在多个客户端之间共享服务器实例
				</Li>
				<Li>
					<strong>自动生命周期管理：</strong>按需启动服务器，不使用时停止
				</Li>
				<Li>
					<strong>内存优化：</strong>通过智能资源共享减少内存占用
				</Li>
				<Li>
					<strong>性能监控：</strong>跟踪资源使用并识别优化机会
				</Li>
			</Ul>

			<H2>优势</H2>
			<P>
				MCPMate
				可以在所有应用程序之间共享单个服务器实例，而不是为不同客户端运行同一服务器的多个实例，从而大幅降低
				CPU 和内存使用率。
			</P>

			<H2>运维检查点</H2>
			<Ul>
				<Li>通过控制台资源趋势观察 CPU/内存是否长期异常。</Li>
				<Li>扩容客户端前先在运行时页面确认缓存与运行环境健康。</Li>
				<Li>重要优化动作建议在审计日志中留痕，便于后续对比。</Li>
			</Ul>
		</DocLayout>
	);
}
