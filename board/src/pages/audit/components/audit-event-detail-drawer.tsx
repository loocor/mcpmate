import { useQuery } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Drawer, DrawerContent, DrawerDescription, DrawerHeader, DrawerTitle } from "../../../components/ui/drawer";
import { auditApi } from "../../../lib/api";
import type { AuditEventRecord } from "../../../lib/types";
import { formatLocalDateTime } from "../../../lib/utils";
import { AuditEventDetails } from "./audit-event-details";

function buildDrawerDescription(event: AuditEventRecord | undefined, t: TFunction): string {
	if (event == null) {
		return t("audit:drawer.subtitle", {
			defaultValue: "Load the complete event payload for inspection",
		});
	}
	const actionLabel = t(`audit:actionValues.${event.action}`, { defaultValue: event.action });
	return `${actionLabel} · ${formatLocalDateTime(event.occurred_at_ms)}`;
}

function AuditDrawerBody(props: {
	isLoading: boolean;
	isError: boolean;
	event: AuditEventRecord | undefined;
	t: TFunction;
}) {
	const { isLoading, isError, event, t } = props;

	if (isLoading) {
		return (
			<div className="text-sm text-muted-foreground">
				{t("audit:drawer.loading", { defaultValue: "Loading event details…" })}
			</div>
		);
	}
	if (isError) {
		return (
			<div className="text-sm text-destructive">
				{t("audit:drawer.error", { defaultValue: "Failed to load event details" })}
			</div>
		);
	}
	if (event) {
		return <AuditEventDetails event={event} t={t} />;
	}
	return null;
}

interface AuditEventDetailDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	eventId: number | null;
}

export function AuditEventDetailDrawer(props: AuditEventDetailDrawerProps) {
	const { open, onOpenChange, eventId } = props;
	const { t } = useTranslation("audit");

	const query = useQuery<AuditEventRecord>({
		queryKey: ["audit", "event-details", eventId],
		queryFn: async () => {
			if (eventId == null) {
				throw new Error("Missing audit event id");
			}
			return auditApi.details(eventId);
		},
		enabled: open && eventId != null,
		refetchOnWindowFocus: false,
		retry: false,
	});

	const event = query.data;
	const descriptionText = buildDrawerDescription(event, t);

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent className="flex h-full flex-col overflow-hidden">
				<DrawerHeader className="shrink-0">
					<DrawerTitle>
						{t("audit:drawer.title", { defaultValue: "Audit Event Details" })}
					</DrawerTitle>
					<DrawerDescription>{descriptionText}</DrawerDescription>
				</DrawerHeader>
				<div className="flex-1 overflow-y-auto px-4 py-3">
					<AuditDrawerBody
						isLoading={query.isLoading}
						isError={query.isError}
						event={event}
						t={t}
					/>
				</div>
			</DrawerContent>
		</Drawer>
	);
}
