import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function BrowserExtensionJA() {
	return (
		<DocLayout
			meta={{
				title: "ブラウザー拡張機能",
				description:
					"MCPMate のブラウザー拡張機能で Discovery エントリを確認し、Web 上の MCP スニペットをデスクトップのインポートフローへ送ります。",
			}}
		>
			<P>
				MCPMate の Chrome / Edge 拡張は、MCP Server が見つかる Web ページの近くに
				発見入口を置きます。ツールバーのポップアップには MCPMate Public
				Discovery 由来の Portals、Servers、Clients が表示され、ページ上の
				MCP スニペットもデスクトップアプリへ送れます。
			</P>

			<H2>Discovery タブ</H2>
			<Ul>
				<Li>
					<strong>Portals</strong> は MCP の発見先やコミュニティリソースを表示します。
				</Li>
				<Li>
					<strong>Servers</strong> は MCPMate Admin から公開された選定サーバーを表示します。
				</Li>
				<Li>
					<strong>Clients</strong> は互換 AI アプリとセットアップに使えるクライアントプリセットを表示します。
				</Li>
			</Ul>

			<H2>スニペット連携</H2>
			<P>
				Web ページに MCP Server 設定らしいコードブロックがある場合、拡張は{" "}
				<strong>Add to MCPMate</strong> アクションを表示します。クリックすると{" "}
				<code>mcpmate://import/server</code> でデスクトップアプリを開き、スニペット本文、
				推定フォーマット、参照元 URL を渡します。その後は Servers ページと同じ
				Uni-Import のプレビューと検証フローに進みます。
			</P>

			<H2>カタログ読み込み</H2>
			<Ul>
				<Li>Discovery エントリは MCPMate Public Discovery API から読み込まれます。</Li>
				<Li>Servers と Clients はポップアップのスクロールに合わせてページング読み込みします。</Li>
				<Li>Discovery レスポンスはローカルにキャッシュされ、次回以降の表示を速くします。</Li>
				<Li>初回はブラウザ言語に合わせ、ポップアップ設定から変更できます。</Li>
			</Ul>

			<H2>インストールリンク</H2>
			<Ul>
				<Li>
					Chrome Web Store:{" "}
					<a
						href="https://chromewebstore.google.com/detail/mcpmate-server-import/jngogcgclencgillbmeeimkcjjnobidf"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
				<Li>
					Microsoft Edge Add-ons:{" "}
					<a
						href="https://microsoftedge.microsoft.com/addons/detail/mcpmate-server-import/nbpdfanhajcjghegoocfmjkpaklidckn"
						target="_blank"
						rel="noopener noreferrer"
					>
						MCPMate Server Import
					</a>
				</Li>
			</Ul>

			<H2>MCPMate とのつながり</H2>
			<H3>Web 上の発見からローカル管理へ</H3>
			<P>
				拡張は Web 側の入口です。インポート後のプレビュー、検証、保存、有効化、
				プロファイルやクライアント展開への接続は MCPMate デスクトップが担当します。
			</P>

			<Callout type="info" title="同じインポート経路">
				拡張での取得、ドラッグ＆ドロップ、貼り付け、Market からのインストールは
				Server Install Wizard に合流し、各サーバーを確認してからローカルワークスペースへ追加できます。
			</Callout>
		</DocLayout>
	);
}
