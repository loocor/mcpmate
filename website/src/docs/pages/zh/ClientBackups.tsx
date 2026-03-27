import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientBackupsZH() {
	return (
		<DocLayout meta={{ title: "备份与恢复", description: "让客户端配置变更具备可回滚能力" }}>
			<P>
				备份标签是客户端配置变更的安全网。大规模应用前、导入后、或行为异常时，都应该优先回到这里确认是否可以快速回滚。
			</P>
			<H2>什么时候最重要</H2>
			<Ul>
				<Li>从透明模式切换到托管模式之前。</Li>
				<Li>从客户端已有配置导入之后。</Li>
				<Li>需要快速回退而不想手工重建旧配置时。</Li>
			</Ul>
			<H2>怎么用</H2>
			<Ul>
				<Li>应用成功后刷新列表，确认最新快照已经生成。</Li>
				<Li>单个客户端出问题时优先恢复单条快照。</Li>
				<Li>批量删除前先核对保留策略，避免把最后可用恢复点删掉。</Li>
			</Ul>
			<Callout type="info" title="保留策略在设置里统一控制">
				备份列表是否足够有用，很大程度取决于设置 → 客户端默认值中的保留策略与数量上限。大迁移前先把策略调好。
			</Callout>
		</DocLayout>
	);
}
