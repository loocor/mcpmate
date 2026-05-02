const en = {
	// Site
	"site.title": "MCPMate - Local MCP control plane",
	"site.description": "MCPMate is a local-first MCP control plane for managing servers, profiles, clients, runtime dependencies, and imports in one place.",

	// Nav
	"nav.home": "Home",
	"nav.features": "Features",
	"nav.documentation": "Documentation",
	"nav.contact": "Contact",
	"nav.preview": "Quick Start",
	"nav.download": "Quick Start",
	"nav.why": "Why MCPMate?",
	"nav.faq": "FAQ",

	// Hero
	"hero.early_access": "Now Open Source",
	"hero.title": "Local MCP control plane",
	"hero.subtitle": "Manage servers, profiles, clients, and runtime from one place.",
	"hero.description":
		"MCPMate is an open-source local control plane that organizes servers, profiles, client connections, and runtime dependencies so you spend less time editing config files.",
	"hero.cta.download": "View Releases",
	"hero.cta.learn": "Learn More",
	"hero.stats.configValue": "One Endpoint",
	"hero.stats.config": "One proxy endpoint for all AI clients",
	"hero.stats.resourceValue": "On-Demand",
	"hero.stats.resource": "Switch profiles, expose only what you need",
	"hero.stats.integrationValue": "Transparent",
	"hero.stats.integration": "Know what you use, inspect how it runs",
	"hero.dashboard": "MCPMate Dashboard",
	"hero.slide.dashboard": "Dashboard — system health and metrics",
	"hero.slide.profiles": "Profiles — switch capability sets",
	"hero.slide.servers": "Servers — connect and monitor MCP servers",
	"hero.slide.clients": "Clients — manage editor integrations",
	"hero.slide.market": "Market — browse the official MCP registry",

	// Download
	"download.title": "Quick Start",
	"download.subtitle":
		"Official installers are listed below; fork the repository on GitHub and build from source with Rust whenever you want full control.",
	"download.macos.arm64": "Apple Silicon (arm64)",
	"download.macos.x64": "Intel (x64)",
	"download.windows": "Windows",
	"download.linux": "Linux",
	"download.btn": "Download",
	"download.coming_soon": "Coming Soon",
	"download.quick_start": "Quick Start",
	"download.official_builds": "Official desktop builds",
	"download.col_platform": "Platform",
	"download.col_arch": "Architecture",
	"download.col_status": "Status",
	"download.col_downloads": "Downloads",
	"download.status_unstable": "Unstable",
	"download.available": "Available",
	"download.loading": "Loading…",
	"download.all_releases": "All releases",
	"download.load_error": "Release metadata could not be loaded. You can still open the releases page.",
	"download.retry": "Retry",
	"download.table_caption": "MCPMate desktop installers linked to the latest GitHub release",
	"download.platform_availability_note":
		"Windows and Linux are under active improvement. Installers are available to download, but features may be unavailable or unstable until those platforms catch up.",
	"download.latest_label": "Latest",
	"download.platform_macos": "macOS",
	"download.platform_windows": "Windows",
	"download.platform_linux": "Linux",
	"download.arch_arm64": "arm64",
	"download.arch_x64": "x64",
	"download.getting_started": "Getting Started",
	"download.getting_started.desc":
		"Follow our quick start guide to set up MCPMate and connect your first MCP service.",
	"download.read_guide": "Read the Guide",

	// Features
	"features.title": "Powerful Features",
	"features.subtitle":
		"MCPMate turns MCP from scattered configuration work into a local control plane for import, rollout, inspection, and switching.",
	"features.centralized": "Centralized Configuration",
	"features.centralized.desc":
		"Manage servers, profiles, and clients from one place instead of editing every client separately.",
	"features.resource": "Resource Optimization",
	"features.resource.desc":
		"Expose only the tools, prompts, and resources the current task actually needs.",
	"features.inspector": "Inspector",
	"features.inspector.desc":
		"Compare proxy and native behavior, run live calls, and capture raw evidence without leaving the dashboard.",
	"features.more_coming": "More powerful features are on the way.",
	"features.feedback_welcome":
		"We'd love to hear your feedback and suggestions!",
	"features.read_more": "Read more",
	"features.explore_all": "Explore all feature guides",
	"features.context": "Seamless Context Switching",
	"features.context.desc":
		"Switch shared profiles or task presets instead of rebuilding capability sets by hand.",
	"features.bridge": "Protocol Bridging",
	"features.bridge.desc":
		"Connect stdio-based clients to Streamable HTTP services without modifying the client.",
	"features.marketplace": "Market Install Flow",
	"features.marketplace.desc":
		"Browse registry entries, import them into preview, and keep discovery close to operations.",
	"features.templates": "Granular Controls",
	"features.templates.desc":
		"Enable or disable servers and capabilities at profile level for precise exposure control.",
	"features.autodiscovery": "Auto Discovery & Import",
	"features.autodiscovery.desc":
		"Detect existing client configs and pull them into MCPMate without starting from scratch.",
	"features.uniimport": "Uni‑Import",
	"features.uniimport.desc":
		"Drop, paste, or capture messy snippets from the web, then normalize, preview, and validate before import.",

	// Value (Why MCPMate?)
	"value.title": "Why MCPMate?",
	"value.subtitle":
		"MCPMate is for people who want MCP to be operable, not just technically possible.",
	"value.creators.title": "Stay in flow, switch by intent",
	"value.creators.p1":
		"Package working modes as shared profiles instead of re-tuning every client one toggle at a time.",
	"value.creators.p2":
		"Let compatible clients switch shared profiles through MCPMate's built-in MCP tools when they explicitly use Hosted + Profiles mode.",
	"value.creators.p3":
		"Keep only the capabilities a task actually needs in view to reduce noise and token waste.",
	"value.creators.diagram": "Creator Flow",
	"value.managers.title": "Roll out flexibly across clients",
	"value.managers.p1":
		"Choose Hosted mode for MCPMate-managed control or Transparent mode for direct native client output.",
	"value.managers.p2":
		"Let one client either follow the active shared profile set as a read-only default, switch explicit shared profiles in Hosted + Profiles mode, or use a client-specific custom profile where supported.",
	"value.managers.p3":
		"Keep compatibility escape hatches without giving up a richer managed path for most users.",
	"value.managers.diagram": "Team Consistency",
	"value.owners.title": "Operate locally with clearer control",
	"value.owners.p1": "Run the core as a local service and reopen the web UI or desktop shell only when you need it.",
	"value.owners.p2": "Use the local API and Inspector to automate checks and investigate real behavior.",
	"value.owners.p3": "Keep privacy, observability, and local boundaries visible instead of burying them in scattered config files.",
	"value.owners.diagram": "Operational Clarity",

	// Architecture (Design Values)
	"arch.title": "Design Principles",
	"arch.subtitle": "What we optimize for: operability, low friction, and local trust.",
	"arch.values.performance.title": "Performance First",
	"arch.values.performance.desc":
		"A Rust core that keeps routing, visibility, and inspection close to the work.",
	"arch.values.performance.p1": "Standalone core service with local API and local MCP endpoint",
	"arch.values.performance.p2": "Reuse one runtime instead of reconfiguring every client",
	"arch.values.performance.p3": "Operational visibility through metrics, Inspector, and runtime status",
	"arch.values.experience.title": "Delightful Experience",
	"arch.values.experience.desc":
		"Make advanced MCP workflows feel like switching modes, not editing files.",
	"arch.values.experience.p1": "Import from snippets, bundles, registries, and browser capture",
	"arch.values.experience.p2": "Profiles and client modes for gradual rollout",
	"arch.values.experience.p3": "Web, desktop, and API surfaces on the same local core",
	"arch.values.security.title": "Safety by Design",
	"arch.values.security.desc":
		"Local-first boundaries with explicit control over what gets exposed.",
	"arch.values.security.p1": "Capability-level exposure through profiles and client selection",
	"arch.values.security.p2": "Use Transparent mode when direct native output is the safer fit",
	"arch.values.security.p3": "Operational event logging before broad rollout",

	// Architecture stack table
	"arch.stack.title": "Technology Stack",
	"arch.stack.backend": "Backend",
	"arch.stack.backendValue": "Rust (Axum), SQLite",
	"arch.stack.frontend": "Frontend",
	"arch.stack.frontendValue": "React 18, TypeScript, Vite, Tailwind CSS",
	"arch.stack.api": "API",
	"arch.stack.apiValue": "REST API (port 8080), MCP endpoint (port 8000)",
	"arch.stack.protocols": "Protocols",
	"arch.stack.protocolsValue": "stdio, Streamable HTTP",
	"arch.stack.license": "License",
	"arch.stack.licenseValue": "AGPL-3.0",
	"arch.stack.clients": "Client modes",
	"arch.stack.clientsValue": "Hosted, Unify, Transparent",

	// Contact
	"contact.title": "Get in Touch",
	"contact.subtitle":
		"Have questions about MCPMate? We'd love to hear from you!",
	"contact.message": "Send a Message",
	"contact.message.label": "Your Message",
	"contact.name": "Your Name",
	"contact.name.placeholder": "Enter your name",
	"contact.email": "Your Email",
	"contact.email.placeholder": "Enter your email",
	"contact.message.placeholder": "How can we help you?",
	"contact.send": "Send Message",
	"contact.email.us": "Email Us",
	"contact.email.desc": "For general inquiries or support questions",
	"contact.github": "GitHub",
	"contact.github.desc": "Check out our repositories and contribute",
	"contact.error.required": "All fields are required",
	"contact.error.email": "Please enter a valid email address",
	"contact.success.title": "Message Sent!",
	"contact.success.message":
		"Thank you for contacting us. We'll get back to you as soon as possible.",

	// Footer
	"footer.description":
		"A local-first MCP control plane for managing servers, profiles, clients, and runtime in one place.",
	"footer.copyright": "© {year} MCPMate. All rights reserved.",
	"footer.product": "Product",
	"footer.resources": "Resources",
	"footer.legal": "Legal",
	"footer.language": "Language",
	"footer.documentation": "Documentation",
	"footer.changelog": "Changelog",
	"footer.roadmap": "Roadmap",
	"footer.privacy": "Privacy Policy",
	"footer.terms": "Terms of Service",

	// FAQ
	"faq.title": "Frequently Asked Questions",
	"faq.group.basics": "Why people choose MCPMate",
	"faq.group.setup": "How to get started smoothly",
	"faq.group.control": "What stays under your control",
	"faq.group.compare": "How it fits your workflow",
	"faq.opensource.title": "Is MCPMate open source?",
	"faq.opensource.answer":
		"Yes! MCPMate is open source under the AGPL-3.0 license. Check out the code at github.com/loocor/mcpmate",
	"faq.functions.title": "What does MCPMate actually do?",
	"faq.functions.answer":
		"MCPMate organizes MCP servers, reusable profiles, client connections, runtime dependencies, and configuration imports in one local dashboard. You can inspect live tool calls, switch profiles without editing files, and import server configs from existing clients.",
	"faq.usage.title": "Who is MCPMate for?",
	"faq.usage.answer":
		"MCPMate is for developers who manage multiple MCP-compatible AI clients and want a single place to control which servers, tools, and prompts each client sees without scattered config files.",
	"faq.platforms.title": "Which platforms are supported?",
	"faq.platforms.answer":
		"macOS installers are the most stable today. Windows and Linux installers are available from GitHub Releases, but those platforms are still catching up and some features may be incomplete or unstable.",
	"faq.security.title": "How does MCPMate handle security?",
	"faq.security.answer":
		"The core is built in Rust and runs entirely on your machine. MCPMate lets you control which capabilities are exposed per profile and per client.",
	"faq.privacy.title": "What about privacy and telemetry?",
	"faq.privacy.answer":
		"Your MCP configurations run locally and MCPMate does not send tool content to external servers.",
	"faq.updates.title": "How do I update MCPMate?",
	"faq.updates.answer":
		"Today, the simplest path is to download the latest installer from GitHub Releases. Built-in desktop auto-update is nearly ready, so this flow should get even smoother soon. If you run MCPMate from source, pull the latest changes from GitHub and rebuild with `cargo build --release`.",
	"faq.different.title": "What makes MCPMate different from other solutions?",
	"faq.different.answer":
		"MCPMate combines profile-based capability control, flexible client rollout modes, import preview, and live inspection in one local-first control plane.",
	"faq.compatible.title": "Can I use MCPMate with my existing MCP tools?",
	"faq.compatible.answer":
		"Yes. MCPMate works with MCP-compatible clients and gives you Unify, Hosted, and Transparent paths, so you can choose between session-local UCAN control, deeper managed control, and direct native client output.",
	"faq.clients.title": "Can MCPMate manage multiple AI clients?",
	"faq.clients.answer":
		"Yes. MCPMate detects installed MCP-compatible clients and lets you manage each one independently from the same dashboard. You can apply different profiles and choose Hosted, Unify, or Transparent mode per client for tools like Claude Code, Cursor, and VS Code.",
	"faq.runtime.title": "Does MCPMate need external runtimes?",
	"faq.runtime.answer":
		"Some MCP servers require runtimes like Node.js, Python (uv), or Bun. MCPMate includes a Runtime page that checks which runtimes are installed and offers install or repair actions when a server needs one.",
	"faq.migration.title": "How do I move my existing MCP configuration into MCPMate?",
	"faq.migration.answer":
		"Use the Import from client action on the Clients page to pull existing MCP configs from a detected client. You can also drag and drop JSON or TOML snippets, or use the built-in Market to browse and install server entries.",
	"faq.hotreload.title": "Do I need to restart MCPMate after changing a profile?",
	"faq.hotreload.answer":
		"Profile changes take effect for new sessions immediately. Active Hosted-mode sessions pick up the updated profile on next tool list refresh without requiring a restart.",
	"faq.languages.title": "What languages does the MCPMate UI support?",
	"faq.languages.answer":
		"The website and dashboard support English, Simplified Chinese, and Japanese.",
	"faq.contributing.title": "How can I contribute to MCPMate?",
	"faq.contributing.answer":
		"Fork the repository on GitHub, make your changes, and open a pull request. Bug reports and feature suggestions via GitHub Issues are also welcome.",
	"faq.vs_claude_desktop.title": "How is MCPMate different from Claude Desktop's MCP support?",
	"faq.vs_claude_desktop.answer":
		"Claude Desktop manages MCP servers for its own session only. MCPMate is a standalone control plane that manages servers, profiles, and client connections across multiple AI clients — Claude Desktop, Cursor, VS Code, and others — from one dashboard with reusable profile switching.",
	"faq.vs_manual.title": "How is MCPMate different from editing config files manually?",
	"faq.vs_manual.answer":
		"Manual config editing works for one client at a time and offers no preview, validation, or rollback. MCPMate provides a unified import pipeline with preview and validation, reusable profiles that can be applied across clients, and an inspector to verify live behavior before committing changes.",

	// Notice ribbon (site updating)
	"notice.construction.ribbon": "Updating",
	"notice.construction.tooltip":
		"We’re actively updating; details may be adjusted.",
} as const;

export default en;
