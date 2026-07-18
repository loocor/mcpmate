import { useMemo } from "react";
import { Link } from "react-router-dom";
import SchemaOrg from "../../../components/SchemaOrg";
import { buildHowTo } from "../../../utils/schema";
import Callout from "../../components/Callout";
import CommunityLinks from "../../components/CommunityLinks";
import CopyableInlineCode from "../../components/CopyableInlineCode";
import DesktopDownloadList from "../../components/DesktopDownloadList";
import { H2, Li, P, Ul } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

const howToSteps = [
	{
		name: "MCPMate をインストールして起動する",
		text: "プラットフォームに合うデスクトップパッケージをダウンロードし、MCPMate を開きます。",
	},
	{
		name: "オンボーディングを完了する",
		text: "既存のクライアントと Server を検出し、利用する設定を確認します。",
	},
	{
		name: "最初の Server を追加する",
		text: "Market から Server を選ぶか既存の MCP 設定をインポートし、導入前に内容を確認します。",
	},
	{
		name: "クライアントへ接続する",
		text: "Default プロファイルを使って、検出された AI クライアントに Server を公開します。",
	},
	{
		name: "接続を確認する",
		text: "クライアントで簡単な MCP 操作を実行し、Server の機能が利用できることを確認します。",
	},
];

export default function Quickstart() {
	const howTo = useMemo(
		() =>
			buildHowTo({
				name: "MCPMate クイックスタート",
				description: "MCPMate をインストールし、最初の MCP Server を AI クライアントで利用できるようにします。",
				steps: howToSteps,
			}),
		[],
	);

	return (
		<DocLayout
			meta={{
				title: "クイックスタート",
				description: "MCPMate のインストールから最初の MCP Server の確認までを短い手順で進めます。",
			}}
		>
			<SchemaOrg schema={howTo} />
			<P>このガイドでは、MCPMate をインストールし、初回設定を完了して、ひとつの Server が AI クライアントで動作するまでの最短ルートを説明します。</P>

			<H2>デスクトップアプリから始める</H2>
			<P>OS とプロセッサーに合うインストーラーを選んでください。以下のリンクは MCPMate の追跡可能なダウンロードサービスを経由し、現在のリリース成果物を取得します。</P>
			<DesktopDownloadList locale="ja" />
			<Callout type="info" title="Homebrew も利用できます">
				macOS と Linux では{" "}
				<CopyableInlineCode
					copyLabel="コマンドをコピー"
					copiedLabel="コピーしました"
					errorLabel="コピーできませんでした"
				>
					brew install --cask loocor/tap/mcpmate@beta
				</CopyableInlineCode>{" "}
				を実行します。対応システム、更新、アンインストール方法は{" "}
				<Link className="font-medium underline" to="/docs/ja/installation">
					インストールガイド
				</Link>
				を参照してください。
			</Callout>

			<H2>オンボーディングを完了する</H2>
			<Ul>
				<Li>インストール後に MCPMate を開き、ウェルカム画面の手順を進めます。</Li>
				<Li>このマシンで検出された AI クライアントと MCP Server を確認します。</Li>
				<Li>利用する既存設定を残します。初めて MCP を使う場合は、スターター Server を選ぶこともできます。</Li>
			</Ul>

			<H2>最初の Server を追加する</H2>
			<Ul>
				<Li>
					<strong>Market</strong> で Server を選ぶか、<strong>Servers</strong> で既存の設定をインポートします。
				</Li>
				<Li>検出されたコマンド、引数、必要な値を確認します。</Li>
				<Li>プレビュー確認を実行し、問題がなければインストールします。</Li>
			</Ul>

			<H2>クライアントへ接続する</H2>
			<Ul>
				<Li>
					<strong>Profiles</strong> で <strong>Default</strong> を開き、追加した Server が含まれていることを確認します。
				</Li>
				<Li>
					<strong>Clients</strong> で検出済みの AI アプリを選び、MCPMate が推奨する設定で Default プロファイルを適用します。
				</Li>
			</Ul>

			<H2>最初の機能を確認する</H2>
			<P>接続した AI クライアントを開くか再起動し、Server が提供する簡単な操作をひとつ実行します。クライアントがその機能を表示して呼び出せれば、最初の設定は完了です。</P>

			<H2>コミュニティに参加する</H2>
			<P>使い方について相談したり、活用例や改善してほしい点を共有したりできます。</P>
			<CommunityLinks locale="ja" />
		</DocLayout>
	);
}
