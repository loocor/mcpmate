import { Minus, X } from "lucide-react";
import { Button } from "../ui/button";
import { Label } from "../ui/label";

// Reusable Field List Component
interface FieldListProps {
	label: string;
	fields: Array<{ id: string; [key: string]: unknown }>;
	onRemove: (index: number) => void;
	renderField: (
		field: { id: string; [key: string]: unknown },
		index: number,
	) => React.ReactNode;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
}

export const FieldList: React.FC<FieldListProps> = ({
	label,
	fields,
	onRemove,
	renderField,
	deleteConfirmStates,
	onDeleteClick,
}) => {
	return (
		<div className="space-y-0">
			<div className="flex gap-4">
				<Label className="w-20 text-right flex items-center justify-end h-10">
					{label}
				</Label>
				<div className="flex-1 space-y-0">
					{fields.map((field, index) => (
						<div
							key={field.id}
							className="flex items-center gap-2 py-0.5 group"
						>
							<div className="flex-1 relative">
								{renderField(field, index)}
								<Button
									type="button"
									variant="ghost"
									size="icon"
									onClick={() => onDeleteClick(field.id, () => onRemove(index))}
									className={`absolute right-2 top-1/2 -translate-y-1/2 h-6 w-6 rounded-full border opacity-0 group-focus-within:opacity-100 transition-opacity ${
										deleteConfirmStates[field.id]
											? "border-red-500 bg-red-50 hover:bg-red-100"
											: "border-slate-300 hover:border-red-500 hover:bg-red-50"
									}`}
								>
									{deleteConfirmStates[field.id] ? (
										<X className="h-3 w-3" />
									) : (
										<Minus className="h-3 w-3" />
									)}
								</Button>
							</div>
						</div>
					))}
					{/* Ghost field for adding new items */}
					<div className="flex items-center gap-2 py-0.5">
						<div className="flex-1 relative">
							{renderField({ id: "ghost" }, fields.length)}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
