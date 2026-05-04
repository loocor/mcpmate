import type { TFunction } from "i18next";

import type { SkippedServer } from "./types";

export type SkippedQueryField = "existing" | "incoming";

const SKIPPED_REASON_LABELS: Record<
  string,
  { key: string; defaultValue: string }
> = {
  duplicate_name: {
    key: "serverImport.skippedReasons.duplicateName",
    defaultValue: "Duplicate name",
  },
  duplicate_fingerprint: {
    key: "serverImport.skippedReasons.duplicateFingerprint",
    defaultValue: "Duplicate fingerprint",
  },
  url_query_mismatch: {
    key: "serverImport.skippedReasons.urlQueryMismatch",
    defaultValue: "URL query mismatch",
  },
};

const SKIPPED_QUERY_FIELD_LABELS: Record<
  SkippedQueryField,
  { key: string; defaultValue: string }
> = {
  existing: {
    key: "serverImport.queryLabels.existing",
    defaultValue: "Existing query",
  },
  incoming: {
    key: "serverImport.queryLabels.incoming",
    defaultValue: "Incoming query",
  },
};

const translateCommon = (
  t: TFunction,
  key: string,
  defaultValue: string,
  options?: Record<string, string | number>,
) => t(key, { ns: "translation", defaultValue, ...options });

export const getSkippedReasonLabel = (reason: string, t: TFunction) => {
  const spec = SKIPPED_REASON_LABELS[reason];
  if (!spec) {
    return translateCommon(
      t,
      "serverImport.skippedReasons.unknown",
      "Unknown reason",
    );
  }
  return translateCommon(t, spec.key, spec.defaultValue);
};

export const getSkippedQueryFieldLabel = (
  field: SkippedQueryField,
  t: TFunction,
) => {
  const spec = SKIPPED_QUERY_FIELD_LABELS[field];
  return translateCommon(t, spec.key, spec.defaultValue);
};

export const formatNameList = (
  names: string[],
  t: TFunction,
  limit = 3,
): string => {
  if (!names.length) return "";
  if (names.length <= limit) return names.join(", ");
  const head = names.slice(0, limit).join(", ");
  const suffix = translateCommon(
    t,
    "serverImport.nameList.more",
    "+{{count}} more",
    { count: names.length - limit },
  );
  return `${head}, ${suffix}`;
};

const describeSkip = (detail: SkippedServer, t: TFunction): string => {
  const label = getSkippedReasonLabel(detail.reason, t);
  const parts: string[] = [label];
  if (detail.incoming_query || detail.existing_query) {
    const queryParts: string[] = [];
    if (detail.incoming_query) {
      queryParts.push(
        `${getSkippedQueryFieldLabel("incoming", t)}=${detail.incoming_query}`,
      );
    }
    if (detail.existing_query) {
      queryParts.push(
        `${getSkippedQueryFieldLabel("existing", t)}=${detail.existing_query}`,
      );
    }
    if (queryParts.length) {
      parts.push(queryParts.join(", "));
    }
  }
  return `${detail.name} (${parts.join("; ")})`;
};

export const summarizeSkipped = (
  details: SkippedServer[],
  t: TFunction,
): string => {
  if (!details.length) return "";
  return details.map((detail) => describeSkip(detail, t)).join("; ");
};
