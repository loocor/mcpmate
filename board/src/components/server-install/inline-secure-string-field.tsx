import { KeyRound, X } from "lucide-react";
import {
	useCallback,
	useEffect,
	useLayoutEffect,
	useMemo,
	useRef,
	type ClipboardEvent,
	type CompositionEvent,
	type FocusEvent,
	type KeyboardEvent,
} from "react";
import { useTranslation } from "react-i18next";
import {
	appendInlineText,
	backspaceInlineAtEnd,
	backspaceSecretBeforeTextBoundary,
	buildInlineDisplayItems,
	insertInlineSecretPlaceholder,
	isFlexibleInlineTextSlot,
	type InlineFocusTarget,
	type InlineInsertTarget,
	prependInlineText,
	removeInlineSecretSegment,
	resolveFocusAfterAppendInlineText,
	resolveFocusAfterBackspaceAtTextBoundary,
	resolveFocusAfterPrependInlineText,
	resolveInlineFocusTargetAfterUpdate,
	updateInlineSecretTextSegment,
} from "../../lib/inline-secret-value";
import type { SecretOrigin } from "../../lib/types";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import {
	FIELD_PAIR_VALUE_ACTIONS_PR,
	FIELD_PAIR_VALUE_PICKER_CLASS,
	FIELD_SINGLE_VALUE_ACTIONS_PR,
	FIELD_SINGLE_VALUE_PICKER_CLASS,
	PairFieldRemoveButton,
	type PairFieldRemoveProps,
} from "./field-list";
import { SecretPickerButton } from "./secret-picker-button";

interface InlineSecureStringFieldProps {
	id?: string;
	value: string;
	onChange: (value: string) => void;
	onRequestPlainFocus?: (caretOffset: number) => void;
	onBlur?: () => void;
	placeholder?: string;
	className?: string;
	pickerClassName?: string;
	onCreateSecret?: (origin: SecretOrigin) => void;
	origin?: SecretOrigin;
	pairLayout?: boolean;
	pairRemove?: PairFieldRemoveProps;
}

const secretBadgeClassName =
	"inline-flex h-6 min-w-0 max-w-full items-center gap-1 rounded-full px-2 py-0 font-mono text-xs bg-emerald-100 text-emerald-800 hover:bg-emerald-200 dark:bg-emerald-500/20 dark:text-emerald-200 dark:hover:bg-emerald-500/30";

const secretBadgeShellClassName =
	"relative inline-flex h-6 shrink-0 items-center max-w-[11rem] group/badge";

const secretBadgeIconSlotClassName =
	"relative inline-flex h-3 w-3 shrink-0 items-center justify-center";

const secretBadgeKeyIconClassName =
	"h-3 w-3 transition-opacity group-hover/badge:opacity-0 group-focus-within/badge:opacity-0 [@media(hover:none)]:opacity-0";

const secretBadgeClearButtonClassName =
	"absolute inset-0 flex h-3 w-3 items-center justify-center rounded-full bg-destructive text-white opacity-0 transition-opacity hover:bg-destructive/90 focus:opacity-100 group-hover/badge:opacity-100 group-focus-within/badge:opacity-100 [@media(hover:none)]:opacity-100";

const chipTextInputClassName =
	"h-6 min-w-[1ch] border-0 bg-transparent p-0 text-sm outline-none focus:outline-none focus:ring-0";

/** Zero-flow prefix caret: sits in left padding, does not push the first badge right. */
const prefixInputClassName =
	"absolute left-2 top-1/2 z-[1] h-6 w-2 -translate-y-1/2 border-0 bg-transparent p-0 text-sm outline-none focus:outline-none focus:ring-0";

/** Match shadcn `Input` field chrome; px-2 aligns with vertical inset for h-6 chips in h-10. */
const inlineFieldShellClassName =
	"relative flex h-10 w-full min-w-0 flex-nowrap items-center gap-1 overflow-x-auto rounded-md border border-input bg-background px-2 text-sm ring-offset-background focus-within:outline-none focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2";

