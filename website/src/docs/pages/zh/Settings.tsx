import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function SettingsZH() {
	return (
		<DocLayout meta={{ title: "设置", description: "应用偏好与配置" }}>
			<P>
				设置页面集中管理控制台的外观、默认行为、市场来源、开发者开关以及后端端口。通过左右分栏的标签切换，可在不离开页面的情况下完成所有参数调整。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/settings-general-light.png"
				darkSrc="/screenshot/settings-general-dark.png"
				alt="设置：通用标签与默认视图"
			/>

			<H2>标签概览</H2>
			<Ul>
				<Li>
					<strong>通用</strong>：选择默认列表视图、运行模式（引导式 / 高级）、界面语言（简体中文、English、日本語）。
				</Li>
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

			<H3>不必一直开着完整桌面界面</H3>
			<P>
				桌面端会单独跟踪本地 core source，并区分 <code>service</code> 与 <code>desktop_managed</code> 等运行模式。对用户来说，这意味着 MCPMate 的本地核心服务可以在后台持续运行，而不是要求你把那个完整桌面窗口一直摆在前台才能工作。
			</P>

			<H3>本地 Web 与 API 访问</H3>
			<P>
				控制台本身就是通过本地 API 和 WebSocket 在工作，API Docs 页面也会指向当前运行中的本地后端。因此，只要服务还在运行，你既可以在浏览器里重新打开管理界面，也可以直接接本地 API 去做自动化或调度集成。
			</P>

			<Callout type="warning" title="端口改动需重启">
				更新端口后，请先用复制的命令重新启动后端，再刷新控制台。否则前端仍会请求旧端口并触发网络错误。
			</Callout>

			<Callout type="info" title="为什么端口配置值得特别说明">
				代码里已经有“端口变更后重写 hosted + managed client 配置”的专门支持，也有面向 transparent 客户端的 profile 同步路径。对用户来说，这意味着受管模式本来就是朝着“尽量不要手工回头逐个改客户端配置”这个方向设计的。
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
