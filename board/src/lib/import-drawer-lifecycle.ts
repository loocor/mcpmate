export function resolveImportDrawerOpen(
	requestedOpen: boolean,
	importPending: boolean,
): boolean {
	return requestedOpen || importPending;
}

export function shouldAcceptImportDrawerChange(
	nextOpen: boolean,
	importPending: boolean,
): boolean {
	return nextOpen || !importPending;
}
