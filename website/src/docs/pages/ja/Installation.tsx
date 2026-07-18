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
				title: "インストール",
				description: "macOS、Windows、Linux で MCPMate をインストール、更新、アンインストールします。",
			}}
		>
			<P>
				macOS、Windows、Linux では公式デスクトップインストーラーを利用できます。コマンドラインでパッケージを管理したい場合は、macOS と Linux で Homebrew も利用できます。
			</P>

			<H2 id="supported-systems">対応システム</H2>
			<Ul>
				<Li>macOS：ARM64（Apple Silicon）および x64（Intel）。</Li>
				<Li>Windows：ARM64 および x64。</Li>
				<Li>Linux：ARM64 および x64。</Li>
				<Li>Linux の Homebrew には Homebrew 5.1.12 以降が必要です。</Li>
			</Ul>

			<H2 id="install">インストール</H2>
			<H3 id="desktop-install">デスクトップインストーラー</H3>
			<P>
				OS とプロセッサーアーキテクチャに合うパッケージを選んでください。以下のリンクは MCPMate の追跡可能なダウンロードサービスを経由し、現在のリリース成果物を取得します。
			</P>
			<DesktopDownloadList locale="ja" />
			<Ul>
				<Li>macOS：DMG を開き、MCPMate を Applications に移動します。</Li>
				<Li>Windows：MSI インストーラーを実行し、画面の案内に従います。</Li>
				<Li>Linux：システムのパッケージマネージャーで DEB をインストールします。</Li>
			</Ul>

			<H3 id="homebrew">Homebrew</H3>
			<P>
				完全修飾された Cask 名を使うと、別途 <code>brew tap</code> を実行せずに現在の Homebrew Beta チャンネルをインストールできます。
			</P>
			<Command>{installCommand}</Command>

			<H2 id="upgrade">更新</H2>
			<H3 id="desktop-upgrade">デスクトップインストーラー</H3>
			<P>
				MCPMate を終了し、同じプラットフォームとアーキテクチャの最新パッケージをダウンロードして、既存のインストールに上書きしてください。ユーザーデータは保持されます。
			</P>
			<H3 id="homebrew-upgrade">Homebrew</H3>
			<Command>{upgradeCommand}</Command>

			<H2 id="uninstall">アンインストール</H2>
			<P>MCPMate を終了し、関連サービスを通常どおり停止してからアンインストールしてください。</P>
			<H3 id="desktop-uninstall">デスクトップインストーラー</H3>
			<Ul>
				<Li>macOS：Applications の MCPMate をゴミ箱へ移動します。</Li>
				<Li>Windows：設定 &gt; アプリ &gt; インストールされているアプリから MCPMate を削除します。</Li>
				<Li>Linux：DEB のインストールに使用したパッケージマネージャーで MCPMate を削除します。</Li>
			</Ul>
			<H3 id="homebrew-uninstall">Homebrew</H3>
			<Command>{uninstallCommand}</Command>
			<Callout type="info" title="ユーザーデータは保持されます">
				アプリを削除しても、設定、データベース、ログを含む <code>~/.mcpmate</code> は保持されます。Homebrew は別途実行中のバックグラウンドサービス状態を削除しません。
			</Callout>
			<P>
				現在は <code>mcpmate service uninstall</code> コマンドを提供していません。アプリを削除する前にサービスを通常どおり停止してください。
			</P>
		</DocLayout>
	);
}
