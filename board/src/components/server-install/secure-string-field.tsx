import { useCallback, useLayoutEffect, useRef, type FocusEvent } from "react";
import { useTranslation } from "react-i18next";
import {
	insertSecretPlaceholderIntoFieldValue,
	shouldUseInlineEditor,
	type InlineInsertTarget,
} from "../../lib/inline-secret-value";
import { resolveSecureFieldVariant } from "../../lib/secure-field";
import { InlineSecureStringField } from "./inline-secure-string-field";
import type { SecretOrigin } from "../../lib/types";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import { Input } from "../ui/input";
import {
	FIELD_PAIR_VALUE_ACTIONS_PR,
	FIELD_PAIR_VALUE_PICKER_CLASS,
	FIELD_SINGLE_VALUE_ACTIONS_PR,
	FIELD_SINGLE_VALUE_PICKER_CLASS,
	PairFieldRemoveButton,
	type PairFieldRemoveProps,
} from "./field-list";
import { SecretPickerButton } from "./secret-picker-button";

interface SecureStringFieldProps {
	id?: string;
	value: string;
	onChange: (value: string) => void;
	onBlur?: () => void;
	name?: string;
	headerKey?: string | null;
	placeholder?: string;
	className?: string;
	pickerClassName?: string;
	onCreateSecret?: (origin: SecretOrigin) => void;
	origin?: SecretOrigin;
	pairLayout?: boolean;
	pairRemove?: PairFieldRemoveProps;
}

const storedSecretBadgeClassName =
	"border-emerald-200 bg-emerald-50 text-emerald-800 hover:bg-emerald-100 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200 dark:hover:bg-emerald-500/20";

export function SecureStringField({
	id,
	value,
	onChange,
	onBlur,
	name,
	headerKey,
	placeholder,
	className,
	pickerClassName,
	onCreateSecret,
	origin,
	pairLayout = false,
	pairRemove,
}: SecureStringFieldProps) {
	const { t } = useTranslation("servers");
	const plainInputRef = useRef<HTMLInputElement>(null);
	const plainFieldRef = useRef<HTMLDivElement>(null);
	const plainInsertTargetRef = useRef<InlineInsertTarget | null>(null);
	const pickerInsertTargetRef = useRef<InlineInsertTarget | null>(null);
	const pendingPlainFocusRef = useRef<number | null>(null);
	const variant = resolveSecureFieldVariant(value, headerKey);
	const usePairChrome = pairLayout || Boolean(pairRemove);
	const actionsPadding = usePairChrome
		? FIELD_PAIR_VALUE_ACTIONS_PR
		: FIELD_SINGLE_VALUE_ACTIONS_PR;
	const pickerPositionClass = usePairChrome
		? FIELD_PAIR_VALUE_PICKER_CLASS
		: FIELD_SINGLE_VALUE_PICKER_CLASS;

	const handleValueChange = useCallback(
		(next: string) => {
			if (shouldUseInlineEditor(value) && !shouldUseInlineEditor(next)) {
				pendingPlainFocusRef.current ??= next.length;
			}
			onChange(next);
		},
		[onChange, value],
	);

	const handleInlinePlainFocus = useCallback((caretOffset: number) => {
		pendingPlainFocusRef.current = caretOffset;
	}, []);

	const syncPlainInsertTarget = useCallback((input: HTMLInputElement) => {
		plainInsertTargetRef.current = {
			segmentIndex: 0,
			offset: input.selectionStart ?? input.value.length,
		};
	}, []);

	const handlePlainInputBlur = useCallback(
		(event: FocusEvent<HTMLInputElement>) => {
			const next = event.relatedTarget as Node | null;
			if (next && plainFieldRef.current?.contains(next)) {
				return;
			}
			plainInsertTargetRef.current = null;
			onBlur?.();
		},
		[onBlur],
	);

	useLayoutEffect(() => {
		const caretOffset = pendingPlainFocusRef.current;
		if (caretOffset === null) {
			return;
		}
		pendingPlainFocusRef.current = null;
		const input = plainInputRef.current;
		if (!input) {
			return;
		}
		input.focus();
		input.setSelectionRange(caretOffset, caretOffset);
	}, [value]);

	const capturePlainInsertTargetForPicker = useCallback(() => {
		pickerInsertTargetRef.current = plainInsertTargetRef.current;
	}, []);

	const handleSecretPick = (placeholderValue: string) => {
		const target =
			pickerInsertTargetRef.current ?? plainInsertTargetRef.current ?? undefined;
		pickerInsertTargetRef.current = null;
		onChange(
			insertSecretPlaceholderIntoFieldValue(value, placeholderValue, {
				headerKey,
				target,
			}),
		);
	};

	const storedSecretLabel = t("manual.secrets.storedSecret", {
		defaultValue: "Stored secret",
	});

	if (shouldUseInlineEditor(value)) {
		return (
			<InlineSecureStringField
				id={id}
				value={value}
				onChange={handleValueChange}
				onRequestPlainFocus={handleInlinePlainFocus}
				onBlur={onBlur}
				placeholder={placeholder}
				className={className}
				pickerClassName={pickerClassName}
				onCreateSecret={onCreateSecret}
				origin={origin}
				pairLayout={usePairChrome}
				pairRemove={pairRemove}
			/>
		);
	}

	if (variant === "redacted" || variant === "bearer-redacted") {
		return (
			<div
				className={cn(
					"group/secret-field relative flex h-10 w-full min-w-0 items-center gap-2 rounded-md border border-input bg-muted/30 px-3 text-sm ring-offset-background",
					actionsPadding,
					className,
				)}
			>
				<Badge variant="outline" className={storedSecretBadgeClassName}>
					{storedSecretLabel}
				</Badge>
				<SecretPickerButton
					className={cn(pickerPositionClass, pickerClassName)}
					origin={origin}
					onCreateNew={onCreateSecret}
					onSelect={handleSecretPick}
					forceVisible
				/>
				{pairRemove ? <PairFieldRemoveButton {...pairRemove} /> : null}
			</div>
		);
	}

	return (
		<div
			ref={plainFieldRef}
			className={cn(
				"group/secret-field relative w-full min-w-0",
				className,
			)}
		>
			<Input
				ref={plainInputRef}
				id={id}
				name={name}
				value={value}
				onChange={(event) => onChange(event.target.value)}
				onFocus={(event) => syncPlainInsertTarget(event.currentTarget)}
				onClick={(event) => syncPlainInsertTarget(event.currentTarget)}
				onKeyUp={(event) => syncPlainInsertTarget(event.currentTarget)}
				onSelect={(event) => syncPlainInsertTarget(event.currentTarget)}
				onBlur={handlePlainInputBlur}
				placeholder={placeholder}
				className={cn("w-full", actionsPadding)}
			/>
			<div onPointerDownCapture={capturePlainInsertTargetForPicker}>
				<SecretPickerButton
					className={cn(pickerPositionClass, pickerClassName)}
					origin={origin}
					onCreateNew={onCreateSecret}
					onSelect={handleSecretPick}
				/>
			</div>
			{pairRemove ? <PairFieldRemoveButton {...pairRemove} /> : null}
		</div>
	);
}
