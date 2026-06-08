import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useCallback, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { secretsApi } from "../lib/api";
import { notifyError, notifySuccess, stringifyError } from "../lib/notify";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "./ui/alert-dialog";
import { buttonVariants } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

export type ProtectionPasswordDialogMode = "set" | "change" | "clear" | "verify";

interface ProtectionPasswordDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	mode: ProtectionPasswordDialogMode;
	scope?: string[];
	onSuccess?: (verifiedPassword?: string) => void;
	onCancel?: () => void;
}

export function ProtectionPasswordDialog({
	open,
	onOpenChange,
	mode,
	scope = ["startup"],
	onSuccess,
	onCancel,
}: ProtectionPasswordDialogProps) {
	const queryClient = useQueryClient();
	const { t } = useTranslation();
	const newPasswordId = useId();
	const confirmPasswordId = useId();
	const currentPasswordId = useId();

	const [newPassword, setNewPassword] = useState("");
	const [confirmPassword, setConfirmPassword] = useState("");
	const [oldPassword, setOldPassword] = useState("");
	const [formError, setFormError] = useState<string | null>(null);
	const firstInputRef = useRef<HTMLInputElement>(null);

	const resetForm = useCallback(() => {
		setNewPassword("");
		setConfirmPassword("");
		setOldPassword("");
		setFormError(null);
	}, []);

	const handleOpenChange = useCallback(
		(nextOpen: boolean) => {
			onOpenChange(nextOpen);
			if (!nextOpen) {
				resetForm();
			}
		},
		[onOpenChange, resetForm],
	);

	const setPasswordMutation = useMutation({
		mutationFn: () => secretsApi.setPassword(newPassword, confirmPassword, scope),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["password", "status"] });
			handleOpenChange(false);
			onSuccess?.();
			notifySuccess(
				t("settings:security.passwordSet", { defaultValue: "Password set successfully" }),
			);
		},
		onError: (error: unknown) =>
			notifyError(
				t("settings:security.passwordSetError", { defaultValue: "Failed to set password" }),
				stringifyError(error),
			),
	});

	const changePasswordMutation = useMutation({
		mutationFn: () => secretsApi.changePassword(oldPassword, newPassword, confirmPassword),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["password", "status"] });
			handleOpenChange(false);
			onSuccess?.();
			notifySuccess(
				t("settings:security.passwordChanged", { defaultValue: "Password changed successfully" }),
			);
		},
		onError: (error: unknown) =>
			notifyError(
				t("settings:security.passwordChangeError", { defaultValue: "Failed to change password" }),
				stringifyError(error),
			),
	});

	const clearPasswordMutation = useMutation({
		mutationFn: () => secretsApi.clearPassword(oldPassword),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["password", "status"] });
			handleOpenChange(false);
			onSuccess?.();
			notifySuccess(
				t("settings:security.passwordCleared", { defaultValue: "Password removed" }),
			);
		},
		onError: (error: unknown) =>
			notifyError(
				t("settings:security.passwordClearError", { defaultValue: "Failed to remove password" }),
				stringifyError(error),
			),
	});

	const verifyPasswordMutation = useMutation({
		mutationFn: () => secretsApi.verifyPassword(oldPassword),
		onSuccess: async (data) => {
			if (!data.valid) {
				setFormError(
					t("settings:security.passwordIncorrect", { defaultValue: "Password is incorrect." }),
				);
				return;
			}
			handleOpenChange(false);
			onSuccess?.(oldPassword);
		},
		onError: (error: unknown) =>
			notifyError(
				t("settings:security.passwordVerifyError", { defaultValue: "Failed to verify password" }),
				stringifyError(error),
			),
	});

	const isPending =
		setPasswordMutation.isPending ||
		changePasswordMutation.isPending ||
		clearPasswordMutation.isPending ||
		verifyPasswordMutation.isPending;

	const handleSubmit = useCallback(() => {
		if (mode === "set") {
			if (!newPassword.trim()) {
				setFormError(
					t("settings:security.passwordRequired", { defaultValue: "Enter a password to continue." }),
				);
				return;
			}
			if (newPassword !== confirmPassword) {
				setFormError(
					t("settings:security.passphraseMismatch", { defaultValue: "Passwords do not match." }),
				);
				return;
			}
			setFormError(null);
			setPasswordMutation.mutate();
			return;
		}

		if (mode === "change") {
			if (!oldPassword.trim() || !newPassword.trim()) {
				setFormError(
					t("settings:security.passwordChangeRequired", {
						defaultValue: "Enter your current and new passwords.",
					}),
				);
				return;
			}
			if (newPassword !== confirmPassword) {
				setFormError(
					t("settings:security.passphraseMismatch", { defaultValue: "Passwords do not match." }),
				);
				return;
			}
			setFormError(null);
			changePasswordMutation.mutate();
			return;
		}

		if (mode === "verify") {
			if (!oldPassword.trim()) {
				setFormError(
					t("settings:security.passwordRequired", { defaultValue: "Enter a password to continue." }),
				);
				return;
			}
			setFormError(null);
			verifyPasswordMutation.mutate();
			return;
		}

		if (!oldPassword.trim()) {
			setFormError(
				t("settings:security.passwordClearRequired", {
					defaultValue: "Enter your current password to remove protection.",
				}),
			);
			return;
		}
		setFormError(null);
		clearPasswordMutation.mutate();
	}, [
		mode,
		newPassword,
		confirmPassword,
		oldPassword,
		setPasswordMutation,
		changePasswordMutation,
		clearPasswordMutation,
		t,
	]);

	const title =
		mode === "set"
			? t("settings:security.setPasswordTitle", { defaultValue: "Set Login Password" })
			: mode === "change"
				? t("settings:security.changePasswordTitle", { defaultValue: "Change Login Password" })
				: mode === "verify"
					? t("settings:security.verifyPasswordTitle", { defaultValue: "Verify Password" })
					: t("settings:security.removePasswordTitle", { defaultValue: "Remove Login Password" });

	const description =
		mode === "set"
			? t("settings:security.setPasswordDescription", {
				defaultValue: "Add a local privacy screen. This password gates UI access on this machine only — it does not encrypt data at rest or protect against other local users with filesystem access.",
			})
			: mode === "change"
				? t("settings:security.changePasswordDescription", {
					defaultValue: "Update the password used to unlock MCPMate.",
				})
				: mode === "verify"
					? t("settings:security.verifyPasswordDescription", {
						defaultValue: "Enter your current password to confirm this change.",
					})
					: t("settings:security.clearDescription", {
						defaultValue: "Enter your current password to remove protection.",
					});

	const actionLabel =
		mode === "set"
			? t("settings:security.setPassword", { defaultValue: "Set Password" })
			: mode === "change"
				? t("settings:security.changePasswordAction", { defaultValue: "Change Password" })
				: mode === "verify"
					? t("settings:security.verifyAction", { defaultValue: "Verify" })
					: t("settings:security.removePasswordAction", { defaultValue: "Remove Password" });

	return (
		<AlertDialog open={open} onOpenChange={handleOpenChange}>
			<AlertDialogContent
				onOpenAutoFocus={(event) => {
					event.preventDefault();
					firstInputRef.current?.focus();
				}}
			>
				<AlertDialogHeader>
					<AlertDialogTitle>{title}</AlertDialogTitle>
					<AlertDialogDescription>{description}</AlertDialogDescription>
				</AlertDialogHeader>
				<div className="space-y-3">
					{(mode === "change" || mode === "clear" || mode === "verify") && (
						<div className="space-y-2">
							<Label htmlFor={currentPasswordId}>
								{t("settings:security.currentPassword", { defaultValue: "Current Password" })}
							</Label>
							<Input
								ref={firstInputRef}
								id={currentPasswordId}
								type="password"
								value={oldPassword}
								onChange={(e) => {
									setOldPassword(e.target.value);
									setFormError(null);
								}}
								placeholder={t("settings:security.passphrasePlaceholder", {
									defaultValue: "Enter password...",
								})}
								className="h-9"
							/>
						</div>
					)}
					{mode !== "clear" && mode !== "verify" && (
						<>
							<div className="space-y-2">
								<Label htmlFor={newPasswordId}>
									{t("settings:security.newPassword", { defaultValue: "New Password" })}
								</Label>
								<Input
									ref={mode === "set" ? firstInputRef : undefined}
									id={newPasswordId}
									type="password"
									value={newPassword}
									onChange={(e) => {
										setNewPassword(e.target.value);
										setFormError(null);
									}}
									placeholder={t("settings:security.passphrasePlaceholder", {
										defaultValue: "Enter password...",
									})}
									className="h-9"
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor={confirmPasswordId}>
									{t("settings:security.confirmPassword", { defaultValue: "Confirm Password" })}
								</Label>
								<Input
									id={confirmPasswordId}
									type="password"
									value={confirmPassword}
									onChange={(e) => {
										setConfirmPassword(e.target.value);
										setFormError(null);
									}}
									placeholder={t("settings:security.passphraseConfirmPlaceholder", {
										defaultValue: "Re-enter password...",
									})}
									className="h-9"
								/>
							</div>
						</>
					)}
					{formError ? <p className="text-sm text-destructive">{formError}</p> : null}
				</div>
				<AlertDialogFooter>
					<AlertDialogCancel disabled={isPending} onClick={() => onCancel?.()}>
						{t("settings:security.confirmCancel", { defaultValue: "Cancel" })}
					</AlertDialogCancel>
					<AlertDialogAction
						className={buttonVariants({ variant: mode === "clear" ? "destructive" : "default" })}
						onClick={(event) => {
							event.preventDefault();
							handleSubmit();
						}}
						disabled={isPending}
					>
						{isPending
							? t("settings:security.saving", { defaultValue: "Saving..." })
							: actionLabel}
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}
