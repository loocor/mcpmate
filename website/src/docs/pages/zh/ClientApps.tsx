import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ClientAppsZH() {
	return (
		<DocLayout meta={{ title: "兼容应用", description: "与 MCPMate 集成的应用" }}>
			<P>
				客户端页面用于管理能够连接 MCPMate 的桌面应用（如 Cursor、Claude Desktop、Zed、Codex 以及用户自定义客户端）。页面集成自动发现、托管开关与配置位置信息，帮助你随时掌握编辑器与代理的协同状态。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/clients-light.png"
				darkSrc="/screenshot/clients-dark.png"
				alt="客户端网格：检测状态与托管开关"
			/>

			<H2>文档导航</H2>
			<Ul>
				<Li><strong>详情概览</strong> 解释状态徽标、检测结果、文档链接、传输方式与当前服务器卡片。</Li>
				<Li><strong>配置管理</strong> 解释统一模式 / 托管模式 / 透明模式、能力来源、应用流程与导入预览。</Li>
				<Li><strong>备份与恢复</strong> 解释保留策略、回滚、批量删除与恢复场景。</Li>
			</Ul>

			<H2>统计与筛选</H2>
			<Ul>
				<Li>顶部卡片展示发现的应用数、实际检测到的数量、托管中的数量以及已有 MCP 配置的数量。</Li>
				<Li>工具栏支持按名称/标识/描述搜索、按名称或状态排序，并提供与其他页面一致的列表/网格切换。</Li>
				<Li>过滤器可在“全部”“已检测”“托管中”之间切换；选项会写入设置，后续打开页面会沿用该偏好。</Li>
			</Ul>

			<H2>管理集成状态</H2>
			<H3>检测徽标</H3>
			<P>
				每个卡片会显示绿色“已检测”徽章以表明本地找到对应程序。若未检测到，可点击刷新图标触发强制扫描（会调用
				<code>/clients?force_refresh=true</code>），同时确认应用是否安装在默认路径。
			</P>

			<H3>托管开关</H3>
			<P>
				现在这组主操作更偏向治理放行：你可以显式允许或禁行某个客户端。禁行的含义是把它挡在 MCPMate 的能力放行圈之外，而不是锁死它的配置编辑能力。
				即使客户端当前被禁行，你仍然可以继续调整管理模式、能力来源与路径元数据，再决定何时重新允许它。
			</P>

			<H3>详情页</H3>
			<P>点击卡片进入 <code>/clients/:identifier</code>，详情分为三个标签：</P>

			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="客户端详情：配置路径与当前服务器"
			/>

			<Ul>
				<Li>
					<strong>概览</strong>：检测状态、托管开关、应用配置集等操作，以及打开 MCP 配置目录等快捷入口。
				</Li>
				<Li>
					<strong>配置</strong>：展示 MCPMate 将为该客户端写入的 MCP 服务、从客户端导入配置，以及统一模式 / 托管模式 / 透明模式相关说明；对遵循同一 MCP 配置契约的自定义客户端也适用。
				</Li>
				<Li>
					<strong>备份</strong>：应用配置集或导入时生成的轮转快照；可恢复、批量删除，或在成功后刷新列表。
				</Li>
			</Ul>
			<P>
				备份保留数量与客户端页默认过滤器由 <strong>设置 → 客户端默认值</strong>{" "}
				控制，大规模推广前建议先调整好。
			</P>

			<Callout type="warning" title="长期未检测到客户端的处理">
				请确认目标应用安装在默认位置，并确保 MCPMate 具有访问 <strong>/Applications</strong>
				目录的权限。若使用 macOS，必要时在“系统设置 → 隐私与安全性”中授予完整磁盘访问权限，随后点击“刷新”重新扫描。
			</Callout>

			<Callout type="info" title="只有已验证的本地配置目标才允许写入">
				只有当客户端在 New / Edit 抽屉里声明并通过验证，确认它确实拥有一个可写的本地 MCP 配置文件目标时，MCPMate 才会真正写入该客户端自己的配置文件。
				托管模式、统一模式、透明模式都可能产生写入结果，但它们都不能靠“仅仅保存治理状态”来创建这份资格。
			</Callout>
		</DocLayout>
	);
}
