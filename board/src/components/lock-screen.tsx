import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Lock, Unlock } from "lucide-react";
import { secretsApi } from "../lib/api";
import { Button } from "./ui/button";
import { Input } from "./ui/input";

export type LockScreenVariant = "login" | "encryption";

interface LockScreenProps {
	variant?: LockScreenVariant;
	onSuccess: (password: string) => void | Promise<void>;
}

export function LockScreen({ variant = "login", onSuccess }: LockScreenProps) {
	const { t } = useTranslation();
	const [password, setPassword] = useState("");
	const [focused, setFocused] = useState(true);
	const [unlocked, setUnlocked] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [loading, setLoading] = useState(false);
	const isEncryption = variant === "encryption";

	const handleSubmit = useCallback(
		async (e?: React.FormEvent) => {
			e?.preventDefault();
			if (!password || loading) return;

			setLoading(true);
			setError(null);

			try {
				if (isEncryption) {
					await secretsApi.unlock(password);
					setUnlocked(true);
					await onSuccess(password);
				} else {
					const result = await secretsApi.verifyPassword(password);
					if (result.valid) {
						setUnlocked(true);
						await onSuccess(password);
					} else {
						setError(
							t("lock.login.wrongPassword", {
								defaultValue: "Incorrect login password. Please try again.",
							}),
						);
						setPassword("");
					}
				}
			} catch {
				setError(
					isEncryption
						? t("lock.encryption.unlockError", {
							defaultValue:
								"Could not unlock the secure store. Check your encryption password and try again.",
						})
						: t("lock.login.verifyError", {
							defaultValue: "Could not verify login password. Please try again.",
						}),
				);
			} finally {
				setLoading(false);
			}
		},
		[password, loading, onSuccess, t, isEncryption],
	);

	return (
		<div className="flex h-screen w-screen items-center justify-center bg-background">
			<div className="w-full max-w-sm space-y-6 px-6">
				<div className="flex flex-col items-center space-y-2 text-center">
					<div className="relative inline-flex">
						<div className="rounded-full bg-muted p-5">
							<img
								src="/logo.svg"
								alt="MCPMate"
								className="h-12 w-12 object-contain dark:invert dark:brightness-0"
							/>
						</div>
						<span
							className="absolute -bottom-0.5 -right-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-background ring-1 ring-border"
							aria-hidden
						>
							{unlocked ? (
								<Unlock className="h-3 w-3 text-muted-foreground" />
							) : (
								<Lock className="h-3 w-3 text-muted-foreground" />
							)}
						</span>
					</div>
					<h1 className="text-xl font-semibold tracking-tight">
						{t("lock.title", { defaultValue: "MCPMate" })}
					</h1>
					<p className="text-sm text-muted-foreground">
						{isEncryption
							? t("lock.encryption.description", {
								defaultValue:
									"Enter your encryption password to unlock the secure store.",
							})
							: t("lock.login.description", {
								defaultValue: "Enter your login password to continue.",
							})}
					</p>
				</div>

				<form onSubmit={handleSubmit} className="space-y-4">
					<div className="space-y-2">
						<Input
							type="password"
							value={password}
							onChange={(e) => setPassword(e.target.value)}
							aria-label={
								isEncryption
									? t("lock.encryption.passwordPlaceholder", {
										defaultValue: "Encryption password",
									})
									: t("lock.login.passwordPlaceholder", {
										defaultValue: "Login password",
									})
							}
							placeholder={
								focused
									? ""
									: isEncryption
										? t("lock.encryption.passwordPlaceholder", {
											defaultValue: "Encryption password",
										})
										: t("lock.login.passwordPlaceholder", {
											defaultValue: "Login password",
										})
							}
							onFocus={() => setFocused(true)}
							onBlur={() => setFocused(false)}
							autoFocus
							disabled={loading}
							className="h-10 text-center"
						/>
					</div>

					{error && (
						<p className="text-center text-sm text-destructive">{error}</p>
					)}

					<Button
						type="submit"
						className="w-full"
						disabled={!password || loading}
					>
						{loading
							? t("lock.verifying", { defaultValue: "Verifying..." })
							: t("lock.unlock", { defaultValue: "Unlock" })}
					</Button>
				</form>
			</div>
		</div>
	);
}
