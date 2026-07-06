import { Highlight, themes, type PrismTheme } from "prism-react-renderer";
import { useCallback, useRef, useSyncExternalStore, type UIEvent } from "react";
import { useAppStore } from "../lib/store";
import { cn } from "../lib/utils";

export const JSON_CODE_BLOCK_CLASSNAME =
	"m-0 max-w-full overflow-x-auto whitespace-pre-wrap break-words rounded bg-slate-50 p-2 font-mono text-xs leading-normal dark:bg-slate-900";

const JSON_CODE_EDITOR_TEXTAREA_CLASSNAME =
	"absolute inset-0 resize-none overflow-auto border-0 bg-transparent p-2 font-mono text-xs leading-normal whitespace-pre-wrap break-words text-transparent caret-foreground selection:bg-primary/20 focus:outline-none focus:ring-0";

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
					className={cn(hlClass, JSON_CODE_BLOCK_CLASSNAME, className)}
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

/** Editable JSON surface with the same prism highlighting as {@link JsonCodeBlock}. */
export function JsonEditableCodeBlock({
	code,
	onCodeChange,
	readOnly = false,
	className,
	id,
	language = "json",
	"aria-label": ariaLabel,
}: {
	code: string;
	onCodeChange?: (code: string) => void;
	readOnly?: boolean;
	className?: string;
	id?: string;
	language?: string;
	"aria-label"?: string;
}) {
	const highlightRef = useRef<HTMLDivElement>(null);

	const syncHighlightScroll = useCallback((event: UIEvent<HTMLTextAreaElement>) => {
		const highlight = highlightRef.current;
		if (!highlight) {
			return;
		}
		highlight.scrollTop = event.currentTarget.scrollTop;
		highlight.scrollLeft = event.currentTarget.scrollLeft;
	}, []);

	if (readOnly) {
		return <JsonCodeBlock code={code} className={className} language={language} />;
	}

	return (
		<div className={cn("relative min-h-0 flex-1", className)}>
			<div
				ref={highlightRef}
				className="pointer-events-none absolute inset-0 overflow-auto"
				aria-hidden
			>
				<JsonCodeBlock
					code={code || " "}
					language={language}
					className="min-h-full rounded-none bg-transparent dark:bg-transparent"
				/>
			</div>
			<textarea
				id={id}
				value={code}
				onChange={(event) => onCodeChange?.(event.target.value)}
				onScroll={syncHighlightScroll}
				spellCheck={false}
				aria-label={ariaLabel}
				className={JSON_CODE_EDITOR_TEXTAREA_CLASSNAME}
				style={{ WebkitTextFillColor: "transparent" }}
			/>
		</div>
	);
}
