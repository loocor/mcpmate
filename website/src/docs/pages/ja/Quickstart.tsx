import { useMemo } from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";

const howToSteps = [
	{
		name: "デスクトップインストーラを入手",
		text: "まずは GitHub Releases から利用中のプラットフォーム向けインストーラを取得します。現時点では macOS 版が最も安定しており、Windows と Linux 版も入手できますが、まだ追い込み中の部分があります。",
	},
	{
		name: "MCPMate を起動",
		text: "アプリを開くと、同梱されたローカルプロキシが起動します。ダッシュボードに加えて、8080 番ポートの REST API と 8000 番ポートの MCP エンドポイントが利用可能になります。",
	},
	{
		name: "MCP サーバーを取り込む",
		text: "組み込みマーケットを使うか、JSON/TOML スニペットを取り込むか、既存クライアントから設定を引き継ぐことができます。",
	},
	{
		name: "プロファイルを整える",
		text: "Default プロファイルを開き、使いたいサーバーを追加したうえで、ツール、プロンプト、リソースをワークフローに合わせて切り替えます。",
	},
	{
		name: "クライアントへ展開",
		text: "クライアントページでエディタが検出されていることを確認し、Hosted、Unify、Transparent のいずれかを選んでからプロファイルを適用し、エディタ内で確認します。",
	},
];

