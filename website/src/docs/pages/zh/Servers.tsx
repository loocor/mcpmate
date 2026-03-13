import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ServersZH() {
	return (
		<DocLayout meta={{ title: "服务器", description: "管理与连接 MCP 服务器" }}>
			<P>
				服务器页面集中展示所有 MCP 服务器，包含连接状态、能力统计、启停开关以及导入流程。无需手动编辑配置文件，就能完成安装与维护。
			</P>

			<H2>统计卡与工具栏</H2>
			<Ul>
				<Li>顶部卡片给出总数量、已启用数量、当前连通数以及实例总数，方便第一时间捕捉异常。</Li>
				<Li>工具栏提供名称/描述搜索、名称或启用状态排序，并继承全局的列表/网格视图设置。</Li>
				<Li>操作区包含刷新按钮与“新增”按钮，后者同时充当 Uni-Import 的拖拽区域。</Li>
			</Ul>

			<H2>卡片信息点</H2>
			<P>
				每张卡片都会显示能力统计（工具、提示词、资源、模板）、传输类型徽章以及实时状态指示。右下角开关用于启停服务器，状态将同步到后端。
			</P>
			<Ul>
				<Li>点击卡片跳转 <code>/servers/:id</code> 查看实例详情、日志以及市场来源信息。</Li>
				<Li>在设置 → 开发者中开启“服务器调试”后，会出现 Inspect 按钮，可在当前页或新窗口中查看原始数据。</Li>
				<Li>当服务器状态异常（error/unhealthy 等）时，状态徽章会闪烁提醒。</Li>
			</Ul>

			<H2>导入与编辑</H2>
			<H3>Uni-Import 流程</H3>
			<P>
				将 <code>.mcpb</code>、<code>.dxt</code>、JSON 片段、URL 或纯文本拖拽到新增按钮上即可触发安装向导。系统会解析内容、标准化传输配置，并在提交前提供预览。
			</P>

			<H3>手动表单与编辑抽屉</H3>
			<P>
				点击“新增服务器”打开手动录入表单。已有服务器可在详情页调用编辑抽屉调整元信息、密钥或实例设置，整个过程无需重启代理。
			</P>

			<Callout type="info" title="排查加载失败">
				若列表加载失败，可在设置 → 开发者启用“服务器调试”，随后点击 Inspect 按钮查看原始响应、错误信息与渲染数据，方便比对后端接口返回。
			</Callout>

			<H2>操作建议</H2>
			<Ul>
				<Li>导入后立即核对能力统计是否与市场或配置文件一致。</Li>
				<Li>确认实例总数与 Inspector CLI 输出相符，以防遗漏。</Li>
				<Li>启停服务器时同时关注运行时日志，确保没有额外错误。</Li>
			</Ul>
		</DocLayout>
	);
}
