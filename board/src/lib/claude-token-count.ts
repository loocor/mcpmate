/**
 * Claude token counts using tiktoken/lite (ESM) + Anthropic's published BPE data.
 * Same algorithm as @anthropic-ai/tokenizer, without its CJS `require("tiktoken/lite")`
 * which breaks Vite/esbuild dev pre-bundling (WASM + top-level await).
 *
 * BPE data: Apache-2.0, from anthropics/anthropic-tokenizer-typescript (claude.json).
 */

import { Tiktoken } from "tiktoken/lite";

import claudeTokenizerData from "./vendor/claude-tokenizer.json";

type ClaudeTokenizerJson = {
	pat_str: string;
	special_tokens: Record<string, number>;
	bpe_ranks: string;
};

const data = claudeTokenizerData as ClaudeTokenizerJson;

let tokenizer: Tiktoken | null = null;

function getTokenizer(): Tiktoken {
	if (!tokenizer) {
		tokenizer = new Tiktoken(data.bpe_ranks, data.special_tokens, data.pat_str);
	}
	return tokenizer;
}

/** Count tokens the way Claude tokenization expects (NFKC + tiktoken encode). */
export function countClaudeTokens(text: string | null | undefined): number {
	if (!text) return 0;
	try {
		const t = getTokenizer();
		const encoded = t.encode(text.normalize("NFKC"), "all");
		return encoded.length;
	} catch {
		return Math.ceil(text.length / 4);
	}
}
