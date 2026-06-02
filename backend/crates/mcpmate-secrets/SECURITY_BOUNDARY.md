# MCPMate Secrets Security Boundary

## Scope

`mcpmate-secrets` provides MCPMate's local server-runtime secret boundary.
It models secret references, resolver/provider contracts, usage references,
and runtime-only injection semantics for managed MCP server configuration.

This crate is part of MCPMate and is licensed under AGPL-3.0-only by default.

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

## Current Desktop UAT Provider

The first desktop UAT implementation uses a local encrypted vault provider.
Secret values are encrypted before they are written to SQLite, while metadata
and usage references remain queryable for Board and API workflows.

The current root-key boundary is intentionally narrow:

- `MCPMATE_SECRETS_LOCAL_KEY` may provide deterministic root material for tests
  or controlled development runs.
- Otherwise MCPMate creates a random local root key at
  `~/.mcpmate/secrets/local-root.key` under the active MCPMate base directory.
- On Unix platforms the local root-key file is created with `0600`
  permissions.

This provider is not an operating-system keychain provider and does not claim
OS-backed custody, hardware-backed custody, or managed-vault custody. It is the
minimum desktop UAT provider behind the replaceable provider boundary. A future
macOS Keychain, Windows Credential Manager, Linux Secret Service, KMS, HSM, or
managed-vault provider must be implemented and documented as a separate
provider, not inferred from this local encrypted vault.

## Audit Boundary

Audit entries may include metadata such as secret id, alias, kind, version,
server id, usage location, operation, and result. Audit entries must not include
plaintext secret values.
