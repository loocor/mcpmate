/**
 * Token estimation utilities for MCPMate.
 *
 * Uses gpt-tokenizer for accurate token counting based on cl100k_base
 * encoding (GPT-4/3.5-turbo). This provides credible token estimates
 * that match what users would see in actual LLM contexts.
 */

import { encode, decode, encodeChat } from 'gpt-tokenizer';

/**
 * Count tokens in a text string using cl100k_base encoding.
 * This matches GPT-4 and GPT-3.5-turbo tokenization.
 */
export function countTokens(text: string | undefined | null): number {
  if (!text) return 0;
  try {
    return encode(text).length;
  } catch {
    // Fallback to approximation if encoding fails
    return Math.ceil(text.length / 4);
  }
}

/**
 * Count tokens for a chat message format.
 * Useful for estimating prompt token costs.
 */
export function countChatTokens(messages: Array<{ role: string; content: string }>): number {
  try {
    return encodeChat(messages as Parameters<typeof encodeChat>[0]).length;
  } catch {
    // Fallback: sum individual message tokens
    return messages.reduce((sum, msg) => {
      return sum + countTokens(msg.role) + countTokens(msg.content) + 4; // +4 for message overhead
    }, 3); // +3 for reply priming
  }
}

/**
 * Token estimates for MCP capability metadata.
 * These are baseline estimates for the structural overhead of each capability type,
 * not including the actual content (description, schema, etc.).
 */
export const CAPABILITY_BASE_TOKENS = {
  /** Tool: name + inputSchema structure overhead (~20 tokens) */
  tool: 20,
  /** Prompt: name + arguments structure overhead (~15 tokens) */
  prompt: 15,
  /** Resource: URI structure overhead (~10 tokens) */
  resource: 10,
  /** ResourceTemplate: URI template structure overhead (~8 tokens) */
  resourceTemplate: 8,
} as const;

/**
 * Estimate tokens for a tool definition.
 * Includes name, description, and input schema.
 */
export function estimateToolTokens(params: {
  name: string;
  description?: string | null;
  inputSchema?: unknown;
}): number {
  const { name, description, inputSchema } = params;
  let tokens = CAPABILITY_BASE_TOKENS.tool;

  tokens += countTokens(name);
  tokens += countTokens(description);

  if (inputSchema) {
    try {
      const schemaStr = typeof inputSchema === 'string'
        ? inputSchema
        : JSON.stringify(inputSchema);
      tokens += countTokens(schemaStr);
    } catch {
      // Ignore serialization errors
    }
  }

  return tokens;
}

/**
 * Estimate tokens for a prompt definition.
 */
export function estimatePromptTokens(params: {
  name: string;
  description?: string | null;
  arguments?: unknown;
}): number {
  const { name, description, arguments: args } = params;
  let tokens = CAPABILITY_BASE_TOKENS.prompt;

  tokens += countTokens(name);
  tokens += countTokens(description);

  if (args) {
    try {
      const argsStr = typeof args === 'string' ? args : JSON.stringify(args);
      tokens += countTokens(argsStr);
    } catch {
      // Ignore serialization errors
    }
  }

  return tokens;
}

/**
 * Estimate tokens for a resource definition.
 */
export function estimateResourceTokens(params: {
  uri: string;
  name?: string | null;
  mimeType?: string | null;
  description?: string | null;
}): number {
  const { uri, name, mimeType, description } = params;
  let tokens = CAPABILITY_BASE_TOKENS.resource;

  tokens += countTokens(uri);
  tokens += countTokens(name);
  tokens += countTokens(mimeType);
  tokens += countTokens(description);

  return tokens;
}

/**
 * Estimate tokens for a resource template definition.
 */
export function estimateResourceTemplateTokens(params: {
  uriTemplate: string;
  name?: string | null;
  mimeType?: string | null;
  description?: string | null;
}): number {
  const { uriTemplate, name, mimeType, description } = params;
  let tokens = CAPABILITY_BASE_TOKENS.resourceTemplate;

  tokens += countTokens(uriTemplate);
  tokens += countTokens(name);
  tokens += countTokens(mimeType);
  tokens += countTokens(description);

  return tokens;
}

