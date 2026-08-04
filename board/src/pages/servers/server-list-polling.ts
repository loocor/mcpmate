interface ServerPollingState {
	status?: string;
	instances?: Array<{ status: string }>;
}

const TRANSITIONAL_SERVER_STATUSES = new Set([
	"initializing",
	"starting",
	"connecting",
	"busy",
	"stopping",
]);

function isTransitionalStatus(status: string | undefined): boolean {
	return TRANSITIONAL_SERVER_STATUSES.has(status?.toLowerCase() ?? "");
}

export function getServerListRefetchInterval(
	servers: ServerPollingState[],
): number {
	const hasTransitionalServer = servers.some(
		(server) =>
			isTransitionalStatus(server.status) ||
			server.instances?.some((instance) =>
				isTransitionalStatus(instance.status),
			),
	);

	return hasTransitionalServer ? 5000 : 30000;
}
