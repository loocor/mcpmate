import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ProfileZH() {
	return (
		<DocLayout
			meta={{ title: "配置集", description: "将服务器、工具与提示词打包为可复用预设" }}
		>
			<P>
				配置集（内部称作 <em>suit</em>）用于把 MCP 服务器、工具、资源、提示词组合成命名预设。页面提供检索、统计、抽屉式编辑与启停按钮，所有操作都会立即同步到代理。
			</P>

			<Callout type="info" title="默认锚点套件">
				角色为 <code>default_anchor</code> 的套件会固定在列表顶部且不可停用，用于保证基础能力始终可用。
			</Callout>

			<H2>统计卡与工具栏</H2>
			<Ul>
				<Li>四张卡片汇总已激活套件、启用服务器、启用工具、就绪实例数量，数据来自 <code>/config/suits</code> 系列接口。</Li>
				<Li>工具栏支持名称/描述搜索、名称或启用状态排序，以及网格/列表切换；切换结果会写入设置中的全局默认视图。</Li>
				<Li>右上角的刷新按钮可强制重新拉取数据，新增按钮会打开创建抽屉。</Li>
			</Ul>

			<H2>创建与编辑</H2>
			<H3>新建抽屉</H3>
			<P>
				点击“新建配置集”会打开侧边抽屉，可填写展示名称、描述、套件类型等信息。若通过预设快捷入口（如
				<code>?type=writer</code>）进入，表单会预选对应模板。
			</P>

			<H3>详情页</H3>
			<P>
				选择套件卡片会跳转至 <code>/profiles/:id</code>，在该页可查看服务器、工具、资源、提示词的明细与开关，并附带返回面包屑。
			</P>

			<H2>激活流程</H2>
			<Ul>
				<Li>卡片或列表项右侧的开关用于启用/暂停套件，操作后立即调用激活/停用接口并通过提示信息反馈结果。</Li>
				<Li>多个套件同时启用时，其服务器与工具会在运行时合并，顶部统计会实时反映合并后的规模。</Li>
				<Li>默认锚点套件的开关会处于禁用状态，避免误操作。</Li>
			</Ul>

			<Callout type="warning" title="激活卡顿的排查方式">
				若开关长时间无响应，可先在运行时页面确认代理健康，再点击刷新按钮重新拉取数据。必要时通过 API 文档访问
				<code>/config/suits/:id</code>，验证后端是否已持久化变更。
			</Callout>
		</DocLayout>
	);
}
