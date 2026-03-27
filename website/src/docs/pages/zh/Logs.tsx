import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function LogsZH() {
	return (
		<DocLayout meta={{ title: "日志", description: "诊断与审计日志" }}>
			<P>
				审计日志页面提供统一时间线，用于追踪 MCPMate 中“谁在何时对哪个对象执行了什么操作”。
			</P>

			<H2>记录范围</H2>
			<Ul>
				<Li>配置集生命周期：创建、更新、激活、停用等。</Li>
				<Li>客户端操作：应用配置、备份、恢复、模式调整等。</Li>
				<Li>服务器操作：创建/导入/更新/启停，以及能力刷新行为。</Li>
				<Li>由审计子系统输出的安全相关事件。</Li>
			</Ul>

			<H2>使用方式</H2>
			<H3>先筛选再下钻</H3>
			<P>
				先按动作类型、分类与时间范围缩小窗口，再查看详细记录。多人协作环境中，这一步能显著提升定位效率。
			</P>

			<H3>使用游标分页加载历史</H3>
			<P>
				审计列表采用基于游标的分页，适合高频写入场景。建议按游标连续拉取，而不是依赖传统 offset 分页。
			</P>

			<H3>结合页面通知做时序复盘</H3>
			<P>
				排查问题时，可对照控制台、客户端、服务器页面通知时间，快速重建完整操作链路。
			</P>

			<Callout type="info" title="运维建议">
				在批量导入服务器、切换客户端模式或大规模调整配置后，建议第一时间复核审计日志，可尽早发现误操作并缩短故障定位时间。
			</Callout>
		</DocLayout>
	);
}
