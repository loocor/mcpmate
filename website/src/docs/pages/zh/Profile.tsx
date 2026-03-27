import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfileZH() {
	return (
		<DocLayout
			meta={{ title: "配置集", description: "将服务器、工具与提示词打包为可复用预设" }}
		>
			<P>
				配置集用于把 MCP 服务器、工具、资源、提示词组合成命名预设。页面提供检索、统计、抽屉式编辑与启停按钮，所有操作都会立即同步到核心服务。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="配置集列表与默认配置集卡片"
			/>

			<H2>文档导航</H2>
			<Ul>
				<Li>
					<strong>预设模板</strong> 说明只读预设路由的作用，以及何时应该先克隆再修改。
				</Li>
				<Li>
					<strong>详情概览</strong> 聚焦 <code>/profiles/:profileId</code> 的概览标签，重点解释激活、默认配置集与统计卡。
				</Li>
				<Li>
					<strong>能力标签页</strong> 说明服务器、工具、提示词、资源与模板标签页里的精细控制逻辑。
				</Li>
			</Ul>

			<Callout type="info" title="默认锚点配置集">
				带有 <code>default_anchor</code> 角色的配置集会固定在列表顶部且不可停用，用于保证基础能力始终可用。
			</Callout>

			<H2>统计卡与工具栏</H2>
			<Ul>
				<Li>四张卡片汇总已激活配置集、启用服务器、启用工具、就绪实例数量。</Li>
				<Li>工具栏支持名称/描述搜索、名称或启用状态排序，以及网格/列表切换；切换结果会写入设置中的全局默认视图。</Li>
				<Li>右上角的刷新按钮可强制重新拉取数据，新增按钮会打开创建抽屉。</Li>
			</Ul>

			<H2>创建与编辑</H2>
			<H3>新建抽屉</H3>
			<P>
				点击“新建配置集”会打开侧边抽屉，可填写展示名称、描述、配置集类型等信息。若通过预设快捷入口（如
				<code>?type=writer</code>）进入，表单会预选对应模板。
			</P>

			<H3>详情页</H3>
			<P>
				选择配置集卡片会进入 <code>/profiles/:profileId</code>，可查看该配置集下的服务器、工具、资源、提示词及各自开关，并可通过面包屑返回列表。
			</P>
			<P>
				内置模板走 <code>/profiles/presets/:presetId</code> 路由，适合作为只读参考：浏览预置服务与能力后，如需可编辑副本再克隆或新建配置集。
			</P>

			<H2>为什么共享配置集很重要</H2>
			<P>
				共享配置集是可以被客户端直接引用的可复用层。你不需要一次又一次地重新打开复杂配置界面，逐项去勾服务器、工具、提示词和资源，而是可以先准备好“文案编撰”“前端开发”“研究分析”这类工作模式，再把整组能力作为一个整体去切换。
			</P>
			<Ul>
				<Li>同一套能力组合需要在多个客户端复用时，可以大幅减少重复配置动作。</Li>
				<Li>它可以把当前会话真正暴露出来的能力面收窄到任务所需范围。</Li>
				<Li>不需要的工具、提示词与资源不会长期挂在会话里，更容易控制开销和干扰。</Li>
			</Ul>

			<H2>不打开 MCPMate 也能切换</H2>
			<P>
				MCPMate 内建了 profile 相关的 MCP 工具，以及面向客户端的共享 profile 选择工具。只要当前客户端模式允许暴露这些工具，像 Cursor、Cherry Studio 这样的环境里就可以通过对话触发 profile 切换或共享 profile 选择，而不必重新回到 MCPMate 界面操作。
			</P>
			<Callout type="info" title="自然语言只是入口，真正执行的是 MCP 工具">
				这里没有单独的“自然语言切换解析器”。实际体验来自模型在当前模式下能够调用 MCPMate 内建的 profile / client profile 工具，因此你可以用一句话完成切换。
			</Callout>

			<Callout type="info" title="和 Claude Code 的关系怎么理解">
				Claude Code 官方文档强调的是 skills 与 MCP schema 的按需加载，以及 agent loop 里的自动工具选择。MCPMate 达到的是相近的结果——减少无关能力长期暴露——但实现方式是 profile 切换，而不是 Claude Code 那套 schema 加载与权限分类机制。
			</Callout>

			<H2>激活流程</H2>
			<Ul>
				<Li>卡片或列表项右侧的开关用于启用/暂停配置集，操作后立即调用激活/停用接口并通过提示信息反馈结果。</Li>
				<Li>多个配置集同时启用时，其服务器与工具会在运行时合并，顶部统计会实时反映合并后的规模。</Li>
				<Li>默认锚点配置集的开关会处于禁用状态，避免误操作。</Li>
			</Ul>

			<Callout type="warning" title="激活卡顿的排查方式">
				若开关长时间无响应，可先在运行时页面确认代理健康，再点击刷新按钮重新拉取数据。必要时通过 API 文档访问
				对应配置集接口，验证后端是否已持久化变更。
			</Callout>
		</DocLayout>
	);
}
