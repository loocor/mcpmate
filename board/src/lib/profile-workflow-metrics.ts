export function formatWorkflowToolBindingCount(
	toolBindingCount: number | null | undefined,
	isLoading: boolean,
	isError: boolean,
): string {
	if (isLoading) return "...";
	if (isError || toolBindingCount === null || toolBindingCount === undefined) {
		return "—";
	}
	return `${toolBindingCount}/${toolBindingCount}`;
}
