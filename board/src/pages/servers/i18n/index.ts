export const serversTranslations = {
    en: {
            title: "Connect and monitor MCP servers",
		toolbar: {
			search: {
				placeholder: "Search servers...",
				fields: {
					name: "Name",
					description: "Description",
				},
			},
			sort: {
				options: {
					name: "Name",
					enabled: "Enable Status",
				},
			},
		},
		actions: {
			debug: {
				title: "Inspect",
				show: "Inspect",
				hide: "Hide Inspect",
				open: "Open inspect view",
			},
			refresh: {
				title: "Refresh",
			},
			add: {
				title: "Add Server",
			},
		},
		emptyState: {
			title: "No servers found",
			description: "Add your first MCP server to get started",
			action: "Add First Server",
		},
		notifications: {
			importUnsupported: {
				title: "Unsupported content",
				message:
					"Drop text, JSON snippets, URLs, or config files to use Uni-Import.",
			},
			importRejections: {
				bundleDisabled: "MCPB and DXT bundle import is currently disabled.",
				fileTooLarge: "Dropped file exceeds the {{maxMb}} MB import limit.",
				textTooLarge: "Dropped text exceeds the {{maxMb}} MB import limit.",
				tooManyFiles: "Drop up to {{maxFiles}} files at a time.",
			},
			importEmpty: {
				title: "Nothing to import",
				message:
					"We could not detect any usable configuration from the dropped content.",
			},
			deepLinkImport: {
				title: "Configuration received",
				message:
					"Review the imported server snippet in the drawer before saving.",
			},
			toggle: {
				enabledTitle: "Server enabled",
				disabledTitle: "Server disabled",
				message: "Server {{serverId}}",
				enabledDetail: "Server {{serverId}} has been enabled",
				disabledDetail: "Server {{serverId}} has been disabled",
				enableAction: "enable",
				disableAction: "disable",
				error: "Unable to {{action}} server: {{message}}",
				failedTitle: "Failed to toggle server",
			},
			update: {
				title: "Server updated",
				message: "Server {{serverId}}",
				errorTitle: "Update failed",
				errorMessage: "Unable to update {{serverId}}: {{message}}",
			},
			delete: {
				title: "Server deleted",
        message:
          "Server {{serverId}}. Review Secure Store cleanup if it used stored secrets.",
        cleanupReview:
          "Review Secure Store cleanup if this server used stored secrets.",
				errorFallback: "Error deleting server",
			},
			genericError: {
				title: "Operation failed",
				unknown: "Unknown error",
			},
		},
		statsCards: {
			total: {
				title: "Total Servers",
				description: "registered",
			},
			enabled: {
				title: "Enabled",
				description: "feature toggled",
			},
			connected: {
				title: "Connected",
				description: "active connections",
			},
			instances: {
				title: "Instances",
				description: "total across servers",
			},
		},
		errors: {
			loadFailed: "Failed to load servers",
		},
		debug: {
			cardTitle: "Inspect Details",
			close: "Close",
			info: {
				baseUrl: "API Base URL",
				currentTime: "Current Time",
				error: "Error",
				data: "Servers Data",
			},
		},
		entity: {
			tags: {
				unifyEligible: "Direct Exposure",
			},
			description: {
				serverLabel: "Server: {{name}}",
			},
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				streamableHttp: "Streamable HTTP",
				headerAuth: "Header auth",
				oauth: "OAuth",
				oauthWarning: "Authorization expired — reauthorize required",
			},
			iconAlt: {
				named: "{{name}} icon",
				fallback: "Server icon",
			},
			stats: {
				tools: "Tools",
				prompts: "Prompts",
				resources: "Resources",
				templates: "Templates",
			},
		},
			capabilityList: {
				searchPlaceholder: "Search {{label}}...",
				emptyFallback: "No data.",
				detailsToggle: "Details",
				inputSchemaTitle: "Input Schema",
				outputSchemaTitle: "Output Schema",
				table: {
					argument: "Argument",
					required: "Required",
					requiredYes: "Yes",
					requiredNo: "No",
					description: "Description",
					property: "Property",
					type: "Type",
					details: "Details",
					enum: "enum:",
					items: "items:",
					itemsEnum: "items.enum:",
				},
			},
		detail: {
			errors: {
				noServerId: "No server ID provided",
			},
			loading: {
				title: "Loading server details",
        description:
          "The service is responding, but its detail snapshot is still warming up.",
			},
			viewModes: {
				browse: "Browse",
				debug: "Inspect",
			},
			overview: {
				labels: {
					service: "Service",
					upstreamName: "Upstream name",
					namespace: "Namespace",
					runtime: "Runtime",
					type: "Type",
					auth: "Auth",
					protocol: "Protocol",
					version: "Version",
					capabilities: "Capabilities",
					description: "Description",
					defaultHeaders: "Default Headers",
					category: "Category",
					scenario: "Scenario",
					command: "Command",
					repository: "Repository",
				},
				status: {
					enabled: "Enabled",
					disabled: "Disabled",
					unifyEligible: "Direct Exposure Eligible",
				},
			},
			namespaceIssue: {
				title: "Namespace remediation required",
				statusConflict: "Conflict",
				statusInvalid: "Invalid namespace",
				popoverCapabilityCollision:
					"This namespace would produce capability names already owned by another server. Set a unique namespace manually.",
				popoverCapabilityCollisionWithServer:
					"This namespace would produce capability names already owned by the existing <server>{{namespace}}</server> server. Set a unique namespace manually.",
				popoverLegacyCollision:
					"Normalizing this namespace would duplicate an existing canonical server namespace. Set a unique namespace manually.",
				popoverLegacyCollisionWithServer:
					"Normalizing this namespace would conflict with the canonical namespace of the existing <server>{{namespace}}</server> server. Set a unique namespace manually.",
				popoverInvalid:
					"This namespace cannot be normalized safely. Set a unique lowercase snake_case namespace manually.",
				retryConflict:
					"This namespace still fails uniqueness validation. Choose another namespace and save again.",
				description:
					'The legacy namespace “{{namespace}}” cannot be exposed safely. Enter a unique canonical namespace to repair its references.',
				capabilityCollision:
					'The namespace “{{namespace}}” produces a capability name already used by another server. Enter a unique canonical namespace to repair the server without renaming individual capabilities.',
				conflicts: "Conflicting namespace: {{namespaces}}",
				inputLabel: "Replacement namespace",
				action: "Repair",
				success: "Namespace repaired",
				pendingTitle: "Namespace repaired; refresh pending",
				pendingDescription:
					"The namespace change was committed, but runtime or cache refresh has not converged yet. Refresh the server details before retrying downstream calls.",
				errorTitle: "Namespace repair failed",
				iconLabel: "Namespace requires remediation",
			},
			deleteDialog: {
				title: "Delete Server",
				description: "This action cannot be undone.",
				cancel: "Cancel",
				confirm: "Delete",
				pending: "Deleting...",
			},
			actions: {
				refresh: "Refresh",
				edit: "Edit",
				disable: "Disable",
				enable: "Enable",
				delete: "Delete",
			},
			instances: {
				title: "Instances ({{count}})",
				empty: "No instances.",
			},
			tabs: {
				overview: "Overview",
				tools: "Tools ({{count}})",
				prompts: "Prompts ({{count}})",
				resources: "Resources ({{count}})",
				templates: "Resource Templates ({{count}})",
				capabilities: "Capabilities ({{count}})",
				logs: "Logs",
			},
			filters: {
				kind: {
					all: "All Types",
					selected: "{{count}} Types",
				},
				status: {
					all: "All",
					enabled: "Enabled",
					disabled: "Disabled",
				},
			},
			logs: {
				title: "Logs",
				description: "Runtime and activity logs related to this server.",
				searchPlaceholder: "Search logs...",
				refresh: "Refresh Logs",
				expand: "Expand Logs",
				collapse: "Collapse Logs",
				loading: "Loading logs...",
				headers: {
					timestamp: "Timestamp",
					action: "Action",
					category: "Category",
					status: "Status",
					target: "Target",
				},
				empty: "No log entries recorded for this server yet.",
			},
			capabilityList: {
				labels: {
					tools: "Tools",
					resources: "Resources",
					prompts: "Prompts",
					templates: "Resource Templates",
				},
				title: "{{label}} ({{count}})",
				empty: "No {{label}} from this server",
				emptyAll: "No capabilities from this server",
			},
			debug: {
				proxyUnavailable:
					"Proxy mode unavailable: server not enabled in any active profile.",
			},
			inspector: {
				channel: {
					proxy: "Proxy",
					native: "Native",
					fallback: "Fallback to native until proxy is available",
					hintTitle: "Proxy unavailable",
					hintDescription:
						"Enable this server in an active profile to exercise proxy mode.",
					openProfiles: "Open Profiles",
				},
				labels: {
					tools: "Tools",
					resources: "Resources",
					prompts: "Prompts",
					templates: "Resource Templates",
				},
				tabs: {
					results: "Results ({{count}})",
					logs: "Logs ({{count}})",
				},
				filterPlaceholder: "Filter {{label}}...",
				actions: {
					refresh: "Refresh",
					list: "List",
					inspect: "Inspect",
				},
				logs: {
					search: "Search logs...",
					clear: "Clear Logs",
					empty: "No inspector events yet.",
					status: {
						mode: {
							proxy: "PROXY",
							native: "NATIVE",
						},
						event: {
							request: "REQUEST",
							success: "SUCCESS",
							error: "ERROR",
							progress: "PROGRESS",
							log: "LOG",
							cancelled: "CANCELLED",
						},
					},
				},
				results: {
					lastFetched: "Last listed at {{time}}",
					emptyFetched: "No {{label}} returned.",
					emptyPrompt: "Run {{label}} list to fetch live data.",
				},
				oauth: {
					title: "OAuth Authorization",
					description:
						"Configure and connect OAuth for this hosted MCP server before using it in the main workflow.",
					loading: "Refreshing OAuth status…",
					state: {
						notConfigured: "Not configured",
						disconnected: "Disconnected",
						connected: "Connected",
						expired: "Expired",
					},
					fields: {
						authorizationEndpoint: "Authorization endpoint",
						tokenEndpoint: "Token endpoint",
						clientId: "Client ID",
						clientSecret: "Client secret",
						clientSecretPlaceholderExisting:
							"Leave blank to keep the stored secret",
						clientSecretPlaceholderNew: "Optional for public clients",
						scopes: "Scopes",
						redirectUri: "Redirect URI",
					},
					actions: {
						save: "Save settings",
						connect: "Connect with OAuth",
						reconnect: "Reconnect OAuth",
						revoke: "Revoke token",
					},
					summary: {
						configured: "Configured",
						clientSecret: "Client secret",
						stored: "Stored",
						notSet: "Not set",
						expiresAt: "Expires at",
						notAvailable: "Not available",
					},
					manualOverride: {
						title: "Manual Authorization header is active",
						description:
							"This server already has an Authorization header in its transport settings, so that manual value will take precedence over the stored OAuth token.",
					},
					notifications: {
						savedTitle: "OAuth settings saved",
            savedMessage: "The server OAuth configuration has been updated.",
						saveFailedTitle: "Failed to save OAuth settings",
						connectFailedTitle: "Unable to start OAuth",
						revokedTitle: "OAuth token revoked",
						revokedMessage:
							"Stored OAuth credentials were removed for this server.",
						revokeFailedTitle: "Failed to revoke OAuth",
					},
				},
			},
		},
		oauth: {
			callback: {
				title: "OAuth Authorization",
				processing: "Completing authorization, please wait...",
				success:
					"Authorization successful. Returning to the server detail page...",
				error: "Authorization failed.",
				back: "Back to servers",
			},
			errors: {
				missingParams: "Missing required OAuth parameters.",
				callbackFailed: "OAuth callback processing failed.",
			},
		},
		instanceDetail: {
			errors: {
				missingParams: "Server ID or instance ID not provided",
				notFound: "Instance not found or error loading instance details.",
			},
			sections: {
				details: {
					title: "Instance Details",
					fields: {
						instanceId: "Instance ID:",
						server: "Server:",
						status: "Status:",
						connectionAttempts: "Connection Attempts:",
						connectedFor: "Connected For:",
						tools: "Tools:",
						processId: "Process ID:",
						health: "Health:",
					},
				},
				controls: {
					title: "Instance Controls",
					description: "Manage the instance connection state",
					actions: {
						cancelInitialization: "Cancel Initialization",
						disconnect: "Disconnect Instance",
						forceDisconnect: "Force Disconnect",
						reconnect: "Reconnect Instance",
						resetReconnect: "Reset & Reconnect",
					},
					hints: {
						reconnectDelay: "Reconnection may take a few moments to complete",
					},
				},
				metrics: {
					title: "Instance Metrics",
					description: "Performance metrics and statistics for this instance",
					fields: {
						cpuUsage: "CPU Usage:",
						memoryUsage: "Memory Usage:",
						connectionStability: "Connection Stability:",
					},
				},
			},
			notices: {
				healthIssue: "Health issue detected:",
				statusNote: "Status note:",
				error: "Error:",
			},
			healthMessages: {
				idlePlaceholder: "Instance is idle (placeholder, not connected)",
			},
		},
		manual: {
			ingest: {
				default:
					"Drop or paste JSON, TOML, or text, or click the icon to scan local configs",
				parsingDropped: "Parsing dropped text",
				parsingPasted: "Parsing pasted content",
				success: "Server configuration loaded successfully",
				noneDetectedError: "No servers detected in the input",
				noneDetectedTitle: "No servers detected",
				noneDetectedDescription:
					"We could not find any server definitions in the input.",
				parseFailedFallback: "Failed to parse input",
				parseFailedTitle: "Parsing failed",
				editing: "Editing server",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "Tip: press",
				tipSuffix: "to paste instantly.",
			},
			refreshFromRegistry: "Refresh from Registry",
			refresh: {
				success: "Metadata refreshed from registry",
				error: "Failed to refresh metadata",
				notFound: "Server not found in registry",
			},
			viewMode: {
				form: "Form",
				json: "JSON",
			},
			tabs: {
				core: "Core configuration",
					unify: "Direct Exposure",
				meta: "Meta information",
				metaWip: "WIP",
			},
			header: {
				title: {
					edit: "Editing server",
					import: "Import Server",
					create: "Server Uni-Import",
				},
				description: {
					edit: "Review and update the existing server settings. JSON preview remains read-only in this mode.",
					import: "Configure and import this server from the registry.",
					create:
						"You can directly drag and drop the configuration information, or enter it manually.",
				},
			},
			buttons: {
				reset: "Reset form",
				resetAria: "Reset form",
				cancel: "Cancel",
				preview: "Preview",
				previewing: "Previewing...",
				save: "Save changes",
				import: "Import server",
				saving: "Saving...",
				importing: "Importing...",
				processing: "Processing...",
			},
			bulk: {
				title: "{{count}} servers detected",
				description:
					"Select the servers to preview and import. {{count}} selected.",
				selectAll: "Select all",
				clearSelection: "Clear",
				enable: "Enable",
				disable: "Disable",
				bulkModeEnter: "Bulk select",
				bulkModeExit: "Exit bulk select",
				bulkModeDescription: "{{count}} selected for bulk actions",
				includeInImport: "Add to import",
				excludeFromImport: "Remove from import",
				selectServer: "Select {{name}}",
				includeForImport: "Include {{name}} in import",
				missingEndpoint: "Missing command or URL",
				backToList: "Back to detected servers",
				noSelectionTitle: "No servers selected",
				noSelectionDescription:
					"Select at least one server to continue with preview and import.",
			},
			scan: {
				action: "Scan local configs",
				actionHint: "Click to scan local configs",
				noneTitle: "No local configs found",
				noneDescription:
					"No detected MCP clients have a local configuration file to scan.",
				noServersTitle: "No servers detected",
				noServersDescription:
					"The local scan did not find importable MCP server entries.",
				failedTitle: "Local scan failed",
			},
			fields: {
				name: {
					label: "Name",
					placeholder: "e.g., local-mcp",
					readOnlyTitle: "Editing server names is disabled",
					readOnlyTitleAfterOAuth:
						"Editing server names is disabled after OAuth setup starts",
				},
				namespace: {
					label: "Namespace",
					placeholder: "e.g., local_mcp",
          suggestionAction: "Use suggested namespace:",
					importMapping: "{{original}} → {{namespace}}",
				},
				type: {
					label: "Type",
					options: {
						stdio: "Stdio",
						streamable_http: "Streamable HTTP",
						sse: "SSE (Legacy)",
					},
				},
				command: {
					label: "Command",
					placeholder: "e.g., uvx my-mcp",
				},
				url: {
					label: "Server URL",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "Arguments",
					ghost: "Add a new argument",
					placeholder: "Argument {{count}}",
				},
				env: {
					label: "Environment Variables",
					ghostKey: "Add a new key",
					keyPlaceholder: "KEY",
				},
				headers: {
					label: "HTTP Headers",
					ghostKey: "Add a new header",
					keyPlaceholder: "Header",
				},
				urlParams: {
					label: "URL Parameters",
					ghostKey: "Parameter name",
					ghostValue: "Value",
					keyPlaceholder: "Parameter",
				},
				common: {
					addRow: "Add row",
					ghostValue: "Add a new value",
					valuePlaceholder: "Value",
					removeRow: "Remove row",
					confirmRemoveRow: "Confirm remove",
				},
				meta: {
					iconAlt: "Server icon",
					version: {
						label: "Version",
						placeholder: "e.g., 1.0.0",
					},
					website: {
						label: "Website",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "Repository URL",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "Repository Source",
							placeholder: "e.g., github",
						},
						subfolder: {
							label: "Repository Subfolder",
							placeholder: "Optional subfolder",
						},
						id: {
							label: "Repository Entry ID (Metadata)",
							placeholder: "Optional metadata identifier",
						},
					},
					description: {
						label: "Description",
						placeholder: "Short description",
					},
				},
				json: {
					label: "Server JSON",
				},
				unifyEligibility: {
					badge: "Advanced exposure control",
					title: "Mark as Unify-eligible server",
          description:
            "This option marks the server as eligible for direct exposure in Unify mode. Eligible servers can expose tools, prompts, resources, and templates directly to selected clients.",
					whatIsIt: "What is it?",
          whatIsItDesc:
            "Enable this only when the server should be available for direct capability exposure in Unify instead of being reached only through the UCAN broker workflow.",
					whenToUse: "When to use it?",
          whenToUseDesc:
            "Use this for servers that should allow direct exposure of key capabilities to selected Unify clients, such as memory, audit, or always-on context services.",
					watchOut: "What to watch out for?",
          watchOutDesc:
            "Do not enable this casually. Once a client selects direct exposure, capabilities from this server can bypass the UCAN-only path and enter the direct client context.",
					howToEnable: "How to enable it",
          howToEnableDesc:
            "First mark the server as eligible here. Then open a Client in Unify mode and choose Server Level (all capabilities) or Capability Level (selected tools/prompts/resources/templates).",
          toggleHint:
            "This only marks eligibility. Clients still decide whether and how to expose it.",
				},
			},
			auth: {
				label: "AUTH",
				transportHint:
					"URL Parameters and HTTP Headers are optional transport extras. They still apply after OAuth if this server needs them.",
				mode: {
					header: "Header-based",
					oauth: "OAuth",
				},
				oauth: {
					connectedTitle: "OAuth connected",
					connectedMessage: "Successfully authorized.",
					listenerNotReady:
						"OAuth callback listener is still initializing. Please try again in a moment.",
					connectFailedTitle: "Unable to start OAuth",
					revokedTitle: "OAuth token revoked",
          revokedMessage:
            "Stored OAuth credentials were removed for this server.",
					revokeFailedTitle: "Failed to revoke OAuth",
					unknownError: "Unknown error",
					state: {
						connected: "Connected",
						expired: "Expired",
						disconnected: "Disconnected",
						notConfigured: "Not configured",
					},
					statusLabel: "OAuth status",
					secureStoreStored: "OAuth credentials are stored in Secure Store",
					secureStoreUnavailable: {
						title: "Secure Store needs attention",
						message:
							"Secure Store is not ready. Unlock or initialize it before connecting OAuth.",
					},
					legacyReconnect: {
						title: "Reconnect OAuth to secure credentials",
						message:
							"Reconnect OAuth to move existing credentials into Secure Store custody.",
					},
					manualOverride: {
						title: "Manual Authorization header is active",
						description:
							"This server already has an Authorization header in its transport settings, which will take precedence over the stored OAuth token.",
					},
					progress: {
						discover: "Metadata discovery",
						register: "Client registration",
						authorize: "Authorization",
						complete: "Authentication complete",
						preparingMessage:
							"Preparing OAuth flow and opening the authorization page…",
            listenerPreparingMessage:
              "Preparing the desktop callback listener…",
						awaitingMessage:
							"Waiting for authorization to complete in the popup window…",
						connectedMessage: "OAuth is connected for this server.",
						errorMessage:
							"OAuth needs attention. Try reconnecting to refresh the authorization.",
					},
					actions: {
						reconnect: "Reconnect OAuth",
						connect: "Connect with OAuth",
						revoke: "Revoke token",
						configure: "Configure",
					},
					fields: {
						authorizationEndpoint: "Authorization endpoint",
						tokenEndpoint: "Token endpoint",
						clientId: "Client ID",
						clientSecret: "Client secret",
            clientSecretPlaceholderExisting:
              "Leave blank to keep the stored secret",
						clientSecretPlaceholderNew: "Optional for public clients",
						scopes: "Scopes",
						redirectUri: "Redirect URI",
            placeholderAuthorizationEndpoint:
              "https://issuer.example.com/authorize",
						placeholderTokenEndpoint: "https://issuer.example.com/token",
						placeholderScopes: "read write",
					},
					loading: "Refreshing OAuth status…",
				},
			},
			errors: {
				nameRequired: "Name is required",
				namespaceRequired: "Namespace is required",
				namespaceInvalid:
					"Use 1–64 lowercase letters, digits, and single underscores; start with a letter.",
				namespaceReviewTitle: "Review server namespaces",
				namespaceReviewDescription:
					"{{name}} does not have a canonical namespace. Apply the suggestion or edit it before previewing.",
				kindRequired: "Select a server type",
				urlInvalid: "Provide a valid URL",
				commandRequired: "Command is required for stdio servers",
				urlRequired: "URL is required for non-stdio servers",
				commandRequiredTitle: "Command required",
				commandRequiredBody: "Provide a command for stdio servers.",
				endpointRequiredTitle: "Endpoint required",
				endpointRequiredBody: "Provide a URL for non-stdio servers.",
				jsonNoServers: "No servers found in JSON payload",
				jsonMultipleServers:
					"Manual entry accepts exactly one server in JSON mode",
				jsonParseFailedTitle: "Invalid JSON",
				jsonParseFailedFallback: "Failed to parse JSON",
				invalidJsonTitle: "Invalid JSON",
				oauthDraftServerFailed: "Failed to create OAuth draft server",
				oauthServerIdRequired: "Server ID is required to initiate OAuth",
			},
			secrets: {
				pick: "Use secret",
				title: "Use Secure Store",
				description: "Insert a write-only placeholder into this runtime field.",
				unavailablePick: "Secure Store unavailable",
				unavailableTitle: "Secure Store unavailable",
				unavailableDescription:
					"Secret placeholders cannot be selected until secure storage is restored.",
				unavailableListTitle: "Secret access needs attention",
				unavailableListDescription:
					"Open Security settings to restore Secure Store access.",
				openSecuritySettings: "Security settings",
				search: "Search secrets...",
				loading: "Loading secrets",
				empty: "No secrets stored",
				createInline: "New secret",
				tagAlias: "{{alias}}",
				inspect: "Open {{alias}} in Secure Store",
				inlinePrefix: "Text before secret",
				inlineTrailing: "Text after secret",
				inlineText: "Secret value text",
				clear: "Clear secret",
				storedSecret: "Stored secret",
			},
		},
		wizard: {
			steps: {
				form: { label: "Configuration", hint: "Setup" },
				preview: { label: "Preview", hint: "Review" },
				result: { label: "Import & Profile", hint: "Complete" },
			},
			header: {
				addTitle_one: "Add MCP Server",
				addTitle_other: "Add MCP Servers",
				addDescription_one: "Configure and install a new MCP server",
				addDescription_other:
					"Review and install {{count}} detected MCP servers",
				editTitle: "Edit Server",
				editDescription: "Update server configuration",
			},
			preview: {
				retry: "Retry preview",
				retryDescription:
					"Regenerate capability preview for the selected server.",
				generating: "Generating capability preview…",
				serverPickerLabel: "Server",
				selectServer: "Select server",
				filterCapabilities: "Filter capabilities...",
				capabilitiesTitle: "Capabilities",
				capabilitiesSummary: "Capabilities · {{summary}}",
        emptyCapabilities: "No capabilities discovered for this server.",
				capabilities: {
					tool: "tool",
					tools: "tools",
					resource: "resource",
					resources: "resources",
					template: "template",
					templates: "templates",
					prompt: "prompt",
					prompts: "prompts",
				},
			},
			buttons: {
				back: "Back",
				cancel: "Cancel",
				preview: "Preview",
				previewing: "Previewing...",
				next: "Next",
				import: "Import",
				importing: "Importing...",
				validating: "Validating...",
				done: "Done",
				reset: "Reset form",
				resetDescription:
					"Clear all fields and restore the initial configuration.",
			},
			result: {
				validation: {
					ready: "Ready to import",
					skipped: "Will be skipped",
					failed: "Failed validation",
				},
				readyTitle: "Ready to Import",
				readyDescription:
					"Click the Import button to proceed with installation",
				validating: "Validating import...",
				validationFailedTitle: "Import Validation Failed",
				validationFailedDescription:
					"Resolve the blocking issues below and run validation again.",
				validatedTitle: "Import Validated",
				validatedDescription:
					"Pre-validation succeeded. The server can be safely imported.",
				pendingImportReadyTitle: "Ready to publish",
				pendingImportReadyDescription:
					"OAuth authorization is complete. Import will publish this server and make it visible in your Servers list.",
				validatedWithWarningsTitle: "Import Validated With Warnings",
				validatedWithWarningsDescription:
					"Pre-validation succeeded, but some servers will be skipped.",
				alreadyInstalledTitle: "Already Installed",
				alreadyInstalledDescription:
					"Every selected server already exists. You can use it immediately—no import required.",
				skipSummary: {
					baseSingle: "Skipped {{count}} server",
					baseMultiple: "Skipped {{count}} servers",
					withDetail: "{{base}}: {{detail}}",
					suffixAlreadyInstalled: "Already installed—no new import required.",
				},
				skipNotice: {
					titlePartial: "{{count}} servers will be skipped",
					descriptionAlreadyInstalled:
						"These servers are already in your library and won't be imported again.",
					descriptionDuplicateName:
						"These servers use names that are already taken and won't be imported again.",
					descriptionMixed:
						"These servers won't be imported. Review the reason for each item below.",
				},
				summary: {
					badgeReady: "Ready",
					badgeSkipped: "Skipped",
					badgeFailed: "Failed",
					titleMixed: "Import Review",
					titleReadyFailed: "Import Partially Ready",
					descriptionReadySkipped:
						"{{ready}} will import and {{skipped}} are already installed and will be skipped.",
					descriptionReadyFailed:
						"{{ready}} will import. Resolve validation failures for the remaining {{failed}} before importing.",
					descriptionSkippedFailed:
						"{{skipped}} are already installed and {{failed}} failed validation.",
					descriptionReadySkippedFailed:
						"{{ready}} will import, {{skipped}} will be skipped, and {{failed}} failed validation.",
				},
				readyStatusTitle: "Import Ready",
				readyStatusDescription:
					"The server configuration is ready to be imported. Review the information below and click Import when ready.",
				importingStatus: "Importing servers…",
				successTitle: "Import Successful",
				successAllSkipped:
					"All selected servers were already installed. No changes were applied.",
				successInstalled:
					"The server has been successfully installed and is ready to use.",
				successAutoEnabled: 'Enabled automatically in "{{profile}}".',
				failureTitle: "Import Failed",
				failureGeneric: "An error occurred during import",
				stats: {
					imported: "Imported",
					skipped: "Skipped",
				},
				installedServersTitle: "Installed Servers",
				success: {
					close:
						"Close this drawer to continue browsing or queue another server for import.",
					servers:
						"Open the Servers dashboard to review and manage the new server.",
					profiles:
						"Visit Profiles to add this server to the appropriate activation sets.",
					profilesWithName:
						'Open Profiles to verify "{{profile}}" reflects the new server.',
				},
				failure: {
					adjustServers:
						"Return to the Servers dashboard to adjust or remove the configuration before retrying.",
					reviewPreview:
						"Review the preview output above for errors and apply the necessary fixes before confirming again.",
					rerunPreview:
						"Keep this drawer open, update the configuration, and rerun Preview before another import attempt.",
				},
				nextSteps: {
					title: "Next steps",
				},
				skipSteps: {
					useExisting: "Close this drawer and start using the existing server.",
					chooseAnother:
						"Go back to the previous step to choose a different server if needed.",
				},
				failedSummary:
					"Import validation failed for {{servers}}. Resolve the issues before importing.",
				failedSummaryFallback_one: "the selected server",
				failedSummaryFallback_other: "the selected servers",
				validationErrorGeneric: "Failed to validate import",
				readySteps: {
					reviewConfig:
						"Review the server configuration and capabilities from the previous step.",
					autoAdd:
						"The server will be automatically added to the Default profile based on your settings.",
					manualAssign:
						"The server will remain unassigned. You can add it to profiles later from the Profiles page.",
					importAction:
						"Click the Import button below to install the server to your system.",
				},
			},
		},
		confirmDelete: {
			title: "Delete Server",
			description:
				'Are you sure you want to delete the server "{{serverId}}"? This action cannot be undone.',
			confirm: "Delete",
			cancel: "Cancel",
		},
	},
    "zh-CN": {
            title: "接入与监控 MCP 服务器运行状态",
		toolbar: {
			search: {
				placeholder: "搜索服务器...",
				fields: {
					name: "名称",
					description: "描述",
				},
			},
			sort: {
				options: {
					name: "名称",
					enabled: "启用状态",
				},
			},
		},
		actions: {
			debug: {
				title: "检视",
				show: "检视",
				hide: "隐藏检视",
				open: "打开检视视图",
			},
			refresh: {
				title: "刷新",
			},
			add: {
				title: "添加服务器",
			},
		},
		emptyState: {
			title: "没有找到服务器",
			description: "添加你的首个 MCP 服务器以开始使用",
			action: "添加首个服务器",
		},
		notifications: {
			importUnsupported: {
				title: "不支持的内容",
				message: "请拖放文本、JSON 片段、URL 或配置文件以使用 Uni-Import。",
			},
			importRejections: {
				bundleDisabled: "MCPB 与 DXT 安装包导入目前已暂停。",
				fileTooLarge: "拖入的文件超过 {{maxMb}} MB 导入限制。",
				textTooLarge: "拖入的文本超过 {{maxMb}} MB 导入限制。",
				tooManyFiles: "一次最多拖入 {{maxFiles}} 个文件。",
			},
			importEmpty: {
				title: "没有可导入的内容",
				message: "无法从拖放的内容中检测到可用的配置。",
			},
			deepLinkImport: {
				title: "已收到配置",
				message: "请在抽屉中核对导入的服务器配置后再保存。",
			},
			toggle: {
				enabledTitle: "服务器已启用",
				disabledTitle: "服务器已禁用",
				message: "服务器 {{serverId}}",
				enabledDetail: "服务器 {{serverId}} 已启用",
				disabledDetail: "服务器 {{serverId}} 已禁用",
				enableAction: "启用",
				disableAction: "禁用",
				error: "无法{{action}}服务器：{{message}}",
				failedTitle: "切换服务器失败",
			},
			update: {
				title: "服务器已更新",
				message: "服务器 {{serverId}}",
				errorTitle: "更新失败",
				errorMessage: "无法更新 {{serverId}}：{{message}}",
			},
			delete: {
				title: "服务器已删除",
        message:
          "服务器 {{serverId}}。如果它使用过已存储密钥，请复核安全存储清理。",
				cleanupReview: "如果此服务器使用过已存储密钥，请复核安全存储清理。",
				errorFallback: "删除服务器时出错",
			},
			genericError: {
				title: "操作失败",
				unknown: "未知错误",
			},
		},
		statsCards: {
			total: {
				title: "服务器总数",
				description: "已登记",
			},
			enabled: {
				title: "已启用",
				description: "功能开关",
			},
			connected: {
				title: "已连接",
				description: "活动连接",
			},
			instances: {
				title: "实例",
				description: "服务器实例总数",
			},
		},
		errors: {
			loadFailed: "加载服务器失败",
		},
		debug: {
			cardTitle: "检视详情",
			close: "关闭",
			info: {
				baseUrl: "API 基础地址",
				currentTime: "当前时间",
				error: "错误",
				data: "服务器数据",
			},
		},
		entity: {
			tags: {
				unifyEligible: "直达暴露",
			},
			description: {
				serverLabel: "服务器：{{name}}",
			},
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				streamableHttp: "Streamable HTTP",
				headerAuth: "请求头鉴权",
				oauth: "OAuth",
				oauthWarning: "授权已过期，请重新授权",
			},
			iconAlt: {
				named: "{{name}} 图标",
				fallback: "服务器图标",
			},
			stats: {
				tools: "工具",
				prompts: "提示",
				resources: "资源",
				templates: "模板",
			},
		},
			capabilityList: {
				searchPlaceholder: "搜索{{label}}...",
				emptyFallback: "暂无数据",
				detailsToggle: "详情",
				inputSchemaTitle: "输入模式",
				outputSchemaTitle: "输出模式",
				table: {
					argument: "参数",
					required: "必填",
					requiredYes: "是",
					requiredNo: "否",
					description: "说明",
					property: "字段",
					type: "类型",
					details: "细节",
					enum: "枚举:",
					items: "子项:",
					itemsEnum: "子项枚举:",
				},
			},
		detail: {
			errors: {
				noServerId: "未提供服务器 ID。",
			},
			loading: {
				title: "正在加载服务详情",
				description: "服务已开始响应，但详情快照仍在准备中。",
			},
			viewModes: {
				browse: "浏览",
				debug: "检视",
			},
			overview: {
				labels: {
					service: "服务",
					upstreamName: "上游名称",
					namespace: "命名空间",
					runtime: "运行时",
					type: "类型",
					auth: "鉴权",
					protocol: "协议",
					version: "版本",
					capabilities: "能力",
					description: "描述",
					defaultHeaders: "默认 Header",
					category: "分类",
					scenario: "场景",
					command: "启动指令",
					repository: "仓库",
				},
				status: {
					enabled: "已启用",
					disabled: "已禁用",
					unifyEligible: "直达暴露可用",
				},
			},
			namespaceIssue: {
				title: "需要修复命名空间",
				statusConflict: "冲突",
				statusInvalid: "命名空间不合规",
				popoverCapabilityCollision:
					"此命名空间会生成已被其他 Server 占用的能力名称，请手工设置唯一的 namespace。",
				popoverCapabilityCollisionWithServer:
					"此命名空间会生成已被现有 Server <server>{{namespace}}</server> 占用的能力名称。请手工设置唯一的 namespace。",
				popoverLegacyCollision:
					"此命名空间在尝试规范化后，会与已存在且合规的 Server 重名。请手工设置唯一的 namespace。",
				popoverLegacyCollisionWithServer:
					"规范化此命名空间后，将与现有 Server <server>{{namespace}}</server> 的合规命名空间冲突。请手工设置唯一的 namespace。",
				popoverInvalid:
					"此命名空间无法安全规范化，请手工设置唯一的小写 snake_case 命名空间。",
				retryConflict:
					"新的 namespace 仍无法通过唯一性验证，请继续修改后再次保存。",
				description:
					"历史命名空间“{{namespace}}”无法安全暴露。请输入一个唯一且合规的命名空间，系统会自动修复相关引用。",
				capabilityCollision:
					"命名空间“{{namespace}}”生成的能力名称已被其他服务器使用。请输入一个唯一且合规的命名空间，系统会在不修改单项能力名称的前提下完成修复。",
				conflicts: "发生冲突的命名空间：{{namespaces}}",
				inputLabel: "替换后的命名空间",
				action: "修复",
				success: "命名空间已修复",
				pendingTitle: "命名空间已修复，刷新仍在进行",
				pendingDescription:
					"命名空间变更已经提交，但运行时或缓存尚未完成收敛。请刷新 Server 详情后再重试下游调用。",
				errorTitle: "命名空间修复失败",
				iconLabel: "命名空间需要修复",
			},
			deleteDialog: {
				title: "删除服务器",
				description: "此操作无法撤销。",
				cancel: "取消",
				confirm: "删除",
				pending: "正在删除...",
			},
			actions: {
				refresh: "刷新",
				edit: "编辑",
				disable: "禁用",
				enable: "启用",
				delete: "删除",
			},
			instances: {
				title: "实例 ({{count}})",
				empty: "暂无实例。",
			},
			tabs: {
				overview: "概览",
				tools: "工具 ({{count}})",
				prompts: "提示 ({{count}})",
				resources: "资源 ({{count}})",
				templates: "模板 ({{count}})",
				capabilities: "能力 ({{count}})",
				logs: "日志",
			},
			filters: {
				kind: {
					all: "全部类型",
					selected: "{{count}} 种类型",
				},
				status: {
					all: "全部",
					enabled: "已启用",
					disabled: "已禁用",
				},
			},
			logs: {
				title: "日志",
				description: "展示与该服务器相关的运行日志与活动记录。",
				searchPlaceholder: "搜索日志...",
				refresh: "刷新日志",
				expand: "展开日志",
				collapse: "收起日志",
				loading: "正在加载日志...",
				headers: {
					timestamp: "时间",
					action: "动作",
					category: "类别",
					status: "状态",
					target: "目标",
				},
				empty: "暂未记录该服务器相关日志。",
			},
			capabilityList: {
				labels: {
					tools: "工具",
					resources: "资源",
					prompts: "提示",
					templates: "模板",
				},
				title: "{{label}} ({{count}})",
				empty: "该服务器无 {{label}}",
				emptyAll: "该服务器暂无能力",
			},
			debug: {
				proxyUnavailable: "代理模式不可用：该服务器未在任何激活的配置中启用。",
			},
			inspector: {
				channel: {
					proxy: "代理",
					native: "本地",
					fallback: "代理不可用时自动使用本地模式",
					hintTitle: "代理不可用",
					hintDescription: "请在某个启用的配置中启用该服务器以使用代理模式。",
					openProfiles: "打开配置",
				},
				labels: {
					tools: "工具",
					resources: "资源",
					prompts: "提示",
					templates: "模板",
				},
				tabs: {
					results: "结果 ({{count}})",
					logs: "日志 ({{count}})",
				},
				filterPlaceholder: "筛选 {{label}}...",
				actions: {
					refresh: "刷新",
					list: "列出",
					inspect: "检视",
				},
				logs: {
					search: "搜索日志...",
					clear: "清空日志",
					empty: "暂无检测事件。",
					status: {
						mode: {
							proxy: "代理",
							native: "本地",
						},
						event: {
							request: "请求",
							success: "成功",
							error: "错误",
							progress: "进度",
							log: "日志",
							cancelled: "已取消",
						},
					},
				},
				results: {
					lastFetched: "上次列出时间 {{time}}",
					emptyFetched: "未返回任何 {{label}}。",
					emptyPrompt: "运行 {{label}} 列表以获取最新数据。",
				},
				oauth: {
					title: "OAuth 授权",
					description:
						"请先为这个托管 MCP 服务器配置并连接 OAuth，再进入主业务流程使用。",
					loading: "正在刷新 OAuth 状态…",
					state: {
						notConfigured: "未配置",
						disconnected: "未连接",
						connected: "已连接",
						expired: "已过期",
					},
					fields: {
						authorizationEndpoint: "授权端点",
						tokenEndpoint: "令牌端点",
						clientId: "Client ID",
						clientSecret: "Client Secret",
            clientSecretPlaceholderExisting: "留空以保留当前已存储的密钥",
						clientSecretPlaceholderNew: "公开客户端可选填",
						scopes: "Scopes",
						redirectUri: "回调 URI",
					},
					actions: {
						save: "保存设置",
						connect: "使用 OAuth 连接",
						reconnect: "重新连接 OAuth",
						revoke: "撤销令牌",
					},
					summary: {
						configured: "已配置",
						clientSecret: "Client Secret",
						stored: "已存储",
						notSet: "未设置",
						expiresAt: "过期时间",
						notAvailable: "暂无",
					},
					manualOverride: {
						title: "当前启用了手动 Authorization Header",
						description:
							"这个服务器的传输设置里已经配置了 Authorization Header，因此手动值会优先于已存储的 OAuth 令牌生效。",
					},
					notifications: {
						savedTitle: "OAuth 设置已保存",
						savedMessage: "该服务器的 OAuth 配置已更新。",
						saveFailedTitle: "保存 OAuth 设置失败",
						connectFailedTitle: "无法启动 OAuth",
						revokedTitle: "OAuth 令牌已撤销",
						revokedMessage: "该服务器保存的 OAuth 凭据已移除。",
						revokeFailedTitle: "撤销 OAuth 失败",
					},
				},
			},
		},
		oauth: {
			callback: {
				title: "OAuth 授权",
				processing: "正在完成授权，请稍候...",
				success: "授权成功，正在返回服务器详情页...",
				error: "授权失败。",
				back: "返回服务器列表",
			},
			errors: {
				missingParams: "缺少必需的 OAuth 参数。",
				callbackFailed: "OAuth 回调处理失败。",
			},
		},
		instanceDetail: {
			errors: {
				missingParams: "未提供服务器 ID 或实例 ID。",
				notFound: "未找到实例或加载实例详情时出错。",
			},
			sections: {
				details: {
					title: "实例详情",
					fields: {
						instanceId: "实例 ID",
						server: "服务器",
						status: "状态",
						connectionAttempts: "连接尝试次数",
						connectedFor: "已连接时长",
						tools: "工具",
						processId: "进程 ID",
						health: "健康状态",
					},
				},
				controls: {
					title: "实例控制",
					description: "管理实例的连接状态",
					actions: {
						cancelInitialization: "取消初始化",
						disconnect: "断开实例",
						forceDisconnect: "强制断开",
						reconnect: "重新连接实例",
						resetReconnect: "重置并重新连接",
					},
					hints: {
						reconnectDelay: "重新连接可能需要一些时间完成",
					},
				},
				metrics: {
					title: "实例指标",
					description: "查看该实例的性能指标与统计数据",
					fields: {
						cpuUsage: "CPU 使用率",
						memoryUsage: "内存使用",
						connectionStability: "连接稳定性",
					},
				},
			},
			notices: {
				healthIssue: "检测到健康状况问题",
				statusNote: "状态提示",
				error: "错误",
			},
			healthMessages: {
				idlePlaceholder: "实例处于空闲状态（占位，未连接）",
			},
		},
		manual: {
			ingest: {
        default: "拖放或粘贴 JSON、TOML 或文本，或点击图标扫描本地配置",
				parsingDropped: "正在解析拖入的文本",
				parsingPasted: "正在解析粘贴的内容",
				success: "服务器配置已成功载入",
				noneDetectedError: "输入中未检测到服务器",
				noneDetectedTitle: "未检测到服务器",
				noneDetectedDescription: "无法在输入内容中找到任何服务器定义。",
				parseFailedFallback: "解析输入失败",
				parseFailedTitle: "解析失败",
				editing: "编辑服务器",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "提示：按下",
				tipSuffix: "即可快速粘贴。",
			},
			refreshFromRegistry: "从注册表刷新",
			refresh: {
				success: "已从注册表刷新元数据",
				error: "刷新元数据失败",
				notFound: "在注册表中未找到该服务器",
			},
			viewMode: {
				form: "表单",
				json: "JSON",
			},
			tabs: {
				core: "核心配置",
					unify: "直达暴露",
				meta: "元信息",
				metaWip: "预览",
			},
			header: {
				title: {
					edit: "编辑服务器",
					import: "导入服务器",
					create: "服务器统一导入",
				},
				description: {
					edit: "检查并更新服务器设置。此模式下 JSON 预览仅可读。",
					import: "配置并从仓库导入此服务器。",
					create: "可以直接拖拽配置，也可以手动录入。",
				},
			},
			buttons: {
				reset: "重置表单",
				resetAria: "重置表单",
				cancel: "取消",
				preview: "预览",
				previewing: "正在预览...",
				save: "保存",
				import: "导入服务器",
				saving: "正在保存...",
				importing: "正在导入...",
				processing: "正在处理...",
			},
			bulk: {
				title: "检测到 {{count}} 个服务器",
				description: "选择要预览和导入的服务器。已选 {{count}} 个。",
				selectAll: "全选",
				clearSelection: "清空",
				enable: "启用",
				disable: "禁用",
				bulkModeEnter: "批量选择",
				bulkModeExit: "退出批量选择",
				bulkModeDescription: "已选 {{count}} 项用于批量操作",
				includeInImport: "纳入导入",
				excludeFromImport: "移出导入",
				selectServer: "选择 {{name}}",
				includeForImport: "将 {{name}} 纳入导入",
				missingEndpoint: "缺少命令或 URL",
				backToList: "返回检测到的服务器",
				noSelectionTitle: "未选择服务器",
				noSelectionDescription: "请至少选择一个服务器后再继续预览和导入。",
			},
			scan: {
				action: "扫描本地配置",
				actionHint: "点击扫描本地配置",
				noneTitle: "未找到本地配置",
				noneDescription: "未检测到带有可扫描本地配置文件的 MCP 客户端。",
				noServersTitle: "未检测到服务器",
				noServersDescription: "本地扫描未发现可导入的 MCP 服务器条目。",
				failedTitle: "本地扫描失败",
			},
			fields: {
				name: {
					label: "名称",
					placeholder: "例如：local-mcp",
					readOnlyTitle: "编辑模式下不可修改名称",
					readOnlyTitleAfterOAuth: "OAuth 流程开始后不可修改名称",
				},
				namespace: {
					label: "命名空间",
					placeholder: "例如：local_mcp",
          suggestionAction: "采用建议的命名空间：",
					importMapping: "{{original}} → {{namespace}}",
				},
				type: {
					label: "类型",
					options: {
						stdio: "Stdio",
						streamable_http: "Streamable HTTP",
						sse: "SSE（旧版）",
					},
				},
				command: {
					label: "启动命令",
					placeholder: "例如：uvx my-mcp",
				},
				url: {
					label: "服务器地址",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "命令参数",
					ghost: "添加参数",
					placeholder: "参数 {{count}}",
				},
				env: {
					label: "环境变量",
					ghostKey: "添加键名",
					keyPlaceholder: "键名",
				},
				headers: {
					label: "HTTP 头",
					ghostKey: "添加 Header",
					keyPlaceholder: "Header",
				},
				urlParams: {
					label: "URL 参数",
					ghostKey: "参数名",
					ghostValue: "参数值",
					keyPlaceholder: "参数",
				},
				common: {
					addRow: "添加行",
					ghostValue: "添加值",
					valuePlaceholder: "值",
					removeRow: "删除行",
					confirmRemoveRow: "确认删除",
				},
				meta: {
					iconAlt: "服务器图标",
					version: {
						label: "版本",
						placeholder: "例如：1.0.0",
					},
					website: {
						label: "网站",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "仓库地址",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "仓库来源",
							placeholder: "例如：github",
						},
						subfolder: {
							label: "仓库子目录",
							placeholder: "可选子目录",
						},
						id: {
							label: "仓库条目 ID（元数据）",
							placeholder: "可选元数据标识",
						},
					},
					description: {
						label: "描述",
						placeholder: "简短说明",
					},
				},
				json: {
					label: "服务器 JSON",
				},
				unifyEligibility: {
					badge: "高级暴露控制",
					title: "标记为直达模式服务器",
          description:
            "这个选项会将服务器标记为 Unify 模式下可直达暴露的候选。被选中的客户端可直接暴露该服务器的工具、提示、资源与模板。",
					whatIsIt: "这是什么？",
          whatIsItDesc:
            "仅当该服务器需要在 Unify 中支持直达能力暴露，而非仅通过 UCAN 代理路径访问时，才应启用。",
					whenToUse: "什么时候使用？",
          whenToUseDesc:
            "适用于 Memory、Audit 等需要向指定 Unify 客户端持续直达暴露关键能力的服务器。",
					watchOut: "需要注意什么？",
          watchOutDesc:
            "不要把它当普通开关随手开启。一旦客户端启用直达暴露，该服务器能力可绕过纯 UCAN 路径，直接进入客户端上下文。",
					howToEnable: "如何启用",
          howToEnableDesc:
            "先在这里标记 eligible，再到 Unify 模式的 Client 中选择 Server Level（全部能力）或 Capability Level（按工具/提示/资源/模板选择）来决定暴露范围。",
          toggleHint:
            "这里仅标记资格，真正是否暴露、如何暴露，由 Client 侧决定。",
				},
			},
			auth: {
				label: "鉴权",
				transportHint:
					"URL 参数与 HTTP 头是可选的传输附加项。若服务器仍需要，在使用 OAuth 后它们仍会生效。",
				mode: {
					header: "基于请求头",
					oauth: "OAuth",
				},
				oauth: {
					connectedTitle: "OAuth 已连接",
					connectedMessage: "授权成功。",
					listenerNotReady: "OAuth 回调监听仍在初始化，请稍后再试。",
					connectFailedTitle: "无法启动 OAuth",
					revokedTitle: "OAuth 令牌已撤销",
					revokedMessage: "已移除该服务器保存的 OAuth 凭据。",
					revokeFailedTitle: "撤销 OAuth 失败",
					unknownError: "未知错误",
					state: {
						connected: "已连接",
						expired: "已过期",
						disconnected: "未连接",
						notConfigured: "未配置",
					},
					statusLabel: "OAuth 状态",
					secureStoreStored: "OAuth 凭据已存入安全存储",
					secureStoreUnavailable: {
						title: "安全存储需要处理",
						message: "安全存储尚未就绪。请先解锁或初始化后再连接 OAuth。",
					},
					legacyReconnect: {
						title: "重新连接 OAuth 以安全保存凭据",
						message: "请重新连接 OAuth，将现有凭据迁移到安全存储托管。",
					},
					manualOverride: {
						title: "手动 Authorization 请求头已生效",
						description:
							"该服务器在传输设置中已配置 Authorization 请求头，将优先于已存储的 OAuth 令牌。",
					},
					progress: {
						discover: "元数据发现",
						register: "客户端注册",
						authorize: "授权",
						complete: "认证完成",
						preparingMessage: "正在准备 OAuth 流程并打开授权页面…",
						listenerPreparingMessage: "正在准备桌面端回调监听…",
						awaitingMessage: "请在弹出窗口中完成授权…",
						connectedMessage: "该服务器的 OAuth 已连接。",
						errorMessage: "OAuth 需要处理，请尝试重新连接以刷新授权。",
					},
					actions: {
						reconnect: "重新连接 OAuth",
						connect: "使用 OAuth 连接",
						revoke: "撤销令牌",
						configure: "高级配置",
					},
					fields: {
						authorizationEndpoint: "授权端点",
						tokenEndpoint: "令牌端点",
						clientId: "客户端 ID",
						clientSecret: "客户端密钥",
						clientSecretPlaceholderExisting: "留空以保留已存储的密钥",
						clientSecretPlaceholderNew: "公开客户端可留空",
						scopes: "权限范围（Scopes）",
						redirectUri: "重定向 URI",
            placeholderAuthorizationEndpoint:
              "https://issuer.example.com/authorize",
						placeholderTokenEndpoint: "https://issuer.example.com/token",
						placeholderScopes: "read write",
					},
					loading: "正在刷新 OAuth 状态…",
				},
			},
			errors: {
				nameRequired: "名称为必填项",
				namespaceRequired: "命名空间为必填项",
				namespaceInvalid:
					"请使用 1–64 位小写字母、数字和单个下划线，并以字母开头。",
				namespaceReviewTitle: "请检查服务器命名空间",
				namespaceReviewDescription:
					"{{name}} 不是规范的命名空间，请采用建议或修改后再进行预览。",
				kindRequired: "请选择服务器类型",
				urlInvalid: "请输入合法的 URL",
				commandRequired: "Stdio 服务器需要提供启动命令",
				urlRequired: "非 Stdio 服务器需要提供 URL",
				commandRequiredTitle: "缺少命令",
				commandRequiredBody: "请为 Stdio 服务器提供启动命令。",
				endpointRequiredTitle: "缺少端点",
				endpointRequiredBody: "请为非 Stdio 服务器提供 URL。",
				jsonNoServers: "JSON 中未找到服务器定义",
				jsonMultipleServers: "手动录入模式仅支持单个服务器",
				jsonParseFailedTitle: "JSON 无法解析",
				jsonParseFailedFallback: "解析 JSON 失败",
				invalidJsonTitle: "JSON 无效",
				oauthDraftServerFailed: "创建 OAuth 草稿服务器失败",
				oauthServerIdRequired: "启动 OAuth 需要服务器 ID",
			},
			secrets: {
				pick: "使用密钥",
				title: "使用安全存储",
				description: "在此运行时字段中插入一个只写占位符。",
				unavailablePick: "安全存储不可用",
				unavailableTitle: "安全存储不可用",
				unavailableDescription: "恢复安全存储后才能选择密钥占位符。",
				unavailableListTitle: "密钥访问需要处理",
				unavailableListDescription: "进入 Security 设置恢复安全存储访问。",
				openSecuritySettings: "Security 设置",
				search: "搜索密钥...",
				loading: "加载密钥中",
				empty: "暂无存储密钥",
				createInline: "新建密钥",
				tagAlias: "{{alias}}",
				inspect: "在安全存储中打开 {{alias}}",
				inlinePrefix: "密钥前的文本",
				inlineTrailing: "密钥后的文本",
				inlineText: "密钥字段文本",
				clear: "清除密钥",
				storedSecret: "已存储密钥",
			},
		},
		wizard: {
			steps: {
				form: { label: "配置", hint: "设置" },
				preview: { label: "预览", hint: "复核" },
				result: { label: "导入", hint: "完成" },
			},
			header: {
				addTitle_one: "新增 MCP 服务器",
				addTitle_other: "新增 MCP 服务器",
				addDescription_one: "配置并安装新的 MCP 服务器",
				addDescription_other: "查看并安装检测到的 {{count}} 个 MCP 服务器",
				editTitle: "编辑服务器",
				editDescription: "更新 MCP 服务器配置",
			},
			preview: {
				retry: "重新预览",
				retryDescription: "为当前选中的服务器重新生成能力预览。",
				generating: "正在生成能力预览…",
				serverPickerLabel: "服务器",
				selectServer: "选择服务器",
				filterCapabilities: "筛选能力...",
				capabilitiesTitle: "能力",
				capabilitiesSummary: "能力 · {{summary}}",
				emptyCapabilities: "未发现该服务器的能力。",
				capabilities: {
					tool: "工具",
					tools: "工具",
					resource: "资源",
					resources: "资源",
					template: "模板",
					templates: "模板",
					prompt: "提示",
					prompts: "提示",
				},
			},
			buttons: {
				back: "返回",
				cancel: "取消",
				preview: "预览",
				previewing: "正在预览...",
				next: "下一步",
				import: "导入",
				importing: "正在导入...",
				validating: "正在校验...",
				done: "完成",
				reset: "重置表单",
				resetDescription: "清空所有字段并恢复初始配置。",
			},
			result: {
				validation: {
					ready: "可被导入",
					skipped: "将被跳过",
					failed: "校验失败",
				},
				readyTitle: "准备导入",
				readyDescription: "点击“导入”按钮继续安装流程",
				validating: "正在校验导入...",
				validationFailedTitle: "导入校验失败",
				validationFailedDescription: "请解决下方列出的阻塞项后重新运行校验。",
				validatedTitle: "导入校验通过",
				validatedDescription: "预校验成功，可以安全执行导入。",
				pendingImportReadyTitle: "可以发布",
				pendingImportReadyDescription:
					"OAuth 授权已完成。导入将发布该服务器并使其出现在服务器列表中。",
				validatedWithWarningsTitle: "导入校验通过（含提示）",
				validatedWithWarningsDescription: "预校验成功，但部分服务器会被跳过。",
				alreadyInstalledTitle: "无需导入",
				alreadyInstalledDescription:
					"所选服务器已经存在，可直接使用，无需再次导入。",
				skipSummary: {
					baseSingle: "已跳过 {{count}} 个服务器",
					baseMultiple: "已跳过 {{count}} 个服务器",
					withDetail: "{{base}}：{{detail}}",
					suffixAlreadyInstalled: "已存在可直接使用，无需重新导入。",
				},
				skipNotice: {
					titlePartial: "将跳过 {{count}} 个服务器",
          descriptionAlreadyInstalled: "这些服务器已在库中，不会再次导入。",
          descriptionDuplicateName: "这些服务器名称已被占用，不会再次导入。",
					descriptionMixed: "以下服务器不会导入，请查看各项原因。",
				},
				summary: {
					badgeReady: "可导入",
					badgeSkipped: "跳过",
					badgeFailed: "失败",
					titleMixed: "导入预览",
					titleReadyFailed: "部分可导入",
					descriptionReadySkipped:
						"{{ready}} 个将导入，{{skipped}} 个已安装并将跳过。",
					descriptionReadyFailed:
						"{{ready}} 个将导入，请先解决其余 {{failed}} 个的校验问题。",
					descriptionSkippedFailed:
						"{{skipped}} 个已安装，{{failed}} 个校验失败。",
					descriptionReadySkippedFailed:
						"{{ready}} 个将导入，{{skipped}} 个将跳过，{{failed}} 个校验失败。",
				},
				readyStatusTitle: "可执行导入",
				readyStatusDescription: "配置已就绪，请确认信息后点击“导入”。",
				importingStatus: "正在导入服务器…",
				successTitle: "导入成功",
				successAllSkipped: "所选服务器已安装，本次未做任何更改。",
				successInstalled: "服务器已成功安装，可立即使用。",
				successAutoEnabled: '已自动启用至 "{{profile}}"。',
				failureTitle: "导入失败",
				failureGeneric: "导入过程中发生错误",
				stats: {
					imported: "已导入",
					skipped: "已跳过",
				},
				installedServersTitle: "已安装服务器",
				success: {
					close: "关闭抽屉，继续浏览或排队下一个服务器。",
					servers: "打开服务器面板检查并管理新服务器。",
					profiles: "前往配置页，将该服务器加入适用的激活集合。",
					profilesWithName: '打开配置页，确认 "{{profile}}" 已显示该服务器。',
				},
				failure: {
					adjustServers: "返回服务器面板调整或移除配置后再试。",
					reviewPreview: "根据上方预览结果查找错误并修复后重新确认。",
					rerunPreview: "保持当前抽屉，更新配置并重新预览后再导入。",
				},
				nextSteps: {
					title: "后续操作",
				},
				skipSteps: {
					useExisting: "关闭抽屉，直接使用已有服务器。",
					chooseAnother: "返回上一步，重新选择其他服务器（如有需要）。",
				},
				failedSummary: "导入校验失败：{{servers}}。请解决问题后再试。",
				failedSummaryFallback_one: "所选服务器",
				failedSummaryFallback_other: "所选服务器",
				validationErrorGeneric: "导入校验失败",
				readySteps: {
					reviewConfig: "请再次检查上一阶段生成的服务器配置与能力。",
					autoAdd: "将按设置自动加入 Default 配置集。",
					manualAssign: "当前不会自动分配，可稍后在配置页手动添加。",
					importAction: "点击下方“导入”按钮安装服务器。",
				},
			},
		},
		confirmDelete: {
			title: "删除服务器",
			description: '确定要删除服务器 "{{serverId}}" 吗？此操作无法撤销。',
			confirm: "删除",
			cancel: "取消",
		},
	},
    "ja-JP": {
            title: "MCP サーバーの接続と監視",
		toolbar: {
			search: {
				placeholder: "サーバーを検索...",
				fields: {
					name: "名前",
					description: "説明",
				},
			},
			sort: {
				options: {
					name: "名前",
					enabled: "有効状態",
				},
			},
		},
		actions: {
			debug: {
				title: "検査",
				show: "検査",
				hide: "検査を隠す",
				open: "検査ビューを開く",
			},
			refresh: {
				title: "更新",
			},
			add: {
				title: "サーバーを追加",
			},
		},
		emptyState: {
			title: "サーバーが見つかりません",
			description: "最初の MCP サーバーを追加して利用を開始してください",
			action: "最初のサーバーを追加",
		},
		notifications: {
			importUnsupported: {
				title: "サポートされていない内容",
				message:
					"Uni-Import を使うにはテキスト、JSON スニペット、URL、または設定ファイルをドロップしてください。",
			},
			importRejections: {
        bundleDisabled: "MCPB と DXT バンドルのインポートは現在無効です。",
				fileTooLarge:
					"ドロップされたファイルは {{maxMb}} MB のインポート上限を超えています。",
				textTooLarge:
					"ドロップされたテキストは {{maxMb}} MB のインポート上限を超えています。",
        tooManyFiles:
          "一度にドロップできるファイルは {{maxFiles}} 個までです。",
			},
			importEmpty: {
				title: "インポートできる内容がありません",
				message: "ドロップされた内容から利用可能な設定を検出できませんでした。",
			},
			deepLinkImport: {
				title: "設定を受け取りました",
				message:
					"保存前にドロワーでインポートしたサーバー設定を確認してください。",
			},
			toggle: {
				enabledTitle: "サーバーを有効化しました",
				disabledTitle: "サーバーを無効化しました",
				message: "サーバー {{serverId}}",
				enabledDetail: "サーバー {{serverId}} を有効化しました",
				disabledDetail: "サーバー {{serverId}} を無効化しました",
				enableAction: "有効化",
				disableAction: "無効化",
				error: "サーバーを{{action}}できません: {{message}}",
				failedTitle: "サーバーの切り替えに失敗しました",
			},
			update: {
				title: "サーバーを更新しました",
				message: "サーバー {{serverId}}",
				errorTitle: "更新に失敗しました",
				errorMessage: "{{serverId}} を更新できません: {{message}}",
			},
			delete: {
				title: "サーバーを削除しました",
        message:
          "サーバー {{serverId}}。保存済みシークレットを使用していた場合は Secure Store のクリーンアップを確認してください。",
        cleanupReview:
          "このサーバーが保存済みシークレットを使用していた場合は Secure Store のクリーンアップを確認してください。",
				errorFallback: "サーバーの削除中にエラーが発生しました",
			},
			genericError: {
				title: "操作に失敗しました",
				unknown: "不明なエラー",
			},
		},
		statsCards: {
			total: {
				title: "サーバー総数",
				description: "登録済み",
			},
			enabled: {
				title: "有効",
				description: "機能トグル",
			},
			connected: {
				title: "接続中",
				description: "アクティブ接続",
			},
			instances: {
				title: "インスタンス",
				description: "全サーバーの合計",
			},
		},
		errors: {
			loadFailed: "サーバーの読み込みに失敗しました",
		},
		debug: {
			cardTitle: "検査情報",
			close: "閉じる",
			info: {
				baseUrl: "API ベース URL",
				currentTime: "現在時刻",
				error: "エラー",
				data: "サーバーデータ",
			},
		},
		entity: {
			tags: {
				unifyEligible: "直接公開",
			},
      description: { serverLabel: "サーバー: {{name}}" },
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				streamableHttp: "Streamable HTTP",
				headerAuth: "ヘッダー認証",
				oauth: "OAuth",
				oauthWarning: "認可の期限切れ — 再認可が必要です",
			},
			iconAlt: {
				named: "{{name}} のアイコン",
				fallback: "サーバーのアイコン",
			},
			stats: {
				tools: "ツール",
				prompts: "プロンプト",
				resources: "リソース",
				templates: "テンプレート",
			},
		},
		capabilityList: {
			searchPlaceholder: "{{label}} を検索...",
			emptyFallback: "データがありません",
			detailsToggle: "詳細",
			inputSchemaTitle: "入力スキーマ",
			outputSchemaTitle: "出力スキーマ",
			table: {
				argument: "引数",
				required: "必須",
				requiredYes: "はい",
				requiredNo: "いいえ",
				description: "説明",
				property: "プロパティ",
				type: "型",
				details: "詳細",
				enum: "列挙:",
				items: "items:",
				itemsEnum: "items.enum:",
			},
		},
		detail: {
			errors: {
				noServerId: "サーバー ID が指定されていません。",
			},
			loading: {
				title: "サーバー詳細を読み込み中",
        description:
          "サービスは応答していますが、詳細スナップショットの準備に少し時間がかかっています。",
			},
			viewModes: {
				browse: "閲覧",
				debug: "検査",
			},
			overview: {
				labels: {
					service: "サービス",
					upstreamName: "アップストリーム名",
					namespace: "名前空間",
					runtime: "ランタイム",
					type: "タイプ",
					auth: "認証",
					protocol: "プロトコル",
					version: "バージョン",
					capabilities: "機能",
					description: "説明",
					defaultHeaders: "既定ヘッダー",
					category: "カテゴリ",
					scenario: "シナリオ",
					command: "コマンド",
					repository: "リポジトリ",
				},
				status: {
					enabled: "有効",
					disabled: "無効",
					unifyEligible: "直接公開が利用可能",
				},
			},
			namespaceIssue: {
				title: "名前空間の修復が必要です",
				statusConflict: "競合",
				statusInvalid: "無効な名前空間",
				popoverCapabilityCollision:
					"この名前空間は別の Server が所有する機能名を生成します。一意の namespace を手動で指定してください。",
				popoverCapabilityCollisionWithServer:
					"この名前空間は既存の Server <server>{{namespace}}</server> が所有する機能名を生成します。一意の namespace を手動で指定してください。",
				popoverLegacyCollision:
					"この名前空間を正規化すると、既存の有効な Server と同じ名前になります。一意の namespace を手動で指定してください。",
				popoverLegacyCollisionWithServer:
					"この名前空間を正規化すると、既存の Server <server>{{namespace}}</server> の有効な名前空間と競合します。一意の namespace を手動で指定してください。",
				popoverInvalid:
					"この名前空間は安全に正規化できません。一意の小文字 snake_case を手動で指定してください。",
				retryConflict:
					"この namespace は一意性の検証に失敗しました。変更してから再度保存してください。",
				description:
					"従来の名前空間「{{namespace}}」は安全に公開できません。一意で規則に準拠した名前空間を入力すると、関連する参照が自動的に修復されます。",
				capabilityCollision:
					"名前空間「{{namespace}}」が生成する機能名は別のサーバーですでに使用されています。個々の機能名を変更せずに修復するには、一意で規則に準拠した名前空間を入力してください。",
				conflicts: "競合する名前空間: {{namespaces}}",
				inputLabel: "置換後の名前空間",
				action: "修復",
				success: "名前空間を修復しました",
				pendingTitle: "名前空間を修復しました。更新は保留中です",
				pendingDescription:
					"名前空間の変更はコミットされましたが、ランタイムまたはキャッシュの更新はまだ完了していません。ダウンストリーム呼び出しを再試行する前に Server の詳細を更新してください。",
				errorTitle: "名前空間の修復に失敗しました",
				iconLabel: "名前空間の修復が必要です",
			},
			deleteDialog: {
				title: "サーバーを削除",
				description: "この操作は元に戻せません。",
				cancel: "キャンセル",
				confirm: "削除",
				pending: "削除中...",
			},
			actions: {
				refresh: "更新",
				edit: "編集",
				disable: "無効化",
				enable: "有効化",
				delete: "削除",
			},
			instances: {
				title: "インスタンス ({{count}})",
				empty: "インスタンスがありません。",
			},
			tabs: {
				overview: "概要",
				tools: "ツール ({{count}})",
				prompts: "プロンプト ({{count}})",
				resources: "リソース ({{count}})",
				templates: "テンプレート ({{count}})",
				capabilities: "機能 ({{count}})",
				logs: "ログ",
			},
			filters: {
				kind: {
					all: "すべての種類",
					selected: "{{count}} 種類",
				},
				status: {
					all: "すべて",
					enabled: "有効",
					disabled: "無効",
				},
			},
			logs: {
				title: "ログ",
        description:
          "このサーバーに関連する実行ログとアクティビティログを表示します。",
				searchPlaceholder: "ログを検索...",
				refresh: "ログを更新",
				expand: "ログを展開",
				collapse: "ログを折りたたむ",
				loading: "ログを読み込み中...",
				headers: {
					timestamp: "時刻",
					action: "アクション",
					category: "カテゴリ",
					status: "ステータス",
					target: "対象",
				},
				empty: "このサーバーに関連するログはまだありません。",
			},
			capabilityList: {
				labels: {
					tools: "ツール",
					resources: "リソース",
					prompts: "プロンプト",
					templates: "テンプレート",
				},
				title: "{{label}} ({{count}})",
				empty: "このサーバーには {{label}} がありません",
				emptyAll: "このサーバーには機能がありません",
			},
			debug: {
				proxyUnavailable:
					"プロキシモードは利用できません：このサーバーは有効なプロファイルに含まれていません。",
			},
			inspector: {
				channel: {
					proxy: "プロキシ",
					native: "ネイティブ",
					fallback:
						"プロキシが利用可能になるまでネイティブにフォールバックします",
					hintTitle: "プロキシが利用不可",
					hintDescription:
						"プロキシモードを使うには、有効なプロファイルでこのサーバーを有効化してください。",
					openProfiles: "プロファイルを開く",
				},
				labels: {
					tools: "ツール",
					resources: "リソース",
					prompts: "プロンプト",
					templates: "テンプレート",
				},
				tabs: {
					results: "結果 ({{count}})",
					logs: "ログ ({{count}})",
				},
				filterPlaceholder: "{{label}} をフィルタ...",
				actions: {
					refresh: "更新",
					list: "取得",
					inspect: "検査",
				},
				logs: {
					search: "ログを検索...",
					clear: "ログをクリア",
					empty: "インスペクターのイベントはありません。",
					status: {
						mode: {
							proxy: "プロキシ",
							native: "ネイティブ",
						},
						event: {
							request: "リクエスト",
							success: "成功",
							error: "エラー",
							progress: "進行",
							log: "ログ",
							cancelled: "キャンセル済み",
						},
					},
				},
				results: {
					lastFetched: "最終取得 {{time}}",
					emptyFetched: "{{label}} は返されませんでした。",
					emptyPrompt:
						"{{label}} リストを実行して最新データを取得してください。",
				},
				oauth: {
					title: "OAuth 認可",
					description:
						"このホスト型 MCP サーバーをメインフローで使う前に、OAuth を設定して接続してください。",
					loading: "OAuth 状態を更新しています…",
					state: {
						notConfigured: "未設定",
						disconnected: "未接続",
						connected: "接続済み",
						expired: "期限切れ",
					},
					fields: {
						authorizationEndpoint: "認可エンドポイント",
						tokenEndpoint: "トークンエンドポイント",
						clientId: "Client ID",
						clientSecret: "Client Secret",
						clientSecretPlaceholderExisting:
							"空欄のまま保存すると既存のシークレットを保持します",
						clientSecretPlaceholderNew: "公開クライアントでは任意です",
						scopes: "Scopes",
						redirectUri: "リダイレクト URI",
					},
					actions: {
						save: "設定を保存",
						connect: "OAuth で接続",
						reconnect: "OAuth を再接続",
						revoke: "トークンを削除",
					},
					summary: {
						configured: "設定済み",
						clientSecret: "Client Secret",
						stored: "保存済み",
						notSet: "未設定",
						expiresAt: "有効期限",
						notAvailable: "なし",
					},
					manualOverride: {
						title: "手動の Authorization ヘッダーが有効です",
						description:
							"このサーバーの転送設定にはすでに Authorization ヘッダーがあるため、保存済みの OAuth トークンより手動値が優先されます。",
					},
					notifications: {
						savedTitle: "OAuth 設定を保存しました",
            savedMessage: "このサーバーの OAuth 設定を更新しました。",
						saveFailedTitle: "OAuth 設定の保存に失敗しました",
						connectFailedTitle: "OAuth を開始できませんでした",
						revokedTitle: "OAuth トークンを削除しました",
						revokedMessage:
							"このサーバーに保存されていた OAuth 資格情報を削除しました。",
						revokeFailedTitle: "OAuth の削除に失敗しました",
					},
				},
			},
		},
		oauth: {
			callback: {
				title: "OAuth 認可",
				processing: "認可を完了しています。しばらくお待ちください...",
				success: "認可が完了しました。サーバー詳細ページへ戻ります...",
				error: "認可に失敗しました。",
				back: "サーバー一覧へ戻る",
			},
			errors: {
				missingParams: "必要な OAuth パラメータが不足しています。",
				callbackFailed: "OAuth コールバックの処理に失敗しました。",
			},
		},
		instanceDetail: {
			errors: {
				missingParams:
					"サーバー ID またはインスタンス ID が指定されていません。",
				notFound:
					"インスタンスが見つからないか、詳細の読み込み中にエラーが発生しました。",
			},
			sections: {
				details: {
					title: "インスタンス詳細",
					fields: {
						instanceId: "インスタンス ID：",
						server: "サーバー：",
						status: "状態：",
						connectionAttempts: "接続試行回数：",
						connectedFor: "接続時間：",
						tools: "ツール：",
						processId: "プロセス ID：",
						health: "ヘルス：",
					},
				},
				controls: {
					title: "インスタンス制御",
					description: "インスタンスの接続状態を管理します",
					actions: {
						cancelInitialization: "初期化をキャンセル",
						disconnect: "インスタンスを切断",
						forceDisconnect: "強制切断",
						reconnect: "インスタンスを再接続",
						resetReconnect: "リセットして再接続",
					},
					hints: {
						reconnectDelay: "再接続の完了には数秒かかる場合があります",
					},
				},
				metrics: {
					title: "インスタンスメトリクス",
					description: "このインスタンスのパフォーマンス指標と統計",
					fields: {
						cpuUsage: "CPU 使用率：",
						memoryUsage: "メモリ使用量：",
						connectionStability: "接続安定性：",
					},
				},
			},
			notices: {
				healthIssue: "ヘルス問題を検出しました：",
				statusNote: "ステータスメモ：",
				error: "エラー：",
			},
			healthMessages: {
				idlePlaceholder:
					"インスタンスはアイドル状態です（プレースホルダー、未接続）",
			},
		},
		manual: {
			ingest: {
				default:
					"JSON、TOML、またはテキストをドロップ／貼り付け、またはアイコンをクリックしてローカル設定をスキャン",
				parsingDropped: "ドロップしたテキストを解析しています",
				parsingPasted: "貼り付けた内容を解析しています",
				success: "サーバー構成を読み込みました",
				noneDetectedError: "入力からサーバーが検出されませんでした",
				noneDetectedTitle: "サーバーが見つかりません",
				noneDetectedDescription:
					"入力内容にサーバー定義が含まれていませんでした。",
				parseFailedFallback: "入力の解析に失敗しました",
				parseFailedTitle: "解析に失敗しました",
				editing: "サーバーを編集",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "ヒント：",
				tipSuffix: "を押すとすぐに貼り付けできます。",
			},
			refreshFromRegistry: "レジストリから更新",
			refresh: {
				success: "レジストリからメタデータを更新しました",
				error: "メタデータの更新に失敗しました",
				notFound: "レジストリにサーバーが見つかりません",
			},
			viewMode: {
				form: "フォーム",
				json: "JSON",
			},
			tabs: {
				core: "基本設定",
					unify: "直接公開",
				meta: "メタ情報",
				metaWip: "プレビュー",
			},
			header: {
				title: {
					edit: "サーバーを編集",
					import: "サーバーをインポート",
					create: "サーバー統合インポート",
				},
				description: {
					edit: "既存のサーバー設定を確認して更新します。このモードでは JSON プレビューは読み取り専用です。",
					import: "レジストリから取得したサーバーを設定してインポートします。",
					create: "構成をドラッグ＆ドロップするか、手動で入力してください。",
				},
			},
			buttons: {
				reset: "フォームをリセット",
				resetAria: "フォームをリセット",
				cancel: "キャンセル",
				preview: "プレビュー",
				previewing: "プレビュー中...",
				save: "変更を保存",
				import: "サーバーをインポート",
				saving: "保存中...",
				importing: "インポート中...",
				processing: "処理中...",
			},
			bulk: {
				title: "{{count}} 件のサーバーを検出",
				description:
					"プレビューとインポートするサーバーを選択してください。{{count}} 件選択中。",
				selectAll: "すべて選択",
				clearSelection: "クリア",
				enable: "有効化",
				disable: "無効化",
				bulkModeEnter: "一括選択",
				bulkModeExit: "一括選択を終了",
				bulkModeDescription: "一括操作の対象 {{count}} 件",
				includeInImport: "インポートに追加",
				excludeFromImport: "インポートから除外",
				selectServer: "{{name}} を選択",
				includeForImport: "{{name}} をインポート対象に含める",
				missingEndpoint: "コマンドまたは URL がありません",
				backToList: "検出されたサーバーに戻る",
				noSelectionTitle: "サーバーが選択されていません",
				noSelectionDescription:
					"プレビューとインポートを続行するには、少なくとも 1 件のサーバーを選択してください。",
			},
			scan: {
				action: "ローカル設定をスキャン",
				actionHint: "クリックしてローカル設定をスキャン",
				noneTitle: "ローカル設定が見つかりません",
				noneDescription:
					"スキャン可能なローカル設定ファイルを持つ MCP クライアントが検出されませんでした。",
				noServersTitle: "サーバーが検出されませんでした",
				noServersDescription:
					"ローカルスキャンでインポート可能な MCP サーバー項目は見つかりませんでした。",
				failedTitle: "ローカルスキャンに失敗しました",
			},
			fields: {
				name: {
					label: "名称",
					placeholder: "例: local-mcp",
					readOnlyTitle: "編集モードでは名称を変更できません",
					readOnlyTitleAfterOAuth:
						"OAuth のセットアップ開始後は名称を変更できません",
				},
				namespace: {
					label: "名前空間",
					placeholder: "例: local_mcp",
          suggestionAction: "推奨される名前空間を使用:",
					importMapping: "{{original}} → {{namespace}}",
				},
				type: {
					label: "種別",
					options: {
						stdio: "Stdio",
						streamable_http: "ストリーミング HTTP",
						sse: "SSE（レガシー）",
					},
				},
				command: {
					label: "コマンド",
					placeholder: "例: uvx my-mcp",
				},
				url: {
					label: "サーバー URL",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "引数",
					ghost: "引数を追加",
					placeholder: "引数 {{count}}",
				},
				env: {
					label: "環境変数",
					ghostKey: "キーを追加",
					keyPlaceholder: "キー",
				},
				headers: {
					label: "HTTP ヘッダー",
					ghostKey: "ヘッダーを追加",
					keyPlaceholder: "ヘッダー",
				},
				urlParams: {
					label: "URL パラメータ",
					ghostKey: "パラメータ名",
					ghostValue: "値",
					keyPlaceholder: "パラメータ",
				},
				common: {
					addRow: "行を追加",
					ghostValue: "値を追加",
					valuePlaceholder: "値",
					removeRow: "行を削除",
					confirmRemoveRow: "削除を確認",
				},
				meta: {
					iconAlt: "サーバーアイコン",
					version: {
						label: "バージョン",
						placeholder: "例: 1.0.0",
					},
					website: {
						label: "Web サイト",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "リポジトリ URL",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "リポジトリ種類",
							placeholder: "例: github",
						},
						subfolder: {
							label: "リポジトリ サブフォルダ",
							placeholder: "任意のサブフォルダ",
						},
						id: {
							label: "リポジトリエントリ ID（メタデータ）",
							placeholder: "任意のメタデータ識別子",
						},
					},
					description: {
						label: "説明",
						placeholder: "簡単な説明",
					},
				},
				json: {
					label: "サーバー JSON",
				},
				unifyEligibility: {
					badge: "高度な公開制御",
					title: "Unify 直接公開対象としてマーク",
          description:
            "このオプションは、サーバーを Unify モードで直接公開可能な候補としてマークします。選択されたクライアントには、ツール・プロンプト・リソース・テンプレートを直接公開できます。",
					whatIsIt: "これは何ですか？",
          whatIsItDesc:
            "このサーバーを Unify で直接公開対象として扱い、UCAN ブローカー経由のみの到達に限定したくない場合に有効化してください。",
					whenToUse: "いつ使いますか？",
          whenToUseDesc:
            "Memory、Audit など、選択した Unify クライアントに主要ケイパビリティを直接公開したいサーバーで使います。",
					watchOut: "注意点は？",
          watchOutDesc:
            "通常の設定項目として気軽に有効化しないでください。クライアントが直接公開を選ぶと、このサーバーのケイパビリティは UCAN 専用経路を迂回してクライアントのコンテキストに入ります。",
					howToEnable: "有効化手順",
          howToEnableDesc:
            "まずここで eligible に設定し、次に Unify モードの Client で Server Level（全能力）または Capability Level（ツール/プロンプト/リソース/テンプレート単位）を選んで公開範囲を決めます。",
          toggleHint:
            "ここでは資格だけを付与します。実際に公開するかどうか、どう公開するかは Client 側で決まります。",
				},
			},
			auth: {
				label: "認証",
				transportHint:
					"URL パラメータと HTTP ヘッダーは任意の転送オプションです。OAuth 利用後も、このサーバーに必要なら引き続き適用されます。",
				mode: {
					header: "ヘッダー方式",
					oauth: "OAuth",
				},
				oauth: {
					connectedTitle: "OAuth に接続しました",
					connectedMessage: "認可が完了しました。",
					listenerNotReady:
						"OAuth コールバックの待受けを初期化中です。しばらくしてから再度お試しください。",
					connectFailedTitle: "OAuth を開始できませんでした",
					revokedTitle: "OAuth トークンを削除しました",
          revokedMessage:
            "このサーバーに保存されていた OAuth 資格情報を削除しました。",
					revokeFailedTitle: "OAuth の削除に失敗しました",
					unknownError: "不明なエラー",
					state: {
						connected: "接続済み",
						expired: "期限切れ",
						disconnected: "未接続",
						notConfigured: "未設定",
					},
					statusLabel: "OAuth の状態",
					secureStoreStored: "OAuth 認証情報は Secure Store に保存されています",
					secureStoreUnavailable: {
						title: "Secure Store の対応が必要です",
						message:
							"Secure Store の準備ができていません。OAuth を接続する前にロック解除または初期化してください。",
					},
					legacyReconnect: {
						title: "認証情報を保護するため OAuth を再接続してください",
						message:
							"既存の認証情報を Secure Store の管理下に移すため、OAuth を再接続してください。",
					},
					manualOverride: {
						title: "手動の Authorization ヘッダーが有効です",
						description:
							"このサーバーの転送設定にすでに Authorization ヘッダーがあるため、保存済みの OAuth トークンより手動値が優先されます。",
					},
					progress: {
						discover: "メタデータの取得",
						register: "クライアント登録",
						authorize: "認可",
						complete: "認証完了",
            preparingMessage: "OAuth フローを準備し、認可ページを開いています…",
            listenerPreparingMessage:
              "デスクトップのコールバック待受けを準備しています…",
						awaitingMessage: "ポップアップで認可を完了するまでお待ちください…",
						connectedMessage: "このサーバーの OAuth は接続済みです。",
						errorMessage:
							"OAuth に問題があります。再接続して認可を更新してください。",
					},
					actions: {
						reconnect: "OAuth を再接続",
						connect: "OAuth で接続",
						revoke: "トークンを削除",
						configure: "詳細設定",
					},
					fields: {
						authorizationEndpoint: "認可エンドポイント",
						tokenEndpoint: "トークンエンドポイント",
						clientId: "クライアント ID",
						clientSecret: "クライアントシークレット",
            clientSecretPlaceholderExisting:
              "空欄のままにすると保存済みのシークレットを維持します",
						clientSecretPlaceholderNew: "パブリッククライアントでは任意です",
						scopes: "スコープ",
						redirectUri: "リダイレクト URI",
            placeholderAuthorizationEndpoint:
              "https://issuer.example.com/authorize",
						placeholderTokenEndpoint: "https://issuer.example.com/token",
						placeholderScopes: "read write",
					},
					loading: "OAuth 状態を更新しています…",
				},
			},
			errors: {
				nameRequired: "名称は必須です",
				namespaceRequired: "名前空間は必須です",
				namespaceInvalid:
					"1～64 文字の小文字、数字、単一のアンダースコアを使用し、文字で始めてください。",
				namespaceReviewTitle: "サーバーの名前空間を確認してください",
				namespaceReviewDescription:
					"{{name}} は正規の名前空間ではありません。推奨値を適用するか、編集してからプレビューしてください。",
				kindRequired: "サーバー種別を選択してください",
				urlInvalid: "有効な URL を入力してください",
				commandRequired: "Stdio サーバーにはコマンドが必要です",
				urlRequired: "非 Stdio サーバーには URL が必要です",
				commandRequiredTitle: "コマンドが必要です",
				commandRequiredBody:
					"Stdio サーバーに使用するコマンドを入力してください。",
				endpointRequiredTitle: "エンドポイントが必要です",
				endpointRequiredBody: "非 Stdio サーバーには URL を入力してください。",
				jsonNoServers: "JSON からサーバーが見つかりませんでした",
				jsonMultipleServers: "JSON モードではサーバーを 1 件のみ扱えます",
				jsonParseFailedTitle: "JSON を解析できません",
				jsonParseFailedFallback: "JSON の解析に失敗しました",
				invalidJsonTitle: "JSON が無効です",
				oauthDraftServerFailed: "OAuth 用の下書きサーバーの作成に失敗しました",
				oauthServerIdRequired: "OAuth を開始するにはサーバー ID が必要です",
			},
			secrets: {
				pick: "シークレットを使用",
				title: "セキュアストアを使用",
        description:
          "このランタイムフィールドにライトオンリーのプレースホルダーを挿入します。",
				unavailablePick: "セキュアストアを使用できません",
				unavailableTitle: "セキュアストアを使用できません",
				unavailableDescription:
					"セキュアストアが復旧するまで、シークレットのプレースホルダーは選択できません。",
				unavailableListTitle: "シークレットアクセスに対応が必要です",
				unavailableListDescription:
					"Security 設定を開いてセキュアストアへのアクセスを復旧してください。",
				openSecuritySettings: "Security 設定",
				search: "シークレットを検索...",
				loading: "シークレットを読み込み中",
				empty: "保存されたシークレットはありません",
				createInline: "新しいシークレット",
				tagAlias: "{{alias}}",
				inspect: "Secure Store で {{alias}} を開く",
				inlinePrefix: "シークレット前のテキスト",
				inlineTrailing: "シークレット後のテキスト",
				inlineText: "シークレットフィールドのテキスト",
				clear: "シークレットを削除",
				storedSecret: "保存済みシークレット",
			},
		},
		wizard: {
			steps: {
				form: { label: "構成", hint: "セットアップ" },
				preview: { label: "プレビュー", hint: "確認" },
				result: { label: "インポートと割り当て", hint: "完了" },
			},
			header: {
				addTitle_one: "MCP サーバーを追加",
				addTitle_other: "MCP サーバーを追加",
				addDescription_one: "新しい MCP サーバーを設定してインストールします",
				addDescription_other:
					"検出した {{count}} 件の MCP サーバーを確認してインストールします",
				editTitle: "サーバーを編集",
				editDescription: "サーバー設定を更新します",
			},
			preview: {
				retry: "プレビューを再実行",
        retryDescription: "選択中のサーバーの機能プレビューを再生成します。",
				generating: "機能プレビューを生成中…",
				serverPickerLabel: "サーバー",
				selectServer: "サーバーを選択",
				filterCapabilities: "機能をフィルタ...",
				capabilitiesTitle: "機能",
				capabilitiesSummary: "機能 · {{summary}}",
        emptyCapabilities: "このサーバーでは機能が見つかりませんでした。",
				capabilities: {
					tool: "ツール",
					tools: "ツール",
					resource: "リソース",
					resources: "リソース",
					template: "テンプレート",
					templates: "テンプレート",
					prompt: "プロンプト",
					prompts: "プロンプト",
				},
			},
			buttons: {
				back: "戻る",
				cancel: "キャンセル",
				preview: "プレビュー",
				previewing: "プレビュー中...",
				next: "次へ",
				import: "インポート",
				importing: "インポート中...",
				validating: "検証中...",
				done: "完了",
				reset: "フォームをリセット",
				resetDescription: "すべてのフィールドをクリアし、初期設定に戻します。",
			},
			result: {
				validation: {
					ready: "インポート対象",
					skipped: "スキップ予定",
					failed: "検証失敗",
				},
				readyTitle: "インポートの準備完了",
				readyDescription:
					"インポートボタンを押してインストールを続行してください",
				validating: "インポートを検証中...",
				validationFailedTitle: "インポート検証に失敗しました",
				validationFailedDescription:
					"以下のブロッカーを解消してから、再度検証を実行してください。",
				validatedTitle: "インポート検証に成功",
				validatedDescription:
					"事前検証を通過しました。安全にインポートできます。",
				pendingImportReadyTitle: "公開の準備ができました",
				pendingImportReadyDescription:
					"OAuth の認可が完了しました。インポートするとこのサーバーが公開され、サーバー一覧に表示されます。",
				validatedWithWarningsTitle: "警告付きで検証に成功",
				validatedWithWarningsDescription:
					"事前検証は成功しましたが、一部のサーバーはスキップされます。",
				alreadyInstalledTitle: "インポート不要",
				alreadyInstalledDescription:
					"選択したサーバーはすでに存在します。インポートせずにそのまま利用できます。",
				skipSummary: {
					baseSingle: "{{count}} 件のサーバーをスキップしました",
					baseMultiple: "{{count}} 件のサーバーをスキップしました",
					withDetail: "{{base}}：{{detail}}",
					suffixAlreadyInstalled:
						"既に利用可能なため、再インポートは不要です。",
				},
				skipNotice: {
					titlePartial: "{{count}} 件のサーバーはスキップされます",
					descriptionAlreadyInstalled:
						"これらのサーバーは既にライブラリにあり、再インポートされません。",
					descriptionDuplicateName:
						"これらのサーバー名は既に使用されているため、インポートされません。",
					descriptionMixed:
						"以下のサーバーはインポートされません。各項目の理由を確認してください。",
				},
				summary: {
					badgeReady: "準備完了",
					badgeSkipped: "スキップ",
					badgeFailed: "失敗",
					titleMixed: "インポート確認",
					titleReadyFailed: "一部のみインポート可能",
					descriptionReadySkipped:
						"{{ready}} 件はインポートされ、{{skipped}} 件は既にインストール済みのためスキップされます。",
					descriptionReadyFailed:
						"{{ready}} 件はインポート可能です。残り {{failed}} 件の検証エラーを解消してください。",
					descriptionSkippedFailed:
						"{{skipped}} 件は既にインストール済み、{{failed}} 件は検証に失敗しました。",
					descriptionReadySkippedFailed:
						"{{ready}} 件はインポート、{{skipped}} 件はスキップ、{{failed}} 件は検証失敗です。",
				},
				readyStatusTitle: "インポート可能",
				readyStatusDescription:
					"設定は完了しています。内容を確認し、準備が整ったらインポートを実行してください。",
				importingStatus: "サーバーをインポートしています…",
				successTitle: "インポート成功",
				successAllSkipped:
					"選択したサーバーは既にインストール済みのため、変更はありませんでした。",
				successInstalled:
					"サーバーは正常にインストールされ、すぐに利用できます。",
				successAutoEnabled: '"{{profile}}" に自動的に割り当てました。',
				failureTitle: "インポート失敗",
				failureGeneric: "インポート中にエラーが発生しました",
				stats: {
					imported: "インポート済み",
					skipped: "スキップ",
				},
				installedServersTitle: "インストール済みサーバー",
				success: {
					close:
						"このドロワーを閉じてブラウズを続けるか、別のサーバーを追加してください。",
					servers:
						"サーバーダッシュボードを開き、新しいサーバーを確認・管理します。",
					profiles:
						"プロファイルを開いて適切なアクティベーションセットに追加します。",
					profilesWithName:
						'プロファイルを開き、"{{profile}}" に新しいサーバーが反映されているか確認します。',
				},
				failure: {
					adjustServers:
						"サーバーダッシュボードに戻り、設定を調整または削除してから再試行してください。",
					reviewPreview:
						"上部のプレビュー結果を確認し、問題を解消してから再度実行してください。",
					rerunPreview:
						"このドロワーを開いたまま設定を更新し、プレビューを再実行してからもう一度試してください。",
				},
				nextSteps: {
					title: "次のステップ",
				},
				skipSteps: {
					useExisting:
						"ドロワーを閉じ、既存のサーバーをそのまま利用してください。",
					chooseAnother:
						"必要に応じて前のステップへ戻り、別のサーバーを選択してください。",
				},
				failedSummary:
					"インポート検証に失敗しました（対象: {{servers}}）。問題を解消してから再試行してください。",
				failedSummaryFallback_one: "選択したサーバー",
				failedSummaryFallback_other: "選択したサーバー",
				validationErrorGeneric: "インポート検証に失敗しました",
				readySteps: {
					reviewConfig:
						"前のステップで生成された構成と機能を再確認してください。",
					autoAdd: "設定に従い自動的に Default プロファイルへ追加されます。",
					manualAssign:
						"現在は割り当てられません。後からプロファイル画面で追加できます。",
					importAction:
						"下の「インポート」ボタンを押してサーバーをインストールします。",
				},
			},
		},
		confirmDelete: {
			title: "サーバーを削除",
			description:
				'サーバー "{{serverId}}" を削除してもよろしいですか？この操作は取り消せません。',
			confirm: "削除",
			cancel: "キャンセル",
		},
	},
};
