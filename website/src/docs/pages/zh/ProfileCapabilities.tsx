import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ProfileCapabilitiesZH() {
	return (
		<DocLayout meta={{ title: "能力标签页", description: "控制某个配置集真正暴露哪些服务器与能力" }}>
			<P>
				这些标签页决定配置集最终向客户端暴露什么。这里不是单纯浏览数据，而是在制定可调用能力的边界。
			</P>
			<section id="servers"><H2>Servers</H2><P>当你要决定整个服务器是否进入该配置集时，从这里开始最合适。</P><P>如果已开启服务器调试，每个 server 行在悬浮时还会出现 Browse 与 Inspect 操作。需要在当前 profile 上下文里直接跳到该服务器的实时调试工作台时，就用 Inspect。</P></section>
			<section id="tools"><H2>Tools</H2><P>当服务器要保留，但部分可调用动作应对某个工作流隐藏时，用这个标签。</P></section>
			<section id="prompts"><H2>Prompts</H2><P>当写作、编码、分析三类配置集需要不同提示词资产时，用这个标签。</P></section>
			<section id="resources"><H2>Resources</H2><P>当服务器仍需存在，但可读资源只想对部分配置集开放时，用这个标签。</P></section>
			<section id="templates"><H2>Resource Templates</H2><P>当服务器提供结构化资源入口且客户端依赖它们时，检查这个标签。</P></section>
			<H2>通用操作建议</H2>
			<Ul>
				<Li>先过滤，再批量操作，避免误伤无关条目。</Li>
				<Li>优先在配置集层禁用，再考虑全局停用服务器。</Li>
				<Li>批量改动后回到客户端页确认实际暴露面是否符合预期。</Li>
			</Ul>
			<Callout type="warning" title="能力开关会影响真实暴露面">
				这些操作不是标签分类，而是直接改变客户端能看到和调用的内容。有些情况下，它们还会连带影响服务器的可用性判断。
			</Callout>
		</DocLayout>
	);
}
