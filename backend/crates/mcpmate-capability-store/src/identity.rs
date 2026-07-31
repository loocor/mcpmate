use std::{fmt, str::FromStr};

use rmcp::model::{Prompt, Resource, ResourceTemplate, Tool};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::{CapabilityKind, CapabilityPayload, CatalogError, Result};

pub const BUILTIN_CAPABILITY_SOURCE_ID: &str = "mcpmate://builtin/core";
pub const CAPABILITY_REF_FORMAT_V1: &str = "mcpmate.capability-ref.v1";
pub const EFFECTIVE_CAPABILITY_FORMAT_V1: &str = "mcpmate.effective-capability.v1";
pub const SURFACE_MANIFEST_FORMAT_V1: &str = "mcpmate.surface-manifest.v1";

const CAPABILITY_REF_ID_PREFIX: &str = "cref_sha256:";
const CAPABILITY_ID_PREFIX: &str = "cap_sha256:";
const SURFACE_MANIFEST_ID_PREFIX: &str = "surf_sha256:";
const SHA256_HEX_LENGTH: usize = 64;

macro_rules! typed_digest_id {
    ($name:ident, $prefix:expr, $kind:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }

            fn from_digest(digest: impl AsRef<[u8]>) -> Self {
                Self(format!("{}{:x}", $prefix, Sha256::digest(digest)))
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(
                &self,
                formatter: &mut fmt::Formatter<'_>,
            ) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = CatalogError;

            fn from_str(value: &str) -> Result<Self> {
                let Some(digest) = value.strip_prefix($prefix) else {
                    return Err(CatalogError::InvalidIdentity {
                        identity_kind: $kind,
                        value: value.to_string(),
                    });
                };
                if digest.len() != SHA256_HEX_LENGTH
                    || !digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(CatalogError::InvalidIdentity {
                        identity_kind: $kind,
                        value: value.to_string(),
                    });
                }
                Ok(Self(value.to_string()))
            }
        }
    };
}

typed_digest_id!(CapabilityRefId, CAPABILITY_REF_ID_PREFIX, "capability ref");
typed_digest_id!(CapabilityId, CAPABILITY_ID_PREFIX, "capability");
typed_digest_id!(SurfaceManifestId, SURFACE_MANIFEST_ID_PREFIX, "surface manifest");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySourceIdentity {
    pub server_id: String,
    pub kind: CapabilityKind,
    pub origin_key: String,
}

impl CapabilitySourceIdentity {
    pub fn new(
        server_id: impl Into<String>,
        kind: CapabilityKind,
        origin_key: impl Into<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            kind,
            origin_key: origin_key.into(),
        }
    }

    pub fn from_payload(
        server_id: impl Into<String>,
        payload: &CapabilityPayload,
    ) -> Self {
        Self::new(server_id, payload.kind(), exact_origin_key(payload))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityRefIdentityV1<'a> {
    format: &'static str,
    server_id: &'a str,
    kind: CapabilityKind,
    origin_key: &'a str,
}

impl CapabilityRefId {
    pub fn derive(source: &CapabilitySourceIdentity) -> Result<Self> {
        let identity = CapabilityRefIdentityV1 {
            format: CAPABILITY_REF_FORMAT_V1,
            server_id: &source.server_id,
            kind: source.kind,
            origin_key: &source.origin_key,
        };
        Ok(Self::from_digest(serde_json_canonicalizer::to_vec(&identity)?))
    }

