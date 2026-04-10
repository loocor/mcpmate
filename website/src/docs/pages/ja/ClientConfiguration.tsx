import DocLayout from "../../layout/DocLayout";
import { H2, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";

export default function ClientConfiguration() {
	return (
		<DocLayout
			meta={{
				title: "クライアント設定",
				description: "MCPMateがクライアント能力設定をどのように記述し、どこから取得するかを選択する",
			}}
		>
			<P>
				設定タブは、クライアントがプロファイル状態をどのように消費するかを決定する場所です。管理モード、能力ソース、適用アクション、およびインポートプレビューを組み合わせているため、望ましい将来の状態とディスク上の現在のファイルの両方を制御できます。
			</P>

			<H2>主な選択肢</H2>
			<Ul>
				<Li><strong>Unify モード</strong>は、ダッシュボード側でクライアントのワークセットを維持せず、内蔵 MCP / UCAN ツールでセッション内制御を行いたい場合に適しています。</Li>
				<Li><strong>Hosted モード</strong>は、ライブ切り替えやより細かいポリシー制御などの MCPMate 機能を使いたい場合に最適です。</Li>
				<Li><strong>トランスペアレントモード</strong>は、クライアントへ明示的なサーバー設定を書き込む必要があり、MCPMate 側の制御が減っても許容できる場合に適しています。</Li>
				<Li><strong>能力ソース</strong>は、Hosted またはトランスペアレントモードで、クライアントがアクティブなプロファイル、選択した共有プロファイル、またはクライアント固有のカスタムプロファイルのどれに従うかを決めます。</Li>
			</Ul>

			<H2>各モードの本当の意味</H2>
			<Ul>
				<Li><strong>Unify</strong> は内蔵 MCP ツールのみで開始し、現在のセッションではグローバルに有効なサーバーの capability を参照し、セッション終了時に自動的にリセットされます。</Li>
				<Li><strong>Hosted</strong> は、クライアントに MCPMate 管理のエンドポイントを 1 つ提供するため、ポリシー、プロファイル切り替え、可視性ロジックを中間層に維持できます。</Li>
				<Li><strong>トランスペアレントモード</strong> は、互換性や特殊な制御のために、有効なサーバーをクライアント自身の MCP 設定へ直接書き込みます。</Li>
			</Ul>

			<H2>ソースの選択は Hosted とトランスペアレントモードで使います</H2>
			<Ul>
				<Li><strong>Unify</strong> ではここでダッシュボード上のプロファイル選択を使いません。現在のセッションでは、内蔵 UCAN ツールでグローバルに有効なサーバーの capability を参照・呼び出します。</Li>
				<Li><strong>アクティベート済み</strong>は、グローバルにアクティブなプロファイルセットに従います。</Li>
				<Li><strong>プロファイル</strong>を使用すると、グローバルなアクティブセットが異なる場合でも、1つのクライアントが選択した共有プロファイルに従うことができます。</Li>
				<Li><strong>カスタマイズ</strong>は、クライアント固有のカスタムプロファイルを作成または再利用します。</Li>
			</Ul>

			<H2>推奨されるワークフロー</H2>
			<Ul>
				<Li>セッション内の内蔵制御を重視するなら Unify、持続的な管理ロールアウトなら Hosted、直接クライアント設定へ書き出したいならトランスペアレントモードを選びます。</Li>
				<Li>能力ソースは Hosted またはトランスペアレントモードを使う場合にのみ選択します。</Li>
				<Li>既存のクライアント設定を上書きする前に比較する必要がある場合は、プレビューまたはインポートを行います。</Li>
				<Li>概要タブにクライアントが検出され、到達可能であることが示された後にのみ適用します。</Li>
			</Ul>

			<H2>Unify 直接公開（Unify 専用）</H2>
			<P>
				直接公開は Unify にのみ適用されます。Hosted とトランスペアレントモードの意味は変わりません。
			</P>

			<Ul>
				<Li><strong>All Proxy</strong>（既定）: 直接公開対象としてマークされたサーバーを含め、すべての有効サーバーは内蔵 UCAN ツール経由でアクセスされます。</Li>
				<Li><strong>Server Direct</strong>: Unify 直接公開対象としてマークされた選択済みサーバーの全 capability をクライアントへ直接公開します。</Li>
				<Li><strong>Capability-Level Direct</strong>（上級）: 選択したツールだけを直接公開します。v1 ではツールのみ対象で、prompts / resources / templates は引き続き broker 経由です。</Li>
			</Ul>

			<P>
				Capability-Level Direct は Profiles ルートを流用せず、Clients 配下の専用編集ページを開きます。これにより、ナビゲーション状態を正しく保ったまま、ツールの一括編集 UI を再利用できます。
			</P>

			<Callout type="warning" title="トランスペアレントモードではトレードオフが変わります">
				トランスペアレントモードはサーバー設定をクライアントへ直接書き込みます。互換性には役立ちますが、capability レベルで MCPMate が制御できる範囲は減少します。
			</Callout>

			<Callout type="info" title="なぜ Hosted モードのほうが強力に見えるのか">
				Hosted モードは、MCPMate の内蔵プロファイル / クライアントツール、クライアント対応の可視性ロジック、より豊富なポリシー判断を有効に保ちます。トランスペアレントモードは意図的により単純で、ランタイム対応の制御よりも直接的な設定出力を優先します。
			</Callout>
		</DocLayout>
	);
}
