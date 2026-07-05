import { ChevronUp, Pin } from "lucide-react";
import {
	forwardRef,
	type PointerEvent as ReactPointerEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
} from "react";
import { cn } from "../lib/utils";
import {
	INSPECTOR_BOTTOM_BAR_HEADER_ACTIONS_CLASSNAME,
	INSPECTOR_BOTTOM_BAR_HEADER_WITH_ACTIONS_CLASSNAME,
	INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME,
	INSPECTOR_BOTTOM_BAR_TOGGLE_CLASSNAME,
	INSPECTOR_BOTTOM_BAR_MAX_HEIGHT_PX,
	INSPECTOR_BOTTOM_BAR_MIN_HEIGHT_PX,
	resolveInspectorBottomBarHeight,
} from "../lib/inspector-bottom-bar";
import {
	InspectorExpandableSearch,
	type InspectorExpandableSearchProps,
} from "./inspector-expandable-search";
import { INSPECTOR_ACTIVITY_TRIGGER_SELECTOR } from "./layout/inspector-chrome-context";

export type InspectorBottomPanelSearchConfig = InspectorExpandableSearchProps;

type InspectorBottomPanelProps = {
	expanded: boolean;
	onExpandedChange: (expanded: boolean) => void;
	height: number;
	onHeightChange: (height: number) => void;
	title: ReactNode;
	search?: InspectorBottomPanelSearchConfig;
	pinned?: boolean;
	onPinnedChange?: (pinned: boolean) => void;
	pinAriaLabel?: string;
	unpinAriaLabel?: string;
	headerActions?: ReactNode;
	children: ReactNode;
	className?: string;
	minHeight?: number;
	maxHeight?: number;
	resizeAriaLabel?: string;
	toggleAriaLabel?: string;
};

export const InspectorBottomPanel = forwardRef<HTMLDivElement, InspectorBottomPanelProps>(
	function InspectorBottomPanel(
		{
			expanded,
			onExpandedChange,
			height,
			onHeightChange,
			title,
			search,
			pinned = false,
			onPinnedChange,
			pinAriaLabel = "Pin panel",
			unpinAriaLabel = "Unpin panel",
			headerActions,
			children,
			className,
			minHeight = INSPECTOR_BOTTOM_BAR_MIN_HEIGHT_PX,
			maxHeight = INSPECTOR_BOTTOM_BAR_MAX_HEIGHT_PX,
			resizeAriaLabel = "Resize panel",
			toggleAriaLabel,
		},
		ref,
	) {
		const panelRef = useRef<HTMLDivElement | null>(null);

		const setPanelRef = useCallback(
			(node: HTMLDivElement | null) => {
				panelRef.current = node;
				if (typeof ref === "function") {
					ref(node);
				} else if (ref) {
					ref.current = node;
				}
			},
			[ref],
		);

		useEffect(() => {
			if (!expanded || pinned) {
				return;
			}

			const collapseIfOutside = (target: EventTarget | null) => {
				if (!(target instanceof Node) || panelRef.current?.contains(target)) {
					return;
				}
				if (
					target instanceof Element &&
					target.closest(INSPECTOR_ACTIVITY_TRIGGER_SELECTOR)
				) {
					return;
				}
				onExpandedChange(false);
			};

			const handlePointerDown = (event: PointerEvent) => {
				collapseIfOutside(event.target);
			};

			const handleFocusIn = (event: FocusEvent) => {
				collapseIfOutside(event.target);
			};

			document.addEventListener("pointerdown", handlePointerDown);
			document.addEventListener("focusin", handleFocusIn);
			return () => {
				document.removeEventListener("pointerdown", handlePointerDown);
				document.removeEventListener("focusin", handleFocusIn);
			};
		}, [expanded, onExpandedChange, pinned]);

		const handleHeaderClick = useCallback(() => {
			if (pinned) {
				onExpandedChange(!expanded);
				return;
			}
			if (!expanded) {
				onExpandedChange(true);
			}
		}, [expanded, onExpandedChange, pinned]);

		const handlePinClick = useCallback(() => {
			onPinnedChange?.(!pinned);
		}, [onPinnedChange, pinned]);

		const clampHeight = useCallback(
			(nextHeight: number) => Math.min(maxHeight, Math.max(minHeight, nextHeight)),
			[maxHeight, minHeight],
		);

		const handleResizePointerDown = useCallback(
			(event: ReactPointerEvent<HTMLButtonElement>) => {
				event.preventDefault();
				event.stopPropagation();
				const startY = event.clientY;
				const startHeight = height;
				const handlePointerMove = (moveEvent: PointerEvent) => {
					const delta = startY - moveEvent.clientY;
					onHeightChange(clampHeight(startHeight + delta));
				};
				const handlePointerUp = () => {
					window.removeEventListener("pointermove", handlePointerMove);
					window.removeEventListener("pointerup", handlePointerUp);
				};
				window.addEventListener("pointermove", handlePointerMove);
				window.addEventListener("pointerup", handlePointerUp);
			},
			[clampHeight, height, onHeightChange],
		);

		const panelHeight = resolveInspectorBottomBarHeight(expanded, height);

		return (
			<div
				ref={setPanelRef}
				className={cn(
					"pointer-events-auto absolute inset-x-0 bottom-0 z-20 flex flex-col overflow-hidden border-t border-border bg-card shadow-[0_-10px_40px_rgba(15,23,42,0.08)] dark:shadow-[0_-10px_40px_rgba(0,0,0,0.45)]",
					className,
				)}
				style={{ height: panelHeight }}
			>
				{expanded ? (
					<button
						type="button"
						aria-label={resizeAriaLabel}
						className="absolute inset-x-0 top-0 z-10 h-1.5 cursor-row-resize bg-transparent focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						onPointerDown={handleResizePointerDown}
					/>
				) : null}
				<div className={INSPECTOR_BOTTOM_BAR_HEADER_WITH_ACTIONS_CLASSNAME}>
					<button
						type="button"
						className={INSPECTOR_BOTTOM_BAR_TOGGLE_CLASSNAME}
						aria-expanded={expanded}
						aria-label={toggleAriaLabel}
						onClick={handleHeaderClick}
					>
						<ChevronUp
							className={cn(
								"h-3 w-3 shrink-0 text-muted-foreground transition-transform duration-200",
								expanded && "rotate-180",
							)}
							aria-hidden
						/>
						<span className="min-w-0 truncate text-xs font-medium text-foreground">
							{title}
						</span>
					</button>
					{search || onPinnedChange || headerActions ? (
						<div className={INSPECTOR_BOTTOM_BAR_HEADER_ACTIONS_CLASSNAME}>
							{search ? <InspectorExpandableSearch {...search} /> : null}
							{headerActions}
							{onPinnedChange ? (
								<button
									type="button"
									className={cn(
										INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME,
										pinned && "text-foreground",
									)}
									aria-label={pinned ? unpinAriaLabel : pinAriaLabel}
									aria-pressed={pinned}
									onClick={handlePinClick}
								>
									<Pin
										className={cn(
											"h-3.5 w-3.5 origin-center transition-transform duration-200",
											pinned ? "rotate-0" : "rotate-45",
										)}
									/>
								</button>
							) : null}
						</div>
					) : null}
				</div>
				{expanded ? (
					<div className="flex min-h-0 flex-1 flex-col overflow-hidden">{children}</div>
				) : null}
			</div>
		);
	},
);

InspectorBottomPanel.displayName = "InspectorBottomPanel";
