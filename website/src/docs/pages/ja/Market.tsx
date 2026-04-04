import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function MarketJA() {
	return (
		<DocLayout
			meta={{
				title: "マーケット",
				description: "コミュニティ サーバーの閲覧と管理",
			}}
		>
			<P>
				マーケットは MCPMate を公式 MCP レジストリに接続します。ここでは一覧の参照、メタデータの確認、ノイズ項目の非表示化、候補サーバーのインストールウィザード送り込みを行えます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="MCP Market with server listings and search"
			/>

			<H2>レジストリとデータ</H2>
			<Ul>
				<Li>
					マーケットには公式 MCP レジストリが表示されます。検索（デバウンス入力付き）とソート（最近、アルファベット順）はキャッシュ済みページに対してクライアント側で動作し、必要に応じて追加ページを読み込みます。
				</Li>
				<Li>
					任意のウェブサイトからサーバースニペットをインポートするには、デスクトップアプリで <code>mcpmate://import/server</code> を開く <strong>MCPMate サーバー インポート</strong> Chrome拡張機能（<code>extension/chrome</code>）を使用します。
				</Li>
				<Li>
					Settings → <strong>MCP マーケット</strong> では、既定マーケットの選択、非表示項目の管理、ブラウザー拡張の入口確認を行えます。
				</Li>
			</Ul>

			<H2>マーケットからのインストール</H2>
			<H3>プレビュードロワー</H3>
			<P>
				サーバーカードを選択すると、プレビュードロワーが開きます。説明、機能の数、トランスポートタイプ、環境変数、およびバンドルされたアイコンが表示されます。セカンダリボタンを押すと、サーバーが事前入力された状態でユニインポートウィザードが起動するため、保存する前にエイリアスを微調整できます。
			</P>

			<H3>OAuth 対応の上流サーバー</H3>
			<P>
				OAuth が必要な上流の Streamable HTTP サーバーでは、インストールウィザードが認可メタデータを準備し、プロバイダーのログインポップアップを開きます。承認後は MCPMate がコールバックを受け取り、ポップアップを閉じて同じインストールフローを続行します。
			</P>

			<H3>アイテムを非表示またはブラックリストに登録</H3>
			<P>
				「非表示」アクションを使うと、エントリはローカルのマーケットブラックリストへ移動します。非表示のサーバーはグリッドから消えますが、あとで必要になれば「設定」→「MCP マーケット」から復元できます。
			</P>

			<H2>ブラックリスト</H2>
			<P>
				「設定」→「MCP マーケット」で非表示のレジストリエントリを管理します。エントリを復元すると、グリッドに戻ります。
			</P>

			<Callout type="info" title="サーバーページとの関係">
				すべてのインストールは、ドラッグアンドドロップインポートと同じ <strong>サーバー インストール ウィザード</strong> を経由します。マーケットから追加したものはすぐにサーバー一覧へ現れ、設定確認、グローバル有効化、適切なプロファイルへの追加へ進めます。
			</Callout>
		</DocLayout>
	);
}
