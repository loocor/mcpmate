import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";

export default function ServerInstancesZH() {
	return (
		<DocLayout meta={{ title: "实例管理", description: "针对单个传输实例做定位，而不是笼统地看整台服务器" }}>
			<P>
				当一个服务器包含多个传输实例时，单个实例异常并不一定意味着整台服务器都不可用。实例页适合做精确排查与书签化定位。
			</P>
			<H2>典型场景</H2>
			<Ul>
				<Li>多传输服务器里只有一个连接持续失败。</Li>
				<Li>QA 需要固定追踪某个实例的行为。</Li>
				<Li>需要把单个实例与运行时日志或审计事件对上。</Li>
			</Ul>
			<H2>使用建议</H2>
			<Ul>
				<Li>先从服务器概览判断问题是全局还是局部。</Li>
				<Li>只有在实例层确认异常后，再决定是否重启整台服务器。</Li>
				<Li>实例稳定后，再回到能力页看是否还存在能力读取异常。</Li>
			</Ul>
		</DocLayout>
	);
}
