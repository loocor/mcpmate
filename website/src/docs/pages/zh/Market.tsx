import Callout from "../../components/Callout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function MarketZH() {
	return (
		<DocLayout meta={{ title: "服务源", description: "浏览与管理社区服务器" }}>
			<P>
				服务源承载 MCPMate
				的服务器目录，可浏览官方仓库、接入自建门户、查看元数据并将候选项直接送往安装向导。
			</P>

			<H2>标签页与数据源</H2>
			<Ul>
				<Li>
					<strong>官方</strong> 标签展示 MCPMate 官方注册表，支持实时搜索（带
					300ms 防抖）与多种排序（最近、热门、字母序），翻页时会追加加载列表。
				</Li>
				<Li>
					<strong>门户</strong> 标签以 iframe 方式加载第三方市场。可在设置 → MCP
					市场添加或移除门户；切换语言时会自动刷新请求翻译后的目录。
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

			<H3>隐藏与黑名单</H3>
			<P>
				选择“隐藏”后条目会加入本地黑名单，从官方列表中移除。可在设置 → MCP
				市场搜索并恢复这些条目。
			</P>

			<H2>门户管理技巧</H2>
			<Ul>
				<Li>
					在设置中指定 <strong>默认市场</strong>
					，控制台会自动打开该标签且无法关闭。
				</Li>
				<Li>
					使用“打开门户”按钮在新窗口访问外部界面，同时保留当前面板用于安装。
				</Li>
				<Li>
					如门户需要认证头或自定义变量，可在设置里配置，导入流程会自动复用。
				</Li>
			</Ul>

			<Callout type="info" title="与服务器页面的联动">
				所有在市场完成的导入都会经过相同的服务器安装向导，并立即出现在服务器列表中，随后即可针对配置集启用或禁用、查看连接状态。
			</Callout>
		</DocLayout>
	);
}
