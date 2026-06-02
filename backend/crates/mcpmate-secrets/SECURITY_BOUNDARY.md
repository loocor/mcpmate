# MCPMate Secrets Security Boundary

## Scope

`mcpmate-secrets` provides MCPMate's local server-runtime secret boundary.
It provides secret references, resolver/provider contracts, OS-backed root-key
providers, encrypted local secure KV storage, usage references, and
runtime-only injection semantics for managed MCP server configuration.

This crate is part of MCPMate and is licensed under AGPL-3.0-only by default.

The crate must remain independently useful to callers outside the main backend
crate. The main backend crate may initialize the store, expose REST handlers,
and connect the resolver to MCPMate's connection pool, but it must not own the
cryptographic storage implementation or OS secure-storage provider logic.

## Security Target

The default target is enterprise-local secret storage:

- Sensitive MCP server parameters are stored as references or placeholders in
  business configuration.
- Plaintext secret material is resolved only for runtime server startup paths.
- API responses, Board views, client config rendering, logs, audit payloads,
  and SQLite business configuration must not expose plaintext secret values.
- Missing secrets, locked providers, invalid references, or decrypt failures
  must fail explicitly. Runtime code must not silently substitute placeholders,
  empty values, defaults, or plaintext fallbacks.
- Production/default secret custody must be operating-system backed on every
  supported desktop platform without requiring paid hosted services.
- A local root-key file is not an acceptable production/default custody
  boundary. It may only be used for tests, controlled development, or explicit
  migration tooling.

## Cross-Platform OS Custody Target

MCPMate's no-extra-cost desktop security target is OS-backed root-key custody:

- macOS: macOS Keychain.
- Windows: Windows Credential Manager or DPAPI-backed OS credential storage.
- Linux: OS Secret Service, such as GNOME Keyring or KWallet through a
  standards-compatible provider.

The OS-backed provider owns the root key used to protect encrypted local secret
records. SQLite remains the metadata and ciphertext store; it must not become a
plaintext secret store.

If the required OS secret provider is unavailable, locked, or cannot persist the
root key, MCPMate must fail closed with an actionable error. It must not
silently fall back to environment keys, local files, empty values, or plaintext
server configuration.

## Runtime Parameter Boundary

The first supported consumers are MCPMate-managed upstream server runtime
configuration fields:

- `stdio`: command, arguments, and environment variable values.
- `streamable_http`: URL and default HTTP header values.

OAuth-derived client secrets, access tokens, and refresh tokens are sensitive
runtime inputs and should move behind the same provider boundary when that
storage path is implemented.

## Cryptographic Boundary

The architecture is FIPS-aligned but does not claim FIPS 140-3 validation,
certification, or compliance.

Allowed public wording:

- "FIPS-aligned architecture."
- "Designed to support FIPS-validated cryptographic providers."
- "FIPS-ready integration boundary" in engineering documentation only, when
  accompanied by this non-validation statement.

Disallowed wording unless a validated module and operating mode are documented:

- "FIPS compliant."
- "FIPS certified."
- "FIPS validated."
- Unqualified "FIPS ready" in product, website, README, or sales copy.

Future FIPS-provider integrations must record:

- Provider name.
- Cryptographic module name and version.
- Certificate number.
- Platform and operating mode.
- Whether the running provider is operating in the validated configuration.

## Provider Boundary

The crate must keep a replaceable provider interface so MCPMate can support
different local or enterprise backends later:

- OS-backed local root key providers.
- Local encrypted vault storage.
- Enterprise KMS, HSM, or managed-vault providers.
- Future commercial provider integrations under a separate license.

The first implementation slice should not require a standalone daemon. Process
separation can be introduced later behind the provider interface.

## Implementation Boundary

`mcpmate-secrets` owns the security-sensitive implementation:

- Root-key provider selection and provider metadata.
- OS-backed root-key custody integrations.
- Explicit development/test root-key provider.
- AEAD encryption/decryption of secret values.
- SQLite schema and operations for secret metadata, ciphertext, and usage refs.
- Secret resolver behavior and redacted secret value handling.

The main backend crate owns MCPMate application wiring only:

- Choosing MCPMate's data directory for explicit development/test key files.
- Starting the secure store with the crate-provided default provider.
- Exposing REST API request/response wrappers.
- Mapping MCPMate server configuration fields into usage references.
- Injecting the crate-provided resolver into the MCPMate connection pool.

## Development Provider

The development implementation uses a local encrypted vault provider.
Secret values are encrypted before they are written to SQLite, while metadata
and usage references remain queryable for Board and API workflows.

The current root-key boundary is intentionally narrow:

- `MCPMATE_SECRETS_LOCAL_KEY` may provide deterministic root material for tests
  or controlled development runs.
- Otherwise the explicit development provider creates a random local root key
  at the caller-provided development key path. The MCPMate backend wrapper uses
  `~/.mcpmate/secrets/local-root.key` under the active MCPMate base directory
  only for that explicit development/test provider.
- On Unix platforms the local root-key file is created with `0600`
  permissions.

This provider is not an operating-system keychain provider and does not claim
OS-backed custody, hardware-backed custody, or managed-vault custody. It is the
minimum development provider behind the replaceable provider boundary. It must
not be the default production secure-store custody path.

A macOS Keychain, Windows Credential Manager or DPAPI, Linux Secret Service,
KMS, HSM, or managed-vault provider must be implemented and documented as a
separate provider, not inferred from this local encrypted vault.

## Audit Boundary

Audit entries may include metadata such as secret id, alias, kind, version,
server id, usage location, operation, and result. Audit entries must not include
plaintext secret values.
