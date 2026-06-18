import { Card, CardContent, CardHeader } from "../../components/ui/card";
import { cn } from "../../lib/utils";
import { SKELETON_BLOCK_CLASS } from "./market-card";

function MetadataRowSkeleton() {
	return (
		<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2">
			<div className={cn("h-4 w-24 rounded", SKELETON_BLOCK_CLASS)} />
			<div className={cn("h-4 w-full max-w-xs rounded", SKELETON_BLOCK_CLASS)} />
		</div>
	);
}

function ReadmeLineSkeleton({ className }: { className?: string }) {
	return <div className={cn("h-3 w-full rounded", SKELETON_BLOCK_CLASS, className)} />;
}

export function MarketDetailSkeleton() {
	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden" aria-hidden="true">
			<div className={cn("h-9 w-56 shrink-0 rounded-lg", SKELETON_BLOCK_CLASS)} />

			<Card className="shrink-0">
				<CardContent className="relative p-4">
					<div className="mb-3 flex flex-wrap justify-end gap-2 sm:absolute sm:top-4 sm:right-4 sm:z-10 sm:mb-0">
						<div className={cn("h-9 w-24 rounded-md", SKELETON_BLOCK_CLASS)} />
						<div className={cn("h-9 w-28 rounded-md", SKELETON_BLOCK_CLASS)} />
						<div className={cn("h-9 w-20 rounded-md", SKELETON_BLOCK_CLASS)} />
					</div>
					<div className="flex w-full flex-wrap items-start gap-4 sm:pr-56">
						<div className={cn("h-12 w-12 shrink-0 rounded-[10px]", SKELETON_BLOCK_CLASS)} />
						<div className="min-w-0 flex-1 space-y-2">
							{Array.from({ length: 5 }, (_, index) => (
								<MetadataRowSkeleton key={`summary-meta-${index}`} />
							))}
						</div>
					</div>
				</CardContent>
			</Card>

			<div className="grid min-h-0 flex-1 grid-cols-1 gap-6 lg:grid-cols-2 lg:gap-4">
				<Card className="min-h-0">
					<CardHeader>
						<div className={cn("h-6 w-48 rounded-lg", SKELETON_BLOCK_CLASS)} />
					</CardHeader>
					<CardContent className="space-y-4 p-4">
						<div className={cn("h-3 w-36 rounded", SKELETON_BLOCK_CLASS)} />
						<div className="space-y-2">
							{Array.from({ length: 2 }, (_, index) => (
								<MetadataRowSkeleton key={`repo-meta-${index}`} />
							))}
						</div>
						<div className={cn("h-3 w-32 rounded", SKELETON_BLOCK_CLASS)} />
						<div className="space-y-2">
							{Array.from({ length: 4 }, (_, index) => (
								<MetadataRowSkeleton key={`registry-meta-${index}`} />
							))}
						</div>
					</CardContent>
				</Card>

				<Card className="flex min-h-0 flex-col">
					<CardHeader className="shrink-0">
						<div className={cn("h-6 w-24 rounded-lg", SKELETON_BLOCK_CLASS)} />
					</CardHeader>
					<CardContent className="min-h-0 flex-1 space-y-3 p-4">
						<ReadmeLineSkeleton className="max-w-[42%]" />
						<ReadmeLineSkeleton />
						<ReadmeLineSkeleton />
						<ReadmeLineSkeleton className="max-w-[88%]" />
						<ReadmeLineSkeleton />
						<ReadmeLineSkeleton className="max-w-[76%]" />
						<ReadmeLineSkeleton />
						<ReadmeLineSkeleton className="max-w-[64%]" />
					</CardContent>
				</Card>
			</div>
		</div>
	);
}
