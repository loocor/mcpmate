import { ExternalLink, Github, MessagesSquare } from "lucide-react";
import { trackMCPMateEvents } from "../../utils/analytics";

type DocsLocale = "en" | "zh" | "ja";

type CommunityLinksProps = {
	locale: DocsLocale;
};

const DISCORD_URL = "https://discord.gg/pc5YfEVbKj";
const FEISHU_URL =
	"https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=bd4hb1f5-7dd8-4e89-9e83-103364a81fbf&qr_code=true";
const ISSUES_URL = "https://github.com/loocor/MCPMate/issues";
const DISCUSSIONS_URL = "https://github.com/loocor/MCPMate/discussions";

const copy = {
	en: {
		discord: ["Discord", "Chat with the community, get support, and follow product updates."],
		issues: ["GitHub Issues", "Report a bug or request a feature."],
		discussions: ["GitHub Discussions", "Ask questions and share ideas with maintainers and users."],
	},
	zh: {
		feishu: ["飞书社区", "加入中文用户社区，获取支持、使用技巧和产品动态。"],
		discord: ["Discord", "也可加入国际社区，与更多 MCPMate 用户交流。"],
		issues: ["GitHub Issues", "报告问题或提出功能建议。"],
		discussions: ["GitHub Discussions", "提问、分享想法并参与产品讨论。"],
	},
	ja: {
		discord: ["Discord", "コミュニティで相談し、サポートや製品情報を受け取れます。"],
		issues: ["GitHub Issues", "不具合の報告や機能の提案に利用できます。"],
		discussions: ["GitHub Discussions", "質問やアイデアを開発者やユーザーと共有できます。"],
	},
} as const;

type CommunityCard = {
	title: string;
	description: string;
	href: string;
	icon: typeof MessagesSquare;
};

function getChatCard(locale: DocsLocale): CommunityCard {
	if (locale === "zh") {
		return {
			title: copy.zh.feishu[0],
			description: copy.zh.feishu[1],
			href: FEISHU_URL,
			icon: MessagesSquare,
		};
	}

	return {
		title: copy[locale].discord[0],
		description: copy[locale].discord[1],
		href: DISCORD_URL,
		icon: MessagesSquare,
	};
}

export default function CommunityLinks({ locale }: CommunityLinksProps): JSX.Element {
	const localeCopy = copy[locale];
	const cards: CommunityCard[] = [
		getChatCard(locale),
		{
			title: localeCopy.issues[0],
			description: localeCopy.issues[1],
			href: ISSUES_URL,
			icon: Github,
		},
		{
			title: localeCopy.discussions[0],
			description: localeCopy.discussions[1],
			href: DISCUSSIONS_URL,
			icon: MessagesSquare,
		},
	];

	return (
		<div className="not-prose space-y-3">
			<div className="grid gap-3 md:grid-cols-3">
				{cards.map((card) => {
					const Icon = card.icon;
					return (
						<a
							key={card.href}
							href={card.href}
							target="_blank"
							rel="noopener noreferrer"
							onClick={() => trackMCPMateEvents.externalLinkClick(card.href)}
							className="group rounded-lg border border-brand-border bg-brand-elevated p-4 transition-colors hover:border-brand-accent/60 hover:bg-brand-overlay"
						>
							<div className="mb-3 flex items-center justify-between gap-3">
								<Icon className="h-5 w-5 text-brand-accent" aria-hidden />
								<ExternalLink className="h-4 w-4 text-brand-muted-soft group-hover:text-brand-accent" aria-hidden />
							</div>
							<div className="font-semibold text-brand-foreground">{card.title}</div>
							<p className="mt-1 text-sm leading-6 text-brand-muted">{card.description}</p>
						</a>
					);
				})}
			</div>

			{locale === "zh" ? (
				<p className="text-sm text-brand-muted">
					国际用户也可以加入{" "}
					<a
						href={DISCORD_URL}
						target="_blank"
						rel="noopener noreferrer"
						onClick={() => trackMCPMateEvents.externalLinkClick(DISCORD_URL)}
						className="font-medium text-brand-accent hover:underline"
					>
						Discord 社区
					</a>
					。
				</p>
			) : null}
		</div>
	);
}
