import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function AutoDiscovery() {
	return (
		<DocLayout
			meta={{
				title: "自動検出とインポート",
				description:
					"既存の構成を自動的に検出してインポート - 手動での編集は不要",
			}}
		>
			<P>
				MCPMateは、システム上の既存のMCPサーバー構成を自動的に検出し、ワンクリックでインポートできます。これにより、新しいツールで設定を手動で再作成するという面倒なプロセスが不要になります。
			</P>

			<H2>仕組み</H2>
			<P>
				MCPMateは、人気のあるMCPクライアントが使用する一般的な構成ファイルの場所をスキャンします：
			</P>
			<Ul>
				<Li>Claude Desktopの構成ファイル</Li>
				<Li>VS CodeのMCP拡張機能の設定</Li>
				<Li>CursorのMCP構成</Li>
				<Li>その他の標準的なMCPクライアントのセットアップ</Li>
			</Ul>

			<H2>インポートプロセス</H2>
			<Ul>
				<Li>MCPMateが既存の構成を自動的にスキャンします。</Li>
				<Li>インポートビューに、検出されたサーバーが確認用に一覧表示されます。</Li>
				<Li>インポートする対象と、配置先のプロファイルを選択します。</Li>
				<Li>インポートされたエントリは正規化され、MCPMateに保存されます。</Li>
			</Ul>

			<H2>メリット</H2>
			<Ul>
				<Li>
					<strong>迅速なオンボーディング:</strong> すぐにMCPMateを使い始めることができます
				</Li>
				<Li>
					<strong>手作業の排除:</strong> 設定の詳細を手動でコピーする手間が省けます
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