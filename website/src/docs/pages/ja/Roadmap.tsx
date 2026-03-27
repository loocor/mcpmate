import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "OAuth 統合",
		description:
			"カスタムの接続作業なしで、外部トークンの受け入れ、スコープのマッピング、セッションの更新を行う予定です。",
	},
	{
		title: "クライアント固有のセーフガード",
		description:
			"各クライアントが必要なものだけを参照できるように、詳細なツールの可視性とセッションの分離を洗練させています。",
	},
	{
		title: "クロスプラットフォームのパッケージング",
		description:
			"自動更新とシステムサービスをサポートする、macOS、Windows、およびLinuxのインストーラーが準備中です。",
	},
	{
		title: "設定履歴",
		description:
			"現在のバックアップシステムを基盤として、プレビュー、差分、およびロールバックのオプションを追加し、チームが復元する前に確認できるようにする予定です。",
	},
	{
		title: "スマートなプロファイルの提案",
		description:
			"手動での切り替えなしに、自然言語のリクエストをすぐに使えるツールバンドルに変換する推奨事項を改善しています。",
	},
];

const onTheHorizon = [
	{
		title: "組み込みサービス",
		description:
			"ダッシュボードを離れることなく日常のメンテナンスを合理化する、より充実したインプレースのMCP管理サービス。",
	},
	{
		title: "プロファイルの共有",
		description:
			"厳選されたプロファイルバンドルを公開およびインポートし、チームがワンクリックで実績のあるツールセットを再利用できるようにします。",
	},
	{
		title: "コスト センター",
		description:
			"MCPサーバーごとのトークン消費を追跡および照合し、財務と運用に使用状況の明確なビューを提供します。",
	},
	{
		title: "アカウント レイヤー",
		description:
			"環境間で構成を揃えておくための、軽量なクラウド同期およびホスト型オプション。",
	},
	{
		title: "マスター・フォロワー モード",
		description:
			"フォロワーノードを指定してプライマリインスタンスをミラーリングし、大規模なチーム内での調整されたロールアウトを可能にします。",
	},
	{
		title: "サンドボックス モード",
		description:
			"高リスクのツールを安全に実行するための、レート制限と機能の許可リストを備えた分離された環境。",
	},
];

const Roadmap = () => {
	const meta = {
		title: "ロードマップ",
		description: "今後のMCPMateエクスペリエンスのスナップショット。",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>進行中</h2>
				<p>
					これらのイニシアチブは現在積極的に形成されています。実際のワークフローでエクスペリエンスを検証する際に、早期プレビューとアルファアクセスを期待してください。
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>最近提供された機能</h2>
				<ul className="space-y-2">
					<li>
						コアサーバーとUIが分離された操作モードが利用可能になりました。これにより、コントロールプレーンのバックエンドをウェブ/デスクトップシェルから独立して実行できます。
					</li>
					<li>
						大規模な操作タイムライン向けのフィルタリングとカーソルページネーションを備えた監査ログが稼働しました。
					</li>
				</ul>

				<h2>今後予定されているもの</h2>
				<p>
					以下は、長期的な要望リストです。順序を確認するためにフィードバックをお待ちしておりますので、気になるものがあればお気軽にお問い合わせください。
				</p>
				<ul className="space-y-2">
					{onTheHorizon.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>最新情報を入手する</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						リリースノートとコミュニティニュースレターを通じて、マイルストーンと早期アクセスのサインアップを共有しています。特定の機能を試験的に導入したい場合は、購読するかご連絡ください。
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
