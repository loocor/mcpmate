import type React from "react";
import {
	createContext,
	useCallback,
	useContext,
	useMemo,
	useState,
} from "react";

export type Heading = {
	id: string;
	level: 2 | 3;
	text: string;
	el?: HTMLElement | null;
};

type DocContextValue = {
	headings: Heading[];
	registerHeading: (h: Heading) => void;
	unregisterHeading: (id: string) => void;
};

const DocContext = createContext<DocContextValue | undefined>(undefined);

const DOCUMENT_POSITION_PRECEDING = 2;
const DOCUMENT_POSITION_FOLLOWING = 4;

function sortByDocumentOrder(headings: Heading[]): Heading[] {
	return [...headings].sort((a, b) => {
		if (!a.el || !b.el || a.el === b.el) {
			return 0;
		}

		const position = a.el.compareDocumentPosition(b.el);
		if (position & DOCUMENT_POSITION_FOLLOWING) {
			return -1;
		}
		if (position & DOCUMENT_POSITION_PRECEDING) {
			return 1;
		}
		return 0;
	});
}

export function useDocContext(): DocContextValue {
	const ctx = useContext(DocContext);
	if (!ctx) throw new Error("useDocContext must be used within <DocProvider>");
	return ctx;
}

export function DocProvider({ children }: { children: React.ReactNode }) {
	const [headings, setHeadings] = useState<Heading[]>([]);

	const registerHeading = useCallback((h: Heading) => {
		setHeadings((prev) => {
			const index = prev.findIndex((heading) => heading.id === h.id);
			if (index === -1) {
				return sortByDocumentOrder([...prev, h]);
			}

			const next = [...prev];
			next[index] = h;
			return sortByDocumentOrder(next);
		});
	}, []);

	const unregisterHeading = useCallback((id: string) => {
		setHeadings((prev) => prev.filter((h) => h.id !== id));
	}, []);

	const value = useMemo(
		() => ({ headings, registerHeading, unregisterHeading }),
		[headings, registerHeading, unregisterHeading],
	);

	return <DocContext.Provider value={value}>{children}</DocContext.Provider>;
}
