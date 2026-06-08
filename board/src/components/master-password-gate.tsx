import { type ReactNode, useCallback, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { secretsApi } from "../lib/api";
import { requiresStartupPasswordGate } from "../lib/protection-password";
import { LockScreen } from "./lock-screen";

const SESSION_KEY = "mcp_password_verified";

interface MasterPasswordGateProps {
	children: ReactNode;
}

export function MasterPasswordGate({ children }: MasterPasswordGateProps) {
	const [verified, setVerified] = useState(
		() => sessionStorage.getItem(SESSION_KEY) === "true",
	);

	const statusQuery = useQuery({
		queryKey: ["password", "status"],
		queryFn: secretsApi.passwordStatus,
		enabled: !verified,
		staleTime: 30_000,
	});

	const handleUnlock = useCallback(async (_password: string) => {
		sessionStorage.setItem(SESSION_KEY, "true");
		setVerified(true);
	}, []);

	if (verified) {
		return <>{children}</>;
	}

	if (statusQuery.isLoading) {
		return null;
	}

	if (statusQuery.isError) {
		return (
			<div className="flex h-screen items-center justify-center">
				<div className="text-center space-y-2">
					<p className="text-sm text-muted-foreground">
						Unable to verify password status. Please check that the backend is running.
					</p>
					<button
						type="button"
						className="text-sm text-primary underline"
						onClick={() => statusQuery.refetch()}
					>
						Retry
					</button>
				</div>
			</div>
		);
	}

	if (!requiresStartupPasswordGate(statusQuery.data)) {
		return <>{children}</>;
	}

	return <LockScreen variant="login" onSuccess={handleUnlock} />;
}
