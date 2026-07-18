import Callout from "../../components/Callout";
import DesktopDownloadList from "../../components/DesktopDownloadList";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

const installCommand = "brew install --cask loocor/tap/mcpmate@beta";
const upgradeCommand = "brew upgrade --cask loocor/tap/mcpmate@beta";
const uninstallCommand = "brew uninstall --cask loocor/tap/mcpmate@beta";

function Command({ children }: { children: string }) {
	return (
		<pre className="not-prose overflow-x-auto rounded-lg border border-brand-border bg-brand-elevated p-3 text-sm text-brand-foreground">
			<code>{children}</code>
		</pre>
	);
}

export default function Installation() {
	return (
		<DocLayout
			meta={{
				title: "安装",
				description: "在 macOS、Windows 和 Linux 上安装、更新或卸载 MCPMate。",
			}}
		>
			<P>
				macOS、Windows 和 Linux 用户可以直接使用官方桌面安装包；如果更习惯通过命令行管理软件，macOS
				和 Linux 也可以使用 Homebrew。
			</P>

			<H2 id="supported-systems">支持的系统</H2>
			<Ul>
				<Li>macOS：ARM64（Apple Silicon）和 x64（Intel）。</Li>
				<Li>Windows：ARM64 和 x64。</Li>
				<Li>Linux：ARM64 和 x64。</Li>
				<Li>Linux 使用 Homebrew 时，最低版本为 Homebrew 5.1.12。</Li>
			</Ul>

			<H2 id="install">安装</H2>
			<H3 id="desktop-install">桌面安装包</H3>
			<P>
				请选择与你的操作系统和处理器架构匹配的安装包。以下链接使用 MCPMate 的下载统计跳转服务，并指向当前版本的发布产物。
			</P>
			<DesktopDownloadList locale="zh" />
			<Ul>
				<Li>macOS：打开 DMG，将 MCPMate 拖入“应用程序”。</Li>
				<Li>Windows：运行 MSI 安装程序并按提示完成安装。</Li>
				<Li>Linux：使用系统包管理器安装 DEB 软件包。</Li>
			</Ul>

			<H3 id="homebrew">Homebrew</H3>
			<P>
				使用下面的完整 Cask 名称即可安装当前 Homebrew Beta 通道，无需预先执行 <code>brew tap</code>。
			</P>
			<Command>{installCommand}</Command>

			<H2 id="upgrade">更新</H2>
			<H3 id="desktop-upgrade">桌面安装包</H3>
			<P>
				先退出 MCPMate，再下载与当前平台和架构相同的最新安装包，并在现有安装上运行。MCPMate 用户数据会继续保留。
			</P>
			<H3 id="homebrew-upgrade">Homebrew</H3>
			<Command>{upgradeCommand}</Command>

			<H2 id="uninstall">卸载</H2>
			<P>卸载前请退出 MCPMate，并让相关服务正常停止。</P>
			<H3 id="desktop-uninstall">桌面安装包</H3>
			<Ul>
				<Li>macOS：将“应用程序”中的 MCPMate 移到废纸篓。</Li>
				<Li>Windows：在“设置 &gt; 应用 &gt; 已安装的应用”中卸载 MCPMate。</Li>
				<Li>Linux：使用安装 DEB 时所用的包管理器移除 MCPMate。</Li>
			</Ul>
			<H3 id="homebrew-uninstall">Homebrew</H3>
			<Command>{uninstallCommand}</Command>
			<Callout type="info" title="用户数据会被保留">
				卸载应用不会删除 <code>~/.mcpmate</code>，其中的配置、数据库和日志都会保留。Homebrew
				也不会清理由其他方式启动的后台服务状态。
			</Callout>
			<P>
				当前尚未提供 <code>mcpmate service uninstall</code> 命令，请在移除应用前正常停止服务。
			</P>
		</DocLayout>
	);
}
