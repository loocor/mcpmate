import { Highlight, themes, type PrismTheme } from "prism-react-renderer";
import { useSyncExternalStore } from "react";
import { useAppStore } from "../lib/store";
import { cn } from "../lib/utils";

function useResolvedDark(): boolean {
	const theme = useAppStore((s) => s.theme);
	const systemDark = useSyncExternalStore(
		(onChange) => {
			const mq = window.matchMedia("(prefers-color-scheme: dark)");
			mq.addEventListener("change", onChange);
			return () => mq.removeEventListener("change", onChange);
		},
		() => window.matchMedia("(prefers-color-scheme: dark)").matches,
		() => false,
	);
	if (theme === "dark") {
		return true;
	}
	if (theme === "light") {
		return false;
	}
	return systemDark;
}

/** Syntax-highlighted code block (via prism-react-renderer). Defaults to JSON. */
export function JsonCodeBlock({
	code,
	className,
	language = "json",
}: {
	code: string;
	className?: string;
	/** Prism language id (e.g. `json`, `plaintext`). */
	language?: string;
}) {
	const isDark = useResolvedDark();
	const prismTheme: PrismTheme = isDark ? themes.vsDark : themes.vsLight;

	return (
		<Highlight theme={prismTheme} code={code} language={language}>
			{({ className: hlClass, style, tokens, getLineProps, getTokenProps }) => (
				<pre
					className={cn(
						hlClass,
						"m-0 max-w-full overflow-x-auto whitespace-pre-wrap break-words rounded bg-slate-50 p-2 font-mono text-xs dark:bg-slate-900",
						className,
					)}
					style={{
						...style,
						background: undefined,
						backgroundColor: undefined,
					}}
				>
					{tokens.map((line, lineIdx) => (
						<div key={`line-${lineIdx}`} {...getLineProps({ line })}>
							{line.map((token, tokenIdx) => (
								<span key={`tok-${lineIdx}-${tokenIdx}`} {...getTokenProps({ token })} />
							))}
						</div>
					))}
				</pre>
			)}
		</Highlight>
	);
}
