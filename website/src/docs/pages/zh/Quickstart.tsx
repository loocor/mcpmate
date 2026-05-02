import { useMemo } from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";

const howToSteps = [
	{
		name: "下载桌面应用",
		text: "先到 GitHub Releases 下载适合你平台的安装包。当前 macOS 构建最稳定；Windows 和 Linux 安装包也已提供，但这两个平台仍在持续补齐体验。",
	},
	{
		name: "启动 MCPMate",
		text: "打开应用，让它自动启动内置的本地代理。你会同时得到仪表盘、8080 端口上的 REST API，以及 8000 端口上的 MCP 端点。",
	},
	{
		name: "接入 MCP 服务",
		text: "可以浏览内置服务源、导入 JSON/TOML 片段，或直接从已有客户端拉取服务配置。",
	},
	{
		name: "整理配置集",
		text: "打开默认配置集，加入所需服务，并按当前任务启用或禁用工具、提示、资源。",
	},
	{
		name: "发布到客户端",
		text: "在客户端页面确认编辑器已被检测，选择托管、统一或透明模式，然后应用配置集并回到编辑器验证。",
	},
];

export default function QuickstartZH() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "如何设置 MCPMate",
				description:
					"从 GitHub Releases 快速启动 MCPMate，再逐步接入服务、整理配置集并发布到客户端的分步指南。",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout meta={{ title: "快速开始", description: "安装、配置并运行 MCPMate" }}>
			<SchemaOrg schema={howTo} />
			<P>
				目前最快的起步方式，是直接从 GitHub Releases 安装桌面版。启动后，你就可以在同一个控制台里接入服务、整理配置集，并把它们发布到编辑器里。
			</P>

			<H2>先从桌面应用开始</H2>
			<Callout type="info" title="当前最快的路径">
				现在最直接的方式，是使用 GitHub Releases 上的官方桌面安装包：
				https://github.com/loocor/mcpmate/releases
			</Callout>
			<Ul>
				<Li>打开 Releases 页面，选择适合你平台的安装包。</Li>
				<Li>
					当前 macOS 安装包最稳定。Windows 和 Linux 安装包也可下载，但部分功能可能仍在补齐或暂时不稳定。
				</Li>
				<Li>
					安装后启动 MCPMate。桌面应用会把仪表盘与本地代理一起打包好，让你从一个入口就能开始管理。
				</Li>
			</Ul>

			<H3>需要完全自控时再从源码构建</H3>
			<Ul>
				<Li>先安装 Rust 1.75+ 与 Node.js 18+（或 Bun）。</Li>
				<Li>克隆仓库：<code>git clone https://github.com/loocor/mcpmate.git</code></Li>
				<Li>进入后端目录：<code>cd mcpmate/backend</code></Li>
				<Li>构建并运行：<code>cargo run --release</code></Li>
				<Li>代理启动后，REST API 位于 8080 端口，MCP 端点位于 8000 端口。</Li>
			</Ul>

			<H3>从源码运行仪表盘</H3>
			<Ul>
				<Li>进入前端目录：<code>cd mcpmate/board</code></Li>
				<Li>安装依赖：<code>bun install</code></Li>
				<Li>启动开发服务器：<code>bun run dev</code></Li>
				<Li>打开 http://localhost:5173 访问管理仪表盘。</Li>
			</Ul>

			<H2>选择外壳：Web 还是桌面版</H2>
			<P>
				同一套 Board 界面可以运行在两种外壳里。你可以按自己希望如何运行代理来选择。
			</P>
			<Ul>
				<Li>
					<strong>浏览器 + 开发代理</strong>：Vite 提供前端界面，请求默认发往 <code>http://127.0.0.1:8080</code>（或你覆盖后的 API 基址）。适合前后端分开迭代开发。
				</Li>
				<Li>
					<strong>Tauri 桌面应用（macOS / Windows / Linux）</strong>：将仪表盘与本地代理打包在一起，官方安装包发布于 GitHub Releases。侧栏的 <strong>账户</strong> 在 macOS 上支持可选的 GitHub 登录，为后续云端相关能力预留身份；应用内的 <strong>文档</strong> 按钮会打开 <code>mcp.umate.ai</code> 上与当前页面对应的指南。
				</Li>
			</Ul>

			<H2>分离运行核心服务与 UI</H2>
			<P>
				如果你希望把 MCPMate 放到另一台机器上运行，或者只是更偏好拆分部署，也可以把核心服务与 UI 外壳解耦。
			</P>
			<Ul>
				<Li>在目标主机上启动后端，并暴露你计划使用的 REST / MCP 端口。</Li>
				<Li>让 Web 控制台或桌面壳连接这个后端，而不是只运行本地一体化实例。</Li>
				<Li>在“设置 → 系统”中核对 API / MCP 端口，并在端点变化后复制重启命令。</Li>
			</Ul>

			<H2>把 MCP 服务接入 MCPMate</H2>
			<P>按你现有配置所在的位置，选择最顺手的导入路径即可。</P>
			<H3>浏览内置服务源</H3>
			<Ul>
				<Li>从左侧导航打开 <strong>服务源</strong>。</Li>
				<Li>搜索或筛选目标服务，点击 <strong>安装</strong> 即可加入工作区。</Li>
			</Ul>
			<H3>拖拽导入外部 Bundle</H3>
			<Ul>
				<Li>在 <strong>服务器</strong> 页面点击 <strong>新增</strong>，把 MCP Bundle 或 JSON / TOML 片段拖入窗口。</Li>
				<Li>确认预览信息无误后提交，服务条目就会被创建出来。</Li>
			</Ul>
			<H3>从已有客户端导入</H3>
			<Ul>
				<Li>打开 <strong>客户端</strong> 页面，选择一个已检测到的客户端。</Li>
				<Li>使用 <strong>从客户端导入</strong>，把原先在客户端里的 MCP 配置带回 MCPMate。</Li>
			</Ul>

			<H2>按真实任务组织配置集</H2>
			<P>
				配置集决定了客户端最终能看到哪些服务与能力。MCPMate 默认附带一个 <strong>默认</strong> 配置集，你也可以围绕不同工作流再建更多。
			</P>
			<Ul>
				<Li>进入 <strong>配置集</strong> 页面，打开 <strong>默认</strong> 配置集。</Li>
				<Li>加入刚刚安装的服务，并按任务需要启用或禁用工具、提示、资源。</Li>
				<Li>通过 <strong>新建配置集</strong> 创建额外预设，例如写作、数据探索或调试场景。</Li>
			</Ul>

			<H2>把配置集发布到客户端</H2>
			<Ul>
				<Li>确认你的编辑器在 <strong>客户端</strong> 页面中显示为 <strong>已检测</strong>。</Li>
				<Li>如果该客户端需要允许 MCPMate 回写自身的 MCP 配置，请先在 New / Edit 抽屉中确认路径指向真实且可写的本地配置文件；MCPMate 会先校验，再把它作为写入目标。</Li>
				<Li>使用 <strong>托管模式</strong> 可由仪表盘直接切换配置集；如果你更希望使用会话内建控制，可选 <strong>统一模式</strong>；<strong>透明模式</strong> 只负责写入配置文件，不支持就地切换。</Li>
				<Li>选择准备好的配置集并应用，然后回到编辑器触发一次 MCP 命令，确认工具已经出现。</Li>
			</Ul>

			<H2>运行时出问题时</H2>
			<Ul>
				<Li>如果服务启动失败或工具返回错误，先打开 <strong>运行时</strong> 页面。</Li>
				<Li>使用 <strong>安装 / 修复</strong> 补齐 uv、Bun 等依赖；如果怀疑缓存陈旧，也可以在同一页清理缓存。</Li>
			</Ul>

			<H2>用审计日志追踪变更</H2>
			<Ul>
				<Li>进入 <strong>审计日志</strong> 页面，查看配置集、客户端、服务器相关操作。</Li>
				<Li>按动作类型与时间范围筛选，更快定位是谁在什么时候改了什么。</Li>
			</Ul>

			<H2>保持更新，也欢迎贡献</H2>
			<P>
				如果你使用桌面版，请以 GitHub Releases 作为最新安装包与发布说明的首选来源；如果你从源码运行 MCPMate，请拉取最新代码后重新构建。遇到问题或有改进想法，也欢迎提交 issue 或 pull request。
			</P>
		</DocLayout>
	);
}
