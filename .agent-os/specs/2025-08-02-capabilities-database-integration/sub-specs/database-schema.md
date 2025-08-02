# Database Schema

This is the database schema implementation for the spec detailed in @.agent-os/specs/2025-08-02-capabilities-database-integration/spec.md

> Created: 2025-08-02
> Version: 1.0.0

## Schema Changes

### Redb Table Definitions

#### Core Tables

**Servers Table**
```rust
const SERVERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("servers");

// Key: server_id (String)
// Value: SerializedServerData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedServerData {
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub status: ServerStatus,
    pub connection_info: ConnectionInfo,
    pub cached_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub fingerprint: String,
    pub instance_type: InstanceType,
}
```

**Tools Table**
```rust
const TOOLS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("tools");

// Key: (server_id, tool_name)
// Value: SerializedToolData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedToolData {
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub parameters: Vec<ToolParameter>,
    pub cached_at: DateTime<Utc>,
    pub fingerprint: String,
    pub usage_count: u64,
    pub last_used: Option<DateTime<Utc>>,
}
```

**Resources Table**
```rust
const RESOURCES_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("resources");

// Key: (server_id, resource_uri)
// Value: SerializedResourceData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedResourceData {
    pub server_id: String,
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub annotations: Option<ResourceAnnotations>,
    pub cached_at: DateTime<Utc>,
    pub fingerprint: String,
    pub access_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
}
```

**Prompts Table**
```rust
const PROMPTS_TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("prompts");

// Key: (server_id, prompt_name)
// Value: SerializedPromptData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedPromptData {
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
    pub cached_at: DateTime<Utc>,
    pub fingerprint: String,
    pub usage_count: u64,
    pub last_used: Option<DateTime<Utc>>,
}
```

#### Metadata Tables

**Fingerprints Table**
```rust
const FINGERPRINTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("fingerprints");

// Key: server_id (String)
// Value: SerializedFingerprintData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedFingerprintData {
    pub server_id: String,
    pub code_fingerprint: CodeFingerprint,
    pub dependency_fingerprint: DependencyFingerprint,
    pub capability_fingerprint: CapabilityFingerprint,
    pub config_fingerprint: ConfigFingerprint,
    pub combined_hash: String,
    pub generated_at: DateTime<Utc>,
    pub last_checked: DateTime<Utc>,
}
```

**Instance Metadata Table**
```rust
const INSTANCES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("instances");

// Key: instance_id (String)
// Value: SerializedInstanceData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedInstanceData {
    pub instance_id: String,
    pub server_id: String,
    pub instance_type: InstanceType,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub ttl: Option<Duration>,
    pub expires_at: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub visible_to_downstream: bool,
    pub status: InstanceStatus,
}
```

**Conflicts Table**
```rust
const CONFLICTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("conflicts");

// Key: conflict_id (String)
// Value: SerializedConflictData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedConflictData {
    pub conflict_id: String,
    pub conflicting_servers: Vec<String>,
    pub conflict_type: ConflictType,
    pub similarity_score: f64,
    pub conflict_details: ConflictDetails,
    pub recommendation: SmartRecommendation,
    pub status: ConflictStatus,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
```

#### Performance Optimization Tables

**Query Cache Table**
```rust
const QUERY_CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("query_cache");

// Key: query_hash (String)
// Value: SerializedQueryResult (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedQueryResult {
    pub query_hash: String,
    pub query_type: QueryType,
    pub result_data: Vec<u8>,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub hit_count: u64,
    pub last_hit: DateTime<Utc>,
}
```

**Metrics Table**
```rust
const METRICS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("metrics");

// Key: metric_key (String)
// Value: SerializedMetricData (bincode)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedMetricData {
    pub metric_key: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub tags: HashMap<String, String>,
    pub aggregation_period: Option<Duration>,
}
```

### Supporting Data Structures

