import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "自动发现与导入",
				description: "自动检测并导入现有配置，无需手工编辑",
			}}
		>
			<P>
				MCPMate 可以自动发现系统上现有的 MCP
				服务器配置，并一键导入。这消除了在新工具中手动重新创建设置的繁琐过程。
				对于把 MCP 配置保存在标准位置的用户自定义客户端，这套导入流程同样适用。
			</P>

			<H2>工作原理</H2>
			<P>MCPMate 扫描流行 MCP 客户端使用的常见配置位置：</P>
			<Ul>
				<Li>Claude Desktop 配置文件</Li>
				<Li>VS Code MCP 扩展设置</Li>
				<Li>Cursor MCP 配置</Li>
				<Li>其他标准 MCP 客户端设置（包括用户自定义客户端）</Li>
			</Ul>

			<H2>导入流程</H2>
			<Ul>
				<Li>MCPMate 自动扫描现有配置。</Li>
				<Li>在导入界面展示可导入服务器。</Li>
				<Li>选择导入对象及目标配置集。</Li>
				<Li>导入后自动完成结构归一化并持久化。</Li>
			</Ul>

			<H2>优势</H2>
			<Ul>
				<Li>
					<strong>快速上手：</strong>立即开始使用 MCPMate
				</Li>
				<Li>
					<strong>无需手动工作：</strong>避免手动复制配置详细信息
				</Li>
				<Li>
					<strong>保留现有设置：</strong>您的原始配置保持不变
				</Li>
				<Li>
					<strong>防止错误：</strong>减少手动输入导致的配置错误
				</Li>
			</Ul>
		</DocLayout>
	);
}
