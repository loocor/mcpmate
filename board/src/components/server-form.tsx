import { zodResolver } from "@hookform/resolvers/zod";
import { AlertCircle, Loader2 } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import * as z from "zod";
import type { MCPServerConfig } from "../lib/types";
import { Alert, AlertDescription, AlertTitle } from "./ui/alert";
import { Button } from "./ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Textarea } from "./ui/textarea";

// Form validation schema
const serverFormSchema = z.object({
	name: z
		.string()
		.min(1, "Server name cannot be empty")
		.max(50, "Server name cannot exceed 50 characters"),
	kind: z.enum(["stdio", "sse", "streamable_http"], {
		required_error: "Please select a server type",
	}),
	command: z.string().optional(),
	command_path: z.string().optional(),
	args: z.string().optional(),
	env: z.string().optional(),
	max_instances: z.coerce.number().int().positive().optional(),
});

type ServerFormValues = z.infer<typeof serverFormSchema>;

interface ServerFormProps {
	isOpen: boolean;
	onClose: () => void;
	onSubmit: (data: Partial<MCPServerConfig>) => Promise<void>;
	initialData?: Partial<MCPServerConfig>;
	title?: string;
	submitLabel?: string;
}

export function ServerForm({
	isOpen,
	onClose,
	onSubmit,
	initialData,
	title = "Add Server",
	submitLabel = "Save",
}: ServerFormProps) {
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [error, setError] = useState<string | null>(null);

	// Convert initial data to form values
	const defaultValues: Partial<ServerFormValues> = {
		name: initialData?.name || "",
		kind: initialData?.kind || "stdio",
		command: initialData?.command || "",
		command_path: initialData?.command_path || "",
		args: initialData?.args ? initialData.args.join(" ") : "",
		env: initialData?.env
			? Object.entries(initialData.env)
					.map(([key, value]) => `${key}=${value}`)
					.join("\n")
			: "",
		max_instances: initialData?.max_instances || 1,
	};

	const {
		register,
		handleSubmit,
		formState: { errors },
		reset,
		setValue,
		watch,
	} = useForm<ServerFormValues>({
		resolver: zodResolver(serverFormSchema),
		defaultValues,
	});

	// Monitor server type changes
	const serverType = watch("kind");

	// Handle form submission
	const handleFormSubmit = async (data: ServerFormValues) => {
		setIsSubmitting(true);
		setError(null);

		try {
			// Convert form data to server configuration
			const serverConfig: Partial<MCPServerConfig> = {
				name: data.name,
				kind: data.kind,
				command: data.command || undefined,
				command_path: data.command_path || undefined,
				args: data.args ? data.args.split(" ").filter(Boolean) : undefined,
				env: data.env
					? data.env.split("\n").reduce(
							(acc, line) => {
								const [key, ...valueParts] = line.split("=");
								if (key && valueParts.length > 0) {
									acc[key.trim()] = valueParts.join("=").trim();
								}
								return acc;
							},
							{} as Record<string, string>,
						)
					: undefined,
				max_instances: data.max_instances || undefined,
			};

			await onSubmit(serverConfig);
			reset();
			onClose();
		} catch (err) {
			setError(
				err instanceof Error
					? err.message
					: "Error saving server configuration",
			);
		} finally {
			setIsSubmitting(false);
		}
	};

	return (
		<Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
			<DialogContent className="sm:max-w-[500px] bg-slate-50">
				<DialogHeader>
					<DialogTitle>{title}</DialogTitle>
					<DialogDescription>
						Configure server connection information. Different types of servers
						require different configuration parameters.
					</DialogDescription>
				</DialogHeader>

				<form
					onSubmit={handleSubmit(handleFormSubmit)}
					className="space-y-4 py-4"
				>
					{error && (
						<Alert variant="destructive">
							<AlertCircle className="h-4 w-4" />
							<AlertTitle>Error</AlertTitle>
							<AlertDescription>{error}</AlertDescription>
						</Alert>
					)}

					<div className="grid grid-cols-2 gap-4">
						<div className="space-y-2">
							<Label htmlFor="name">Server Name</Label>
							<Input
								id="name"
								{...register("name")}
								placeholder="e.g., my-server"
							/>
							{errors.name && (
								<p className="text-xs text-red-500">{errors.name.message}</p>
							)}
						</div>

						<div className="space-y-2">
							<Label htmlFor="kind">Server Type</Label>
							<Select
								defaultValue={defaultValues.kind}
								onValueChange={(value) =>
									setValue("kind", value as ServerFormValues["kind"])
								}
							>
								<SelectTrigger>
									<SelectValue placeholder="Select server type" />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="stdio">
										Standard Input/Output (stdio)
									</SelectItem>
									<SelectItem value="sse">Server-Sent Events (SSE)</SelectItem>
									<SelectItem value="streamable_http">
										HTTP Stream (Streamable HTTP)
									</SelectItem>
								</SelectContent>
							</Select>
							{errors.kind && (
								<p className="text-xs text-red-500">{errors.kind.message}</p>
							)}
						</div>
					</div>

					{serverType === "stdio" && (
						<>
							<div className="space-y-2">
								<Label htmlFor="command">Command</Label>
								<Input
									id="command"
									{...register("command")}
									placeholder="e.g., python -m my_script"
								/>
							</div>

							<div className="space-y-2">
								<Label htmlFor="command_path">Command Path</Label>
								<Input
									id="command_path"
									{...register("command_path")}
									placeholder="e.g., /usr/local/bin"
								/>
							</div>

							<div className="space-y-2">
								<Label htmlFor="args">Arguments (space separated)</Label>
								<Input
									id="args"
									{...register("args")}
									placeholder="e.g., --debug --port 8080"
								/>
							</div>
						</>
					)}

					{(serverType === "sse" || serverType === "streamable_http") && (
						<div className="space-y-2">
							<Label htmlFor="command">URL</Label>
							<Input
								id="command"
								{...register("command")}
								placeholder="e.g., http://localhost:8080"
							/>
						</div>
					)}

					<div className="space-y-2">
						<Label htmlFor="env">
							Environment Variables (one per line, KEY=VALUE format)
						</Label>
						<Textarea
							id="env"
							{...register("env")}
							placeholder="e.g.,&#10;PORT=8080&#10;DEBUG=true"
							rows={4}
						/>
					</div>

					<div className="space-y-2">
						<Label htmlFor="max_instances">Maximum Instances</Label>
						<Input
							id="max_instances"
							type="number"
							min="1"
							{...register("max_instances")}
						/>
					</div>

					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							onClick={onClose}
							disabled={isSubmitting}
						>
							Cancel
						</Button>
						<Button type="submit" disabled={isSubmitting}>
							{isSubmitting && (
								<Loader2 className="mr-2 h-4 w-4 animate-spin" />
							)}
							{submitLabel}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}
