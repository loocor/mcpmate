import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Tools() {
	return (
		<DocLayout
			meta={{ title: "ツール", description: "ダッシュボードでのMCPツールの使用と管理" }}
		>
			<P>
				MCP<strong>ツール</strong>は、各MCPサーバーによってアドバタイズされる呼び出し可能な機能です。ダッシュボードのUIでは、これらはトップレベルのサイドバーセクションではありません。信頼するサーバーに可視性が結びついたままになるように、<strong>プロファイル</strong>と<strong>サーバー</strong>の内部で操作します。
			</P>

			<Callout type="info" title="ダッシュボードのどこをクリックするか">
				<code>/servers/:serverId</code> でサーバーを開き、<strong>機能</strong>エリアと<strong>ツール</strong>タブを使用して、名前、説明、および有効化状態をリストアップします。プロファイルレベルでは、同じツールキーがトグルと共に表示されるため、各クライアントが必要とするものだけを公開できます。
			</Callout>

			<H2>有効化レイヤー</H2>
			<Ul>
				<Li>
					<strong>サーバー</strong> &mdash; サーバーをオフにすると、再び有効になるまで、すべてのプロファイルからそのツールが削除されます。
				</Li>
				<Li>
					<strong>プロファイル</strong> &mdash; アクティブなプロファイル内のツールごとのスイッチにより、サーバーをアンインストールすることなく、マージされたサーフェスを絞り込むことができます。
				</Li>
				<Li>
					<strong>クライアント</strong> &mdash; 管理対象クライアントは、アクティブなプロファイルからマージされたセットを受け取ります。透過的モードは、ディスクに書き込んだもののみを反映します。
				</Li>
			</Ul>

			<H2>発見とデバッグ</H2>
			<H3>機能キャッシュ</H3>
			<P>
				ツールメタデータはパフォーマンスのためにキャッシュされます。サーバーが更新されたマニフェストを出荷した場合は、リストがインスペクターCLIに表示されるものと一致するように、<strong>ランタイム</strong>ページから機能キャッシュをリセットするか、プロキシを再起動します。
			</P>

			<H3>生のJSON</H3>
			<P>
				プロキシの応答とダッシュボードがレンダリングしたものを比較する必要がある場合は、設定 → 開発者の下にある<strong>生の機能JSONを表示する</strong>を有効にします。
			</P>

			<P>
				UIフローのステップバイステップについては、このドキュメントの<strong>サーバー</strong>と<strong>プロファイル</strong>のガイドをお読みください。これらは、ツールが編集されるダッシュボードのルートを反映しています。
			</P>
		</DocLayout>
	);
}
