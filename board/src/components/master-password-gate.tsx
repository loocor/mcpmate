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

	if (!requiresStartupPasswordGate(statusQuery.data)) {
		return <>{children}</>;
	}

	return <LockScreen variant="login" onSuccess={handleUnlock} />;
}
