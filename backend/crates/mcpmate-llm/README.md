# mcpmate-llm

`mcpmate-llm` provides MCPMate's local LLM provider domain layer. It owns
provider contracts, provider-specific request adapters, provider lifecycle
orchestration, provider configuration validation, credential-store integration
contracts, repository contracts, and provider-domain events.

This crate is part of MCPMate and is licensed under AGPL-3.0-only by default.

## Responsibilities

This crate is intended to be independently useful to callers outside the main
MCPMate backend crate. It should own LLM-provider business behavior instead of
serving as a thin collection of HTTP client helpers.

The crate owns:

- Provider type parsing and provider capability contracts.
- OpenAI Chat Completions-compatible provider calls.
- Anthropic Messages-compatible provider calls.
- Streaming chat response normalization.
- Model-list and connectivity-test operations.
- Provider default parameter models, including thinking controls.
- Provider create, update, delete, default-provider, test, and model-list
  orchestration.
- Base URL safety validation for provider endpoints.
- Stored-key reuse rules for model preview calls.
- Credential-store contracts for provider API keys.
- Repository contracts for provider persistence.
- Event contracts for provider lifecycle and operational events.

The main MCPMate backend crate should only perform application wiring:

- Expose REST API request/response wrappers.
- Implement the repository trait with MCPMate's SQLite schema.
- Implement the credential-store trait with MCPMate Secure Store.
- Implement or select an event sink.
- Map crate errors into HTTP errors.
- Convert Board/API DTOs into crate domain inputs.

## Module Layout

- `config.rs`: provider type, provider spec, default params, and thinking
  configuration models.
- `types.rs`: chat, tool-call, stream-delta, and token-usage models shared by
  provider implementations.
- `provider.rs`: `LlmProvider` trait and connectivity-test result model.
- `factory.rs`: provider construction from `LlmProviderSpec`.
- `openai.rs`: OpenAI Chat Completions-compatible HTTP adapter.
- `anthropic.rs`: Anthropic Messages-compatible HTTP adapter.
- `manager.rs`: provider lifecycle and operational orchestration.
- `repository.rs`: persistence-port traits and stored-provider records.
- `credentials.rs`: provider API-key custody-port traits and secret-reference
  helpers.
- `events.rs`: provider-domain event types and event-sink trait.
- `error.rs`: crate-level error kind and result types.

## Domain Boundary

`mcpmate-llm` is the owner of LLM-provider behavior. Host crates should avoid
duplicating or bypassing these rules:

- Reject unsupported provider types before persistence or provider calls.
- Validate base URLs before creating, updating, or previewing providers.
- Reject partial secret placeholders in provider API-key fields.
- Resolve provider API keys only through the credential-store port.
- Reuse a saved provider API key for config previews only when provider type
  and normalized base URL match the saved provider.
- Clear provider secret usage before deleting an owned provider secret.
- Clean up newly created owned credentials when provider create/update
  orchestration fails.
- Reset thinking controls when a provider is changed away from a provider type
  that supports them.

If a new caller needs different persistence, credential custody, or event
handling, add a new adapter outside this crate rather than moving business
rules back into the host crate.

## Provider Boundary

Provider implementations translate MCPMate's normalized chat and tool-call
models into each remote API's request/response shape. They should not know
about MCPMate's REST API, SQLite schema, Board DTOs, or Secure Store schema.

Current provider families:

- `openai_chat`: OpenAI Chat Completions-compatible APIs. This adapter calls
  `/chat/completions` and covers OpenAI-compatible providers that implement
  the Chat Completions contract.
- `anthropic`: Anthropic Messages-compatible APIs.

`openai_responses` is modeled as a provider type but is not enabled for
provider management until a dedicated `/responses` implementation and behavior
contract are added. Do not treat `openai_chat` as a Responses API adapter.

## Persistence Boundary

This crate defines `LlmProviderRepository` but does not depend on `sqlx` or
MCPMate's database schema. Repository adapters are responsible for converting
host persistence rows into `StoredLlmProvider` records.

The repository port stores only provider metadata and credential references.
It must not persist plaintext API keys.

## Credential Boundary

This crate defines `LlmCredentialStore` but does not depend on MCPMate Secure
Store directly. Credential adapters own the concrete custody implementation.

The credential port is intentionally narrow:

- Resolve a referenced provider API key for runtime provider calls.
- Verify that an existing secret reference can be used.
- Create an MCPMate-owned provider key when a plaintext API key is supplied.
- Replace provider usage metadata after provider create/update/delete.
- Delete owned provider credentials when they are no longer used.

Provider API keys may be supplied as plaintext for initial storage or as a
whole secret reference such as `[[secret:<alias>]]`. Partial placeholders are
not accepted for provider API-key fields.

## Event Boundary

This crate emits provider-domain events through `LlmProviderEventSink`.
Callers decide where those events go: tracing, audit log, activity log, UI
notifications, tests, or a no-op sink.

Events are intentionally domain-level. They should identify operations and
provider ids, but they must not include plaintext API keys or chat content.

## Runtime and Security Notes

This crate calls remote LLM provider APIs. It does not implement local model
inference and should not reintroduce embedded model runtimes such as local
weights, tokenizers, or inference engines. Local providers should be integrated
through provider-compatible endpoints such as Ollama or LM Studio when needed.

Provider endpoint validation is a safety boundary, not a network sandbox. Host
applications remain responsible for their deployment environment, outbound
network policy, user authorization, and audit retention.

## Agent Guidance

When modifying LLM-provider behavior:

1. Put business-rule changes in `manager.rs`.
2. Put provider protocol changes in the provider adapter module.
3. Put new persistence requirements behind `repository.rs`.
4. Put new credential-custody requirements behind `credentials.rs`.
5. Put new lifecycle/operation notifications behind `events.rs`.
6. Keep REST, Board, SQLite, and Secure Store adapter details outside this
   crate unless they are part of a crate-owned contract.

Do not add silent fallbacks for provider calls, credential resolution,
persistence errors, or unsupported provider types. Surface explicit errors so
callers can decide the appropriate product behavior.
