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
				MCPMateインスペクターは、MCPサーバーを監視およびデバッグするための強力なインターフェースを提供します。サーバーの動作に関するリアルタイムの洞察を得たり、ログを調べたり、問題を診断したりすることが、すべてMCPMateコンソール内から行えます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/inspector-tool-call-light.png"
				darkSrc="/screenshot/inspector-tool-call-dark.png"
				alt="サーバー機能に対するインスペクターのツール呼び出しパネル"
			/>

			<H2>機能</H2>
			<Ul>
				<Li>
					<strong>リアルタイム監視:</strong> 発生しているサーバーのアクティビティを監視
				</Li>
				<Li>
					<strong>ログビューア:</strong> サーバーログを閲覧および検索
				</Li>
				<Li>
					<strong>リクエスト/レスポンスインスペクター:</strong> MCPプロトコルメッセージを詳細に調査
				</Li>
				<Li>
					<strong>パフォーマンスメトリクス:</strong> 応答時間とリソース使用量を追跡
				</Li>
				<Li>
					<strong>エラー診断:</strong> 問題を迅速に特定してトラブルシューティング
				</Li>
			</Ul>

			<H2>ユースケース</H2>
			<Ul>
				<Li>サーバー構成の問題をデバッグする。</Li>
				<Li>ロールアウト期間中のランタイムの動作を監視する。</Li>
				<Li>すべてのクライアントに対して有効にする前に機能のペイロードを検証する。</Li>
				<Li>クライアントとサーバー間の通信の問題をトラブルシューティングする。</Li>
			</Ul>

			<H2>推奨されるワークフロー</H2>
			<Ul>
				<Li>サーバーの詳細から開始して、対象のサーバー/機能を特定します。</Li>
				<Li>インスペクターで制御された呼び出しを実行し、レスポンスのメタデータを取得します。</Li>
				<Li>タイムスタンプを監査ログと照らし合わせて、操作の完全なコンテキストを確認します。</Li>
			</Ul>
		</DocLayout>
	);
}