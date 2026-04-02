import { H3, P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";
import DocScreenshot from "../../components/DocScreenshot";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "服务源安装流程",
				description: "从服务源卡片进入 MCPMate 安装向导的流程说明",
			}}
		>
			<P>
				本页说明你在服务源中选中一个注册中心卡片后会发生什么。MCPMate 会打开与手动导入共用的安装向导，让你在保存前检查传输方式、规范化清单，以及服务器后续应加入哪些配置集。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="服务源安装流程：浏览官方 MCP 注册中心卡片"
			/>

			<h2>这条流程能解决什么</h2>
			<ul>
				<li>
					<strong>服务源到安装的衔接：</strong>从服务源卡片直接进入引导式安装向导
				</li>
				<li>
					<strong>规范化预览：</strong>在保存前确认传输方式与清单细节
				</li>
				<li>
					<strong>受控投放：</strong>先把服务器加入工作区，再决定哪些配置集需要暴露它
				</li>
				<li>
					<strong>一致的导入路径：</strong>服务源安装与拖拽导入共享同一条后续流程
				</li>
			</ul>

			<h2>它在整体流程中的位置</h2>
			<P>
				服务源负责浏览官方 MCP 注册中心；当你决定继续安装时，就会进入这条安装流程，对服务器细节做最后确认，再落到服务器列表中。
			</P>

			<h2>优势</h2>
			<P>
				你不必在注册中心页面、代码片段与本地配置文件之间来回跳转；MCPMate 把发现、检查与安装串成了一条连续流程。
			</P>

			<H3>新增 MCP 服务器向导</H3>
			<P>
				从注册卡片安装时会打开引导流程：配置传输方式、预览规范化后的清单、保存服务器，然后再从“服务器”或“配置集”页面决定它应该加入哪些配置集。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="新增 MCP 服务器：核心配置步骤"
			/>
		</DocLayout>
	);
}
