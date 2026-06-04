import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function OnboardingZH() {
	return (
		<DocLayout
			meta={{
				title: "首次引导",
				description:
					"通过 MCPMate 首次引导检测客户端、导入已有服务器，并从 Discovery 预设开始配置。",
			}}
		>
			<P>
				Onboarding 会把第一次使用 MCPMate 的过程从本地发现带到可用工作区。它会检测兼容客户端，
				审阅已有 MCP Server 配置，也可以在本地配置还为空时，从 MCPMate Public Discovery
				加载起步条目。
			</P>

			<H2>流程概览</H2>
			<Ul>
				<Li>检测已安装的 AI 客户端及其 MCP 配置目标。</Li>
				<Li>审阅本地客户端配置文件中发现的服务器。</Li>
				<Li>从 Public Discovery 选择起步客户端预设和服务器条目。</Li>
				<Li>把选中的服务器导入 MCPMate，并放入起步配置集。</Li>
				<Li>继续进入 Clients、Servers、Market 或 Profiles 做更细配置。</Li>
			</Ul>

			<H2>客户端发现</H2>
			<P>
				客户端步骤会结合本地检测和 MCPMate Discovery 预设。已检测到的应用会显示
				MCPMate 可以读取或写入的配置位置。预设条目则可以帮助你添加受支持的客户端，
				也适合在连接应用前先准备客户端记录。
			</P>

			<H2>服务器选择</H2>
			<P>
				服务器步骤既可以导入本地客户端文件中发现的条目，也可以展示来自 Public Discovery
				的精选起步服务器。选中的服务器会被组合成导入请求，由 MCPMate 规范化配置并保存到本地服务器库。
			</P>

			<H3>选择之后会发生什么</H3>
			<Ul>
				<Li>MCPMate 会把选中的服务器定义保存到本地工作区。</Li>
				<Li>导入后的服务器会出现在 Servers 页面。</Li>
				<Li>Profiles 可以使用这些服务器，控制不同客户端可见的能力。</Li>
				<Li>客户端配置可以继续选择 Hosted、Unify 或 Transparent 下发方式。</Li>
			</Ul>

			<Callout type="info" title="Discovery 支撑的起步数据">
				Public Discovery 为 Onboarding 提供常用客户端和服务器条目的起步数据。
				同一套 Admin 管理的 Discovery 数据也会用于浏览器扩展的目录标签页。
			</Callout>
		</DocLayout>
	);
}
