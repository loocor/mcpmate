export interface KeyValueLike {
	key?: string | null;
	value?: string | null;
}

export function isBlankKeyValuePair(pair: KeyValueLike): boolean {
	return !pair.key?.trim() && !pair.value?.trim();
}

export function shouldAppendKeyValueRow(
	fields: KeyValueLike[] | undefined,
): boolean {
	if (!fields?.length) return true;
	return !isBlankKeyValuePair(fields[fields.length - 1]);
}

/** Drop fully blank rows left behind by duplicate ghost clicks. */
export function compactKeyValueFields<T extends KeyValueLike>(fields: T[]): T[] {
	return fields.filter((pair) => !isBlankKeyValuePair(pair));
}
