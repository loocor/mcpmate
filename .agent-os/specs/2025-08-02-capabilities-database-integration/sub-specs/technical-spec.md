# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-08-02-capabilities-database-integration/spec.md

> Created: 2025-08-02
> Version: 1.0.0

## Technical Requirements

### Core Architecture Components

#### 1. Unified Connection Pool Enhancement

**Current State Analysis**:
- `src/inspect/client.rs`: Independent MCP connection management
- `src/core/pool/`: Existing connection pool infrastructure
- Duplicate connection logic causing resource waste

**Technical Implementation**:
```rust
// Enhanced connection pool with instance classification
pub struct EnhancedConnectionPool {
    production_instances: HashMap<ServerId, Arc<McpConnection>>,
    exploration_instances: HashMap<SessionId, HashMap<ServerId, Arc<McpConnection>>>,
    validation_instances: HashMap<SessionId, HashMap<ServerId, Arc<McpConnection>>>,
    instance_metadata: HashMap<InstanceId, InstanceMetadata>,
}

pub enum InstanceType {
    Production,
    Exploration { session_id: String, ttl_minutes: u32 },
    Validation { session_id: String, ttl_minutes: u32 },
}

pub struct InstanceMetadata {
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    ttl: Duration,
    access_count: u64,
    visible_to_downstream: bool,
}
```

**Key Technical Features**:
- **Instance Isolation**: Separate connection pools prevent cross-contamination
- **TTL Management**: Automatic cleanup of temporary instances
- **Visibility Control**: Downstream clients only see Production instances
- **Resource Optimization**: Connection reuse across instance types where safe

#### 2. Redb Cache Implementation

**Performance Requirements**:
- Query latency: <100ms (vs current 500-2000ms)
- Storage efficiency: 65% reduction vs JSON
- Concurrent access: MVCC support for multi-user scenarios

**Technical Architecture**:
```rust
// Redb schema design
const SERVERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("servers");
const TOOLS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("tools");
const RESOURCES_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("resources");
const PROMPTS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("prompts");
const FINGERPRINTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("fingerprints");

pub struct RedbCacheManager {
    db: Database,
    write_txn_pool: Arc<Mutex<Vec<WriteTransaction>>>,
    metrics: Arc<CacheMetrics>,
}

// Serialization strategy
#[derive(Serialize, Deserialize)]
pub struct CachedServerData {
    server_info: ServerInfo,
    tools: Vec<ToolInfo>,
    resources: Vec<ResourceInfo>,
    prompts: Vec<PromptInfo>,
    cached_at: DateTime<Utc>,
    fingerprint: String,
}
```

**Migration Strategy**:
```rust
pub struct CacheMigrator {
    json_cache_dir: PathBuf,
    redb_path: PathBuf,
}

impl CacheMigrator {
    pub async fn migrate_all(&self) -> Result<MigrationReport, MigrationError> {
        // 1. Scan existing JSON cache files
        // 2. Parse and validate JSON data
        // 3. Convert to binary format
        // 4. Batch insert into Redb
        // 5. Verify data integrity
        // 6. Generate migration report
    }
}
```

#### 3. Unified Fingerprinting System

