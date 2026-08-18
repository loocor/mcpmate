import { memo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { cn } from "../lib/utils";
import { SyntaxHighlightedCode } from "./json-code-block";
import {
	materialPreviewLanguage,
	shouldHighlightMaterialSource,
} from "./material-preview-language";

const SOURCE_PRE_CLASS =
	"m-0 min-h-full min-w-0 whitespace-pre-wrap break-words rounded-none bg-transparent p-0 font-mono text-xs leading-5 text-slate-700 dark:bg-transparent dark:text-slate-300";

function fencedCodeLanguage(className?: string): string {
	const match = /language-([\w-]+)/.exec(className ?? "");
	return materialPreviewLanguage(match?.[1] ?? "");
}

function MarkdownInlineCode({ children }: { children: ReactNode }) {
	return <code className="font-mono text-xs">{children}</code>;
}

function hasFencedCodeLanguage(children: ReactNode): boolean {
	if (
		typeof children !== "object" ||
		children === null ||
		!("props" in children)
	) {
		return false;
	}
	const className = (children.props as { className?: string }).className;
	return className?.includes("language-") ?? false;
}

function RichMarkdownPreview({
	content,
	highlightFences,
}: {
	content: string;
	highlightFences: boolean;
}) {
	return (
		<div className="min-w-0 text-sm leading-6 text-slate-700 dark:text-slate-300">
			<ReactMarkdown
				remarkPlugins={[remarkGfm]}
				rehypePlugins={[rehypeSanitize]}
				components={{
					p: ({ children }) => <p className="mb-3">{children}</p>,
					h1: ({ children }) => (
						<h1 className="mb-3 text-xl font-semibold">{children}</h1>
					),
					h2: ({ children }) => (
						<h2 className="mb-3 text-lg font-semibold">{children}</h2>
					),
					h3: ({ children }) => (
						<h3 className="mb-2 text-base font-semibold">{children}</h3>
					),
					ul: ({ children }) => (
						<ul className="mb-3 list-disc space-y-1 pl-5">{children}</ul>
					),
					ol: ({ children }) => (
						<ol className="mb-3 list-decimal space-y-1 pl-5">{children}</ol>
					),
					blockquote: ({ children }) => (
						<blockquote className="mb-3 border-l-2 border-slate-300 pl-3 text-muted-foreground dark:border-slate-700">
							{children}
						</blockquote>
					),
					pre: ({ children }) => {
						if (highlightFences && hasFencedCodeLanguage(children)) {
							return <>{children}</>;
						}
						return (
							<pre className="mb-3 overflow-x-auto whitespace-pre-wrap break-words font-mono text-xs">
								{children}
							</pre>
						);
					},
					code: ({ className, children }) => {
						if (!highlightFences || !className?.includes("language-")) {
							return <MarkdownInlineCode>{children}</MarkdownInlineCode>;
						}
						return (
							<SyntaxHighlightedCode
								code={String(children).replace(/\n$/, "")}
								language={fencedCodeLanguage(className)}
								className="mb-3 rounded-none bg-transparent p-0 dark:bg-transparent"
							/>
						);
					},
				}}
			>
				{content}
			</ReactMarkdown>
		</div>
	);
}

type MaterialFilePreviewProps = {
	content: string;
	extension: string;
	markdownMode?: "rendered" | "source";
	className?: string;
};

export const MaterialFilePreview = memo(function MaterialFilePreview({
	content,
	extension,
	markdownMode = "rendered",
	className,
}: MaterialFilePreviewProps) {
	const { t } = useTranslation();
	const normalizedExtension = extension.toLowerCase();
	const isRenderedPreview =
		normalizedExtension === "md" && markdownMode === "rendered";
	const highlightSource = shouldHighlightMaterialSource(content.length);

	if (isRenderedPreview) {
		return (
			<div className={cn("min-w-0", className)}>
				<RichMarkdownPreview
					content={content}
					highlightFences={highlightSource}
				/>
			</div>
		);
	}

	if (highlightSource) {
		return (
			<div className={cn("min-w-0", className)}>
				<SyntaxHighlightedCode
					code={content}
					language={materialPreviewLanguage(normalizedExtension)}
					className={SOURCE_PRE_CLASS}
				/>
			</div>
		);
	}

	return (
		<div className={cn("min-w-0", className)}>
			<p className="mb-2 text-[11px] text-muted-foreground">
				{t("profiles:detail.workflow.materials.previewSourcePlainForSize", {
					defaultValue:
						"Large file ({{size}} KB): shown as source without syntax highlighting.",
					size: Math.round(content.length / 1024),
				})}
			</p>
			<pre className={SOURCE_PRE_CLASS}>{content}</pre>
		</div>
	);
});
