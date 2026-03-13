import React from "react";
import { Info } from "lucide-react";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Textarea } from "./ui/textarea";
import { Switch } from "./ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "./ui/tooltip";
import type { JsonObject, JsonSchema, JsonValue } from "../types/json";
import { defaultFromSchema } from "./schema-form-utils";

type ArrayItemRowProps = {
	name: string;
	idx: number;
	value: JsonValue | undefined;
	onChange: (value: JsonValue | undefined) => void;
	onRemove: () => void;
};

// Array item row component with 2-click delete confirmation
const ArrayItemRow: React.FC<ArrayItemRowProps> = ({
	name,
	idx,
	value,
	onChange,
	onRemove,
}) => {
	const [confirming, setConfirming] = React.useState(false);
	const label = `${name}[${idx}]`;

	return (
		<div className="group grid grid-cols-[minmax(0,1fr)_minmax(0,5fr)] items-center gap-2">
			<div className="text-xs text-slate-500">{label}</div>
			<div className="relative">
				<Input
					value={value ?? ""}
					onChange={(e) => onChange(e.target.value)}
					className="w-full"
				/>
				<button
					type="button"
					aria-label="Remove"
					className="absolute right-1 top-1/2 -translate-y-1/2 h-6 w-6 rounded-full border text-xs flex items-center justify-center opacity-0 group-focus-within:opacity-100 hover:bg-accent transition"
					onClick={(e) => {
						e.preventDefault();
						e.stopPropagation();
						if (!confirming) {
							setConfirming(true);
							return;
						}
						onRemove();
					}}
				>
					{confirming ? "×" : "−"}
				</button>
			</div>
		</div>
	);
};

const isJsonObject = (value: unknown): value is JsonObject =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toSingleSchema = (
	schema: JsonSchema | JsonSchema[] | undefined,
): JsonSchema =>
	Array.isArray(schema) ? schema[0] : schema ?? {};

type FieldProps = {
	name: string;
	schema: JsonSchema;
	required?: boolean;
	value: JsonValue | undefined;
	onChange: (v: JsonValue | undefined) => void;
};

function sanitizeId(input: string) {
	return input.replace(/[^a-zA-Z0-9_-]/g, "-");
}

