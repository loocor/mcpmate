import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ProfilePresets() {
	return (
		<DocLayout
			meta={{
				title: "プロファイルプリセットテンプレート",
				description: "ガイド付きの開始点として組み込みのプロファイルテンプレートを使用する",
			}}
		>
			<P>
				<code>/profiles/presets/:presetId</code>の下にあるプリセットルートは、意思決定の補助として使用するのに最適です。これらは、チームが有効化、編集、およびクライアントに割り当てることができる実際のプロファイルを作成する前に、推奨されるバンドルを比較するのに役立ちます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/profiles-light.png"
				darkSrc="/screenshot/profiles-dark.png"
				alt="Profile presets list with template cards"
			/>

			<H2>プリセットを使用するタイミング</H2>
			<Ul>
				<Li>ライティングやコーディングなど、既知のワークフローの迅速な開始点が必要な場合。</Li>
				<Li>ロールアウト前にバンドルされたサーバーと能力の範囲を確認する必要がある場合。</Li>
				<Li>カスタムの選択を行う前に例を必要とするチームメイトをオンボーディングする場合。</Li>
			</Ul>

			<H2>プリセットからの作業方法</H2>
			<Ul>
				<Li>プリセットルートを開き、含まれているサーバーと能力の組み合わせを確認します。</Li>
				<Li>編集可能な所有権が必要な場合は、新しいプロファイルを作成するか、複製します。</Li>
				<Li>ワークスペースの下に新しいプロファイルが存在した後にのみ、詳細ページに移動します。</Li>
			</Ul>

			<Callout type="info" title="プリセットがライブプロファイルから分離されている理由">
				プリセットページは、日常の操作ではなく比較に最適化されています。これらを分離しておくことで、テンプレートがランタイムですでにアクティブであると誤って想定されるのを防ぎます。
			</Callout>

			<H2>よくある質問</H2>
			<Ul>
				<Li><strong>プリセットを直接編集できますか？</strong> プリセットはリファレンスとして扱ってください。継続的な変更にはプロファイルを作成または複製してください。</Li>
				<Li><strong>クライアントで何も変わらないのはなぜですか？</strong> 実際のプロファイルが有効化または選択されるまで、プリセットはクライアントに影響を与えません。</Li>
			</Ul>
		</DocLayout>
	);
}
