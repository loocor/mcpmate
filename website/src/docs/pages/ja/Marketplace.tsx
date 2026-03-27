import React from "react";
import DocLayout from "../../layout/DocLayout";
import { H3, P } from "../../components/Headings";
import DocScreenshot from "../../components/DocScreenshot";

export default function Marketplace() {
	return (
		<DocLayout
			meta={{
				title: "インラインマーケットプレイス",
				description:
					"組み込みの公式MCPレジストリ - アプリから離れることなくサーバーを検索",
			}}
		>
			<P>
				MCPMateには、公式のMCPレジストリへのアクセスを提供する統合マーケットプレイスが含まれています。アプリケーションから離れることなく、新しいMCPサーバーを検索、インストール、および構成することができます。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/market-light.png"
				darkSrc="/screenshot/market-dark.png"
				alt="公式MCPレジストリを閲覧するインラインマーケットプレイス"
			/>

			<h2>機能</h2>
			<ul>
				<li>
					<strong>統合検索:</strong> 公式レジストリを検索
				</li>
				<li>
					<strong>ワンクリックインストール:</strong> マーケットプレイスからサーバーを直接インストール
				</li>
				<li>
					<strong>自動構成:</strong> サーバーはアクティブなプロファイルに自動的に追加されます
				</li>
				<li>
					<strong>バージョン管理:</strong> 新しいバージョンが利用可能な場合にサーバーを更新
				</li>
				<li>
					<strong>評価とレビュー:</strong> インストールする前にコミュニティのフィードバックを確認
				</li>
			</ul>

			<h2>サポートされているレジストリ</h2>
			<ul>
				<li>
					<strong>公式MCPレジストリ:</strong> Anthropicの公式サーバーコレクション
				</li>
			</ul>

			<h2>メリット</h2>
			<P>
				GitHubやドキュメントサイトを手動で検索したり、インストール手順を読んだり、構成ファイルを手動で編集したりする代わりに、マーケットプレイスを利用することで数回のクリックでプロセス全体を合理化できます。
			</P>

			<H3>MCPサーバー追加ウィザード</H3>
			<P>
				レジストリカードからインストールすると、ガイド付きのフローが開きます。トランスポートを構成し、正規化されたマニフェストをプレビューしてから、目的のプロファイルにインポートします。
			</P>
			<DocScreenshot
				lightSrc="/screenshot/market-add-server-light.png"
				darkSrc="/screenshot/market-add-server-dark.png"
				alt="コア構成フォームを備えたMCPサーバー追加ステッパー"
			/>
		</DocLayout>
	);
}