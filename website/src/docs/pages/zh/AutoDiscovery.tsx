import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "自动发现与导入",
				description: "发现本地 MCP 配置，并使用 Discovery 预设加快设置。",
			}}
		>
			<P>
				MCPMate 会把本地配置扫描和 Public Discovery 目录结合起来。本地发现用于识别当前机器上已有的 MCP 设置；Discovery 预设则为新的客户端和服务器设置提供经过整理的起点。
			</P>

			<H2>本地发现</H2>
			<P>MCPMate 扫描流行 MCP 客户端使用的常见配置位置：</P>
			<Ul>
				<Li>Claude Desktop 配置文件</Li>
				<Li>VS Code MCP 扩展设置</Li>
				<Li>Cursor MCP 配置</Li>
				<Li>其他标准 MCP 客户端设置（包括用户自定义客户端）</Li>
			</Ul>

			<H2>Discovery 预设</H2>
			<P>
				Public Discovery 会为首次运行流程、客户端新增/编辑抽屉，以及浏览器扩展提供预设条目。这些条目包含标识符、显示名称、链接、图标和导入元数据，方便 MCPMate 在你确认前先生成更清晰的草稿。
			</P>
			<Ul>
				<Li>客户端预设用于添加已有 MCP 配置目标的 AI 应用。</Li>
				<Li>服务器条目为服务器安装向导提供可导入的元数据。</Li>
				<Li>门户条目会连接服务源文档和浏览器扩展的 Portal 标签页。</Li>
			</Ul>

			<H2>导入流程</H2>
			<Ul>
				<Li>MCPMate 扫描本地已有配置。</Li>
				<Li>导入界面会展示已发现服务器和 Discovery 支持的草稿。</Li>
				<Li>选择导入对象及目标配置集。</Li>
				<Li>导入后自动完成结构归一化并持久化。</Li>
			</Ul>

			<H2>优势</H2>
			<Ul>
				<Li>
					<strong>快速上手：</strong>立即开始使用 MCPMate
				</Li>
				<Li>
					<strong>引导式设置：</strong>从已检测的本地状态或 Discovery 预设开始
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
