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
	upsertMeta('meta[name="twitter:title"]', {
		name: "twitter:title",
		content: title,
	});

	if (typeof window !== "undefined") {
		const canonicalUrl = new URL(pathname ?? window.location.pathname, window.location.origin).toString();
		upsertCanonical(canonicalUrl);
	}
}
