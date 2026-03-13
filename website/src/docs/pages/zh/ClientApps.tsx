import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientAppsZH() {
	return (
		<DocLayout meta={{ title: "兼容应用", description: "与 MCPMate 集成的应用" }}>
			<P>
				客户端页面用于管理能够连接 MCPMate 的桌面应用（如 Cursor、Claude Desktop、Zed 等）。页面集成自动发现、托管开关与配置位置信息，帮助你随时掌握编辑器与代理的协同状态。
			</P>

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
				右下角开关负责打开或关闭托管模式。启用后 MCPMate 会同步配置集变更到该客户端；切换完成后会通过通知提示成功或失败。
			</P>

			<H3>详情页</H3>
			<P>
				点击卡片跳转至 <code>/clients/:identifier</code>，可查看 MCP 服务器绑定、版本信息、下载链接以及打开配置目录等快捷操作。
			</P>

			<Callout type="warning" title="长期未检测到客户端的处理">
				请确认目标应用安装在默认位置，并确保 MCPMate 具有访问 <strong>/Applications</strong>
				目录的权限。若使用 macOS，必要时在“系统设置 → 隐私与安全性”中授予完整磁盘访问权限，随后点击“刷新”重新扫描。
			</Callout>
		</DocLayout>
	);
}
