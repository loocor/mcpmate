import type { ComponentType, ReactNode } from "react";
import { Activity, Sparkles } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { MetricsTrendChart } from "../../components/metrics-trend-chart";
import { TokenSavingsTrendCard } from "../../components/token-savings-trend-card";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { cn } from "../../lib/utils";

const SLIDE_CLASS =
	"w-full shrink-0 grow-0 basis-full snap-center snap-always";

const METRICS_LOGICAL_INDEX = 0;
const TOKENS_LOGICAL_INDEX = 1;
/** Index of the first real slide inside the extended loop track. */
const LOOP_REAL_START_INDEX = 1;

type OperatorChartId = "metrics" | "tokens";

interface OperatorChartSlideDef {
	id: OperatorChartId | `${OperatorChartId}-loop-head` | `${OperatorChartId}-loop-tail`;
	title: string;
	icon: ComponentType<{ className?: string }>;
	render: () => ReactNode;
}

function OperatorChartSlide({
	ariaHidden,
	children,
	className,
	slideId,
	title,
	icon: Icon,
}: {
	ariaHidden?: boolean;
	children: ReactNode;
	className?: string;
	slideId: string;
	title: string;
	icon: ComponentType<{ className?: string }>;
}) {
	return (
		<article
			className={cn(SLIDE_CLASS, "box-border px-3", className)}
			data-slide-id={slideId}
			aria-hidden={ariaHidden || undefined}
		>
			<div className="mb-1.5 flex items-center justify-between gap-2">
				<div className="flex min-w-0 items-center gap-1.5">
					<Icon className="h-3.5 w-3.5 shrink-0 text-sky-500" aria-hidden />
					<h2 className="truncate text-xs font-medium text-slate-900 dark:text-slate-100">
						{title}
					</h2>
				</div>
			</div>
			{children}
		</article>
	);
}

function buildLoopTrack(slides: OperatorChartSlideDef[]): OperatorChartSlideDef[] {
	const metrics = slides[METRICS_LOGICAL_INDEX];
	const tokens = slides[TOKENS_LOGICAL_INDEX];
	return [
		{
			...tokens,
			id: "tokens-loop-head",
			render: tokens.render,
		},
		metrics,
		tokens,
		{
			...metrics,
			id: "metrics-loop-tail",
			render: metrics.render,
		},
	];
}

