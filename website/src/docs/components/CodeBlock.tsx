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
		<div className="not-prose relative rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900">
			<div className="flex items-center justify-between px-3 py-2 border-b border-slate-200 dark:border-slate-700 text-xs text-slate-600 dark:text-slate-300">
				<span className="font-mono">{filename || lang}</span>
				<button
					onClick={onCopy}
					className="inline-flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-200 dark:hover:bg-slate-800"
				>
					<Copy size={14} /> {copied ? "Copied" : "Copy"}
				</button>
			</div>
			<pre className="overflow-x-auto p-3 text-sm">
				<code className={`language-${lang}`}>{code}</code>
			</pre>
		</div>
	);
}
