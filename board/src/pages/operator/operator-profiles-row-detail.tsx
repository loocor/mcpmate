import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { Plus } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { configSuitsApi } from "../../lib/api";
import { DEFAULT_ANCHOR_ROLE } from "../../lib/default-profile";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import type { ConfigSuit } from "../../lib/types";
import { cn } from "../../lib/utils";
import {
	OperatorChipAvatar,
	type OperatorChipVisual,
} from "./operator-row-detail-shared";

const noDragRegionStyle = { WebkitAppRegion: "no-drag" } as React.CSSProperties;
const PROFILE_ITEM_WIDTH_CLASS = "w-[52px]";

function formatProfileDisplayName(
	name: string | null | undefined,
	fallback: string | undefined,
	t: TFunction,
): string {
	const raw = typeof name === "string" ? name.trim() : "";
	if (raw.length > 0) {
		return raw
			.split(/\s+/)
			.map((word) =>
				word.length > 0
					? word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
					: word,
			)
			.join(" ");
	}
	return fallback ?? t("profiles:untitledProfile", { defaultValue: "Untitled Profile" });
}

function sortOperatorProfiles(profiles: ConfigSuit[]): ConfigSuit[] {
	return profiles
		.filter((profile) => profile.suit_type !== "host_app")
		.sort((left, right) => {
			if (left.is_active !== right.is_active) {
				return left.is_active ? -1 : 1;
			}
			if (left.priority !== right.priority) {
				return left.priority - right.priority;
			}
			return left.name.localeCompare(right.name);
		});
}

function OperatorProfilesMoreControl({
	isTauriShell,
	label,
	moreLabel,
	onOpenProfilesBoard,
}: {
	isTauriShell: boolean;
	label: string;
	moreLabel: string;
	onOpenProfilesBoard: () => void;
}) {
	const buttonClass = cn(
		"flex h-10 w-10 items-center justify-center rounded-full border border-dashed border-slate-300 bg-white text-slate-500 transition-colors hover:border-slate-400 hover:bg-slate-50 hover:text-slate-700 dark:border-slate-600 dark:bg-slate-950 dark:text-slate-400 dark:hover:border-slate-500 dark:hover:bg-slate-900 dark:hover:text-slate-200",
	);

	const profileItemLayoutClass = cn(
		PROFILE_ITEM_WIDTH_CLASS,
		"flex shrink-0 flex-col items-center",
	);

	const content = (
		<>
			<span className={buttonClass}>
				<Plus className="h-4 w-4" aria-hidden />
			</span>
			<span className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-500 dark:text-slate-400">
				{moreLabel}
			</span>
		</>
	);

	if (isTauriShell) {
		return (
			<button
				type="button"
				className={profileItemLayoutClass}
				style={noDragRegionStyle}
				aria-label={label}
				title={label}
				onClick={(event) => {
					event.stopPropagation();
					onOpenProfilesBoard();
				}}
			>
				{content}
			</button>
		);
	}

	return (
		<Link
			to="/profiles"
			className={profileItemLayoutClass}
			style={noDragRegionStyle}
			aria-label={label}
			title={label}
			onClick={(event) => event.stopPropagation()}
		>
			{content}
		</Link>
	);
}

