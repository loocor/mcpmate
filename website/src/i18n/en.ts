const en = {
	// Site
	"site.title": "MCPMate - Your MCP Assistant",

	// Nav
	"nav.home": "Home",
	"nav.features": "Features",
	"nav.documentation": "Documentation",
	"nav.pricing": "Pricing",
	"nav.contact": "Contact",
	"nav.waitlist": "Join Waitlist",
	"nav.preview": "Download Preview",
	"nav.download": "Download",
	"nav.why": "Why MCPMate?",
	"nav.faq": "FAQ",

	// Hero
	"hero.early_access": "Now in Preview",
	"hero.title": "Your MCP Assistant",
	"hero.subtitle": "One configuration. Multiple services. Maximum efficiency.",
	"hero.description":
		"MCPMate handles MCP complexity so you can focus on building and creating.",
	"hero.cta.waitlist": "Join Waitlist",
	"hero.cta.download": "Download Preview",
	"hero.cta.learn": "Learn More",
	"hero.stats.config": "Less Configuration",
	"hero.stats.resource": "Resource Reduction",
	"hero.stats.integration": "Integration Rate",
	"hero.dashboard": "MCPMate Dashboard",

	// Download
	"download.title": "Download MCPMate Preview",
	"download.expired": "Public preview is paused while we finish the refactor.",
	"download.expires_in": "Preview expires in",
	"download.subtitle":
		"Available for macOS (Apple Silicon, Intel). Windows and Linux builds coming soon.",
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
		"MCP is powerful but complex. MCPMate makes it easy to unlock that power without the configuration headaches",
	"features.centralized": "Centralized Configuration",
	"features.centralized.desc":
		"Configure once, use everywhere. Eliminate redundant setups across different clients.",
	"features.resource": "Resource Optimization",
	"features.resource.desc":
		"Intelligently manage server resources to reduce system overhead and improve performance.",
	"features.inspector": "Inspector",
	"features.inspector.desc":
		"Deep‑dive into server states, logs, and diagnostics without leaving the dashboard.",
	"features.more_coming": "More powerful features are on the way.",
	"features.feedback_welcome":
		"We'd love to hear your feedback and suggestions!",
	"features.read_more": "Read more",
	"features.context": "Seamless Context Switching",
	"features.context.desc":
		"Switch between different work scenarios with instant configuration changes.",
	"features.bridge": "Protocol Bridging",
	"features.bridge.desc":
		"Connect stdio-based clients to SSE services without modifying the client.",
	"features.marketplace": "Inline Marketplace",
	"features.marketplace.desc":
		"Official registry + mcpmarket.cn — find tools without hunting.",
	"features.templates": "Granular Controls",
	"features.templates.desc":
		"Toggle capabilities per tool/profile with fine‑grained switches.",
	"features.autodiscovery": "Auto Discovery & Import",
	"features.autodiscovery.desc":
		"Detect existing configs and import them without manual edits.",
	"features.uniimport": "Uni‑Import",
	"features.uniimport.desc":
		"Drag, drop, or paste configs; JSON/TOML, mcpb soon.",

	// Value (Why MCPMate?)
	"value.title": "Why MCPMate?",
	"value.subtitle":
		"Less setup, more flow — for creators, team leads, and owners alike.",
	"value.creators.title": "Keep Flow, Not Friction",
	"value.creators.p1":
		"No more copying configs between clients; configure once, use everywhere.",
	"value.creators.p2":
		"One place to start/stop tools; less memory, fewer distractions.",
	"value.creators.p3":
		"Switch work scenes instantly with profiles and presets.",
	"value.creators.diagram": "Creator Flow",
	"value.managers.title": "Consistency by Default",
	"value.managers.p1":
		"Onboard faster with shared presets and policy‑guarded configs.",
	"value.managers.p2":
		"Reduce “works on my machine” with uniform tooling and visibility.",
	"value.managers.p3":
		"Track usage, version drift, and common errors to prevent outages.",
	"value.managers.diagram": "Team Consistency",
	"value.owners.title": "Cost, Control, Confidence",
	"value.owners.p1": "Roll out safely across orgs with staged deployment.",
	"value.owners.p2": "Keep token spend and data boundaries under control.",
	"value.owners.p3": "Audit trails and guardrails to meet compliance needs.",
	"value.owners.diagram": "Enterprise Readiness",

	// Architecture (Design Values)
	"arch.title": "Design Principles",
	"arch.subtitle": "What we optimize for: performance, experience, and safety.",
	"arch.values.performance.title": "Performance First",
	"arch.values.performance.desc":
		"Rust core with a bias for low latency and low overhead.",
	"arch.values.performance.p1": "Thin proxy paths, near‑native throughput",
	"arch.values.performance.p2": "Resource‑aware orchestration; start on demand",
	"arch.values.performance.p3": "Practical metrics to spot bottlenecks",
	"arch.values.experience.title": "Delightful Experience",
	"arch.values.experience.desc":
		"Setup should be minutes, not days. Clear defaults over knobs.",
	"arch.values.experience.p1": "One configuration, many surfaces",
	"arch.values.experience.p2": "Profiles/presets for instant context switch",
	"arch.values.experience.p3": "Neutral to tools — no vendor lock‑in",
	"arch.values.security.title": "Safety by Design",
	"arch.values.security.desc":
		"Privacy‑respecting, auditable, principle‑of‑least‑privilege.",
	"arch.values.security.p1": "Local‑first, minimal telemetry, explicit consent",
	"arch.values.security.p2":
		"Clear boundaries between upstream/downstream tools",
	"arch.values.security.p3": "Audit trails and policy hooks",

	// Pricing
	"pricing.title": "Simple, Transparent Pricing",
	"pricing.subtitle": "From individuals to organizations, choose what fits you",
	"pricing.notice.pending":
		"Preview is free. Pricing will be finalized before GA.",
	"pricing.billing.monthly": "Monthly",
	"pricing.billing.annual": "Annual",
	"pricing.per_month": "/ month",
	"pricing.billed_annually": "billed annually",
	"pricing.billed_monthly": "billed monthly",
	"pricing.starter": "Starter",
	"pricing.starter.desc":
		"Personal use with real control — no server/client caps",
	"pricing.starter.price": "Free",
	"pricing.starter.feature1":
		"Default + up to 3 custom profiles (capability control enabled)",
	"pricing.starter.feature2":
		"Unified proxy, connection pool, and notifications",
	"pricing.starter.feature3":
		"Built‑in MCP server: list, view, switch profiles",
	"pricing.starter.feature4": "Official client templates with atomic backups",
	"pricing.starter.feature5": "Uni‑Import and MCP Bundle; inline marketplace",
	"pricing.starter.feature6": "Runtime manager (uv/Bun) and capability cache",
	"pricing.professional": "Professional",
	"pricing.professional.desc": "Unlimited governance with built‑in Inspector",
	"pricing.professional.price": "TBD",
    "pricing.price.professional.monthly": "TBD",
    "pricing.price.professional.annual_per_month": "TBD",
	"pricing.professional.includes": "Includes everything in Starter, plus:",
	"pricing.professional.feature1": "Unlimited profiles and full governance",
	"pricing.professional.feature2":
		"Inspector (Express/Expert), proxy/native modes",
	"pricing.professional.feature3":
		"Built‑in MCP server: create profiles and more",
	"pricing.professional.feature4": "Batch import with preflight and preview",
	"pricing.professional.feature5":
		"Audit & logs with query/export and reporting",
	"pricing.professional.feature6": "Connection pool & lifecycle management",
	"pricing.professional.feature7": "Standard support (business days)",
	"pricing.advanced": "Advanced",
	"pricing.advanced.desc": "Audit, automation, and distribution at scale",
	"pricing.advanced.price": "Contact Us",
	"pricing.advanced.includes": "Includes everything in Professional, plus:",
	"pricing.advanced.feature1": "REST API tokens for external automation",
	"pricing.advanced.feature2":
		"Client template overrides (user/community), hot reload",
	"pricing.advanced.feature3":
		"Template behavior controls (Auto Run/Keep Alive/Auto Approve)",
	"pricing.advanced.feature4":
		"Prompt override management with version control",
	"pricing.advanced.feature5": "Cost tracking & analytics across services",
	"pricing.advanced.feature6": "Marketplace governance (allow/deny lists)",
	"pricing.advanced.feature7":
		"Priority support, preview access & multi-tenant roadmap",
	"pricing.month": "/ month",
	"pricing.custom": "Need a custom solution?",
	"pricing.contact": "Contact Us",
	"pricing.whats_included": "What's included",
	"pricing.popular": "Popular",
	"pricing.contact_sales": "Contact Sales",

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
	"faq.when.title": "When will MCPMate be available?",
	"faq.when.answer":
		"MCPMate is currently in preview. You can download the preview today.",
	"faq.opensource.title": "Is MCPMate open source?",
	"faq.opensource.answer":
		"Not for now. We are focused on quality and stability first, and may consider open sourcing parts of the project in the future.",
	"faq.free.title": "Is MCPMate free?",
	"faq.free.answer":
		"During preview, all features are free. After GA, we plan to keep a free personal edition. Exact pricing will be finalized closer to launch.",
	"faq.expiry.title": "What happens when the preview expires?",
	"faq.expiry.answer":
    "This preview build expires on Oct 25, 2025. The app will stop running and prompt you to download the latest build from this website.",
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
		"Until in‑app updates ship, please download the latest preview from this website when notified.",
	"faq.different.title": "What makes MCPMate different from other solutions?",
	"faq.different.answer":
		"MCPMate is built with Rust for maximum performance and reliability. Our AI-driven configuration management and themed tool groups make it easier than ever to manage your MCP ecosystem.",
	"faq.compatible.title": "Can I use MCPMate with my existing MCP tools?",
	"faq.compatible.answer":
		"Yes! MCPMate works with any MCP-compatible tools and clients, including Claude Desktop, Cherry Studio, Cursor, VSCode, Windsurf, Zed, and more.",
	"faq.upgrade.title": "Can I upgrade my plan later?",
	"faq.upgrade.answer":
		"Yes, you can upgrade from Starter to Professional or Advanced. Your configurations and settings will be preserved during the upgrade.",

	// Notice ribbon (site updating)
	"notice.construction.ribbon": "Updating",
	"notice.construction.tooltip":
		"We’re actively updating; details may be adjusted.",
} as const;

export default en;
