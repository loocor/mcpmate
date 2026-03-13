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
import { useNotify } from "../lib/notify";

export function NotificationCenter() {
	const { t } = useTranslation();
	const { items, markAllRead, markRead, clear, isOpen, setOpen } = useNotify();
	const unreadCount = items.reduce((acc, it) => acc + (it.read ? 0 : 1), 0);

	function icon(level: string) {
		switch (level) {
			case "success":
				return <CheckCircle2 className="h-4 w-4 text-emerald-600" />;
			case "warning":
				return <AlertTriangle className="h-4 w-4 text-amber-600" />;
			case "error":
				return <XCircle className="h-4 w-4 text-red-600" />;
			default:
				return <Info className="h-4 w-4 text-blue-600" />;
		}
	}

	return (
		<DropdownMenu open={isOpen} onOpenChange={setOpen}>
			<DropdownMenuTrigger asChild>
				<button
					type="button"
					className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
					aria-label={t("notifications.title")}
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
			<DropdownMenuContent
				align="end"
				className="w-[360px] max-h-[60vh] overflow-auto p-0"
			>
				<div className="px-3 py-2 flex items-center justify-between sticky top-0 bg-popover z-10 border-b border-slate-200 dark:border-slate-700">
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
					<div className="p-4 text-sm text-slate-500">
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
										<div className="flex items-center justify-between">
											<div
												className={`text-sm font-medium ${n.read ? "text-slate-600" : "text-slate-900"}`}
											>
												{n.title}
											</div>
											<div className="ml-2 text-[10px] text-slate-400 whitespace-nowrap">
												{new Date(n.createdAt).toLocaleTimeString()}
											</div>
										</div>
										{n.description ? (
											<div className="mt-0.5 text-xs text-slate-500 line-clamp-3">
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
