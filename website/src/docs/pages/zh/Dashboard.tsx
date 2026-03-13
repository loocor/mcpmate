import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function DashboardZH() {
	return (
		<DocLayout meta={{ title: "控制台", description: "系统状态与关键面板总览" }}>
			<P>
				控制台是进入 MCPMate 控制台后的默认页面，用于快速了解代理健康度、活跃的配置集、连通的服务器与客户端，以及 CPU/内存趋势，帮助你决定下一步的排查方向。
			</P>

			<Callout type="info" title="刷新节奏">
				系统状态、配置集、服务器、客户端卡片以 30 秒为周期轮询；底部资源图表每 10 秒采样一次，并将最近 60
				个点写入本地存储，短暂刷新页面不会丢失上下文。
			</Callout>

			<H2>状态卡片</H2>
			<Ul>
				<Li>
					<strong>系统状态</strong>：显示运行状态、版本、持续运行时间，点击卡片可跳转至运行时页面执行修复。
				</Li>
				<Li>
					<strong>配置集</strong>：展示总数与已激活数量，排序策略与配置集页面一致，便于发现差异。
				</Li>
				<Li>
					<strong>服务器</strong>：列出注册数量、启用数量和当前连通数，点击后进入服务器列表查看详细能力与 Uni-Import。
				</Li>
				<Li>
					<strong>客户端</strong>：提示检测到与被托管的桌面端应用，帮助你评估配置推广情况。
				</Li>
			</Ul>

			<H2>资源指标折线图</H2>
			<P>
				底部图表展示四条曲线：MCPMate CPU%、MCPMate 内存%、主机 CPU%、主机内存%。悬停可查看具体值，暗色模式下网格与配色会自动适配。若暂未产生数据，界面会显示提示而非空图。
			</P>

			<H3>读图要点</H3>
			<Ul>
				<Li>关注 MCPMate CPU 峰值，可快速定位高负载服务器或深度检视操作。</Li>
				<Li>对比 MCPMate 与主机内存占比，判断代理本身消耗与整体压力。</Li>
				<Li>提示框会同步显示内存的 MB 数值，便于换算绝对体量。</Li>
			</Ul>

			<H2>建议检查清单</H2>
			<Ul>
				<Li>确认状态为“运行中”且版本正确，再执行配置或服务器变更。</Li>
				<Li>观察服务器/客户端连接数是否突降，判断是否出现重启或异常。</Li>
				<Li>在并发测试或 Inspector 循环时关注 CPU 趋势，确保落在预期阈值以内。</Li>
			</Ul>

			<Callout type="warning" title="数据不更新时的处理">
				请确认浏览器能访问 <code>http://127.0.0.1:8080</code>。若后端进程停止，卡片会保留上次结果。重新启动代理并刷新页面即可恢复实时数据。
			</Callout>
		</DocLayout>
	);
}
