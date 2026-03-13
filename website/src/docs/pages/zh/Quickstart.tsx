import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function QuickstartZH() {
	return (
		<DocLayout meta={{ title: "快速开始", description: "安装、配置并应用 MCPMate" }}>
			<P>本文档将带你从下载到在客户端生效，完整体验 MCPMate 预览版。</P>

			<H2>下载与安装</H2>
			<Callout type="info" title="目前为 macOS 预览版">
				暂未提供 Windows / Linux 安装包，预览阶段功能仍在完善。
			</Callout>
			<Ul>
				<Li>访问 MCPMate 官网并点击 <strong>下载</strong>。</Li>
				<Li>打开下载得到的 DMG，将 <strong>MCPMate</strong> 拖入 <strong>应用程序</strong>。</Li>
				<Li>首次启动时若看到安全提示，请选择继续。</Li>
				<Li>启动后系统会自动检测已安装的 MCP 客户端（例如 Cursor、Claude Desktop、Zed）。</Li>
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

			<H2>预览版提示</H2>
			<P>
				预览阶段仍会持续修复问题与更新功能。请随时关注 <strong>更新日志</strong> 页面，了解最新功能、已修复及已知未修复的问题；若遇到未列出的异常，欢迎反馈以便我们跟进。
			</P>
		</DocLayout>
	);
}
