import DocLayout from "../../layout/DocLayout";
import { H2, H3, P, Ul, Li } from "../../components/Headings";
import Callout from "../../components/Callout";
import DocScreenshot from "../../components/DocScreenshot";

export default function ClientApps() {
	return (
		<DocLayout
			meta={{
				title: "クライアントアプリ",
				description: "MCPMateと統合されるアプリ",
			}}
		>
			<P>
				クライアント画面は、MCPMateと通信できるデスクトップアプリケーション（Cursor、Claude Desktop、Zedなど）を追跡します。エディターの統合をプロキシと同期させるために、自動検出、管理トグル、および構成のヒントを組み合わせています。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/clients-light.png"
				darkSrc="/screenshot/clients-dark.png"
				alt="Clients grid with detection and managed toggles"
			/>

			<H2>ガイドマップ</H2>
			<Ul>
				<Li>
					<strong>詳細概要</strong>は、状態バッジ、検出、ドキュメントリンク、トランスポートバッジ、および現在のサーバーカードについて説明します。
				</Li>
				<Li>
					<strong>設定</strong>は、Unify、Hosted、トランスペアレントモードの違い、能力ソース、適用フロー、およびインポートプレビューについて説明します。
				</Li>
				<Li>
					<strong>バックアップ</strong>は、保持、ロールバック、一括削除、およびリカバリのガイダンスに焦点を当てています。
				</Li>
			</Ul>

			<H2>統計とフィルター</H2>
			<Ul>
				<Li>
					統計カードは、発見されたクライアント、ディスク上で検出された数、管理モードになっている数、およびすでにMCP設定ファイルを持っている数をカウントします。
				</Li>
				<Li>
					ツールバーは、検索（表示名、識別子、説明）、ソートオプション（アルファベット順、検出済み、管理対象）、および設定に保存されているのと同じデフォルトを共有するグリッド/リストトグルを提供します。
				</Li>
				<Li>
					フィルタードロップダウンを使用して、<em>すべて</em>、<em>検出済み</em>のみ、または<em>管理対象</em>のみのクライアントをすばやく表示します。選択はストアにフィードバックされるため、次回のアクセス時にも同じビューが読み込まれます。
				</Li>
			</Ul>

			<H2>統合状態の管理</H2>
			<H3>検出バッジ</H3>
			<P>
				MCPMateがクライアントバイナリを特定すると、各カード/リスト行に緑色の<strong>検出済み</strong>バッジが表示されます。クライアントが見つからない場合は、更新アイコンを使用して再スキャン（<code>/clients?force_refresh=true</code>）をトリガーし、アプリをインストールしてリロードします。
			</P>

			<H3>管理トグル</H3>
			<P>
				各アイテムの右下にあるスイッチは、管理モードを有効または無効にします。有効にすると、MCPMateはクライアントの構成をアクティブなプロファイルセットと同期させます。トグルはすぐに更新され、成功または失敗時にトーストを表示します。
			</P>

			<H3>クライアントの詳細</H3>
			<P>
				カードをクリックすると<code>/clients/:identifier</code>が開きます。詳細レイアウトは3つのタブを使用します。
			</P>

			<DocScreenshot
				lightSrc="/screenshot/client-detail-light.png"
				darkSrc="/screenshot/client-detail-dark.png"
				alt="Client detail overview with config path and current servers"
			/>

			<Ul>
				<Li>
					<strong>概要</strong> &mdash; 検出状態、管理モード、プロファイル適用アクション、およびショートカット（クライアントのMCP構成フォルダーを開くなど）。
				</Li>
				<Li>
					<strong>設定</strong> &mdash; MCPMate がこのクライアント用に書き込む MCP サーバーのライブビュー、クライアントからのインポートフロー、および Unify / Hosted / トランスペアレントモードのガイダンス。
				</Li>
				<Li>
					<strong>バックアップ</strong> &mdash; プロファイルやインポートを適用したときに作成されるローテーションスナップショット。スナップショットを復元してロールバックしたり、選択したバックアップを削除したり、適用が成功した後にリストを更新したりできます。
				</Li>
			</Ul>
			<P>
				バックアップの保持制限とデフォルトのフィルターは<strong>設定 → クライアントのデフォルト</strong>から取得されるため、大規模なロールアウトを行う前にそれらを調整してください。
			</P>

			<Callout type="warning" title="クライアントが検出されないままの場合">
				クライアントがデフォルトの場所にインストールされていること、およびプロキシプロセスにアプリケーションディレクトリをスキャンする権限があることを確認してください。macOSの場合、MCPMateサービスに「フルディスクアクセス」を付与する必要がある場合があります。権限を調整した後、<strong>更新</strong>を押して再スキャンを強制します。
			</Callout>
		</DocLayout>
	);
}
