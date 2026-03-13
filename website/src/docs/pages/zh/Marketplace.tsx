import { P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "内联商城",
				description: "内建官方注册中心与 mcpmarket.cn，不用东奔西走找服务",
			}}
		>
			<P>
				MCPMate 包含一个集成市场，可以访问官方 MCP 注册中心和
				mcpmarket.cn。无需离开应用程序即可发现、安装和配置新的 MCP 服务器。
			</P>

			<h2>功能特性</h2>
			<ul>
				<li>
					<strong>统一搜索：</strong>同时搜索多个注册中心
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
				<li>
					<strong>mcpmarket.cn：</strong>社区策划的中文 MCP 服务器市场
				</li>
			</ul>

			<h2>优势</h2>
			<P>
				无需手动搜索 GitHub
				或文档站点、浏览安装说明和编辑配置文件，市场将整个过程简化为几次点击。
			</P>

			<P>市场使用指南和截图即将推出。</P>
		</DocLayout>
	);
}
