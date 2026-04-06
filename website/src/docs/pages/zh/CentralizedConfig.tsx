import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function CentralizedConfig() {
	return (
		<DocLayout
			meta={{
				title: "集中配置",
				description: "一次配置，随处使用。消除跨不同客户端的重复设置",
			}}
		>
			<P>
				MCPMate
				的核心功能之一是集中配置管理。您无需为每个 MCP
				客户端维护单独的配置，只需在 MCPMate
				中配置一次服务器，它们就会自动在所有连接的客户端中可用，
				包括遵循标准 MCP 配置布局的用户自定义客户端。
			</P>

			<H2>优势</H2>
			<Ul>
				<Li>
					<strong>单一数据源：</strong>所有 MCP 服务器配置都在一个地方管理
				</Li>
				<Li>
					<strong>无需重复：</strong>消除在不同客户端之间复制设置的需要
				</Li>
				<Li>
					<strong>一致体验：</strong>确保所有客户端使用相同的服务器配置
				</Li>
				<Li>
					<strong>轻松更新：</strong>一次更改配置即可应用到所有地方
				</Li>
			</Ul>

			<H2>工作原理</H2>
			<P>
				MCPMate 充当所有 MCP 服务器的中心枢纽。当您在 MCPMate
				中配置服务器时，它会自动对所有连接的客户端可用。这消除了为每个客户端应用程序手动编辑配置文件的传统工作流程。
			</P>

			<H2>使用场景</H2>
			<Ul>
				<Li>在 Claude Desktop、Cursor、Codex 与用户自定义客户端中复用同一套服务器。</Li>
				<Li>管理团队级的统一服务器配置。</Li>
				<Li>快速接入新客户端而无需重复配置。</Li>
			</Ul>

			<H2>分离部署下的一致性</H2>
			<P>
				即使采用 Core Server + UI 分离运行，集中配置模型不变：UI 负责编辑与编排，核心服务负责持久化与分发，客户端按托管策略读取生效结果。
			</P>
		</DocLayout>
	);
}
