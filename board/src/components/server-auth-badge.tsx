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
  t,
}: {
  authMode?: string | null;
  oauthStatus?: string | null;
  readiness?: OAuthReadiness | null;
  t: TFunction<"servers">;
}): ServerAuthBadgeDisplay {
  const normalizedMode = (authMode ?? "").toLowerCase();
  const normalizedStatus = (oauthStatus ?? "").toLowerCase();

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
  showLabel = true,
  onAction,
}: ServerAuthBadgeProps) {
  const { t } = useTranslation("servers");
  const display = resolveServerAuthBadgeDisplay({
    authMode,
    oauthStatus,
    readiness,
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
              <span className="inline-flex items-center">
                <AlertTriangle className="h-4 w-4 text-red-500 animate-pulse" />
              </span>
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
          className="w-fit cursor-pointer text-left text-sm text-red-600 underline underline-offset-2 transition-colors hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
        >
          {display.label}
        </button>
      );
    }

    return (
      <span className="text-sm text-red-600 dark:text-red-400">
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
