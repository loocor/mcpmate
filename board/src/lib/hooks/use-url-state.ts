import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router-dom";

type UrlStateValue = string | number | boolean | null | undefined;

interface UseUrlStateOptions<T extends UrlStateValue> {
	paramName: string;
	defaultValue: T;
	validate?: (value: string) => boolean;
	serialize?: (value: T) => string;
	deserialize?: (value: string) => T;
}

type UseUrlStateReturn<T> = [
	value: T,
	setValue: (value: T) => void,
	removeParam: () => void,
];

export function useUrlState<T extends UrlStateValue>(
	options: UseUrlStateOptions<T>,
): UseUrlStateReturn<T> {
	const {
		paramName,
		defaultValue,
		validate,
		serialize,
		deserialize,
	} = options;

	const [searchParams, setSearchParams] = useSearchParams();

	const value = useMemo<T>(() => {
		const paramValue = searchParams.get(paramName);
		if (paramValue === null) {
			return defaultValue;
		}

		if (validate && !validate(paramValue)) {
			return defaultValue;
		}

		if (deserialize) {
			return deserialize(paramValue);
		}

		if (typeof defaultValue === "number") {
			const parsed = Number(paramValue);
			return (Number.isNaN(parsed) ? defaultValue : parsed) as T;
		}

		if (typeof defaultValue === "boolean") {
			return (paramValue === "true") as T;
		}

		return paramValue as T;
	}, [searchParams, paramName, defaultValue, validate, deserialize]);

	const setValue = useCallback(
		(newValue: T) => {
			setSearchParams(
				(prev) => {
					const next = new URLSearchParams(prev);
					if (newValue === null || newValue === undefined || newValue === defaultValue) {
						next.delete(paramName);
					} else {
						const serialized = serialize
							? serialize(newValue)
							: String(newValue);
						next.set(paramName, serialized);
					}
					return next;
				},
				{ replace: true },
			);
		},
		[paramName, defaultValue, serialize, setSearchParams],
	);

	const removeParam = useCallback(() => {
		setSearchParams(
			(prev) => {
				const next = new URLSearchParams(prev);
				next.delete(paramName);
				return next;
			},
			{ replace: true },
		);
	}, [paramName, setSearchParams]);

	return [value, setValue, removeParam];
}

interface UseUrlTabOptions {
	paramName?: string;
	defaultTab?: string;
	validTabs: string[];
}

export function useUrlTab(options: UseUrlTabOptions): {
	activeTab: string;
	setActiveTab: (tab: string) => void;
} {
	const {
		paramName = "tab",
		defaultTab = "overview",
		validTabs,
	} = options;

	const [activeTab, setActiveTab] = useUrlState({
		paramName,
		defaultValue: defaultTab,
		validate: (value) => validTabs.includes(value),
	});

	return { activeTab, setActiveTab };
}

interface UseUrlViewOptions {
	paramName?: string;
	defaultView?: string;
	validViews: string[];
}

export function useUrlView(options: UseUrlViewOptions): {
	view: string;
	setView: (view: string) => void;
} {
	const {
		paramName = "view",
		defaultView = "overview",
		validViews,
	} = options;

	const [view, setView] = useUrlState({
		paramName,
		defaultValue: defaultView,
		validate: (value) => validViews.includes(value),
	});

	return { view, setView };
}

interface UseUrlSearchOptions {
	paramName?: string;
	defaultValue?: string;
	debounceMs?: number;
}

export function useUrlSearch(options: UseUrlSearchOptions = {}): {
	search: string;
	setSearch: (search: string) => void;
} {
	const { paramName = "q", defaultValue = "" } = options;

	const [search, setSearch] = useUrlState({
		paramName,
		defaultValue,
	});

	return { search, setSearch };
}

type SortDirection = "asc" | "desc";

interface SortState {
	field: string;
	direction: SortDirection;
}

interface UseUrlSortOptions {
	paramName?: string;
	defaultField?: string;
	defaultDirection?: SortDirection;
	validFields?: string[];
}

export function useUrlSort(options: UseUrlSortOptions = {}): {
	sortState: SortState;
	setSortState: (state: SortState) => void;
	setSortField: (field: string) => void;
	toggleSortDirection: () => void;
} {
	const {
		paramName = "sort",
		defaultField = "name",
		defaultDirection = "asc",
		validFields,
	} = options;

	const [sortParam, setSortParam] = useUrlState({
		paramName,
		defaultValue: `${defaultField}:${defaultDirection}`,
		validate: (value) => {
			const parts = value.split(":");
			if (parts.length !== 2) return false;
			const [field, direction] = parts;
			if (direction !== "asc" && direction !== "desc") return false;
			if (validFields && !validFields.includes(field)) return false;
			return true;
		},
	});

	const sortState = useMemo<SortState>(() => {
		const parts = sortParam.split(":");
		const field = parts[0] || defaultField;
		const direction = (parts[1] as SortDirection) || defaultDirection;
		return { field, direction };
	}, [sortParam, defaultField, defaultDirection]);

	const setSortState = useCallback(
		(state: SortState) => {
			setSortParam(`${state.field}:${state.direction}`);
		},
		[setSortParam],
	);

	const setSortField = useCallback(
		(field: string) => {
			setSortState({ field, direction: sortState.direction });
		},
		[sortState.direction, setSortState],
	);

	const toggleSortDirection = useCallback(() => {
		setSortState({
			field: sortState.field,
			direction: sortState.direction === "asc" ? "desc" : "asc",
		});
	}, [sortState, setSortState]);

	return { sortState, setSortState, setSortField, toggleSortDirection };
}

interface UseUrlFilterOptions {
	paramName?: string;
	defaultValue?: string;
	validValues?: string[];
}

export function useUrlFilter(options: UseUrlFilterOptions = {}): {
	filter: string;
	setFilter: (filter: string) => void;
} {
	const { paramName = "filter", defaultValue = "all", validValues } = options;

	const [filter, setFilter] = useUrlState({
		paramName,
		defaultValue,
		validate: validValues ? (value) => validValues.includes(value) : undefined,
	});

	return { filter, setFilter };
}

export type { SortState, SortDirection };
