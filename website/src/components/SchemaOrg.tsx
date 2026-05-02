import { useEffect } from "react";

interface SchemaOrgProps {
  /** One or more JSON-LD objects to inject. */
  schema: Record<string, unknown> | Record<string, unknown>[];
}

/**
 * Injects JSON-LD structured data into the page head.
 * Removes previous instances on unmount to avoid duplicates on SPA navigation.
 */
export default function SchemaOrg({ schema }: SchemaOrgProps) {
  useEffect(() => {
    const schemas = Array.isArray(schema) ? schema : [schema];
    const elements: HTMLScriptElement[] = [];

    for (const s of schemas) {
      const el = document.createElement("script");
      el.type = "application/ld+json";
      el.textContent = JSON.stringify(s);
      document.head.appendChild(el);
      elements.push(el);
    }

    return () => {
      for (const el of elements) {
        el.remove();
      }
    };
  }, [schema]);

  return null;
}
