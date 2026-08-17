import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowDown,
  ArrowUp,
  Code2,
  Eye,
  MoreHorizontal,
  ExternalLink,
  FileText,
  FolderOpen,
  Plus,
  Search,
  ShieldCheck,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { configSuitsApi } from "../lib/api";
import { notifyError, notifySuccess } from "../lib/notify";
import { useDesktopCoreState } from "../lib/desktop-core-state";
import {
  createEmptyWorkflowStep,
  isUnsavedWorkflowStep,
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
import { MaterialFilePreview } from "./material-file-preview";
import { resolveMaterialUploadTitle } from "../lib/material-display-name";
import {
  CapsuleStripeList,
  CapsuleStripeListItem,
  PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS,
  PROFILE_EDITOR_SIDEBAR_HOVER_ACTIONS_CLASS,
  PROFILE_EDITOR_SIDEBAR_ITEM_CLASS,
  PROFILE_EDITOR_SIDEBAR_LIST_CLASS,
  PROFILE_EDITOR_SIDEBAR_SCROLL_CLASS,
  PROFILE_EDITOR_SIDEBAR_STICKY_ACTION_CLASS,
} from "./capsule-stripe-list";
import { Button } from "./ui/button";
import { Card, CardContent, CardDescription } from "./ui/card";
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
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
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

interface ProfileWorkflowEditorProps {
  profileId: string;
  capabilities: WorkflowCapabilityOption[];
  capabilitiesLoading?: boolean;
  capabilityMetrics: ProfileSurfaceMetric[];
  onCreateMaterial: () => void;
  onSelectMaterial: (materialId: string) => void;
  onOpenCapability: (capability: WorkflowCapabilityOption) => void;
  selectedStepId: string | null;
  onSelectedStepIdChange: (stepId: string | null) => void;
}

export function ProfileWorkflowEditor({
  profileId,
  capabilities,
  capabilitiesLoading,
  capabilityMetrics,
  onCreateMaterial,
  onSelectMaterial,
  onOpenCapability,
  selectedStepId,
  onSelectedStepIdChange,
}: ProfileWorkflowEditorProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [steps, setSteps] = useState<WorkflowStepDraft[]>([]);
  const [selectedStepIndex, setSelectedStepIndex] = useState<number | null>(
    null,
  );
  const [newStep, setNewStep] = useState<WorkflowStepDraft>(
    createEmptyWorkflowStep,
  );
  const [draftMaterialIds, setDraftMaterialIds] = useState<string[]>([]);
  const [draggedStepIndex, setDraggedStepIndex] = useState<number | null>(null);
  const [materialPickerOpen, setMaterialPickerOpen] = useState(false);
  const [materialPickerQuery, setMaterialPickerQuery] = useState("");
  const [pendingRemovalStepIndex, setPendingRemovalStepIndex] = useState<
    number | null
  >(null);
  const hydratedProfileIdRef = useRef<string | null>(null);
  const pendingStepSelectionRef = useRef<{
    targetId: string;
    staleId: string | null;
    staleIgnored: boolean;
  } | null>(null);

  const specificationQuery = useQuery({
    queryKey: ["workflowSpecification", profileId],
    queryFn: () => configSuitsApi.getWorkflowSpecification(profileId),
    retry: false,
  });
  const materialsQuery = useQuery({
    queryKey: ["workflowMaterials", profileId],
    queryFn: () => configSuitsApi.getWorkflowMaterials(profileId),
  });
  const stepMaterialsMutation = useMutation({
    mutationFn: ({
      stepId,
      materialIds,
    }: {
      stepId: string;
      materialIds: string[];
    }) =>
      configSuitsApi.saveWorkflowStepMaterials({
        profile_id: profileId,
        step_id: stepId,
        material_ids: materialIds,
        expected_materials_revision:
          materialsQuery.data?.materials_revision ?? -1,
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: ["workflowMaterials", profileId],
      }),
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : "Failed to save step materials",
      ),
  });
  useEffect(() => {
    hydratedProfileIdRef.current = null;
    pendingStepSelectionRef.current = null;
    setSteps([]);
    setSelectedStepIndex(null);
    setNewStep(createEmptyWorkflowStep());
    setDraftMaterialIds([]);
  }, [profileId]);

  useEffect(() => {
    if (
      !specificationQuery.data ||
      hydratedProfileIdRef.current === profileId
    ) {
      return;
    }
    const nextSteps = workflowDraftFromSpecification(specificationQuery.data);
    setSteps(nextSteps);
    const requestedStepIndex = nextSteps.findIndex(
      (step) => step.step_id === selectedStepId,
    );
    setSelectedStepIndex(
      requestedStepIndex >= 0
        ? requestedStepIndex
        : nextSteps.length > 0
          ? 0
          : null,
    );
    hydratedProfileIdRef.current = profileId;
  }, [profileId, selectedStepId, specificationQuery.data]);

  useEffect(() => {
    if (steps.length > 0 && selectedStepIndex === null) {
      setSelectedStepIndex(0);
    }
  }, [selectedStepIndex, steps.length]);

  useEffect(() => {
    const pendingSelection = pendingStepSelectionRef.current;
    if (pendingSelection) {
      if (selectedStepId === pendingSelection.targetId) {
        pendingStepSelectionRef.current = null;
      } else if (
        selectedStepId === pendingSelection.staleId &&
        !pendingSelection.staleIgnored
      ) {
        pendingSelection.staleIgnored = true;
        return;
      } else {
        pendingStepSelectionRef.current = null;
      }
    }
    if (!selectedStepId) return;
    const requestedStepIndex = steps.findIndex(
      (step) => step.step_id === selectedStepId,
    );
    if (requestedStepIndex >= 0 && requestedStepIndex !== selectedStepIndex) {
      setSelectedStepIndex(requestedStepIndex);
    }
  }, [selectedStepId, selectedStepIndex, steps]);

  useEffect(() => {
    if (hydratedProfileIdRef.current !== profileId) return;
    const nextSelectedStepId =
      selectedStepIndex === null
        ? null
        : (steps[selectedStepIndex]?.step_id ?? null);
    if (
      nextSelectedStepId === selectedStepId ||
      pendingStepSelectionRef.current?.targetId === nextSelectedStepId
    ) {
      return;
    }
    if (nextSelectedStepId) {
      pendingStepSelectionRef.current = {
        targetId: nextSelectedStepId,
        staleId: selectedStepId,
        staleIgnored: false,
      };
    }
    onSelectedStepIdChange(nextSelectedStepId);
  }, [
    onSelectedStepIdChange,
    profileId,
    selectedStepId,
    selectedStepIndex,
    steps,
  ]);

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
      setSteps(workflowDraftFromSpecification(specification));
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

  const persistedStepIds = useMemo(
    () =>
      new Set(
        (specificationQuery.data?.steps ?? [])
          .map((step) => step.step_id)
          .filter((stepId): stepId is string => Boolean(stepId)),
      ),
    [specificationQuery.data?.steps],
  );

  const selectedStep =
    selectedStepIndex === null
      ? newStep
      : (steps[selectedStepIndex] ?? newStep);
  const selectedStepIsUnsaved = isUnsavedWorkflowStep(
    selectedStep,
    persistedStepIds,
  );
  const selectStep = (index: number) => {
    const stepId = steps[index]?.step_id ?? null;
    setSelectedStepIndex(index);
    if (!stepId) return;
    pendingStepSelectionRef.current = {
      targetId: stepId,
      staleId: selectedStepId,
      staleIgnored: false,
    };
    onSelectedStepIdChange(stepId);
  };
  const editingStep = selectedStep;
  const selectedStepBinding = editingStep.bindings[0] ?? null;
  const selectedCapability = selectedStepBinding
    ? capabilities.find(
      (capability) => capability.ref_id === selectedStepBinding.ref_id,
    )
    : null;
  const editingStepMaterialIds = selectedStepIsUnsaved
    ? draftMaterialIds
    : selectedStep?.step_id
      ? (materialsQuery.data?.step_material_ids[selectedStep.step_id] ?? [])
      : draftMaterialIds;
  const attachMaterialToSelectedStep = (materialId: string) => {
    if (editingStepMaterialIds.includes(materialId)) return;
    if (selectedStepIsUnsaved || !selectedStep?.step_id) {
      setDraftMaterialIds((current) => [...current, materialId]);
      setMaterialPickerOpen(false);
      setMaterialPickerQuery("");
      return;
    }
    stepMaterialsMutation.mutate({
      stepId: selectedStep.step_id,
      materialIds: [...editingStepMaterialIds, materialId],
    });
    setMaterialPickerOpen(false);
    setMaterialPickerQuery("");
  };
  const detachMaterialFromSelectedStep = (materialId: string) => {
    if (selectedStepIsUnsaved || !selectedStep?.step_id) {
      setDraftMaterialIds((current) =>
        current.filter((id) => id !== materialId),
      );
      return;
    }
    stepMaterialsMutation.mutate({
      stepId: selectedStep.step_id,
      materialIds: editingStepMaterialIds.filter((id) => id !== materialId),
    });
  };
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
  const updateEditingStep = (
    update: (step: WorkflowStepDraft) => WorkflowStepDraft,
  ) => {
    if (selectedStepIndex === null) {
      setNewStep(update);
      return;
    }
    updateStep(selectedStepIndex, update);
  };
  const addStep = () => {
    const draft =
      selectedStepIndex === null && steps.length === 0
        ? {
          ...newStep,
          step_id: newStep.step_id ?? crypto.randomUUID(),
        }
        : createEmptyWorkflowStep();
    const nextIndex = steps.length === 0 ? 0 : steps.length;
    setSteps((current) =>
      current.length === 0 ? [draft] : [...current, draft],
    );
    setSelectedStepIndex(nextIndex);
    if (draft.step_id) {
      pendingStepSelectionRef.current = {
        targetId: draft.step_id,
        staleId: selectedStepId,
        staleIgnored: false,
      };
      onSelectedStepIdChange(draft.step_id);
    }
    if (steps.length === 0) {
      setNewStep(createEmptyWorkflowStep());
    }
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
  const dropStepAtIndex = (index: number) => {
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
  const cancelUnsavedStep = (index: number) => {
    if (!isUnsavedWorkflowStep(steps[index], persistedStepIds)) return;
    dropStepAtIndex(index);
    setDraftMaterialIds([]);
  };
  const removeStep = (index: number) => {
    if (isUnsavedWorkflowStep(steps[index], persistedStepIds)) {
      cancelUnsavedStep(index);
      return;
    }
    if (steps.length === 1) {
      saveMutation.mutate([], {
        onSuccess: () => {
          setSteps([]);
          setSelectedStepIndex(null);
        },
      });
      return;
    }

    dropStepAtIndex(index);
  };
  const updateStepBinding = (refId: string | null) => {
    updateEditingStep((current) =>
      withSingleWorkflowCapabilityBinding(current, refId),
    );
  };
  const saveEditedSteps = () => {
    if (selectedStepIndex === null) {
      const newStepIndex = steps.length;
      saveMutation.mutate([...steps, newStep], {
        onSuccess: async (specification) => {
          setSelectedStepIndex(newStepIndex);
          const stepId =
            workflowDraftFromSpecification(specification).steps[newStepIndex]
              ?.step_id;
          if (stepId && draftMaterialIds.length > 0) {
            try {
              const materials =
                await configSuitsApi.getWorkflowMaterials(profileId);
              await configSuitsApi.saveWorkflowStepMaterials({
                profile_id: profileId,
                step_id: stepId,
                material_ids: draftMaterialIds,
                expected_materials_revision: materials.materials_revision,
              });
              setDraftMaterialIds([]);
              queryClient.invalidateQueries({
                queryKey: ["workflowMaterials", profileId],
              });
            } catch (error) {
              notifyError(
                error instanceof Error
                  ? error.message
                  : "Failed to save step materials",
              );
            }
          }
        },
      });
      return;
    }

    const pendingDraftMaterials =
      selectedStepIsUnsaved &&
        selectedStep.step_id &&
        draftMaterialIds.length > 0
        ? {
          stepId: selectedStep.step_id,
          materialIds: [...draftMaterialIds],
        }
        : null;
    saveMutation.mutate(steps, {
      onSuccess: async () => {
        if (!pendingDraftMaterials) return;
        try {
          const materials =
            await configSuitsApi.getWorkflowMaterials(profileId);
          await configSuitsApi.saveWorkflowStepMaterials({
            profile_id: profileId,
            step_id: pendingDraftMaterials.stepId,
            material_ids: pendingDraftMaterials.materialIds,
            expected_materials_revision: materials.materials_revision,
          });
          setDraftMaterialIds([]);
          queryClient.invalidateQueries({
            queryKey: ["workflowMaterials", profileId],
          });
        } catch (error) {
          notifyError(
            error instanceof Error
              ? error.message
              : "Failed to save step materials",
          );
        }
      },
    });
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
        className="hidden shrink-0"
      />
      <Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
          <ResizableSplitPane
            dividerAriaLabel={t("profiles:detail.workflow.resizeStepColumns")}
            initialLeftWidth={320}
            maxLeftWidth={480}
            preferRightPanelSpace
          >
            <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
              <div className="min-h-16 shrink-0 p-3">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  {t("profiles:detail.workflow.stepsTitle")}
                </div>
                <CardDescription className="truncate text-xs text-slate-500 dark:text-slate-400">
                  {t("profiles:detail.workflow.stepsListDescription")}
                </CardDescription>
              </div>
              <CardListScrollBody
                className={PROFILE_EDITOR_SIDEBAR_SCROLL_CLASS}
              >
                <div className="min-h-full pb-3">
                  <CapsuleStripeList
                    className={PROFILE_EDITOR_SIDEBAR_LIST_CLASS}
                  >
                    {steps.map((step, index) => {
                      const isSelected = selectedStepIndex === index;
                      return (
                        <CapsuleStripeListItem
                          key={index}
                          className={`${PROFILE_EDITOR_SIDEBAR_ITEM_CLASS} ${isSelected ? "bg-primary/10" : "hover:bg-accent/50"
                            }`}
                          draggable
                          onDragEnd={() => setDraggedStepIndex(null)}
                          onDragOver={(event) => {
                            if (
                              draggedStepIndex === null ||
                              draggedStepIndex === index
                            )
                              return;
                            event.preventDefault();
                            event.dataTransfer.dropEffect = "move";
                          }}
                          onDragStart={(event) => {
                            setDraggedStepIndex(index);
                            event.dataTransfer.effectAllowed = "move";
                            event.dataTransfer.setData(
                              "text/plain",
                              String(index),
                            );
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
                            onClick={() => selectStep(index)}
                          >
                            <span
                              className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-md border text-sm font-semibold ${isSelected
                                ? "border-primary bg-primary text-primary-foreground"
                                : "border-slate-200 bg-white text-slate-600 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300"
                                }`}
                            >
                              {index + 1}
                            </span>
                            <span className="min-w-0 flex-1">
                              <span
                                className={`block truncate font-medium ${isSelected
                                  ? "text-primary"
                                  : "text-slate-900 dark:text-slate-100"
                                  }`}
                              >
                                {step.title ||
                                  t("profiles:detail.workflow.untitledStep")}
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
                          <div
                            className={
                              PROFILE_EDITOR_SIDEBAR_HOVER_ACTIONS_CLASS
                            }
                          >
                            {index > 0 ? (
                              <Button
                                type="button"
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7 bg-transparent text-muted-foreground shadow-none hover:bg-transparent hover:text-foreground"
                                aria-label={t(
                                  "profiles:detail.workflow.moveUp",
                                )}
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
                                aria-label={t(
                                  "profiles:detail.workflow.moveDown",
                                )}
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
                  <div className={PROFILE_EDITOR_SIDEBAR_STICKY_ACTION_CLASS}>
                    <Button
                      type="button"
                      variant="outline"
                      className="w-full border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700"
                      onClick={addStep}
                    >
                      <Plus className="mr-2 h-4 w-4" />
                      {t("profiles:detail.workflow.addStep")}
                    </Button>
                  </div>
                </div>
              </CardListScrollBody>
            </div>
            <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
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
              <div
                className={`min-h-0 flex-1 overflow-y-auto ${PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS}`}
              >
                {editingStep ? (
                  <div className="grid gap-4">
                    <div className="flex items-start gap-4">
                      <Label className="w-20 shrink-0 pt-2.5 text-right">
                        {t("profiles:detail.workflow.fields.title")}
                      </Label>
                      <Input
                        className="min-w-0 flex-1"
                        value={editingStep.title}
                        onChange={(event) =>
                          updateEditingStep((current) => ({
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
                        value={editingStep.description}
                        onChange={(event) =>
                          updateEditingStep((current) => ({
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
                          <div className="group relative flex h-10 min-w-0 items-center gap-2 rounded-md border px-3">
                            <div className="min-w-0 flex-1">
                              <button
                                type="button"
                                className="inline-block max-w-full truncate rounded-sm text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                title={t(
                                  "profiles:detail.workflow.viewCapabilityDetails",
                                )}
                                disabled={!selectedCapability}
                                onClick={() => {
                                  if (selectedCapability)
                                    onOpenCapability(selectedCapability);
                                }}
                              >
                                {selectedCapability?.label ??
                                  selectedStepBinding.ref_id}
                              </button>
                            </div>
                            <div className="ml-auto -mr-[9px] transition-[margin] duration-150 group-hover:mr-[27px] group-focus-within:mr-[27px]">
                              <Select
                                value={selectedStepBinding.binding_policy}
                                onValueChange={(value) =>
                                  updateEditingStep((current) => ({
                                    ...current,
                                    bindings: current.bindings.map(
                                      (currentBinding) =>
                                        currentBinding.ref_id ===
                                          selectedStepBinding.ref_id
                                          ? {
                                            ...currentBinding,
                                            binding_policy:
                                              value as WorkflowBindingPolicy,
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
                                    {t(
                                      "profiles:detail.workflow.bindingPolicies.metaOnDemand",
                                    )}
                                  </SelectItem>
                                  <SelectItem value="direct">
                                    {t(
                                      "profiles:detail.workflow.bindingPolicies.direct",
                                    )}
                                  </SelectItem>
                                </SelectContent>
                              </Select>
                            </div>
                            <Button
                              type="button"
                              size="icon"
                              variant="ghost"
                              className="pointer-events-none absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100"
                              aria-label={t(
                                "profiles:detail.workflow.resetCapabilityBinding",
                              )}
                              onClick={() => updateStepBinding(null)}
                            >
                              <Undo2 className="h-4 w-4" />
                            </Button>
                          </div>
                        ) : (
                          <CapabilityCombobox
                            kind="capability"
                            items={capabilities}
                            loading={capabilitiesLoading}
                            onChange={(refId) => updateStepBinding(refId)}
                            placeholder={t(
                              "profiles:detail.workflow.bindCapability",
                            )}
                            emptyLabel={t(
                              "profiles:detail.workflow.noCapabilities",
                            )}
                            triggerClassName="border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700"
                            getKey={(capability) => capability.ref_id}
                            getLabel={(capability) => capability.label}
                            getDescription={(capability) =>
                              capability.description
                            }
                          />
                        )}
                      </div>
                    </div>
                    <div className="flex items-start gap-4">
                      <Label className="w-20 shrink-0 pt-2.5 text-right">
                        {t("profiles:detail.workflow.materials.title")}
                      </Label>
                      <div className="grid min-w-0 flex-1 gap-2">
                        {(materialsQuery.data?.materials.length ?? 0) > 0 ? (
                          <>
                            {editingStepMaterialIds.map((materialId) => {
                              const material =
                                materialsQuery.data?.materials.find(
                                  (item) => item.material_id === materialId,
                                );
                              if (!material) return null;
                              return (
                                <div
                                  key={material.material_id}
                                  className="group flex items-center gap-2 rounded-md border px-3 py-2 text-sm"
                                >
                                  <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
                                  <div className="min-w-0 flex-1">
                                    <button
                                      type="button"
                                      className="inline-block max-w-full truncate rounded-sm text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                      title={t(
                                        "profiles:detail.workflow.materials.viewDetails",
                                      )}
                                      onClick={() =>
                                        onSelectMaterial(material.material_id)
                                      }
                                    >
                                      {material.title}
                                    </button>
                                  </div>
                                  <Button
                                    type="button"
                                    size="icon"
                                    variant="ghost"
                                    className="h-6 w-6 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                                    aria-label={t(
                                      "profiles:detail.workflow.materials.remove",
                                    )}
                                    disabled={stepMaterialsMutation.isPending}
                                    onClick={() =>
                                      detachMaterialFromSelectedStep(
                                        material.material_id,
                                      )
                                    }
                                  >
                                    <X className="h-3.5 w-3.5" />
                                  </Button>
                                </div>
                              );
                            })}
                            <Popover
                              open={materialPickerOpen}
                              onOpenChange={setMaterialPickerOpen}
                            >
                              <PopoverTrigger asChild>
                                <Button
                                  type="button"
                                  variant="outline"
                                  className="w-full border-dashed"
                                  disabled={stepMaterialsMutation.isPending}
                                >
                                  <Plus className="mr-2 h-4 w-4" />
                                  {t("profiles:detail.workflow.materials.add")}
                                </Button>
                              </PopoverTrigger>
                              <PopoverContent
                                align="start"
                                className="w-80 p-3"
                              >
                                <div className="space-y-3">
                                  <div className="relative">
                                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                                    <Input
                                      value={materialPickerQuery}
                                      onChange={(event) =>
                                        setMaterialPickerQuery(
                                          event.target.value,
                                        )
                                      }
                                      className="h-9 pl-9"
                                      placeholder={t(
                                        "profiles:detail.workflow.materials.search",
                                      )}
                                    />
                                  </div>
                                  <div className="max-h-56 space-y-1 overflow-y-auto">
                                    {(materialsQuery.data?.materials ?? [])
                                      .filter(
                                        (material) =>
                                          !editingStepMaterialIds.includes(
                                            material.material_id,
                                          ),
                                      )
                                      .filter((material) =>
                                        material.title
                                          .toLowerCase()
                                          .includes(
                                            materialPickerQuery
                                              .trim()
                                              .toLowerCase(),
                                          ),
                                      )
                                      .map((material) => (
                                        <button
                                          key={material.material_id}
                                          type="button"
                                          className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-sm hover:bg-accent"
                                          onClick={() =>
                                            attachMaterialToSelectedStep(
                                              material.material_id,
                                            )
                                          }
                                        >
                                          <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
                                          <span className="truncate">
                                            {material.title}
                                          </span>
                                        </button>
                                      ))}
                                    {!(
                                      materialsQuery.data?.materials ?? []
                                    ).some(
                                      (material) =>
                                        !editingStepMaterialIds.includes(
                                          material.material_id,
                                        ) &&
                                        material.title
                                          .toLowerCase()
                                          .includes(
                                            materialPickerQuery
                                              .trim()
                                              .toLowerCase(),
                                          ),
                                    ) ? (
                                      <p className="px-2 py-3 text-sm text-muted-foreground">
                                        {t(
                                          "profiles:detail.workflow.materials.empty",
                                        )}
                                      </p>
                                    ) : null}
                                  </div>
                                  <div className="border-t pt-2">
                                    <Button
                                      type="button"
                                      variant="ghost"
                                      className="w-full justify-start"
                                      onClick={() => {
                                        setMaterialPickerOpen(false);
                                        onCreateMaterial();
                                      }}
                                    >
                                      <Plus className="mr-2 h-4 w-4" />
                                      {t(
                                        "profiles:detail.workflow.materials.add",
                                      )}
                                    </Button>
                                  </div>
                                </div>
                              </PopoverContent>
                            </Popover>
                          </>
                        ) : (materialsQuery.data?.materials.length ?? 0) ===
                          0 ? (
                          <Button
                            type="button"
                            variant="outline"
                            className="w-full border-dashed"
                            onClick={onCreateMaterial}
                          >
                            <Plus className="mr-2 h-4 w-4" />
                            {t("profiles:detail.workflow.materials.add")}
                          </Button>
                        ) : null}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="flex min-h-full items-center justify-center text-center text-sm text-muted-foreground">
                    {t("profiles:detail.workflow.emptySteps")}
                  </div>
                )}
              </div>
              {editingStep ? (
                <div className="flex shrink-0 items-center p-3">
                  {selectedStep && selectedStepIndex !== null ? (
                    isUnsavedWorkflowStep(selectedStep, persistedStepIds) ? (
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        className="h-9 w-9 text-muted-foreground hover:text-foreground"
                        aria-label={t("profiles:form.buttons.cancel")}
                        onClick={() => cancelUnsavedStep(selectedStepIndex)}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    ) : (
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        className="h-9 w-9 text-destructive hover:bg-destructive/10 hover:text-destructive"
                        aria-label={t("profiles:detail.workflow.removeStep")}
                        onClick={() =>
                          setPendingRemovalStepIndex(selectedStepIndex)
                        }
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )
                  ) : null}
                  <Button
                    type="button"
                    size="sm"
                    className="ml-auto"
                    disabled={saveMutation.isPending}
                    onClick={saveEditedSteps}
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


const MATERIAL_PREVIEW_HOVER_BUTTON_CLASS =
  "h-8 w-8 bg-transparent opacity-0 shadow-none transition-opacity hover:bg-transparent group-hover:opacity-100 focus-visible:opacity-100";

const MaterialMarkdownEditorField = memo(
  function MaterialMarkdownEditorField({
    editorKey,
    initialValue,
    onChange,
    onOpenPreview,
    placeholder,
    previewLabel,
  }: {
    editorKey: string;
    initialValue: string;
    onChange: (value: string) => void;
    onOpenPreview: () => void;
    placeholder: string;
    previewLabel: string;
  }) {
    return (
      <div className="grid min-h-0 flex-1 grid-cols-[5rem_minmax(0,1fr)] items-stretch gap-2 [grid-template-rows:minmax(0,1fr)]">
        <Label
          htmlFor="workflow-material-markdown"
          className="pt-2.5 text-right"
        >
          Markdown
        </Label>
        <div className="group relative flex min-h-0 min-w-0 flex-col">
          {/*
            Uncontrolled editor: large markdown (100KB–MB) must not be a controlled
            React value, or every parent render rewrites the whole DOM string.
            Remount via editorKey when selection/hydration changes.
          */}
          <Textarea
            key={editorKey}
            id="workflow-material-markdown"
            className={
              initialValue.length > 48_000
                ? "h-full min-h-[12rem] flex-1 resize-none overflow-auto whitespace-pre pr-10 font-mono text-xs leading-5"
                : "h-full min-h-[12rem] flex-1 resize-none pr-10"
            }
            defaultValue={initialValue}
            onChange={(event) => onChange(event.target.value)}
            placeholder={placeholder}
            spellCheck={initialValue.length < 48_000}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
          />
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="absolute right-2 top-2 h-7 w-7 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
            title={previewLabel}
            aria-label={previewLabel}
            onClick={onOpenPreview}
          >
            <Eye className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    );
  },
  (prev, next) =>
    prev.editorKey === next.editorKey &&
    prev.placeholder === next.placeholder &&
    prev.previewLabel === next.previewLabel &&
    prev.onChange === next.onChange &&
    prev.onOpenPreview === next.onOpenPreview,
);

export function ProfileWorkflowMaterials({
  profileId,
  focusTitleToken,
  isActive,
  selectedMaterialId,
  onSelectedMaterialIdChange,
}: {
  profileId: string;
  focusTitleToken?: number;
  isActive: boolean;
  selectedMaterialId: string | null;
  onSelectedMaterialIdChange: (materialId: string | null) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { isTauriShell, coreView } = useDesktopCoreState();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<
    Array<{
      material_id: string;
      title: string;
      kind: "external_url" | "markdown_file" | "uploaded_file";
      external_url: string;
      markdown_content: string;
    }>
  >([]);
  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  const [markdownEditorSeed, setMarkdownEditorSeed] = useState("");
  const [markdownEditorKey, setMarkdownEditorKey] = useState("empty");
  const [hasMarkdownContent, setHasMarkdownContent] = useState(false);
  const [creationKind, setCreationKind] = useState<
    "external_url" | "markdown_file" | "uploaded_file"
  >("external_url");
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);
  const [markdownPreviewMode, setMarkdownPreviewMode] = useState<
    "rendered" | "source"
  >("source");
  const [previewContent, setPreviewContent] = useState("");
  const [deleteConfirmationOpen, setDeleteConfirmationOpen] = useState(false);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const formHydrationKeyRef = useRef<string | null>(null);
  // markdownRef is the source of truth while editing (especially large pastes).
  // Do not copy it into React state — a parent re-render would rewrite the textarea.
  const markdownRef = useRef("");
  const applyMarkdownEditor = useCallback((content: string, editorKey: string) => {
    markdownRef.current = content;
    setHasMarkdownContent(content.trim().length > 0);
    setMarkdownEditorSeed(content);
    setMarkdownEditorKey(editorKey);
  }, []);
  const selectedIdRef = useRef(selectedId);
  selectedIdRef.current = selectedId;
  const uploadInputRef = useRef<HTMLInputElement>(null);
  const handledFocusTitleTokenRef = useRef(focusTitleToken);
  const pendingMaterialSelectionRef = useRef<{
    targetId: string;
    staleId: string | null;
    staleIgnored: boolean;
  } | null>(null);
  const ensuredEmptyDraftRef = useRef(false);
  const materialsQuery = useQuery({
    queryKey: ["workflowMaterials", profileId],
    queryFn: () => configSuitsApi.getWorkflowMaterials(profileId),
  });
  const materials = useMemo(
    () => materialsQuery.data?.materials ?? [],
    [materialsQuery.data?.materials],
  );
  const persistedMaterialIds = useMemo(
    () => new Set(materials.map((material) => material.material_id)),
    [materials],
  );
  const persistedMaterialIdsRef = useRef(persistedMaterialIds);
  persistedMaterialIdsRef.current = persistedMaterialIds;
  const selectedDraft =
    drafts.find((draft) => draft.material_id === selectedId) ?? null;
  const isCreating = selectedDraft !== null;
  const selected =
    materials.find((material) => material.material_id === selectedId) ?? null;
  const selectMaterial = (materialId: string) => {
    const activeId = selectedIdRef.current;
    const latestMarkdown = markdownRef.current;
    if (activeId && activeId !== materialId) {
      setDrafts((current) => {
        let changed = false;
        const next = current.map((draft) => {
          if (draft.material_id !== activeId) return draft;
          if (draft.markdown_content === latestMarkdown) return draft;
          changed = true;
          return { ...draft, markdown_content: latestMarkdown };
        });
        return changed ? next : current;
      });
    }
    pendingMaterialSelectionRef.current = {
      targetId: materialId,
      staleId: selectedMaterialId,
      staleIgnored: false,
    };
    setSelectedId(materialId);
    onSelectedMaterialIdChange(materialId);
  };
  const updateSelectedDraft = (
    update: (draft: {
      material_id: string;
      title: string;
      kind: "external_url" | "markdown_file" | "uploaded_file";
      external_url: string;
      markdown_content: string;
    }) => {
      material_id: string;
      title: string;
      kind: "external_url" | "markdown_file" | "uploaded_file";
      external_url: string;
      markdown_content: string;
    },
  ) => {
    if (!selectedDraft) return;
    setDrafts((current) =>
      current.map((draft) =>
        draft.material_id === selectedDraft.material_id
          ? update(draft)
          : draft,
      ),
    );
  };

  const markdownFlushTimerRef = useRef<number | null>(null);
  const previewFlushTimerRef = useRef<number | null>(null);
  const isPreviewOpenRef = useRef(isPreviewOpen);
  isPreviewOpenRef.current = isPreviewOpen;

  const flushMarkdownToDraft = useCallback((draftId: string, content: string) => {
    setDrafts((current) => {
      let changed = false;
      const next = current.map((draft) => {
        if (draft.material_id !== draftId) return draft;
        if (draft.markdown_content === content) return draft;
        changed = true;
        return { ...draft, markdown_content: content };
      });
      return changed ? next : current;
    });
  }, []);

  const handleMarkdownChange = useCallback(
    (nextMarkdown: string) => {
      markdownRef.current = nextMarkdown;
      // Save enablement only — do not put multi-MB strings into React state
      // on every keypress.
      setHasMarkdownContent(nextMarkdown.trim().length > 0);

      const draftId = selectedIdRef.current;
      if (draftId && !persistedMaterialIdsRef.current.has(draftId)) {
        if (markdownFlushTimerRef.current !== null) {
          window.clearTimeout(markdownFlushTimerRef.current);
        }
        markdownFlushTimerRef.current = window.setTimeout(
          () => {
            flushMarkdownToDraft(draftId, markdownRef.current);
            markdownFlushTimerRef.current = null;
          },
          nextMarkdown.length > 64_000 ? 500 : 150,
        );
      }

      if (isPreviewOpenRef.current) {
        if (previewFlushTimerRef.current !== null) {
          window.clearTimeout(previewFlushTimerRef.current);
        }
        previewFlushTimerRef.current = window.setTimeout(
          () => {
            setPreviewContent(markdownRef.current);
            previewFlushTimerRef.current = null;
          },
          nextMarkdown.length > 64_000 ? 450 : 120,
        );
      }
    },
    [flushMarkdownToDraft],
  );
  const handleOpenMarkdownPreview = useCallback(() => {
    setPreviewContent(markdownRef.current);
    setMarkdownPreviewMode("rendered");
    setIsPreviewOpen(true);
  }, []);
  const selectedExtension =
    (selected?.original_filename ?? selected?.relative_path)
      ?.split(".")
      .pop()
      ?.toLowerCase() ?? (selected?.kind === "markdown_file" ? "md" : "");
  const selectedIsText = [
    "md",
    "js",
    "mjs",
    "cjs",
    "py",
    "json",
    "yaml",
    "yml",
    "toml",
  ].includes(selectedExtension);
  const desktopLocalFileActionsAvailable =
    isTauriShell && coreView?.selectedSource === "localhost";
  const runDesktopFileAction = async (action: "open" | "reveal") => {
    if (!selected) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("mcp_shell_open_workflow_material", {
        profileId,
        materialId: selected.material_id,
        action,
      });
    } catch (error) {
      notifyError(
        error instanceof Error
          ? error.message
          : "Unable to open workflow material file",
      );
    }
  };
  const previewQuery = useQuery({
    queryKey: ["workflowMaterialPreview", profileId, selected?.material_id],
    queryFn: () =>
      configSuitsApi.getWorkflowMaterialPreview(
        profileId,
        selected?.material_id ?? "",
      ),
    enabled: Boolean(
      selected &&
      !selected.external_url &&
      !selected.markdown_content &&
      selectedIsText,
    ),
  });
  useEffect(() => {
    if (!isPreviewOpen) {
      return;
    }
    if (creationKind === "markdown_file") {
      setPreviewContent(markdownRef.current);
      return;
    }
    const nextContent = selected?.markdown_content ?? previewQuery.data ?? "";
    setPreviewContent(nextContent);
  }, [
    isPreviewOpen,
    creationKind,
    selected?.markdown_content,
    selected?.material_id,
    previewQuery.data,
  ]);

  const saveMutation = useMutation({
    mutationFn: (kind: "external_url" | "markdown_file" | "uploaded_file") => {
      let markdownContent: string | null = null;
      if (kind === "markdown_file") {
        const domValue = (
          document.getElementById(
            "workflow-material-markdown",
          ) as HTMLTextAreaElement | null
        )?.value;
        if (typeof domValue === "string") {
          markdownRef.current = domValue;
        }
        markdownContent = markdownRef.current;
      }
      return configSuitsApi.saveWorkflowMaterial({
        profile_id: profileId,
        material_id: selected && !isCreating ? selected.material_id : null,
        expected_material_revision:
          selected && !isCreating ? selected.material_revision : null,
        expected_materials_revision:
          materialsQuery.data?.materials_revision ?? -1,
        title,
        kind,
        external_url: kind === "external_url" ? url : null,
        markdown_content: markdownContent,
      });
    },
    onSuccess: (material) => {
      const draftId = selectedDraft?.material_id ?? null;
      if (draftId) {
        setDrafts((current) =>
          current.filter((draft) => draft.material_id !== draftId),
        );
      }
      formHydrationKeyRef.current = null;
      setTitle(material.title);
      setCreationKind(material.kind);
      setUrl(material.external_url ?? "");
      applyMarkdownEditor(
        material.markdown_content ?? "",
        `material:${material.material_id}:${material.material_revision}`,
      );
      selectMaterial(material.material_id);
      queryClient.invalidateQueries({
        queryKey: ["workflowMaterials", profileId],
      });
    },
    onError: (error) =>
      notifyError(
        error instanceof Error ? error.message : "Failed to save material",
      ),
  });
  const uploadMutation = useMutation({
    mutationFn: (file: File) => {
      const form = new FormData();
      form.set("profile_id", profileId);
      form.set(
        "expected_materials_revision",
        String(materialsQuery.data?.materials_revision ?? -1),
      );
      form.set("title", resolveMaterialUploadTitle(title, file.name));
      form.set("file", file);
      if (selected && !isCreating) {
        form.set("material_id", selected.material_id);
        form.set(
          "expected_material_revision",
          String(selected.material_revision),
        );
        return configSuitsApi.uploadWorkflowMaterial(form, true);
      }
      return configSuitsApi.uploadWorkflowMaterial(form);
    },
    onSuccess: (material) => {
      const draftId = selectedDraft?.material_id ?? null;
      if (draftId) {
        setDrafts((current) =>
          current.filter((draft) => draft.material_id !== draftId),
        );
      }
      formHydrationKeyRef.current = null;
      setTitle(material.title);
      setCreationKind(material.kind);
      setUrl(material.external_url ?? "");
      applyMarkdownEditor(
        material.markdown_content ?? "",
        `material:${material.material_id}:${material.material_revision}`,
      );
      selectMaterial(material.material_id);
      queryClient.invalidateQueries({
        queryKey: ["workflowMaterials", profileId],
      });
    },
    onError: (error) =>
      notifyError(
        error instanceof Error ? error.message : "Failed to upload material",
      ),
  });
  const deleteMutation = useMutation({
    mutationFn: () =>
      selected
        ? configSuitsApi.deleteWorkflowMaterial({
          profile_id: profileId,
          material_id: selected.material_id,
          expected_material_revision: selected.material_revision,
          expected_materials_revision:
            materialsQuery.data?.materials_revision ?? -1,
        })
        : Promise.resolve(),
    onSuccess: () => {
      setSelectedId(null);
      onSelectedMaterialIdChange(null);
      queryClient.invalidateQueries({
        queryKey: ["workflowMaterials", profileId],
      });
    },
  });
  const reorderMutation = useMutation({
    mutationFn: (materialIds: string[]) =>
      configSuitsApi.reorderWorkflowMaterials({
        profile_id: profileId,
        material_ids: materialIds,
        expected_materials_revision:
          materialsQuery.data?.materials_revision ?? -1,
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: ["workflowMaterials", profileId],
      }),
    onError: (error) =>
      notifyError(
        error instanceof Error ? error.message : "Failed to reorder Materials",
      ),
  });
  const moveMaterial = (materialId: string, direction: -1 | 1) => {
    if (!persistedMaterialIds.has(materialId)) return;
    const materialIds = materials.map((material) => material.material_id);
    const currentIndex = materialIds.indexOf(materialId);
    const targetIndex = currentIndex + direction;
    if (
      currentIndex < 0 ||
      targetIndex < 0 ||
      targetIndex >= materialIds.length
    )
      return;
    [materialIds[currentIndex], materialIds[targetIndex]] = [
      materialIds[targetIndex],
      materialIds[currentIndex],
    ];
    reorderMutation.mutate(materialIds);
  };

  const beginCreate = () => {
    const draft = {
      material_id: crypto.randomUUID(),
      title: "",
      kind: "external_url" as const,
      external_url: "",
      markdown_content: "",
    };
    setDrafts((current) => [...current, draft]);
    setTitle("");
    setUrl("");
    applyMarkdownEditor("", `draft:${draft.material_id}`);
    setCreationKind("external_url");
    selectMaterial(draft.material_id);
  };
  const cancelDraft = (materialId: string) => {
    if (persistedMaterialIds.has(materialId)) return;
    const remainingDrafts = drafts.filter(
      (draft) => draft.material_id !== materialId,
    );
    setDrafts(remainingDrafts);
    const nextId =
      remainingDrafts[remainingDrafts.length - 1]?.material_id ??
      materials[materials.length - 1]?.material_id ??
      null;
    if (nextId) {
      selectMaterial(nextId);
      return;
    }
    setSelectedId(null);
    onSelectedMaterialIdChange(null);
    setTitle("");
    setUrl("");
    applyMarkdownEditor("", "empty");
  };
  useEffect(() => {
    pendingMaterialSelectionRef.current = null;
    ensuredEmptyDraftRef.current = false;
    setSelectedId(null);
    setDrafts([]);
    setTitle("");
    setUrl("");
    applyMarkdownEditor("", "empty");
    setCreationKind("external_url");
  }, [applyMarkdownEditor, profileId]);
  // Hydrate the form only when the selection identity (or persisted revision) changes.
  // Draft field keystrokes must not rewrite markdown/title local state from drafts[],
  // or large materials hitch on every Title keypress.
  useEffect(() => {
    if (!selectedId) {
      formHydrationKeyRef.current = null;
      return;
    }
    const draft = drafts.find((item) => item.material_id === selectedId);
    const material = materials.find((item) => item.material_id === selectedId);
    if (!draft && !material) {
      return;
    }
    const hydrationKey = draft
      ? `draft:${selectedId}`
      : `material:${selectedId}:${material?.material_revision ?? 0}`;
    if (formHydrationKeyRef.current === hydrationKey) {
      return;
    }
    formHydrationKeyRef.current = hydrationKey;
    if (draft) {
      setTitle(draft.title);
      setCreationKind(draft.kind);
      setUrl(draft.external_url);
      applyMarkdownEditor(draft.markdown_content, `draft:${selectedId}`);
      return;
    }
    if (material) {
      setTitle(material.title);
      setCreationKind(material.kind);
      setUrl(material.external_url ?? "");
      applyMarkdownEditor(
        material.markdown_content ?? "",
        `material:${selectedId}:${material.material_revision}`,
      );
    }
  }, [applyMarkdownEditor, drafts, materials, selectedId]);
  useEffect(() => {
    setIsPreviewOpen(false);
    setMarkdownPreviewMode("source");
  }, [selectedId]);
  const canSaveMaterial = () => {
    if (!title.trim()) return false;
    if (creationKind === "external_url") return Boolean(url.trim());
    if (creationKind === "markdown_file") {
      return hasMarkdownContent || markdownRef.current.trim().length > 0;
    }
    return Boolean(selected && !isCreating);
  };
  useEffect(() => {
    if (
      focusTitleToken === undefined ||
      focusTitleToken === handledFocusTitleTokenRef.current
    ) {
      return;
    }
    handledFocusTitleTokenRef.current = focusTitleToken;
    beginCreate();
    requestAnimationFrame(() => titleInputRef.current?.focus());
  }, [focusTitleToken]);
  useEffect(() => {
    if (!isActive) return;
    if (!materialsQuery.data) return;

    const knownIds = new Set([
      ...materials.map((material) => material.material_id),
      ...drafts.map((draft) => draft.material_id),
    ]);

    if (knownIds.size === 0) {
      if (!ensuredEmptyDraftRef.current) {
        ensuredEmptyDraftRef.current = true;
        beginCreate();
      }
      return;
    }
    ensuredEmptyDraftRef.current = false;

    const pendingSelection = pendingMaterialSelectionRef.current;
    if (pendingSelection) {
      if (selectedMaterialId === pendingSelection.targetId) {
        pendingMaterialSelectionRef.current = null;
      } else if (
        selectedMaterialId === pendingSelection.staleId &&
        !pendingSelection.staleIgnored
      ) {
        pendingSelection.staleIgnored = true;
        return;
      } else {
        pendingMaterialSelectionRef.current = null;
      }
    }

    if (selectedMaterialId && knownIds.has(selectedMaterialId)) {
      if (selectedId !== selectedMaterialId) {
        setSelectedId(selectedMaterialId);
      }
      return;
    }

    if (selectedId && knownIds.has(selectedId)) {
      if (selectedId !== selectedMaterialId) {
        pendingMaterialSelectionRef.current = {
          targetId: selectedId,
          staleId: selectedMaterialId,
          staleIgnored: false,
        };
        onSelectedMaterialIdChange(selectedId);
      }
      return;
    }

    const fallbackId =
      drafts[drafts.length - 1]?.material_id ??
      materials[0]?.material_id ??
      null;
    if (!fallbackId) return;
    if (selectedId !== fallbackId) {
      setSelectedId(fallbackId);
    }
    if (fallbackId !== selectedMaterialId) {
      pendingMaterialSelectionRef.current = {
        targetId: fallbackId,
        staleId: selectedMaterialId,
        staleIgnored: false,
      };
      onSelectedMaterialIdChange(fallbackId);
    }
  }, [
    drafts,
    isActive,
    materials,
    materialsQuery.data,
    selectedId,
    selectedMaterialId,
    onSelectedMaterialIdChange,
  ]);
  const materialSurfaceMetrics: ProfileSurfaceMetric[] = [
    {
      id: "materials",
      label: t("profiles:detail.workflow.materials.metric"),
      value: String(materials.length + drafts.length),
      description: t("profiles:detail.workflow.materials.metricDescription"),
    },
    {
      id: "markdown",
      label: t("profiles:detail.workflow.materials.markdownMetric"),
      value: String(
        materials.filter((material) => material.kind === "markdown_file")
          .length +
        drafts.filter((draft) => draft.kind === "markdown_file").length,
      ),
    },
    {
      id: "files",
      label: t("profiles:detail.workflow.materials.filesMetric"),
      value: String(
        materials.filter((material) => material.kind === "uploaded_file")
          .length +
        drafts.filter((draft) => draft.kind === "uploaded_file").length,
      ),
    },
    {
      id: "urls",
      label: t("profiles:detail.workflow.materials.urlsMetric"),
      value: String(
        materials.filter((material) => material.kind === "external_url")
          .length +
        drafts.filter((draft) => draft.kind === "external_url").length,
      ),
    },
  ];

  const listItems = [
    ...materials.map((material) => ({
      id: material.material_id,
      title: material.title,
      isDraft: false as const,
      material,
    })),
    ...drafts.map((draft) => ({
      id: draft.material_id,
      title:
        draft.title.trim() ||
        t("profiles:detail.workflow.materials.untitled", {
          defaultValue: "Untitled material",
        }),
      isDraft: true as const,
      draft,
    })),
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <ProfileSurfaceMetrics
        metrics={materialSurfaceMetrics}
        description={t("profiles:detail.workflow.materials.metricDescription")}
        className="hidden shrink-0"
      />
      <Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
          <ResizableSplitPane
            dividerAriaLabel={t(
              "profiles:detail.workflow.materials.resizeColumns",
            )}
            initialLeftWidth={320}
            maxLeftWidth={480}
            preferRightPanelSpace
          >
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
              <div className="min-h-16 shrink-0 p-3">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  {t("profiles:detail.workflow.materials.title")}
                </div>
                <CardDescription className="truncate text-xs text-slate-500 dark:text-slate-400">
                  {t("profiles:detail.workflow.materials.description")}
                </CardDescription>
              </div>
              <CardListScrollBody
                className={PROFILE_EDITOR_SIDEBAR_SCROLL_CLASS}
              >
                <div className="min-h-full pb-3">
                  <CapsuleStripeList
                    className={PROFILE_EDITOR_SIDEBAR_LIST_CLASS}
                  >
                    {listItems.map((item) => {
                      const isSelected = selectedId === item.id;
                      const canReorder = !item.isDraft;
                      const persistedIndex = item.isDraft
                        ? -1
                        : materials.findIndex(
                          (material) => material.material_id === item.id,
                        );
                      return (
                        <CapsuleStripeListItem
                          key={item.id}
                          className={`${PROFILE_EDITOR_SIDEBAR_ITEM_CLASS} ${isSelected ? "bg-primary/10" : "hover:bg-accent/50"}`}
                        >
                          <button
                            type="button"
                            aria-current={isSelected ? "true" : undefined}
                            onClick={() => selectMaterial(item.id)}
                            className="flex min-w-0 flex-1 items-center gap-3 text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
                          >
                            <span
                              className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-md border ${isSelected ? "border-primary bg-primary text-primary-foreground" : "border-slate-200 bg-white text-slate-600 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300"}`}
                            >
                              <FileText className="h-4 w-4" />
                            </span>
                            <span className="min-w-0 flex-1">
                              <span
                                className={`block truncate font-medium ${isSelected ? "text-primary" : "text-slate-900 dark:text-slate-100"}`}
                              >
                                {item.title}
                              </span>
                            </span>
                          </button>
                          {canReorder ? (
                            <div
                              className={
                                PROFILE_EDITOR_SIDEBAR_HOVER_ACTIONS_CLASS
                              }
                            >
                              <Button
                                type="button"
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7 bg-transparent text-muted-foreground shadow-none hover:bg-transparent hover:text-foreground"
                                disabled={
                                  persistedIndex <= 0 ||
                                  reorderMutation.isPending
                                }
                                onClick={() => moveMaterial(item.id, -1)}
                              >
                                <ArrowUp className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                type="button"
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7 bg-transparent text-muted-foreground shadow-none hover:bg-transparent hover:text-foreground"
                                disabled={
                                  persistedIndex < 0 ||
                                  persistedIndex >= materials.length - 1 ||
                                  reorderMutation.isPending
                                }
                                onClick={() => moveMaterial(item.id, 1)}
                              >
                                <ArrowDown className="h-3.5 w-3.5" />
                              </Button>
                            </div>
                          ) : null}
                        </CapsuleStripeListItem>
                      );
                    })}
                  </CapsuleStripeList>
                  <div className={PROFILE_EDITOR_SIDEBAR_STICKY_ACTION_CLASS}>
                    <Button
                      type="button"
                      variant="outline"
                      className="w-full border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700"
                      onClick={beginCreate}
                    >
                      <Plus className="mr-2 h-4 w-4" />
                      {t("profiles:detail.workflow.materials.add")}
                    </Button>
                  </div>
                </div>
              </CardListScrollBody>
            </div>
            <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
              {isCreating || selected ? (
                <div className="min-h-[62px] shrink-0 p-3">
                  <div className="min-w-0">
                    <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                      {isCreating
                        ? t("profiles:detail.workflow.materials.create")
                        : (selected?.title ?? "")}
                    </div>
                    <CardDescription className="truncate text-xs text-slate-500 dark:text-slate-400">
                      {isCreating
                        ? t(
                          "profiles:detail.workflow.materials.createDescription",
                        )
                        : `${(selected?.kind ?? "").replaceAll("_", " ")} · ${t("profiles:detail.workflow.materials.usedBy", { count: selected?.reference_step_ids.length ?? 0 })}`}
                    </CardDescription>
                  </div>
                </div>
              ) : null}
              <div
                data-creating={isCreating}
                data-selected={Boolean(selected)}
                data-has-header={Boolean(isCreating || selected)}
                data-uploaded-file={selected?.kind === "uploaded_file"}
                data-markdown-editor={creationKind === "markdown_file"}
                className={`min-h-0 flex-1 ${PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS} [&>div>div.grid.gap-2]:grid-cols-[5rem_minmax(0,1fr)] [&>div>div.grid.gap-2]:items-start [&>div>div.grid.gap-2:has(textarea)]:items-stretch [&>div>div.grid.gap-2:has(textarea)]:min-h-0 [&>div>div.grid.gap-2>label]:pt-2.5 [&>div>div.grid.gap-2>label]:text-right [&[data-has-header=true]>div>div:first-child]:hidden [&[data-has-header=true]>div>div:nth-child(2)]:!mt-0 [&[data-selected=true]>div>div:first-child>button]:hidden [&[data-uploaded-file=true]>div>pre]:hidden [&[data-markdown-editor=true]]:flex [&[data-markdown-editor=true]]:flex-col [&[data-markdown-editor=true]]:overflow-hidden [&:not([data-markdown-editor=true])]:overflow-y-auto [&[data-markdown-editor=true]>div>div.grid.gap-2:not(:has(textarea))]:shrink-0`}
              >
                {isCreating || selected ? (
                  <div
                    className={
                      creationKind === "markdown_file"
                        ? "flex min-h-0 flex-1 flex-col gap-4"
                        : "space-y-4"
                    }
                  >
                    <div>
                      <h3 className="font-semibold">
                        {isCreating
                          ? t("profiles:detail.workflow.materials.create")
                          : (selected?.title ?? "")}
                      </h3>
                      <p className="text-sm text-muted-foreground">
                        {isCreating
                          ? t(
                            "profiles:detail.workflow.materials.createDescription",
                          )
                          : `${(selected?.kind ?? "").replaceAll("_", " ")} · ${t("profiles:detail.workflow.materials.usedBy", { count: selected?.reference_step_ids.length ?? 0 })}`}
                      </p>
                    </div>
                    <div className="grid gap-2">
                      <Label htmlFor="workflow-material-title">
                        {t("profiles:detail.workflow.materials.materialTitle")}
                      </Label>
                      <Input
                        ref={titleInputRef}
                        id="workflow-material-title"
                        value={title}
                        onChange={(event) => {
                          const nextTitle = event.target.value;
                          setTitle(nextTitle);
                          updateSelectedDraft((draft) => ({
                            ...draft,
                            title: nextTitle,
                          }));
                        }}
                        placeholder={t(
                          "profiles:detail.workflow.materials.materialTitle",
                        )}
                      />
                    </div>
                    <div className="grid gap-2">
                      <Label>
                        {t("profiles:detail.workflow.materials.kind")}
                      </Label>
                      <Select
                        value={creationKind}
                        onValueChange={(value) => {
                          const nextKind = value as
                            | "external_url"
                            | "markdown_file"
                            | "uploaded_file";
                          setCreationKind(nextKind);
                          updateSelectedDraft((draft) => ({
                            ...draft,
                            kind: nextKind,
                          }));
                        }}
                        disabled={
                          selected?.kind === "uploaded_file" && !isCreating
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="external_url">
                            {t("profiles:detail.workflow.materials.createUrl")}
                          </SelectItem>
                          <SelectItem value="markdown_file">
                            {t(
                              "profiles:detail.workflow.materials.createMarkdown",
                            )}
                          </SelectItem>
                          <SelectItem value="uploaded_file">
                            {t("profiles:detail.workflow.materials.uploadFile")}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    {creationKind === "external_url" ? (
                      <div className="grid gap-2">
                        <Label htmlFor="workflow-material-url">URL</Label>
                        <div className="group relative">
                          <Input
                            id="workflow-material-url"
                            className={
                              selected?.external_url ? "pr-10" : undefined
                            }
                            value={url}
                            onChange={(event) => {
                              const nextUrl = event.target.value;
                              setUrl(nextUrl);
                              updateSelectedDraft((draft) => ({
                                ...draft,
                                external_url: nextUrl,
                              }));
                            }}
                            placeholder={t(
                              "profiles:detail.workflow.materials.urlPlaceholder",
                            )}
                          />
                          {selected?.external_url ? (
                            <Button
                              asChild
                              type="button"
                              size="icon"
                              variant="ghost"
                              className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                            >
                              <a
                                href={selected.external_url}
                                target="_blank"
                                rel="noreferrer"
                                aria-label={t(
                                  "profiles:detail.workflow.materials.openExternalUrl",
                                )}
                              >
                                <ExternalLink className="h-3.5 w-3.5" />
                              </a>
                            </Button>
                          ) : null}
                        </div>
                      </div>
                    ) : null}
                    {creationKind === "markdown_file" ? (
                      <MaterialMarkdownEditorField
                        editorKey={markdownEditorKey}
                        initialValue={markdownEditorSeed}
                        onChange={handleMarkdownChange}
                        onOpenPreview={handleOpenMarkdownPreview}
                        placeholder={t(
                          "profiles:detail.workflow.materials.markdownPlaceholder",
                        )}
                        previewLabel={t(
                          "profiles:detail.workflow.materials.preview",
                          { defaultValue: "Preview" },
                        )}
                      />
                    ) : null}
                    {creationKind === "uploaded_file" ? (
                      <div className="grid gap-2">
                        <Label htmlFor="workflow-material-file">
                          {t("profiles:detail.workflow.materials.uploadFile")}
                        </Label>
                        {selected && !isCreating ? (
                          <div className="group relative">
                            <Input
                              ref={uploadInputRef}
                              id="workflow-material-file"
                              type="file"
                              className="sr-only"
                              accept=".md,.js,.mjs,.cjs,.py,.pdf,.json,.yaml,.yml,.toml,.docx,.xlsx"
                              disabled={uploadMutation.isPending}
                              onChange={(event) => {
                                const file = event.target.files?.[0];
                                if (file) uploadMutation.mutate(file);
                              }}
                            />
                            <button
                              type="button"
                              className="flex h-10 w-full min-w-0 items-center gap-2 rounded-md border bg-background px-3 pr-10 text-left text-sm transition-colors hover:bg-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                              title={
                                selected.relative_path ??
                                selected.original_filename ??
                                selected.title
                              }
                              onClick={() => uploadInputRef.current?.click()}
                            >
                              <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
                              <span className="truncate">
                                {selected.relative_path ??
                                  selected.original_filename ??
                                  selected.title}
                              </span>
                            </button>
                            <Button
                              type="button"
                              size="icon"
                              variant="ghost"
                              className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                              title="Preview file"
                              aria-label="Preview file"
                              onClick={() => setIsPreviewOpen(true)}
                            >
                              <Eye className="h-3.5 w-3.5" />
                            </Button>
                          </div>
                        ) : (
                          <Input
                            ref={uploadInputRef}
                            id="workflow-material-file"
                            type="file"
                            accept=".md,.js,.mjs,.cjs,.py,.pdf,.json,.yaml,.yml,.toml,.docx,.xlsx"
                            disabled={uploadMutation.isPending}
                            onChange={(event) => {
                              const file = event.target.files?.[0];
                              if (file) uploadMutation.mutate(file);
                            }}
                          />
                        )}
                      </div>
                    ) : null}
                  </div>
                ) : (
                  <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                    <ShieldCheck className="mr-2 h-4 w-4" />
                    {t("profiles:detail.workflow.materials.selectDetail")}
                  </div>
                )}
              </div>
              {isPreviewOpen &&
                (creationKind === "markdown_file" ||
                  selected?.kind === "uploaded_file" ||
                  selected?.kind === "markdown_file") ? (
                // Transparent shell: only the rounded preview frame is opaque so it masks the form beneath.
                <div className="absolute inset-x-0 bottom-0 top-[62px] z-20 flex min-h-0 flex-col bg-transparent">
                  {/*
                    Frozen inset: PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS (`px-3 pb-3 pt-1`).
                    `pt-1` keeps this preview frame aligned with the left materials list
                    scroll body and the workflow step detail pane. Do not change to `p-3`.
                    Overlay hover buttons use `right-4 top-2`: `right-4` offsets
                    `px-3` vs `pt-1` so the visual gap from the preview frame matches
                    the top gap. Do not set `right-2` to "match" `top-2` in class names.
                  */}
                  <div
                    className={`group relative flex min-h-0 flex-1 flex-col ${PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS}`}
                  >
                    <CardListScrollBody className="min-h-0 flex-1">
                      {/*
											  Document-flow content inside the rounded frame. CardListScrollBody
											  is the only vertical scroller (shade attaches to it).
											*/}
                      <div className="bg-background p-3">
                        {creationKind === "markdown_file" || selectedIsText ? (
                          <MaterialFilePreview
                            content={previewContent}
                            extension={
                              creationKind === "markdown_file"
                                ? "md"
                                : selectedExtension
                            }
                            markdownMode={markdownPreviewMode}
                          />
                        ) : (
                          <p className="text-sm text-muted-foreground">
                            {t(
                              "profiles:detail.workflow.materials.previewUnavailable",
                            )}
                          </p>
                        )}
                      </div>
                    </CardListScrollBody>
                    <div className="absolute right-4 top-2 z-10 flex items-center gap-1">
                      {creationKind === "markdown_file" ||
                        (selectedExtension === "md" && selectedIsText) ? (
                        <Button
                          type="button"
                          size="icon"
                          variant="ghost"
                          className={MATERIAL_PREVIEW_HOVER_BUTTON_CLASS}
                          title={
                            markdownPreviewMode === "rendered"
                              ? "Show source"
                              : "Show preview"
                          }
                          aria-label={
                            markdownPreviewMode === "rendered"
                              ? "Show source"
                              : "Show preview"
                          }
                          onClick={() =>
                            setMarkdownPreviewMode((mode) =>
                              mode === "rendered" ? "source" : "rendered",
                            )
                          }
                        >
                          {markdownPreviewMode === "rendered" ? (
                            <Code2 className="h-4 w-4" />
                          ) : (
                            <Eye className="h-4 w-4" />
                          )}
                        </Button>
                      ) : null}
                      {desktopLocalFileActionsAvailable &&
                        selected &&
                        !isCreating ? (
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              type="button"
                              size="icon"
                              variant="ghost"
                              className={`${MATERIAL_PREVIEW_HOVER_BUTTON_CLASS} data-[state=open]:opacity-100`}
                              aria-label="File actions"
                            >
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem
                              onSelect={() => void runDesktopFileAction("open")}
                            >
                              <FileText className="mr-2 h-4 w-4" />
                              Open with default app
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onSelect={() =>
                                void runDesktopFileAction("reveal")
                              }
                            >
                              <FolderOpen className="mr-2 h-4 w-4" />
                              Show in Finder or Explorer
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      ) : null}
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        className={MATERIAL_PREVIEW_HOVER_BUTTON_CLASS}
                        aria-label="Close preview"
                        onClick={() => setIsPreviewOpen(false)}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              ) : null}
              {!isPreviewOpen && (isCreating || selected) ? (
                <div className="flex shrink-0 items-center p-3">
                  {isCreating && selectedDraft ? (
                    <Button
                      type="button"
                      size="icon"
                      variant="ghost"
                      className="h-9 w-9 text-muted-foreground hover:text-foreground"
                      aria-label={t("profiles:form.buttons.cancel")}
                      onClick={() => cancelDraft(selectedDraft.material_id)}
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  ) : selected ? (
                    <Button
                      type="button"
                      size="icon"
                      variant="ghost"
                      className="h-9 w-9 text-destructive hover:bg-destructive/10 hover:text-destructive"
                      aria-label={t(
                        "profiles:detail.workflow.materials.delete",
                      )}
                      onClick={() => setDeleteConfirmationOpen(true)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  ) : null}
                  {creationKind !== "uploaded_file" ||
                    (selected && !isCreating) ? (
                    <Button
                      type="button"
                      size="sm"
                      className="ml-auto"
                      disabled={!canSaveMaterial() || saveMutation.isPending}
                      onClick={() => saveMutation.mutate(creationKind)}
                    >
                      {isCreating
                        ? creationKind === "external_url"
                          ? t("profiles:detail.workflow.materials.createUrl")
                          : t(
                            "profiles:detail.workflow.materials.createMarkdown",
                          )
                        : t("profiles:detail.workflow.save")}
                    </Button>
                  ) : null}
                </div>
              ) : null}
              <AlertDialog
                open={deleteConfirmationOpen}
                onOpenChange={setDeleteConfirmationOpen}
              >
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>
                      {t(
                        "profiles:detail.workflow.materials.deleteConfirmation.title",
                      )}
                    </AlertDialogTitle>
                    <AlertDialogDescription>
                      {t(
                        "profiles:detail.workflow.materials.deleteConfirmation.description",
                        { count: selected?.reference_step_ids.length ?? 0 },
                      )}
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>{t("common:cancel")}</AlertDialogCancel>
                    <AlertDialogAction onClick={() => deleteMutation.mutate()}>
                      {t(
                        "profiles:detail.workflow.materials.deleteConfirmation.confirm",
                      )}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </div>
          </ResizableSplitPane>
        </CardContent>
      </Card>
    </div>
  );
}
