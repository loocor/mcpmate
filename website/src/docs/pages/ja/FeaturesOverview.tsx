import { P } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

export default function FeaturesOverview() {
	return (
		<DocLayout
			meta={{
				title: "機能の概要",
				description: "MCPMateの強力な機能を探索する",
			}}
		>
			<P>
				MCPMateは、MCPサーバーでの作業をより簡単に、より効率的に、そしてより強力にするために設計された包括的な機能セットを提供します。
			</P>

			<h2>コア機能</h2>
			<P>
				私たちの機能セットは、一元化された構成とリソースの最適化から、高度なツールとシームレスな統合にまで及びます。各機能は、ユーザーエクスペリエンスと開発者の生産性を念頭に置いて設計されています。
			</P>

			<h3>設定と管理</h3>
			<ul>
				<li>
					<strong>一元化された設定:</strong> 一度設定すれば、すべてのクライアントでどこでも使用可能
				</li>
				<li>
					<strong>シームレスなコンテキストの切り替え:</strong> 異なる作業シナリオ間を瞬時に切り替え
				</li>
				<li>
					<strong>きめ細かいコントロール:</strong> 正確なトグルですべての機能を微調整
				</li>
				<li>
					<strong>コアサーバー + UIの分離:</strong> バックエンドのコアサービスを独立して実行し、必要に応じてWeb/デスクトップのUIシェルを接続
				</li>
			</ul>

			<h3>パフォーマンスと最適化</h3>
			<ul>
				<li>
					<strong>リソースの最適化:</strong> パフォーマンス向上のためのインテリジェントなサーバーリソース管理
				</li>
				<li>
					<strong>プロトコルブリッジ:</strong> stdioベースのクライアントを無変更でStreamable HTTPサービスに接続
				</li>
			</ul>

			<h3>開発者ツール</h3>
			<ul>
				<li>
					<strong>インスペクター:</strong> サーバーの状態、ログ、診断に関する深い洞察
				</li>
				<li>
					<strong>自動検出とインポート:</strong> 既存の構成を自動的に検出してインポート
				</li>
				<li>
					<strong>ユニバーサルインポート (Uni-Import):</strong> ドラッグアンドドロップまたはペーストによる簡単な構成
				</li>
				<li>
					<strong>監査ログ:</strong> フィルタリング可能でカーソルベースのページネーション付きの履歴で、管理アクションとMCPアクティビティを追跡
				</li>
			</ul>

			<h3>エコシステム</h3>
			<ul>
				<li>
					<strong>インラインマーケットプレイス:</strong> 組み込みの公式MCPレジストリ — アプリから離れることなくサーバーを検索してインストール
				</li>
			</ul>

			<P>
				以下のセクションを通じて各機能を詳細に探索し、MCPMateがMCPワークフローをどのように強化できるかを学んでください。
			</P>
		</DocLayout>
	);
}