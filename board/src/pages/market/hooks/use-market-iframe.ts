import { useRef } from "react";
import type { UseMarketIframeReturn } from "../types";

export function useMarketIframe(): UseMarketIframeReturn {
	const iframeRef = useRef<HTMLIFrameElement | null>(null);

	const handleIframeLoad = () => {
		// Complex iframe loading logic can be implemented here
		// This is a simplified version for now
	};

	return {
		iframeRef,
		handleIframeLoad,
	};
}