export function OperatorChartCarousel() {
	usePageTranslations("dashboard");
	usePageTranslations("operator");
	const { t, i18n } = useTranslation();
	const trackRef = React.useRef<HTMLDivElement>(null);
	const jumpingRef = React.useRef(false);
	const [activeIndex, setActiveIndex] = React.useState(METRICS_LOGICAL_INDEX);

	const logicalSlides = React.useMemo<OperatorChartSlideDef[]>(
		() => [
			{
				id: "metrics",
				title: t("dashboard:metrics.title", {
					lng: i18n.language,
					defaultValue: "Metrics",
				}),
				icon: Activity,
				render: () => <MetricsTrendChart variant="compact" />,
			},
			{
				id: "tokens",
				title: t("dashboard:tokenSavings.title", {
					lng: i18n.language,
					defaultValue: "Token Savings",
				}),
				icon: Sparkles,
				render: () => <TokenSavingsTrendCard hideHeader variant="compact" />,
			},
		],
		[t, i18n.language],
	);

	const loopTrack = React.useMemo(() => buildLoopTrack(logicalSlides), [logicalSlides]);

	const scrollToLogicalIndex = React.useCallback(
		(logicalIndex: number, behavior: ScrollBehavior = "smooth") => {
			const track = trackRef.current;
			if (!track) {
				return;
			}
			const width = track.clientWidth;
			if (width <= 0) {
				return;
			}
			track.scrollTo({
				left: width * (logicalIndex + LOOP_REAL_START_INDEX),
				behavior,
			});
			setActiveIndex(logicalIndex);
		},
		[],
	);

	const syncLoopPosition = React.useCallback(() => {
		if (jumpingRef.current) {
			return;
		}
		const track = trackRef.current;
		if (!track) {
			return;
		}
		const width = track.clientWidth;
		if (width <= 0) {
			return;
		}

		const rawIndex = Math.round(track.scrollLeft / width);
		const lastIndex = loopTrack.length - 1;

		if (rawIndex <= 0) {
			jumpingRef.current = true;
			track.scrollTo({
				left: width * (TOKENS_LOGICAL_INDEX + LOOP_REAL_START_INDEX),
				behavior: "instant",
			});
			setActiveIndex(TOKENS_LOGICAL_INDEX);
			window.requestAnimationFrame(() => {
				jumpingRef.current = false;
			});
			return;
		}

		if (rawIndex >= lastIndex) {
			jumpingRef.current = true;
			track.scrollTo({
				left: width * (METRICS_LOGICAL_INDEX + LOOP_REAL_START_INDEX),
				behavior: "instant",
			});
			setActiveIndex(METRICS_LOGICAL_INDEX);
			window.requestAnimationFrame(() => {
				jumpingRef.current = false;
			});
			return;
		}

		setActiveIndex(rawIndex - LOOP_REAL_START_INDEX);
	}, [loopTrack.length]);

	React.useLayoutEffect(() => {
		scrollToLogicalIndex(METRICS_LOGICAL_INDEX, "instant");
	}, [scrollToLogicalIndex]);

	React.useEffect(() => {
		const track = trackRef.current;
		if (!track) {
			return;
		}

		let scrollTimer: number | undefined;

		const onScroll = () => {
			if (scrollTimer !== undefined) {
				window.clearTimeout(scrollTimer);
			}
			scrollTimer = window.setTimeout(syncLoopPosition, 100);
		};

		track.addEventListener("scrollend", syncLoopPosition);
		track.addEventListener("scroll", onScroll);

		const resizeObserver = new ResizeObserver(() => {
			scrollToLogicalIndex(activeIndex, "instant");
		});
		resizeObserver.observe(track);

		return () => {
			track.removeEventListener("scrollend", syncLoopPosition);
			track.removeEventListener("scroll", onScroll);
			resizeObserver.disconnect();
			if (scrollTimer !== undefined) {
				window.clearTimeout(scrollTimer);
			}
		};
	}, [activeIndex, scrollToLogicalIndex, syncLoopPosition]);

	return (
		<section
			className="shrink-0 overflow-hidden border-b border-slate-200 pt-2 dark:border-slate-800"
			data-testid="operator-chart-carousel"
			aria-label={t("operator:charts.carouselLabel", {
				defaultValue: "Resource and token usage charts",
			})}
		>
			<div
				ref={trackRef}
				className="flex w-full snap-x snap-mandatory overflow-x-auto overscroll-x-contain scroll-smooth touch-pan-x [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
				role="group"
				aria-roledescription="carousel"
				aria-live="polite"
			>
				{loopTrack.map((slide, index) => {
					const isClone =
						index === 0 || index === loopTrack.length - 1;
					return (
						<OperatorChartSlide
							key={`${slide.id}-${index}`}
							slideId={slide.id}
							title={slide.title}
							icon={slide.icon}
							ariaHidden={isClone}
						>
							{slide.render()}
						</OperatorChartSlide>
					);
				})}
			</div>
			<div
				className="flex items-center justify-center gap-1.5 px-3 pb-2 pt-1"
				data-testid="operator-chart-carousel-dots"
				role="tablist"
				aria-label={t("operator:charts.pagination", {
					defaultValue: "Chart pagination",
				})}
			>
				{logicalSlides.map((slide, index) => {
					const isActive = activeIndex === index;
					return (
						<button
							key={slide.id}
							type="button"
							role="tab"
							aria-selected={isActive}
							aria-label={t("operator:charts.goToSlide", {
								title: slide.title,
								defaultValue: "Go to {{title}}",
							})}
							className={cn(
								"h-1.5 rounded-full transition-all duration-200",
								isActive
									? "w-4 bg-sky-500"
									: "w-1.5 bg-slate-300 hover:bg-slate-400 dark:bg-slate-600 dark:hover:bg-slate-500",
							)}
							onClick={() => {
								scrollToLogicalIndex(index);
							}}
						/>
					);
				})}
			</div>
		</section>
	);
}
