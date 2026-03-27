import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerInspectorZH() {
	return (
		<DocLayout
			meta={{
				title: "Server Inspector",
				description: "在真实服务器上下文中验证 MCP 行为，而不是只看静态能力摘要",
			}}
		>
			<P>
				真正的 Inspector 工作流发生在 <code>/servers/:serverId</code> 切到
				<code>view=debug</code> 之后。这里可以比较 proxy / native 两种通道，拉取实时能力列表，打开 Inspector 抽屉，并在不离开服务器上下文的前提下获取请求与事件证据。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="Inspector 调试视图与抽屉"
			/>

			<H2>入口在哪里</H2>
			<Ul>
				<Li>在 Servers 列表里，Inspect 按钮会直接把某个服务器打开到 debug 视图。</Li>
				<Li>在 Profile 详情页里，每个 server 行都有 Browse 与 Inspect 的悬浮动作。</Li>
				<Li>路由会保留当前 server 上下文，因此切换不同能力标签时不会丢目标服务器。</Li>
			</Ul>

			<H2>先选对通道</H2>
			<P>
				如果你要验证“激活的 profile 最终暴露了什么”，优先用 proxy。若你要绕过 profile，看服务器自身原始能力，就用 native。
			</P>

			<Callout type="warning" title="看不到 proxy 不一定是故障">
				当服务器没有被任何激活中的 profile 启用时，debug 视图会回落到 native，并说明原因。这通常是暴露状态线索，而不是 Inspector 本身坏了。
			</Callout>

			<section id="tools">
				<H2>Tools</H2>
				<P>先拉取实时工具列表，再进入抽屉发起受控 tool call，可设置超时、使用 schema 生成表单、切换原始 JSON、查看事件流，并在必要时取消请求。</P>
			</section>

			<section id="prompts">
				<H2>Prompts</H2>
				<P>先确认当前服务器真实暴露了哪些 prompt，再用抽屉发起 get 请求，并通过生成参数表单或原始 JSON 方式构造输入。</P>
			</section>

			<section id="resources">
				<H2>Resources</H2>
				<P>当你需要验证某个具体 URI 实际返回了什么，而不是只看列表摘要时，用这个标签最合适。</P>
			</section>

			<section id="templates">
				<H2>Resource Templates</H2>
				<P>当资源 URI 带参数占位符时，抽屉可以自动识别占位符、预填 mock 值，并在真正读取前生成最终 URI。</P>
			</section>

			<H2>抽屉真正增加了什么</H2>
			<Ul>
				<Li>当 schema 完整时，自动生成工具 / prompt 参数表单。</Li>
				<Li>当你需要精确构造请求体时，可切换到原始 JSON 模式。</Li>
				<Li>Tool call 支持 started、progress、log、result、error、cancelled 等实时事件。</Li>
				<Li>输出支持复制与清空，方便做问题复现与结果比对。</Li>
			</Ul>

			<H3>什么时候该用 Inspector，而不是 browse</H3>
			<P>只想看规范化能力清单时，用 browse 足够；需要证明真实返回、比较通道差异、或发起受控请求时，再进入 Inspector。</P>

			<H2>常见排查问题</H2>
			<Ul>
				<Li><strong>为什么只有 native？</strong> 多半是该服务器没有被激活中的 profile 启用，因此 proxy 没有可用路径。</Li>
				<Li><strong>为什么 tool call 卡住？</strong> 先在抽屉里 cancel，再看事件流里是否有 progress 或 log，判断是服务端阻塞还是请求本身异常。</Li>
				<Li><strong>为什么有的条目是表单，有的只能写 JSON？</strong> Inspector 会根据每个 capability 实际暴露的 schema 元数据自适应。</Li>
			</Ul>
		</DocLayout>
	);
}
