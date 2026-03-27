import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function RuntimeJA() {
	return (
		<DocLayout
			meta={{
				title: "ランタイム",
				description: "ランタイムの制御と正常性",
			}}
		>
			<P>
				ランタイム画面では、MCPMateがMCPサーバー用に管理する組み込み環境（現在は<strong>uv</strong>と<strong>Bun</strong>）を公開します。これを使用して、新しいトランスポートやサーバーのアップグレードをテストする際に、インストールの確認、キャッシュのクリア、機能状態のリセットを行います。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/runtime-light.png"
				darkSrc="/screenshot/runtime-dark.png"
				alt="Runtime page with uv, Bun, and capabilities cache stats"
			/>

			<H2>ランタイム ステータスカード</H2>
			<Ul>
				<Li>
					各ランタイムカードには、可用性、バージョン、インストールフォルダ、最後のステータスメッセージ、キャッシュサイズ、パッケージ数、および最終変更時刻が表示されます。
				</Li>
				<Li>
					<strong>インストール / 修復</strong>ボタンは、<code>verbose=true</code>でインストーラーを実行するため、バックエンドコンソールで詳細なログを確認できます。キャッシュをクリアした後、またはランタイムがヘルスチェックに失敗した場合にこれを使用します。
				</Li>
				<Li>
					<strong>キャッシュリセット</strong>ボタンは、そのランタイムに対してのみダウンロードされたパッケージを消去します。MCPMateは、次のサーバーリクエスト時にキャッシュを再構築します。
				</Li>
			</Ul>

			<H2>機能キャッシュの制御</H2>
			<P>
				下部のカードには、機能キャッシュデータベース（パス、サイズ、最終クリーンアップ）と、サーバー、ツール、リソース、プロンプト、リソーステンプレートの数がまとめられています。また、キャッシュの効率を評価するのに役立つヒット/ミス率のメトリクスも追跡します。
			</P>
			<Ul>
				<Li>
					サーバーマニフェストを変更したり、インスペクターにツールメタデータを再取得させたい場合は、<strong>機能をリセット</strong>を使用します。
				</Li>
				<Li>
					キャッシュをクリアすると署名が直ちに無効になります。その後のリクエストにより、プロキシからの最新データでエントリが再入力されます。
				</Li>
			</Ul>

			<H2>推奨されるメンテナンスフロー</H2>
			<H3>新しいサーバーをインポートする前に</H3>
			<P>
				両方のランタイムが適切なバージョンで<em>実行中（running）</em>と表示されていることを確認します。ステータスバッジが<em>停止（stopped）</em>の場合は、最初に「インストール / 修復」を実行します。
			</P>

			<H3>プロファイルの大幅な編集後</H3>
			<P>
				古いプロンプトやリソーステンプレートを避けるために、機能キャッシュをクリアします。キャッシュが再入力されたら、インスペクターのチェックリストを再実行して、リストの応答が目標の5秒未満に収まっていることを確認します。
			</P>

			<Callout type="warning" title="キャッシュのリセットはダウンロードしたパッケージを削除します">
				uvまたはBunのキャッシュをリセットすると、仮想環境のコンテンツが削除されます。その後のサーバー呼び出しでは依存関係が再インストールされ、時間がかかる場合があります。エンドユーザーが接続している間ではなく、メンテナンスウィンドウ中または自動テストを実行する前にリセットをスケジュールしてください。
			</Callout>
		</DocLayout>
	);
}
