import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function PromptsZH() {
	return (
		<DocLayout meta={{ title: "提示词", description: "提示词管理与覆盖" }}>
			<P>
				提示词是由 MCP 服务器暴露的可复用指令资产。MCPMate 通过配置集启停来控制提示词在客户端中的可见范围。
			</P>

			<H2>提示词可见性控制</H2>
			<Ul>
				<Li>在配置集中启用或禁用相关服务器与能力项。</Li>
				<Li>在托管（Hosted）模式下应用配置集，实现即时切换。</Li>
				<Li>通过服务器详情与检视器验证提示词是否按预期暴露。</Li>
			</Ul>

			<H2>实践建议</H2>
			<Ul>
				<Li>按写作/编码/分析场景拆分提示词配置集，减少干扰。</Li>
				<Li>团队共用提示词可放入默认锚点配置集。</Li>
				<Li>涉及提示词策略调整时，建议在审计日志中复核变更轨迹。</Li>
			</Ul>
		</DocLayout>
	);
}
