import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

export default function Inspector() {
	return (
		<DocLayout
			meta={{
				title: "检视器",
				description: "无需离开控制台，即可深入查看服务器状态、日志和诊断信息",
			}}
		>
			<P>
				MCPMate 检视器提供了一个强大的界面，用于监控和调试您的 MCP
				服务器。实时了解服务器行为，检查日志，诊断问题——所有这些都可以在
				MCPMate 控制台内完成。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="检视器：针对服务器能力发起工具调用"
			/>

			<H2>功能特性</H2>
			<Ul>
				<Li>
					<strong>实时监控：</strong>实时观察服务器活动
				</Li>
				<Li>
					<strong>日志查看器：</strong>浏览和搜索服务器日志
				</Li>
				<Li>
					<strong>请求/响应检视器：</strong>详细检查 MCP 协议消息
				</Li>
				<Li>
					<strong>性能指标：</strong>跟踪响应时间和资源使用情况
				</Li>
				<Li>
					<strong>错误诊断：</strong>快速识别和排查问题
				</Li>
			</Ul>

			<H2>使用场景</H2>
			<Ul>
				<Li>调试服务器配置问题。</Li>
				<Li>在发布窗口观察运行时行为。</Li>
				<Li>全量启用前验证能力返回结构。</Li>
				<Li>排查客户端与服务器通信问题。</Li>
			</Ul>

			<H2>推荐操作路径</H2>
			<Ul>
				<Li>先在服务器详情定位目标服务与能力项。</Li>
				<Li>在检视器中发起受控调用并记录响应元信息。</Li>
				<Li>结合审计日志时间线还原完整操作上下文。</Li>
			</Ul>
		</DocLayout>
	);
}
