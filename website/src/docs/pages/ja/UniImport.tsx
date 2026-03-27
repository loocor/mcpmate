import DocLayout from "../../layout/DocLayout";
import { H2, Li, P, Ul } from "../../components/Headings";

export default function UniImport() {
	return (
		<DocLayout
			meta={{
				title: "ユニバーサルインポート (Uni-Import)",
				description:
					"正規化されたトランスポート処理を使用した、ドラッグアンドドロップまたはペーストによる簡単な構成",
			}}
		>
			<P>
				Uni-Importは、MCPMateの柔軟な構成インポートシステムです。JSONファイル、TOML構成、またはテキストスニペットのいずれであっても、シンプルなドラッグアンドドロップやペースト操作でMCPMateにインポートできます。
			</P>

			<H2>サポートされている形式</H2>
			<Ul>
				<Li>
					<strong>JSON:</strong> 標準のMCP構成形式
				</Li>
				<Li>
					<strong>TOML:</strong> 代替の構成形式
				</Li>
				<Li>
					<strong>スニペットテキスト:</strong> ドキュメント、チャット、またはチームWikiからの直接ペースト
				</Li>
			</Ul>

			<H2>インポート方法</H2>
			<Ul>
				<Li>
					<strong>ドラッグ＆ドロップ:</strong> 構成ファイルをMCPMateにドラッグするだけ
				</Li>
				<Li>
					<strong>ペースト:</strong> 構成テキストをコピーしてインポートダイアログにペースト
				</Li>
				<Li>
					<strong>ファイルブラウザ:</strong> 従来のファイル選択ダイアログ
				</Li>
			</Ul>

			<H2>スマート解析</H2>
			<P>
				Uni-Importは構成の形式を自動的に検出し、インポート前に検証します。問題がある場合、MCPMateは明確なエラーメッセージと修正のための提案を提供します。従来のSSEスタイルの入力も受け入れられ、永続化の際にStreamable HTTPに正規化されます。
			</P>

			<H2>ユースケース</H2>
			<Ul>
				<Li>チームの共有構成をインポートする。</Li>
				<Li>他のMCPツールから移行する。</Li>
				<Li>ドキュメントの例からすばやくセットアップする。</Li>
				<Li>バックアップから復元する。</Li>
			</Ul>
		</DocLayout>
	);
}