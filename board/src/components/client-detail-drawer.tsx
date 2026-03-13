import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useMemo, useState } from "react";
import { clientsApi } from "../lib/api";
import type {
	ClientBackupEntry,
	ClientBackupPolicySetReq,
	ClientConfigMode,
	ClientConfigSelected,
} from "../lib/types";
import { Button } from "./ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Switch } from "./ui/switch";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { notifyError, notifySuccess, notifyInfo } from "../lib/notify";
import { formatBackupTime } from "../lib/utils";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import { RefreshCw, RotateCcw, Trash2, Upload } from "lucide-react";

interface ClientDetailDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	identifier: string;
	displayName: string;
}

export function ClientDetailDrawer({
	open,
	onOpenChange,
	identifier,
	displayName,
}: ClientDetailDrawerProps) {
	const qc = useQueryClient();
	const modeId = useId();
	const selectedId = useId();
	const limitId = useId();

	// Config apply options
	const [mode, setMode] = useState<ClientConfigMode>("hosted");
	const [selectedConfig, setSelectedConfig] =
		useState<ClientConfigSelected>("default");
	const [preview, setPreview] = useState(false);

	const {
		data: configDetails,
		isLoading: loadingConfig,
		refetch: refetchDetails,
	} = useQuery({
		queryKey: ["client-config", identifier],
		queryFn: () => clientsApi.configDetails(identifier, false),
		enabled: open,
	});

	const importMutation = useMutation({
		mutationFn: () => clientsApi.configDetails(identifier, true),
		onSuccess: () => {
			notifySuccess("Imported", "Servers imported from client config (if any)");
		},
		onError: (e) => notifyError("Import failed", String(e)),
	});

	const {
		data: backupsData,
		isLoading: loadingBackups,
		refetch: refetchBackups,
	} = useQuery({
		queryKey: ["client-backups", identifier],
		queryFn: () => clientsApi.listBackups(identifier),
		enabled: open,
	});

	const backups: ClientBackupEntry[] = useMemo(
		() => backupsData?.backups || [],
		[backupsData],
	);

	const { data: policyData, refetch: refetchPolicy } = useQuery({
		queryKey: ["client-policy", identifier],
		queryFn: () => clientsApi.getBackupPolicy(identifier),
		enabled: open,
	});

	const applyMutation = useMutation({
		mutationFn: () =>
			clientsApi.applyConfig({
				identifier,
				mode,
				selected_config: selectedConfig,
				preview,
			}),
		onSuccess: () => {
			if (preview) {
				notifyInfo("Preview generated", "Preview ready");
			} else {
				notifySuccess("Applied", "Configuration applied");
			}
			qc.invalidateQueries({ queryKey: ["client-config", identifier] });
		},
		onError: (e) => notifyError("Apply failed", String(e)),
	});

	const restoreMutation = useMutation({
		mutationFn: ({ backup }: { backup: string }) =>
			clientsApi.restoreConfig({ identifier, backup }),
		onSuccess: () => {
			notifySuccess("Restored", "Configuration restored from backup");
			refetchDetails();
			refetchBackups();
		},
		onError: (e) => notifyError("Restore failed", String(e)),
	});

	const deleteBackupMutation = useMutation({
		mutationFn: ({ backup }: { backup: string }) =>
			clientsApi.deleteBackup(identifier, backup),
		onSuccess: () => {
			notifySuccess("Deleted", "Backup deleted");
			refetchBackups();
		},
		onError: (e) => notifyError("Delete failed", String(e)),
	});

	const setPolicyMutation = useMutation({
		mutationFn: (payload: ClientBackupPolicySetReq) =>
			clientsApi.setBackupPolicy(payload),
		onSuccess: () => {
			notifySuccess("Saved", "Backup policy updated");
			refetchPolicy();
		},
		onError: (e) => notifyError("Save failed", String(e)),
	});

	// Local input for policy
	const [policyLabel, setPolicyLabel] = useState<string>("keep_n");
	const [policyLimit, setPolicyLimit] = useState<number | undefined>(30);

	useEffect(() => {
		if (policyData) {
			setPolicyLabel(policyData.policy || "keep_n");
			setPolicyLimit(policyData.limit ?? undefined);
		}
	}, [policyData]);

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent>
				<DrawerHeader>
					<DrawerTitle>Client Details: {displayName}</DrawerTitle>
					<DrawerDescription>
						Manage configuration and backups for {identifier}
					</DrawerDescription>
				</DrawerHeader>

				<div className="flex-1 overflow-y-auto px-4 py-4 space-y-4">
					<Card>
						<CardHeader>
							<CardTitle>Overview</CardTitle>
						</CardHeader>
						<CardContent className="space-y-2 text-sm">
							{loadingConfig ? (
								<div className="animate-pulse h-16 bg-slate-200 dark:bg-slate-800 rounded" />
							) : configDetails ? (
								<>
									<div>
										Config Path:{" "}
										<span className="font-mono">
											{configDetails.config_path}
										</span>
									</div>
									<div>Exists: {String(configDetails.config_exists)}</div>
									<div>
										MCP Config: {String(configDetails.has_mcp_config)} (Servers:{" "}
										{configDetails.mcp_servers_count})
									</div>
									<div>Last Modified: {formatBackupTime(configDetails.last_modified)}</div>
								</>
							) : (
								<div className="text-slate-500">No details available</div>
							)}
						</CardContent>
					</Card>

					<Card>
						<CardHeader>
							<CardTitle>Apply Configuration</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							<div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
								<div className="space-y-1">
									<Label htmlFor={modeId}>Mode</Label>
									<Select
										value={mode}
										onValueChange={(v) => setMode(v as ClientConfigMode)}
									>
										<SelectTrigger id={modeId}>
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="hosted">hosted</SelectItem>
											<SelectItem value="transparent">transparent</SelectItem>
										</SelectContent>
									</Select>
								</div>
								<div className="space-y-1">
									<Label htmlFor={selectedId}>Source</Label>
									<Select
										value={selectedConfig}
										onValueChange={(v) =>
											setSelectedConfig(v as ClientConfigSelected)
										}
									>
										<SelectTrigger id={selectedId}>
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="default">default</SelectItem>
											<SelectItem value="profile">
												profile <sup>(WIP)</sup>
											</SelectItem>
											<SelectItem value="custom">
												custom <sup>(WIP)</sup>
											</SelectItem>
										</SelectContent>
									</Select>
								</div>
								<div className="space-y-1">
									<Label>Preview</Label>
									<div className="flex items-center gap-2">
										<Switch checked={preview} onCheckedChange={setPreview} />
										<span className="text-xs text-slate-500">
											Only generate preview
										</span>
									</div>
								</div>
							</div>
							<div className="flex gap-2">
								<Button
									onClick={() => applyMutation.mutate()}
									disabled={applyMutation.isPending}
								>
									<Upload className="mr-2 h-4 w-4" />
									{preview ? "Preview" : "Apply"}
								</Button>
								<Button
									variant="outline"
									onClick={() => refetchDetails()}
									disabled={loadingConfig}
								>
									<RefreshCw className="mr-2 h-4 w-4" />
									Refresh
								</Button>
								<Button
									variant="outline"
									onClick={() => importMutation.mutate()}
									disabled={importMutation.isPending}
								>
									Import Servers
								</Button>
							</div>
						</CardContent>
					</Card>

					<Card>
						<CardHeader>
							<CardTitle>Backups</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							{loadingBackups ? (
								<div className="space-y-2">
									{[1, 2, 3].map((i) => (
										<div
											key={i}
											className="h-10 bg-slate-200 dark:bg-slate-800 animate-pulse rounded"
										/>
									))}
								</div>
							) : backups.length ? (
								<CapsuleStripeList>
									{backups
										.filter((b) => b.identifier === identifier)
										.map((b) => (
											<CapsuleStripeListItem
												key={b.path}
												interactive
												className="group"
											>
												<div className="flex items-center justify-between flex-1">
													<div className="font-mono">{b.backup}</div>
													<div className="flex items-center justify-end gap-4">
														<div className="flex items-center gap-4 transition-transform duration-200 group-hover:translate-x-0 translate-x-4 -mr-3">
															<div className="text-slate-500">
																{formatBackupTime(b.created_at)}
															</div>
															<div className="text-slate-500">
																{(b.size / 1024).toFixed(1)} KB
															</div>
														</div>
														<div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
															<Button
																size="sm"
																variant="outline"
																onClick={() =>
																	restoreMutation.mutate({ backup: b.backup })
																}
															>
																<RotateCcw className="mr-2 h-4 w-4" />
																Restore
															</Button>
															<Button
																size="icon"
																variant="outline"
																onClick={() =>
																	deleteBackupMutation.mutate({
																		backup: b.backup,
																	})
																}
															>
																<Trash2 className="h-4 w-4" />
															</Button>
														</div>
													</div>
												</div>
											</CapsuleStripeListItem>
										))}
								</CapsuleStripeList>
							) : (
								<div className="text-slate-500 text-sm">No backups.</div>
							)}

							<div className="mt-3">
								<div className="grid grid-cols-1 sm:grid-cols-3 gap-3 items-end">
									<div className="space-y-1">
										<Label>Policy</Label>
										<Select value={policyLabel} onValueChange={setPolicyLabel}>
											<SelectTrigger>
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="keep_n">keep_n</SelectItem>
											</SelectContent>
										</Select>
									</div>
									<div className="space-y-1">
										<Label htmlFor={limitId}>Limit</Label>
										<Input
											id={limitId}
											type="number"
											min={0}
											value={policyLimit ?? 0}
											onChange={(e) => setPolicyLimit(Number(e.target.value))}
										/>
									</div>
									<div className="space-y-1">
										<Button
											onClick={() =>
												setPolicyMutation.mutate({
													identifier,
													policy: { policy: policyLabel, limit: policyLimit },
												})
											}
											disabled={setPolicyMutation.isPending}
										>
											Save Policy
										</Button>
									</div>
								</div>
							</div>
						</CardContent>
					</Card>
				</div>

				<DrawerFooter>
					<div className="flex gap-2 w-full">
						<Button
							variant="outline"
							className="flex-1"
							onClick={() => onOpenChange(false)}
						>
							Close
						</Button>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}

export default ClientDetailDrawer;
