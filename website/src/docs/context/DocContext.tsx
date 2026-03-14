import type React from "react";
import {
	createContext,
	useCallback,
	useContext,
	useMemo,
	useRef,
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

export function useDocContext(): DocContextValue {
	const ctx = useContext(DocContext);
	if (!ctx) throw new Error("useDocContext must be used within <DocProvider>");
	return ctx;
}

export function DocProvider({ children }: { children: React.ReactNode }) {
	const [headings, setHeadings] = useState<Heading[]>([]);
	const ids = useRef(new Set<string>());

	const registerHeading = useCallback((h: Heading) => {
		setHeadings((prev) => {
			if (ids.current.has(h.id)) return prev;
			ids.current.add(h.id);
			const next = [...prev, h];
			return next.sort(
				(a, b) => a.level - b.level || a.text.localeCompare(b.text),
			);
		});
	}, []);

	const unregisterHeading = useCallback((id: string) => {
		setHeadings((prev) => prev.filter((h) => h.id !== id));
		ids.current.delete(id);
	}, []);

	const value = useMemo(
		() => ({ headings, registerHeading, unregisterHeading }),
		[headings, registerHeading, unregisterHeading],
	);

	return <DocContext.Provider value={value}>{children}</DocContext.Provider>;
}
