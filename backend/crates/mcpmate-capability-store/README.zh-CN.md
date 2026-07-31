# MCPMate Capability Store

> 当前实现以已经复审的中文收口稿
> [Capability 身份与 Surface Manifest](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.zh-CN.md)
> 为设计权威；英文设计稿是其同步目标。

`mcpmate-capability-store` 负责 MCPMate 的持久 Capability Catalog 和有界 Derived Cache。

当前 Crate 已经持久保存每个 Server 的最新类型化 Capability Snapshot、Per-Kind Observation State、
Catalog Revision，并提供基于 LRU 的 Derived Projection。已经批准的目标模型将在此基础上增加：

- 稳定的 `CapabilityRef` 意图锚点；
- 不可变、Content-Addressed 的 `CapabilityId` 版本；
- 不可变、Content-Addressed 的 `SurfaceManifest`；
- 分离的 `SurfaceProposal`、Consumer-Scoped Review Item 与 `SurfacePublication`；
- Consumer 到 Active Publication 的原子 Binding；
- 持久化 Reconciliation Job、Outbox、Change Classification、Review 和 Rollback Evidence。

权威架构规范见：

- [Capability Identity and Surface Manifest](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.md)
- [Capability 身份与 Surface Manifest](https://github.com/loocor/mcpmate-docs/blob/main/architecture/capability-identity-and-surface-manifest.zh-CN.md)

## Ownership Boundary

本 Crate 负责：

- Capability Identity Primitive 与 Canonicalization；
- Observation、Ref、Version 和 Manifest 的 SQLite Persistence；
- Transactional Catalog 与 Publication Contract；
- Bounded Raw Snapshot 与 Projection Cache；
- Storage-Level Contract Test 与 Scale Test。

Application Crate 负责：

- Upstream MCP Discovery 与 External Name/URI Projection；
- Profile、Custom Profile 与 Direct Exposure Authoring；
- Consumer Identity Resolution；
- Materialization Policy 与 Review Authorization；
- Runtime List 与 Call Adapter；
- Management API、Board Presentation、Audit Event 与 Inspector Evidence。

## Identity Summary

```text
Verified Consumer Credential or Trusted Local Binding -> ConsumerAccessContext -> ConsumerId
ConsumerId -> active SurfacePublication -> immutable SurfaceManifest
SurfaceProposal -> proposed SurfaceManifest + per-Ref Review Items
SurfaceManifest -> pinned CapabilityId entries
CapabilityRef -> current CapabilityId
CapabilityId -> canonical effective capability record
```

Profile 和 Capability Level Direct Exposure 持久关联 `CapabilityRef`，不持久关联 Content Hash。
Runtime 只暴露 Consumer Active Publication 所指向 Manifest 固定的 Capability。Session、Peer 和
`clientInfo` 不能作为持久 Consumer 身份。

## 当前实现说明

本分支已经实现上述身份与发布层：Catalog Observation 持久化稳定 Ref 与不可变内容版本；
Management Save 和 Catalog Reconciliation 编制不可变 Manifest，发布 Consumer-Scoped Binding，
并保留 Review、Job、Outbox 与 Rollback Evidence；Runtime List 和 Invocation 从同一 Active
Publication 读取。

MCP `2026-07-28` Cache Hint 与 `rmcp 3.x` 有状态/无状态协议适配仍属于后续 Adapter 工作，不是当前
Crate 能力，也不参与 Consumer 身份、Capability 身份或 Publication Lifecycle 定义。
