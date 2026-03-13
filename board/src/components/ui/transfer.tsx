import { Info } from "lucide-react";
import type React from "react";
import { useMemo, useState } from "react";
import { Badge } from "./badge";
import { Button } from "./button";
import { Input } from "./input";

export interface TransferItem {
	id: string;
	name: string;
	description?: string;
	type?: string;
	status?: string;
	disabled?: boolean;
}

export interface TransferProps {
	dataSource: TransferItem[];
	targetKeys: string[];
	onChange: (
		targetKeys: string[],
		direction: "left" | "right",
		moveKeys: string[],
	) => void;
	onItemInfo?: (item: TransferItem) => void;
	leftTitle?: string;
	rightTitle?: string;
	searchPlaceholder?: string;
	emptyText?: string;
	className?: string;
	disabled?: boolean;
	loading?: boolean;
}

interface TransferPanelProps {
	title: string;
	items: TransferItem[];
	selectedKeys: string[];
	onSelectChange: (keys: string[]) => void;
	onItemInfo?: (item: TransferItem) => void;
	searchValue: string;
	onSearchChange: (value: string) => void;
	searchPlaceholder: string;
	emptyText: string;
	disabled?: boolean;
	loading?: boolean;
	// New props for direct move functionality
	onDirectMove?: (itemId: string) => void;
	panelType: "left" | "right";
}

const TransferPanel: React.FC<TransferPanelProps> = ({
	title,
	items,
	selectedKeys,
	onSelectChange,
	onItemInfo,
	searchValue,
	onSearchChange,
	searchPlaceholder,
	emptyText,
	disabled = false,
	loading = false,
	onDirectMove,
	panelType: _panelType,
}) => {
	const handleSelectAll = (checked: boolean) => {
		if (checked) {
			const allKeys = items
				.filter((item) => !item.disabled)
				.map((item) => item.id);
			onSelectChange(allKeys);
		} else {
			onSelectChange([]);
		}
	};

	const handleItemSelect = (itemId: string, checked: boolean) => {
		if (checked) {
			onSelectChange([...selectedKeys, itemId]);
		} else {
			onSelectChange(selectedKeys.filter((key) => key !== itemId));
		}
	};

	const handleItemClick = (itemId: string) => {
		// If direct move is enabled, move the item directly
		if (onDirectMove) {
			onDirectMove(itemId);
		} else {
			// Fallback to selection behavior
			const isSelected = selectedKeys.includes(itemId);
			handleItemSelect(itemId, !isSelected);
		}
	};

	const selectableItems = items.filter((item) => !item.disabled);
	const isAllSelected =
		selectableItems.length > 0 &&
		selectableItems.every((item) => selectedKeys.includes(item.id));
	const isIndeterminate = selectedKeys.length > 0 && !isAllSelected;

	return (
		<div className="flex flex-1 flex-col rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950">
			{/* Header */}
			<div className="flex items-center justify-between border-b border-slate-200 p-3 dark:border-slate-800">
				<div className="flex items-center gap-2">
					<input
						type="checkbox"
						checked={isAllSelected}
						ref={(el) => {
							if (el) el.indeterminate = isIndeterminate;
						}}
						onChange={(e) => handleSelectAll(e.target.checked)}
						disabled={disabled || loading || selectableItems.length === 0}
						className="rounded border-slate-300 text-primary focus:ring-primary"
					/>
					<span className="text-sm font-medium">
						{title} ({selectedKeys.length}/{items.length})
					</span>
				</div>
			</div>

			{/* Search */}
			<div className="p-3 pb-2">
				<Input
					placeholder={searchPlaceholder}
					value={searchValue}
					onChange={(e) => onSearchChange(e.target.value)}
					disabled={disabled || loading}
					className="h-8"
				/>
			</div>

			{/* List */}
			<div className="flex-1 overflow-auto">
				{loading ? (
					<div className="flex h-full items-center justify-center">
						<span className="text-sm text-muted-foreground">Loading...</span>
					</div>
				) : items.length === 0 ? (
					<div className="flex h-full items-center justify-center">
						<span className="text-sm text-muted-foreground">{emptyText}</span>
					</div>
				) : (
					<div>
						{items.map((item, index) => {
							const isSelected = selectedKeys.includes(item.id);
							const isEven = index % 2 === 0;

							return (
								<button
									key={item.id}
									type="button"
									className={`flex items-center gap-2 p-2 w-full text-left cursor-pointer transition-colors border-b ${
										isEven
											? "bg-white dark:bg-slate-950"
											: "bg-slate-50 dark:bg-slate-900/30"
									} hover:bg-slate-100 dark:hover:bg-slate-800/50 ${
										isSelected
											? "ring-primary/50 bg-primary/5 border-primary/10"
											: "border-slate-100 dark:border-slate-800/30"
									} ${index === items.length - 1 ? "border-b-0" : ""}`}
									onClick={() => handleItemClick(item.id)}
									disabled={disabled || loading || item.disabled}
								>
									<input
										type="checkbox"
										checked={isSelected}
										onChange={(e) => {
											e.stopPropagation();
											handleItemSelect(item.id, e.target.checked);
										}}
										disabled={disabled || loading || item.disabled}
										className="rounded border-slate-300 text-primary focus:ring-primary"
									/>
									<div className="min-w-0 flex-1">
										<div className="flex items-center gap-2">
											<span className="truncate text-sm font-medium">
												{item.name}
											</span>
											{item.type && (
												<Badge
													variant="secondary"
													className="text-[10px] uppercase"
												>
													{item.type}
												</Badge>
											)}
										</div>
										{item.description && (
											<p className="truncate text-xs text-muted-foreground">
												{item.description}
											</p>
										)}
									</div>
									{onItemInfo && (
										<span
											role="button"
											tabIndex={disabled || loading ? -1 : 0}
											aria-label="View server details"
											className={`flex h-6 w-6 items-center justify-center rounded-md text-slate-400 transition-colors hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
												disabled || loading ? "pointer-events-none opacity-40" : "cursor-pointer"
											}`}
											onClick={(e) => {
												e.stopPropagation();
												if (!disabled && !loading) {
													onItemInfo(item);
												}
											}}
											onKeyDown={(e) => {
												if (disabled || loading) return;
												if (e.key === "Enter" || e.key === " ") {
													e.preventDefault();
													e.stopPropagation();
													onItemInfo(item);
												}
											}}
										>
											<Info className="h-3 w-3" />
										</span>
									)}
								</button>
							);
						})}
					</div>
				)}
			</div>
		</div>
	);
};

