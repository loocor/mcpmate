import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function CentralizedConfig() {
	return (
		<DocLayout
			meta={{
				title: "一元化された設定",
				description:
					"一度設定すれば、すべてのMCPクライアントでどこでも使用可能",
			}}
		>
			<P>
				MCPMateのコア機能の1つは、一元化された設定管理です。
				各MCPクライアントに対して個別の構成を維持する代わりに、MCPMateでサーバーを一度設定するだけで、接続されているすべてのクライアントで自動的に利用可能になります。
			</P>

			<H2>メリット</H2>
			<Ul>
				<Li>
					<strong>単一の信頼できる情報源 (Single Source of Truth):</strong> すべてのMCPサーバー構成が1か所で管理されます
				</Li>
				<Li>
					<strong>重複の排除:</strong> 異なるクライアント間で設定をコピーする必要がなくなります
				</Li>
				<Li>
					<strong>一貫したエクスペリエンス:</strong> すべてのクライアントが同じサーバー構成を使用することを保証します
				</Li>
				<Li>
					<strong>簡単なアップデート:</strong> 構成を一度変更するだけで、すべての場所に適用されます
				</Li>
			</Ul>

			<H2>仕組み</H2>
			<P>
				MCPMateは、すべてのMCPサーバーの中央ハブとして機能します。
				MCPMateでサーバーを設定すると、接続されているすべてのクライアントで自動的に利用可能になります。これにより、クライアントアプリケーションごとに設定ファイルを手動で編集するという従来のワークフローが不要になります。
			</P>

			<H2>ユースケース</H2>
			<Ul>
				<Li>Claude Desktop、Cursor、VS Code間で同じサーバーを使用する。</Li>
				<Li>チーム全体のサーバー構成を管理する。</Li>
				<Li>再構成なしで新しいクライアントをオンボーディングする。</Li>
			</Ul>

			<H2>分離デプロイモードの利用</H2>
			<P>
				コアサービスがUIシェルから独立して実行される場合でも、一元化された設定は同じ運用モデルを維持します。UIが構成を編集し、コアサービスがそれを永続化し、クライアントは管理されたワークフローを通じてそれを消費します。
			</P>
		</DocLayout>
	);
}