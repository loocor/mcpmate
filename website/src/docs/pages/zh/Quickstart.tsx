import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function QuickstartZH() {
	return (
		<DocLayout meta={{ title: "快速开始", description: "构建、配置并运行 MCPMate" }}>
			<P>本文档将带你从源码构建到在客户端生效，完整体验 MCPMate。</P>

			<H2>从源码构建</H2>
			<Callout type="info" title="开源项目">
				MCPMate 已在 MIT 许可下开源，代码见 github.com/loocor/mcpmate
			</Callout>
			<Ul>
				<Li>安装 Rust 1.75+ 和 Node.js 18+（或 Bun）。</Li>
				<Li>克隆仓库：<code>git clone https://github.com/loocor/mcpmate.git</code></Li>
				<Li>进入后端目录：<code>cd mcpmate/backend</code></Li>
				<Li>构建并运行：<code>cargo run --release</code></Li>
				<Li>代理启动后，REST API 在 8080 端口，MCP 端点在 8000 端口。</Li>
			</Ul>

			<H3>运行 Dashboard</H3>
			<Ul>
				<Li>进入前端目录：<code>cd mcpmate/board</code></Li>
				<Li>安装依赖：<code>bun install</code></Li>
				<Li>启动开发服务器：<code>bun run dev</code></Li>
				<Li>打开 http://localhost:5173 访问管理界面。</Li>
			</Ul>

			<H2>安装 MCP 服务</H2>
			<P>可以根据需求选择以下任意方式。</P>
			<H3>浏览内置商城</H3>
			<Ul>
				<Li>左侧导航进入 <strong>市场</strong>。</Li>
				<Li>搜索或筛选所需服务，点击 <strong>安装</strong> 即可添加到工作区。</Li>
			</Ul>
			<H3>拖拽导入其他服务</H3>
			<Ul>
				<Li>在 <strong>服务器</strong> 页面点击 <strong>新增</strong>，将 MCP Bundle 或 JSON/TOML 片段拖入窗口。</Li>
				<Li>确认预览信息无误后提交，服务即会出现在列表中。</Li>
			</Ul>
			<H3>从客户端导入现有配置</H3>
			<Ul>
				<Li>打开 <strong>客户端</strong> 页面，选择一个已检测到的客户端。</Li>
				<Li>使用 <strong>从客户端导入</strong>，可将原先在客户端配置的 MCP 服务同步到 MCPMate。</Li>
			</Ul>

			<H2>组织配置集</H2>
			<P>
				配置集用于决定哪些服务与能力暴露给客户端。MCPMate 默认提供一个
				<strong> 默认</strong> 配置集，可根据场景自行增删。
			</P>
			<Ul>
				<Li>进入 <strong>配置集</strong> 页面，打开 <strong>默认</strong> 配置集。</Li>
				<Li>添加刚安装的服务，按需启停工具、资源、提示词等能力。</Li>
				<Li>可通过 <strong>新建配置集</strong> 创建不同场景（写作/分析等）的组合，并启用匹配的能力。</Li>
			</Ul>

			<H2>在客户端应用配置</H2>
			<Ul>
				<Li>确认客户端在 <strong>客户端</strong> 页面显示为 <strong>已检测</strong>。</Li>
				<Li>
					将客户端设置为 <strong>托管（Hosted）</strong> 模式，即可在 MCPMate 内直接切换配置集；透明模式仅会写入配置文件，不支持就地切换。
				</Li>
				<Li>在客户端页面选择要应用的配置集，随后回到编辑器中执行 MCP 命令验证工具是否出现。</Li>
			</Ul>

			<H2>运行异常时</H2>
			<Ul>
				<Li>若服务无法启动或提示缺少运行环境，请打开 <strong>运行时</strong> 页面。</Li>
				<Li>使用 <strong>安装 / 修复</strong> 按钮为 uv、Bun 等运行时一键安装依赖，必要时可清理缓存。</Li>
			</Ul>

			<H2>更新与贡献</H2>
			<P>
				从 GitHub 拉取最新代码获取新功能和修复。如有问题或建议，欢迎提交 issue 或 pull request。
			</P>
		</DocLayout>
	);
}
