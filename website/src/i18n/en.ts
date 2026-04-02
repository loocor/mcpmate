const en = {
	// Site
	"site.title": "MCPMate - Your MCP Assistant",

	// Nav
	"nav.home": "Home",
	"nav.features": "Features",
	"nav.documentation": "Documentation",
	"nav.contact": "Contact",
	"nav.waitlist": "Join Waitlist",
	"nav.preview": "Quick Start",
	"nav.download": "Quick Start",
	"nav.why": "Why MCPMate?",
	"nav.faq": "FAQ",

	// Hero
	"hero.early_access": "Now Open Source",
	"hero.title": "Your MCP Assistant",
	"hero.subtitle": "One configuration. Multiple services. Maximum efficiency.",
	"hero.description":
		"MCPMate handles MCP complexity so you can focus on building and creating.",
	"hero.cta.waitlist": "Join Waitlist",
	"hero.cta.download": "View on GitHub",
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
	"download.expired": "Public preview is paused while we finish the refactor.",
	"download.expires_in": "Preview expires in",
	"download.subtitle":
		"Build from source with Rust. Available for macOS, Windows, and Linux.",
	"download.for": "Download for",
	"download.version": "Version",
	"download.early_access": "(Preview)",
	"download.macos.arm64": "Apple Silicon (arm64)",
	"download.macos.x64": "Intel (x64)",
	"download.windows": "Windows",
	"download.linux": "Linux",
	"download.btn": "Download",
	"download.coming_soon": "Coming Soon",
	"download.preview_paused": "Preview paused",
	"download.copy": "Copy",
	"download.sha256": "SHA256",
	"download.quick_start": "Quick Start",
	"download.install_cli": "Install via command line",
	"download.cli_coming_soon": "CLI installer is coming soon.",
	"download.getting_started": "Getting Started",
	"download.getting_started.desc":
		"Follow our quick start guide to set up MCPMate and connect your first MCP service.",
	"download.read_guide": "Read the Guide",
	"download.notarize_notice":
		"The public preview is paused while we finish the architecture refactor.",
	"download.contact_intro": "Need access in the meantime? Email",
	"download.contact_or": "or join",
	"download.contact_discord": "our Discord community",
	"download.contact_suffix": "and we’ll share builds case by case.",

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
	"value.owners.p3": "Keep privacy, auditability, and local boundaries visible instead of burying them in scattered config files.",
	"value.owners.diagram": "Enterprise Readiness",

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
	"arch.values.security.p3": "Audit and validation surfaces before broad rollout",

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
		"A 'Maybe All-in-One' MCP Service Manager for developers and creators.",
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
	"faq.opensource.title": "Is MCPMate open source?",
	"faq.opensource.answer":
		"Yes! MCPMate is open source under the MIT license. Check out the code at github.com/loocor/mcpmate",
	"faq.expiry.title": "What happens when the preview expires?",
	"faq.expiry.answer":
    	"Pull the latest changes from GitHub and rebuild to get the newest version.",
	"faq.platforms.title": "Which platforms are supported?",
	"faq.platforms.answer":
		"macOS (Apple Silicon, Intel) is available now. Windows and Linux builds are planned.",
	"faq.security.title": "How does MCPMate handle security?",
	"faq.security.answer":
		"The core is built in Rust with a focus on safety. MCPMate orchestrates your tools and continues to harden sandboxing and validations before GA.",
	"faq.privacy.title": "What about privacy and telemetry?",
	"faq.privacy.answer":
		"Your MCP configurations run locally; MCPMate does not send tool content to our servers. Optional, privacy‑respecting crash and usage metrics may be added before GA with clear disclosure.",
	"faq.updates.title": "How do I update MCPMate?",
	"faq.updates.answer":
		"Pull the latest changes from GitHub and rebuild with `cargo build --release`.",
	"faq.different.title": "What makes MCPMate different from other solutions?",
	"faq.different.answer":
		"MCPMate combines profile-based capability control, flexible client rollout modes, import preview, and live inspection in one local-first control plane.",
	"faq.compatible.title": "Can I use MCPMate with my existing MCP tools?",
	"faq.compatible.answer":
		"Yes. MCPMate works with MCP-compatible clients and gives you Unify, Hosted, and Transparent paths, so you can choose between session-local UCAN control, deeper managed control, and direct native client output.",

	// Notice ribbon (site updating)
	"notice.construction.ribbon": "Updating",
	"notice.construction.tooltip":
		"We’re actively updating; details may be adjusted.",
} as const;

export default en;
