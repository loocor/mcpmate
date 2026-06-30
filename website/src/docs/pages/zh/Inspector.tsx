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
				MCPMate 检视器是面向 MCP 服务器的实时能力工作台。你可以比较 native
				与 proxy 行为，发起受控 tool / prompt 请求，读取 resource，并在不离开控制台的情况下记录响应或事件证据。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="检视器：针对服务器能力发起工具调用"
			/>

			<H2>功能特性</H2>
			<Ul>
				<Li>
					<strong>Native 与 proxy 通道：</strong>对比服务器原始行为与 MCPMate 托管暴露结果
				</Li>
				<Li>
					<strong>Schema 感知输入：</strong>根据能力元数据生成表单，也可切换到原始 JSON
				</Li>
				<Li>
					<strong>响应与事件复查：</strong>分开查看最终输出、progress、log、error 与 cancel 状态
				</Li>
				<Li>
					<strong>能力读取：</strong>用同一套抽屉流程验证工具、提示词、资源和资源模板
				</Li>
				<Li>
					<strong>错误诊断：</strong>快速识别和排查问题
				</Li>
			</Ul>

			<H2>使用场景</H2>
			<Ul>
				<Li>调试服务器配置问题。</Li>
				<Li>对比服务器原始输出与配置集作用后的 proxy 输出。</Li>
				<Li>全量启用前验证能力返回结构。</Li>
				<Li>排查客户端与服务器通信问题。</Li>
			</Ul>

			<H2>推荐操作路径</H2>
			<Ul>
				<Li>先在服务器详情定位目标服务与能力项。</Li>
				<Li>在检视器中发起受控调用，并分开复查响应与事件输出。</Li>
				<Li>结合审计日志时间线还原完整操作上下文。</Li>
			</Ul>
		</DocLayout>
	);
}
