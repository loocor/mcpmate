import { useMemo } from "react";
import { Link } from "react-router-dom";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";
import Callout from "../../components/Callout";
import CommunityLinks from "../../components/CommunityLinks";
import CopyableInlineCode from "../../components/CopyableInlineCode";
import DesktopDownloadList from "../../components/DesktopDownloadList";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

const howToSteps = [
	{
		name: "安装并启动 MCPMate",
		text: "下载与你的平台匹配的桌面安装包，完成安装并打开 MCPMate。",
	},
	{
		name: "完成引导流程",
		text: "让 MCPMate 检测已有客户端和 Server，并确认需要保留的设置。",
	},
	{
		name: "添加第一个 Server",
		text: "从 Market 选择 Server，或导入已有 MCP 配置，并在安装前完成检查。",
	},
	{
		name: "连接客户端",
		text: "通过 Default 配置集把 Server 提供给已检测到的 AI 客户端。",
	},
	{
		name: "验证连接",
		text: "在客户端中调用一个简单的 MCP 能力，确认 Server 已经可用。",
	},
];

export default function Quickstart() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "MCPMate 快速开始",
				description: "安装 MCPMate，完成引导，添加一个 MCP Server，并在 AI 客户端中使用它。",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout
			meta={{
				title: "快速开始",
				description: "从安装 MCPMate 到验证第一个 MCP Server，只需几个步骤。",
			}}
		>
			<SchemaOrg schema={howTo} />
			<P>这份指南只走一条最短路径：安装 MCPMate、完成首次引导、添加一个 Server，并确认它能在你的 AI 客户端中正常工作。</P>

			<H2>从桌面应用开始</H2>
			<P>请选择与你的操作系统和处理器匹配的安装包。以下链接使用 MCPMate 的下载统计跳转服务，并指向当前版本的发布产物。</P>
			<DesktopDownloadList locale="zh" />
			<Callout type="info" title="也可以使用 Homebrew">
				macOS 和 Linux 用户可以直接运行{" "}
				<CopyableInlineCode copyLabel="复制命令" copiedLabel="已复制" errorLabel="复制失败">
					brew install --cask loocor/tap/mcpmate@beta
				</CopyableInlineCode>
				。支持的系统、更新和卸载方式请查看{" "}
				<Link className="font-medium underline" to="/docs/zh/installation">
					安装指南
				</Link>
				。
			</Callout>

			<H2>完成引导流程</H2>
			<Ul>
				<Li>安装后打开 MCPMate，并继续欢迎页面中的引导步骤。</Li>
				<Li>检查 MCPMate 在本机发现的 AI 客户端和 MCP Server。</Li>
				<Li>保留你希望继续使用的现有设置；如果是第一次使用 MCP，也可以直接选择一个入门 Server。</Li>
			</Ul>

			<H2>添加第一个 Server</H2>
			<Ul>
				<Li>
					打开 <strong>Market</strong> 选择一个 Server，或者进入 <strong>Servers</strong> 导入已有配置。
				</Li>
				<Li>检查识别出的命令、参数和必填信息。</Li>
				<Li>运行预览检查，确认无误后完成安装。</Li>
			</Ul>

			<H2>连接到客户端</H2>
			<Ul>
				<Li>
					打开 <strong>Profiles</strong>，选择 <strong>Default</strong>，确认其中已经包含刚添加的 Server。
				</Li>
				<Li>
					打开 <strong>Clients</strong>，选择已检测到的 AI 应用，并按照 MCPMate 推荐的设置应用 Default 配置集。
				</Li>
			</Ul>

			<H2>验证第一个能力</H2>
			<P>打开或重新启动已连接的 AI 客户端，让它执行一个由该 Server 提供的简单操作。当客户端能够看到并调用这个能力时，首次设置就完成了。</P>

			<H2>加入社区</H2>
			<P>获取帮助、分享你的使用方式，或者告诉我们接下来应该改进什么。</P>
			<CommunityLinks locale="zh" />
		</DocLayout>
	);
}
