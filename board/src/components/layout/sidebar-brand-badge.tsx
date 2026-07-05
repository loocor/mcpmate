import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";

export type SidebarBrandBadgeVariant = "beta" | "inspector";

type SidebarBrandBadgeProps = {
	variant?: SidebarBrandBadgeVariant;
	className?: string;
};

export function SidebarBrandBadge({
	variant = "beta",
	className,
}: SidebarBrandBadgeProps) {
	const { t } = useTranslation(variant === "inspector" ? "inspector" : undefined);

	const label =
		variant === "inspector"
			? t("standalone.eyebrow", { defaultValue: "Inspector" })
			: t("layout.alpha", { defaultValue: "Beta" });

	return (
		<sup className={cn("text-[9px] text-muted-foreground", className)}>
			{label}
		</sup>
	);
}
