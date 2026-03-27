import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfileDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "プロファイル詳細概要",
				description: "変更を加える前に、プロファイルの状態、有効化ルール、およびカウンターを読み取る",
			}}
		>
			<P>
				<code>/profiles/:profileId</code>の概要タブは、個々のサーバーや能力に触れる前に、プロファイルが何を制御しているかを理解するための最も安全な場所です。状態、タイプ、複数選択の動作、優先度、および更新、編集、デフォルト、有効化、無効化、削除などのクイックアクションを要約しています。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="Profile detail overview and counters"
			/>

			<H2>最初に確認すること</H2>
			<Ul>
				<Li><strong>状態</strong>は、プロファイルが現在マージされたランタイムに貢献しているかどうかを示します。</Li>
				<Li><strong>タイプ</strong>は、プロファイルが共有されているか、ホストアプリ固有であるか、またはその他の特別なワークフローバケットであるかを示します。</Li>
				<Li><strong>優先度</strong>は、複数のアクティブなプロファイルが重複しており、予測可能な解決が必要な場合に重要です。</Li>
			</Ul>

			<H2>カウンターが重要な理由</H2>
			<Ul>
				<Li>カウンターを使用すると、より深いタブを開く前に影響を見積もることができます。</Li>
				<Li>また、ロールアウト後にクライアントに表示される能力の増減の理由を説明するのにも役立ちます。</Li>
				<Li>カードをクリックすると、トラブルシューティング時に関連するタブに最もすばやく移動できます。</Li>
			</Ul>

			<H3>クイックアクション</H3>
			<P>
				更新は詳細データを再取得し、編集はプロファイルフォームを再度開きます。また、有効化または削除ボタンはランタイムが公開するものを変更します。完全なコンテキストで制御された変更が必要な場合は、概要からそれらを使用してください。
			</P>

			<Callout type="warning" title="デフォルトおよびアンカールールは意図的なものです">
				ベースラインの能力セットを保護しているため、一部のプロファイルは無効化または削除できません。ボタンが利用できない場合は、UIエラーではなくポリシーとして扱い、最初にプロファイルの役割を確認してください。
			</Callout>

			<H2>よくある質問</H2>
			<Ul>
				<Li><strong>このプロファイルを無効にできないのはなぜですか？</strong> デフォルトアンカーの保護により、必要なフォールバックカバレッジの削除が防止されます。</Li>
				<Li><strong>カウントが1つのタブより大きく見えるのはなぜですか？</strong> 概要は、現在表示されているカテゴリだけでなく、プロファイル全体を要約しています。</Li>
			</Ul>
		</DocLayout>
	);
}
