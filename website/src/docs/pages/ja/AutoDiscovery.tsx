import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "自動検出とインポート",
				description:
					"ローカルの MCP 設定と Discovery プリセットを使ってセットアップを速めます。",
			}}
		>
			<P>
				MCPMate はローカル設定のスキャンと Public Discovery カタログを組み合わせます。ローカル検出はマシン上にある MCP 設定を見つけ、Discovery プリセットは新しいクライアントやサーバー設定の起点を提供します。
			</P>

			<H2>ローカル検出</H2>
			<P>
				MCPMate は、人気のある MCP クライアントが使用する一般的な構成ファイルの場所をスキャンします：
			</P>
			<Ul>
				<Li>Claude Desktopの構成ファイル</Li>
				<Li>VS CodeのMCP拡張機能の設定</Li>
				<Li>CursorのMCP構成</Li>
				<Li>ユーザー定義クライアントを含む標準的なMCPクライアント設定</Li>
			</Ul>

			<H2>Discovery プリセット</H2>
			<P>
				Public Discovery は、初回起動フロー、クライアントの追加/編集ドロワー、ブラウザー拡張機能にプリセット項目を提供します。項目には識別子、表示名、リンク、アイコン、インポートメタデータが含まれ、確認前のドラフト作成に役立ちます。
			</P>
			<Ul>
				<Li>クライアントプリセットは、既知の MCP 設定先を持つ AI アプリの追加に使われます。</Li>
				<Li>サーバー項目は、サーバーインストールウィザードにインポート用メタデータを提供します。</Li>
				<Li>ポータル項目は、マーケット文書とブラウザー拡張機能の Portal タブをつなぎます。</Li>
			</Ul>

			<H2>インポートプロセス</H2>
			<Ul>
				<Li>MCPMate がローカルの既存設定をスキャンします。</Li>
				<Li>インポートビューには、検出されたサーバーと Discovery 由来のドラフトが確認用に表示されます。</Li>
				<Li>インポートする対象と、配置先のプロファイルを選択します。</Li>
				<Li>インポートされたエントリは正規化され、MCPMateに保存されます。</Li>
			</Ul>

			<H2>メリット</H2>
			<Ul>
				<Li>
					<strong>迅速なオンボーディング:</strong> すぐにMCPMateを使い始めることができます
				</Li>
				<Li>
					<strong>ガイド付きセットアップ:</strong> 検出済みのローカル状態または Discovery プリセットから始められます
				</Li>
				<Li>
					<strong>既存のセットアップの保持:</strong> 元の構成ファイルはそのまま保持されます
				</Li>
				<Li>
					<strong>エラーの防止:</strong> 手入力による設定ミスを減らします
				</Li>
			</Ul>
		</DocLayout>
	);
}
