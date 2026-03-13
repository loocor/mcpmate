import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { JsonSchema, JsonValue } from "../types/json";
import { Badge } from "./ui/badge";

type LabelSet = {
  enumText: string;
  itemsText: string;
  itemsEnumText: string;
};

const asType = (t: JsonSchema["type"]): string | undefined => {
  if (!t) return undefined;
  return Array.isArray(t) ? t.join("|") : t;
};

const asEnumBadges = (values: JsonValue[] | undefined): React.ReactNode => {
  if (!values || !values.length) return null;
  return (
    <div className="flex flex-wrap gap-1">
      {values.map((v, i) => (
        <Badge key={i} variant="secondary" className="px-1.5 py-0 text-[10px]">
          {typeof v === "string" ? v : JSON.stringify(v)}
        </Badge>
      ))}
    </div>
  );
};

function ensureObjectType(schema: JsonSchema | undefined): JsonSchema | undefined {
  if (!schema) return schema;
  if (!schema.type && schema.properties) return { ...schema, type: "object" };
  return schema;
}

interface SchemaRowProps {
  name: string;
  schema: JsonSchema;
  required: boolean;
  labels: LabelSet;
  level?: number;
}

function SchemaRow({ name, schema, required, labels, level = 0 }: SchemaRowProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const childSchema = ensureObjectType(schema);
  const typeLabel = asType(childSchema?.type) ?? (childSchema?.properties ? "object" : undefined);

  const details: React.ReactNode[] = [];
  if (childSchema?.enum && childSchema.enum.length) {
    details.push(
      <div key={`${name}-enum`} className="flex items-center gap-1">
        <span className="text-slate-500">{labels.enumText}</span>
        {asEnumBadges(childSchema.enum)}
      </div>,
    );
  }

  // Array items
  let arrayItemsSchema: JsonSchema | undefined;
  if (childSchema?.type === "array") {
    const items = Array.isArray(childSchema.items)
      ? childSchema.items[0]
      : childSchema.items;
    if (items) {
      const itemsType = asType(items.type) ?? (items.properties ? "object" : undefined);
      details.push(
        <div key={`${name}-items`} className="flex items-center gap-1">
          <span className="text-slate-500">{labels.itemsText}</span>
          <code className="rounded bg-slate-100 px-1 py-0.5 dark:bg-slate-800">{itemsType ?? "unknown"}</code>
        </div>,
      );
      if (items.enum && items.enum.length) {
        details.push(
          <div key={`${name}-items-enum`} className="flex items-center gap-1">
            <span className="text-slate-500">{labels.itemsEnumText}</span>
            {asEnumBadges(items.enum)}
          </div>,
        );
      }
      // If array items have properties, we'll show them nested
      if ((items as JsonSchema).properties) {
        arrayItemsSchema = items as JsonSchema;
      }
    }
  }

  const hasNestedProperties = !!(childSchema?.properties || arrayItemsSchema);
  const nestedProperties = childSchema?.properties
    ? Object.entries(childSchema.properties)
    : arrayItemsSchema?.properties
    ? Object.entries(arrayItemsSchema.properties)
    : [];

  const nestedRequired = new Set<string>(
    childSchema?.required ?? arrayItemsSchema?.required ?? []
  );

  const { t } = useTranslation();

  return (
    <>
      <tr className="hover:bg-slate-50 dark:hover:bg-slate-800/50">
        <td className="border-b py-1 pr-2">
          <div className="flex items-center gap-1" style={{ paddingLeft: `${level * 16}px` }}>
            {hasNestedProperties ? (
              <button
                type="button"
                onClick={() => setIsExpanded(!isExpanded)}
                className="inline-flex items-center justify-center hover:bg-slate-200 dark:hover:bg-slate-700 rounded p-0.5 -ml-0.5"
              >
                {isExpanded ? (
                  <ChevronDown className="h-3 w-3" />
                ) : (
                  <ChevronRight className="h-3 w-3" />
                )}
              </button>
            ) : (
              <span className="inline-block w-4" />
            )}
            <span className="font-mono">{name}</span>
          </div>
        </td>
        <td className="border-b py-1 pr-2">{typeLabel ?? "-"}</td>
        <td className="border-b py-1 pr-2">
          {required
            ? t("servers:capabilityList.table.requiredYes", { defaultValue: "Yes" })
            : t("servers:capabilityList.table.requiredNo", { defaultValue: "No" })}
        </td>
        <td className="border-b py-1 pr-2">{childSchema?.description ?? ""}</td>
        <td className="border-b py-1 pr-2">
          {details.length ? (
            <div className="flex flex-col gap-1">{details}</div>
          ) : null}
        </td>
      </tr>
      {isExpanded && hasNestedProperties && nestedProperties.map(([childName, childSchema]) => (
        <SchemaRow
          key={childName}
          name={childName}
          schema={childSchema as JsonSchema}
          required={nestedRequired.has(childName)}
          labels={labels}
          level={level + 1}
        />
      ))}
    </>
  );
}

export interface SchemaTableProps {
  schema?: JsonSchema;
}

export function SchemaTable({ schema }: SchemaTableProps) {
  const { t } = useTranslation();
  const labels: LabelSet = {
    enumText: t("servers:capabilityList.table.enum", { defaultValue: "enum:" }),
    itemsText: t("servers:capabilityList.table.items", { defaultValue: "items:" }),
    itemsEnumText: t("servers:capabilityList.table.itemsEnum", { defaultValue: "items.enum:" }),
  };

  const s = ensureObjectType(schema);
  if (!s?.properties) return null;

  const properties = Object.entries(s.properties);
  if (!properties.length) return null;

  const requiredSet = new Set<string>(s.required ?? []);

  return (
    <table className="w-full border-collapse text-xs">
      <thead>
        <tr className="text-left text-slate-500">
          <th className="border-b py-1 pr-2">{t("servers:capabilityList.table.property", { defaultValue: "Property" })}</th>
          <th className="border-b py-1 pr-2">{t("servers:capabilityList.table.type", { defaultValue: "Type" })}</th>
          <th className="border-b py-1 pr-2">{t("servers:capabilityList.table.required", { defaultValue: "Required" })}</th>
          <th className="border-b py-1 pr-2">{t("servers:capabilityList.table.description", { defaultValue: "Description" })}</th>
          <th className="border-b py-1 pr-2">{t("servers:capabilityList.table.details", { defaultValue: "Details" })}</th>
        </tr>
      </thead>
      <tbody>
        {properties.map(([name, propSchema]) => (
          <SchemaRow
            key={name}
            name={name}
            schema={propSchema as JsonSchema}
            required={requiredSet.has(name)}
            labels={labels}
          />
        ))}
      </tbody>
    </table>
  );
}

export default SchemaTable;
