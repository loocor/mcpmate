import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ServerCapabilitiesZH() {
	return (
		<DocLayout meta={{ title: "服务器能力浏览", description: "先看规范化能力清单，再决定是否进入实时调试" }}>
			<P>
				能力标签页主要回答两个问题：服务器声称自己会什么，以及 MCPMate 当前实际拿到了什么。它更像验证与取证工具，而不是普通目录页。
			</P>
			<Callout type="info" title="这里看库存，Inspector 看实时证据">
				如果你要真正发起调用、比较 proxy 与 native、或观察实时事件流，请转到同一 Servers 分组下的 <strong>Inspector</strong> 子页。
			</Callout>
			<section id="tools"><H2>Tools</H2><P>确认可调用动作是否齐全，并与后续配置集暴露面进行对照。</P></section>
			<section id="prompts"><H2>Prompts</H2><P>确认提示词资产是否适合写作、分析或编码类工作流。</P></section>
			<section id="resources"><H2>Resources</H2><P>确认非工具资源是否能正常返回，避免只看名称不看实际可读性。</P></section>
			<section id="templates"><H2>Resource Templates</H2><P>当下游客户端依赖结构化资源入口时，这个标签尤其关键。</P></section>
			<H2>什么时候应该离开这里并进入 Inspector</H2>
			<Ul>
				<Li>导入或编辑后，能力计数看起来不对时。</Li>
				<Li>需要原始响应，而不是规范化后的 UI 摘要时。</Li>
				<Li>需要比较 proxy 与 native channel 行为差异时。</Li>
			</Ul>
			<Callout type="info" title="Inspect 最适合拿证据">
				当你怀疑 UI 症状背后有接口返回问题时，Inspect 是最快把现象和原始数据连起来的方法。
			</Callout>
		</DocLayout>
	);
}
