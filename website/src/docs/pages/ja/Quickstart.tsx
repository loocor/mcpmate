import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function Quickstart() {
	return (
		<DocLayout
			meta={{ title: "クイックスタート", description: "MCPMateのビルド、設定、実行" }}
		>
			<P>
				このガイドでは、ソースからのMCPMateのビルド、サーバーの追加、プロファイルの準備、およびMCPクライアント内での適用方法について説明します。
			</P>

			<H2>ソースからのビルド</H2>
			<Callout type="info" title="オープンソース">
				MCPMateはMITライセンスのオープンソースです。リポジトリをクローン：github.com/loocor/mcpmate
			</Callout>
			<Ul>
				<Li>システムにRust 1.75+ と Node.js 18+ (または Bun) をインストールします。</Li>
				<Li>リポジトリをクローン：<code>git clone https://github.com/loocor/mcpmate.git</code></Li>
				<Li>バックエンドに移動：<code>cd mcpmate/backend</code></Li>
				<Li>ビルドと実行：<code>cargo run --release</code></Li>
				<Li>プロキシは、REST APIをポート8080、MCPエンドポイントをポート8000で起動します。</Li>
			</Ul>

			<H3>ダッシュボードの実行</H3>
			<Ul>
				<Li>ダッシュボードに移動：<code>cd mcpmate/board</code></Li>
				<Li>依存関係をインストール：<code>bun install</code></Li>
				<Li>開発サーバーを起動：<code>bun run dev</code></Li>
				<Li>管理ダッシュボードにアクセスするには、http://localhost:5173 を開きます。</Li>
			</Ul>

			<H2>Webダッシュボード vs デスクトップアプリ</H2>
			<P>
				同じダッシュボードUIが2つの環境で提供されます。プロキシの実行方法に合わせて選択してください。
			</P>
			<Ul>
				<Li>
					<strong>ブラウザ + 開発プロキシ</strong> &mdash; ViteがUIを提供し、APIリクエストは<code>http://127.0.0.1:8080</code>（または上書きされたベースURL）へ送信されます。フロントエンドやバックエンドを個別に反復開発するコントリビューターに最適です。
				</Li>
				<Li>
					<strong>Tauriデスクトップ (macOS, Windows, Linux)</strong> &mdash; ダッシュボードとローカルプロキシをバンドルします。サイドバーの<strong>アカウント</strong>エントリは、今後のクラウド機能向けにmacOSでのオプションのGitHubサインインをサポートします。アプリ内の<strong>ドキュメント</strong>ボタンから、表示中のページに関する<code>mcp.umate.ai</code>のガイドを開くことができます。
				</Li>
			</Ul>

			<H2>分離モードでの実行 (コアサーバー + UI)</H2>
			<P>
				リモートや複数マシンでの運用のため、MCPMateのコアサービスをUIシェルから切り離すことができます。
			</P>
			<Ul>
				<Li>
					ターゲットホストでバックエンドを実行し、使用予定のREST/MCPポートを公開します。
				</Li>
				<Li>
					ローカルのオールインワンバンドルを実行する代わりに、ダッシュボードシェル（Webまたはデスクトップ）をそのバックエンドに接続します。
				</Li>
				<Li>
					設定 → システムセクションを使用してAPI/MCPポートを確認し、エンドポイントが変更された場合の再起動コマンドをコピーします。
				</Li>
			</Ul>

			<H2>MCPサーバーのインストール</H2>
			<P>使用したいサービスに合わせてアプローチを選択してください。</P>
			<H3>組み込みマーケットプレイスの閲覧</H3>
			<Ul>
				<Li>左サイドバーから<strong>マーケット</strong>を開きます。</Li>
				<Li>サーバーを検索またはフィルタリングし、<strong>インストール</strong>を選択してワークスペースに追加します。</Li>
			</Ul>
			<H3>外部バンドルのドラッグ＆ドロップ</H3>
			<Ul>
				<Li><strong>サーバー</strong>から<strong>追加</strong>を選択し、MCPバンドルやJSON/TOMLスニペットをウィンドウにドロップします。</Li>
				<Li>プレビューを確認し、インポートを確定してサーバーエントリを作成します。</Li>
			</Ul>
			<H3>既存クライアントからのサーバーインポート</H3>
			<Ul>
				<Li><strong>クライアント</strong>を開き、検出されたクライアントを選択します。</Li>
				<Li><strong>クライアントからインポート</strong>アクションを使用して、既存のMCP構成をMCPMateに取り込みます。</Li>
			</Ul>

			<H2>プロファイルの整理</H2>
			<P>
				プロファイルは、クライアントに公開するサーバーや機能を決定します。MCPMateには<strong> Default</strong>プロファイルが同梱されており、特定のシナリオ用にさらに追加作成できます。
			</P>
			<Ul>
				<Li><strong>プロファイル</strong>に移動し、<strong>Default</strong>プロファイルを開きます。</Li>
				<Li>インストールしたサーバーを追加し、必要なツール、プロンプト、リソースを有効または無効にします。</Li>
				<Li>
					<strong>新規プロファイル</strong>を使用して追加のプリセット（執筆用やデータ探索用など）を作成し、有効にする機能を調整します。
				</Li>
			</Ul>

			<H2>クライアント内でのプロファイル適用</H2>
			<Ul>
				<Li>
					<strong>クライアント</strong>で、使用しているエディタが<strong>検出済み</strong>として表示されていることを確認します。表示されていない場合は、クライアントを再インストールするかパスを確認してください。
				</Li>
				<Li>
					MCPMateからのインプレースプロファイル切り替えを有効にするには、クライアントを<strong>Hosted</strong>モードに設定します。Transparentモードでは構成ファイルを書き込むのみで、プロファイルをリアルタイムで切り替えることはできません。
				</Li>
				<Li>準備したプロファイルを選択して適用します。エディタを起動し、MCPコマンドをトリガーしてツールが表示されることを確認します。</Li>
			</Ul>

			<H2>ランタイムのトラブルシューティング</H2>
			<Ul>
				<Li>
					サーバーの起動に失敗したり、ツールがエラーを返す場合は、<strong>ランタイム</strong>ページを開き、<strong>インストール / 修復</strong>を使用して必要なランタイム（uv, Bun）をプロビジョニングします。
				</Li>
				<Li>古いデータが疑われる場合は、同じページからキャッシュをクリアしてください。</Li>
			</Ul>

			<H2>監査ログの確認</H2>
			<Ul>
				<Li>
					<strong>監査ログ</strong>ページを開いて、プロファイル/クライアント/サーバーの操作やセキュリティ関連のアクションを確認します。
				</Li>
				<Li>
					アクションの種類と期間でフィルタリングし、長時間稼働する環境向けにカーソルベースのページネーションを使用して読み込みます。
				</Li>
			</Ul>

			<H2>アップデートと貢献</H2>
			<P>
				新機能やバグ修正を入手するには、GitHubから最新の変更をプルしてください。
				問題が発生した場合や提案がある場合は、Issueを開くかPull Requestを送信してください。
			</P>
		</DocLayout>
	);
}