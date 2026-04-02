import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientConfigurationZH() {
	return (
		<DocLayout meta={{ title: "配置管理", description: "决定 MCPMate 如何为客户端写入和选择能力来源" }}>
			<P>
				配置管理标签用于决定客户端的管理模式、能力来源、应用行为以及导入预览。它既控制未来想要的目标状态，也帮助你看清磁盘上的现状。
			</P>
			<H2>关键选择</H2>
			<Ul>
				<Li><strong>统一模式</strong> 适合希望通过内建 MCP / UCAN 工具进行会话内控制、且不维护仪表板侧客户端工作集的场景。</Li>
				<Li><strong>托管模式</strong> 适合需要实时切换与更细控制的场景。</Li>
				<Li><strong>透明模式</strong> 适合必须直接写客户端配置文件的兼容场景，但会减少 MCPMate 的精细控制价值。</Li>
				<Li><strong>能力来源</strong> 决定托管模式或透明模式下，客户端跟随已激活配置集、所选共享配置集，还是客户端专属的自定义配置集。</Li>
			</Ul>
			<H2>三种模式真正意味着什么</H2>
			<Ul>
				<Li><strong>统一模式</strong> 初始只提供内建 MCP 工具，会在当前会话中通过内建流程浏览全局启用服务器的 capabilities，并在会话结束后自动重置。</Li>
				<Li><strong>托管模式</strong> 会让客户端只面对 MCPMate 提供的统一入口，因此 profile 切换、可见性控制与策略判断都还能保留在中间层。</Li>
				<Li><strong>透明模式</strong> 会把启用的 server 直接写进客户端自己的 MCP 配置，更适合兼容性或特殊场景。</Li>
			</Ul>
			<H2>来源选择适用于托管模式与透明模式</H2>
			<Ul>
				<Li><strong>统一模式</strong> 不使用这里的仪表板配置集选择。请在当前会话内通过内建 UCAN 工具浏览并调用来自全局启用服务器的 capabilities。</Li>
				<Li><strong>Activated</strong> 跟随全局当前已激活的配置集。</Li>
				<Li><strong>Profiles</strong> 允许某个客户端单独选择一组共享配置集，而不完全跟着全局活动集走。</Li>
				<Li><strong>Customize</strong> 会创建或复用这个客户端自己的专属配置集。</Li>
			</Ul>
			<H2>推荐流程</H2>
			<Ul>
				<Li>先在统一模式、托管模式、透明模式之间选定管理路径，再决定是否需要能力来源选择。</Li>
				<Li>只有在托管模式或透明模式下，才需要继续选择能力来源。</Li>
				<Li>覆盖写入前先看导入预览，避免误覆盖已有配置。</Li>
				<Li>应用前回到概览确认客户端已被正确检测。</Li>
			</Ul>
			<Callout type="warning" title="透明模式意味着不同的取舍">
				透明模式对兼容性很有帮助，但能力级开关的收益会下降，因为最终落到客户端的是更直接的服务器配置。
			</Callout>
			<Callout type="info" title="为什么托管模式会显得更强大">
				托管模式保留了 MCPMate 内建 profile / client 工具、按客户端计算可见性的逻辑，以及更丰富的策略控制。透明模式则是有意简化：它优先保证直接写配置，而不是保留这层运行时控制。
			</Callout>
		</DocLayout>
	);
}
