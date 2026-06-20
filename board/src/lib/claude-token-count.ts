/**
 * Claude token count using Anthropic's published BPE via tiktoken/lite.
 *
 * tiktoken is loaded via dynamic import() so the WASM fetch never blocks the
 * main module graph. If the WASM fails to load (e.g. inside Tauri's custom
 * protocol webview on macOS/Linux), countClaudeTokens gracefully falls back
 * to gpt-tokenizer (cl100k_base) so token estimates still work.
 */

import { encode as encodeCl100k } from "gpt-tokenizer";
import type { Tiktoken } from "tiktoken/lite";
import claudeTokenizerData from "./vendor/claude-tokenizer.json";

let claudeEncoder: Tiktoken | null = null;
let initialized = false;

/**
 * Lazily initialize the Claude tiktoken encoder in the background.
 * This is fire-and-forget: it never throws and never blocks the caller.
 */
function initClaudeTokenizer(): void {
	if (initialized) return;
	initialized = true;

	import("tiktoken/lite")
		.then(({ Tiktoken: TiktokenCls }) => {
			claudeEncoder = new TiktokenCls(
				claudeTokenizerData.bpe_ranks,
				claudeTokenizerData.special_tokens as Record<string, number>,
				claudeTokenizerData.pat_str,
			);
		})
		.catch(() => {
			// WASM unavailable (e.g. Tauri webview) – fall back to gpt-tokenizer silently.
		});
}

// Kick off background initialization as soon as this module is loaded.
initClaudeTokenizer();

/**
 * Count tokens for Claude content.
 *
 * Returns Anthropic BPE tokens when the WASM encoder is ready, otherwise
 * falls back to cl100k_base (gpt-tokenizer) which provides a credible estimate.
 */
export function countClaudeTokens(text: string | null | undefined): number {
	if (!text) return 0;
	const normalized = text.normalize("NFKC");
	try {
		if (claudeEncoder) {
			return claudeEncoder.encode(normalized, "all").length;
		}
		return encodeCl100k(normalized).length;
	} catch {
		return Math.ceil(text.length / 4);
	}
}
