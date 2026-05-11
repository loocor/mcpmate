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

interface ClientImportReviewDrawerProps {
  entries: ServerEntryData[];
  isImporting: boolean;
  isLoading: boolean;
  onImport: () => void;
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
}: ClientImportReviewDrawerProps) {
  const { t } = useTranslation("clients");
  const skippedServers = entries
    .filter((entry) => entry.import_status === "skipped")
    .map((entry) => ({
      name: entry.name,
      reason: entry.skip_reason ?? entry.issue ?? "config_unrecognized",
    }));
  const importableCount = entries.length - skippedServers.length;
  const importPreviewSnapshot = {
    entries,
    summary: {
      importableCount,
      skippedCount: skippedServers.length,
      failedCount: 0,
      skippedServers,
    },
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
                    {entries.map((entry) => (
                      <li
                        key={entry.name}
                        className="px-3 py-2 flex items-center justify-between text-xs"
                      >
                        <div className="font-mono">{entry.name}</div>
                        <ServerEntryTransportBadge entry={entry} />
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              <div className="grid grid-cols-[120px_1fr] gap-y-2 gap-x-4 text-sm leading-6">
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
                <div>{skippedServers.length}</div>
              </div>
              <details className="mt-2 flex-1 min-h-0">
                <summary className="text-xs text-slate-500 cursor-pointer">
                  {t("detail.importReview.sections.raw", {
                    defaultValue: "Raw preview JSON",
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
              <Button onClick={onImport} disabled={isImporting}>
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
