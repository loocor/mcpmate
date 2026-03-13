import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useMemo, useState } from "react";
import { configSuitsApi } from "../lib/api";
import { notifyError, notifySuccess } from "../lib/notify";
import type {
	ConfigSuit,
	CreateConfigSuitRequest,
	UpdateConfigSuitRequest,
} from "../lib/types";
import { Button } from "./ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Switch } from "./ui/switch";
import { Textarea } from "./ui/textarea";

interface ProfileFormDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	mode: "create" | "edit";
	suit?: ConfigSuit;
	onSuccess?: () => void;
}

export function ProfileFormDialog({
	open,
	onOpenChange,
	mode,
	suit,
	onSuccess,
}: ProfileFormDialogProps) {
	const queryClient = useQueryClient();
	const nameId = useId();
	const descriptionId = useId();

	// Form state - initialize with empty data for create mode
	const [formData, setFormData] = useState({
		name: "",
		description: "",
		suit_type: "shared",
		multi_select: true,
		priority: 50,
		is_active: false,
		is_default: false,
		clone_from_id: "",
	});

	// Reset form data when dialog opens or mode/suit changes
	useEffect(() => {
		if (open) {
			if (mode === "edit" && suit) {
				setFormData({
					name: suit.name,
					description: suit.description || "",
					suit_type: suit.suit_type,
					multi_select: suit.multi_select,
					priority: suit.priority,
					is_active: suit.is_active,
					is_default: suit.is_default,
					clone_from_id: "",
				});
			} else {
				// Create mode - reset to empty form
				setFormData({
					name: "",
					description: "",
					suit_type: "shared",
					multi_select: true,
					priority: 50,
					is_active: false,
					is_default: false,
					clone_from_id: "none",
				});
			}
		}
	}, [open, mode, suit]);

	// Fetch all suits for cloning option
	const { data: suitsResponse } = useQuery({
		queryKey: ["configSuits"],
		queryFn: configSuitsApi.getAll,
		enabled: mode === "create",
	});

	// Create mutation
	const createMutation = useMutation({
		mutationFn: (data: CreateConfigSuitRequest) =>
			configSuitsApi.createSuit(data),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess("Created", "Profile created successfully");
			onOpenChange(false);
			resetForm();
			onSuccess?.();
		},
		onError: (error) => {
			notifyError(
				"Create failed",
				`Failed to create profile: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Update mutation
	const updateMutation = useMutation({
		mutationFn: ({ id, data }: { id: string; data: UpdateConfigSuitRequest }) =>
			configSuitsApi.updateSuit(id, data),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			queryClient.invalidateQueries({ queryKey: ["configSuit", suit?.id] });
			notifySuccess("Updated", "Profile updated successfully");
			onOpenChange(false);
			onSuccess?.();
		},
		onError: (error) => {
			notifyError(
				"Update failed",
				`Failed to update profile: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const resetForm = () => {
		setFormData({
			name: "",
			description: "",
			suit_type: "shared",
			multi_select: true,
			priority: 50,
			is_active: false,
			is_default: false,
			clone_from_id: "",
		});
	};

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();

		if (!formData.name.trim()) {
			notifyError("Validation failed", "Name is required");
			return;
		}

		if (mode === "create") {
			const createData: CreateConfigSuitRequest = {
				name: formData.name,
				description: formData.description || undefined,
				suit_type: formData.suit_type,
				multi_select: formData.multi_select,
				priority: formData.priority,
				is_active: formData.is_active,
				is_default: formData.is_default,
				clone_from_id:
					formData.clone_from_id && formData.clone_from_id !== "none"
						? formData.clone_from_id
						: undefined,
			};
			createMutation.mutate(createData);
		} else if (suit) {
			const updateData: UpdateConfigSuitRequest = {
				name: formData.name !== suit.name ? formData.name : undefined,
				description:
					formData.description !== (suit.description || "")
						? formData.description
						: undefined,
				suit_type:
					formData.suit_type !== suit.suit_type
						? formData.suit_type
						: undefined,
				multi_select:
					formData.multi_select !== suit.multi_select
						? formData.multi_select
						: undefined,
				priority:
					formData.priority !== suit.priority ? formData.priority : undefined,
				is_active:
					formData.is_active !== suit.is_active
						? formData.is_active
						: undefined,
				is_default:
					formData.is_default !== suit.is_default
						? formData.is_default
						: undefined,
			};
			updateMutation.mutate({ id: suit.id, data: updateData });
		}
	};

	const handleOpenChange = (newOpen: boolean) => {
		if (!newOpen) {
			resetForm();
		}
		onOpenChange(newOpen);
	};

	const availableSuits =
		suitsResponse?.suits
			?.filter((s) => s.id !== suit?.id)
			.filter((s) => s.suit_type === "shared") || [];
	const selectedCloneProfile = useMemo(
		() =>
			formData.clone_from_id && formData.clone_from_id !== "none"
				? availableSuits.find((s) => s.id === formData.clone_from_id)
				: undefined,
		[availableSuits, formData.clone_from_id],
	);
	const isLoading = createMutation.isPending || updateMutation.isPending;

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="sm:max-w-[600px]">
				<DialogHeader>
					<DialogTitle>
						{mode === "create"
							? "Create New Configuration Suit"
							: "Edit Configuration Suit"}
					</DialogTitle>
					<DialogDescription>
						{mode === "create"
							? "Create a new configuration suit to organize your MCP servers, tools, and resources."
							: "Update the configuration suit settings."}
					</DialogDescription>
				</DialogHeader>

				<form onSubmit={handleSubmit} className="space-y-4">
					<div className="grid gap-4">
						{/* Name Field */}
						<div className="space-y-2">
							<Label htmlFor={nameId}>Name *</Label>
							<Input
								id={nameId}
								placeholder="Enter suit name"
								value={formData.name}
								onChange={(e) =>
									setFormData((prev) => ({ ...prev, name: e.target.value }))
								}
								required
							/>
						</div>

						{/* Description Field */}
						<div className="space-y-2">
							<Label htmlFor={descriptionId}>Description</Label>
							<Textarea
								id={descriptionId}
								placeholder="Enter suit description (optional)"
								className="resize-none"
								value={formData.description}
								onChange={(e) =>
									setFormData((prev) => ({
										...prev,
										description: e.target.value,
									}))
								}
							/>
						</div>

						{/* Clone From Field (Create mode only) */}
						{mode === "create" && availableSuits.length > 0 && (
							<div className="space-y-2">
								<Label htmlFor="clone_from_id">Clone From</Label>
								<Select
									value={formData.clone_from_id}
									onValueChange={(value) =>
										setFormData((prev) => ({ ...prev, clone_from_id: value }))
									}
								>
									<SelectTrigger>
										<SelectValue placeholder="None">
											{selectedCloneProfile?.name ?? "None"}
										</SelectValue>
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="none">None</SelectItem>
										{availableSuits.map((s) => {
											const rawDescription = s.description?.trim() ?? "";
											const truncatedDescription =
												rawDescription.length > 80
													? `${rawDescription.slice(0, 77).trimEnd()}…`
													: rawDescription;
											return (
												<SelectItem key={s.id} value={s.id}>
													<div className="flex w-full items-center justify-between gap-3">
														<span className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">
															{s.name}
														</span>
														{rawDescription ? (
															<span className="hidden text-xs text-muted-foreground sm:block sm:max-w-[220px] sm:truncate sm:text-right">
																{truncatedDescription}
															</span>
														) : (
															<span className="hidden text-xs text-muted-foreground sm:block sm:text-right">
																No description
															</span>
														)}
													</div>
												</SelectItem>
											);
										})}
									</SelectContent>
								</Select>
								<p className="text-xs text-muted-foreground">
									Copy servers and tools from an existing suit
								</p>
							</div>
						)}

						{/* Boolean Fields */}
						<div className="grid gap-4 md:grid-cols-2">
							<div className="flex flex-row items-center justify-between rounded-lg border p-3">
								<div className="space-y-0.5">
									<Label>Multi-select</Label>
									<p className="text-sm text-muted-foreground">
										Allow multiple selections
									</p>
								</div>
								<Switch
									checked={formData.multi_select}
									onCheckedChange={(checked) =>
										setFormData((prev) => ({ ...prev, multi_select: checked }))
									}
								/>
							</div>

							<div className="flex flex-row items-center justify-between rounded-lg border p-3">
								<div className="space-y-0.5">
									<Label>Active</Label>
									<p className="text-sm text-muted-foreground">
										Activate this suit immediately
									</p>
								</div>
								<Switch
									checked={formData.is_active}
									onCheckedChange={(checked) =>
										setFormData((prev) => ({ ...prev, is_active: checked }))
									}
								/>
							</div>

							<div className="flex flex-row items-center justify-between rounded-lg border p-3">
								<div className="space-y-0.5">
									<Label>Default</Label>
									<p className="text-sm text-muted-foreground">
										Set as default suit
									</p>
								</div>
								<Switch
									checked={formData.is_default}
									onCheckedChange={(checked) =>
										setFormData((prev) => ({ ...prev, is_default: checked }))
									}
								/>
							</div>
						</div>
					</div>

					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							onClick={() => handleOpenChange(false)}
							disabled={isLoading}
						>
							Cancel
						</Button>
						<Button type="submit" disabled={isLoading}>
							{isLoading
								? "Saving..."
								: mode === "create"
									? "Create Suit"
									: "Update Suit"}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}
