export const accountTranslations = {
	en: {
		title: "Account",
		welcomeSubtitle: "Sign in with your GitHub account",
		signedInTitle: "You're signed in",
		signedInSubtitle: "Your GitHub account is connected to this app.",
		legalNotice:
			"By clicking continue, you agree to our <termsLink>Terms of Service</termsLink> and <privacyLink>Privacy Policy</privacyLink>.",
		localDeviceSection: "This device",
		cloudSignedInFootnote:
			"When backup and sync are ready, you'll see them here first.",
		description:
			"Optional GitHub sign-in for future cloud backup. You can keep using MCPMate fully offline.",
		desktopOnly:
			"Account linking is available in the MCPMate desktop app on macOS.",
		deviceLabel: "Device ID",
		hostLabel: "Device name",
		connect: "Sign in with GitHub",
		disconnect: "Sign out",
		syncSoon:
			"Cross-device backup and sync are not available yet. Signed-in users will get access first.",
		oauthSuccess: "Signed in successfully.",
		oauthFailed: "Sign-in failed",
		oauthErrorInvalidState:
			"Could not verify the sign-in after GitHub (invalid_state). Try the full flow again. If it keeps failing, redeploy the auth worker: OAuth state must be written to KV before redirecting to GitHub (await the KV put).",
	},
	"zh-CN": {
		title: "账户",
		welcomeSubtitle: "使用你的 GitHub 账户登录",
		signedInTitle: "已登录",
		signedInSubtitle: "你的 GitHub 账户已与此应用关联。",
		legalNotice:
			"点击继续，即表示你同意我们的 <termsLink>服务条款</termsLink> 和 <privacyLink>隐私政策</privacyLink>。",
		localDeviceSection: "本机",
		cloudSignedInFootnote: "备份与同步就绪后，将优先在此向你展示。",
		description:
			"可选的 GitHub 登录，用于后续云端备份。你也可以完全离线使用 MCPMate。",
		desktopOnly: "账户相关功能仅在 macOS 桌面版 MCPMate 中提供。",
		deviceLabel: "设备 ID",
		hostLabel: "设备名称",
		connect: "使用 GitHub 登录",
		disconnect: "退出登录",
		syncSoon:
			"跨设备备份与同步尚未开放，上线后将优先向已登录用户开放。",
		oauthSuccess: "登录成功。",
		oauthFailed: "登录失败",
		oauthErrorInvalidState:
			"从 GitHub 返回后会话校验失败（invalid_state）。请重新完整走一遍登录。若仍失败，请重新部署认证 Worker：必须在跳转 GitHub 之前把 OAuth state 写入 KV 并 await 完成（否则回调时读不到 state）。",
	},
	"ja-JP": {
		title: "アカウント",
		welcomeSubtitle: "GitHub アカウントでサインイン",
		signedInTitle: "サインイン済み",
		signedInSubtitle: "GitHub アカウントがこのアプリに連携されています。",
		legalNotice:
			"続行をクリックすると、<termsLink>利用規約</termsLink>および<privacyLink>プライバシーポリシー</privacyLink>に同意したことになります。",
		localDeviceSection: "この端末",
		cloudSignedInFootnote:
			"バックアップと同期の準備ができ次第、ここでお知らせします。",
		description:
			"将来のクラウドバックアップ用の任意の GitHub サインイン。オフラインのみの利用も可能です。",
		desktopOnly:
			"アカウント連携は macOS 版 MCPMate デスクトップアプリでのみ利用できます。",
		deviceLabel: "デバイス ID",
		hostLabel: "デバイス名",
		connect: "GitHub でサインイン",
		disconnect: "サインアウト",
		syncSoon:
			"デバイス間のバックアップと同期はまだ利用できません。リリース時はサインイン済みの方から順にご利用いただけます。",
		oauthSuccess: "サインインに成功しました。",
		oauthFailed: "サインインに失敗しました",
		oauthErrorInvalidState:
			"GitHub から戻った後にセッションを検証できませんでした（invalid_state）。もう一度最初からお試しください。解消しない場合は認証 Worker を再デプロイしてください。GitHub へリダイレクトする前に OAuth state を KV へ書き込み、完了を await する必要があります。",
	},
} as const;