#### Enums and Types

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InstanceType {
    Production,
    Exploration { session_id: String, ttl_minutes: u32 },
    Validation { session_id: String, ttl_minutes: u32 },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Active,
    Inactive,
    Error { message: String },
    Connecting,
    Disconnected,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InstanceStatus {
    Running,
    Stopped,
    Error { message: String },
    Expired,
    Cleanup,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConflictType {
    DuplicateTools,
    SimilarCapabilities,
    ResourceOverlap,
    NameCollision,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConflictStatus {
    Detected,
    UnderReview,
    Resolved,
    Ignored,
    AutoResolved,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QueryType {
    ServerList,
    ToolsList,
    ResourcesList,
    PromptsList,
    ConflictAnalysis,
    CapabilitySearch,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
}
```

#### Complex Data Structures

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionInfo {
    pub transport: TransportType,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
    pub timeout: Option<Duration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub parameter_type: String,
    pub description: Option<String>,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceAnnotations {
    pub audience: Option<Vec<String>>,
    pub priority: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConflictDetails {
    pub tool_conflicts: Vec<ToolConflict>,
    pub resource_conflicts: Vec<ResourceConflict>,
    pub capability_overlaps: Vec<CapabilityOverlap>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolConflict {
    pub tool_name: String,
    pub servers: Vec<String>,
    pub similarity_score: f64,
    pub parameter_differences: Vec<ParameterDifference>,
}
```

### Database Operations Interface

#### Core Database Manager

```rust
pub struct RedbCacheManager {
    db: Database,
    write_txn_pool: Arc<Mutex<Vec<WriteTransaction>>>,
    read_txn_pool: Arc<Mutex<Vec<ReadTransaction>>>,
    metrics: Arc<CacheMetrics>,
    config: CacheConfig,
}

impl RedbCacheManager {
    pub async fn new(db_path: &Path, config: CacheConfig) -> Result<Self, CacheError> {
        let db = Database::create(db_path)?;
        
        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(SERVERS_TABLE)?;
            write_txn.open_table(TOOLS_TABLE)?;
            write_txn.open_table(RESOURCES_TABLE)?;
            write_txn.open_table(PROMPTS_TABLE)?;
            write_txn.open_table(FINGERPRINTS_TABLE)?;
            write_txn.open_table(INSTANCES_TABLE)?;
            write_txn.open_table(CONFLICTS_TABLE)?;
            write_txn.open_table(QUERY_CACHE_TABLE)?;
            write_txn.open_table(METRICS_TABLE)?;
        }
        write_txn.commit()?;
        
        Ok(Self {
            db,
            write_txn_pool: Arc::new(Mutex::new(Vec::new())),
            read_txn_pool: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(CacheMetrics::new()),
            config,
        })
    }
}
```

#### CRUD Operations

```rust
impl RedbCacheManager {
    // Server operations
    pub async fn store_server(&self, server_data: CachedServerData) -> Result<(), CacheError> {
        let serialized = bincode::serialize(&server_data)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(SERVERS_TABLE)?;
            table.insert(&server_data.server_id, serialized.as_slice())?;
        }
        write_txn.commit()?;
        self.metrics.increment_writes().await;
        Ok(())
    }
    
    pub async fn get_server(&self, server_id: &str) -> Result<Option<CachedServerData>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SERVERS_TABLE)?;
        
        if let Some(data) = table.get(server_id)? {
            let server_data: CachedServerData = bincode::deserialize(data.value())?;
            self.metrics.increment_reads().await;
            Ok(Some(server_data))
        } else {
            Ok(None)
        }
    }
    
    // Tool operations
    pub async fn store_tools(&self, server_id: &str, tools: Vec<CachedToolData>) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TOOLS_TABLE)?;
            for tool in tools {
                let key = (server_id, tool.name.as_str());
                let serialized = bincode::serialize(&tool)?;
                table.insert(key, serialized.as_slice())?;
            }
        }
        write_txn.commit()?;
        self.metrics.increment_batch_writes().await;
        Ok(())
    }
    
    pub async fn get_server_tools(&self, server_id: &str) -> Result<Vec<CachedToolData>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TOOLS_TABLE)?;
        
        let mut tools = Vec::new();
        let prefix = (server_id, "");
        
        for result in table.range((prefix..)..)? {
            let (key, value) = result?;
            if key.0 != server_id {
                break;
            }
            let tool_data: CachedToolData = bincode::deserialize(value.value())?;
            tools.push(tool_data);
        }
        
        self.metrics.increment_range_reads().await;
        Ok(tools)
    }
    
    // Batch operations for performance
    pub async fn batch_store_capabilities(
        &self,
        server_id: &str,
        tools: Vec<CachedToolData>,
        resources: Vec<CachedResourceData>,
        prompts: Vec<CachedPromptData>,
    ) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;
        {
            // Store tools
            let mut tools_table = write_txn.open_table(TOOLS_TABLE)?;
            for tool in tools {
                let key = (server_id, tool.name.as_str());
                let serialized = bincode::serialize(&tool)?;
                tools_table.insert(key, serialized.as_slice())?;
            }
            
            // Store resources
            let mut resources_table = write_txn.open_table(RESOURCES_TABLE)?;
            for resource in resources {
                let key = (server_id, resource.uri.as_str());
                let serialized = bincode::serialize(&resource)?;
                resources_table.insert(key, serialized.as_slice())?;
            }
            
            // Store prompts
            let mut prompts_table = write_txn.open_table(PROMPTS_TABLE)?;
            for prompt in prompts {
                let key = (server_id, prompt.name.as_str());
                let serialized = bincode::serialize(&prompt)?;
                prompts_table.insert(key, serialized.as_slice())?;
            }
        }
        write_txn.commit()?;
        self.metrics.increment_batch_writes().await;
        Ok(())
    }
}
```

## Migrations

### Migration from JSON Cache

#### Migration Strategy

```rust
pub struct JsonToRedbMigrator {
    json_cache_dir: PathBuf,
    redb_manager: RedbCacheManager,
    migration_config: MigrationConfig,
}

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub batch_size: usize,
    pub validate_data: bool,
    pub backup_json: bool,
    pub parallel_processing: bool,
    pub max_concurrent_files: usize,
}

impl JsonToRedbMigrator {
    pub async fn migrate_all(&self) -> Result<MigrationReport, MigrationError> {
        let mut report = MigrationReport::new();
        
        // 1. Discover all JSON cache files
        let json_files = self.discover_json_files().await?;
        report.total_files = json_files.len();
        
        // 2. Create backup if requested
        if self.migration_config.backup_json {
            self.create_backup().await?;
        }
        
        // 3. Process files in batches
        let batches = json_files.chunks(self.migration_config.batch_size);
        
        for batch in batches {
            let batch_result = if self.migration_config.parallel_processing {
                self.process_batch_parallel(batch).await?
            } else {
                self.process_batch_sequential(batch).await?
            };
            
            report.merge(batch_result);
        }
        
        // 4. Validate migration if requested
        if self.migration_config.validate_data {
            let validation_result = self.validate_migration().await?;
            report.validation_result = Some(validation_result);
        }
        
        Ok(report)
    }
    
    async fn process_file(&self, json_file: &Path) -> Result<FileMigrationResult, MigrationError> {
        // Read JSON file
        let json_content = tokio::fs::read_to_string(json_file).await?;
        let legacy_data: LegacyCacheData = serde_json::from_str(&json_content)?;
        
        // Convert to new format
        let converted_data = self.convert_legacy_data(legacy_data)?;
        
        // Store in Redb
        self.store_converted_data(converted_data).await?;
        
        Ok(FileMigrationResult {
            file_path: json_file.to_path_buf(),
            records_migrated: converted_data.record_count(),
            size_before: json_content.len(),
            size_after: converted_data.binary_size(),
            migration_time: Instant::now().duration_since(start_time),
        })
    }
}
```

#### Data Conversion Logic

```rust
impl JsonToRedbMigrator {
    fn convert_legacy_data(&self, legacy: LegacyCacheData) -> Result<ConvertedData, ConversionError> {
        let mut converted = ConvertedData::new();
        
        // Convert server data
        let server_data = CachedServerData {
            server_id: legacy.server_id.clone(),
            name: legacy.name,
            description: legacy.description,
            version: legacy.version,
            status: ServerStatus::from_legacy(legacy.status),
            connection_info: ConnectionInfo::from_legacy(legacy.connection_info),
            cached_at: legacy.cached_at,
            last_accessed: legacy.last_accessed.unwrap_or(legacy.cached_at),
            fingerprint: legacy.fingerprint.unwrap_or_else(|| self.generate_fingerprint(&legacy)),
            instance_type: InstanceType::Production, // Default for existing data
        };
        converted.server_data = Some(server_data);
        
        // Convert tools
        if let Some(tools) = legacy.tools {
            converted.tools = tools.into_iter()
                .map(|tool| CachedToolData {
                    server_id: legacy.server_id.clone(),
                    name: tool.name,
                    description: tool.description,
                    input_schema: tool.input_schema,
                    parameters: tool.parameters.into_iter()
                        .map(ToolParameter::from_legacy)
                        .collect(),
                    cached_at: legacy.cached_at,
                    fingerprint: self.generate_tool_fingerprint(&tool),
                    usage_count: 0, // Reset usage statistics
                    last_used: None,
                })
                .collect();
        }
        
        // Convert resources and prompts similarly...
        
        Ok(converted)
    }
}
```

### Schema Versioning

#### Version Management

```rust
const SCHEMA_VERSION_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("schema_version");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaVersion {
    pub version: String,
    pub applied_at: DateTime<Utc>,
    pub migration_id: String,
    pub checksum: String,
}

pub struct SchemaMigrationManager {
    db: Database,
    migrations: Vec<Box<dyn SchemaMigration>>,
}

pub trait SchemaMigration: Send + Sync {
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn up(&self, db: &Database) -> Result<(), MigrationError>;
    fn down(&self, db: &Database) -> Result<(), MigrationError>;
    fn checksum(&self) -> String;
}
```

#### Future Migration Support

```rust
// Example future migration: Adding indexing support
pub struct AddIndexingMigration;

impl SchemaMigration for AddIndexingMigration {
    fn version(&self) -> &str { "2025-08-15-001" }
    
    fn description(&self) -> &str {
        "Add indexing tables for improved query performance"
    }
    
    fn up(&self, db: &Database) -> Result<(), MigrationError> {
        let write_txn = db.begin_write()?;
        {
            // Create new index tables
            write_txn.open_table(TOOL_NAME_INDEX_TABLE)?;
            write_txn.open_table(SERVER_STATUS_INDEX_TABLE)?;
            
            // Populate indexes from existing data
            self.populate_indexes(&write_txn)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    
    fn down(&self, db: &Database) -> Result<(), MigrationError> {
        let write_txn = db.begin_write()?;
        {
            write_txn.delete_table(TOOL_NAME_INDEX_TABLE)?;
            write_txn.delete_table(SERVER_STATUS_INDEX_TABLE)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
```

### Performance Optimization Schema

#### Query Optimization Tables

```rust
// Frequently accessed data optimization
const HOT_DATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("hot_data");
const COLD_DATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("cold_data");

// Query pattern optimization
const QUERY_PATTERNS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("query_patterns");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryPattern {
    pub pattern_id: String,
    pub query_type: QueryType,
    pub frequency: u64,
    pub avg_response_time: Duration,
    pub last_optimized: DateTime<Utc>,
    pub optimization_strategy: OptimizationStrategy,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OptimizationStrategy {
    Precompute,
    Index,
    Cache,
    Denormalize,
}
```

This schema design provides a robust foundation for the capabilities database integration refactor, with comprehensive support for caching, performance optimization, conflict resolution, and future extensibility.