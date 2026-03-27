import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ClientDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "クライアント詳細概要",
				description: "設定を適用する前に、クライアントの状態、統合の準備状況、および現在のサーバーの露出を確認する",
			}}
		>
			<P>
				<code>/clients/:identifier</code>の概要タブでは、クライアントが検出され、管理されており、MCPMate制御の設定を受信する準備ができているかどうかを確認できます。これは、設定を適用する前に、トランスポートのサポート、製品ドキュメントのリンク、および現在のサーバーセットを検査するための適切な場所です。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="Client detail overview"
			/>

			<H2>このページの目的</H2>
			<Ul>
				<Li>クライアントのアイデンティティと、MCPMateが現在それを管理できるかどうかを確認します。</Li>
				<Li>クライアントの接続方法を変更する前に、サポートされているトランスポートを確認します。</Li>
				<Li>クライアントの有効な設定から抽出された現在のサーバーを確認します。</Li>
			</Ul>

			<H3>価値の高いアクション</H3>
			<P>
				クライアントをインストールまたは移動した後は、<strong>更新</strong>を使用して、MCPMateが検出状態を再スキャンできるようにします。MCPMateがクライアント設定のライフサイクルを所有する準備ができたら、管理トグルを使用します。
			</P>

			<Callout type="info" title="概要のドキュメントリンクは製品固有です">
				ここに表示されるドキュメントおよびホームページのリンクは、クライアントのメタデータ自体から取得されます。これらは、MCPMate独自のガイダンスに加えて、ベンダー固有のセットアップの注意事項が必要な場合に役立ちます。
			</Callout>

			<H2>よくある質問</H2>
			<Ul>
				<Li><strong>クライアントが未検出と表示されるのはなぜですか？</strong> アプリが予期されたパスにインストールされていないか、バックエンドがスキャンするための権限を持っていない可能性があります。</Li>
				<Li><strong>現在のサーバーがアクティブなプロファイルと異なるのはなぜですか？</strong> 現在のサーバーは、望ましいターゲット状態だけでなく、クライアントの現在の設定を反映しています。</Li>
			</Ul>
		</DocLayout>
	);
}
