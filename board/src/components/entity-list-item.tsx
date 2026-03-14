import type { ReactNode } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "../components/ui/avatar";
import { Switch } from "../components/ui/switch";

export interface EntityListItemProps {
	// Basic info
	id: string;
	title: string;
	description?: string | ReactNode;
	avatar?: {
		src?: string;
		alt?: string;
		fallback: string;
	};

	// Content area
	titleBadges?: ReactNode[];
	stats?: Array<{
		label: string;
		value: string | number;
	}>;
	bottomTags?: ReactNode[];

	// Right side controls
	statusBadge?: ReactNode;
	enableSwitch?: {
		checked: boolean;
		onChange: (checked: boolean) => void;
		disabled?: boolean;
	};
	actionButtons?: ReactNode[];

	// Interaction
	onClick?: () => void;
	onKeyDown?: (e: React.KeyboardEvent) => void;

	// Styling
	className?: string;
}

export function EntityListItem({
	id,
	title,
	description,
	avatar,
	titleBadges = [],
	stats = [],
	bottomTags = [],
	statusBadge,
	enableSwitch,
	actionButtons = [],
	onClick,
	onKeyDown,
	className = "",
}: EntityListItemProps) {
	const handleClick = (e: React.MouseEvent) => {
		const target = e.target as HTMLElement;
		// Prevent clicking interactive components in child elements, but not decorative elements
		if (
			target.closest(
				"button:not([data-decorative]):not([data-list-item]), a, input, [role='switch']",
			)
		) {
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
		<div
			key={id}
			role="button"
			tabIndex={0}
			data-list-item
			className={`w-full flex items-center justify-between rounded-lg border border-slate-200 bg-white px-4 py-4 cursor-pointer shadow-[0_4px_12px_-10px_rgba(15,23,42,0.2)] transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 dark:border-slate-800 dark:bg-slate-950 dark:shadow-[0_4px_12px_-10px_rgba(15,23,42,0.5)] focus:outline-none focus:ring-2 focus:ring-primary/20 ${className}`}
			onClick={handleClick}
			onKeyDown={handleKeyDown}
		>
			{/* Left side content */}
			<div className="flex items-center gap-3">
				{/* Avatar */}
				<Avatar className="bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200">
					{avatar?.src && (
						<AvatarImage src={avatar.src} alt={avatar.alt || title} />
					)}
					<AvatarFallback>
						{avatar?.fallback || title.charAt(0).toUpperCase()}
					</AvatarFallback>
				</Avatar>

				{/* Content area */}
				<div className="space-y-2">
					{/* Title and title badges */}
					<div className="flex items-center gap-2">
						<h3 className="font-medium text-sm leading-tight">{title}</h3>
						{titleBadges.map((badge, index) => (
							<span key={`title-badge-${id}-${index}`}>{badge}</span>
						))}
					</div>

					{/* Description */}
					<div className="text-sm text-slate-500 line-clamp-2 text-left">
						{description || "N/A"}
					</div>

					{/* Stats */}
					{stats.length > 0 && (
						<div className="flex flex-wrap gap-4 text-xs text-slate-400">
							{stats.map((stat, index) => (
								<span key={`stat-${id}-${index}`}>
									{stat.label}: {stat.value}
								</span>
							))}
						</div>
					)}

					{/* Bottom tags */}
					{bottomTags.length > 0 && (
						<div className="flex flex-wrap gap-1 text-xs text-slate-500">
							{bottomTags.map((tag, index) => (
								<span key={`bottom-tag-${id}-${index}`}>{tag}</span>
							))}
						</div>
					)}
				</div>
			</div>

			{/* Right side controls */}
			<div className="flex items-center gap-2">
				{/* Status badge */}
				{statusBadge && statusBadge}

				{/* Enable/disable switch */}
				{enableSwitch && (
					<Switch
						checked={enableSwitch.checked}
						onCheckedChange={enableSwitch.onChange}
						onClick={(e) => e.stopPropagation()}
						disabled={enableSwitch.disabled}
					/>
				)}

				{/* Action buttons */}
				{actionButtons.map((button, index) => (
					<div
						key={`action-${id}-${index}`}
						onClick={(e) => e.stopPropagation()}
						onKeyDown={(e) => e.stopPropagation()}
					>
						{button}
					</div>
				))}
			</div>
		</div>
	);
}
