import { useId, useState } from "react";
import { Clock, ListChecks, Settings2, TimerReset } from "lucide-react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import type { InspectorConfigurationState } from "./inspector-feature-config";
import {
	INSPECTOR_REQUEST_TIMEOUT_PRESETS,
	INSPECTOR_MAX_TOTAL_TIMEOUT_PRESETS,
} from "./inspector-feature-config";

type InspectorConfigurationWorkspaceProps = {
	config: InspectorConfigurationState;
	onConfigChange: (patch: Partial<InspectorConfigurationState>) => void;
};

export function InspectorConfigurationWorkspace({
	config,
	onConfigChange,
}: InspectorConfigurationWorkspaceProps) {
	const [activeTab, setActiveTab] = useState("capabilities");
	const autoListId = useId();
	const reconnectId = useId();
	const resetTimeoutId = useId();

	const tabTriggerClass =
		"w-full justify-center gap-2 px-2 py-2 text-left text-sm font-medium text-slate-600 data-[state=active]:text-emerald-700 md:justify-start md:px-3 dark:text-slate-300";
	const settingItemTitleClass = "text-base font-medium";
	const settingItemDescriptionClass = "text-sm text-muted-foreground";
	const settingsRowClass =
		"flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4";
	const settingsLabelClass = "min-w-0 space-y-0.5";
	const settingsControlClass = "w-full shrink-0 sm:w-72";

	return (
		<div className="space-y-4">
			<div className="rounded-md border border-dashed border-border bg-card/40 p-4">
				<div className="flex items-center gap-2">
					<Settings2 className="h-5 w-5 text-muted-foreground" />
					<div>
						<p className="text-lg font-semibold text-foreground">Configuration</p>
						<p className="text-sm text-muted-foreground">
							Inspector defaults and capability behavior. Connection settings and
							credentials live in the Connect workspace.
						</p>
					</div>
				</div>
			</div>

			<Tabs
				value={activeTab}
				onValueChange={setActiveTab}
				orientation="vertical"
				className="flex items-start gap-3"
			>
				<TabsList className="sticky top-3 flex w-14 shrink-0 flex-col gap-1 self-start rounded-lg bg-slate-100 p-1 dark:bg-slate-800 md:w-52 md:p-2">
					<TabsTrigger value="capabilities" className={tabTriggerClass}>
						<ListChecks className="h-4 w-4 shrink-0" />
						<span className="hidden truncate md:inline">Capabilities</span>
					</TabsTrigger>
					<TabsTrigger value="session" className={tabTriggerClass}>
						<Clock className="h-4 w-4 shrink-0" />
						<span className="hidden truncate md:inline">Session</span>
					</TabsTrigger>
					<TabsTrigger value="timeouts" className={tabTriggerClass}>
						<TimerReset className="h-4 w-4 shrink-0" />
						<span className="hidden truncate md:inline">Timeouts</span>
					</TabsTrigger>
				</TabsList>

				<div className="min-w-0 flex-1">
					<TabsContent value="capabilities" className="mt-0">
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Capabilities</CardTitle>
								<CardDescription>
									Defaults for capability family behavior in the Inspect workspace.
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label htmlFor={autoListId} className={settingItemTitleClass}>
											Auto-list on family switch
										</Label>
										<p className={settingItemDescriptionClass}>
											When enabled, switching accordion families triggers List
											automatically.
										</p>
									</div>
									<div className="shrink-0">
										<Switch
											id={autoListId}
											checked={config.autoListOnFamilySwitch}
											onCheckedChange={(checked) =>
												onConfigChange({ autoListOnFamilySwitch: checked })
											}
										/>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="session" className="mt-0">
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Session lifecycle</CardTitle>
								<CardDescription>
									Session defaults for Inspector connections and reconnect behavior.
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label
											htmlFor="session-idle-timeout"
											className={settingItemTitleClass}
										>
											Idle timeout
										</Label>
										<p className={settingItemDescriptionClass}>
											Minutes before an inactive Inspector session expires.
										</p>
									</div>
									<div className={settingsControlClass}>
										<Select
											value={String(config.sessionIdleTimeoutMinutes)}
											onValueChange={(value) =>
												onConfigChange({
													sessionIdleTimeoutMinutes: Number(value),
												})
											}
										>
											<SelectTrigger id="session-idle-timeout" className="h-9">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="15">15</SelectItem>
												<SelectItem value="30">30</SelectItem>
												<SelectItem value="60">60</SelectItem>
												<SelectItem value="120">120</SelectItem>
											</SelectContent>
										</Select>
									</div>
								</div>

								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label
											htmlFor="default-transport"
											className={settingItemTitleClass}
										>
											Default transport mode
										</Label>
										<p className={settingItemDescriptionClass}>
											Preferred runtime path for new Inspector sessions.
										</p>
									</div>
									<div className={settingsControlClass}>
										<Select
											value={config.defaultTransportMode}
											onValueChange={(value) =>
												onConfigChange({
													defaultTransportMode:
														value as InspectorConfigurationState["defaultTransportMode"],
												})
											}
										>
											<SelectTrigger id="default-transport" className="h-9">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="native">Native</SelectItem>
												<SelectItem value="proxy">Proxy</SelectItem>
												<SelectItem value="bridge">Bridge</SelectItem>
											</SelectContent>
										</Select>
									</div>
								</div>

								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label htmlFor={reconnectId} className={settingItemTitleClass}>
											Reconnect on expiry
										</Label>
										<p className={settingItemDescriptionClass}>
											Attempt session refresh before requiring manual reconnect.
										</p>
									</div>
									<div className="shrink-0">
										<Switch
											id={reconnectId}
											checked={config.reconnectOnExpiry}
											onCheckedChange={(checked) =>
												onConfigChange({ reconnectOnExpiry: checked })
											}
										/>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="timeouts" className="mt-0">
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Request timeouts</CardTitle>
								<CardDescription>
									Client-side timeouts independent of server-side timeout behavior.
								</CardDescription>
							</CardHeader>
							<CardContent className="space-y-5">
								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label
											htmlFor="request-timeout"
											className={settingItemTitleClass}
										>
											Request timeout
										</Label>
										<p className={settingItemDescriptionClass}>
											Maximum wait time for a single Inspector request.
										</p>
									</div>
									<div className={settingsControlClass}>
										<Select
											value={String(config.requestTimeoutMs)}
											onValueChange={(value) =>
												onConfigChange({ requestTimeoutMs: Number(value) })
											}
										>
											<SelectTrigger id="request-timeout" className="h-9">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												{INSPECTOR_REQUEST_TIMEOUT_PRESETS.map((preset) => (
													<SelectItem key={preset.value} value={preset.value}>
														{preset.label}
													</SelectItem>
												))}
											</SelectContent>
										</Select>
									</div>
								</div>

								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label
											htmlFor={resetTimeoutId}
											className={settingItemTitleClass}
										>
											Reset timeout on progress
										</Label>
										<p className={settingItemDescriptionClass}>
											Reset the timeout clock when progress notifications are
											received.
										</p>
									</div>
									<div className="shrink-0">
										<Switch
											id={resetTimeoutId}
											checked={config.resetTimeoutOnProgress}
											onCheckedChange={(checked) =>
												onConfigChange({ resetTimeoutOnProgress: checked })
											}
										/>
									</div>
								</div>

								<div className={settingsRowClass}>
									<div className={settingsLabelClass}>
										<Label
											htmlFor="max-total-timeout"
											className={settingItemTitleClass}
										>
											Max timeout with progress
										</Label>
										<p className={settingItemDescriptionClass}>
											Maximum total duration for requests using progress
											notifications.
										</p>
									</div>
									<div className={settingsControlClass}>
										<Select
											value={String(config.maxTotalTimeoutMs)}
											onValueChange={(value) =>
												onConfigChange({ maxTotalTimeoutMs: Number(value) })
											}
										>
											<SelectTrigger id="max-total-timeout" className="h-9">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												{INSPECTOR_MAX_TOTAL_TIMEOUT_PRESETS.map((preset) => (
													<SelectItem key={preset.value} value={preset.value}>
														{preset.label}
													</SelectItem>
												))}
											</SelectContent>
										</Select>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>
				</div>
			</Tabs>
		</div>
	);
}
