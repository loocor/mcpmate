"use client";

import * as React from "react";
import { Drawer as DrawerPrimitive } from "vaul";

import { cn } from "../../lib/utils";

function cleanupBodyLocks() {
	// No-op; rely on Vaul/Radix internals
}

function hasAnyOpenLayer() { return true; }

function ensureBodyInteractive() { /* no-op */ }

function setAppInert(_inert: boolean) { /* no-op */ }

// Create a context to share the close handler with DrawerOverlay
const DrawerContext = React.createContext<{
	onClose?: () => void;
}>({});

const Drawer = ({
	shouldScaleBackground = false,
	direction = "right",
	onOpenChange,
	...props
}: React.ComponentProps<typeof DrawerPrimitive.Root> & {
	direction?: "right" | "left" | "top" | "bottom";
}) => {
	const handleOpenChange = React.useCallback(
		(open: boolean) => {
			try {
				onOpenChange?.(open);
			} catch {
				/* noop */
			}
			if (open) {
				setAppInert(true);
			}
			if (!open) {
				// Defer cleanup to after transition completes
				requestAnimationFrame(() =>
					setTimeout(() => {
						cleanupBodyLocks();
						ensureBodyInteractive();
					}, 80),
				);
			}
		},
		[onOpenChange],
	);

	const handleClose = React.useCallback(() => {
		handleOpenChange(false);
	}, [handleOpenChange]);

    // Removed global guard to avoid interfering with focus/aria
    React.useEffect(() => {}, []);

    // No-op cleanup on unmount
    React.useEffect(() => () => {}, []);

	return (
		<DrawerContext.Provider value={{ onClose: handleClose }}>
			<DrawerPrimitive.Root
				shouldScaleBackground={shouldScaleBackground}
				direction={direction}
				onOpenChange={handleOpenChange}
				dismissible={false}
				modal={true}
				{...props}
			/>
		</DrawerContext.Provider>
	);
};
Drawer.displayName = "Drawer";

const DrawerTrigger = DrawerPrimitive.Trigger;

const DrawerPortal = DrawerPrimitive.Portal;

const DrawerClose = DrawerPrimitive.Close;

const DrawerOverlay = React.forwardRef<
	React.ElementRef<typeof DrawerPrimitive.Overlay>,
	React.ComponentPropsWithoutRef<typeof DrawerPrimitive.Overlay>
>(({ className, onClick, style, ...props }, ref) => {
	const { onClose } = React.useContext(DrawerContext);

	const handleClick = React.useCallback(
		(e: React.MouseEvent<HTMLDivElement>) => {
			// Call original onClick if provided
			onClick?.(e);
			// Close the drawer when clicking the overlay
			if (e.target === e.currentTarget && onClose) {
				onClose();
			}
		},
		[onClick, onClose],
	);

	return (
		<DrawerPrimitive.Overlay
			ref={ref}
			className={cn("fixed inset-0 z-40 bg-black/80", className)}
			style={style}
			onClick={handleClick}
			{...props}
		/>
	);
});
DrawerOverlay.displayName = DrawerPrimitive.Overlay.displayName;

const DrawerContent = React.forwardRef<
	React.ElementRef<typeof DrawerPrimitive.Content>,
	React.ComponentPropsWithoutRef<typeof DrawerPrimitive.Content>
>(({ className, children, ...props }, ref) => {
	return (
		<DrawerPortal>
			<DrawerOverlay />
			<DrawerPrimitive.Content
				ref={ref}
				className={cn(
					"fixed top-0 right-0 bottom-0 z-50 h-screen w-full sm:w-[560px] md:w-[720px] flex flex-col border-l border-slate-200 dark:border-slate-700 bg-background shadow-lg overflow-y-auto",
					className,
				)}
				{...props}
			>
				{children}
			</DrawerPrimitive.Content>
		</DrawerPortal>
	);
});
DrawerContent.displayName = "DrawerContent";

const DrawerHeader = ({
	className,
	...props
}: React.HTMLAttributes<HTMLDivElement>) => (
	<div
		className={cn("grid gap-1.5 p-4 text-center sm:text-left", className)}
		{...props}
	/>
);
DrawerHeader.displayName = "DrawerHeader";

const DrawerFooter = ({
	className,
	...props
}: React.HTMLAttributes<HTMLDivElement>) => (
	<div
		className={cn("mt-auto flex flex-col gap-2 p-4", className)}
		{...props}
	/>
);
DrawerFooter.displayName = "DrawerFooter";

const DrawerTitle = React.forwardRef<
	React.ElementRef<typeof DrawerPrimitive.Title>,
	React.ComponentPropsWithoutRef<typeof DrawerPrimitive.Title>
>(({ className, ...props }, ref) => (
	<DrawerPrimitive.Title
		ref={ref}
		className={cn(
			"text-lg font-semibold leading-none tracking-tight",
			className,
		)}
		{...props}
	/>
));
DrawerTitle.displayName = DrawerPrimitive.Title.displayName;

const DrawerDescription = React.forwardRef<
	React.ElementRef<typeof DrawerPrimitive.Description>,
	React.ComponentPropsWithoutRef<typeof DrawerPrimitive.Description>
>(({ className, ...props }, ref) => (
	<DrawerPrimitive.Description
		ref={ref}
		className={cn("text-sm text-muted-foreground", className)}
		{...props}
	/>
));
DrawerDescription.displayName = DrawerPrimitive.Description.displayName;

export {
	Drawer,
	DrawerPortal,
	DrawerOverlay,
	DrawerTrigger,
	DrawerClose,
	DrawerContent,
	DrawerHeader,
	DrawerFooter,
	DrawerTitle,
	DrawerDescription,
};
