import React from "react";
import { useDocContext } from "../context/DocContext";

function slugify(input: string): string {
	return input
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9\u4e00-\u9fa5\s-]/g, "")
		.replace(/\s+/g, "-")
		.replace(/-+/g, "-");
}

export const H2: React.FC<React.PropsWithChildren<{ id?: string }>> = ({
	id,
	children,
}) => {
	const { registerHeading, unregisterHeading } = useDocContext();
	const text = React.useMemo(
		() => (typeof children === "string" ? children : String(children)),
		[children],
	) as string;
	const resolvedId = React.useMemo(() => id || slugify(text), [id, text]);
	const ref = React.useRef<HTMLHeadingElement>(null);

	React.useEffect(() => {
		const el = ref.current;
		registerHeading({ id: resolvedId, level: 2, text, el });
		return () => unregisterHeading(resolvedId);
	}, [registerHeading, unregisterHeading, resolvedId, text]);

	return (
		<h2
			id={resolvedId}
			ref={ref}
			className="scroll-mt-24 text-2xl font-semibold flex items-center gap-2"
		>
			<a
				href={`#${resolvedId}`}
				className="opacity-30 group-hover:opacity-100 transition-opacity text-slate-400 dark:text-slate-500 hover:text-blue-600 dark:hover:text-blue-400"
			>
				#
			</a>
			<span>{children}</span>
		</h2>
	);
};

export const H3: React.FC<React.PropsWithChildren<{ id?: string }>> = ({
	id,
	children,
}) => {
	const { registerHeading, unregisterHeading } = useDocContext();
	const text = React.useMemo(
		() => (typeof children === "string" ? children : String(children)),
		[children],
	) as string;
	const resolvedId = React.useMemo(() => id || slugify(text), [id, text]);
	const ref = React.useRef<HTMLHeadingElement>(null);

	React.useEffect(() => {
		const el = ref.current;
		registerHeading({ id: resolvedId, level: 3, text, el });
		return () => unregisterHeading(resolvedId);
	}, [registerHeading, unregisterHeading, resolvedId, text]);

	return (
		<h3
			id={resolvedId}
			ref={ref}
			className="scroll-mt-24 text-xl font-semibold flex items-center gap-2"
		>
			<a
				href={`#${resolvedId}`}
				className="opacity-30 group-hover:opacity-100 transition-opacity text-slate-400 dark:text-slate-500 hover:text-blue-600 dark:hover:text-blue-400"
			>
				#
			</a>
			<span>{children}</span>
		</h3>
	);
};

export const P: React.FC<React.PropsWithChildren> = ({ children }) => (
	<p className="leading-7 text-slate-700 dark:text-slate-300">{children}</p>
);

export const Ul: React.FC<React.PropsWithChildren> = ({ children }) => (
	<ul className="list-disc pl-6 space-y-2">{children}</ul>
);

export const Ol: React.FC<React.PropsWithChildren> = ({ children }) => (
	<ol className="list-decimal pl-6 space-y-2">{children}</ol>
);

export const Li: React.FC<React.PropsWithChildren> = ({ children }) => (
	<li>{children}</li>
);
