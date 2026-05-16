import { useEffect, useMemo, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import { Button } from "../../../components/ui/button";
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
} from "../../../components/ui/drawer";
import type { ServerEntryData } from "../../../lib/types";
import { ServerEntryTransportBadge } from "./server-entry-transport-badge";

const REDACTED_VALUE = "[REDACTED]";
const AVAILABILITY_CLASS_NAMES = {
  importable:
    "rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-medium text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300",
  managed:
    "rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-medium text-slate-600 dark:bg-slate-800 dark:text-slate-300",
  notImportable:
    "rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300",
} as const;
const AVAILABILITY_LABELS = {
  importable: {
    defaultValue: "Importable",
    key: "detail.importReview.fields.importable",
  },
  managed: {
    defaultValue: "Managed",
    key: "detail.importReview.fields.managedByMcpmate",
  },
  notImportable: {
    defaultValue: "Not Importable",
    key: "detail.importReview.fields.notImportable",
  },
} as const;

type EntryAvailability = keyof typeof AVAILABILITY_CLASS_NAMES;

interface ImportPreviewEntry {
  args: string[];
  command?: string | null;
  env: Record<string, string>;
  headers: Record<string, string>;
  import_status?: string;
  managed_by_mcpmate?: boolean;
  issue?: string | null;
  name: string;
  skip_reason?: string | null;
  transport?: string;
  url?: string | null;
}

interface SkippedServerPreview {
  name: string;
  reason: string;
}

interface ImportPreviewSnapshot {
  entries: ImportPreviewEntry[];
  summary: {
    detectedCount: number;
    failedCount: number;
    importableCount: number;
    managedCount: number;
    skippedCount: number;
    skippedServers: SkippedServerPreview[];
  };
}

function isImportableEntry(entry: ServerEntryData): boolean {
  return (
    entry.import_status === "importable" && entry.managed_by_mcpmate !== true
  );
}

function isSkippedEntry(entry: ServerEntryData): boolean {
  return entry.import_status === "skipped";
}

function getEntryAvailability(entry: ServerEntryData): EntryAvailability {
  if (entry.managed_by_mcpmate === true) {
    return "managed";
  }
  if (isImportableEntry(entry)) {
    return "importable";
  }
  return "notImportable";
}

function getAvailabilityLabel(
  t: TFunction<"clients">,
  availability: EntryAvailability,
): string {
  const label = AVAILABILITY_LABELS[availability];
  return t(label.key, { defaultValue: label.defaultValue });
}

function redactSecrets(values: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    Object.keys(values).map((key) => [key, REDACTED_VALUE]),
  );
}

function buildPreviewEntry(entry: ServerEntryData): ImportPreviewEntry {
  return {
    name: entry.name,
    transport: entry.transport,
    import_status: entry.import_status,
    managed_by_mcpmate: entry.managed_by_mcpmate,
    skip_reason: entry.skip_reason,
    issue: entry.issue,
    command: entry.command,
    args: entry.args,
    env: redactSecrets(entry.env),
    headers: redactSecrets(entry.headers),
    url: entry.url,
  };
}

function buildSkippedServerPreview(
  entry: ServerEntryData,
): SkippedServerPreview {
  return {
    name: entry.name,
    reason: entry.skip_reason ?? entry.issue ?? "config_unrecognized",
  };
}

interface ImportPreviewModel {
  importableNames: string[];
  snapshot: ImportPreviewSnapshot;
}

function buildImportPreviewModel(entries: ServerEntryData[]): ImportPreviewModel {
  const managedCount = entries.filter(
    (entry) => entry.managed_by_mcpmate === true,
  ).length;
  const skippedServers = entries
    .filter(isSkippedEntry)
    .map(buildSkippedServerPreview);
  const importableNames = entries
    .filter(isImportableEntry)
    .map((entry) => entry.name);

  return {
    importableNames,
    snapshot: {
      entries: entries.map(buildPreviewEntry),
      summary: {
        detectedCount: entries.length,
        failedCount: 0,
        importableCount: importableNames.length,
        managedCount,
        skippedCount: skippedServers.length,
        skippedServers,
      },
    },
  };
}

interface ClientImportReviewDrawerProps {
  entries: ServerEntryData[];
  isImporting: boolean;
  isLoading: boolean;
  onImport: (selectedServerNames: string[]) => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}

