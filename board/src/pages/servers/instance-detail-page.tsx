import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import { serversApi } from "../../lib/api";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	PlayCircle,
	RefreshCw,
	RotateCw,
	Shield,
	StopCircle,
} from "lucide-react";
import { StatusBadge } from "../../components/status-badge";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { useTranslation } from "react-i18next";

export function InstanceDetailPage() {
	usePageTranslations("servers");
    const { t } = useTranslation("servers");
	const { serverId, instanceId } = useParams<{
		serverId: string;
		instanceId: string;
	}>();

	const {
		data: instance,
		isLoading,
		refetch,
		isRefetching,
	} = useQuery({
		queryKey: ["instance", serverId, instanceId],
		queryFn: () => serversApi.getInstance(serverId || "", instanceId || ""),
		enabled: !!serverId && !!instanceId,
		refetchInterval: 10000,
	});

	const { data: serverDetails } = useQuery({
		queryKey: ["serverForInstance", serverId],
		queryFn: () => serversApi.getServer(serverId || ""),
		enabled: !!serverId,
	});

	const { data: health, isLoading: isLoadingHealth } = useQuery({
		queryKey: ["instanceHealth", serverId, instanceId],
		queryFn: () =>
			serversApi.getInstanceHealth(serverId || "", instanceId || ""),
		enabled: !!serverId && !!instanceId,
		refetchInterval: 10000,
	});

	const translateHealthMessage = (message?: string | null) => {
		if (!message) return message || undefined;
		const normalized = message.trim();
		switch (normalized) {
			case "Instance is idle (placeholder, not connected)":
				return t("instanceDetail.healthMessages.idlePlaceholder", {
					defaultValue: "Instance is idle (placeholder, not connected)",
				});
			default:
				return message;
		}
	};

	const healthStatus = (health?.status || "").toLowerCase();
	const benignHealthStatuses = new Set([
		"idle",
		"initializing",
		"validating",
		"shutdown",
	]);
	const isBenignHealthNotice =
		!health?.healthy &&
		!!health?.message &&
		benignHealthStatuses.has(healthStatus);
	const isCriticalHealthIssue =
		!health?.healthy &&
		!!health?.message &&
		!benignHealthStatuses.has(healthStatus);
	const healthMessage = translateHealthMessage(health?.message);
	const healthStatusForBadge = health
		? healthStatus || (health.healthy ? "ready" : "unhealthy")
		: undefined;

	if (!serverId || !instanceId) {
		return (
			<div>
				{t("instanceDetail.errors.missingParams", {
					defaultValue: "Server ID or instance ID not provided",
				})}
			</div>
		);
	}

	const handleDisconnect = () => {
		serversApi.disconnectInstance(serverId, instanceId).then(() => refetch());
	};

	const handleForceDisconnect = () => {
		serversApi
			.forceDisconnectInstance(serverId, instanceId)
			.then(() => refetch());
	};

	const handleReconnect = () => {
		serversApi.reconnectInstance(serverId, instanceId).then(() => refetch());
	};

	const handleResetAndReconnect = () => {
		serversApi
			.resetAndReconnectInstance(serverId, instanceId)
			.then(() => refetch());
	};

	const handleCancel = () => {
		serversApi.cancelInstance(serverId, instanceId).then(() => refetch());
	};

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<div className="flex items-center">
					<h2
						className="text-2xl font-bold tracking-tight truncate"
						title={serverDetails?.name || serverId}
					>
						<Link
							to={`/servers/${encodeURIComponent(serverId || "")}`}
							className="hover:underline"
						>
							{serverDetails?.name || serverId}
						</Link>
					</h2>
					{!isLoading && instance && (
						<StatusBadge
							status={
								typeof instance.status === "string"
									? instance.status.toLowerCase()
									: "unknown"
							}
							className="ml-3"
						/>
					)}
				</div>
				<Button
					onClick={() => refetch()}
					disabled={isRefetching}
					variant="outline"
					size="sm"
				>
					<RefreshCw
						className={`mr-2 h-4 w-4 ${isRefetching ? "animate-spin" : ""}`}
					/>
					{t("actions.refresh.title", { defaultValue: "Refresh" })}
				</Button>
			</div>

			{isLoading ? (
				<Card>
					<CardContent className="p-4">
						<div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
					</CardContent>
				</Card>
			) : instance ? (
				<>
					<div className="grid gap-4 md:grid-cols-2">
						<Card>
							<CardHeader>
								<CardTitle>
									{t("instanceDetail.sections.details.title", {
										defaultValue: "Instance Details",
									})}
								</CardTitle>
							</CardHeader>
							<CardContent>
								<dl className="space-y-2">
									<div className="flex justify-between text-sm">
										<dt className="font-medium text-sm">
											{t("instanceDetail.sections.details.fields.instanceId", {
												defaultValue: "Instance ID:",
											})}
										</dt>
										<dd className="font-mono text-sm">{instance.id}</dd>
									</div>
									<div className="flex justify-between text-sm">
										<dt className="font-medium text-sm">
											{t("instanceDetail.sections.details.fields.server", {
												defaultValue: "Server:",
											})}
										</dt>
										<dd className="text-sm">{instance.server_name}</dd>
									</div>
									<div className="flex justify-between text-sm">
										<dt className="font-medium text-sm">
											{t("instanceDetail.sections.details.fields.status", {
												defaultValue: "Status:",
											})}
										</dt>
										<dd>
											<StatusBadge
												status={
													typeof instance.status === "string"
														? instance.status.toLowerCase()
														: "unknown"
												}
											/>
										</dd>
									</div>
									<div className="flex justify-between text-sm">
										<dt className="font-medium text-sm">
											{t(
												"instanceDetail.sections.details.fields.connectionAttempts",
												{ defaultValue: "Connection Attempts:" },
											)}
										</dt>
										<dd className="text-sm">
											{instance.details.connection_attempts}
										</dd>
									</div>
									{instance.details.last_connected_seconds !== undefined && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t(
													"instanceDetail.sections.details.fields.connectedFor",
													{ defaultValue: "Connected For:" },
												)}
											</dt>
											<dd className="text-sm">
                                            {new Date(
                                                Date.now() -
                                                    instance.details.last_connected_seconds * 1000,
                                            ).toLocaleString()}
											</dd>
										</div>
									)}
									{instance.details.tools_count !== undefined && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t("instanceDetail.sections.details.fields.tools", {
													defaultValue: "Tools:",
												})}
											</dt>
											<dd className="text-sm">{instance.details.tools_count}</dd>
										</div>
									)}
									{instance.details.process_id && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t(
													"instanceDetail.sections.details.fields.processId",
													{ defaultValue: "Process ID:" },
												)}
											</dt>
											<dd className="text-sm">{instance.details.process_id}</dd>
										</div>
									)}
									<div className="flex justify-between text-sm">
										<dt className="font-medium text-sm">
											{t("instanceDetail.sections.details.fields.health", {
												defaultValue: "Health:",
											})}
										</dt>
										<dd>
											{isLoadingHealth ? (
												<div className="h-5 w-20 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
											) : (
												<StatusBadge
													status={healthStatusForBadge || "unknown"}
												/>
											)}
										</dd>
									</div>
								</dl>
							</CardContent>
						</Card>

						<Card>
							<CardHeader>
								<CardTitle>
									{t("instanceDetail.sections.controls.title", {
										defaultValue: "Instance Controls",
									})}
								</CardTitle>
								<CardDescription>
									{t("instanceDetail.sections.controls.description", {
										defaultValue: "Manage the instance connection state",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent>
								<div className="w-full">
									{instance.status.toLowerCase() === "initializing" ? (
										<div className="flex flex-col gap-3 w-full">
											<Button
												variant="destructive"
												onClick={handleCancel}
												className="w-full"
											>
												<Shield className="mr-2 h-4 w-4" />
												{t(
													"instanceDetail.sections.controls.actions.cancelInitialization",
													{ defaultValue: "Cancel Initialization" },
												)}
											</Button>
										</div>
									) : instance.status.toLowerCase() === "ready" ||
										instance.status.toLowerCase() === "busy" ? (
										<div className="flex flex-col gap-3 w-full">
											<Button
												variant="secondary"
												onClick={handleDisconnect}
												className="w-full"
											>
												<StopCircle className="mr-2 h-4 w-4" />
												{t(
													"instanceDetail.sections.controls.actions.disconnect",
													{ defaultValue: "Disconnect Instance" },
												)}
											</Button>
											<Button
												variant="destructive"
												onClick={handleForceDisconnect}
												className="w-full"
											>
												<Shield className="mr-2 h-4 w-4" />
												{t(
													"instanceDetail.sections.controls.actions.forceDisconnect",
													{ defaultValue: "Force Disconnect" },
												)}
											</Button>
										</div>
									) : (
										<div className="flex flex-col gap-3 w-full">
											<Button
												size="lg"
												onClick={handleReconnect}
												className="w-full"
											>
												<PlayCircle className="mr-2 h-5 w-5" />
												{t(
													"instanceDetail.sections.controls.actions.reconnect",
													{ defaultValue: "Reconnect Instance" },
												)}
											</Button>
											<p className="text-xs text-center text-slate-500 mt-1">
												{t(
													"instanceDetail.sections.controls.hints.reconnectDelay",
													{
														defaultValue:
															"Reconnection may take a few moments to complete",
													},
												)}
											</p>
											<Button
												variant="outline"
												onClick={handleResetAndReconnect}
												className="w-full"
											>
												<RotateCw className="mr-2 h-4 w-4" />
												{t(
													"instanceDetail.sections.controls.actions.resetReconnect",
													{ defaultValue: "Reset & Reconnect" },
												)}
											</Button>
										</div>
									)}
								</div>

								{isCriticalHealthIssue && (
									<div className="mt-4">
										<p className="text-sm font-medium text-red-500">
											{t("instanceDetail.notices.healthIssue", {
												defaultValue: "Health issue detected:",
											})}
										</p>
										<div className="mt-1 rounded bg-red-50 dark:bg-red-950 max-h-16 overflow-y-auto">
											<p className="p-2 text-sm text-red-800 dark:text-red-200 break-words whitespace-pre-wrap">
												{healthMessage}
											</p>
										</div>
									</div>
								)}

								{isBenignHealthNotice && (
									<div className="mt-4">
										<p className="text-sm font-medium text-amber-600">
											{t("instanceDetail.notices.statusNote", {
												defaultValue: "Status note:",
											})}
										</p>
										<div className="mt-1 rounded bg-amber-50 dark:bg-amber-950 max-h-16 overflow-y-auto">
											<p className="p-2 text-sm text-amber-900 dark:text-amber-200 break-words whitespace-pre-wrap">
												{healthMessage}
											</p>
										</div>
									</div>
								)}

								{instance.details.error_message && (
									<div className="mt-4">
										<p className="text-sm font-medium text-red-500">
											{t("instanceDetail.notices.error", {
												defaultValue: "Error:",
											})}
										</p>
										<div className="mt-1 rounded bg-red-50 dark:bg-red-950 max-h-16 overflow-y-auto">
											<p className="p-2 text-sm text-red-800 dark:text-red-200 break-words whitespace-pre-wrap">
												{instance.details.error_message}
											</p>
										</div>
									</div>
								)}
							</CardContent>
						</Card>
					</div>

					{(instance.details.cpu_usage !== undefined ||
						instance.details.memory_usage !== undefined) && (
						<Card>
							<CardHeader>
								<CardTitle>
									{t("instanceDetail.sections.metrics.title", {
										defaultValue: "Instance Metrics",
									})}
								</CardTitle>
								<CardDescription>
									{t("instanceDetail.sections.metrics.description", {
										defaultValue:
											"Performance metrics and statistics for this instance",
									})}
								</CardDescription>
							</CardHeader>
							<CardContent>
								<dl className="space-y-2">
									{instance.details.cpu_usage !== undefined && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t("instanceDetail.sections.metrics.fields.cpuUsage", {
													defaultValue: "CPU Usage:",
												})}
											</dt>
											<dd className="text-sm">
												{(instance.details.cpu_usage * 100).toFixed(2)}%
											</dd>
										</div>
									)}
									{instance.details.memory_usage !== undefined && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t(
													"instanceDetail.sections.metrics.fields.memoryUsage",
													{ defaultValue: "Memory Usage:" },
												)}
											</dt>
											<dd className="text-sm">
												{(
													instance.details.memory_usage /
													(1024 * 1024)
												).toFixed(2)}{" "}
												MB
											</dd>
										</div>
									)}
									{health?.connection_stability !== undefined && (
										<div className="flex justify-between text-sm">
											<dt className="font-medium text-sm">
												{t(
													"instanceDetail.sections.metrics.fields.connectionStability",
													{ defaultValue: "Connection Stability:" },
												)}
											</dt>
											<dd className="text-sm">
												{(health.connection_stability * 100).toFixed(0)}%
											</dd>
										</div>
									)}
								</dl>
							</CardContent>
						</Card>
					)}
				</>
			) : (
				<Card>
					<CardContent className="p-4">
						<p className="text-center text-slate-500">
							{t("instanceDetail.errors.notFound", {
								defaultValue:
									"Instance not found or error loading instance details.",
							})}
						</p>
					</CardContent>
				</Card>
			)}
		</div>
	);
}
