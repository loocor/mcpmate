import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function BrowserExtensionZH() {
	return (
		<DocLayout
			meta={{
				title: "浏览器扩展",
				description:
					"使用 MCPMate 浏览器扩展浏览精选 Discovery 条目，并把网页中的 MCP 片段送入桌面端导入流程。",
			}}
		>
			<P>
				MCPMate 的 Chrome / Edge 扩展把发现入口放到 MCP Server 常出现的网页旁边。
				工具栏弹窗会展示来自 MCPMate Public Discovery 的 Portals、Servers 与
				Clients；页面脚本则可以把识别到的 MCP 片段发送到桌面端。
			</P>

			<H2>Discovery 标签页</H2>
			<Ul>
				<Li>
					<strong>Portals</strong> 展示常用 MCP 发现入口与社区资源。
				</Li>
				<Li>
					<strong>Servers</strong> 展示通过 MCPMate Admin 发布的精选服务器条目。
				</Li>
				<Li>
					<strong>Clients</strong> 展示兼容的 AI 应用与可辅助配置的客户端预设。
				</Li>
			</Ul>

			<H2>片段交接</H2>
			<P>
				当网页里出现疑似 MCP Server 配置块时，扩展会加入{" "}
				<strong>Add to MCPMate</strong> 操作。点击后会通过{" "}
				<code>mcpmate://import/server</code> 打开桌面端，并附带片段文本、推断格式
				和来源 URL。随后 MCPMate 会继续进入 Servers 页面同一套 Uni-Import
				预览与校验流程。
			</P>

			<H2>目录加载</H2>
			<Ul>
				<Li>Discovery 条目来自 MCPMate Public Discovery API。</Li>
				<Li>Servers 与 Clients 标签页会在弹窗滚动时分页加载。</Li>
				<Li>弹窗会在本地缓存 Discovery 响应，让重复打开更快。</Li>
				<Li>首次打开时语言跟随浏览器语言，也可以在弹窗设置中调整。</Li>
			</Ul>

			<H2>安装链接</H2>
			<Ul>
				<Li>
					Chrome Web Store：{" "}
					<a
						href="https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
				<Li>
					Microsoft Edge Add-ons：{" "}
					<a
						href="https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
			</Ul>

			<H2>它如何接入 MCPMate</H2>
			<H3>从网页发现到本地治理</H3>
			<P>
				扩展负责网页侧入口。MCPMate 桌面端负责导入后的预览、校验、保存、启用，
				以及加入配置集或客户端下发流程。
			</P>

			<Callout type="info" title="同一条导入路径">
				扩展捕获、拖拽、粘贴和 Market 安装最终都会进入 Server Install Wizard，
				因此每个服务器都可以先被检查，再进入本地工作区。
			</Callout>
		</DocLayout>
	);
}
