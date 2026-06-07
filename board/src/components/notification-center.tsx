import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Button } from "./ui/button";
import { useTranslation } from "react-i18next";
import {
	BellRing,
	CheckCircle2,
	Info,
	AlertTriangle,
	XCircle,
} from "lucide-react";
import { Badge } from "./ui/badge";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "./ui/tooltip";
import { useNotify } from "../lib/notify";

export function NotificationCenter() {
	const { t } = useTranslation();
	const { items, markAllRead, markRead, clear, isOpen, setOpen } = useNotify();
	const unreadCount = items.reduce((acc, it) => acc + (it.read ? 0 : 1), 0);

	function icon(level: string) {
		switch (level) {
			case "success":
				return (
					<CheckCircle2 className="h-4 w-4 text-emerald-600 dark:text-emerald-400" />
				);
			case "warning":
				return (
					<AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
				);
			case "error":
				return <XCircle className="h-4 w-4 text-red-600 dark:text-red-400" />;
			default:
				return <Info className="h-4 w-4 text-blue-600 dark:text-blue-400" />;
		}
	}

	const notificationsLabel = t("notifications.title");

	return (
		<DropdownMenu open={isOpen} onOpenChange={setOpen}>
			<Tooltip>
				<TooltipTrigger asChild>
					<DropdownMenuTrigger asChild>
						<button
							type="button"
							className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
							aria-label={notificationsLabel}
						>
							<div className="relative">
								<BellRing size={20} />
								{unreadCount > 0 ? (
									<Badge
										className="absolute -top-1 -right-1 h-4 min-w-4 px-1 p-0 flex items-center justify-center"
										variant="destructive"
									>
										{unreadCount > 9 ? "9+" : unreadCount}
									</Badge>
								) : null}
							</div>
						</button>
					</DropdownMenuTrigger>
				</TooltipTrigger>
				<TooltipContent side="bottom" align="end">
					{notificationsLabel}
				</TooltipContent>
			</Tooltip>
			<DropdownMenuContent
				align="end"
				className="w-[360px] max-h-[60vh] overflow-auto p-0"
			>
				<div className="px-3 py-2 flex items-center justify-between sticky top-0 bg-popover z-10 border-b border-border">
					<DropdownMenuLabel className="p-0">
						{t("notifications.title")}
					</DropdownMenuLabel>
					<div className="flex items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onClick={markAllRead}
							disabled={unreadCount === 0}
						>
							{t("notifications.markAllRead")}
						</Button>
						<Button
							variant="outline"
							size="sm"
							onClick={clear}
							disabled={items.length === 0}
						>
							{t("notifications.clear")}
						</Button>
					</div>
				</div>
				{items.length === 0 ? (
					<div className="p-4 text-sm text-muted-foreground">
						{t("notifications.noNotifications")}
					</div>
				) : (
					<div className="py-1">
						{items.map((n) => (
							<DropdownMenuItem
								key={n.id}
								className="px-3 py-2 cursor-pointer"
								onSelect={(e) => {
									// Mark as read on click; open link if provided
									e.preventDefault();
									markRead(n.id);
									if (n.href) {
										try {
											window.open(n.href, "_blank", "noopener,noreferrer");
										} catch {
											/* noop */
										}
									}
								}}
							>
								<div className="flex w-full items-start gap-2">
									{icon(n.level)}
									<div className="flex-1 min-w-0">
										<div className="flex items-center justify-between gap-2">
											<div
												className={`text-sm font-medium ${n.read ? "text-muted-foreground" : "text-foreground"}`}
											>
												{n.title}
											</div>
											<div className="ml-2 text-[10px] text-muted-foreground whitespace-nowrap">
												{new Date(n.createdAt).toLocaleTimeString()}
											</div>
										</div>
										{n.description ? (
											<div className="mt-0.5 text-xs text-muted-foreground line-clamp-3">
												{n.description}
											</div>
										) : null}
									</div>
								</div>
							</DropdownMenuItem>
						))}
					</div>
				)}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
