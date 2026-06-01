import React from "react";
import { Copy } from "lucide-react";

export default function CodeBlock({
	lang = "",
	filename,
	children,
}: {
	lang?: string;
	filename?: string;
	children: string | React.ReactNode;
}) {
	const code =
		typeof children === "string" ? children.trim() : String(children);
	const [copied, setCopied] = React.useState(false);
	const onCopy = async () => {
		try {
			await navigator.clipboard.writeText(code);
			setCopied(true);
			setTimeout(() => setCopied(false), 1200);
		} catch {
			/* ignore */
		}
	};
	return (
		<div className="not-prose relative rounded-lg border border-brand-border bg-brand-elevated">
			<div className="flex items-center justify-between border-b border-brand-border px-3 py-2 text-xs text-brand-muted">
				<span className="font-mono">{filename || lang}</span>
				<button
					onClick={onCopy}
					className="inline-flex items-center gap-1 rounded px-2 py-1 hover:bg-brand-overlay"
				>
					<Copy size={14} /> {copied ? "Copied" : "Copy"}
				</button>
			</div>
			<pre className="overflow-x-auto p-3 text-sm text-brand-foreground">
				<code className={`language-${lang}`}>{code}</code>
			</pre>
		</div>
	);
}
