/**
 * Schema.org JSON-LD generators for MCPMate website.
 * Produces machine-readable structured data for AI answer engines.
 */

import {
	SITE_LOGO_URL,
	SITE_PREVIEW_IMAGE_URL,
	SITE_URL,
	buildSiteUrl,
} from "./site";

export interface FAQItem {
  question: string;
  answer: string;
}

export function buildSoftwareApplication(overrides?: {
  name?: string;
  description?: string;
  url?: string;
}) {
  return {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: overrides?.name ?? "MCPMate",
    description:
      overrides?.description ??
      "A local-first MCP control plane for managing servers, profiles, clients, runtime dependencies, and imports in one place.",
    applicationCategory: "DeveloperApplication",
    operatingSystem: "macOS",
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    author: {
      "@type": "Organization",
      name: "MCPMate",
      url: "https://github.com/loocor",
    },
    license: "https://www.gnu.org/licenses/agpl-3.0.html",
    url: overrides?.url ?? SITE_URL,
    logo: SITE_LOGO_URL,
    image: SITE_PREVIEW_IMAGE_URL,
    screenshot: SITE_PREVIEW_IMAGE_URL,
    softwareHelp: {
      "@type": "CreativeWork",
      url: buildSiteUrl("/docs/en/quickstart"),
    },
    featureList: [
      "MCP server management",
      "Reusable profiles for capability control",
      "Client configuration and rollout modes",
      "Runtime dependency provisioning",
      "Uni-Import for server config import",
      "Live tool call inspection",
    ],
  };
}

export function buildOrganization() {
  return {
    "@context": "https://schema.org",
    "@type": "Organization",
    name: "MCPMate",
    url: SITE_URL,
    logo: SITE_LOGO_URL,
    image: SITE_PREVIEW_IMAGE_URL,
    sameAs: ["https://github.com/loocor/mcpmate"],
  };
}

export function buildFAQPage(items: FAQItem[]) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.question,
      acceptedAnswer: {
        "@type": "Answer",
        text: item.answer,
      },
    })),
  };
}

export function buildBreadcrumbList(
  items: { name: string; url: string }[],
) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name: item.name,
      item: item.url,
    })),
  };
}

export function buildHowTo(data: {
  name: string;
  description: string;
  steps: { name: string; text: string }[];
}) {
  return {
    "@context": "https://schema.org",
    "@type": "HowTo",
    name: data.name,
    description: data.description,
    step: data.steps.map((s, i) => ({
      "@type": "HowToStep",
      position: i + 1,
      name: s.name,
      text: s.text,
    })),
  };
}
