import { Fragment, useSyncExternalStore } from "react";
import {
	Highlight,
	themes,
	type PrismTheme,
	type Token,
	type TokenOutputProps,
} from "prism-react-renderer";
import { useAppStore } from "../lib/store";
import { cn } from "../lib/utils";

function isBlankPrismLine(line: Token[]): boolean {
	return line.every(
		(token) => token.empty === true || token.content === "" || token.content === "\n",
	);
}

function PrismSourceLine({
	line,
	lineIdx,
	isLastLine,
	getTokenProps,
}: {
	line: Token[];
	lineIdx: number;
	isLastLine: boolean;
	getTokenProps: (input: { token: Token }) => TokenOutputProps;
}) {
	if (isBlankPrismLine(line)) {
		return "\n";
	}

	return (
		<Fragment>
			{line.map((token, tokenIdx) => (
				<span
					key={`tok-${lineIdx}-${tokenIdx}`}
					{...getTokenProps({ token })}
				/>
			))}
			{isLastLine ? null : "\n"}
		</Fragment>
	);
}

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
export function SyntaxHighlightedCode({
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
			{({ className: hlClass, style, tokens, getTokenProps }) => (
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
						<PrismSourceLine
							key={`line-${lineIdx}`}
							line={line}
							lineIdx={lineIdx}
							isLastLine={lineIdx === tokens.length - 1}
							getTokenProps={getTokenProps}
						/>
					))}
				</pre>
			)}
		</Highlight>
	);
}

export function JsonCodeBlock(props: {
	code: string;
	className?: string;
	language?: string;
}) {
	return <SyntaxHighlightedCode {...props} />;
}
