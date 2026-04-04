import Callout from "../../components/Callout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";
import DocScreenshot from "../../components/DocScreenshot";

export default function MarketZH() {
	return (
		<DocLayout meta={{ title: "服务源", description: "浏览与管理社区服务器" }}>
			<P>
				服务源展示 MCPMate 官方注册表，可搜索、排序并将候选项送往安装向导。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="MCP 服务源列表与搜索"
			/>

			<H2>数据与导入</H2>
			<Ul>
				<Li>
					<strong>官方</strong> 注册表支持搜索（带 300ms 防抖）与排序（最近、字母序），翻页追加加载。
				</Li>
				<Li>
					在任意网页导入 MCP 配置片段可使用 Chrome 扩展（仓库内{" "}
					<code>extension/chrome</code>），通过{" "}
					<code>mcpmate://import/server</code> 唤起桌面端。
				</Li>
				<Li>
					远程连接器会显示在“Remote”区域，通常是预先配置好的 URL/Git
					仓库，可一键进入安装流程。
				</Li>
			</Ul>

			<H2>安装流程</H2>
			<H3>预览抽屉</H3>
			<P>
				点击服务器卡片打开预览抽屉，可查看描述、能力统计、传输类型、必要的
				Header 或环境变量。按下“导入”会启动 Uni-Import
				向导并预填草稿，方便在保存前调整别名。
			</P>

			<H3>支持 OAuth 的上游服务器</H3>
			<P>
				对于需要 OAuth 的上游 Streamable HTTP 服务器，安装向导会先准备授权元数据，再拉起提供方登录窗口。授权完成后，MCPMate 接收回调并自动关闭弹窗，随后继续当前安装流程。
			</P>

			<H3>隐藏与黑名单</H3>
			<P>
				选择“隐藏”后条目会加入本地黑名单，从官方列表中移除。可在设置 → MCP
				市场搜索并恢复这些条目。
			</P>

			<H2>黑名单</H2>
			<P>
				在设置 → MCP 市场管理已隐藏的条目，可随时恢复并重新显示在列表中。
			</P>

			<Callout type="info" title="与服务器页面的联动">
				所有在市场完成的导入都会经过相同的服务器安装向导，并立即出现在服务器列表中，随后即可针对配置集启用或禁用、查看连接状态。
			</Callout>
		</DocLayout>
	);
}