export function OperatorProfilesRowDetail({
	detailId,
	isError,
	isLoading,
	isTauriShell,
	onOpenProfilesBoard,
	profiles,
}: {
	detailId: string;
	isError: boolean;
	isLoading: boolean;
	isTauriShell: boolean;
	onOpenProfilesBoard: () => void;
	profiles: ConfigSuit[];
}) {
	usePageTranslations("profiles");
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const [togglingProfileId, setTogglingProfileId] = React.useState<string | null>(null);

	const sortedProfiles = React.useMemo(
		() => sortOperatorProfiles(profiles),
		[profiles],
	);

	const toggleMutation = useMutation({
		mutationFn: async ({
			profile,
			nextActive,
		}: {
			profile: ConfigSuit;
			nextActive: boolean;
		}) => {
			if (nextActive) {
				await configSuitsApi.activateSuit(profile.id);
			} else {
				await configSuitsApi.deactivateSuit(profile.id);
			}
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: ["operator", "profiles"] });
			void queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			if (variables.nextActive) {
				notifySuccess(
					t("profiles:messages.profileActivated", {
						defaultValue: "Profile activated",
					}),
					t("profiles:messages.profileActivatedDescription", {
						defaultValue: "Profile has been successfully activated",
					}),
				);
			} else {
				notifySuccess(
					t("profiles:messages.profileDeactivated", {
						defaultValue: "Profile deactivated",
					}),
					t("profiles:messages.profileDeactivatedDescription", {
						defaultValue: "Profile has been successfully deactivated",
					}),
				);
			}
		},
		onError: (error, variables) => {
			notifyError(
				variables.nextActive
					? t("profiles:messages.activationFailed", {
							defaultValue: "Activation failed",
						})
					: t("profiles:messages.deactivationFailed", {
							defaultValue: "Deactivation failed",
						}),
				error instanceof Error ? error.message : String(error),
			);
		},
		onSettled: () => {
			setTogglingProfileId(null);
		},
	});

	const handleToggleProfile = React.useCallback(
		(profile: ConfigSuit) => {
			const isDefaultAnchor = (profile.role ?? "user") === DEFAULT_ANCHOR_ROLE;
			if (isDefaultAnchor || toggleMutation.isPending) {
				return;
			}
			setTogglingProfileId(profile.id);
			toggleMutation.mutate({
				profile,
				nextActive: !profile.is_active,
			});
		},
		[toggleMutation],
	);

	const moreLabel = t("operator:detail.profiles.more", {
		defaultValue: "More...",
	});
	const openProfilesLabel = t("operator:detail.profiles.openProfiles", {
		defaultValue: "Open Profiles in Full Board",
	});

	return (
		<div
			id={detailId}
			className="border-t border-slate-100 px-3 py-2.5 dark:border-slate-800"
			data-testid="operator-inline-detail"
		>
			{isLoading ? (
				<p className="text-xs text-slate-500 dark:text-slate-400">
					{t("operator:rows.profiles.loading", {
						defaultValue: "Loading profiles",
					})}
				</p>
			) : isError ? (
				<p className="text-xs text-red-600 dark:text-red-400">
					{t("operator:rows.profiles.error", {
						defaultValue: "Profiles are unavailable",
					})}
				</p>
			) : sortedProfiles.length === 0 ? (
				<p className="text-xs text-slate-500 dark:text-slate-400">
					{t("operator:detail.profiles.empty", {
						defaultValue: "Open Full Board to create or activate a profile.",
					})}
				</p>
			) : (
				<div className="-mx-1 flex items-start gap-2 overflow-x-auto px-1 py-0.5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
					{sortedProfiles.map((profile) => {
						const displayName = formatProfileDisplayName(profile.name, profile.id, t);
						const avatarInitial = displayName.charAt(0).toUpperCase() || "P";
						const isDefaultAnchor = (profile.role ?? "user") === DEFAULT_ANCHOR_ROLE;
						const isBusy = togglingProfileId === profile.id;
						const visual: OperatorChipVisual = profile.is_active ? "active" : "neutral";
						const toggleLabel = profile.is_active
							? t("operator:detail.profiles.deactivate", {
									name: displayName,
									defaultValue: "Deactivate {{name}}",
								})
							: t("operator:detail.profiles.activate", {
									name: displayName,
									defaultValue: "Activate {{name}}",
								});

						return (
							<button
								key={profile.id}
								type="button"
								className={cn(
									PROFILE_ITEM_WIDTH_CLASS,
									"group flex shrink-0 flex-col items-center disabled:cursor-not-allowed disabled:opacity-60",
								)}
								style={noDragRegionStyle}
								disabled={isDefaultAnchor || isBusy}
								aria-pressed={profile.is_active}
								aria-label={toggleLabel}
								title={toggleLabel}
								onClick={(event) => {
									event.stopPropagation();
									handleToggleProfile(profile);
								}}
							>
								<span className={cn(isBusy && "opacity-70")}>
									<OperatorChipAvatar
										avatar={avatarInitial}
										innerClassName={
											profile.is_active
												? undefined
												: "group-hover:border-slate-300 group-hover:bg-slate-50 dark:group-hover:border-slate-600 dark:group-hover:bg-slate-900/80"
										}
										visual={visual}
									/>
								</span>
								<span
									className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-600 dark:text-slate-300"
									title={displayName}
								>
									{displayName}
								</span>
							</button>
						);
					})}
					<OperatorProfilesMoreControl
						isTauriShell={isTauriShell}
						label={openProfilesLabel}
						moreLabel={moreLabel}
						onOpenProfilesBoard={onOpenProfilesBoard}
					/>
				</div>
			)}
		</div>
	);
}
