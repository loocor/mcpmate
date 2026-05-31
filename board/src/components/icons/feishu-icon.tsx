import type { SVGProps } from "react";
import { cn } from "../../lib/utils";

/** Feishu (Lark) brand mark — three-path bird logo with official palette. */
export function FeishuIcon({ className, ...props }: SVGProps<SVGSVGElement>) {
	return (
		<svg
			viewBox="0 0 152.43 121.72"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			aria-hidden="true"
			className={cn("h-7 w-7", className)}
			{...props}
		>
			<path d="m59.72 78.46c10.91 5.21 22.6 9.68 34.96 12.41 9.16 2.02 18.42.19 26.07-4.59 2.1-1.31 3.48-3.13 6.17-3.19-27.19 39.64-82.54 50.85-123.14 23.88-2.49-1.44-3.78-4.35-3.78-6.13v-65.92c18.29 19.15 37.71 32.95 59.72 43.54z" fill="#3570fa" />
			<path d="m114.54 36.97c-15.74 4.73-23.4 15.72-35.31 26.5-14.16-24.41-33.12-45.28-56.81-63.47h71.87c10.51 10.59 16.27 23.5 20.24 36.97z" fill="#06d4b9" />
			<path d="m126.92 83.09c-2.69.06-4.07 1.88-6.17 3.19-7.65 4.78-16.91 6.62-26.07 4.59-12.36-2.73-24.05-7.21-34.96-12.41 7.37-4.17 13.47-9.52 19.5-14.99 11.91-10.78 19.57-21.77 35.31-26.5 12.29-3.7 25.56-3.01 37.89 2.95-11.65 12.95-15.3 28.29-25.51 43.17z" fill="#143d99" />
		</svg>
	);
}
