import { useState, useEffect } from "react";

/**
 * 防抖 hook，延迟更新值
 * @param value 要防抖的值
 * @param delay 延迟时间（毫秒）
 * @returns 防抖后的值
 */
export function useDebouncedValue<T>(value: T, delay = 300): T {
	const [debounced, setDebounced] = useState(value);

	useEffect(() => {
		const handle = window.setTimeout(() => setDebounced(value), delay);
		return () => window.clearTimeout(handle);
	}, [value, delay]);

	return debounced;
}