export function InlineSecureStringField({
	id,
	value,
	onChange,
	onRequestPlainFocus,
	onBlur,
	placeholder,
	className,
	pickerClassName,
	onCreateSecret,
	origin,
	pairLayout = false,
	pairRemove,
}: InlineSecureStringFieldProps) {
	const { t } = useTranslation("servers");
	const containerRef = useRef<HTMLDivElement>(null);
	const insertTargetRef = useRef<InlineInsertTarget | null>(null);
	const pickerInsertTargetRef = useRef<InlineInsertTarget | null>(null);
	const inputRefs = useRef<Record<string, HTMLInputElement | null>>({});
	const prefixRef = useRef<HTMLInputElement>(null);
	const trailingRef = useRef<HTMLInputElement>(null);
	const badgeClearRefs = useRef<Record<string, HTMLButtonElement | null>>({});
	const focusRestoreRef = useRef<InlineFocusTarget | null>(null);
	const suppressContainerBlurRef = useRef(false);
	const displayItems = useMemo(() => buildInlineDisplayItems(value), [value]);

	const secretBadgeLabel = (alias: string) =>
		t("manual.secrets.tagAlias", {
			defaultValue: "{{alias}}",
			alias,
		});

	const syncInsertTarget = useCallback(
		(segmentIndex: number, input: HTMLInputElement | null) => {
			if (!input) return;
			insertTargetRef.current = {
				segmentIndex,
				offset: input.selectionStart ?? input.value.length,
			};
		},
		[],
	);

	const handleContainerBlur = useCallback(
		(event: FocusEvent<HTMLDivElement>) => {
			if (suppressContainerBlurRef.current || focusRestoreRef.current) {
				return;
			}

			const next = event.relatedTarget as Node | null;
			if (!next) {
				requestAnimationFrame(() => {
					if (suppressContainerBlurRef.current || focusRestoreRef.current) {
						return;
					}
					if (containerRef.current?.contains(document.activeElement)) {
						return;
					}
					insertTargetRef.current = null;
					onBlur?.();
				});
				return;
			}

			if (containerRef.current?.contains(next)) {
				return;
			}
			insertTargetRef.current = null;
			onBlur?.();
		},
		[onBlur],
	);

	const captureInsertTargetForPicker = useCallback(() => {
		pickerInsertTargetRef.current = insertTargetRef.current;
	}, []);

	const handleSecretPick = (placeholderValue: string) => {
		const target =
			pickerInsertTargetRef.current ?? insertTargetRef.current ?? undefined;
		pickerInsertTargetRef.current = null;
		const next = insertInlineSecretPlaceholder(value, placeholderValue, target);
		onChange(next);
	};

	const resolveInputRef = useCallback((key: string) => {
		if (key === "prefix") return prefixRef.current;
		if (key === "trailing") return trailingRef.current;
		return inputRefs.current[key];
	}, []);

	const applyFocusTarget = useCallback(
		(target: InlineFocusTarget) => {
			if (target.mode === "plain") {
				onRequestPlainFocus?.(target.caretOffset);
				return;
			}

			const input = resolveInputRef(target.inputKey);
			input?.focus();
			input?.setSelectionRange(target.caretOffset, target.caretOffset);
		},
		[onRequestPlainFocus, resolveInputRef],
	);

	const commitChange = useCallback(
		(next: string, focusTarget: InlineFocusTarget) => {
			suppressContainerBlurRef.current = true;
			if (focusTarget.mode === "plain") {
				onRequestPlainFocus?.(focusTarget.caretOffset);
			} else {
				focusRestoreRef.current = focusTarget;
			}
			onChange(next);
		},
		[onChange, onRequestPlainFocus],
	);

	useLayoutEffect(() => {
		const target = focusRestoreRef.current;
		if (!target) {
			return;
		}
		focusRestoreRef.current = null;
		applyFocusTarget(target);
		suppressContainerBlurRef.current = false;
	}, [value, applyFocusTarget]);

	const handleRemoveSecret = (segmentIndex: number) => {
		const next = removeInlineSecretSegment(value, segmentIndex);
		commitChange(next, resolveInlineFocusTargetAfterUpdate(next));
	};

	const focusInput = useCallback(
		(key: string) => {
			queueMicrotask(() => {
				const input = resolveInputRef(key);
				input?.focus();
				const length = input?.value.length ?? 0;
				input?.setSelectionRange(length, length);
			});
		},
		[resolveInputRef],
	);

	const focusBadge = useCallback((key: string) => {
		queueMicrotask(() => {
			badgeClearRefs.current[key]?.focus();
		});
	}, []);

	const commitPrefixText = useCallback(
		(text: string) => {
			if (!text) {
				return;
			}
			const next = prependInlineText(value, text);
			insertTargetRef.current = { segmentIndex: 0, offset: text.length };
			commitChange(next, resolveFocusAfterPrependInlineText(next));
		},
		[commitChange, value],
	);

	const commitTrailingText = useCallback(
		(text: string) => {
			if (!text) {
				return;
			}
			const next = appendInlineText(value, text);
			insertTargetRef.current = {
				segmentIndex: Number.MAX_SAFE_INTEGER,
				offset: text.length,
			};
			commitChange(next, resolveFocusAfterAppendInlineText(next));
		},
		[commitChange, value],
	);

	const handleBadgeKeyDown = useCallback(
		(event: KeyboardEvent<HTMLButtonElement>, itemKey: string) => {
			const itemIndex = displayItems.findIndex((item) => item.key === itemKey);
			if (itemIndex < 0) {
				return;
			}

			if (event.key === "ArrowLeft") {
				event.preventDefault();
				for (let index = itemIndex - 1; index >= 0; index -= 1) {
					const previous = displayItems[index];
					if (previous.kind === "text") {
						focusInput(previous.key);
						return;
					}
					if (previous.kind === "prefix") {
						focusInput("prefix");
						return;
					}
				}
				return;
			}

			if (event.key === "ArrowRight") {
				event.preventDefault();
				for (let index = itemIndex + 1; index < displayItems.length; index += 1) {
					const next = displayItems[index];
					if (next.kind === "text" || next.kind === "trailing") {
						focusInput(next.key);
						return;
					}
				}
			}
		},
		[displayItems, focusInput],
	);

	const handlePrefixKeyDown = useCallback(
		(event: KeyboardEvent<HTMLInputElement>) => {
			const input = event.currentTarget;
			const offset = input.selectionStart ?? 0;

			if (event.key === "ArrowRight" && offset === input.value.length) {
				event.preventDefault();
				const firstItem = displayItems.find(
					(item) => item.kind !== "prefix",
				);
				if (firstItem?.kind === "text") {
					focusInput(firstItem.key);
				} else if (firstItem?.kind === "secret") {
					focusBadge(firstItem.key);
				} else if (firstItem?.kind === "trailing") {
					focusInput("trailing");
				}
				return;
			}

			if (event.nativeEvent.isComposing) {
				return;
			}

			if (event.key.length !== 1 || event.ctrlKey || event.metaKey) {
				return;
			}
			event.preventDefault();
			commitPrefixText(event.key);
		},
		[commitPrefixText, displayItems, focusBadge, focusInput],
	);

	const handlePrefixPaste = useCallback(
		(event: ClipboardEvent<HTMLInputElement>) => {
			const pasted = event.clipboardData.getData("text");
			if (!pasted) {
				return;
			}
			event.preventDefault();
			commitPrefixText(pasted);
		},
		[commitPrefixText],
	);

	const handlePrefixCompositionEnd = useCallback(
		(event: CompositionEvent<HTMLInputElement>) => {
			const composed = event.data;
			if (!composed) {
				return;
			}
			event.preventDefault();
			commitPrefixText(composed);
		},
		[commitPrefixText],
	);

	const handleTrailingKeyDown = useCallback(
		(event: KeyboardEvent<HTMLInputElement>) => {
			const input = event.currentTarget;
			const offset = input.selectionStart ?? 0;
			const end = input.selectionEnd ?? 0;

			if (event.key === "Backspace" && offset === 0 && end === 0) {
				event.preventDefault();
				const next = backspaceInlineAtEnd(value);
				if (next !== value) {
					commitChange(next, resolveInlineFocusTargetAfterUpdate(next));
				}
				return;
			}

			if (event.key === "ArrowLeft" && offset === 0) {
				event.preventDefault();
				const trailingIndex = displayItems.findIndex(
					(item) => item.kind === "trailing",
				);
				const startIndex =
					trailingIndex >= 0 ? trailingIndex - 1 : displayItems.length - 1;
				for (let index = startIndex; index >= 0; index -= 1) {
					const previous = displayItems[index];
					if (previous.kind === "text") {
						focusInput(previous.key);
						return;
					}
					if (previous.kind === "secret") {
						focusBadge(previous.key);
						return;
					}
					if (previous.kind === "prefix") {
						focusInput("prefix");
						return;
					}
				}
				return;
			}

			if (event.nativeEvent.isComposing) {
				return;
			}

			if (event.key.length !== 1 || event.ctrlKey || event.metaKey) {
				return;
			}
			event.preventDefault();
			commitTrailingText(event.key);
		},
		[commitChange, commitTrailingText, displayItems, focusBadge, focusInput, value],
	);

	const handleTrailingPaste = useCallback(
		(event: ClipboardEvent<HTMLInputElement>) => {
			const pasted = event.clipboardData.getData("text");
			if (!pasted) {
				return;
			}
			event.preventDefault();
			commitTrailingText(pasted);
		},
		[commitTrailingText],
	);

	const handleTrailingCompositionEnd = useCallback(
		(event: CompositionEvent<HTMLInputElement>) => {
			const composed = event.data;
			if (!composed) {
				return;
			}
			event.preventDefault();
			commitTrailingText(composed);
		},
		[commitTrailingText],
	);

	const handleMidTextKeyDown = useCallback(
		(
			event: KeyboardEvent<HTMLInputElement>,
			storedIndex: number,
			itemKey: string,
		) => {
			const input = event.currentTarget;
			const offset = input.selectionStart ?? 0;
			const end = input.selectionEnd ?? 0;

			if (event.key === "Backspace" && offset === 0 && end === 0) {
				const next = backspaceSecretBeforeTextBoundary(value, storedIndex);
				if (next !== null) {
					event.preventDefault();
					commitChange(
						next,
						resolveFocusAfterBackspaceAtTextBoundary(
							next,
							storedIndex,
							value,
						),
					);
					return;
				}
			}

			if (event.key === "ArrowLeft" && offset === 0) {
				event.preventDefault();
				const itemIndex = displayItems.findIndex((item) => item.key === itemKey);
				for (let index = itemIndex - 1; index >= 0; index -= 1) {
					const previous = displayItems[index];
					if (previous.kind === "text") {
						focusInput(previous.key);
						return;
					}
					if (previous.kind === "secret") {
						focusBadge(previous.key);
						return;
					}
					if (previous.kind === "prefix") {
						focusInput("prefix");
						return;
					}
				}
				return;
			}

			if (event.key === "ArrowRight" && offset === input.value.length) {
				event.preventDefault();
				const itemIndex = displayItems.findIndex((item) => item.key === itemKey);
				for (let index = itemIndex + 1; index < displayItems.length; index += 1) {
					const next = displayItems[index];
					if (next.kind === "secret") {
						focusBadge(next.key);
						return;
					}
					if (next.kind === "text" || next.kind === "trailing") {
						focusInput(next.key);
						return;
					}
				}
				focusInput("trailing");
			}
		},
		[commitChange, displayItems, focusBadge, focusInput, value],
	);

	useEffect(() => {
		if (prefixRef.current) prefixRef.current.value = "";
		if (trailingRef.current) trailingRef.current.value = "";
	}, [value]);

	const showPlaceholder = !value.trim();

	const usePairChrome = pairLayout || Boolean(pairRemove);
	const actionsPadding = usePairChrome
		? FIELD_PAIR_VALUE_ACTIONS_PR
		: FIELD_SINGLE_VALUE_ACTIONS_PR;
	const pickerPositionClass = usePairChrome
		? FIELD_PAIR_VALUE_PICKER_CLASS
		: FIELD_SINGLE_VALUE_PICKER_CLASS;

	return (
		<TooltipProvider delayDuration={200}>
			<div
				ref={containerRef}
				id={id}
				tabIndex={-1}
				onBlur={handleContainerBlur}
				className={cn(
					"group/secret-field",
					inlineFieldShellClassName,
					actionsPadding,
					className,
				)}
			>
				{showPlaceholder ? (
					<span className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">
						{placeholder}
					</span>
				) : null}

				{displayItems.map((item) => {
					if (item.kind === "prefix") {
						return (
							<input
								key={item.key}
								ref={prefixRef}
								type="text"
								defaultValue=""
								tabIndex={0}
								onKeyDown={handlePrefixKeyDown}
								onPaste={handlePrefixPaste}
								onCompositionEnd={handlePrefixCompositionEnd}
								onFocus={() => {
									insertTargetRef.current = { segmentIndex: 0, offset: 0 };
								}}
								className={prefixInputClassName}
								aria-label={t("manual.secrets.inlinePrefix", {
									defaultValue: "Text before secret",
								})}
							/>
						);
					}

					if (item.kind === "trailing") {
						return (
							<input
								key={item.key}
								ref={trailingRef}
								type="text"
								defaultValue=""
								onKeyDown={handleTrailingKeyDown}
								onPaste={handleTrailingPaste}
								onCompositionEnd={handleTrailingCompositionEnd}
								onFocus={() => {
									insertTargetRef.current = {
										segmentIndex: Number.MAX_SAFE_INTEGER,
										offset: 0,
									};
								}}
								onClick={(event) => {
									insertTargetRef.current = {
										segmentIndex: Number.MAX_SAFE_INTEGER,
										offset: event.currentTarget.selectionStart ?? 0,
									};
								}}
								onSelect={(event) => {
									insertTargetRef.current = {
										segmentIndex: Number.MAX_SAFE_INTEGER,
										offset: event.currentTarget.selectionStart ?? 0,
									};
								}}
								className={cn(chipTextInputClassName, "min-w-[2ch] flex-1 basis-0")}
								aria-label={t("manual.secrets.inlineTrailing", {
									defaultValue: "Text after secret",
								})}
							/>
						);
					}

					if (item.kind === "secret") {
						const aliasLabel = secretBadgeLabel(item.alias);
						return (
							<span key={item.key} className={secretBadgeShellClassName}>
								<Tooltip>
									<TooltipTrigger asChild>
										<span className="inline-flex min-w-0 max-w-full cursor-default">
											<Badge variant="secondary" className={secretBadgeClassName}>
												<span className={secretBadgeIconSlotClassName}>
													<KeyRound
														className={secretBadgeKeyIconClassName}
														aria-hidden
													/>
													<button
														type="button"
														ref={(node) => {
															badgeClearRefs.current[item.key] = node;
														}}
														className={secretBadgeClearButtonClassName}
														onKeyDown={(event) =>
															handleBadgeKeyDown(event, item.key)
														}
														onClick={(event) => {
															event.preventDefault();
															event.stopPropagation();
															handleRemoveSecret(item.storedIndex);
														}}
														aria-label={t("manual.secrets.clear", {
															defaultValue: "Clear secret",
														})}
													>
														<X className="h-2.5 w-2.5" strokeWidth={3} />
													</button>
												</span>
												<span className="truncate">{aliasLabel}</span>
											</Badge>
										</span>
									</TooltipTrigger>
									<TooltipContent side="top" className="max-w-sm font-mono">
										{item.alias}
									</TooltipContent>
								</Tooltip>
							</span>
						);
					}

					const isFlexible = isFlexibleInlineTextSlot(item, displayItems);

					return (
						<input
							key={item.key}
							ref={(node) => {
								inputRefs.current[item.key] = node;
							}}
							type="text"
							value={item.text}
							onChange={(event) => {
								syncInsertTarget(item.storedIndex, event.currentTarget);
								onChange(
									updateInlineSecretTextSegment(
										value,
										item.storedIndex,
										event.target.value,
									),
								);
							}}
							onFocus={(event) => syncInsertTarget(item.storedIndex, event.currentTarget)}
							onClick={(event) => syncInsertTarget(item.storedIndex, event.currentTarget)}
							onKeyUp={(event) => syncInsertTarget(item.storedIndex, event.currentTarget)}
							onSelect={(event) => syncInsertTarget(item.storedIndex, event.currentTarget)}
							onKeyDown={(event) =>
								handleMidTextKeyDown(event, item.storedIndex, item.key)
							}
							className={cn(
								chipTextInputClassName,
								isFlexible
									? "min-w-[2ch] flex-1 basis-0"
									: "min-w-[1ch] shrink-0 [field-sizing:content]" /* Chrome 123+; min-w fallback for older WebKit */,
							)}
							aria-label={t("manual.secrets.inlineText", {
								defaultValue: "Secret value text",
							})}
						/>
					);
				})}

				<div onPointerDownCapture={captureInsertTargetForPicker}>
					<SecretPickerButton
						className={cn(pickerPositionClass, pickerClassName)}
						origin={origin}
						onCreateNew={onCreateSecret}
						onSelect={handleSecretPick}
					/>
				</div>
				{pairRemove ? <PairFieldRemoveButton {...pairRemove} /> : null}
			</div>
		</TooltipProvider>
	);
}
