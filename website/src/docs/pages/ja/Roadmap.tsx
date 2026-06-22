import { Link } from "react-router-dom";

import DocLayout from "../../layout/DocLayout";

const currentFocus = [
	{
		title: "MCP Server の採用をより安全にする",
		description:
			"ユーザーが Server を見つけ、出どころを理解し、公開される能力を確認し、設定を信頼してよいか判断してからインポートできる状態を目指します。",
	},
	{
		title: "クライアント展開を制御下に置く",
		description:
			"MCPMate は、どのクライアントにどの Server、tool、resource、prompt を渡すかを明確にし、ローカル MCP の変更が複数の設定ファイルへ散らばらないようにします。",
	},
	{
		title: "セットアップを観測可能なワークフローにする",
		description:
			"変更の前後で、読みやすい source context、dry-run checks、credential readiness、runtime state、support-friendly diagnostics を確認できる状態にします。",
	},
];

const nextBets = [
	{
		title: "再利用できるチームワークフロー",
		description:
			"Profiles と capability sets を共有、レビュー、再利用しやすくし、チームが検証済みの MCP setup から始められるようにします。",
	},
	{
		title: "リモートと分離運用",
		description:
			"Core Server、dashboard、将来の remote entry points を、単一ローカルデスクトップを超える利用者にも分かりやすい運用モデルへ整理します。",
	},
	{
		title: "より強いガバナンス信号",
		description:
			"Logs、audit evidence、permission boundaries、高リスク tool controls により、何が変わり、誰または何が使え、いつ介入すべきかを把握しやすくします。",
	},
	{
		title: "より賢いワークフロー支援",
		description:
			"Inspector-driven checks、skill-like workflows、prompt や provider helpers は、説明可能で operator の制御下にある限り、手動セットアップを減らせます。",
	},
	{
		title: "利用量とコストの可視化",
		description:
			"より長期的には、サーバー単位の利用パターンや token cost の判断材料を見えるようにし、tool exposure を自信を持って調整できるようにします。",
	},
];

const shippedFoundation = [
	{
		title: "インポートと発見の基盤",
		description:
			"ブラウザー発見、GitHub MCP import、Cursor.directory handoff、Market README 表示、source metadata、multi-server import preview、dry-run validation が、最初の end-to-end adoption path になりました。",
	},
	{
		title: "Credential と OAuth の custody",
		description:
			"Secure Store、OAuth token custody、lifecycle views、degraded-state guidance、reconnect prompts、cleanup controls により、機密性の高い Server state を平文設定ファイルから外しました。",
	},
	{
		title: "管理されたクライアント設定",
		description:
			"Profiles、bulk include / exclude controls、backend-maintained compatibility standards、diagnostics export、改善された Inspector lifecycle handling により、MCP changes をレビューしやすく、サポートしやすくしました。",
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
				<h2>現在の重点</h2>
				<p>
					MCPMate は、MCP の採用を複数クライアント設定ファイルの手作業編集ではなく、管理されたワークフローに近づけます。利用可能な能力を見つけ、変更内容を検証し、適切な能力を適切なクライアントへ公開します。
				</p>
				<ul className="space-y-2">
					{currentFocus.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>次の方向性</h2>
				<p>
					ここにあるのは戦略的な方向であり、特定リリースの約束ではありません。実際の利用、サポート上のシグナル、展開上の制約を見ながら順序を決めます。
				</p>
				<ul className="space-y-2">
					{nextBets.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<h2>最近できた基盤</h2>
				<p>
					0.3.x はこのワークフローの基盤づくりに集中してきました。詳細なリリース記録は changelog に残し、このページでは product-level の building blocks だけを扱います。
				</p>
				<ul className="space-y-2">
					{shippedFoundation.map((item) => (
						<li key={item.title}>{`${item.title} ${item.description}`}</li>
					))}
				</ul>

				<div className="rounded-lg border border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20 p-4">
					<h4>いちばん新しい動きを追うには</h4>
					<p className="text-sm text-slate-600 dark:text-slate-300">
						最も新しい出荷済みの記録を見るには、サイト内の{" "}
						<Link
							to="/docs/ja/changelog"
							className="font-medium text-blue-700 underline underline-offset-2 dark:text-blue-300"
						>
							変更履歴
						</Link>
						を確認してください。そこにはすでに着地した内容が反映され、このページにはいま形を整えている方向性が反映されます。
					</p>
				</div>
			</div>
		</DocLayout>
	);
};

export default Roadmap;
