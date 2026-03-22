import { H3, P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";
import DocScreenshot from "../../components/DocScreenshot";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "内联商城",
				description: "内建官方 MCP 注册中心，不用东奔西走找服务",
			}}
		>
			<P>
				MCPMate 包含一个集成市场，可以访问官方 MCP 注册中心。无需离开应用程序即可发现、安装和配置新的 MCP 服务器。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="内联商城：浏览官方 MCP 注册中心"
			/>

			<h2>功能特性</h2>
			<ul>
				<li>
					<strong>统一搜索：</strong>搜索官方注册中心
				</li>
				<li>
					<strong>一键安装：</strong>直接从市场安装服务器
				</li>
				<li>
					<strong>自动配置：</strong>服务器会自动添加到您的活动配置集
				</li>
				<li>
					<strong>版本管理：</strong>在新版本可用时更新服务器
				</li>
				<li>
					<strong>评分与评论：</strong>在安装前查看社区反馈
				</li>
			</ul>

			<h2>支持的注册中心</h2>
			<ul>
				<li>
					<strong>官方 MCP 注册中心：</strong>Anthropic 的官方服务器集合
				</li>
			</ul>

			<h2>优势</h2>
			<P>
				无需手动搜索 GitHub
				或文档站点、浏览安装说明和编辑配置文件，市场将整个过程简化为几次点击。
			</P>

			<H3>新增 MCP 服务器向导</H3>
			<P>
				从注册卡片安装时会打开引导流程：配置传输方式、预览规范化后的清单，再导入到目标配置集。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="新增 MCP 服务器：核心配置步骤"
			/>
		</DocLayout>
	);
}
