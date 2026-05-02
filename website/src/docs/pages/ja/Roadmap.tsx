import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "デスクトップ配布パイプライン",
		description:
			"GitHub Releases を起点にした配布経路について、自動更新の振る舞い、プレリリース処理、macOS / Windows / Linux 間の梱包整合性を引き続き磨いています。",
	},
	{
		title: "プラットフォーム成熟度の底上げ",
		description:
			"現時点で最も安定しているのは macOS 版です。次の重点は、Windows と Linux のインストーラ、ランタイム挙動、デスクトップ体験をその水準へ近づけることです。",
	},
	{
		title: "コンテナと分離デプロイ",
		description:
			"コンテナ向けに扱いやすいコア配布形態を強化しつつ、Core Server / UI 分離運用のリモート・複数マシン展開もさらに整えています。",
	},
	{
		title: "クライアント統制の磨き込み",
		description:
			"検出済みクライアントへの展開、書き込み可能ターゲットの検証、適用 / クリーンアップ経路を改善し、管理された変更をより信頼しやすくしています。",
	},
	{
		title: "ドキュメントと導線の同期",
		description:
			"website、クイックスタート、ダッシュボードの文言を実際に出荷済みの挙動へ合わせ続け、配布フローの変化で案内が古くならないようにしています。",
	},
];

const exploringNext = [
	{
		title: "内蔵自動更新の磨き込み",
		description:
			"最初の配布パイプラインが形になった今、次はデスクトップ更新をもっと自然で日常的な体験にする段階です。",
	},
	{
		title: "プロファイル共有",
		description:
			"チームが実績あるプロファイルの束を再利用できるようにし、毎回同じ能力セットを作り直さなくて済む形を目指しています。",
	},
	{
		title: "軽量なアカウント層",
		description:
			"任意のアカウント連携や軽量なクラウド同期は引き続き魅力的ですが、MCPMate の local-first な境界は明示的に保つ方針です。",
	},
	{
		title: "より安全なサンドボックス",
		description:
			"高リスクツールを公開する際に、より細かなガードレールや隔離手段を持たせる方向を検討しています。",
	},
	{
		title: "利用量とコストの可視化",
		description:
			"より長期的には、サーバー単位の利用パターンやトークンコストの判断材料を運用者へ分かりやすく届けたいと考えています。",
	},
];

const Roadmap = () => {
	const meta = {
		title: "ロードマップ",
		description: "MCPMate が次に重点を置いて改善していること。",
	};

	return (
		<DocLayout meta={meta}>
			<div className="space-y-6">
				<h2>進行中</h2>
				<p>
					いま最もユーザー体験に近い作業は、配布、プラットフォーム成熟度、クライアント展開の安全性、そして分かりやすい導線です。
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>最近届けたもの</h2>
				<ul className="space-y-2">
					<li>
						Streamable HTTP MCP サーバー向けの OAuth upstream 対応が入り、メタデータ発見、認可フロー、トークン更新まで扱えるようになりました。
					</li>
					<li>
						監査ログはフィルタとカーソルページネーション付きで利用可能になり、Core Server と UI の分離運用も行えるようになりました。
					</li>
					<li>
						マーケットとインポートの流れには、より豊富なレジストリ情報、詳しいプレビュー、ブラウザ補助によるスニペット取得が加わりました。
					</li>
					<li>
						デスクトップ配布は GitHub Releases 駆動の経路を持ち、梱包整理とコンテナ公開のカバレッジも整ってきました。
					</li>
				</ul>

				<h2>次の候補として見ているもの</h2>
				<p>
					ここにあるのは有力候補であって、固定された約束ではありません。実際のフィードバックと展開上の制約を見ながら順序を決めていきます。
				</p>
				<ul className="space-y-2">
					{exploringNext.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>いちばん新しい動きを追うには</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						最も新鮮なシグナルを見たい場合は、まず GitHub Releases と changelog を確認してください。そこにはすでに着地した内容が反映され、このページにはいま形を整えている方向性が反映されます。
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
