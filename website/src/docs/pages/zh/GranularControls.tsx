import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function GranularControls() {
	return (
		<DocLayout
			meta={{
				title: "精细控制",
				description: "逐项开关每个能力，按需启用/禁用",
			}}
		>
			<P>
				MCPMate 对 MCP
				服务器的每个方面都提供细粒度控制。您可以有选择地启用或禁用每个服务器内的单个工具、提示词和资源，而不是采用全有或全无的方法。
			</P>

			<H2>控制级别</H2>
			<Ul>
				<Li>
					<strong>服务器级别：</strong>启用或禁用整个服务器
				</Li>
				<Li>
					<strong>能力级别：</strong>独立切换工具、提示词和资源
				</Li>
				<Li>
					<strong>单项级别：</strong>控制服务器内的特定工具或提示词
				</Li>
			</Ul>

			<H2>使用场景</H2>
			<Ul>
				<Li>
					<strong>安全性：</strong>禁用潜在的危险操作
				</Li>
				<Li>
					<strong>性能：</strong>通过禁用未使用的功能来减少开销
				</Li>
				<Li>
					<strong>专注：</strong>隐藏无关工具以减少混乱
				</Li>
				<Li>
					<strong>测试：</strong>在开发期间逐步启用功能
				</Li>
			</Ul>

			<H2>优势</H2>
			<P>
				精细控制使您能够精确控制客户端可用的功能。这在团队环境中特别有价值，因为不同用户可能需要不同级别的访问权限。
			</P>

			<H2>推荐上线模式</H2>
			<Ul>
				<Li>先在小范围配置集中启用新能力。</Li>
				<Li>通过检视器验证后再逐步扩大到更多客户端。</Li>
				<Li>结合审计日志确认每次开关变更的时间和操作者。</Li>
			</Ul>
		</DocLayout>
	);
}
