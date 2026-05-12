import { useEffect, useMemo, useState } from "react";
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

interface ImportPreviewEntry {
  args: string[];
  command?: string | null;
  env: Record<string, string>;
  headers: Record<string, string>;
  import_status?: string;
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
    failedCount: number;
    importableCount: number;
    skippedCount: number;
    skippedServers: SkippedServerPreview[];
  };
}

function isImportableEntry(entry: ServerEntryData): boolean {
  return entry.import_status === "importable";
}

function isSkippedEntry(entry: ServerEntryData): boolean {
  return entry.import_status === "skipped";
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
        failedCount: 0,
        importableCount: importableNames.length,
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
  const { importableCount, skippedCount } = importPreviewSnapshot.summary;
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
                      const importable = isImportableEntry(entry);
                      const selected = selectedNames.has(entry.name);
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
                          <ServerEntryTransportBadge entry={entry} />
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
                  {t("detail.importReview.fields.importable", {
                    defaultValue: "Importable",
                  })}
                </div>
                <div>{importableCount}</div>
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