**Technical Design**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerFingerprint {
    pub code_fingerprint: CodeFingerprint,
    pub dependency_fingerprint: DependencyFingerprint,
    pub capability_fingerprint: CapabilityFingerprint,
    pub config_fingerprint: ConfigFingerprint,
    pub combined_hash: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFingerprint {
    pub file_hashes: HashMap<PathBuf, String>,
    pub total_files: usize,
    pub total_size: u64,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyFingerprint {
    pub package_lock_hash: Option<String>,  // package-lock.json, Cargo.lock, etc.
    pub manifest_hash: String,              // package.json, Cargo.toml, etc.
    pub resolved_versions: HashMap<String, String>,
}

pub trait FingerprintGenerator {
    async fn generate_fingerprint(&self, server_path: &Path) -> Result<MCPServerFingerprint, FingerprintError>;
    async fn compare_fingerprints(&self, old: &MCPServerFingerprint, new: &MCPServerFingerprint) -> FingerprintDiff;
}
```

**Change Detection Logic**:
```rust
pub enum ChangeType {
    CodeChange { files_changed: Vec<PathBuf> },
    DependencyChange { packages_changed: Vec<String> },
    CapabilityChange { capabilities_diff: CapabilityDiff },
    ConfigChange { config_diff: ConfigDiff },
    NoChange,
}

pub struct ChangeDetector {
    fingerprint_cache: Arc<RedbCacheManager>,
    file_watcher: Option<RecommendedWatcher>,
}
```

### Performance Optimization Strategies

#### 1. Concurrent Processing Architecture

**Current Bottleneck**:
```rust
// Current serial processing in src/inspect/manager.rs:269-291
for server_id in server_ids {
    let result = self.fetch_and_cache(server_id).await?;
    results.push(result);
}
```

**Optimized Implementation**:
```rust
// Concurrent batch processing
pub async fn batch_fetch_and_cache(
    &self,
    server_ids: Vec<ServerId>,
    concurrency_limit: usize,
) -> Result<Vec<CacheResult>, BatchError> {
    let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let futures: Vec<_> = server_ids
        .into_iter()
        .map(|server_id| {
            let semaphore = semaphore.clone();
            let manager = self.clone();
            async move {
                let _permit = semaphore.acquire().await?;
                manager.fetch_and_cache(server_id).await
            }
        })
        .collect();
    
    try_join_all(futures).await
}
```

#### 2. Intelligent Caching Strategy

**Context-Aware Freshness Levels**:
```rust
#[derive(Debug, Clone)]
pub enum FreshnessLevel {
    Cached,           // Use cache if available, no freshness check
    RecentlyFresh,    // Use cache if < 5 minutes old, otherwise refresh
    RealTime,         // Always fetch fresh data, update cache
}

pub struct CacheStrategy {
    pub freshness_level: FreshnessLevel,
    pub fallback_to_cache: bool,
    pub background_refresh: bool,
}

impl From<InstanceType> for CacheStrategy {
    fn from(instance_type: InstanceType) -> Self {
        match instance_type {
            InstanceType::Production => CacheStrategy {
                freshness_level: FreshnessLevel::Cached,
                fallback_to_cache: true,
                background_refresh: true,
            },
            InstanceType::Exploration { .. } => CacheStrategy {
                freshness_level: FreshnessLevel::RecentlyFresh,
                fallback_to_cache: true,
                background_refresh: false,
            },
            InstanceType::Validation { .. } => CacheStrategy {
                freshness_level: FreshnessLevel::RealTime,
                fallback_to_cache: false,
                background_refresh: false,
            },
        }
    }
}
```

### Intelligent Conflict Resolution

#### 1. Multi-Layer Similarity Detection

**Technical Implementation**:
```rust
#[derive(Debug, Clone)]
pub struct ConflictAnalyzer {
    similarity_threshold: f64,
    weight_config: SimilarityWeights,
}

#[derive(Debug, Clone)]
pub struct SimilarityWeights {
    pub command_name: f64,      // 0.4
    pub parameter_overlap: f64, // 0.3
    pub description_semantic: f64, // 0.2
    pub functionality_overlap: f64, // 0.1
}

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub conflicting_servers: Vec<ServerId>,
    pub similarity_score: f64,
    pub conflict_details: ConflictDetails,
    pub recommendation: SmartRecommendation,
}

#[derive(Debug, Clone)]
pub enum SmartRecommendation {
    ReplaceWithNew {
        reason: String,
        migration_plan: MigrationPlan,
        confidence: f64,
    },
    KeepExisting {
        reason: String,
        confidence: f64,
    },
    UserChoice {
        options: Vec<ConflictOption>,
        recommendation: Option<ConflictOption>,
        reasoning: String,
    },
    OptimizedCoexistence {
        optimization_plan: CoexistencePlan,
        resource_impact: ResourceImpact,
    },
}
```

**Semantic Analysis Integration**:
```rust
// Optional LLM integration for semantic similarity
pub struct SemanticAnalyzer {
    embedding_client: Option<EmbeddingClient>,
    fallback_analyzer: KeywordAnalyzer,
}

impl SemanticAnalyzer {
    pub async fn analyze_tool_similarity(
        &self,
        tool1: &ToolInfo,
        tool2: &ToolInfo,
    ) -> Result<f64, AnalysisError> {
        if let Some(client) = &self.embedding_client {
            // Use embeddings for semantic similarity
            let embedding1 = client.get_embedding(&tool1.description).await?;
            let embedding2 = client.get_embedding(&tool2.description).await?;
            Ok(cosine_similarity(&embedding1, &embedding2))
        } else {
            // Fallback to keyword-based analysis
            self.fallback_analyzer.analyze_similarity(tool1, tool2)
        }
    }
}
```

### API Layer Transformation

#### 1. Simplified Endpoint Architecture

**Enhanced Parameter Handling**:
```rust
#[derive(Debug, Deserialize)]
pub struct EnhancedServerQuery {
    pub instance_type: Option<InstanceType>,
    pub include_cache_info: Option<bool>,
    pub freshness_level: Option<FreshnessLevel>,
    pub session_id: Option<String>,
}

// Unified handler pattern
pub async fn get_server_tools(
    Path(server_id): Path<String>,
    Query(params): Query<EnhancedServerQuery>,
    State(app_state): State<AppState>,
) -> Result<Json<ToolsResponse>, ApiError> {
    let instance_type = params.instance_type.unwrap_or(InstanceType::Production);
    let cache_strategy = CacheStrategy::from(instance_type.clone());
    
    let tools = app_state
        .connection_pool
        .get_server_tools(&server_id, instance_type, cache_strategy)
        .await?;
    
    Ok(Json(ToolsResponse {
        tools,
        cache_info: if params.include_cache_info.unwrap_or(false) {
            Some(app_state.cache_manager.get_cache_info(&server_id).await?)
        } else {
            None
        },
        instance_info: InstanceInfo::from(instance_type),
    }))
}
```

#### 2. Backward Compatibility Layer

**Migration Support**:
```rust
// Compatibility wrapper for deprecated endpoints
pub async fn deprecated_capabilities_handler(
    Path(server_id): Path<String>,
    Query(old_params): Query<CapabilitiesQuery>,
    State(app_state): State<AppState>,
) -> Result<Json<CapabilitiesResponse>, ApiError> {
    // Log deprecation warning
    warn!("Deprecated endpoint used: /api/mcp/servers/{}/capabilities", server_id);
    
    // Convert old parameters to new format
    let instance_type = match old_params.refresh_strategy {
        Some(RefreshStrategy::Force) => InstanceType::Validation {
            session_id: generate_session_id(),
            ttl_minutes: 5,
        },
        Some(RefreshStrategy::RefreshIfStale) => InstanceType::Exploration {
            session_id: generate_session_id(),
            ttl_minutes: 30,
        },
        _ => InstanceType::Production,
    };
    
    // Delegate to new implementation
    let tools = get_server_tools(Path(server_id.clone()), Query(EnhancedServerQuery {
        instance_type: Some(instance_type.clone()),
        include_cache_info: Some(true),
        freshness_level: None,
        session_id: None,
    }), State(app_state.clone())).await?;
    
    // Convert response to old format
    Ok(Json(CapabilitiesResponse::from(tools.0)))
}
```

## Approach

### Implementation Strategy

#### Phase 1: Foundation (Week 1-2)
1. **Redb Integration**: Add dependency, create basic schema, implement core CRUD operations
2. **Migration Tooling**: Build JSON-to-Redb migration utility with data validation
3. **Performance Benchmarking**: Establish baseline metrics and testing framework

#### Phase 2: Connection Pool Enhancement (Week 3-4)
1. **Instance Classification**: Extend connection pool with InstanceType support
2. **Lifecycle Management**: Implement TTL-based cleanup and resource management
3. **Isolation Mechanisms**: Ensure proper separation between instance types

#### Phase 3: Inspect Module Migration (Week 5-6)
1. **Functionality Audit**: Catalog all inspect module capabilities
2. **Gradual Migration**: Move functionality to connection pool piece by piece
3. **API Compatibility**: Maintain existing endpoints during transition

#### Phase 4: Intelligence Features (Week 7-8)
1. **Fingerprinting System**: Implement unified change detection
2. **Conflict Detection**: Build similarity analysis and recommendation engine
3. **Smart Caching**: Deploy context-aware caching strategies

### Risk Mitigation

#### Data Migration Risks
- **Mitigation**: Comprehensive backup strategy before migration
- **Validation**: Automated data integrity checks post-migration
- **Rollback**: Ability to revert to JSON cache if issues arise

#### Performance Regression Risks
- **Mitigation**: Extensive benchmarking at each phase
- **Monitoring**: Real-time performance metrics and alerting
- **Gradual Rollout**: Feature flags for controlled deployment

#### API Compatibility Risks
- **Mitigation**: Comprehensive integration test suite
- **Versioning**: Maintain deprecated endpoints during transition
- **Documentation**: Clear migration guides for consumers

### Testing Strategy

#### Unit Testing
- **Cache Operations**: Test all Redb CRUD operations with edge cases
- **Fingerprinting**: Validate change detection accuracy
- **Conflict Detection**: Test similarity algorithms with known cases

#### Integration Testing
- **End-to-End Workflows**: Test complete server addition and management flows
- **Performance Testing**: Validate query performance improvements
- **Concurrency Testing**: Verify multi-user access patterns

#### Migration Testing
- **Data Integrity**: Ensure no data loss during JSON-to-Redb migration
- **API Compatibility**: Verify all existing endpoints continue working
- **Performance Validation**: Confirm expected performance improvements

## External Dependencies

### New Dependencies

#### Redb Database
- **Version**: `^2.1.0`
- **Purpose**: High-performance embedded database for caching
- **Justification**: 5-10x performance improvement over JSON file caching
- **Risk Assessment**: Mature library with active maintenance

#### Tokio Semaphore
- **Version**: Already included in tokio
- **Purpose**: Concurrency control for batch operations
- **Justification**: Prevent resource exhaustion during parallel processing

#### Additional Serialization
- **bincode**: `^1.3.0` - Efficient binary serialization
- **Purpose**: Optimize data storage format for Redb
- **Justification**: Significant space savings over JSON

### Modified Dependencies

#### Existing Connection Pool
- **Changes**: Extend with instance type classification
- **Risk**: Minimal - additive changes only
- **Testing**: Comprehensive integration tests required

#### API Layer
- **Changes**: Add new parameter handling, maintain backward compatibility
- **Risk**: Low - existing endpoints preserved
- **Migration**: Gradual deprecation of old patterns

### Infrastructure Requirements

#### Storage
- **Redb Files**: Estimated 35% of current JSON cache size
- **Backup Strategy**: Regular snapshots of Redb files
- **Monitoring**: Disk usage and performance metrics

#### Memory
- **Connection Pool**: Increased memory for instance classification
- **Cache**: Reduced memory usage due to efficient binary format
- **Monitoring**: Memory usage patterns and optimization opportunities

#### CPU
- **Serialization**: Reduced CPU usage with binary format
- **Concurrency**: Increased CPU utilization during parallel operations
- **Optimization**: Background processing for non-critical operations