import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function UniImport() {
	return (
		<DocLayout
			meta={{
				title: "全能导入",
				description: "通过拖拽或粘贴快速导入配置，并自动归一化传输类型",
			}}
		>
			<P>
				全能导入是 MCPMate
				的灵活配置导入系统。无论您有 JSON 文件、TOML 配置还是文本片段，都可以通过简单的拖放或粘贴操作将其导入到
				MCPMate 中。
			</P>

			<H2>支持的格式</H2>
			<Ul>
				<Li>
					<strong>JSON：</strong>标准 MCP 配置格式
				</Li>
				<Li>
					<strong>TOML：</strong>替代配置格式
				</Li>
				<Li>
					<strong>文本片段：</strong>可直接粘贴来自文档、聊天或团队 wiki 的配置
				</Li>
			</Ul>

			<H2>导入方式</H2>
			<Ul>
				<Li>
					<strong>拖放：</strong>只需将配置文件拖放到 MCPMate 中
				</Li>
				<Li>
					<strong>粘贴：</strong>复制配置文本并粘贴到导入对话框中
				</Li>
				<Li>
					<strong>文件浏览器：</strong>传统的文件选择对话框
				</Li>
			</Ul>

			<H2>智能解析</H2>
			<P>
				全能导入会自动检测配置格式并在导入前进行验证。如果有任何问题，MCPMate
				会提供清晰的错误消息和修复建议。历史 SSE 风格输入会在持久化时自动归一化为
				Streamable HTTP。
			</P>

			<H2>使用场景</H2>
			<Ul>
				<Li>导入共享的团队配置。</Li>
				<Li>从其他 MCP 工具迁移。</Li>
				<Li>从文档示例快速设置。</Li>
				<Li>从备份恢复。</Li>
			</Ul>
		</DocLayout>
	);
}
