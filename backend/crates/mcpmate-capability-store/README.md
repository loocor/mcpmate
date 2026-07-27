# MCPMate Capability Store

> Design authority notice (2026-07-25): the reviewed
> [Chinese closure draft](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.zh-CN.md)
> is authoritative for this implementation. The English architecture document remains its
> synchronization target.

`mcpmate-capability-store` owns MCPMate's durable capability catalog and bounded derived caches.

The current crate persists the latest typed capability snapshot for each server, per-kind
observation state, catalog revisions, and LRU-backed derived projections. The approved target
model extends this foundation with:

- stable `CapabilityRef` intent anchors;
- immutable, content-addressed `CapabilityId` versions;
- immutable, content-addressed `SurfaceManifest` records;
- separate `SurfaceProposal`, consumer-scoped review-item, and `SurfacePublication` records;
- atomic consumer-to-active-publication bindings;
- durable reconciliation jobs, outbox records, change classification, review, and rollback
  evidence.

The authoritative review architecture is documented in
[Capability 身份与 Surface Manifest](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.zh-CN.md).
The English synchronization target is
[Capability Identity and Surface Manifest](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.md).

## Ownership Boundary

This crate should own:

- capability identity primitives and canonicalization;
- SQLite persistence for observations, references, versions, and manifests;
- transactional catalog and publication contracts;
- bounded raw-snapshot and projection caches;
- storage-level contract and scale tests.

The application crate should own:

- upstream MCP discovery and external name or URI projection;
- profile, custom-profile, and direct-exposure authoring;
- consumer identity resolution;
- materialization policy and review authorization;
- runtime list and call adapters;
- management API, Board presentation, audit events, and Inspector evidence.

## Identity Summary

```text
Verified Consumer Credential or Trusted Local Binding -> ConsumerAccessContext -> ConsumerId
ConsumerId -> active SurfacePublication -> immutable SurfaceManifest
SurfaceProposal -> proposed SurfaceManifest + per-Ref Review Items
SurfaceManifest -> pinned CapabilityId entries
CapabilityRef -> current CapabilityId
CapabilityId -> canonical effective capability record
```

Profiles and capability-level direct exposure persist `CapabilityRef`. They do not persist
content hashes. The runtime exposes only capabilities pinned by the consumer's active
publication. Sessions, peers, and `clientInfo` are not durable consumer identities.

## Current Implementation Note

This branch implements the identity and publication layers described above. Catalog observation
persists stable Refs and immutable content versions; management saves and catalog reconciliation
compile immutable manifests, publish consumer-scoped bindings, and retain review, job, outbox, and
rollback evidence. Runtime list and invocation paths resolve the same active publication.

MCP `2026-07-28` cache hints and `rmcp 3.x` stateful/stateless protocol adaptation remain future
adapter work. They are not current crate capabilities and do not define Consumer identity,
Capability identity, or publication lifecycle.
