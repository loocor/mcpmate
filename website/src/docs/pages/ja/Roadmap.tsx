import DocLayout from "../../layout/DocLayout";

const inProgress = [
	{
		title: "基盤強化後の使いやすさ改善",
		description:
			"0.2.3 の安定性強化を土台に、次のリリースでは Onboarding、クライアント設定、マーケットインストール、サポート向けフィードバックの摩擦を減らしていきます。",
	},
	{
		title: "デスクトップ配布パイプライン",
		description:
			"GitHub Releases を起点にした配布経路について、自動更新の振る舞い、プレリリース処理、macOS / Windows / Linux 間の梱包整合性を引き続き磨いています。",
	},
	{
		title: "プラットフォーム成熟度の底上げ",
		description:
			"macOS、Windows、Linux のデスクトップビルドはいずれも Beta として扱いながら、インストーラの挙動、ランタイム検出、デスクトップ体験の一貫性を引き続き高めています。",
	},
	{
		title: "クライアント統制と認証情報の安全性",
		description:
			"検出済みクライアントへの展開、書き込み可能ターゲットの検証、適用 / クリーンアップ経路、機密 token の扱いを改善し、管理された変更をより信頼しやすくしています。",
	},
	{
		title: "ドキュメントと導線の同期",
		description:
			"website、クイックスタート、ブラウザー拡張のインストール導線、ダッシュボードの文言を実際に出荷済みの挙動へ合わせ続け、配布フローの変化で案内が古くならないようにしています。",
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
		title: "コンテナと分離デプロイの磨き込み",
		description:
			"Core Server と UI はすでに分離運用できます。今後はリモート、コンテナ、複数マシンでの運用をよりパッケージしやすく、説明しやすい形にしていきます。",
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
					0.2.3 の安定性強化を受けて、いま最もユーザー体験に近い作業は、使いやすさの改善、配布、プラットフォーム成熟度、クライアント展開の安全性、そして分かりやすい導線です。
				</p>
				<ul className="space-y-2">
					{inProgress.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>最近届けたもの</h2>
				<ul className="space-y-2">
					<li>
						Onboarding と新規クライアント設定は、バックエンドが管理する互換標準を使うようになり、より新しく適切なクライアント設定を受け取れるようになりました。
					</li>
					<li>
						自動更新の基盤が強化され、認可済みの Streamable HTTP サーバー向け OAuth token 更新にも対応しました。
					</li>
					<li>
						デスクトップ診断エクスポートにより、サポート調査が必要なときに整理されたローカルフィードバックを共有しやすくなりました。
					</li>
					<li>
						Inspector のライフサイクル管理と Registry インストール処理を強化し、繰り返し作業、分かりにくい状態表示、壊れたインストールドラフトを減らしました。
					</li>
					<li>
						ブラウザー拡張、Onboarding、website ドキュメントを更新し、インストールとアップグレードの流れを追いやすくしました。
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
