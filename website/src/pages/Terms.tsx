import { useEffect } from "react";
import { useLanguage } from "../components/LanguageProvider";
import Section from "../components/ui/Section";

const Terms = () => {
	const { language } = useLanguage();

	useEffect(() => {
		document.title =
			language === "zh" ? "服务条款 — MCPMate" : "Terms of Service — MCPMate";
	}, [language]);

	// Ensure reading starts at the top whenever opening Terms or changing language
	useEffect(() => {
		window.scrollTo({ top: 0, left: 0, behavior: "auto" });
	}, [language]);

	return (
		<Section className="pt-28">
			<div className="max-w-3xl mx-auto px-4 md:px-0">
				{language === "zh" ? (
					<div>
						<h1 className="text-3xl font-bold mb-4">服务条款</h1>
						<p className="text-slate-600 dark:text-slate-400 mb-8">
							最后更新：2025年10月12日
						</p>

						<div className="space-y-8 leading-relaxed text-slate-800 dark:text-slate-200">
							<p>
								本《服务条款》（“条款”）适用于 MCPMate
								网站与产品（包含桌面应用与代理）（与通过本网站提供的早期访问版本合称“服务”）。
								访问或使用服务即表示您同意本条款。若您从桌面应用跳转至本页面，本条款同样适用于该使用情境。
							</p>

							<div>
								<h2 className="text-xl font-semibold mb-2">1. 概述</h2>
								<p>
									MCPMate 为 Model Context
									Protocol（MCP）工具提供管理与代理层。在早期访问阶段，功能处于试验性，可能随时变更。
									MCPMate 不运营您的 MCP
									工具或基础设施；您对所处环境及接入的第三方服务负责。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">2. 资格</h2>
								<p>
									您应具备签订本条款的完全民事行为能力。若您代表机构使用服务，您声明已获授权并可使该机构受本条款约束。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									3. 软件许可（早期访问）
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										许可：在遵守本条款的前提下，我们授予您可撤销、非排他、不可转让的许可，仅用于个人或内部评估目的安装与使用早期访问软件。
									</li>
									<li>
										限制：除法律允许外，不得对软件进行反向工程、反编译、反汇编，不得移除或绕过技术措施，不得转售或再许可。
									</li>
									<li>
										第三方条款：您接入的第三方 MCP 服务器、AI
										服务或客户端适用其各自条款与政策。
									</li>
									<li>
										变更权利：早期预览阶段功能与数据结构、文件格式、接口返回等均可能调整或迁移，我们保留随版本变更而调整的权利。
									</li>
									<li>
										生产环境建议：不建议用于生产环境。即便经由 MCP 代理，MCPMate
										本身通常不会直接对业务产生影响，但请谨慎评估整体集成风险，
										预览版迭代快速，我们无法就稳定性与兼容性提供确定性承诺。
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">4. 可接受使用</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>不得将服务用于违法、有害或滥用用途。</li>
									<li>不得尝试未授权访问系统或数据。</li>
									<li>妥善保管您环境中配置的凭据、API Key 与 Token。</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">5. 隐私</h2>
								<p>
									我们如何处理网站分析与联系提交，请参见《隐私政策》。使用服务即表示您同意其中所述做法。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">6. 知识产权</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										MCPMate 自主开发：MCPMate
										网站、桌面应用与代理等由我们自主开发并享有相应知识产权与许可权利。
									</li>
									<li>
										第三方知识产权：您接入的第三方 MCP 服务器、AI
										服务与客户端的知识产权归各自权利人所有，您需自行确保具备合法使用权限与合规性。
									</li>
									<li>
										应用商城与注册库：我们内建两个应用来源——
										<a
											className="text-blue-600 dark:text-blue-400 hover:underline"
											href="https://registry.modelcontextprotocol.io/docs"
											target="_blank"
											rel="noopener noreferrer"
										>
											MCP Official Registry
										</a>
										（依据开源许可接入）与
										<a
											className="text-blue-600 dark:text-blue-400 hover:underline"
											href="https://mcpmarket.cn/"
											target="_blank"
											rel="noopener noreferrer"
										>
											MCPMarket.cn
										</a>
										（经官方授权接入）。两个数据源的内容完整性与正确性由各自网站负责，我们不作任何保证；使用前须接受相应许可/条款，若不同意而仍使用，由此产生的责任由您自行承担。
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">7. 反馈</h2>
								<p>
									您向我们提供的建议或反馈，将授予我们全球性的、永久的、不可撤销且免费的使用许可，以便在不对您承担义务的情况下予以采纳。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">8. 免责声明</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										早期访问软件按“现状/可用”提供，不作任何明示、默示或法定保证（包括适销性、特定用途适用性与非侵权）。
									</li>
									<li>
										MCPMate 不保证与所有 MCP
										工具或客户端兼容；功能与数据结构可能在早期访问期间增删或调整。
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">9. 责任限制</h2>
								<p>
									在法律允许的最大范围内，MCPMate
									及其贡献者不就任何间接、附带、特殊、后果性或惩罚性损害，或利润、数据、商誉等损失承担责任。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">10. 第三方服务</h2>
								<p>
									服务可能与第三方产品/服务协同（如 Google
									Analytics、Formspree、MCP 服务器、AI
									模型、客户端等）。我们不控制第三方服务，
									它们受各自条款与政策约束。此外，对
									<a
										className="text-blue-600 dark:text-blue-400 hover:underline"
										href="https://registry.modelcontextprotocol.io/docs"
										target="_blank"
										rel="noopener noreferrer"
									>
										MCP Official Registry
									</a>
									与
									<a
										className="text-blue-600 dark:text-blue-400 hover:underline"
										href="https://mcpmarket.cn/"
										target="_blank"
										rel="noopener noreferrer"
									>
										MCPMarket.cn
									</a>
									中的清单、描述、版本与许可元数据，我们不保证其准确性、完整性或适用性；如需使用，您应核对来源网站并遵循其许可要求。
								</p>
								<p className="text-slate-600 dark:text-slate-400">
									上述链接仅作为外链展示与索引，不构成对任何第三方的背书或保证；我们不对其内容进行修改或筛选，可能存在变更或同步延迟。
									请以来源网站为准并严格遵守相应许可/条款。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">11. 终止</h2>
								<p>
									您可随时停止使用服务。若我们认为您违反本条款或持续访问存在风险，可暂停或终止您的访问。我们可在任何时候变更或终止服务。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">12. 出口与制裁</h2>
								<p>
									您声明并保证自己不在美国制裁适用的国家/地区或受其控制的实体之列，并遵守适用的出口管制与制裁法律。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									13. 适用法律与管辖
								</h2>
								<p>
									本条款受 MCPMate
									主体注册地的法律管辖（不包含其冲突规范）。若您为消费者，您所在法域的强制性消费者保护权利不受本条款影响。
									与本条款相关的争议应提交至该注册地具有管辖权的法院处理，但强制性消费者保护规则另有要求的除外。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">14. 条款变更</h2>
								<p>
									我们可能不时更新本条款；若变更生效后您继续使用服务，即视为接受修订内容。
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">15. 联系方式</h2>
								<p>
									关于本条款的任何问题，请发送邮件至{" "}
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
						<h1 className="text-3xl font-bold mb-4">Terms of Service</h1>
						<p className="text-slate-600 dark:text-slate-400 mb-8">
							Last updated: October 12, 2025
						</p>

						<div className="space-y-8 leading-relaxed text-slate-800 dark:text-slate-200">
							<p>
								These Terms of Service ("Terms") apply to the MCPMate website
								and products (including the desktop app and proxy), together
								with any early‑access downloads provided via this site
								(collectively, the "Services"). By using the Services, you agree
								to these Terms. If you reached this page from the desktop app,
								these Terms also govern that context.
							</p>

							<div>
								<h2 className="text-xl font-semibold mb-2">1. Overview</h2>
								<p>
									MCPMate provides a management and proxy layer for Model
									Context Protocol (MCP) tools. During the early‑access period,
									features are experimental and may change. MCPMate does not
									operate your MCP tools or infrastructure; you are responsible
									for your environment and any third‑party services you choose
									to connect.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">2. Eligibility</h2>
								<p>
									You must have the legal capacity to enter into these Terms. If
									you use the Services on behalf of an organization, you
									represent that you have authority to bind that organization,
									and "you" refers to the organization.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									3. Software License (Early Access)
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										License. Subject to these Terms, we grant you a revocable,
										non‑exclusive, non‑transferable license to install and use
										the early‑access MCPMate software for personal or internal
										evaluation purposes.
									</li>
									<li>
										Restrictions. Except to the extent allowed by law, you may
										not reverse engineer, decompile, or disassemble the
										software, remove or circumvent technical protections, or
										resell or sublicense it.
									</li>
									<li>
										Third‑party Terms. Your use of third‑party MCP servers, AI
										providers, or client apps remains subject to their own terms
										and policies.
									</li>
									<li>
										Right to change: During preview, features and data
										structures (including file formats and API/CLI outputs) may
										change or migrate between versions. We reserve the right to
										make such changes.
									</li>
									<li>
										Production use: Not recommended for production. Even when
										used behind an MCP proxy, MCPMate itself typically does not
										directly impact your business logic, but you should
										carefully evaluate end‑to‑end integration risks. The product
										iterates quickly and we cannot offer stability/compatibility
										guarantees.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									4. Acceptable Use
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										Do not use the Services for unlawful, harmful, or abusive
										activities.
									</li>
									<li>
										Do not attempt to gain unauthorized access to systems or
										data.
									</li>
									<li>
										Protect credentials, API keys, and tokens that you configure
										in your environment.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">5. Privacy</h2>
								<p>
									Our Privacy Policy describes how we handle website analytics
									and contact submissions. By using the Services, you consent to
									the practices described there.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									6. Intellectual Property
								</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										MCPMate ownership: The MCPMate website, desktop app, and
										proxy are developed by us; we own the associated
										intellectual property and licensing rights.
									</li>
									<li>
										Third‑party IP: Third‑party MCP servers, AI providers, and
										client apps are owned by their respective rights holders.
										You are responsible for ensuring you have the rights to use
										them and for complying with their terms.
									</li>
									<li>
										Marketplaces and registries: The product integrates two
										sources —
										<a
											className="text-blue-600 dark:text-blue-400 hover:underline"
											href="https://registry.modelcontextprotocol.io/docs"
											target="_blank"
											rel="noopener noreferrer"
										>
											MCP Official Registry
										</a>
										(ingested under open‑source licenses) and{" "}
										<a
											className="text-blue-600 dark:text-blue-400 hover:underline"
											href="https://mcpmarket.cn/"
											target="_blank"
											rel="noopener noreferrer"
										>
											MCPMarket.cn
										</a>
										(integrated under official authorization). The completeness
										and correctness of listings are the responsibility of the
										respective sites, and we make no warranties. You must accept
										applicable licenses/terms before use; if you choose to use
										without acceptance, you assume all resulting responsibility.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">7. Feedback</h2>
								<p>
									If you choose to provide suggestions or feedback, you grant us
									a non‑exclusive, worldwide, perpetual, irrevocable,
									royalty‑free license to use and incorporate that feedback
									without obligation to you.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">8. Disclaimers</h2>
								<ul className="list-disc pl-6 space-y-2">
									<li>
										Early‑access software is provided “AS IS” and “AS
										AVAILABLE,” with no warranties of any kind, whether express,
										implied, or statutory. We specifically disclaim implied
										warranties of merchantability, fitness for a particular
										purpose, and non‑infringement.
									</li>
									<li>
										MCPMate does not guarantee compatibility with every MCP tool
										or client. Features and data structures may be added,
										changed, or removed during early access.
									</li>
								</ul>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									9. Limitation of Liability
								</h2>
								<p>
									To the maximum extent permitted by law, MCPMate and its
									contributors will not be liable for any indirect, incidental,
									special, consequential, or exemplary damages, or for loss of
									profits, data, goodwill, or other intangible losses, arising
									from or related to your use of the Services.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									10. Third‑Party Services
								</h2>
								<p>
									The Services may interoperate with third‑party products and
									services (e.g., Google Analytics, Formspree, MCP servers, AI
									models, client applications). We do not control and are not
									responsible for third‑party services, which are governed by
									their own terms and policies. For content and metadata
									surfaced from
									<a
										className="text-blue-600 dark:text-blue-400 hover:underline"
										href="https://registry.modelcontextprotocol.io/docs"
										target="_blank"
										rel="noopener noreferrer"
									>
										MCP Official Registry
									</a>{" "}
									and{" "}
									<a
										className="text-blue-600 dark:text-blue-400 hover:underline"
										href="https://mcpmarket.cn/"
										target="_blank"
										rel="noopener noreferrer"
									>
										MCPMarket.cn
									</a>
									, we do not guarantee accuracy, completeness, or suitability;
									verify details with the source sites and comply with their
									licenses.
								</p>
								<p className="text-slate-600 dark:text-slate-400">
									These links are provided as external references only and do
									not constitute endorsement or warranty. We do not modify or
									curate third‑party content, which may change or be delayed;
									rely on the source sites and comply with their licenses/terms.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">11. Termination</h2>
								<p>
									You may stop using the Services at any time. We may suspend or
									terminate your access if we believe you have violated these
									Terms or if continued access poses risk. We may discontinue or
									modify the Services at any time.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									12. Export and Sanctions
								</h2>
								<p>
									You represent that you are not located in, under the control
									of, or a national or resident of any country or entity subject
									to U.S. sanctions. You agree to comply with applicable export
									control and sanctions laws.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									13. Governing Law and Jurisdiction
								</h2>
								<p>
									These Terms are governed by the laws of the jurisdiction in
									which the MCPMate entity is registered, excluding its
									conflict‑of‑law rules. If you are a consumer, your mandatory
									local consumer rights are not affected by this Section. Any
									dispute arising out of or relating to these Terms shall be
									subject to the jurisdiction of the competent courts located in
									that place of registration, except where mandatory consumer
									protection rules require otherwise.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">
									14. Changes to These Terms
								</h2>
								<p>
									We may update these Terms from time to time. The “Last
									updated” date reflects the latest changes. If you continue
									using the Services after changes take effect, you accept the
									revised Terms.
								</p>
							</div>

							<div>
								<h2 className="text-xl font-semibold mb-2">15. Contact</h2>
								<p>
									Questions about these Terms? Email{" "}
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

export default Terms;
