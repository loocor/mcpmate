import DocLayout from "../../layout/DocLayout";
import { H2, H3, Li, P, Ul } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function OnboardingJA() {
	return (
		<DocLayout
			meta={{
				title: "オンボーディング",
				description:
					"MCPMate のオンボーディングでクライアント検出、既存サーバーのインポート、Discovery プリセットからの開始を行います。",
			}}
		>
			<P>
				Onboarding は、初回の MCPMate セッションをローカル検出から使えるワークスペースへ導きます。
				互換クライアントを検出し、既存の MCP Server 設定を確認し、ローカル設定が空の場合は
				MCPMate Public Discovery からスターターエントリを読み込めます。
			</P>

			<H2>フロー概要</H2>
			<Ul>
				<Li>インストール済み AI クライアントと MCP 設定先を検出します。</Li>
				<Li>既存のローカルクライアント設定ファイルにあるサーバーを確認します。</Li>
				<Li>Public Discovery からスターターのクライアントプリセットとサーバーを選びます。</Li>
				<Li>選択したサーバーを MCPMate にインポートし、スタータープロファイルへ配置します。</Li>
				<Li>Clients、Servers、Market、Profiles へ進んで詳細設定を続けます。</Li>
			</Ul>

			<H2>クライアント検出</H2>
			<P>
				クライアントステップはローカル検出と MCPMate Discovery プリセットを組み合わせます。
				検出されたアプリには MCPMate が読み書きできる設定先が表示されます。プリセットは、
				アプリがまだ検出されていない場合でも、対応クライアントのレコードを準備する助けになります。
			</P>

			<H2>サーバー選択</H2>
			<P>
				サーバーステップでは、ローカルクライアントファイルにあるエントリをインポートし、
				Public Discovery からのスターターサーバーも選択できます。選んだサーバーはインポートリクエストにまとめられ、
				MCPMate が設定を正規化してローカルのサーバーライブラリに保存します。
			</P>

			<H3>選択後の流れ</H3>
			<Ul>
				<Li>MCPMate は選択されたサーバー定義をローカルワークスペースに保存します。</Li>
				<Li>インポートされたサーバーは Servers ページで利用できます。</Li>
				<Li>Profiles はそれらのサーバーを使い、各クライアントに見せる能力を調整できます。</Li>
				<Li>クライアント設定は Hosted、Unify、Transparent の展開方式へ続きます。</Li>
			</Ul>

			<Callout type="info" title="Discovery によるスターターデータ">
				Public Discovery は Onboarding に一般的なクライアントとサーバーの起点を提供します。
				同じ Admin 管理の Discovery データはブラウザ拡張のカタログタブにも使われます。
			</Callout>
		</DocLayout>
	);
}