export function ClientImportReviewDrawer({
  entries,
  isImporting,
  isLoading,
  onImport,
  onOpenChange,
  open,
}: ClientImportReviewDrawerProps): JSX.Element {
  const { t } = useTranslation("clients");
  const { importableNames, snapshot: importPreviewSnapshot } = useMemo(
    () => buildImportPreviewModel(entries),
    [entries],
  );
  const {
    detectedCount,
    importableCount,
    managedCount,
    skippedCount,
  } = importPreviewSnapshot.summary;
  const importableKey = importableNames.join("\u0000");
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());
  const selectedImportableCount = selectedNames.size;

  useEffect(() => {
    if (!open) return;
    setSelectedNames(new Set(importableNames));
  }, [importableKey, importableNames, open]);

  const toggleSelectedName = (name: string) => {
    setSelectedNames((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent>
        <DrawerHeader>
          <DrawerTitle>
            {t("detail.importReview.title", {
              defaultValue: "Import Review",
            })}
          </DrawerTitle>
          <DrawerDescription>
            {t("detail.importReview.description", {
              defaultValue:
                "Summary of servers detected from current client config.",
            })}
          </DrawerDescription>
        </DrawerHeader>
        <div className="p-4 text-sm flex flex-col gap-4 max-h-[70vh]">
          {isLoading ? (
            <div className="h-16 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
          ) : (
            <div className="flex-1 min-h-0 flex flex-col gap-4">
              {entries.length > 0 && (
                <div className="rounded border">
                  <div className="px-3 py-2 text-xs text-slate-500 border-b">
                    {t("detail.importReview.sections.detectedEntries", {
                      defaultValue: "Detected entries",
                    })}
                  </div>
                  <ul className="divide-y max-h-[30vh] overflow-auto">
                    {entries.map((entry) => {
                      const availability = getEntryAvailability(entry);
                      const importable = availability === "importable";
                      const selected = selectedNames.has(entry.name);
                      const availabilityLabel = getAvailabilityLabel(
                        t,
                        availability,
                      );
                      return (
                        <li
                          key={entry.name}
                          className="px-3 py-2 flex items-center justify-between gap-3 text-xs"
                        >
                          <label className="flex min-w-0 flex-1 items-center gap-2">
                            <input
                              type="checkbox"
                              checked={selected}
                              disabled={!importable}
                              onChange={() => toggleSelectedName(entry.name)}
                              className="h-4 w-4 shrink-0"
                            />
                            <span className="truncate font-mono">
                              {entry.name}
                            </span>
                          </label>
                          <div className="flex shrink-0 items-center gap-2">
                            <ServerEntryTransportBadge entry={entry} />
                            <span className={AVAILABILITY_CLASS_NAMES[availability]}>
                              {availabilityLabel}
                            </span>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              <div className="grid grid-cols-[120px_1fr] gap-y-2 gap-x-4 text-sm leading-6">
                <div className="text-slate-500">
                  {t("detail.importReview.fields.selected", {
                    defaultValue: "Selected",
                  })}
                </div>
                <div>{selectedImportableCount}</div>
                <div className="text-slate-500">
                  {t("detail.importReview.fields.detected", {
                    defaultValue: "Detected",
                  })}
                </div>
                <div>{detectedCount}</div>
                <div className="text-slate-500">
                  {t("detail.importReview.fields.importable", {
                    defaultValue: "Importable",
                  })}
                </div>
                <div>{importableCount}</div>
                <div className="text-slate-500">
                  {t("detail.importReview.fields.managedByMcpmate", {
                    defaultValue: "Managed",
                  })}
                </div>
                <div>{managedCount}</div>
                <div className="text-slate-500">
                  {t("detail.importReview.fields.skipped", {
                    defaultValue: "Skipped",
                  })}
                </div>
                <div>{skippedCount}</div>
              </div>
              <details className="mt-2 flex-1 min-h-0">
                <summary className="text-xs text-slate-500 cursor-pointer">
                  {t("detail.importReview.sections.raw", {
                    defaultValue: "Sanitized preview JSON",
                  })}
                </summary>
                <pre className="text-xs bg-slate-50 dark:bg-slate-900 p-2 rounded overflow-auto flex-1 min-h-0 max-h-[40vh]">
                  {JSON.stringify(importPreviewSnapshot, null, 2)}
                </pre>
              </details>
            </div>
          )}
        </div>
        <DrawerFooter>
          <div className="flex w-full items-center justify-between">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t("detail.importReview.buttons.close", {
                defaultValue: "Close",
              })}
            </Button>
            {importableCount > 0 ? (
              <Button
                onClick={() => onImport(Array.from(selectedNames))}
                disabled={isImporting || selectedImportableCount === 0}
              >
                {t("detail.importReview.buttons.apply", {
                  defaultValue: "Apply Import",
                })}
              </Button>
            ) : (
              <div className="text-xs text-slate-500">
                {t("detail.importReview.states.noImportNeeded", {
                  defaultValue: "No import needed",
                })}
              </div>
            )}
          </div>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}