/**
 * Capability token estimates for a profile.
 */
export interface CapabilityTokenEstimate {
  /** Total tokens for all capabilities (all servers) */
  totalTokens: number;
  /** Tokens for enabled capabilities in this profile */
  enabledTokens: number;
  /** Tokens for disabled capabilities (savings) */
  savedTokens: number;
  /** Percentage of tokens saved (0-100) */
  savedPercentage: number;
  /** Breakdown by capability type */
  breakdown: {
    tools: CapabilityTypeBreakdown;
    prompts: CapabilityTypeBreakdown;
    resources: CapabilityTypeBreakdown;
    resourceTemplates: CapabilityTypeBreakdown;
  };
}

export interface CapabilityTypeBreakdown {
  /** Total count across all servers */
  total: number;
  /** Enabled count in profile */
  enabled: number;
  /** Disabled count (savings) */
  saved: number;
  /** Total tokens for all items */
  totalTokens: number;
  /** Tokens for enabled items */
  enabledTokens: number;
  /** Tokens for disabled items (savings) */
  savedTokens: number;
}

type TokenEstimateRow = { enabled: boolean; tokens: number };

function sumTokensFromRows(
  tokenData: TokenEstimateRow[] | undefined,
  totalCount: number,
  enabledCount: number,
  avgPerItem: number,
): { totalTokens: number; enabledTokens: number } {
  if (tokenData?.length) {
    return {
      totalTokens: tokenData.reduce((sum, row) => sum + row.tokens, 0),
      enabledTokens: tokenData
        .filter((row) => row.enabled)
        .reduce((sum, row) => sum + row.tokens, 0),
    };
  }
  return {
    totalTokens: totalCount * avgPerItem,
    enabledTokens: enabledCount * avgPerItem,
  };
}

/**
 * Calculate token estimates using precise tokenization.
 * When detailed capability data is not available, falls back to average estimates.
 */
