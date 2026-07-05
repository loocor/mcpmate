import {
	InspectorConnectServerForm,
	type InspectorConnectCandidate,
	type InspectorConnectedTargetSnapshot,
} from "./inspector-connect-server-form";

type InspectorConnectWorkspaceProps = {
	selectedTargetKey: string | null;
	connectedTargetKey: string | null;
	connectedTargetSnapshot: InspectorConnectedTargetSnapshot | null;
	connected: boolean;
	connecting: boolean;
	onConnect: (candidate: InspectorConnectCandidate) => Promise<void> | void;
	onDisconnect: () => void;
};

export function InspectorConnectWorkspace({
	selectedTargetKey,
	connectedTargetKey,
	connectedTargetSnapshot,
	connected,
	connecting,
	onConnect,
	onDisconnect,
}: InspectorConnectWorkspaceProps) {
	return (
		<InspectorConnectServerForm
			selectedTargetKey={selectedTargetKey}
			connectedTargetKey={connectedTargetKey}
			connectedTargetSnapshot={connectedTargetSnapshot}
			connected={connected}
			connecting={connecting}
			onConnect={onConnect}
			onDisconnect={onDisconnect}
		/>
	);
}
