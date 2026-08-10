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

			<H2>設定の書き込み方法を選ぶ</H2>
			<P>
				クライアントの設定書き戻し方法は、クライアントごとの個別設定がない限り、<strong>設定 → クライアント管理</strong>の<strong>設定書き戻しの既定値</strong>を使います。<strong>自動</strong>は各クライアントが推奨する書き戻し動作を使用し、クライアントごとの設定が優先されます。<strong>マージ</strong>はクライアント設定内の関連しない MCP エントリを保持し、<strong>置換</strong>は MCPMate 管理のサーバー一覧を正とします。
			</P>
			<P>
				承認済みのクライアントに検証済みの書き込み可能な設定先がある場合、個別設定を保存すると有効な設定が直ちに再適用されます。設定の既定値を変更した場合は、その後のクライアント設定の適用または再適用で使われ、既存のクライアント設定ファイルを直ちに書き換えることはありません。
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
