import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ToolsZH() {
	return (
		<DocLayout meta={{ title: "工具", description: "在控制台中查看与治理 MCP 工具" }}>
			<P>
				MCP <strong>工具（tools）</strong> 由各 MCP 服务器声明的可调用能力。在 Board
				里它们不是侧栏独立入口，而是在 <strong>服务器</strong> 与 <strong>配置集</strong>{" "}
				中管理，以便与具体服务及信任边界绑定。
			</P>

			<Callout type="info" title="在 Board 中从哪里操作">
				进入 <code>/servers/:serverId</code>，在 <strong>能力（Capabilities）</strong>{" "}
				区域打开 <strong>工具</strong> 标签，可查看名称、描述与启停状态。在配置集详情中，同一批工具键会以开关形式出现，用于控制该配置集对外暴露的子集。
			</Callout>

			<H2>启停层次</H2>
			<Ul>
				<Li>
					<strong>服务器</strong>：关闭服务器后，其工具在所有配置集中均不可用，直至重新启用。
				</Li>
				<Li>
					<strong>配置集</strong>：在已激活的配置集中可按工具粒度开关，无需卸载服务器即可缩小合并后的能力面。
				</Li>
				<Li>
					<strong>客户端</strong>：托管模式会接收活跃配置集的合并结果；透明模式仅反映已写入磁盘的配置。
				</Li>
			</Ul>

			<H2>发现与调试</H2>
			<H3>能力缓存</H3>
			<P>
				工具元数据会被缓存以提升性能。若服务器更新了清单，可在 <strong>运行时</strong>{" "}
				页面重置能力缓存或重启代理，使列表与 Inspector CLI 一致。
			</P>

			<H3>原始 JSON</H3>
			<P>
				在 <strong>设置 → 开发者</strong> 中开启「显示原始能力 JSON」，便于对照代理返回与界面渲染。
			</P>

			<P>
				具体操作路径请参阅本站的 <strong>服务器</strong> 与 <strong>配置集</strong>{" "}
				指南，其章节与 Board 路由一致。
			</P>
		</DocLayout>
	);
}
