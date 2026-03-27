import React from "react";
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
				マーケットは、MCPMateをMCPサーバーの厳選されたレジストリに接続します。ここから、公式リストの閲覧、独自のポータルの追加、メタデータのプレビュー、候補を直接インストールウィザードに送ることができます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="MCP Market with server listings and search"
			/>

			<H2>レジストリとデータ</H2>
			<Ul>
				<Li>
					マーケットには、公式のMCPMateレジストリがリストされます。検索（デバウンス入力付き）とソート（最近、アルファベット順）はキャッシュされたページに対してクライアントサイドで実行され、アプリはオンデマンドで追加のページをストリーミングします。
				</Li>
				<Li>
					任意のウェブサイトからサーバースニペットをインポートするには、デスクトップアプリで <code>mcpmate://import/server</code> を開く <strong>MCPMate サーバー インポート</strong> Chrome拡張機能（<code>extension/chrome</code>）を使用します。
				</Li>
				<Li>
					リモートコネクタは「リモート」オプションの下に表示されます。これらは、ワンクリックでインポートできる事前定義されたエンドポイント（Gitリポジトリ、zipバンドルなど）を表します。
				</Li>
			</Ul>

			<H2>マーケットからのインストール</H2>
			<H3>プレビュードロワー</H3>
			<P>
				サーバーカードを選択すると、プレビュードロワーが開きます。説明、機能の数、トランスポートタイプ、環境変数、およびバンドルされたアイコンが表示されます。セカンダリボタンを押すと、サーバーが事前入力された状態でユニインポートウィザードが起動するため、保存する前にエイリアスを微調整できます。
			</P>

			<H3>アイテムを非表示またはブラックリストに登録</H3>
			<P>
				「非表示」アクションを使用して、エントリをローカルのマーケットブラックリストに移動します。非表示のサーバーはグリッドから消えますが、後で必要になった場合は「設定」→「マーケットプレイス」から復元できます。
			</P>

			<H2>ブラックリスト</H2>
			<P>
				「設定」→「MCP マーケット」で非表示のレジストリエントリを管理します。エントリを復元すると、グリッドに戻ります。
			</P>

			<Callout type="info" title="サーバーページとの関係">
				すべてのインストールは、ドラッグアンドドロップインポートに使用されるのと同じ <strong>サーバー インストール ウィザード</strong> を経由します。マーケットから追加したものはすべて即座にサーバーリストに表示され、プロファイルごとに有効にして接続を監視できます。
			</Callout>
		</DocLayout>
	);
}
