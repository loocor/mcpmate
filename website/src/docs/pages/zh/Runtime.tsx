import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function RuntimeZH() {
	return (
		<DocLayout meta={{ title: "运行时", description: "运行控制与健康状态" }}>
			<P>
				运行时页面展示 MCPMate 所管理的嵌入式环境（目前涵盖 <strong>uv</strong> 与{" "}
				<strong>Bun</strong>）。可在此确认安装情况、清理缓存，并在测试新服务器或传输协议时重置能力缓存。
			</P>

			<H2>运行时卡片</H2>
			<Ul>
				<Li>每张卡片显示可用状态、版本、安装目录、最近状态信息、缓存体积、包数量与最后修改时间。</Li>
				<Li>
					“安装 / 修复”会以 <code>verbose=true</code> 调用安装接口，建议在清理缓存或检测失败后执行，并结合后端日志查看详细输出。
				</Li>
				<Li>“重置缓存”仅清空对应运行时的安装包；下一次调用服务器时会自动重新下载。</Li>
			</Ul>

			<H2>能力缓存</H2>
			<P>
				底部卡片概览能力缓存数据库的路径、大小、最近清理时间，以及服务器、工具、资源、模板的条目计数，同时给出命中/未命中统计与命中率。
			</P>
			<Ul>
				<Li>当服务器能力更新或想让 Inspector 重新获取元数据时，点击“重置能力缓存”。</Li>
				<Li>清空后会立即使签名失效，后续请求将自动拉取最新数据并填充缓存。</Li>
			</Ul>

			<H2>运维建议</H2>
			<H3>导入前检查</H3>
			<P>在导入新服务器之前，确认 uv 与 Bun 状态均为“运行中”且版本合理；若为“已停止”，优先执行安装 / 修复。</P>

			<H3>大规模调整后</H3>
			<P>
				批量调整配置集或服务器后，可手动清理能力缓存，避免出现陈旧提示词或资源。缓存重新填充后，再按照 Inspector 检查表验证响应时间是否达到 5 秒目标。
			</P>

			<Callout type="warning" title="重置缓存会删除已下载的依赖">
				清理 uv 或 Bun 缓存会移除虚拟环境下的安装包。后续首次访问服务器时会重新安装，可能耗时较长。请尽量在维护窗口或自动化测试前操作，避免对使用中的会话造成影响。
			</Callout>
		</DocLayout>
	);
}
