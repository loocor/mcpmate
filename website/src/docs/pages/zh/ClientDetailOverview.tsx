import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ClientDetailOverviewZH() {
	return (
		<DocLayout meta={{ title: "详情概览", description: "在应用配置前先确认客户端状态、能力与当前服务器" }}>
			<P>
				<code>/clients/:identifier</code> 的概览标签用于判断某个客户端是否已检测、是否处于托管状态，以及当前配置中实际包含哪些服务器。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="客户端详情概览"
			/>
			<H2>先确认什么</H2>
			<Ul>
				<Li>客户端身份与检测状态是否正确。</Li>
				<Li>支持的传输方式是否满足你的接入方式。</Li>
				<Li>当前服务器列表是否符合你对现状的预期。</Li>
			</Ul>
			<H3>高价值动作</H3>
			<P>安装、移动或修复客户端后，先点刷新；准备把配置生命周期交给 MCPMate 时，再切换托管状态。</P>
			<H2>选择配置写入方式</H2>
			<P>
				Client writeback 默认使用 <strong>Settings → Client Defaults</strong> 中的策略，除非该客户端设置了自己的 override。选择 <strong>Merge</strong> 会保留客户端文件中无关的 MCP 条目；选择 <strong>Replace</strong> 则让 MCPMate 托管的 Server 列表成为权威。
			</P>
			<P>当 Client 已有经过验证的可写配置目标时，保存该 Client 的 override 会立即重新应用其有效配置。修改 Settings 中的默认值则会在后续 Client configuration Apply、重新 Apply 或 detach 时生效，不会立即重写已有客户端文件。</P>
			<Callout type="info" title="这里的 Docs / Homepage 链接来自客户端元数据">
				它们补充的是客户端产品自身的资料，适合和 MCPMate 文档搭配阅读，尤其在做兼容性排查时很有价值。
			</Callout>
		</DocLayout>
	);
}
