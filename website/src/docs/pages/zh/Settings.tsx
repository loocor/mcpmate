import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function SettingsZH() {
	return (
		<DocLayout meta={{ title: "设置", description: "应用偏好与配置" }}>
			<P>
				设置页面集中管理控制台的外观、默认行为、市场来源、开发者开关以及后端端口。通过左右分栏的标签切换，可在不离开页面的情况下完成所有参数调整。
			</P>

			<H2>标签概览</H2>
			<Ul>
				<Li><strong>通用</strong>：选择默认列表视图、应用模式（Express / Expert）、预设语言（开发中）。</Li>
				<Li><strong>外观</strong>：切换亮暗主题、是否跟随系统；在 Tauri 桌面端时可配置菜单栏与 Dock 图标策略。</Li>
				<Li><strong>服务器控制</strong>：确定启停是否同步到托管客户端，以及新服务器是否自动加入默认配置集。</Li>
				<Li><strong>客户端默认值</strong>：设置托管模式、默认过滤器、备份策略与备份上限，影响客户端页面的工具栏与行为。</Li>
				<Li><strong>MCP 市场</strong>：指定默认门户、开关黑名单、搜索与恢复隐藏条目。</Li>
				<Li><strong>开发者</strong>：打开服务器调试按钮、选择 Inspect 是否新开窗口、显示 API 文档快捷键、展示原始 JSON 或默认 Header。</Li>
				<Li><strong>系统</strong>：修改 API/MCP 端口、复制启动命令、停止当前运行的后端实例。</Li>
				<Li><strong>关于与许可</strong>：当后端返回许可证数据时出现，用于查看依赖协议信息。</Li>
			</Ul>

			<H2>重点流程</H2>
			<H3>调整默认布局</H3>
			<P>在“通用 → 默认视图”中选择列表或网格，配置立即生效并影响配置集、客户端、服务器三个页面的默认视图。</P>

			<H3>控制客户端策略</H3>
			<P>
				“客户端默认值”允许设定托管模式、默认过滤器与备份策略，这些参数会写入全局状态供客户端页面读取，从而保持工具栏行为一致。
			</P>

			<H3>整理市场内容</H3>
			<P>
				“MCP 市场”标签可指定默认门户、启用或关闭黑名单，并通过搜索/排序快速找到被隐藏的服务器以便恢复，适合 QA 对历史条目回溯。
			</P>

			<H3>调整端口</H3>
			<P>
				在“系统”标签中可修改 API 与 MCP 端口，并一键复制 <code>cargo run</code> 或发布版二进制的启动命令；必要时可直接发起停止请求终止当前后端。
			</P>

			<Callout type="warning" title="端口改动需重启">
				更新端口后，请先用复制的命令重新启动后端，再刷新控制台。否则前端仍会请求旧端口并触发网络错误。
			</Callout>

			<H2>开发者选项</H2>
			<Ul>
				<Li>启用“服务器调试”后，服务器列表会新增 Inspect 按钮用于查看原始数据。</Li>
				<Li>“Inspect 新开窗口”让调试在新标签进行，保持列表上下文不丢失。</Li>
				<Li>开启“显示原始能力 JSON”可在 Uni-Import 与服务器详情中查看未处理的返回体，便于验证后端格式。</Li>
			</Ul>

			<Callout type="info" title="桌面端专属配置">
				菜单栏图标与 Dock 图标选项只在 Tauri 桌面版本出现。若你正在使用 Web 预览，这些开关会自动隐藏，需要在打包的 macOS 应用中验证。
			</Callout>
		</DocLayout>
	);
}
