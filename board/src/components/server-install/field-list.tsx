import { Minus, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import { GHOST_INPUT_CLASS } from "./types";

/** Key column: content-sized within group, but capped so value always keeps room. */
export const FIELD_PAIR_GRID_CLASS =
	"grid min-w-0 grid-cols-[minmax(6rem,fit-content(14rem))_minmax(0,1fr)] gap-x-3 gap-y-1.5";
export const FIELD_PAIR_ROW_CLASS =
	"col-span-2 grid min-w-0 grid-cols-subgrid items-center py-0";

/** Shared left label column for server install / edit drawer form rows. */
export const SERVER_INSTALL_FORM_ROW_LABEL_CLASS =
	"flex min-h-9 w-[84px] shrink-0 items-center justify-end text-right leading-tight";

/** FieldList section labels (multi-row lists such as env / headers). */
export const FIELD_LIST_LABEL_CLASS = cn(
	"self-start",
	SERVER_INSTALL_FORM_ROW_LABEL_CLASS,
);

export const FIELD_PAIR_KEY_INPUT_CLASS = "min-w-0 w-full";
export const FIELD_PAIR_VALUE_CELL_CLASS = "min-w-0 w-full";

/** Room inside value field for secret picker + row remove control (matches args `pr-20`). */
export const FIELD_PAIR_VALUE_ACTIONS_PR = "pr-20";

export const FIELD_PAIR_VALUE_PICKER_CLASS =
	"absolute right-9 top-1/2 h-7 w-7 -translate-y-1/2";

/** Standalone value fields (command, url) — picker only, no row remove. */
export const FIELD_SINGLE_VALUE_ACTIONS_PR = "pr-10";

export const FIELD_SINGLE_VALUE_PICKER_CLASS =
	"absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2";

export interface PairFieldRemoveProps {
	onClick: () => void;
	confirmed?: boolean;
}

export function PairFieldRemoveButton({
	onClick,
	confirmed = false,
}: PairFieldRemoveProps) {
	const { t } = useTranslation("servers");

	return (
		<Button
			type="button"
			variant="ghost"
			size="icon"
			onClick={onClick}
			className={cn(
				"absolute right-2 top-1/2 z-10 h-6 w-6 -translate-y-1/2 rounded-full border transition-opacity",
				confirmed
					? "border-red-500 bg-red-50 opacity-100 hover:bg-red-100"
					: "border-slate-300 opacity-0 hover:border-red-500 hover:bg-red-50 group-focus-within/secret-field:opacity-100",
			)}
			aria-label={
				confirmed
					? t("manual.fields.common.confirmRemoveRow", {
						defaultValue: "Confirm remove",
					})
					: t("manual.fields.common.removeRow", {
						defaultValue: "Remove row",
					})
			}
		>
			{confirmed ? <X className="h-3 w-3" /> : <Minus className="h-3 w-3" />}
		</Button>
	);
}

interface PairGhostRowProps {
	keyPlaceholder: string;
	valuePlaceholder: string;
	onAdd: () => void;
}

/** Single click target for key/value ghost rows — avoids duplicate appends. */
export function PairGhostRow({
	keyPlaceholder,
	valuePlaceholder,
	onAdd,
}: PairGhostRowProps) {
	const { t } = useTranslation("servers");
	return (
		<div
			role="button"
			tabIndex={0}
			aria-label={t("manual.fields.common.addRow", { defaultValue: "Add row" })}
			className="col-span-2 grid min-w-0 cursor-pointer grid-cols-subgrid items-center"
			onClick={onAdd}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					onAdd();
				}
			}}
		>
			<Input
				readOnly
				tabIndex={-1}
				placeholder={keyPlaceholder}
				className={cn(
					GHOST_INPUT_CLASS,
					FIELD_PAIR_KEY_INPUT_CLASS,
					"pointer-events-none",
				)}
			/>
			<Input
				readOnly
				tabIndex={-1}
				placeholder={valuePlaceholder}
				className={cn(
					GHOST_INPUT_CLASS,
					FIELD_PAIR_VALUE_CELL_CLASS,
					"pointer-events-none",
				)}
			/>
		</div>
	);
}

// Reusable Field List Component
interface FieldListProps {
	label: string;
	labelTooltip?: string;
	labelClassName?: string;
	fields: Array<{ id: string;[key: string]: unknown }>;
	onRemove: (index: number) => void;
	renderField: (
		field: { id: string;[key: string]: unknown },
		index: number,
	) => React.ReactNode;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	/** Key/value rows share one auto-sized key column across the list. */
	pairLayout?: boolean;
	/** When true, row remove controls live inside `renderField` (e.g. SecureStringField). */
	embeddedRowActions?: boolean;
}

export const FieldList: React.FC<FieldListProps> = ({
	label,
	labelTooltip,
	labelClassName,
	fields,
	onRemove,
	renderField,
	deleteConfirmStates,
	onDeleteClick,
	pairLayout = false,
	embeddedRowActions = false,
}) => {
	const deleteButtonClassName = (fieldId: string) =>
		cn(
			"absolute right-2 top-1/2 h-6 w-6 -translate-y-1/2 rounded-full border opacity-0 transition-opacity group-focus-within:opacity-100",
			deleteConfirmStates[fieldId]
				? "border-red-500 bg-red-50 hover:bg-red-100"
				: "border-slate-300 hover:border-red-500 hover:bg-red-50",
		);

	const labelNode = labelTooltip ? (
		<TooltipProvider delayDuration={200}>
			<Tooltip>
				<TooltipTrigger asChild>
					<Label
						className={cn(
							"cursor-help select-none",
							FIELD_LIST_LABEL_CLASS,
							labelClassName,
						)}
					>
						{label}
					</Label>
				</TooltipTrigger>
				<TooltipContent
					side="top"
					align="start"
					className="max-w-xs leading-relaxed"
				>
					{labelTooltip}
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	) : (
		<Label
			className={cn(FIELD_LIST_LABEL_CLASS, labelClassName)}
		>
			{label}
		</Label>
	);

	return (
		<div className="space-y-0">
			<div className="flex items-start gap-3">
				{labelNode}
				{pairLayout ? (
					<div className={cn("min-w-0 flex-1", FIELD_PAIR_GRID_CLASS)}>
						{fields.map((field, index) => (
							<div key={field.id} className={FIELD_PAIR_ROW_CLASS}>
								{renderField(field, index)}
							</div>
						))}
						<div className={FIELD_PAIR_ROW_CLASS}>
							{renderField({ id: "ghost" }, fields.length)}
						</div>
					</div>
				) : (
					<div className="flex min-w-0 flex-1 flex-col gap-y-1.5">
						{fields.map((field, index) => (
							<div
								key={field.id}
								className="group flex items-center gap-2 py-0"
							>
								<div className="relative min-w-0 flex-1">
									{renderField(field, index)}
									{embeddedRowActions ? null : (
										<Button
											type="button"
											variant="ghost"
											size="icon"
											onClick={() =>
												onDeleteClick(field.id, () => onRemove(index))
											}
											className={deleteButtonClassName(field.id)}
										>
											{deleteConfirmStates[field.id] ? (
												<X className="h-3 w-3" />
											) : (
												<Minus className="h-3 w-3" />
											)}
										</Button>
									)}
								</div>
							</div>
						))}
						<div className="flex items-center gap-2 py-0">
							<div className="relative min-w-0 flex-1">
								{renderField({ id: "ghost" }, fields.length)}
							</div>
						</div>
					</div>
				)}
			</div>
		</div>
	);
};
