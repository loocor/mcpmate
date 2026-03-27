import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfileDetailOverviewZH() {
	return (
		<DocLayout meta={{ title: "详情概览", description: "先读懂配置集状态与约束，再进入细粒度能力控制" }}>
			<P>
				<code>/profiles/:profileId</code> 的概览标签是最适合做判断的地方。它集中展示状态、类型、多选策略、优先级以及刷新、编辑、默认、启停、删除等动作。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="配置集详情概览"
			/>
			<H2>先看什么</H2>
			<Ul>
				<Li><strong>状态</strong> 决定它是否正在参与运行时合并。</Li>
				<Li><strong>类型</strong> 决定它是共享配置、宿主应用配置还是特殊工作流对象。</Li>
				<Li><strong>优先级</strong> 决定多个激活配置集重叠时的解析顺序。</Li>
			</Ul>
			<H3>为什么统计卡重要</H3>
			<P>统计卡可以让你在改动前预估影响范围，也能在问题发生后快速跳到对应标签页核实细节。</P>
			<Callout type="warning" title="默认与锚点限制是策略，不是故障">
				如果某些按钮不可点，通常是为了保护基础能力覆盖范围。先确认配置集角色，再决定下一步，而不是把它当成界面异常。
			</Callout>
		</DocLayout>
	);
}
