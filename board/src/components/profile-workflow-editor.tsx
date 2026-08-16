import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, Plus, ShieldCheck, Trash2, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { configSuitsApi } from "../lib/api";
import { notifyError, notifySuccess } from "../lib/notify";
import {
	serializeWorkflowSteps,
	withSingleWorkflowCapabilityBinding,
	workflowDraftFromSpecification,
	type WorkflowCapabilityOption,
	type WorkflowStepDraft,
} from "../lib/profile-workflow-specification";
import type { WorkflowBindingPolicy } from "../lib/types";
import CapabilityCombobox from "./capability-combobox";
import {
	ProfileSurfaceMetrics,
	type ProfileSurfaceMetric,
} from "./profile-surface-metrics";
import { ResizableSplitPane } from "./resizable-split-pane";
import { CardListScrollBody } from "./card-list-scroll-body";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import { Button } from "./ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "./ui/card";
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
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "./ui/alert-dialog";

const emptyStep = (): WorkflowStepDraft => ({
	title: "",
	description: "",
	bindings: [],
});

interface ProfileWorkflowEditorProps {
	profileId: string;
	capabilities: WorkflowCapabilityOption[];
	capabilitiesLoading?: boolean;
	capabilityMetrics: ProfileSurfaceMetric[];
}

export function ProfileWorkflowEditor({
	profileId,
	capabilities,
	capabilitiesLoading,
	capabilityMetrics,
}: ProfileWorkflowEditorProps) {
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const [steps, setSteps] = useState<WorkflowStepDraft[]>([]);
	const [selectedStepIndex, setSelectedStepIndex] = useState<number | null>(
		null,
	);
	const [draggedStepIndex, setDraggedStepIndex] = useState<number | null>(null);
	const [pendingRemovalStepIndex, setPendingRemovalStepIndex] = useState<
		number | null
	>(null);

	const specificationQuery = useQuery({
		queryKey: ["workflowSpecification", profileId],
		queryFn: () => configSuitsApi.getWorkflowSpecification(profileId),
		retry: false,
	});

	useEffect(() => {
		if (specificationQuery.data) {
			const nextSteps = workflowDraftFromSpecification(
				specificationQuery.data,
			);
			setSteps(nextSteps);
			setSelectedStepIndex(nextSteps.length > 0 ? 0 : null);
		}
	}, [specificationQuery.data]);

	const saveMutation = useMutation({
		mutationFn: (nextSteps: WorkflowStepDraft[]) =>
			configSuitsApi.saveWorkflowSpecification({
				profile_id: profileId,
				expected_specification_revision:
					specificationQuery.data?.specification_revision ?? null,
				validation_notes: specificationQuery.data?.validation_notes ?? null,
				avoid_rules: specificationQuery.data?.avoid_rules ?? null,
				steps: serializeWorkflowSteps(nextSteps),
			}),
			onSuccess: (specification) => {
			queryClient.setQueryData(
				["workflowSpecification", profileId],
				specification,
			);
			notifySuccess(
				t("profiles:detail.workflow.messages.saved"),
				t("profiles:detail.workflow.messages.savedDescription"),
			);
		},
		onError: (error: Error) =>
			notifyError(
				t("profiles:detail.workflow.messages.saveFailed"),
				error.message,
			),
	});

	const selectedStep =
		selectedStepIndex === null ? null : (steps[selectedStepIndex] ?? null);
	const selectedStepBinding = selectedStep?.bindings[0] ?? null;
	const workflowSurfaceMetrics: ProfileSurfaceMetric[] = [
		{
			id: "steps",
			label: t("profiles:detail.workflow.stepsMetric"),
			value: String(steps.length),
			description: t("profiles:detail.workflow.stepsMetricDescription"),
		},
		...capabilityMetrics,
	];
	const updateStep = (
		index: number,
		update: (step: WorkflowStepDraft) => WorkflowStepDraft,
	) =>
		setSteps((current) =>
			current.map((step, itemIndex) =>
				itemIndex === index ? update(step) : step,
			),
		);
	const addStep = () => {
		const nextIndex = steps.length;
		setSteps((current) => [...current, emptyStep()]);
		setSelectedStepIndex(nextIndex);
	};
	const moveStepTo = (sourceIndex: number, destinationIndex: number) => {
		if (
			sourceIndex === destinationIndex ||
			sourceIndex < 0 ||
			destinationIndex < 0 ||
			sourceIndex >= steps.length ||
			destinationIndex >= steps.length
		) {
			return;
		}
		setSteps((current) => {
			const next = [...current];
			const [movedStep] = next.splice(sourceIndex, 1);
			next.splice(destinationIndex, 0, movedStep);
			return next;
		});
		setSelectedStepIndex((current) => {
			if (current === sourceIndex) return destinationIndex;
			if (
				sourceIndex < destinationIndex &&
				current !== null &&
				current > sourceIndex &&
				current <= destinationIndex
			) {
				return current - 1;
			}
			if (
				sourceIndex > destinationIndex &&
				current !== null &&
				current >= destinationIndex &&
				current < sourceIndex
			) {
				return current + 1;
			}
			return current;
		});
	};
	const moveStep = (index: number, direction: -1 | 1) =>
		moveStepTo(index, index + direction);
	const removeStep = (index: number) => {
		if (steps.length === 1) {
			saveMutation.mutate([], {
				onSuccess: () => {
					setSteps([]);
					setSelectedStepIndex(null);
				},
			});
			return;
		}

		const nextLength = steps.length - 1;
		setSteps((current) =>
			current.filter((_, itemIndex) => itemIndex !== index),
		);
		setSelectedStepIndex((current) => {
			if (nextLength === 0 || current === null) return null;
			if (current === index) return Math.min(index, nextLength - 1);
			return current > index ? current - 1 : current;
		});
	};
	const updateStepBinding = (index: number, refId: string | null) => {
		updateStep(index, (current) =>
			withSingleWorkflowCapabilityBinding(current, refId),
		);
	};

	if (specificationQuery.isLoading) {
		return (
			<p className="text-sm text-muted-foreground">
				{t("profiles:detail.workflow.loading")}
			</p>
		);
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4">
			<ProfileSurfaceMetrics
				metrics={workflowSurfaceMetrics}
				description={t("profiles:detail.overview.enabledAvailable", {
					defaultValue: "enabled / available",
				})}
				className="shrink-0"
			/>
			<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
				<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
					<ResizableSplitPane
						dividerAriaLabel={t("profiles:detail.workflow.resizeStepColumns")}
					>
						<div className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<div className="min-h-16 shrink-0 p-3">
								<div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
									{t("profiles:detail.workflow.stepsTitle")}
								</div>
								<CardDescription className="truncate text-xs text-slate-500 dark:text-slate-400">
									{t("profiles:detail.workflow.stepsListDescription")}
								</CardDescription>
							</div>
							<CardListScrollBody className="mx-3 mb-3 mt-0">
								<CapsuleStripeList className="rounded-none border-0 overflow-visible">
									{steps.map((step, index) => {
										const isSelected = selectedStepIndex === index;
										return (
											<CapsuleStripeListItem
													key={index}
													className={`group relative px-3 transition-colors ${
														isSelected
															? "bg-primary/10"
															: "hover:bg-accent/50"
													}`}
													draggable
													onDragEnd={() => setDraggedStepIndex(null)}
													onDragOver={(event) => {
														if (draggedStepIndex === null || draggedStepIndex === index) return;
														event.preventDefault();
														event.dataTransfer.dropEffect = "move";
													}}
													onDragStart={(event) => {
														setDraggedStepIndex(index);
														event.dataTransfer.effectAllowed = "move";
														event.dataTransfer.setData("text/plain", String(index));
													}}
													onDrop={(event) => {
														event.preventDefault();
														if (draggedStepIndex !== null) {
															moveStepTo(draggedStepIndex, index);
														}
														setDraggedStepIndex(null);
													}}
											>
												<button
													type="button"
													aria-current={isSelected ? "true" : undefined}
													className="flex min-w-0 flex-1 items-center gap-3 text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
													onClick={() => setSelectedStepIndex(index)}
												>
													<span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-slate-200 bg-white text-sm font-semibold text-slate-600 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
														{index + 1}
													</span>
													<span className="min-w-0">
														<span className="block truncate font-medium text-slate-900 dark:text-slate-100">
															{step.title || t("profiles:detail.workflow.untitledStep")}
														</span>
														{step.description ? (
															<span
																className="mt-1 block truncate text-xs text-slate-500"
																title={step.description}
															>
																{step.description}
															</span>
														) : null}
													</span>
												</button>
												<div className="-mr-1 ml-auto flex shrink-0 gap-px opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
													{index > 0 ? (
														<Button
														type="button"
														size="icon"
														variant="ghost"
														className="h-7 w-7 bg-transparent text-muted-foreground shadow-none hover:bg-transparent hover:text-foreground"
														aria-label={t("profiles:detail.workflow.moveUp")}
														onClick={() => moveStep(index, -1)}
													>
														<ArrowUp className="h-3.5 w-3.5" />
														</Button>
													) : null}
													{index < steps.length - 1 ? (
														<Button
														type="button"
														size="icon"
														variant="ghost"
														className="h-7 w-7 bg-transparent text-muted-foreground shadow-none hover:bg-transparent hover:text-foreground"
														aria-label={t("profiles:detail.workflow.moveDown")}
														onClick={() => moveStep(index, 1)}
													>
														<ArrowDown className="h-3.5 w-3.5" />
														</Button>
													) : null}
												</div>
											</CapsuleStripeListItem>
										);
									})}
								</CapsuleStripeList>
									<Button
										type="button"
										variant="outline"
										className="mx-2 mt-2 w-[calc(100%-1rem)] border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700"
										onClick={addStep}
									>
										<Plus className="mr-2 h-4 w-4" />
										{t("profiles:detail.workflow.addStep")}
									</Button>
							</CardListScrollBody>
						</div>
						<div className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<div className="min-h-[62px] shrink-0 p-3">
							<div className="min-w-0">
									<div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
										{selectedStepIndex === null
											? t("profiles:detail.workflow.stepSettingsTitle")
											: t("profiles:detail.workflow.step", {
													number: selectedStepIndex + 1,
												})}
									</div>
								<CardDescription className="truncate text-xs text-slate-500 dark:text-slate-400">
										{selectedStepIndex === null
											? t("profiles:detail.workflow.stepSettingsDescription")
											: t("profiles:detail.workflow.stepDescription")}
								</CardDescription>
								</div>
							</div>
							<div className="min-h-0 flex-1 overflow-y-auto px-3 pb-3 pt-1">
								{selectedStep && selectedStepIndex !== null ? (
									<div className="grid gap-4">
											<div className="flex items-start gap-4">
											<Label className="w-20 shrink-0 pt-2.5 text-right">
												{t("profiles:detail.workflow.fields.title")}
											</Label>
											<Input
												className="min-w-0 flex-1"
												value={selectedStep.title}
												onChange={(event) =>
													updateStep(selectedStepIndex, (current) => ({
														...current,
														title: event.target.value,
													}))
												}
											/>
										</div>
										<div className="flex items-start gap-4">
											<Label className="w-20 shrink-0 pt-3 text-right">
												{t("profiles:detail.workflow.fields.description")}
											</Label>
											<Textarea
												className="min-w-0 flex-1"
												rows={2}
												value={selectedStep.description}
												onChange={(event) =>
													updateStep(selectedStepIndex, (current) => ({
														...current,
														description: event.target.value,
													}))
												}
											/>
										</div>
										<div className="flex items-start gap-4">
											<Label className="w-20 shrink-0 pt-2.5 text-right">
												{t("profiles:detail.workflow.fields.binding")}
											</Label>
											<div className="grid min-w-0 flex-1 gap-2">
												{selectedStepBinding ? (
													<div
														className="group relative flex min-w-0 items-center gap-2 rounded-md border px-3 py-2"
													>
														<span
															className="min-w-0 flex-1 truncate text-sm"
															title={selectedStepBinding.ref_id}
														>
															{capabilities.find(
																(capability) => capability.ref_id === selectedStepBinding.ref_id,
															)
																?.label ?? selectedStepBinding.ref_id}
														</span>
														<div className="ml-auto -mr-1 transition-[margin] duration-150 group-hover:mr-9 group-focus-within:mr-9">
															<Select
																value={selectedStepBinding.binding_policy}
																onValueChange={(value) =>
																	updateStep(selectedStepIndex, (current) => ({
																		...current,
																		bindings: current.bindings.map((currentBinding) =>
																			currentBinding.ref_id === selectedStepBinding.ref_id
																				? {
																						...currentBinding,
																						binding_policy: value as WorkflowBindingPolicy,
																					  }
																				: currentBinding,
																		),
																	}))
																}
															>
																<SelectTrigger className="h-8 w-44">
																	<SelectValue />
																</SelectTrigger>
																<SelectContent>
																	<SelectItem value="meta_on_demand">
																		{t("profiles:detail.workflow.bindingPolicies.metaOnDemand")}
																	</SelectItem>
																	<SelectItem value="direct">
																		{t("profiles:detail.workflow.bindingPolicies.direct")}
																	</SelectItem>
																</SelectContent>
															</Select>
														</div>
														<Button
															type="button"
															size="icon"
															variant="ghost"
															className="pointer-events-none absolute right-2 top-1/2 h-8 w-8 -translate-y-1/2 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100"
															aria-label={t("profiles:detail.workflow.resetCapabilityBinding")}
															onClick={() => updateStepBinding(selectedStepIndex, null)}
														>
															<Undo2 className="h-4 w-4" />
														</Button>
													</div>
												) : (
													<CapabilityCombobox
														kind="capability"
														items={capabilities}
														loading={capabilitiesLoading}
														onChange={(refId) => updateStepBinding(selectedStepIndex, refId)}
														placeholder={t("profiles:detail.workflow.bindCapability")}
														emptyLabel={t(
															"profiles:detail.workflow.noCapabilities",
														)}
														triggerClassName="border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700"
														getKey={(capability) => capability.ref_id}
														getLabel={(capability) => capability.label}
														getDescription={(capability) => capability.description}
													/>
												)}
											</div>
										</div>
									</div>
								) : (
									<div className="flex min-h-full items-center justify-center text-center text-sm text-muted-foreground">
										{t("profiles:detail.workflow.emptySteps")}
									</div>
								)}
							</div>
							{selectedStep && selectedStepIndex !== null ? (
								<div className="flex shrink-0 items-center p-3">
									<Button
										type="button"
										size="icon"
										variant="ghost"
										className="h-9 w-9 text-destructive hover:bg-destructive/10 hover:text-destructive"
										aria-label={t("profiles:detail.workflow.removeStep")}
										onClick={() => setPendingRemovalStepIndex(selectedStepIndex)}
									>
										<Trash2 className="h-4 w-4" />
									</Button>
									<Button
										type="button"
										size="sm"
										className="ml-auto"
										disabled={saveMutation.isPending}
						onClick={() => saveMutation.mutate(steps)}
									>
										{t("profiles:detail.workflow.save")}
									</Button>
								</div>
							) : null}
						</div>
					</ResizableSplitPane>
				</CardContent>
			</Card>
			<AlertDialog
				open={pendingRemovalStepIndex !== null}
				onOpenChange={(open) => {
					if (!open) setPendingRemovalStepIndex(null);
				}}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>
							{t("profiles:detail.workflow.removeStepConfirmation.title")}
						</AlertDialogTitle>
						<AlertDialogDescription>
							{t("profiles:detail.workflow.removeStepConfirmation.description")}
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>
							{t("profiles:form.buttons.cancel")}
						</AlertDialogCancel>
						<AlertDialogAction
							className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
							onClick={() => {
								if (pendingRemovalStepIndex !== null) {
									removeStep(pendingRemovalStepIndex);
								}
								setPendingRemovalStepIndex(null);
							}}
						>
							{t("profiles:detail.workflow.removeStepConfirmation.confirm")}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}

export function ProfileWorkflowMaterials() {
	const { t } = useTranslation();

	return (
		<Card>
			<CardHeader className="space-y-0">
				<div className="flex items-start justify-between gap-4">
					<div className="min-w-0 flex-1 space-y-1.5">
						<CardTitle>
							{t("profiles:detail.workflow.materials.title")}
						</CardTitle>
						<CardDescription>
							{t("profiles:detail.workflow.materials.description")}
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent>
				<div className="flex items-center gap-2 rounded-md border border-dashed p-3 text-sm text-muted-foreground">
					<ShieldCheck className="h-4 w-4 shrink-0" />
					{t("profiles:detail.workflow.materials.empty")}
				</div>
			</CardContent>
		</Card>
	);
}