function Field({ name, schema, required, value, onChange }: FieldProps) {
	const type = Array.isArray(schema?.type) ? schema.type[0] : schema?.type;
	const enumVals = Array.isArray(schema?.enum) ? schema.enum : undefined;
	const labelText = `${name}${required ? " *" : ""}`;
	const desc = schema?.description as string | undefined;
	const fieldId = `schema-${sanitizeId(name)}`;
	const labelId = `${fieldId}-label`;

	const labelNode = desc ? (
		<TooltipProvider delayDuration={200}>
			<Tooltip>
				<TooltipTrigger asChild>
					<Label
						id={labelId}
						htmlFor={fieldId}
						className="flex cursor-help select-none items-center gap-1 text-sm font-medium text-slate-600 dark:text-slate-200"
					>
						<span>{labelText}</span>
						<Info className="h-2.5 w-2.5 relative -top-0.5" />
					</Label>
				</TooltipTrigger>
				<TooltipContent
					side="top"
					align="start"
					className="max-w-xs leading-relaxed"
				>
					{desc}
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	) : (
		<Label
			id={labelId}
			htmlFor={fieldId}
			className="select-none text-sm font-medium text-slate-600 dark:text-slate-200"
		>
			{labelText}
		</Label>
	);

	const renderField = (control: React.ReactNode) => (
		<div className="grid grid-cols-[minmax(140px,200px)_1fr] items-start gap-3">
			<div className="pt-2">{labelNode}</div>
			<div className="space-y-2" aria-labelledby={labelId}>
				{control}
			</div>
		</div>
	);

	if (enumVals && enumVals.length) {
		return renderField(
			<Select
				value={String(value ?? enumVals[0])}
				onValueChange={(v) => onChange(v)}
			>
				<SelectTrigger id={fieldId} aria-labelledby={labelId}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					{enumVals.map((e, i) => (
						<SelectItem key={`${name}-${i}`} value={String(e)}>
							{String(e)}
						</SelectItem>
					))}
				</SelectContent>
			</Select>,
		);
	}

	if (type === "boolean") {
		return renderField(
			<div className="flex items-center gap-2">
				<Switch
					id={fieldId}
					aria-labelledby={labelId}
					checked={!!value}
					onCheckedChange={(v) => onChange(v)}
				/>
				<span className="text-xs text-slate-500">{String(value)}</span>
			</div>,
		);
	}

	if (type === "integer" || type === "number") {
		return renderField(
			<Input
				id={fieldId}
				type="number"
				value={value ?? ""}
				onChange={(e) =>
					onChange(
						e.target.value === ""
							? undefined
							: type === "integer"
								? parseInt(e.target.value)
								: parseFloat(e.target.value),
					)
				}
			/>,
		);
	}

	if (type === "array") {
		const itemsSchema = toSingleSchema(schema?.items) ?? { type: "string" };
		const arr = Array.isArray(value) ? (value as JsonValue[]) : [];

		return renderField(
			<div className="space-y-2">
				{arr.map((v, idx) => (
					<ArrayItemRow
						key={`${name}-${idx}`}
						name={name}
						idx={idx}
						value={v}
						onChange={(newValue) => {
							const next = [...arr];
							next[idx] = newValue;
							onChange(next);
						}}
						onRemove={() => {
							const next = arr.filter((_, i) => i !== idx);
							onChange(next);
						}}
					/>
				))}
				{/* Ghost field as add button - shows the next label and dashed input */}
				<div className="grid grid-cols-[minmax(0,1fr)_minmax(0,5fr)] items-center gap-2">
					<div className="text-xs text-slate-400">{`${name}[${arr.length}]`}</div>
					<button
						type="button"
						className="text-left w-full px-3 py-2 text-sm border border-dashed border-gray-300 rounded-md bg-transparent text-slate-500 hover:bg-accent/30"
						onClick={() => {
							const next = [...(arr || []), defaultFromSchema(itemsSchema)];
							onChange(next);
						}}
					>
						Add new item...
					</button>
				</div>
			</div>,
		);
	}

	if (type === "object") {
		const props = schema?.properties || {};
		const req: string[] = Array.isArray(schema?.required)
			? schema.required
			: [];
		const obj = isJsonObject(value) ? value : {};
		const keys = Object.keys(props);
		return renderField(
			<div className="grid grid-cols-1 gap-3">
				{keys.map((k) => (
					<Field
						key={k}
						name={k}
						schema={props[k]}
						required={req.includes(k)}
						value={obj[k]}
						onChange={(nv) => onChange({ ...obj, [k]: nv })}
					/>
				))}
			</div>,
		);
	}

	// string and fallback
	return renderField(
		schema?.format === "textarea" ? (
			<Textarea
				id={fieldId}
				rows={4}
				value={value ?? ""}
				onChange={(e) => onChange(e.target.value)}
			/>
		) : (
			<Input
				id={fieldId}
				value={value ?? ""}
				onChange={(e) => onChange(e.target.value)}
			/>
		),
	);
}

export function SchemaForm({
	schema,
	value,
	onChange,
}: {
	schema: JsonSchema;
	value: JsonValue | undefined;
	onChange: (v: JsonValue | undefined) => void;
}) {
	const props = schema?.properties || {};
	const req: string[] = Array.isArray(schema?.required) ? schema.required : [];
	const keys = Object.keys(props);
	const obj = isJsonObject(value) ? value : {};
	return (
		<div className="grid grid-cols-1 gap-3">
			{keys.map((k) => (
				<Field
					key={k}
					name={k}
					schema={props[k]}
					required={req.includes(k)}
					value={obj[k]}
					onChange={(nv) => onChange({ ...obj, [k]: nv })}
				/>
			))}
		</div>
	);
}
