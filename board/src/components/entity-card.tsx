import type { ReactNode } from "react";
import { CachedAvatar } from "../components/cached-avatar";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "../components/ui/card";

export interface EntityCardProps {
	id: string;
	title: string;
	description?: string | ReactNode;
	avatar?: {
		src?: string;
		alt?: string;
		fallback: string;
	};
	avatarShape?: "circle" | "rounded" | "square";

	topRightBadge?: ReactNode;

	// 统计信息 (4x2 网格)
	stats?: Array<{ label: string; value: string | number }>;

	// 底部左侧内容
	bottomLeft?: ReactNode;

	// 底部右侧内容
	bottomRight?: ReactNode;

	// 交互
	onClick?: () => void;
	onKeyDown?: (e: React.KeyboardEvent) => void;
	className?: string;
	titleClassName?: string;
}

export function EntityCard({
	id,
	title,
	description,
	avatar,
	avatarShape = "circle",
	topRightBadge,
	stats = [],
	bottomLeft,
	bottomRight,
	onClick,
	onKeyDown,
	className = "",
	titleClassName = "",
}: EntityCardProps) {
	const handleClick = (e: React.MouseEvent) => {
		const target = e.target as HTMLElement;
		// 阻止点击子元素中的交互组件
		if (target.closest("button, a, input, [role='switch']")) {
			return;
		}
		onClick?.();
	};

	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			onClick?.();
		}
		onKeyDown?.(e);
	};

	return (
		<Card
			className={`group flex h-full cursor-pointer flex-col overflow-hidden border border-slate-200 transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 dark:border-slate-700 ${className}`}
			role="button"
			tabIndex={0}
			onClick={handleClick}
			onKeyDown={handleKeyDown}
		>
			<CardHeader className="p-4 pb-2">
				<div className="flex min-w-0 items-start gap-3">
					<CachedAvatar
						src={avatar?.src}
						alt={avatar?.alt || title}
						fallback={avatar?.fallback || title}
						size="lg"
						shape={avatarShape}
						className="bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200 font-semibold"
					/>
					<div className="min-w-0 flex-1 space-y-2">
						<div className="flex min-w-0 items-start justify-between gap-2">
						<CardTitle
							className={`min-w-0 flex-1 truncate text-lg font-semibold leading-tight ${titleClassName}`}
							title={title}
						>
								{title}
							</CardTitle>
							{topRightBadge ? (
								<div className="flex shrink-0 flex-row-reverse flex-nowrap gap-1 pt-0.5">
									{topRightBadge}
								</div>
							) : null}
						</div>
						<div className="h-10 flex min-w-0 items-start">
							{typeof description === "string" || description === undefined ? (
								<CardDescription className="text-sm text-slate-500 line-clamp-2 leading-5">
									{description || "N/A"}
								</CardDescription>
							) : (
								<div className="min-w-0 text-sm text-slate-500 line-clamp-2 leading-5">
									{description}
								</div>
							)}
						</div>
					</div>
				</div>
			</CardHeader>

			{stats.length > 0 && (
				<CardContent className="flex flex-1 flex-col gap-2 px-4 pb-4 pt-2">
					<div className="flex items-start gap-3">
						<div className="w-12"></div>
						<div className="flex-1 grid grid-cols-4 gap-x-6 gap-y-1">
							{stats.map((item) => (
								<span
									key={`label-${id}-${item.label}`}
									className="text-[9px] uppercase tracking-wide text-muted-foreground/80"
								>
									{item.label}
								</span>
							))}
							{stats.map((item) => (
								<span
									key={`value-${id}-${item.label}`}
									className="text-[9px] tracking-wide text-muted-foreground/80"
								>
									{item.value}
								</span>
							))}
						</div>
					</div>
				</CardContent>
			)}

			<CardFooter className="flex items-center justify-between gap-2 px-4 pb-4 pt-0">
				<div className="flex items-start gap-3">
					<div className="w-12"></div>
					<div className="flex flex-wrap items-center gap-2">{bottomLeft}</div>
				</div>
				<div className="flex items-center gap-3">{bottomRight}</div>
			</CardFooter>
		</Card>
	);
}
