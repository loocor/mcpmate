import {
	SITE_NAME,
	SITE_PREVIEW_IMAGE_ALT,
	SITE_PREVIEW_IMAGE_URL,
	buildSiteUrl,
} from "./site";

type MetaConfig = {
	title: string;
	description?: string;
	pathname?: string;
};

function upsertMeta(selector: string, attributes: Record<string, string>) {
	let element = document.head.querySelector<HTMLMetaElement>(selector);
	if (!element) {
		element = document.createElement("meta");
		document.head.appendChild(element);
	}

	Object.entries(attributes).forEach(([key, value]) => {
		element?.setAttribute(key, value);
	});
}

function upsertCanonical(href: string) {
	let link = document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]');
	if (!link) {
		link = document.createElement("link");
		link.setAttribute("rel", "canonical");
		document.head.appendChild(link);
	}
	link.setAttribute("href", href);
}

export function setDocumentMeta({ title, description, pathname }: MetaConfig) {
	document.title = title;

	if (description) {
		upsertMeta('meta[name="description"]', {
			name: "description",
			content: description,
		});
		upsertMeta('meta[property="og:description"]', {
			property: "og:description",
			content: description,
		});
		upsertMeta('meta[name="twitter:description"]', {
			name: "twitter:description",
			content: description,
		});
	}

	upsertMeta('meta[property="og:title"]', {
		property: "og:title",
		content: title,
	});
	upsertMeta('meta[property="og:site_name"]', {
		property: "og:site_name",
		content: SITE_NAME,
	});
	upsertMeta('meta[property="og:type"]', {
		property: "og:type",
		content: "website",
	});
	upsertMeta('meta[property="og:image"]', {
		property: "og:image",
		content: SITE_PREVIEW_IMAGE_URL,
	});
	upsertMeta('meta[property="og:image:alt"]', {
		property: "og:image:alt",
		content: SITE_PREVIEW_IMAGE_ALT,
	});
	upsertMeta('meta[name="twitter:card"]', {
		name: "twitter:card",
		content: "summary_large_image",
	});
	upsertMeta('meta[name="twitter:title"]', {
		name: "twitter:title",
		content: title,
	});
	upsertMeta('meta[name="twitter:image"]', {
		name: "twitter:image",
		content: SITE_PREVIEW_IMAGE_URL,
	});
	upsertMeta('meta[name="twitter:image:alt"]', {
		name: "twitter:image:alt",
		content: SITE_PREVIEW_IMAGE_ALT,
	});

	if (typeof window !== "undefined") {
		const canonicalUrl = buildSiteUrl(pathname ?? window.location.pathname);
		upsertCanonical(canonicalUrl);
		upsertMeta('meta[property="og:url"]', {
			property: "og:url",
			content: canonicalUrl,
		});
	}
}
