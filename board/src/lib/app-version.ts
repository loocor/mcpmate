export const APP_VERSION =
	typeof import.meta !== "undefined" &&
	typeof import.meta.env?.VITE_APP_VERSION === "string"
		? import.meta.env.VITE_APP_VERSION.trim()
		: "";

export const APP_VERSION_LABEL = APP_VERSION ? `v${APP_VERSION}` : "";
