import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfilePresetsZH() {
	return (
		<DocLayout meta={{ title: "预设模板", description: "将内置模板当作有边界的起点，而不是直接运营对象" }}>
			<P>
				<code>/profiles/presets/:presetId</code> 更适合做比较与选型，而不是日常维护。它帮助你在真正创建配置集前，先看清推荐组合包含哪些服务器与能力。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="配置集预设模板列表"
			/>
			<H2>适用场景</H2>
			<Ul>
				<Li>需要快速起步，但还不想从零设计配置集。</Li>
				<Li>希望先审阅推荐组合，再决定是否投放到团队环境。</Li>
				<Li>给新同事演示标准工作流模板时。</Li>
			</Ul>
			<H2>建议做法</H2>
			<Ul>
				<Li>先看模板覆盖了哪些服务器与能力。</Li>
				<Li>需要长期维护时，创建或克隆成真实配置集。</Li>
				<Li>进入真实配置集后，再去详情页做细化控制。</Li>
			</Ul>
			<Callout type="info" title="为什么预设和真实配置集要分开">
				预设的目标是帮助你做决策，而不是直接参与运行时合并。这样可以避免把“参考模板”误当成“已生效策略”。
			</Callout>
		</DocLayout>
	);
}
