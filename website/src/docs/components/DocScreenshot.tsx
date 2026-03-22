type DocScreenshotProps = {
	lightSrc: string;
	darkSrc: string;
	alt: string;
	caption?: string;
};

/**
 * Renders paired light/dark screenshots; visibility follows site theme (Tailwind dark:).
 */
export default function DocScreenshot({
	lightSrc,
	darkSrc,
	alt,
	caption,
}: DocScreenshotProps) {
	return (
		<figure className="not-prose my-8">
			<img
				src={lightSrc}
				alt={alt}
				className="w-full rounded-lg border border-slate-200 shadow-sm dark:hidden"
				loading="lazy"
			/>
			<img
				src={darkSrc}
				alt={alt}
				className="hidden w-full rounded-lg border border-slate-700 shadow-sm dark:block"
				loading="lazy"
			/>
			{caption ? (
				<figcaption className="mt-2 text-center text-sm text-slate-600 dark:text-slate-400">
					{caption}
				</figcaption>
			) : null}
		</figure>
	);
}
