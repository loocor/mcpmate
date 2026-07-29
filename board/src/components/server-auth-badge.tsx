import { AlertTriangle, KeyRound, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { OAuthReadiness } from "../lib/oauth-readiness";
import { Badge } from "./ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";

interface ServerAuthBadgeProps {
  authMode?: string | null;
  oauthStatus?: string | null;
  readiness?: OAuthReadiness | null;
  showOff?: boolean;
  showLabel?: boolean;
  onAction?: () => void;
}

type ServerAuthBadgeDisplay =
  | { kind: "none" }
  | {
    kind: "badge";
    label: string;
    className: string;
    icon: ReactNode;
  }
  | {
    kind: "warning";
    label: string;
  };

function resolveServerAuthBadgeDisplay({
  authMode,
  oauthStatus,
  readiness,
  showOff,
  t,
}: {
  authMode?: string | null;
  oauthStatus?: string | null;
  readiness?: OAuthReadiness | null;
  showOff: boolean;
  t: TFunction<"servers">;
}): ServerAuthBadgeDisplay {
  const normalizedMode = (authMode ?? "").toLowerCase();
  const normalizedStatus = (oauthStatus ?? "").toLowerCase();

  if (!normalizedMode && showOff) {
    return {
      kind: "warning",
      label: t("entity.connectionTags.authOff", {
        defaultValue: "Off",
      }),
    };
  }

  if (normalizedMode === "header") {
    return {
      kind: "badge",
      label: t("entity.connectionTags.headerAuth", {
        defaultValue: "Header auth",
      }),
      className:
        "border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300",
      icon: <KeyRound className="h-3 w-3" />,
    };
  }

  if (normalizedMode !== "oauth") {
    return { kind: "none" };
  }

  if (readiness?.notice) {
    return {
      kind: "warning",
      label: t(readiness.notice.messageKey, {
        defaultValue: readiness.notice.defaultMessage,
      }),
    };
  }

  if (normalizedStatus === "expired" || normalizedStatus === "disconnected") {
    return {
      kind: "warning",
      label: t("entity.connectionTags.oauthWarning", {
        defaultValue: "Authorization expired — reauthorize required",
      }),
    };
  }

  return {
    kind: "badge",
    label: t("entity.connectionTags.oauth", {
      defaultValue: "OAuth",
    }),
    className:
      "border-emerald-200 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300",
    icon: <ShieldCheck className="h-3 w-3" />,
  };
}

export function ServerAuthBadge({
  authMode,
  oauthStatus,
  readiness,
  showOff = false,
  showLabel = true,
  onAction,
}: ServerAuthBadgeProps) {
  const { t } = useTranslation("servers");
  const display = resolveServerAuthBadgeDisplay({
    authMode,
    oauthStatus,
    readiness,
    showOff,
    t,
  });

  if (display.kind === "none") {
    return null;
  }

  if (display.kind === "warning") {
    if (!showLabel) {
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                aria-label={display.label}
                className="inline-flex items-center border-0 bg-transparent p-0"
              >
                <AlertTriangle className="h-4 w-4 text-red-500 animate-pulse" />
              </button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{display.label}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      );
    }

    if (onAction) {
      return (
        <button
          type="button"
          onClick={onAction}
          aria-label={display.label}
          className="inline-flex w-fit cursor-pointer items-center gap-1.5 rounded-full border border-red-200 bg-red-50 px-2.5 py-0.5 text-xs font-semibold text-red-700 transition-colors hover:border-red-300 hover:bg-red-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-400 focus-visible:ring-offset-2 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300 dark:hover:border-red-800 dark:hover:bg-red-950/70"
        >
          <AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" />
          {display.label}
        </button>
      );
    }

    return (
      <span className="inline-flex w-fit items-center gap-1.5 rounded-full border border-red-200 bg-red-50 px-2.5 py-0.5 text-xs font-semibold text-red-700 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300">
        <AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" />
        {display.label}
      </span>
    );
  }

  return (
    <Badge variant="outline" className={`gap-1.5 ${display.className}`}>
      {display.icon}
      {showLabel ? display.label : null}
    </Badge>
  );
}
