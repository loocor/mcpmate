import { useEffect } from "react";
import { useLanguage } from "../components/LanguageProvider";
import Section from "../components/ui/Section";

const Privacy = () => {
	const { language } = useLanguage();

	useEffect(() => {
		document.title =
			language === "zh" ? "隐私政策 — MCPMate" : "Privacy Policy — MCPMate";
	}, [language]);

	// Ensure reading starts at the top whenever opening Privacy or changing language
	useEffect(() => {
		window.scrollTo({ top: 0, left: 0, behavior: "auto" });
	}, [language]);

	return (
		<Section className="pt-28">
			<div className="max-w-3xl mx-auto px-4 md:px-0">
				{language === "zh" ? (
					<div>
						<h1 className="text-3xl font-bold mb-4">隐私政策</h1>
						<p className="text-slate-600 dark:text-slate-400 mb-8">
							最后更新：2025年10月12日
						</p>

						<div className="space-y-8 leading-relaxed text-slate-800 dark:text-slate-200">
							<p>
								本隐私政策适用于 MCPMate
								网站与产品（包含桌面应用与代理）（下称“本产品与网站”）的数据处理做法。
								我们力求简单、透明、可控。随着产品接近正式发布（GA），本政策可能更新，届时将同步在本页面标注日期。
								若您从桌面应用跳转至本页面，请知悉本政策同样适用于该应用的使用情境。
							</p>

							<div>
								<h2 className="text-xl font-semibold mb-2">1. 适用范围</h2>
								<p>
									本政策适用于：（a）mcp.umate.ai
									营销网站及其子页面；（b）您从本网站下载的 MCPMate
									桌面端/代理早期访问版本。 本政策不覆盖您接入的第三方 MCP
									服务器、AI 服务商或客户端，它们各自的政策与条款将适用。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									2. 我们不收集的内容
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>不收集您的 MCP 提示、工具调用或模型返回内容。</li>
									<li>
										自托管的 MCPMate
										代理在您的本地/私有环境运行，我们无法访问其运行时内容。
									</li>
									<li>不出售个人信息。</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									3. 我们可能收集的信息
								</h2>
								<h3 className="text-lg font-semibold mt-2 mb-1">
									3.1 网站分析（Analytics）
								</h3>
								<p>
									为改进网站体验，我们使用 Google Analytics
									4（GA4）收集聚合指标（如页面浏览、来源、近似地域、设备/浏览器）。
									GA4 使用第一方 Cookie。我们不会尝试识别单个访问者。
								</p>
								<h3 className="text-lg font-semibold mt-4 mb-1">
									3.2 联系表单
								</h3>
								<p>
									当您提交联系表单时，我们会接收您提供的姓名、邮箱与消息内容，以便回复咨询。提交由第三方
									Formspree 代处理， 成功提交后消息副本可能暂存在浏览器
									localStorage 以提升可靠性。
								</p>
								<h3 className="text-lg font-semibold mt-4 mb-1">
									3.3 下载/按钮点击事件
								</h3>
								<p>
									我们可能记录匿名的下载或按钮点击事件，用于了解不同平台的兴趣程度并改进发布流程。事件不包含工具内容或配置细节。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									4. Cookie 与本地存储
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>GA4 设置的网站分析 Cookie。</li>
									<li>主题与语言偏好存放在 localStorage。</li>
									<li>联系表单成功提交后的消息备份存放在 localStorage。</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									5. 我们如何使用信息
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>运营与改进网站。</li>
									<li>回复您主动提交的咨询。</li>
									<li>基于聚合使用模式规划产品改进。</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">6. 数据保留</h2>
								<p>
									联系消息会在处理咨询与维持合理业务记录所需的期间内保存。网站分析数据的保留遵循
									GA4 属性中的默认设置； 我们不会在营销站点层面延长其保留期。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">7. 第三方服务</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>Google Analytics 4：网站访问分析。</li>
									<li>Formspree：联系表单代收与转发。</li>
								</ul>
								<p>
									这些服务商在我们的委托下处理有限数据，并受其各自的条款与隐私政策约束。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">8. 您的选择</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>您可在浏览器中禁用 Cookie；网站应仍可基本使用。</li>
									<li>
										您可联系到{" "}
										<a
											className="text-blue-600 dark:text-blue-400"
											href="mailto:loocor@gmail.com"
										>
											loocor@gmail.com
										</a>{" "}
										请求删除我们保留的联系记录。
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">9. 安全</h2>
								<p>
									我们对网站层面的少量数据采取合理的管理与技术保护措施，但任何传输或存储方式都无法保证绝对安全。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">10. 未成年人</h2>
								<p>
									本网站与早期访问软件不面向未成年人。如您认为我们误收集了未成年人的个人信息，请联系我们以便删除。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">11. 国际使用</h2>
								<p>
									我们的网站与供应商可能在美国或其他地区处理数据。若您在境外访问，即表示您同意信息被传输至美国并按本政策处理。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">12. 政策更新</h2>
								<p>当实践变化时，我们会更新本页并调整“最后更新”日期。</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">13. 联系</h2>
								<p>
									如有问题或请求，请发送邮件至{" "}
									<a
										className="text-blue-600 dark:text-blue-400"
										href="mailto:loocor@gmail.com"
									>
										loocor@gmail.com
									</a>
									。
								</p>
							</div>
						</div>
					</div>
				) : (
					<div>
						<h1 className="text-3xl font-bold mb-4">Privacy Policy</h1>
						<p className="text-slate-600 dark:text-slate-400 mb-8">
							Last updated: October 12, 2025
						</p>

						<div className="space-y-8 leading-relaxed text-slate-800 dark:text-slate-200">
							<p>
								This Privacy Policy applies to the MCPMate website and products
								(including the desktop app and proxy) (collectively, “Products
								and Site”). We keep practices simple, transparent, and minimal.
								As we approach GA, we will update this page to reflect changes.
								If you reached this page from the desktop app, this Policy also
								applies to that context.
							</p>

							<div>
								<h2 className="text-xl font-semibold mb-2">1. Scope</h2>
								<p>
									This policy covers: (a) the marketing site at mcp.umate.ai and
									subpages; and (b) optional early‑access builds of the MCPMate
									desktop and proxy software you download from this site. It
									does not cover third‑party MCP servers, AI providers, or
									client apps; their policies apply.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									2. What We Do Not Collect
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										We do not collect your MCP prompts, tool calls, or model
										outputs.
									</li>
									<li>
										Self‑hosted MCPMate proxies run in your environment; we do
										not operate or access them.
									</li>
									<li>We do not sell personal information.</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									3. Information We Collect
								</h2>
								<h3 className="text-lg font-semibold mt-2 mb-1">
									3.1 Website analytics
								</h3>
								<p>
									We use Google Analytics 4 (GA4) to understand aggregate usage
									(page views, referrers, approximate geography,
									device/browser). GA4 uses first‑party cookies. We do not
									attempt to identify individuals.
								</p>
								<h3 className="text-lg font-semibold mt-4 mb-1">
									3.2 Contact form
								</h3>
								<p>
									If you submit the contact form, we process the details you
									provide (name, email, message) to reply. Submissions are
									handled via Formspree and may also be temporarily stored in
									your browser’s localStorage as a reliability backup.
								</p>
								<h3 className="text-lg font-semibold mt-4 mb-1">
									3.3 Download events
								</h3>
								<p>
									We may record anonymous download or CTA click events to gauge
									interest across platforms and improve releases. These events
									do not include tool content or configuration details.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									4. Cookies and Local Storage
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>Analytics cookies set by GA4.</li>
									<li>User preferences (theme, language) in localStorage.</li>
									<li>
										Contact message backup in localStorage after successful
										submit.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									5. How We Use Information
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>Operate and improve the website.</li>
									<li>Respond to inquiries you send us.</li>
									<li>
										Plan product improvements based on aggregated patterns.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									6. Data Retention
								</h2>
								<p>
									We retain contact messages as needed to address your request
									and maintain reasonable business records. Analytics data is
									retained per GA4 defaults; we do not extend retention for the
									site.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									7. Third‑Party Services
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>Google Analytics 4 for website analytics.</li>
									<li>Formspree for contact form processing.</li>
								</ul>
								<p>
									These providers process limited data on our behalf and are
									subject to their own terms and policies.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">8. Your Choices</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										You can disable cookies in your browser; the site should
										remain functional.
									</li>
									<li>
										You can email us to request deletion of contact messages we
										hold about you.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">9. Security</h2>
								<p>
									We apply reasonable administrative and technical safeguards
									for the small amount of data we handle through the site, but
									no method of transmission or storage is completely secure.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									10. Children’s Privacy
								</h2>
								<p>
									The site and early‑access software are not directed to
									children. If you believe we collected personal information
									from a child, contact us and we will delete it.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									11. International Use
								</h2>
								<p>
									Our website infrastructure and vendors may process data in the
									United States and elsewhere. If you access the site from
									outside the U.S., you consent to such transfer and processing
									consistent with this Policy.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									12. Changes to This Policy
								</h2>
								<p>
									We will update this page as our practices evolve; see the date
									above.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">13. Contact</h2>
								<p>
									Questions or requests? Email{" "}
									<a
										className="text-blue-600 dark:text-blue-400"
										href="mailto:loocor@gmail.com"
									>
										loocor@gmail.com
									</a>
									.
								</p>
							</div>
						</div>
					</div>
				)}
			</div>
		</Section>
	);
};

export default Privacy;
