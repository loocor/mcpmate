import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ServerDetailOverview() {
	return (
		<DocLayout
			meta={{
				title: "サーバー詳細概要",
				description: "サーバーの健全性、インスタンス状態、およびライフサイクルアクションを検査する",
			}}
		>
			<P>
				<code>/servers/:serverId</code>の参照ビューは、サーバーがローテーションを維持するのに十分健全であるかどうかを決定する場所です。状態、トランスポート、インスタンス情報、および有効化、無効化、編集、削除などのライフサイクルアクションを組み合わせています。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/server-detail-light.png"
				darkSrc="/screenshot/server-detail-dark.png"
				alt="Server detail overview"
			/>

			<H2>最初に確認すること</H2>
			<Ul>
				<Li>接続状態と、状態が過渡的であるか安定しているか。</Li>
				<Li>特に同じサーバーが複数のトランスポートを公開している場合のインスタンス数。</Li>
				<Li>編集または再起動が、すでにそれに依存しているクライアントに影響を与えるかどうか。</Li>
			</Ul>

			<H3>概要が能力タブの前にある理由</H3>
			<P>
				サーバー自体が健全でない場合、能力のリストは二次的な症状です。最初にライフサイクルを安定させ、その後能力のレビューやデバッグモードに移行します。
			</P>

			<Callout type="warning" title="更新は有効化と同じではありません">
				能力を更新するとメタデータが再取得されます。サーバーを有効または無効にすると、ランタイムの可用性が変わります。実際に解決しようとしている問題に対して適切なアクションを使用してください。
			</Callout>
		</DocLayout>
	);
}
