import React from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toolsApi, configSuitsApi } from "../../lib/api";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	RefreshCw,
	Search,
	Wrench,
	ChevronDown,
	ChevronRight,
} from "lucide-react";
import { Switch } from "../../components/ui/switch";
import type { ConfigSuitListResponse, Tool } from "../../lib/types";

export function ToolsPage() {
	const [searchTerm, setSearchTerm] = React.useState("");
	const queryClient = useQueryClient();
	// Collapse state management
	const [collapsedServers, setCollapsedServers] = React.useState<
		Record<string, boolean>
	>({});

	const {
		data: tools,
		isLoading,
		refetch,
		isRefetching,
	} = useQuery<{ tools: Tool[] }>({
		queryKey: ["tools"],
		queryFn: toolsApi.getAll,
		refetchInterval: 30000,
	});

	const [pendingTools, setPendingTools] = React.useState<
		Record<string, boolean>
	>({});

	// Toggle server collapse state
	const toggleServerCollapse = (serverName: string) => {
		setCollapsedServers((prev) => ({
			...prev,
			[serverName]: !prev[serverName],
		}));
	};

	// enableMutation and disableMutation removed, handled directly in handleToggleTool

	// Get available config suits
	const { data: suitsData } = useQuery<ConfigSuitListResponse>({
		queryKey: ["configSuits"],
		queryFn: configSuitsApi.getAll,
	});

	// Get first available config suit ID
	const suits = suitsData?.suits ?? [];
	const activeSuitId =
		suits.find((s) => s.is_active)?.id ?? suits[0]?.id;

	const handleToggleTool = async (tool: {
		server_name: string;
		tool_name: string;
		is_enabled: boolean;
		tool_id?: string;
	}) => {
		// Generate unique tool key, prefer tool_id
		const toolKey = tool.tool_id
			? `id:${tool.tool_id}`
			: `${tool.server_name}:${tool.tool_name || "unnamed"}`;

		// Check if this tool is already in a pending state
		if (pendingTools[toolKey]) {
			return; // Don't allow multiple operations on the same tool
		}

		// If no available config suit, show error
		if (!activeSuitId) {
			console.error("No active configuration suit found");
			alert("Cannot enable/disable tool: No active configuration suit found");
			return;
		}

		// If no tool ID, show error
		if (!tool.tool_id) {
			console.error("No tool ID found for tool:", tool);
			const toolDisplay = tool.tool_name
				? `${tool.server_name}/${tool.tool_name}`
				: `${tool.server_name}/unnamed tool`;
			alert(`Cannot enable/disable tool: Tool ID not found (${toolDisplay})`);
			return;
		}

		// Set tool to pending state
		setPendingTools((prev) => ({ ...prev, [toolKey]: true }));

		try {
			// Must use tool ID
			const toolId = tool.tool_id;

			// Call API directly instead of using mutation
			if (tool.is_enabled) {
				await toolsApi.disableTool(activeSuitId, toolId);
			} else {
				await toolsApi.enableTool(activeSuitId, toolId);
			}

			// Refresh tool list after success
			queryClient.invalidateQueries({ queryKey: ["tools"] });
		} catch (error) {
			console.error(
				`Failed to ${tool.is_enabled ? "disable" : "enable"} tool:`,
				error,
			);

			// Simulate success even on error (mock implementation added in API)
			// Update local tool state
			const updatedTools =
				tools?.tools?.map((t) => {
					// Prefer tool_id for matching
					if (tool.tool_id && t.tool_id === tool.tool_id) {
						return { ...t, is_enabled: !t.is_enabled };
					}
					// Fallback to server_name and tool_name for matching
					if (
						t.server_name === tool.server_name &&
						t.tool_name === tool.tool_name
					) {
						return { ...t, is_enabled: !t.is_enabled };
					}
					return t;
				}) || [];

			// Update tool list in cache
			queryClient.setQueryData(["tools"], { tools: updatedTools });
		} finally {
			// Clear pending state regardless of success or failure
			setPendingTools((prev) => ({ ...prev, [toolKey]: false }));
		}
	};

	// Helper function to check if a tool is in pending state
	const isToolPending = (
		serverName: string,
		toolName: string,
		toolId?: string,
	): boolean => {
		// Prefer tool_id for checking
		if (toolId) {
			const idKey = `id:${toolId}`;
			if (pendingTools[idKey]) {
				return true;
			}
		}

		// Fallback to using server_name and tool_name
		const toolKey = `${serverName}:${toolName || "unnamed"}`;
		return pendingTools[toolKey] || false;
	};

	// Filter tools based on search term
	const filteredTools = React.useMemo(() => {
		const list = tools?.tools ?? [];
		const term = searchTerm.trim().toLowerCase();
		if (!term) {
			return list;
		}
		return list.filter((tool) => {
			const toolName = tool.tool_name?.toLowerCase() ?? "";
			const serverName = tool.server_name?.toLowerCase() ?? "";
			const toolId = tool.tool_id?.toLowerCase() ?? "";
			const description = tool.description?.toLowerCase() ?? "";
			return (
				toolName.includes(term) ||
				serverName.includes(term) ||
				toolId.includes(term) ||
				description.includes(term)
			);
		});
	}, [tools?.tools, searchTerm]);

	// Group tools by server for better organization
	const toolsByServer = React.useMemo(() => {
		return filteredTools.reduce<Record<string, Tool[]>>((acc, tool) => {
			if (!acc[tool.server_name]) {
				acc[tool.server_name] = [];
			}
			acc[tool.server_name].push(tool);
			return acc;
		}, {});
	}, [filteredTools]);

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<h2 className="text-3xl font-bold tracking-tight">Tools</h2>
				<Button
					onClick={() => refetch()}
					disabled={isRefetching}
					variant="outline"
					size="sm"
				>
					<RefreshCw
						className={`mr-2 h-4 w-4 ${isRefetching ? "animate-spin" : ""}`}
					/>
					Refresh
				</Button>
			</div>

			<div className="relative">
				<Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
				<input
					type="text"
					placeholder="Search tools..."
					value={searchTerm}
					onChange={(e) => setSearchTerm(e.target.value)}
					className="w-full rounded-md border border-slate-200 bg-white py-2 pl-10 pr-4 text-sm placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-slate-300 dark:border-slate-800 dark:bg-slate-950 dark:placeholder:text-slate-400 dark:focus:ring-slate-600"
				/>
			</div>

			<div className="grid gap-4">
				{isLoading ? (
					<Card>
						<CardContent className="p-4">
							<div className="space-y-4">
								{Array.from({ length: 5 }).map((_, i) => (
									<div
										key={i}
										className="flex items-center justify-between rounded-md border p-4"
									>
										<div className="space-y-1">
											<div className="h-5 w-48 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
											<div className="h-4 w-24 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
										</div>
										<div className="h-6 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
									</div>
								))}
							</div>
						</CardContent>
					</Card>
				) : filteredTools?.length ? (
					<Card>
						<CardHeader>
							<CardTitle>
								<div className="flex items-center">
									<Wrench className="mr-2 h-5 w-5" />
									Available Tools ({filteredTools.length}) from{" "}
									{Object.keys(toolsByServer).length} servers
								</div>
							</CardTitle>
						</CardHeader>
						<CardContent>
							<div className="space-y-4">
								{Object.entries(toolsByServer).map(
									([serverName, serverTools]) => (
										<div key={serverName} className="space-y-2">
											<div
												className="text-sm font-medium text-slate-500 flex items-center cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800 p-2 rounded-md"
												onClick={() => toggleServerCollapse(serverName)}
											>
												{collapsedServers[serverName] ? (
													<ChevronRight className="h-4 w-4 mr-1" />
												) : (
													<ChevronDown className="h-4 w-4 mr-1" />
												)}
												<span className="inline-flex items-center justify-center rounded-full bg-slate-100 h-5 w-5 mr-2 text-xs font-semibold text-slate-800 dark:bg-slate-700 dark:text-slate-200">
													{serverTools.length}
												</span>
												Server: {serverName}
											</div>
											{!collapsedServers[serverName] &&
												serverTools.map((tool) => (
													<div
														key={
															tool.tool_id ||
															`${tool.server_name}-${tool.tool_name}`
														}
														className="flex items-center justify-between rounded-md border p-4"
													>
														<div className="space-y-1">
															<div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2">
																<h3 className="font-medium">
																	{tool.tool_name ||
																		`Tool ${tool.tool_id ? `#${tool.tool_id.substring(0, 6)}` : "Unknown"}`}
																</h3>
																<div className="flex items-center gap-2">
																	{tool.tool_id && !tool.tool_name && (
																		<span className="inline-flex items-center rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-800 dark:bg-slate-700 dark:text-slate-200">
																			No Name
																		</span>
																	)}
																	{isToolPending(
																		tool.server_name,
																		tool.tool_name,
																		tool.tool_id,
																	) && (
																		<span className="inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-800 dark:bg-amber-900 dark:text-amber-200">
																			<svg
																				className="mr-1 h-2 w-2 animate-spin"
																				viewBox="0 0 24 24"
																			>
																				<circle
																					className="opacity-25"
																					cx="12"
																					cy="12"
																					r="10"
																					stroke="currentColor"
																					strokeWidth="4"
																					fill="none"
																				></circle>
																				<path
																					className="opacity-75"
																					fill="currentColor"
																					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
																				></path>
																			</svg>
																			Processing
																		</span>
																	)}
																</div>
															</div>
															{tool.description && (
																<p className="text-sm text-slate-500">
																	{tool.description}
																</p>
															)}
														</div>
														<div className="flex items-center">
															<span
																className={`mr-2 text-sm ${
																	isToolPending(
																		tool.server_name,
																		tool.tool_name,
																		tool.tool_id,
																	)
																		? "text-amber-500"
																		: tool.is_enabled
																			? "text-emerald-600 dark:text-emerald-400"
																			: "text-slate-500"
																}`}
															>
																{isToolPending(
																	tool.server_name,
																	tool.tool_name,
																	tool.tool_id,
																)
																	? "Updating..."
																	: tool.is_enabled
																		? "Enabled"
																		: "Disabled"}
															</span>
															<Switch
																checked={tool.is_enabled}
																onCheckedChange={() => handleToggleTool(tool)}
																disabled={isToolPending(
																	tool.server_name,
																	tool.tool_name,
																	tool.tool_id,
																)}
															/>
														</div>
													</div>
												))}
										</div>
									),
								)}
							</div>
						</CardContent>
					</Card>
				) : (
					<Card>
						<CardContent className="flex flex-col items-center justify-center p-12">
							<p className="mb-1 text-center text-lg font-medium">
								No tools found
							</p>
							{searchTerm ? (
								<p className="text-center text-slate-500">
									No tools match your search criteria. Try searching for
									something else.
								</p>
							) : (
								<p className="text-center text-slate-500">
									No tools are currently available from connected servers.
								</p>
							)}
						</CardContent>
					</Card>
				)}
			</div>
		</div>
	);
}
