import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

export default function Inspector() {
	return (
		<DocLayout
			meta={{
				title: "インスペクター",
				description:
					"コンソールから離れることなく、サーバーの状態、ログ、診断に関する深い洞察を得る",
			}}
		>
			<P>
				MCPMate Inspector は MCP server のための live capability workbench です。Native と proxy の動作を比較し、tool / prompt request を制御して実行し、resource を読み取り、console を離れずに response や event の証拠を取得できます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="サーバー機能に対するインスペクターのツール呼び出しパネル"
			/>

			<H2>機能</H2>
			<Ul>
				<Li>
					<strong>Native と proxy channel:</strong> server の raw behavior と MCPMate-managed exposure を比較
				</Li>
				<Li>
					<strong>Schema-aware input:</strong> capability metadata から form を生成し、必要に応じて raw JSON に切り替え
				</Li>
				<Li>
					<strong>Response と event review:</strong> final output、progress、log、error、cancel state を分けて確認
				</Li>
				<Li>
					<strong>Capability read:</strong> tool、prompt、resource、resource template を同じ drawer workflow で検証
				</Li>
				<Li>
					<strong>エラー診断:</strong> 問題を迅速に特定してトラブルシューティング
				</Li>
			</Ul>

			<H2>ユースケース</H2>
			<Ul>
				<Li>サーバー構成の問題をデバッグする。</Li>
				<Li>server の raw output と profile-scoped proxy output を比較する。</Li>
				<Li>すべてのクライアントに対して有効にする前に機能のペイロードを検証する。</Li>
				<Li>クライアントとサーバー間の通信の問題をトラブルシューティングする。</Li>
			</Ul>

			<H2>推奨されるワークフロー</H2>
			<Ul>
				<Li>サーバーの詳細から開始して、対象のサーバー/機能を特定します。</Li>
				<Li>Inspector で制御された呼び出しを実行し、response と event output を分けて確認します。</Li>
				<Li>タイムスタンプを監査ログと照らし合わせて、操作の完全なコンテキストを確認します。</Li>
			</Ul>
		</DocLayout>
	);
}
