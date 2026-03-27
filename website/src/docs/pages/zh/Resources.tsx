import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function ResourcesZH() {
	return (
		<DocLayout meta={{ title: "资源", description: "服务器暴露的共享资源" }}>
			<P>
				资源是 MCP 服务器对外暴露的非工具类内容，例如文档、文件引用、结构化产物等，可由客户端读取和消费。
			</P>

			<H2>在哪里管理资源</H2>
			<Ul>
				<Li>进入服务器详情页，在 Resources 标签查看可用资源。</Li>
				<Li>通过配置集控制该服务器是否对客户端生效。</Li>
				<Li>结合检视器验证资源可见性与返回结构。</Li>
			</Ul>

			<H2>运维建议</H2>
			<Ul>
				<Li>资源体量较大的服务器建议仅在特定配置集中启用。</Li>
				<Li>服务器命名尽量语义化，方便识别资源归属。</Li>
				<Li>涉及资源权限变更时，建议在审计日志中复核操作记录。</Li>
			</Ul>
		</DocLayout>
	);
}
