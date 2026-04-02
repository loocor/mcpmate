import DocLayout from "../../layout/DocLayout";
import { H3, P } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "マーケット導入フロー",
				description:
					"マーケットカードから MCPMate のインストールウィザードへ進む流れ",
			}}
		>
			<P>
				このページは、マーケットでレジストリカードを選んだ後の流れを説明します。MCPMate は手動インポートと同じインストールウィザードを開き、保存前にトランスポート、正規化済みマニフェスト、後続のプロファイル配置先を確認できるようにします。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="マーケット導入フロー：公式 MCP レジストリのカードを閲覧"
			/>

			<h2>このフローでできること</h2>
			<ul>
				<li>
					<strong>マーケットから導入への受け渡し:</strong> マーケットカードからガイド付きインストールへ進めます
				</li>
				<li>
					<strong>正規化プレビュー:</strong> 保存前にトランスポートとマニフェストの詳細を確認できます
				</li>
				<li>
					<strong>制御されたロールアウト:</strong> 先にサーバーを追加し、その後どのプロファイルに公開するかを決めます
				</li>
				<li>
					<strong>一貫した導入経路:</strong> マーケット導入とドラッグ＆ドロップ導入は同じ下流フローを共有します
				</li>
			</ul>

			<h2>全体フローの中での役割</h2>
			<P>
				マーケットは公式 MCP レジストリを閲覧する入口であり、このページのフローは実際にサーバー詳細を確認して Servers へ保存する段階を説明します。
			</P>

			<h2>メリット</h2>
			<P>
				レジストリ、スニペット、ローカル設定ファイルの間を行き来する代わりに、MCPMate は発見・確認・導入を 1 本のガイド付きフローにまとめます。
			</P>

			<H3>MCPサーバー追加ウィザード</H3>
			<P>
				レジストリカードからインストールすると、ガイド付きフローが開きます。トランスポートを設定し、正規化されたマニフェストを確認してサーバーを保存し、その後 Servers または Profiles ページで公開先プロファイルを決めます。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="コア構成フォームを備えたMCPサーバー追加ステッパー"
			/>
		</DocLayout>
	);
}