    pub fn verify_source(
        &self,
        source: &CapabilitySourceIdentity,
    ) -> Result<()> {
        let derived = Self::derive(source)?;
        if derived == *self {
            Ok(())
        } else {
            Err(CatalogError::IntegrityMismatch {
                identity: self.to_string(),
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EffectiveCapabilityDefinition {
    Tool(Tool),
    Prompt(Prompt),
    Resource(Resource),
    ResourceTemplate(ResourceTemplate),
}

impl EffectiveCapabilityDefinition {
    pub const fn kind(&self) -> CapabilityKind {
        match self {
            Self::Tool(_) => CapabilityKind::Tools,
            Self::Prompt(_) => CapabilityKind::Prompts,
            Self::Resource(_) => CapabilityKind::Resources,
            Self::ResourceTemplate(_) => CapabilityKind::ResourceTemplates,
        }
    }

    pub fn external_key(&self) -> String {
        match self {
            Self::Tool(value) => value.name.to_string(),
            Self::Prompt(value) => value.name.to_string(),
            Self::Resource(value) => value.uri.to_string(),
            Self::ResourceTemplate(value) => value.uri_template.to_string(),
        }
    }

    pub fn into_payload(self) -> CapabilityPayload {
        match self {
            Self::Tool(value) => CapabilityPayload::Tool(value),
            Self::Prompt(value) => CapabilityPayload::Prompt(value),
            Self::Resource(value) => CapabilityPayload::Resource(value),
            Self::ResourceTemplate(value) => CapabilityPayload::ResourceTemplate(value),
        }
    }
}

impl From<CapabilityPayload> for EffectiveCapabilityDefinition {
    fn from(payload: CapabilityPayload) -> Self {
        match payload {
            CapabilityPayload::Tool(value) => Self::Tool(value),
            CapabilityPayload::Prompt(value) => Self::Prompt(value),
            CapabilityPayload::Resource(value) => Self::Resource(value),
            CapabilityPayload::ResourceTemplate(value) => Self::ResourceTemplate(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveCapabilityRecordV1 {
    pub format: String,
    pub ref_id: CapabilityRefId,
    pub source: CapabilitySourceIdentity,
    pub definition: EffectiveCapabilityDefinition,
}

impl<'de> Deserialize<'de> for EffectiveCapabilityRecordV1 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireRecord {
            format: String,
            ref_id: CapabilityRefId,
            source: CapabilitySourceIdentity,
            definition: serde_json::Value,
        }

        let wire = WireRecord::deserialize(deserializer)?;
        let definition = match wire.source.kind {
            CapabilityKind::Tools => {
                serde_json::from_value::<Tool>(wire.definition).map(EffectiveCapabilityDefinition::Tool)
            }
            CapabilityKind::Prompts => {
                serde_json::from_value::<Prompt>(wire.definition).map(EffectiveCapabilityDefinition::Prompt)
            }
            CapabilityKind::Resources => {
                serde_json::from_value::<Resource>(wire.definition).map(EffectiveCapabilityDefinition::Resource)
            }
            CapabilityKind::ResourceTemplates => serde_json::from_value::<ResourceTemplate>(wire.definition)
                .map(EffectiveCapabilityDefinition::ResourceTemplate),
        }
        .map_err(serde::de::Error::custom)?;
        Ok(Self {
            format: wire.format,
            ref_id: wire.ref_id,
            source: wire.source,
            definition,
        })
    }
}

impl EffectiveCapabilityRecordV1 {
    pub fn new(
        ref_id: CapabilityRefId,
        source: CapabilitySourceIdentity,
        payload: CapabilityPayload,
    ) -> Result<Self> {
        let payload_kind = payload.kind();
        if source.kind != payload_kind {
            return Err(CatalogError::CapabilityKindMismatch {
                source_kind: source.kind,
                payload_kind,
            });
        }
        ref_id.verify_source(&source)?;
        Ok(Self {
            format: EFFECTIVE_CAPABILITY_FORMAT_V1.to_string(),
            ref_id,
            source,
            definition: payload.into(),
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json_canonicalizer::to_vec(self)?)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != EFFECTIVE_CAPABILITY_FORMAT_V1 {
            return Err(CatalogError::UnsupportedEffectiveCapabilityFormat {
                actual: self.format.clone(),
                expected: EFFECTIVE_CAPABILITY_FORMAT_V1,
            });
        }
        let definition_kind = self.definition.kind();
        if self.source.kind != definition_kind {
            return Err(CatalogError::CapabilityKindMismatch {
                source_kind: self.source.kind,
                payload_kind: definition_kind,
            });
        }
        self.ref_id.verify_source(&self.source)
    }
}

impl CapabilityId {
    pub fn derive(record: &EffectiveCapabilityRecordV1) -> Result<Self> {
        Ok(Self::from_digest(record.canonical_bytes()?))
    }

    pub fn verify_record(
        &self,
        record: &EffectiveCapabilityRecordV1,
    ) -> Result<()> {
        record.validate()?;
        self.verify_canonical_digest(&record.canonical_bytes()?)
    }

    pub fn verify_canonical_content(
        &self,
        saved: &[u8],
        candidate: &[u8],
    ) -> Result<()> {
        self.verify_canonical_encoding(saved)?;
        self.verify_canonical_encoding(candidate)?;
        self.verify_canonical_digest(saved)?;
        if saved == candidate {
            Ok(())
        } else {
            Err(CatalogError::IntegrityMismatch {
                identity: self.to_string(),
            })
        }
    }

    fn verify_canonical_encoding(
        &self,
        content: &[u8],
    ) -> Result<()> {
        let value: serde_json::Value = serde_json::from_slice(content)?;
        let canonical = serde_json_canonicalizer::to_vec(&value)?;
        if content == canonical {
            Ok(())
        } else {
            Err(CatalogError::IntegrityMismatch {
                identity: self.to_string(),
            })
        }
    }

    fn verify_canonical_digest(
        &self,
        canonical_content: &[u8],
    ) -> Result<()> {
        if Self::from_digest(canonical_content) == *self {
            Ok(())
        } else {
            Err(CatalogError::IntegrityMismatch {
                identity: self.to_string(),
            })
        }
    }
}

impl SurfaceManifestId {
    pub fn derive<T: Serialize>(manifest_content: &T) -> Result<Self> {
        Ok(Self::from_digest(serde_json_canonicalizer::to_vec(manifest_content)?))
    }

    pub fn verify_content<T: Serialize>(
        &self,
        manifest_content: &T,
    ) -> Result<()> {
        let canonical_content = serde_json_canonicalizer::to_vec(manifest_content)?;
        if Self::from_digest(canonical_content) == *self {
            Ok(())
        } else {
            Err(CatalogError::IntegrityMismatch {
                identity: self.to_string(),
            })
        }
    }
}

fn exact_origin_key(payload: &CapabilityPayload) -> String {
    match payload {
        CapabilityPayload::Tool(value) => value.name.to_string(),
        CapabilityPayload::Prompt(value) => value.name.to_string(),
        CapabilityPayload::Resource(value) => value.uri.to_string(),
        CapabilityPayload::ResourceTemplate(value) => value.uri_template.to_string(),
    }
}
