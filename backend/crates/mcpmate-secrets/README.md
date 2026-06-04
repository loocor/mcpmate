# mcpmate-secrets

`mcpmate-secrets` provides MCPMate's local server-runtime secret layer. It owns
secret references, runtime placeholder resolution, provider contracts,
OS-backed root-key custody, encrypted local secret storage, usage metadata, and
fail-closed behavior when a secret cannot be resolved.

This crate is part of MCPMate and is licensed under AGPL-3.0-only by default.

## Responsibilities

This crate is intended to remain independently useful to callers outside the
main MCPMate backend crate. It owns the security-sensitive implementation:

- Placeholder and reference models such as `[[secret:<alias>]]`.
- Resolver and store contracts for runtime-only secret injection.
- Root-key provider selection and provider metadata.
- OS-backed root-key custody integrations.
- Explicit development/test root-key provider.
- AEAD encryption/decryption of secret values.
- Per-record data-key generation and root-key wrapping.
- SQLite schema and operations for secret metadata, ciphertext, and usage refs.
- Redacted secret value handling and testing helpers.

The main backend crate should only perform application wiring:

- Choose MCPMate's data directory for explicit development/test key files.
- Start the secure store with the crate-provided default provider.
- Expose REST API request/response wrappers.
- Map MCPMate server configuration fields into usage references.
- Inject the crate-provided resolver into the MCPMate connection pool.

## Module Layout

- `reference.rs`: secret errors, references, values, placeholders, extraction,
  and runtime placeholder resolution.
- `model.rs`: domain models for secret metadata, providers, records, usage
  references, resolver/store traits, and unavailable-provider behavior.
- `types.rs`: crate-facing DTOs used by the local store and backend API
  wrapper.
- `root_key.rs`: root-key providers, OS keyring integration, and explicit
  development/test key material handling.
- `crypto.rs`: envelope encryption and AEAD domain separation.
- `database.rs`: SQLite schema, persistence queries, row mapping, and usage
  metadata queries.
- `store.rs`: `LocalSecretStore` orchestration, cache management, and
  `SecretResolver` implementation.
- `constants.rs`: crate-private persistence and provider identity constants
  shared across modules.
- `testing.rs`: redacted in-memory resolver helper for tests.

Keep constants at the smallest practical scope. Crypto AAD prefixes stay inside
`crypto.rs`, keyring entry defaults stay inside `root_key.rs`, and enum string
mapping stays next to the enum implementations. `constants.rs` is reserved for
cross-module persisted contract values such as encryption algorithm labels and
provider identity strings.

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
- The recommended default secret custody path is operating-system backed on
  every supported desktop platform without requiring paid hosted services.
- Non-OS custody modes are explicit user choices. They must be surfaced with a
  lower security level and must not be selected silently when OS custody fails.

## Cross-Platform OS Custody

MCPMate's no-extra-cost desktop security target is OS-backed root-key custody:

- macOS: macOS Keychain.
- Windows: Windows Credential Manager or DPAPI-backed OS credential storage.
- Linux: OS Secret Service, such as GNOME Keyring or KWallet through a
  standards-compatible provider.

The OS-backed provider owns the root wrapping key used to protect encrypted
local secret records. Each stored secret receives its own generated data key;
the data key encrypts the secret value and is itself wrapped by the OS-backed
root key. SQLite remains the metadata, wrapped-key, nonce, and ciphertext
store; it must not become a plaintext secret store.

If the required OS secret provider is unavailable, locked, or cannot persist
the root key, MCPMate must fail closed with an actionable error. It must not
silently fall back to environment keys, local files, empty values, or plaintext
server configuration. A user may explicitly select a lower security mode from
the MCPMate security settings surface.

## Root Key Provider Modes

`mcpmate-secrets` classifies root-key providers separately from the secret
record encryption engine. The current provider modes are:

- `operating_system`: recommended default. The root wrapping key is held by the
  platform secure-storage provider, such as Keychain, Credential Manager, or
  Secret Service.
- `passphrase`: user-managed local mode. MCPMate stores a generated root key
  wrapped by a key derived from the user's Master Password. This avoids an OS
  keychain dependency, but losing the Master Password can make stored secrets
  unrecoverable.
- `local_file`: basic local protection. MCPMate stores local root material in
  the MCPMate data area. The crate enforces `0600` permissions on Unix-like
  systems; Windows deployments must use a private app data directory with
  appropriate ACLs. This is better than storing plaintext secret values, but it
  is not equivalent to OS custody if an attacker can read the app data
  directory.
- `development`: deterministic or local-file root material for tests and
  controlled development runs.

Provider metadata records both a stable provider kind and a security level so
backend APIs and Board can display the correct UX without interpreting provider
ids by convention.

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

The crate keeps a replaceable provider interface so MCPMate can support
different local or enterprise backends later:

- OS-backed local root key providers.
- Master Password protected local root-key providers.
- Basic local-file root key providers for explicit user opt-in.
- Local development providers for explicit development/test use.
- Enterprise KMS, HSM, or managed-vault providers.
- Future commercial provider integrations under a separate license.

The first implementation slice does not require a standalone daemon. Process
separation can be introduced later behind the provider interface.

## Development Provider

The development implementation uses a local encrypted vault provider. Secret
values are encrypted before they are written to SQLite, while metadata, wrapped
data keys, and usage references remain queryable for Board and API workflows.

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

## Audit Boundary

Audit entries may include metadata such as secret id, alias, kind, version,
server id, usage location, operation, and result. Audit entries must not include
plaintext secret values.
