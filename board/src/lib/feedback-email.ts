const FEEDBACK_EMAIL = "MCPMate Team <info@mcpmate.io>";
const FEEDBACK_SUBJECT = encodeURIComponent("MCPMate preview feedback");
const FEEDBACK_BODY = encodeURIComponent(
	"Hi MCPMate team,\n\nDescribe your feedback here:\n\n\n\n— Sent from MCPMate preview\n",
);

export const FEEDBACK_MAILTO = `mailto:${FEEDBACK_EMAIL}?subject=${FEEDBACK_SUBJECT}&body=${FEEDBACK_BODY}`;

/**
 * Opens the default mail client with a prefilled feedback message.
 * In Tauri, uses the opener plugin so `mailto:` is handled by the OS instead of a browser tab.
 */
export async function openFeedbackEmail(): Promise<void> {
	try {
		if (typeof window !== "undefined" && "__TAURI__" in window) {
			const opener = await import("@tauri-apps/plugin-opener");
			await opener.openUrl(FEEDBACK_MAILTO);
			return;
		}
	} catch (error) {
		console.error("Failed to open feedback email", error);
	}
	if (typeof window !== "undefined") {
		window.open(FEEDBACK_MAILTO, "_blank", "noopener,noreferrer");
	}
}
