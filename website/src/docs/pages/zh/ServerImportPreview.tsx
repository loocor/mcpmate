import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerImportPreviewZH() {
	return (
		<DocLayout
			meta={{
				title: "导入与预览",
				description: "用 Uni-Import 接住真实世界里不完美的 server 片段，并在安装前看到真实能力与校验结果",
			}}
		>
			<P>
				新增 Server 不只是手填表单。MCPMate 的 Uni-Import 可以接住拖拽、粘贴或文件导入的多种输入，把它们清洗成可编辑草稿，再在真正安装前依次做能力预览和导入校验。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="服务器导入向导与配置预览"
			/>

			<H2>入口在哪里</H2>
			<Ul>
				<Li>Servers 页面右上角的新增按钮会打开安装向导。</Li>
				<Li>这个按钮本身同时也是 Uni-Import 的拖拽目标区域。</Li>
				<Li>等 Chrome / Edge 扩展审核通过后，网页中的兼容 MCP 配置片段也可以被一键捕获并送回 MCPMate。</Li>
				<Li>整个向导分成三个步骤：Configuration、Preview、Import &amp; Profile。</Li>
			</Ul>

			<H2>浏览器扩展入口</H2>
			<P>
				MCPMate 的 Chrome / Edge 扩展是这条 Uni-Import 流程的上游入口。它会在网页里识别疑似 MCP 配置代码块，显示 <strong>Add to MCPMate</strong> 操作，并通过 <code>mcpmate://import/server</code> 深链把片段文本、推断格式和来源页面 URL 一起送回桌面端。
			</P>

			<Callout type="info" title="为什么扩展入口很重要">
				这会再减少一步人工动作。用户不需要先复制代码块、清洗内容、再贴回新增抽屉，而是可以直接从网页把识别到的 MCP 片段送进 MCPMate，然后继续后续的 Uni-Import 流程。
			</Callout>

			<H2>Uni-Import 能接什么输入</H2>
			<Ul>
				<Li>从网页、文档、聊天记录里复制出来的纯文本片段。</Li>
				<Li>JSON / JSON5 片段，包括代码块和不完整的顶层属性列表。</Li>
				<Li>TOML 片段，包括被包裹在上下文里的 section 或 key-value 窗口。</Li>
				<Li><code>.mcpb</code>、<code>.dxt</code> 这类 bundle 文件。</Li>
			</Ul>

			<H2>为什么脏数据也常常能导入</H2>
			<P>
				解析器并不要求输入绝对规整。它可以接住 JSON、JSON5 风格负载、TOML 片段以及 MCP bundle，再把它们统一整理成安装向导可审阅的草稿结构。
			</P>

			<Callout type="info" title="这就是拖拽粘贴体验顺手的原因">
				Uni-Import 的目标不是要求用户先把片段清洗干净，而是在“意图仍然清晰”的前提下，尽量从噪声里恢复出可导入的 server 结构。
			</Callout>

			<H2>Preview 在安装前给你什么</H2>
			<Ul>
				<Li>它给的不是简单配置回显，而是真正的能力预览。</Li>
				<Li>每个 server 都会显示 tools、resources、prompts、templates 的摘要。</Li>
				<Li>你可以展开明细查看具体能力名与描述。</Li>
				<Li>如果预览阶段发现问题，也会先暴露出来，让你带着信息做判断。</Li>
			</Ul>

			<H3>为什么这一步很重要</H3>
			<P>
				Preview 让安装前的透明度更高。你不必盲目信任一段配置，而是可以先看它大致会暴露出什么能力，再决定是否真的把它带进工作环境。
			</P>

			<H2>最后一步为什么还有校验</H2>
			<P>
				进入最后一步后，MCPMate 会先做一次 dry-run import。它会告诉你哪些 server 可以导入，哪些会因为已存在而被跳过，以及是否存在阻塞安装的校验错误；只有可导入时，真正的 Import 按钮才会可用。
			</P>

			<H2>推荐使用方式</H2>
			<Ul>
				<Li>扩展上线后，优先从兼容的文档页、仓库说明或服务目录中一键捕获配置片段。</Li>
				<Li>先拖拽或粘贴，再做手工修正；不要一开始就把 Uni-Import 退化成纯手填。</Li>
				<Li>看 Preview 时重点看能力形态，而不只是“能不能装”。</Li>
				<Li>用最后一步的校验去识别重复项和坏条目，避免脏安装。</Li>
				<Li>导入后如果希望它参与受管暴露，再继续去 Profiles 里分配。</Li>
			</Ul>

			<H2>常见问题</H2>
			<Ul>
				<Li><strong>浏览器扩展实际会带回什么？</strong> 它会把片段文本、推断格式以及来源 URL 一起送入同一条桌面导入链路，方便后续追踪来源。</Li>
				<Li><strong>为什么一段很乱的片段也能被识别？</strong> 因为解析器会尽量从噪声里恢复出可导入结构，而不是只接受教科书式输入。</Li>
				<Li><strong>如果 Preview 报了问题该怎么办？</strong> 把它当成安装前的预警，先理解报错原因，再结合最后一步的校验结果判断是否适合继续导入。</Li>
				<Li><strong>为什么最后一步 Import 按钮是灰的？</strong> 多半是 dry-run 发现没有可导入项，或者存在阻塞校验错误。</Li>
			</Ul>
		</DocLayout>
	);
}
