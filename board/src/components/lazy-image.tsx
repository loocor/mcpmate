import { useState, useRef, useEffect, useCallback } from "react";
import type React from "react";
import { cn } from "../lib/utils";
import {
	CACHE_DURATION_MS,
	imageCache,
	loadingStates,
} from "./lazy-image-cache";

export interface LazyImageProps {
	src?: string;
	alt?: string;
	fallback?: React.ReactNode;
	className?: string;
	onLoad?: () => void;
	onError?: () => void;
	placeholder?: React.ReactNode;
	timeout?: number; // 加载超时时间（毫秒）
	cacheKey?: string; // 缓存键，用于区分不同的图片
}

export function LazyImage({
	src,
	alt = "",
	fallback,
	className,
	onLoad,
	onError,
	placeholder,
	timeout = 10000,
	cacheKey,
}: LazyImageProps) {
	const [imageSrc, setImageSrc] = useState<string | null>(null);
	const [isLoading, setIsLoading] = useState(false);
	const [hasError, setHasError] = useState(false);
	const [isVisible, setIsVisible] = useState(false);
	const imgRef = useRef<HTMLImageElement>(null);
	const observerRef = useRef<IntersectionObserver | null>(null);

	// 生成缓存键
	const getCacheKey = useCallback(() => {
		return cacheKey || src || "";
	}, [cacheKey, src]);

	// 检查缓存
	const getCachedImage = useCallback((key: string): string | null => {
		const cached = imageCache.get(key);
		if (cached && Date.now() - cached.timestamp < CACHE_DURATION_MS) {
			return cached.data;
		}
		// 清理过期缓存
		if (cached) {
			imageCache.delete(key);
		}
		return null;
	}, []);

	// 设置缓存
	const setCachedImage = useCallback((key: string, data: string) => {
		imageCache.set(key, { data, timestamp: Date.now() });
	}, []);

	// 处理图片加载 - 带缓存检查
	const handleLoadImage = useCallback(async () => {
		if (!src || !isVisible) return;

		// 再次检查缓存（防止竞态条件）
		const key = getCacheKey();
		const cached = getCachedImage(key);
		if (cached) {
			console.log("Image found in cache during load:", src);
			setImageSrc(cached);
			onLoad?.();
			return;
		}

		setIsLoading(true);
		setHasError(false);

		// 检查是否已有相同的加载请求
		if (loadingStates.has(key)) {
			console.log("Image already loading:", src);
			try {
				const result = await loadingStates.get(key);
				if (result) {
					setImageSrc(result);
					onLoad?.();
				}
			} catch (error) {
				console.warn("Failed to get existing load result:", error);
				setHasError(true);
				onError?.();
			} finally {
				setIsLoading(false);
			}
			return;
		}

		// 创建新的加载请求
		const attemptLoad = (useCrossOrigin: boolean) =>
			new Promise<string | null>((resolve, reject) => {
				const img = new Image();
				if (useCrossOrigin) {
					try {
						img.crossOrigin = "anonymous";
					} catch (error) {
						console.warn("Failed to set crossOrigin on image", src, error);
					}
				}

				const timeoutId = setTimeout(() => {
					console.warn("Image load timeout:", src);
					reject(new Error("Image load timeout"));
				}, timeout);

				img.onload = () => {
					clearTimeout(timeoutId);
					console.log("Image loaded successfully:", src);
					setCachedImage(key, src);
					resolve(src);
				};

				img.onerror = (error) => {
					clearTimeout(timeoutId);
					reject(error instanceof Error ? error : new Error("Image load failed"));
				};

				img.src = src;
			});

		const loadPromise = (async () => {
			try {
				return await attemptLoad(true);
			} catch (error) {
				console.warn(
					"Image load failed with crossOrigin=anonymous, retrying without:",
					src,
					error,
				);
				return attemptLoad(false);
			}
		})();

		loadingStates.set(key, loadPromise);

		try {
			const result = await loadPromise;
			if (result) {
				setImageSrc(result);
				onLoad?.();
			}
		} catch (error) {
			console.warn("Failed to load image:", src, error);
			setHasError(true);
			onError?.();
		} finally {
			setIsLoading(false);
			loadingStates.delete(key);
		}
	}, [
		src,
		isVisible,
		timeout,
		onLoad,
		onError,
		getCacheKey,
		getCachedImage,
		setCachedImage,
	]);

	// 设置 Intersection Observer
	useEffect(() => {
		if (!imgRef.current) return;

		observerRef.current = new IntersectionObserver(
			([entry]) => {
				if (entry.isIntersecting) {
					setIsVisible(true);
					observerRef.current?.disconnect();
				}
			},
			{ threshold: 0.1 },
		);

		observerRef.current.observe(imgRef.current);

		return () => {
			observerRef.current?.disconnect();
		};
	}, []);

	// 组件初始化时检查缓存
	useEffect(() => {
		if (src) {
			const key = getCacheKey();
			const cached = getCachedImage(key);
			if (cached) {
				console.log("Image found in cache:", src);
				setImageSrc(cached);
				onLoad?.();
			}
		}
	}, [src, getCacheKey, getCachedImage, onLoad]);

	// 当可见且需要加载时，开始加载图片
	useEffect(() => {
		if (isVisible && src && !imageSrc && !hasError) {
			handleLoadImage();
		}
	}, [isVisible, src, imageSrc, hasError, handleLoadImage]);

	// 清理函数
	useEffect(() => {
		return () => {
			observerRef.current?.disconnect();
		};
	}, []);

	// 如果没有 src 或发生错误，显示 fallback
	if (!src || hasError) {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				{fallback}
			</div>
		);
	}

	// 如果正在加载，显示占位符
	if (isLoading) {
		return (
			<div
				ref={imgRef}
				className={cn(
					"flex items-center justify-center bg-slate-100 dark:bg-slate-800",
					className,
				)}
			>
				{placeholder || (
					<div className="h-4 w-4 animate-spin rounded-full border-2 border-slate-300 border-t-slate-600" />
				)}
			</div>
		);
	}

	// 如果图片已加载，显示图片
	if (imageSrc) {
		return (
			<img
				ref={imgRef}
				src={imageSrc}
				alt={alt}
				className={cn("aspect-square h-full w-full object-contain", className)}
			/>
		);
	}

	// 默认显示占位符
	return (
		<div
			ref={imgRef}
			className={cn(
				"flex items-center justify-center bg-slate-100 dark:bg-slate-800",
				className,
			)}
		>
			{placeholder || (
				<div className="h-4 w-4 animate-spin rounded-full border-2 border-slate-300 border-t-slate-600" />
			)}
		</div>
	);
}