export default function Quickstart() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "MCPMate のセットアップ方法",
				description:
					"GitHub Releases から MCPMate をすばやく立ち上げ、その後サーバー追加、プロファイル準備、クライアント展開まで進めるためのガイドです。",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout
			meta={{ title: "クイックスタート", description: "MCPMate のインストール、設定、実行" }}
		>
			<SchemaOrg schema={howTo} />
			<P>
				いま始めるなら、いちばん早い入口は GitHub の公式 Releases にあるデスクトップ版です。起動したら、そのままサーバー追加、プロファイル整理、エディタへの反映まで一気に進められます。
			</P>

			<H2>まずはデスクトップアプリから</H2>
			<Callout type="info" title="最短ルート">
				現時点でいちばん手早い方法は、GitHub Releases にある公式デスクトップインストーラを使うことです：
				https://github.com/loocor/mcpmate/releases
			</Callout>
			<Ul>
				<Li>Releases ページを開き、利用するプラットフォーム向けのインストーラを選びます。</Li>
				<Li>
					現在は macOS 版が最も安定しています。Windows と Linux 版も利用できますが、一部機能は未整備または不安定な場合があります。
				</Li>
				<Li>
					インストール後に MCPMate を起動します。デスクトップアプリにはダッシュボードとローカルプロキシが同梱されているため、ひとつの入口から始められます。
				</Li>
			</Ul>

			<H3>フルコントロールが必要ならソースからビルド</H3>
			<Ul>
				<Li>システムに Rust 1.75+ と Node.js 18+（または Bun）をインストールします。</Li>
				<Li>リポジトリをクローン：<code>git clone https://github.com/loocor/mcpmate.git</code></Li>
				<Li>バックエンドへ移動：<code>cd mcpmate/backend</code></Li>
				<Li>ビルドと実行：<code>cargo run --release</code></Li>
				<Li>プロキシは REST API を 8080 番ポート、MCP エンドポイントを 8000 番ポートで起動します。</Li>
			</Ul>

			<H3>ソースからダッシュボードを動かす</H3>
			<Ul>
				<Li>ダッシュボードへ移動：<code>cd mcpmate/board</code></Li>
				<Li>依存関係をインストール：<code>bun install</code></Li>
				<Li>開発サーバーを起動：<code>bun run dev</code></Li>
				<Li>http://localhost:5173 を開くと管理ダッシュボードに入れます。</Li>
			</Ul>

			<H2>Web とデスクトップ、どちらで使うか</H2>
			<P>
				同じ Board UI を 2 つのシェルで使えます。プロキシをどう運用したいかに合わせて選んでください。
			</P>
			<Ul>
				<Li>
					<strong>ブラウザ + 開発プロキシ</strong> &mdash; Vite が UI を配信し、API リクエストは <code>http://127.0.0.1:8080</code>（または上書きしたベース URL）へ向かいます。フロントエンドやバックエンドを別々に反復したいコントリビューター向けです。
				</Li>
				<Li>
					<strong>Tauri デスクトップ（macOS, Windows, Linux）</strong> &mdash; ダッシュボードとローカルプロキシをひとまとめにします。公式インストーラは GitHub Releases で配布されています。サイドバーの <strong>アカウント</strong> は、将来のクラウド連携に向けた macOS 上の任意 GitHub サインインを受け付けます。アプリ内の <strong>ドキュメント</strong> ボタンからは、表示中のページに対応する <code>mcp.umate.ai</code> のガイドを開けます。
				</Li>
			</Ul>

			<H2>Core と UI を分離して動かす</H2>
			<P>
				MCPMate を別マシンで動かしたい場合や、単に分離デプロイの方が扱いやすい場合は、コアサービスと UI シェルを切り離せます。
			</P>
			<Ul>
				<Li>対象ホストでバックエンドを起動し、使う予定の REST / MCP ポートを公開します。</Li>
				<Li>ローカル一体型バンドルの代わりに、ダッシュボードシェル（Web またはデスクトップ）をそのバックエンドへ接続します。</Li>
				<Li>設定 → システム で API / MCP ポートを確認し、エンドポイント変更時の再起動コマンドをコピーします。</Li>
			</Ul>

			<H2>MCP サーバーを取り込む</H2>
			<P>サーバー定義が今どこにあるかに合わせて、いちばん自然な導入経路を選んでください。</P>
			<H3>組み込みマーケットを使う</H3>
			<Ul>
				<Li>左サイドバーから <strong>マーケット</strong> を開きます。</Li>
				<Li>サーバーを検索または絞り込み、<strong>インストール</strong> を選んでワークスペースへ追加します。</Li>
			</Ul>
			<H3>外部バンドルをドラッグ＆ドロップ</H3>
			<Ul>
				<Li><strong>サーバー</strong> から <strong>追加</strong> を選び、MCP バンドルや JSON / TOML スニペットをウィンドウへドロップします。</Li>
				<Li>プレビューを確認してからインポートを確定し、サーバーエントリを作成します。</Li>
			</Ul>
			<H3>既存クライアントからインポート</H3>
			<Ul>
				<Li><strong>クライアント</strong> を開き、検出済みクライアントを選びます。</Li>
				<Li><strong>クライアントからインポート</strong> を使って、既存の MCP 設定を MCPMate に取り込みます。</Li>
			</Ul>

			<H2>実際の作業に合わせてプロファイルを整える</H2>
			<P>
				プロファイルは、クライアントに公開するサーバーや能力を決めます。MCPMate には <strong>Default</strong> プロファイルが同梱されており、用途ごとにさらに増やせます。
			</P>
			<Ul>
				<Li><strong>プロファイル</strong> に移動し、<strong>Default</strong> プロファイルを開きます。</Li>
				<Li>インストールしたサーバーを追加し、必要なツール、プロンプト、リソースを有効または無効にします。</Li>
				<Li><strong>新規プロファイル</strong> を使って、執筆用やデータ探索用などの追加プリセットを作成し、有効化する能力を調整します。</Li>
			</Ul>

			<H2>クライアントへプロファイルを反映する</H2>
			<Ul>
				<Li><strong>クライアント</strong> で、使用中のエディタが <strong>検出済み</strong> として表示されていることを確認します。出てこない場合は、クライアントの再インストールやパス確認を行ってください。</Li>
				<Li>そのクライアントに MCPMate から MCP 設定を書き戻したい場合は、New / Edit ドロワー内で、実在して書き込み可能なローカル設定ファイルを指しているか先に確認してください。MCPMate は書き込み先として使う前にそのパスを検証します。</Li>
				<Li><strong>Hosted</strong> モードではダッシュボード管理のプロファイル切り替えが使えます。セッション内の組み込み制御を優先したい場合は <strong>Unify</strong> を選びます。<strong>Transparent</strong> モードは設定ファイルを書き込むだけで、プロファイルをその場で切り替えることはできません。</Li>
				<Li>準備したプロファイルを選択して適用し、エディタで MCP コマンドを実行してツールが見えることを確認します。</Li>
			</Ul>

			<H2>ランタイムで詰まったら</H2>
			<Ul>
				<Li>サーバーの起動に失敗したり、ツールがエラーを返したりする場合は、<strong>ランタイム</strong> ページを開きます。</Li>
				<Li><strong>インストール / 修復</strong> を使って uv や Bun など必要なランタイムを整え、古い状態が疑わしい場合は同じページからキャッシュをクリアしてください。</Li>
			</Ul>

			<H2>監査ログで変更を追う</H2>
			<Ul>
				<Li><strong>監査ログ</strong> ページを開くと、プロファイル、クライアント、サーバーの操作を確認できます。</Li>
				<Li>アクション種別や時間帯で絞り込めば、いつ何が変わったかを追いやすくなります。</Li>
			</Ul>

			<H2>最新状態を保ち、貢献する</H2>
			<P>
				デスクトップ版を使う場合は、最新インストーラとリリースノートの確認先として GitHub Releases を使ってください。ソースから MCPMate を動かしている場合は、GitHub から最新変更を pull して再ビルドします。問題の報告や改善提案は、Issue や Pull Request で歓迎しています。
			</P>
		</DocLayout>
	);
}