export function calculateCapabilityTokenEstimate(params: {
  /** Total tool count across all servers */
  totalTools: number;
  /** Enabled tool count in profile */
  enabledTools: number;
  /** Total prompt count across all servers */
  totalPrompts: number;
  /** Enabled prompt count in profile */
  enabledPrompts: number;
  /** Total resource count across all servers */
  totalResources: number;
  /** Enabled resource count in profile */
  enabledResources: number;
  /** Total resource template count across all servers */
  totalResourceTemplates: number;
  /** Enabled resource template count in profile */
  enabledResourceTemplates: number;
  /** Optional: precise tool token estimates (if available) */
  toolsTokenData?: Array<{ enabled: boolean; tokens: number }>;
  /** Optional: precise prompt token estimates (if available) */
  promptsTokenData?: Array<{ enabled: boolean; tokens: number }>;
  /** Optional: precise resource token estimates (if available) */
  resourcesTokenData?: Array<{ enabled: boolean; tokens: number }>;
  /** Optional: precise resource template token estimates (if available) */
  resourceTemplatesTokenData?: Array<{ enabled: boolean; tokens: number }>;
}): CapabilityTokenEstimate {
  const {
    totalTools,
    enabledTools,
    totalPrompts,
    enabledPrompts,
    totalResources,
    enabledResources,
    totalResourceTemplates,
    enabledResourceTemplates,
    toolsTokenData,
    promptsTokenData,
    resourcesTokenData,
    resourceTemplatesTokenData,
  } = params;

  // Average tokens per capability (fallback when precise data not available)
  // These are realistic averages based on typical MCP definitions
  const AVG_TOKENS = {
    tool: 150,      // Tool with name + description + simple schema
    prompt: 80,     // Prompt with name + description
    resource: 40,   // Resource with URI + metadata
    resourceTemplate: 35,
  };

  const tools = sumTokensFromRows(toolsTokenData, totalTools, enabledTools, AVG_TOKENS.tool);
  const toolsTotalTokens = tools.totalTokens;
  const toolsEnabledTokens = tools.enabledTokens;
  const toolsSavedTokens = toolsTotalTokens - toolsEnabledTokens;

  const prompts = sumTokensFromRows(
    promptsTokenData,
    totalPrompts,
    enabledPrompts,
    AVG_TOKENS.prompt,
  );
  const promptsTotalTokens = prompts.totalTokens;
  const promptsEnabledTokens = prompts.enabledTokens;
  const promptsSavedTokens = promptsTotalTokens - promptsEnabledTokens;

  const resources = sumTokensFromRows(
    resourcesTokenData,
    totalResources,
    enabledResources,
    AVG_TOKENS.resource,
  );
  const resourcesTotalTokens = resources.totalTokens;
  const resourcesEnabledTokens = resources.enabledTokens;
  const resourcesSavedTokens = resourcesTotalTokens - resourcesEnabledTokens;

  const templates = sumTokensFromRows(
    resourceTemplatesTokenData,
    totalResourceTemplates,
    enabledResourceTemplates,
    AVG_TOKENS.resourceTemplate,
  );
  const templatesTotalTokens = templates.totalTokens;
  const templatesEnabledTokens = templates.enabledTokens;
  const templatesSavedTokens = templatesTotalTokens - templatesEnabledTokens;

  // Totals
  const totalTokens = toolsTotalTokens + promptsTotalTokens + resourcesTotalTokens + templatesTotalTokens;
  const enabledTokens = toolsEnabledTokens + promptsEnabledTokens + resourcesEnabledTokens + templatesEnabledTokens;
  const savedTokens = totalTokens - enabledTokens;
  const savedPercentage = totalTokens > 0 ? Math.round((savedTokens / totalTokens) * 100) : 0;

  return {
    totalTokens,
    enabledTokens,
    savedTokens,
    savedPercentage,
    breakdown: {
      tools: {
        total: totalTools,
        enabled: enabledTools,
        saved: totalTools - enabledTools,
        totalTokens: toolsTotalTokens,
        enabledTokens: toolsEnabledTokens,
        savedTokens: toolsSavedTokens,
      },
      prompts: {
        total: totalPrompts,
        enabled: enabledPrompts,
        saved: totalPrompts - enabledPrompts,
        totalTokens: promptsTotalTokens,
        enabledTokens: promptsEnabledTokens,
        savedTokens: promptsSavedTokens,
      },
      resources: {
        total: totalResources,
        enabled: enabledResources,
        saved: totalResources - enabledResources,
        totalTokens: resourcesTotalTokens,
        enabledTokens: resourcesEnabledTokens,
        savedTokens: resourcesSavedTokens,
      },
      resourceTemplates: {
        total: totalResourceTemplates,
        enabled: enabledResourceTemplates,
        saved: totalResourceTemplates - enabledResourceTemplates,
        totalTokens: templatesTotalTokens,
        enabledTokens: templatesEnabledTokens,
        savedTokens: templatesSavedTokens,
      },
    },
  };
}

/**
 * Format token count for display.
 * - Under 1000: show as-is (e.g., "500")
 * - 1000-999999: show as K (e.g., "12.5K")
 * - 1000000+: show as M (e.g., "1.2M")
 */
export function formatTokenCount(tokens: number): string {
  if (tokens < 1000) {
    return tokens.toString();
  }
  if (tokens < 1000000) {
    const k = tokens / 1000;
    return k >= 10 ? `${Math.round(k)}K` : `${k.toFixed(1)}K`;
  }
  const m = tokens / 1000000;
  return m >= 10 ? `${Math.round(m)}M` : `${m.toFixed(1)}M`;
}

/**
 * Token savings data for a profile over time.
 */
export interface TokenSavingsStats {
  /** Profile ID */
  profileId: string;
  /** Profile name */
  profileName: string;
  /** Total calls made through this profile */
  totalCalls: number;
  /** Token savings estimate per call */
  tokensSavedPerCall: number;
  /** Cumulative tokens saved */
  cumulativeTokensSaved: number;
  /** Savings percentage */
  savingsPercentage: number;
  /** Capability breakdown at time of calculation */
  capabilityEstimate: CapabilityTokenEstimate;
}

// Re-export for advanced usage
export { encode, decode, encodeChat };