export const Transfer: React.FC<TransferProps> = ({
	dataSource,
	targetKeys,
	onChange,
	onItemInfo,
	leftTitle = "Available",
	rightTitle = "Selected",
	searchPlaceholder = "Search...",
	emptyText = "No data",
	className = "",
	disabled = false,
	loading = false,
}) => {
	const [leftSearchValue, setLeftSearchValue] = useState("");
	const [rightSearchValue, setRightSearchValue] = useState("");
	const [leftSelectedKeys, setLeftSelectedKeys] = useState<string[]>([]);
	const [rightSelectedKeys, setRightSelectedKeys] = useState<string[]>([]);

	const { leftItems, rightItems } = useMemo(() => {
		const targetKeySet = new Set(targetKeys);
		const left = dataSource.filter((item) => !targetKeySet.has(item.id));
		const right = dataSource.filter((item) => targetKeySet.has(item.id));

		return {
			leftItems: left.filter(
				(item) =>
					leftSearchValue === "" ||
					item.name.toLowerCase().includes(leftSearchValue.toLowerCase()) ||
					item.description
						?.toLowerCase()
						.includes(leftSearchValue.toLowerCase()) ||
					item.type?.toLowerCase().includes(leftSearchValue.toLowerCase()),
			),
			rightItems: right.filter(
				(item) =>
					rightSearchValue === "" ||
					item.name.toLowerCase().includes(rightSearchValue.toLowerCase()) ||
					item.description
						?.toLowerCase()
						.includes(rightSearchValue.toLowerCase()) ||
					item.type?.toLowerCase().includes(rightSearchValue.toLowerCase()),
			),
		};
	}, [dataSource, targetKeys, leftSearchValue, rightSearchValue]);

	const handleMoveToRight = () => {
		const newTargetKeys = [...targetKeys, ...leftSelectedKeys];
		onChange(newTargetKeys, "right", leftSelectedKeys);
		setLeftSelectedKeys([]);
	};

	const handleMoveToLeft = () => {
		const newTargetKeys = targetKeys.filter(
			(key) => !rightSelectedKeys.includes(key),
		);
		onChange(newTargetKeys, "left", rightSelectedKeys);
		setRightSelectedKeys([]);
	};

	// Handle direct move from left panel (move to right)
	const handleDirectMoveToRight = (itemId: string) => {
		const newTargetKeys = [...targetKeys, itemId];
		onChange(newTargetKeys, "right", [itemId]);
	};

	// Handle direct move from right panel (move to left)
	const handleDirectMoveToLeft = (itemId: string) => {
		const newTargetKeys = targetKeys.filter((key) => key !== itemId);
		onChange(newTargetKeys, "left", [itemId]);
	};

	return (
		<div className={`flex items-stretch gap-2 h-full ${className}`}>
			<TransferPanel
				title={leftTitle}
				items={leftItems}
				selectedKeys={leftSelectedKeys}
				onSelectChange={setLeftSelectedKeys}
				onItemInfo={onItemInfo}
				searchValue={leftSearchValue}
				onSearchChange={setLeftSearchValue}
				searchPlaceholder={searchPlaceholder}
				emptyText={emptyText}
				disabled={disabled}
				loading={loading}
				onDirectMove={handleDirectMoveToRight}
				panelType="left"
			/>

			<div className="flex flex-col justify-center gap-2 px-1">
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={handleMoveToRight}
					disabled={disabled || loading || leftSelectedKeys.length === 0}
					className="h-8 w-8 p-0"
				>
					→
				</Button>
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={handleMoveToLeft}
					disabled={disabled || loading || rightSelectedKeys.length === 0}
					className="h-8 w-8 p-0"
				>
					←
				</Button>
			</div>

			<TransferPanel
				title={rightTitle}
				items={rightItems}
				selectedKeys={rightSelectedKeys}
				onSelectChange={setRightSelectedKeys}
				onItemInfo={onItemInfo}
				searchValue={rightSearchValue}
				onSearchChange={setRightSearchValue}
				searchPlaceholder={searchPlaceholder}
				emptyText={emptyText}
				disabled={disabled}
				loading={loading}
				onDirectMove={handleDirectMoveToLeft}
				panelType="right"
			/>
		</div>
	);
};
