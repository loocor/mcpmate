import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerDetailOverviewZH() {
	return (
		<DocLayout meta={{ title: "详情概览", description: "先判断服务器生命周期是否稳定，再进入能力层排查" }}>
			<P>
				<code>/servers/:serverId</code> 的浏览视图用于判断一个服务器是否值得继续留在运行时中。它集中显示状态、传输、实例，以及启停、编辑、删除等动作。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/server-detail-light.png"
				darkSrc="/screenshot/server-detail-dark.png"
				alt="服务器详情概览"
			/>
			<H2>先确认什么</H2>
			<Ul>
				<Li>状态是过渡中还是稳定状态。</Li>
				<Li>实例数量是否符合预期，尤其是多传输服务器。</Li>
				<Li>编辑或重启是否会影响当前依赖它的客户端。</Li>
			</Ul>
			<H3>为什么要先看概览</H3>
			<P>如果服务器本身不健康，能力列表往往只是表象。先把生命周期稳定下来，再去看能力和调试数据。</P>
			<Callout type="warning" title="刷新能力不等于启用服务器">
				刷新是重新拉取元数据；启停则会改变运行时可用性。排查时请先分清你真正要解决的是哪一类问题。
			</Callout>
		</DocLayout>
	);
}
