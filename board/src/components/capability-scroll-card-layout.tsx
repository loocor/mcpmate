import type { ReactNode } from "react";
import { CardContent } from "./ui/card";

export const CAPABILITY_SCROLL_CARD_CLASS =
	"flex min-h-0 flex-1 flex-col overflow-hidden";

export function CapabilityScrollCardContent({ children }: { children: ReactNode }) {
	return (
		<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden px-4 py-0">
			{children}
		</CardContent>
	);
}
