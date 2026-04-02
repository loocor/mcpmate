import { P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function ProtocolBridging() {
	return (
		<DocLayout
			meta={{
				title: "プロトコルブリッジ",
				description:
					"stdioベースのクライアントを無変更でStreamable HTTPサービスに接続する",
			}}
		>
			<P>
				MCPMateのプロトコルブリッジ機能により、stdioベースのMCPクライアントのコードを一切変更することなく、Streamable HTTPベースのサービスに接続できます。これにより、MCPサーバーのデプロイと使用方法の柔軟性が大幅に向上します。
			</P>

			<h2>仕組み</h2>
			<P>
				MCPMateは、異なるトランスポートプロトコル間の透過的なブリッジとして機能します。stdioベースのクライアントがMCPMateに接続すると、ネイティブのstdioサーバーであるかのようにStreamable HTTPサーバーと通信できます。プロトコル変換はバックグラウンドでシームレスに行われます。従来のSSEスタイルの構成もインポートの境界で受け入れられ、Streamable HTTPに正規化されます。
			</P>

			<h2>ユースケース</h2>
			<ul>
				<li>
					<strong>リモートサーバーアクセス:</strong> ローカルクライアントをクラウドでホストされているMCPサーバーに接続
				</li>
				<li>
					<strong>ハイブリッドデプロイ:</strong> 同じワークフロー内でローカルサーバーとリモートサーバーを混在させる
				</li>
				<li>
					<strong>レガシー構成の互換性:</strong> 過去のSSEスタイルの構成スニペットをインポートし、正規化されたトランスポートモデルを通じて実行
				</li>
				<li>
					<strong>サービス移行:</strong> クライアントを中断することなく、stdioからStreamable HTTPへ段階的に移行
				</li>
			</ul>

			<h2>メリット</h2>
			<ul>
				<li>クライアントコードの変更は不要</li>
				<li>すべてのトランスポートタイプに対応する統一インターフェース</li>
				<li>柔軟なデプロイメントアーキテクチャを実現</li>
				<li>MCPインフラストラクチャの将来性を確保</li>
			</ul>

			<h2>実用的なデプロイメントパターン</h2>
			<ul>
				<li>安定したホストでMCPMateのコアサービスを実行します。</li>
				<li>stdio のみのクライアントをブリッジまたは Hosted クライアントモードで接続します。</li>
				<li>最新の統合のためにStreamable HTTPエンドポイントを公開します。</li>
			</ul>
		</DocLayout>
	);
}
