import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, ArrowRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { surfaceReviewsApi } from "../lib/api";
import {
  getInitialSurfaceReviewOwnerKey,
  getSurfaceReviewOwnerKey,
} from "../lib/surface-reviews";
import type {
  SurfaceIntentPreviewData,
  SurfaceIntentResolutionAction,
  SurfaceReviewOwner,
} from "../lib/types";
import { Badge } from "./ui/badge";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";

const REVIEW_ACTOR = "mcpmate-board";

interface SurfaceReviewDialogProps {
  reviewItemId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preferredOwner?: SurfaceReviewOwner | null;
}

function formatValue(value: unknown): string {
  if (value === undefined || value === null) {
    return "";
  }
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function isReviewConflict(error: Error): boolean {
  return /\b(conflict|changed|stale)\b/i.test(error.message);
}

export function SurfaceReviewDialog({
  reviewItemId,
  open,
  onOpenChange,
  preferredOwner,
}: SurfaceReviewDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedOwnerKey, setSelectedOwnerKey] = useState("");
  const [intentAction, setIntentAction] =
    useState<SurfaceIntentResolutionAction>("keep_intent");
  const [newRefId, setNewRefId] = useState("");
  const [preview, setPreview] = useState<SurfaceIntentPreviewData | null>(null);
  const [actionError, setActionError] = useState<Error | null>(null);

  const detailQuery = useQuery({
    queryKey: ["surfaceReview", reviewItemId],
    queryFn: () => {
      if (!reviewItemId) {
        throw new Error(
          t("surfaceReview:errors.validation.itemRequired", {
            defaultValue: "A Surface review item is required.",
          }),
        );
      }
      return surfaceReviewsApi.get(reviewItemId);
    },
    enabled: open && !!reviewItemId,
    retry: 1,
  });
  const item = detailQuery.data;

  const selectedOwner = useMemo(
    () =>
      item?.owners.find(
        (owner) => getSurfaceReviewOwnerKey(owner) === selectedOwnerKey,
      ) ??
      null,
    [item?.owners, selectedOwnerKey],
  );

  useEffect(() => {
    if (!item) return;
    setSelectedOwnerKey(
      getInitialSurfaceReviewOwnerKey(item.owners, preferredOwner ?? null),
    );
  }, [item, preferredOwner]);

  useEffect(() => {
    setPreview(null);
    setActionError(null);
    setNewRefId("");
    setIntentAction("keep_intent");
  }, [reviewItemId]);

  const invalidateReviewQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["surfaceReviews"] }),
      queryClient.invalidateQueries({ queryKey: ["surfaceReview", reviewItemId] }),
    ]);
  };

  const targetMutation = useMutation({
    mutationFn: async (action: "approve" | "reject") => {
      if (!item) {
        throw new Error(
          t("surfaceReview:errors.validation.detailUnavailable", {
            defaultValue: "Surface review detail is unavailable.",
          }),
        );
      }
      if (item.binding_generation === null) {
        throw new Error(
          t("surfaceReview:errors.validation.publicationUnavailable", {
            defaultValue: "Surface review has no active publication.",
          }),
        );
      }
      const request = {
        expected_target_key: item.target_key,
        expected_binding_generation: item.binding_generation,
        actor: REVIEW_ACTOR,
      };
      return action === "approve"
        ? surfaceReviewsApi.approveTarget(item.review_item_id, request)
        : surfaceReviewsApi.rejectTarget(item.review_item_id, request);
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateReviewQueries();
      onOpenChange(false);
    },
    onError: (error) => setActionError(error as Error),
  });

  const previewMutation = useMutation({
    mutationFn: async () => {
      if (!item || !selectedOwner) {
        throw new Error(
          t("surfaceReview:errors.validation.ownerRequired", {
            defaultValue: "Select an affected configuration.",
          }),
        );
      }
      if (intentAction === "rebind_ref" && !newRefId.trim()) {
        throw new Error(
          t("surfaceReview:errors.validation.refRequired", {
            defaultValue: "Enter a Capability Ref ID.",
          }),
        );
      }
      return surfaceReviewsApi.previewIntent(item.review_item_id, {
        action: intentAction,
        owner: selectedOwner,
        new_ref_id: intentAction === "rebind_ref" ? newRefId.trim() : undefined,
      });
    },
    onSuccess: (data) => {
      setActionError(null);
      setPreview(data);
    },
    onError: (error) => setActionError(error as Error),
  });

  const resolveMutation = useMutation({
    mutationFn: async () => {
      if (!item || !selectedOwner || !preview) {
        throw new Error(
          t("surfaceReview:errors.validation.previewRequired", {
            defaultValue: "Preview the intent impact before confirming.",
          }),
        );
      }
      if (item.binding_generation === null) {
        throw new Error(
          t("surfaceReview:errors.validation.publicationUnavailable", {
            defaultValue: "Surface review has no active publication.",
          }),
        );
      }
      return surfaceReviewsApi.resolveIntent(item.review_item_id, {
        action: preview.action,
        owner: selectedOwner,
        new_ref_id:
          preview.action === "rebind_ref" ? newRefId.trim() : undefined,
        expected_owner_revision: preview.owner_revision,
        impact_token: preview.impact_token,
        expected_target_key: item.target_key,
        expected_binding_generation: item.binding_generation,
        actor: REVIEW_ACTOR,
      });
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateReviewQueries();
      onOpenChange(false);
    },
    onError: (error) => setActionError(error as Error),
  });

  const isPending =
    targetMutation.isPending ||
    previewMutation.isPending ||
    resolveMutation.isPending;
  const canReview = item?.lifecycle === "pending";
  const acceptsTargetAction =
    canReview &&
    (item.target_key.startsWith("capability:") ||
      item.target_key.startsWith("reappeared:"));
  const acceptsIntentAction =
    canReview &&
    (item.target_key.startsWith("missing:") ||
      item.policy_action === "manual_rebind");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] max-w-4xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {t("surfaceReview:dialog.title", {
              defaultValue: "Capability change review",
            })}
          </DialogTitle>
          <DialogDescription>
            {t("surfaceReview:dialog.description", {
              defaultValue:
                "The capability intent is unchanged, but its effective content changed.",
            })}
          </DialogDescription>
        </DialogHeader>

        {detailQuery.isPending ? (
          <div className="space-y-3 py-6">
            <div className="h-5 w-48 animate-pulse rounded bg-muted" />
            <div className="h-32 animate-pulse rounded bg-muted" />
          </div>
        ) : detailQuery.isError ? (
          <div className="rounded-md border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive">
            {t("surfaceReview:errors.detail", {
              defaultValue: "Unable to load this capability review.",
            })}
            <div className="mt-1 text-xs">{String(detailQuery.error)}</div>
          </div>
        ) : item ? (
          <div className="space-y-5">
            <div className="flex flex-wrap items-center gap-2 text-sm">
              <Badge variant="outline">{item.change_class}</Badge>
              <Badge variant={canReview ? "default" : "secondary"}>
                {item.lifecycle}
              </Badge>
              <code className="rounded bg-muted px-2 py-1 text-xs">
                {item.ref_id}
              </code>
            </div>

            <div className="space-y-2">
              <div className="text-sm font-medium">
                {t("surfaceReview:dialog.owner", {
                  defaultValue: "Affected configuration",
                })}
              </div>
              {item.owners.length > 0 ? (
                <Select
                  value={selectedOwnerKey}
                  onValueChange={(value) => {
                    setSelectedOwnerKey(value);
                    setPreview(null);
                  }}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {item.owners.map((owner) => (
                      <SelectItem
                        key={getSurfaceReviewOwnerKey(owner)}
                        value={getSurfaceReviewOwnerKey(owner)}
                      >
                        {owner.owner_type} · {owner.owner_id}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              ) : (
                <div className="text-sm text-muted-foreground">
                  {t("surfaceReview:dialog.noActiveOwner", {
                    defaultValue: "No active configuration",
                  })}
                </div>
              )}
            </div>

            <div className="space-y-3">
              {item.field_diff.map((diff) => (
                <div
                  key={diff.path}
                  className="rounded-md border border-dashed border-amber-400 bg-amber-50/40 p-3 dark:border-amber-700 dark:bg-amber-950/20"
                >
                  <div className="mb-2 font-mono text-xs font-medium">
                    {diff.path}
                  </div>
                  <div className="grid gap-3 md:grid-cols-[1fr_auto_1fr]">
                    <div>
                      <div className="mb-1 text-xs font-medium text-muted-foreground">
                        {t("surfaceReview:dialog.before", {
                          defaultValue: "Before",
                        })}
                      </div>
                      <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-background p-2 text-xs">
                        {formatValue(diff.before) ||
                          t("surfaceReview:dialog.noValue", {
                            defaultValue: "Not present",
                          })}
                      </pre>
                    </div>
                    <ArrowRight className="mt-7 hidden h-4 w-4 text-muted-foreground md:block" />
                    <div>
                      <div className="mb-1 text-xs font-medium text-muted-foreground">
                        {t("surfaceReview:dialog.target", {
                          defaultValue: "Target",
                        })}
                      </div>
                      <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-background p-2 text-xs">
                        {formatValue(diff.target) ||
                          t("surfaceReview:dialog.noValue", {
                            defaultValue: "Not present",
                          })}
                      </pre>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {acceptsIntentAction ? (
              <div className="space-y-3 rounded-md border p-3">
                <div className="grid gap-2 md:grid-cols-[1fr_auto]">
                  <Select
                    value={intentAction}
                    onValueChange={(value) => {
                      setIntentAction(value as SurfaceIntentResolutionAction);
                      setPreview(null);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="keep_intent">
                        {t("surfaceReview:actions.keepIntent", {
                          defaultValue: "Keep intent",
                        })}
                      </SelectItem>
                      <SelectItem value="remove_intent">
                        {t("surfaceReview:actions.removeIntent", {
                          defaultValue: "Remove from configuration",
                        })}
                      </SelectItem>
                      {item.policy_action === "manual_rebind" ? (
                        <SelectItem value="rebind_ref">
                          {t("surfaceReview:actions.rebindRef", {
                            defaultValue: "Bind another capability",
                          })}
                        </SelectItem>
                      ) : null}
                    </SelectContent>
                  </Select>
                  <Button
                    variant="outline"
                    onClick={() => previewMutation.mutate()}
                    disabled={isPending}
                  >
                    {t("surfaceReview:actions.preview", {
                      defaultValue: "Preview impact",
                    })}
                  </Button>
                </div>
                {intentAction === "rebind_ref" ? (
                  <Input
                    value={newRefId}
                    onChange={(event) => {
                      setNewRefId(event.target.value);
                      setPreview(null);
                    }}
                    placeholder={t("surfaceReview:dialog.rebindPlaceholder", {
                      defaultValue: "Capability Ref ID",
                    })}
                  />
                ) : null}
                {preview ? (
                  <div className="space-y-2 rounded-md bg-muted p-3 text-sm">
                    <div className="flex items-center justify-between gap-3">
                      <span>
                        {t("surfaceReview:dialog.impactedConsumers", {
                          count: preview.impacted_consumer_ids.length,
                          defaultValue: "{{count}} affected clients",
                        })}
                      </span>
                      <Button
                        onClick={() => resolveMutation.mutate()}
                        disabled={isPending}
                      >
                        {t("surfaceReview:actions.confirm", {
                          defaultValue: "Confirm",
                        })}
                      </Button>
                    </div>
                    <ul className="space-y-1 text-xs text-muted-foreground">
                      {preview.impacted_consumer_ids.map((consumerId) => (
                        <li key={consumerId}>
                          <code>{consumerId}</code>
                        </li>
                      ))}
                    </ul>
                  </div>
                ) : null}
              </div>
            ) : null}

            {actionError ? (
              <div className="flex gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                <div>
                  {isReviewConflict(actionError)
                    ? t("surfaceReview:errors.conflict", {
                        defaultValue:
                          "The Surface changed while this review was open. Refresh and review the latest state.",
                      })
                    : t("surfaceReview:errors.action", {
                        defaultValue: "Unable to complete the review action.",
                      })}
                  <div className="mt-1 text-xs">{actionError.message}</div>
                </div>
              </div>
            ) : null}
          </div>
        ) : null}

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("surfaceReview:actions.cancel", { defaultValue: "Cancel" })}
          </Button>
          {acceptsTargetAction ? (
            <>
              <Button
                variant="outline"
                onClick={() => targetMutation.mutate("reject")}
                disabled={isPending}
              >
                {t("surfaceReview:actions.rejectTarget", {
                  defaultValue: "Reject target and keep unavailable",
                })}
              </Button>
              <Button
                onClick={() => targetMutation.mutate("approve")}
                disabled={isPending}
              >
                {t("surfaceReview:actions.approveTarget", {
                  defaultValue: "Approve update",
                })}
              </Button>
            </>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
