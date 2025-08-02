# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-08-02-capabilities-database-integration/spec.md

> Created: 2025-08-02
> Version: 1.0.0

## Endpoints

### Enhanced Server Management Endpoints

#### GET /api/mcp/servers
**Purpose**: List all MCP servers with optional filtering
**Status**: Enhanced (backward compatible)

**Query Parameters**:
```rust
#[derive(Debug, Deserialize)]
pub struct ServerListQuery {
    pub instance_type: Option<InstanceType>,
    pub status: Option<ServerStatus>,
    pub include_cache_info: Option<bool>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}
```

**Response**:
```json
{
  "servers": [
    {
      "id": "server-123",
      "name": "File Operations Server",
      "description": "Provides file system operations",
      "status": "active",
      "instance_type": "production",
      "created_at": "2025-08-02T10:00:00Z",
      "last_accessed": "2025-08-02T15:30:00Z",
      "cache_info": {
        "cached_at": "2025-08-02T15:25:00Z",
        "fingerprint": "abc123...",
        "is_fresh": true
      }
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 45,
    "has_next": true
  }
}
```

#### POST /api/mcp/servers
**Purpose**: Create a new MCP server
**Status**: Enhanced (unified creation endpoint)

**Request Body**:
```json
{
  "source_type": "manual|url_drag|url_scheme|config_import|marketplace|dxt_extension|community_suit",
  "server_data": {
    "name": "My Server",
    "description": "Server description",
    "command": "node server.js",
    "args": ["--port", "3000"],
    "env": {"NODE_ENV": "production"},
    "working_directory": "/path/to/server"
  },
  "validation_options": {
    "validate_before_save": true,
    "conflict_resolution": "auto|manual|skip",
    "instance_type": "validation"
  }
}
```

**Response**:
```json
{
  "server": {
    "id": "server-456",
    "name": "My Server",
    "status": "active",
    "validation_result": {
      "is_valid": true,
      "capabilities_detected": {
        "tools_count": 5,
        "resources_count": 2,
        "prompts_count": 1
      },
      "conflicts_detected": [],
      "recommendations": []
    }
  }
}
```

#### GET /api/mcp/servers/{id}
**Purpose**: Get detailed server information
**Status**: Enhanced (backward compatible)

**Query Parameters**:
```rust
#[derive(Debug, Deserialize)]
pub struct ServerDetailQuery {
    pub instance_type: Option<InstanceType>,
    pub include_capabilities_summary: Option<bool>,
    pub include_cache_info: Option<bool>,
    pub session_id: Option<String>,
}
```

**Response**:
```json
{
  "server": {
    "id": "server-123",
    "name": "File Operations Server",
    "description": "Provides file system operations",
    "status": "active",
    "connection_info": {
      "command": "node file-server.js",
      "args": ["--config", "prod.json"],
      "env": {"NODE_ENV": "production"},
      "working_directory": "/opt/servers/file-ops"
    },
    "capabilities_summary": {
      "tools_count": 8,
      "resources_count": 3,
      "prompts_count": 2,
      "last_updated": "2025-08-02T15:25:00Z"
    },
    "instance_info": {
      "type": "production",
      "created_at": "2025-08-02T10:00:00Z",
      "last_accessed": "2025-08-02T15:30:00Z",
      "access_count": 1247
    },
    "cache_info": {
      "cached_at": "2025-08-02T15:25:00Z",
      "fingerprint": "abc123...",
      "is_fresh": true,
      "cache_hit_rate": 0.95
    }
  }
}
```

#### GET /api/mcp/servers/{id}/tools
**Purpose**: Get server tools with intelligent caching
**Status**: Enhanced (instance_type parameter added)

**Query Parameters**:
```rust
#[derive(Debug, Deserialize)]
pub struct ToolsQuery {
    pub instance_type: Option<InstanceType>,
    pub include_usage_stats: Option<bool>,
    pub filter_by_category: Option<String>,
    pub session_id: Option<String>,
}
```

**Response**:
```json
{
  "tools": [
    {
      "name": "read_file",
      "description": "Read contents of a file",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": {"type": "string", "description": "File path to read"}
        },
        "required": ["path"]
      },
      "usage_stats": {
        "usage_count": 156,
        "last_used": "2025-08-02T14:30:00Z",
        "avg_response_time": "120ms"
      }
    }
  ],
  "instance_info": {
    "type": "production",
    "data_source": "cache",
    "freshness": "5_minutes_ago"
  },
  "cache_info": {
    "cached_at": "2025-08-02T15:25:00Z",
    "is_fresh": true
  }
}
```

#### GET /api/mcp/servers/{id}/resources
**Purpose**: Get server resources with intelligent caching
**Status**: Enhanced (instance_type parameter added)

**Query Parameters**: Same as tools endpoint

**Response**:
```json
{
  "resources": [
    {
      "uri": "file:///data/config.json",
      "name": "Application Configuration",
      "description": "Main application configuration file",
      "mime_type": "application/json",
      "annotations": {
        "audience": ["developers"],
        "priority": 0.8
      },
      "access_stats": {
        "access_count": 45,
        "last_accessed": "2025-08-02T13:15:00Z"
      }
    }
  ],
  "instance_info": {
    "type": "exploration",
    "session_id": "session-789",
    "data_source": "real_time",
    "expires_at": "2025-08-02T16:00:00Z"
  }
}
```

#### GET /api/mcp/servers/{id}/prompts
**Purpose**: Get server prompts with intelligent caching
**Status**: Enhanced (instance_type parameter added)

**Query Parameters**: Same as tools endpoint

**Response**:
```json
{
  "prompts": [
    {
      "name": "analyze_code",
      "description": "Analyze code quality and suggest improvements",
      "arguments": [
        {
          "name": "code",
          "description": "Code to analyze",
          "required": true
        },
        {
          "name": "language",
          "description": "Programming language",
          "required": false
        }
      ],
      "usage_stats": {
        "usage_count": 23,
        "last_used": "2025-08-02T12:45:00Z"
      }
    }
  ],
  "instance_info": {
    "type": "validation",
    "session_id": "validation-456",
    "data_source": "real_time",
    "validation_purpose": "pre_save_check"
  }
}
```

#### GET /api/mcp/servers/{id}/resource-templates
**Purpose**: Get server resource templates
**Status**: Enhanced (instance_type parameter added)

**Query Parameters**: Same as other capability endpoints

**Response**:
```json
{
  "resource_templates": [
    {
      "uri_template": "file:///{path}",
      "name": "File Resource",
      "description": "Access any file in the system",
      "mime_type": "*/*"
    }
  ],
  "instance_info": {
    "type": "production",
    "data_source": "cache"
  }
}
```

### Runtime Management Endpoints

#### GET /api/runtime/status
**Purpose**: Get comprehensive runtime status
**Status**: Enhanced (consolidated runtime info)

**Response**:
```json
{
  "runtime_status": {
    "node_js": {
      "version": "18.17.0",
      "status": "available",
      "package_managers": {
        "npm": "9.6.7",
        "npx": "9.6.7"
      }
    },
    "python": {
      "version": "3.11.4",
      "status": "available",
      "package_managers": {
        "pip": "23.1.2",
        "uv": "0.1.35"
      }
    }
  },
  "cache_status": {
    "total_size": "2.3GB",
    "entries_count": 1247,
    "hit_rate": 0.94,
    "last_cleanup": "2025-08-02T06:00:00Z"
  },
  "active_servers": {
    "production": 12,
    "exploration": 3,
    "validation": 1
  }
}
```

#### POST /api/runtime/install
**Purpose**: Install runtime dependencies
**Status**: New endpoint

**Request Body**:
```json
{
  "runtime": "node|python",
  "packages": [
    {
      "name": "express",
      "version": "^4.18.0",
      "dev_dependency": false
    }
  ],
  "server_id": "server-123",
  "install_strategy": "global|local|isolated"
}
```

#### GET /api/runtime/cache
**Purpose**: Get cache status and statistics
**Status**: New endpoint

**Response**:
```json
{
  "cache_statistics": {
    "redb_cache": {
      "size": "156MB",
      "entries": 1247,
      "hit_rate": 0.96,
      "avg_query_time": "12ms"
    },
    "build_artifacts": {
      "size": "2.1GB",
      "entries": 89,
      "last_cleanup": "2025-08-02T06:00:00Z"
    },
    "dependency_cache": {
      "node_modules_size": "1.8GB",
      "python_packages_size": "340MB",
      "shared_dependencies": 156
    }
  },
  "performance_metrics": {
    "cache_hit_rate_trend": [0.94, 0.95, 0.96],
    "query_time_trend": ["15ms", "13ms", "12ms"],
    "storage_growth_rate": "2.3MB/day"
  }
}
```

#### POST /api/runtime/cache/clear
**Purpose**: Clear various cache types
**Status**: New endpoint

**Request Body**:
```json
{
  "cache_types": ["redb", "build_artifacts", "dependencies"],
  "server_ids": ["server-123"],  // Optional: clear specific servers only
  "force": false,
  "backup_before_clear": true
}
```

#### POST /api/runtime/cache/rebuild
**Purpose**: Rebuild cache from scratch
**Status**: New endpoint

**Request Body**:
```json
{
  "rebuild_strategy": "full|incremental|selective",
  "server_ids": ["server-123"],  // Optional
  "parallel_processing": true,
  "validate_after_rebuild": true
}
```

#### GET /api/runtime/versions
**Purpose**: Get version information for all runtimes
**Status**: New endpoint

**Response**:
```json
{
  "versions": {
    "mcpmate": "1.2.0",
    "node_js": "18.17.0",
    "python": "3.11.4",
    "redb": "2.1.0"
  },
  "compatibility_matrix": {
    "node_servers": {
      "supported_versions": ["16.x", "18.x", "20.x"],
      "recommended_version": "18.x"
    },
    "python_servers": {
      "supported_versions": ["3.9", "3.10", "3.11", "3.12"],
      "recommended_version": "3.11"
    }
  }
}
```

### Intelligent Configuration Endpoints

#### POST /api/mcp/suits/intelligent-create
**Purpose**: Create server configuration using LLM intelligence
**Status**: New endpoint

**Request Body**:
```json
{
  "user_intent": "I need to manage my project files and run git commands",
  "context": {
    "project_type": "web_development",
    "languages": ["javascript", "typescript"],
    "existing_servers": ["server-123", "server-456"],
    "preferences": {
      "prefer_lightweight": true,
      "avoid_duplicates": true
    }
  },
  "creation_options": {
    "auto_resolve_conflicts": true,
    "validate_before_save": true,
    "suggest_alternatives": true
  }
}
```

**Response**:
```json
{
  "intelligent_recommendation": {
    "recommended_servers": [
      {
        "name": "File Manager Pro",
        "description": "Advanced file operations with git integration",
        "source": "marketplace",
        "confidence_score": 0.92,
        "reasoning": "Combines file management and git operations as requested"
      }
    ],
    "conflict_analysis": {
      "potential_conflicts": [],
      "resolution_strategy": "no_conflicts_detected"
    },
    "alternative_options": [
      {
        "option": "separate_servers",
        "description": "Use separate file manager and git servers",
        "pros": ["More modular", "Easier to maintain"],
        "cons": ["More resource usage"]
      }
    ]
  },
  "suit_configuration": {
    "name": "Development File Management",
    "servers": [
      {
        "id": "generated-server-789",
        "name": "File Manager Pro",
        "command": "npx file-manager-mcp",
        "args": ["--git-integration"]
      }
    ]
  }
}
```

#### POST /api/mcp/suits/{id}/validate-before-save
**Purpose**: Validate suit configuration before saving
**Status**: New endpoint

**Request Body**:
```json
{
  "suit_configuration": {
    "name": "My Development Suite",
    "servers": [
      {
        "name": "File Server",
        "command": "node file-server.js"
      }
    ]
  },
  "validation_options": {
    "check_conflicts": true,
    "validate_commands": true,
    "test_connectivity": true,
    "analyze_performance_impact": true
  }
}
```

**Response**:
```json
{
  "validation_result": {
    "is_valid": true,
    "validation_score": 0.89,
    "issues_found": [],
    "warnings": [
      {
        "type": "performance",
        "message": "Server may have high memory usage",
        "severity": "low",
        "suggestion": "Consider adding memory limits"
      }
    ],
    "conflict_analysis": {
      "conflicts_detected": false,
      "similar_servers": []
    },
    "performance_impact": {
      "estimated_memory_usage": "45MB",
      "estimated_startup_time": "2.3s",
      "resource_efficiency_score": 0.85
    }
  },
  "optimization_suggestions": [
    {
      "type": "configuration",
      "suggestion": "Add environment variable NODE_ENV=production",
      "impact": "Reduces memory usage by ~15%"
    }
  ]
}
```

### Debug and Monitoring Endpoints

#### GET /api/debug/cache-stats
**Purpose**: Get detailed cache statistics for debugging
**Status**: New endpoint (optional, for debugging)

**Response**:
```json
{
  "cache_performance": {
    "query_distribution": {
      "tools_queries": 45.2,
      "resources_queries": 23.1,
      "server_info_queries": 31.7
    },
    "response_times": {
      "p50": "8ms",
      "p95": "25ms",
      "p99": "45ms"
    },
    "cache_efficiency": {
      "hit_rate": 0.94,
      "miss_rate": 0.06,
      "eviction_rate": 0.02
    }
  },
  "storage_details": {
    "table_sizes": {
      "servers": "12MB",
      "tools": "89MB",
      "resources": "34MB",
      "prompts": "21MB"
    },
    "compression_ratio": 0.35,
    "fragmentation_level": 0.08
  }
}
```

#### GET /api/debug/instance-status
**Purpose**: Get detailed instance status for debugging
**Status**: New endpoint (optional, for debugging)

**Response**:
```json
{
  "active_instances": {
    "production": [
      {
        "instance_id": "prod-server-123",
        "server_id": "server-123",
        "uptime": "2h 34m",
        "memory_usage": "45MB",
        "request_count": 1247,
        "last_activity": "2025-08-02T15:30:00Z"
      }
    ],
    "exploration": [
      {
        "instance_id": "exp-session-789",
        "server_id": "server-456",
        "session_id": "session-789",
        "ttl_remaining": "25m",
        "purpose": "configuration_testing"
      }
    ],
    "validation": []
  },
  "resource_usage": {
    "total_memory": "234MB",
    "total_cpu": "12%",
    "active_connections": 15,
    "connection_pool_utilization": 0.68
  }
}
```

### Removed Endpoints (Deprecated)

#### Capabilities Endpoints (Removed)
```
❌ GET /api/mcp/servers/{id}/capabilities
❌ GET /api/mcp/servers/capabilities/summary
❌ POST /api/mcp/servers/{id}/capabilities/refresh
❌ POST /api/mcp/servers/capabilities/batch-refresh
```

**Migration Path**: Use specific resource endpoints with `instance_type` parameter:
- `/capabilities` → `/tools`, `/resources`, `/prompts` with `instance_type=production`
- `/capabilities/refresh` → Use `instance_type=validation` for real-time data
- `/capabilities/summary` → Combine multiple endpoint calls or use server detail endpoint

#### Inspect Endpoints (Removed)
```
❌ All endpoints under /api/inspect/*
```

**Migration Path**: Use connection pool-based endpoints with appropriate `instance_type`

## Controllers

### Enhanced Server Controller

```rust
#[derive(Clone)]
pub struct EnhancedServerController {
    connection_pool: Arc<EnhancedConnectionPool>,
    cache_manager: Arc<RedbCacheManager>,
    conflict_analyzer: Arc<ConflictAnalyzer>,
    fingerprint_manager: Arc<FingerprintManager>,
    metrics: Arc<ControllerMetrics>,
}

impl EnhancedServerController {
    pub async fn list_servers(
        &self,
        query: ServerListQuery,
    ) -> Result<ServerListResponse, ControllerError> {
        let instance_type = query.instance_type.unwrap_or(InstanceType::Production);
        let cache_strategy = CacheStrategy::from(instance_type.clone());
        
        // Get servers from appropriate source based on instance type
        let servers = match instance_type {
            InstanceType::Production => {
                self.get_production_servers(cache_strategy).await?
            },
            InstanceType::Exploration { session_id, .. } => {
                self.get_exploration_servers(&session_id, cache_strategy).await?
            },
            InstanceType::Validation { session_id, .. } => {
                self.get_validation_servers(&session_id, cache_strategy).await?
            },
        };
        
        // Apply filtering
        let filtered_servers = self.apply_filters(servers, &query).await?;
        
        // Add cache info if requested
        let enriched_servers = if query.include_cache_info.unwrap_or(false) {
            self.enrich_with_cache_info(filtered_servers).await?
        } else {
            filtered_servers
        };
        
        // Apply pagination
        let paginated_result = self.paginate_servers(
            enriched_servers,
            query.page.unwrap_or(1),
            query.limit.unwrap_or(20),
        )?;
        
        Ok(paginated_result)
    }
    
    pub async fn create_server(
        &self,
        request: CreateServerRequest,
    ) -> Result<CreateServerResponse, ControllerError> {
        // Validate server configuration
        let validation_result = if request.validation_options.validate_before_save {
            Some(self.validate_server_config(&request.server_data).await?)
        } else {
            None
        };
        
        // Check for conflicts if requested
        let conflict_analysis = match request.validation_options.conflict_resolution {
            ConflictResolution::Auto | ConflictResolution::Manual => {
                Some(self.analyze_conflicts(&request.server_data).await?)
            },
            ConflictResolution::Skip => None,
        };
        
        // Handle conflicts based on strategy
        if let Some(conflicts) = &conflict_analysis {
            match request.validation_options.conflict_resolution {
                ConflictResolution::Auto => {
                    self.auto_resolve_conflicts(conflicts).await?;
                },
                ConflictResolution::Manual => {
                    if !conflicts.conflicts.is_empty() {
                        return Ok(CreateServerResponse::ConflictDetected {
                            conflicts: conflicts.clone(),
                            resolution_options: self.generate_resolution_options(conflicts).await?,
                        });
                    }
                },
                ConflictResolution::Skip => {},
            }
        }
        
        // Create server instance
        let server = self.create_server_instance(request.server_data).await?;
        
        // Initialize capabilities cache
        self.initialize_server_cache(&server.id).await?;
        
        Ok(CreateServerResponse::Success {
            server,
            validation_result,
            conflict_analysis,
        })
    }
    
    pub async fn get_server_tools(
        &self,
        server_id: &str,
        query: ToolsQuery,
    ) -> Result<ToolsResponse, ControllerError> {
        let instance_type = query.instance_type.unwrap_or(InstanceType::Production);
        let cache_strategy = CacheStrategy::from(instance_type.clone());
        
        // Get tools based on caching strategy
        let tools = match cache_strategy.freshness_level {
            FreshnessLevel::Cached => {
                self.get_cached_tools(server_id).await?
            },
            FreshnessLevel::RecentlyFresh => {
                self.get_fresh_or_cached_tools(server_id, Duration::minutes(5)).await?
            },
            FreshnessLevel::RealTime => {
                self.get_real_time_tools(server_id, instance_type).await?
            },
        };
        
        // Enrich with usage statistics if requested
        let enriched_tools = if query.include_usage_stats.unwrap_or(false) {
            self.enrich_tools_with_stats(tools).await?
        } else {
            tools
        };
        
        // Apply category filtering
        let filtered_tools = if let Some(category) = query.filter_by_category {
            self.filter_tools_by_category(enriched_tools, &category).await?
        } else {
            enriched_tools
        };
        
        Ok(ToolsResponse {
            tools: filtered_tools,
            instance_info: InstanceInfo::from(instance_type),
            cache_info: self.get_cache_info(server_id).await?,
        })
    }
}
```

### Runtime Controller

```rust
#[derive(Clone)]
pub struct RuntimeController {
    runtime_manager: Arc<RuntimeManager>,
    cache_manager: Arc<RedbCacheManager>,
    dependency_manager: Arc<DependencyManager>,
    metrics: Arc<RuntimeMetrics>,
}

impl RuntimeController {
    pub async fn get_runtime_status(&self) -> Result<RuntimeStatusResponse, ControllerError> {
        let node_status = self.runtime_manager.get_node_status().await?;
        let python_status = self.runtime_manager.get_python_status().await?;
        let cache_status = self.cache_manager.get_status().await?;
        let active_servers = self.get_active_servers_count().await?;
        
        Ok(RuntimeStatusResponse {
            runtime_status: RuntimeStatus {
                node_js: node_status,
                python: python_status,
            },
            cache_status,
            active_servers,
        })
    }
    
    pub async fn install_dependencies(
        &self,
        request: InstallDependenciesRequest,
    ) -> Result<InstallationResponse, ControllerError> {
        // Validate installation request
        self.validate_installation_request(&request).await?;
        
        // Check for existing installations
        let existing_deps = self.dependency_manager
            .check_existing_dependencies(&request.packages)
            .await?;
        
        // Install missing dependencies
        let installation_results = self.dependency_manager
            .install_packages(
                &request.runtime,
                &request.packages,
                &request.install_strategy,
            )
            .await?;
        
        // Update server cache if specific server provided
        if let Some(server_id) = &request.server_id {
            self.invalidate_server_cache(server_id).await?;
        }
        
        Ok(InstallationResponse {
            installed_packages: installation_results.installed,
            skipped_packages: installation_results.skipped,
            failed_packages: installation_results.failed,
            total_time: installation_results.duration,
        })
    }
    
    pub async fn clear_cache(
        &self,
        request: ClearCacheRequest,
    ) -> Result<ClearCacheResponse, ControllerError> {
        let mut results = ClearCacheResults::new();
        
        // Create backup if requested
        if request.backup_before_clear {
            let backup_path = self.create_cache_backup().await?;
            results.backup_created = Some(backup_path);
        }
        
        // Clear specified cache types
        for cache_type in &request.cache_types {
            match cache_type {
                CacheType::Redb => {
                    let cleared = self.clear_redb_cache(&request.server_ids).await?;
                    results.redb_cleared = Some(cleared);
                },
                CacheType::BuildArtifacts => {
                    let cleared = self.clear_build_artifacts(&request.server_ids).await?;
                    results.build_artifacts_cleared = Some(cleared);
                },
                CacheType::Dependencies => {
                    let cleared = self.clear_dependency_cache(&request.server_ids).await?;
                    results.dependencies_cleared = Some(cleared);
                },
            }
        }
        
        Ok(ClearCacheResponse {
            success: true,
            results,
            cleared_at: Utc::now(),
        })
    }
}
```

### Intelligent Configuration Controller

```rust
#[derive(Clone)]
pub struct IntelligentConfigController {
    llm_client: Arc<LLMClient>,
    server_analyzer: Arc<ServerAnalyzer>,
    conflict_resolver: Arc<ConflictResolver>,
    marketplace_client: Arc<MarketplaceClient>,
    metrics: Arc<IntelligenceMetrics>,
}

impl IntelligentConfigController {
    pub async fn create_intelligent_suit(
        &self,
        request: IntelligentCreateRequest,
    ) -> Result<IntelligentCreateResponse, ControllerError> {
        // Analyze user intent
        let intent_analysis = self.analyze_user_intent(&request.user_intent).await?;
        
        // Search for relevant servers
        let candidate_servers = self.search_candidate_servers(
            &intent_analysis,
            &request.context,
        ).await?;
        
        // Analyze conflicts with existing servers
        let conflict_analysis = self.analyze_potential_conflicts(
            &candidate_servers,
            &request.context.existing_servers,
        ).await?;
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(
            candidate_servers,
            conflict_analysis,
            &request.creation_options,
        ).await?;
        
        // Auto-resolve conflicts if requested
        let final_recommendations = if request.creation_options.auto_resolve_conflicts {
            self.auto_resolve_recommendations(recommendations).await?
        } else {
            recommendations
        };
        
        // Generate suit configuration
        let suit_config = self.generate_suit_configuration(
            &final_recommendations,
            &request.context,
        ).await?;
        
        Ok(IntelligentCreateResponse {
            intelligent_recommendation: final_recommendations,
            suit_configuration: suit_config,
        })
    }
    
    pub async fn validate_suit_before_save(
        &self,
        request: ValidateBeforeSaveRequest,
    ) -> Result<ValidationResponse, ControllerError> {
        let mut validation_result = ValidationResult::new();
        
        // Validate configuration syntax
        if request.validation_options.check_conflicts {
            let conflicts = self.check_configuration_conflicts(
                &request.suit_configuration
            ).await?;
            validation_result.conflict_analysis = Some(conflicts);
        }
        
        // Validate commands if requested
        if request.validation_options.validate_commands {
            let command_validation = self.validate_server_commands(
                &request.suit_configuration.servers
            ).await?;
            validation_result.command_validation = Some(command_validation);
        }
        
        // Test connectivity if requested
        if request.validation_options.test_connectivity {
            let connectivity_test = self.test_server_connectivity(
                &request.suit_configuration.servers
            ).await?;
            validation_result.connectivity_test = Some(connectivity_test);
        }
        
        // Analyze performance impact
        if request.validation_options.analyze_performance_impact {
            let performance_analysis = self.analyze_performance_impact(
                &request.suit_configuration
            ).await?;
            validation_result.performance_impact = Some(performance_analysis);
        }
        
        // Generate optimization suggestions
        let optimization_suggestions = self.generate_optimization_suggestions(
            &request.suit_configuration,
            &validation_result,
        ).await?;
        
        Ok(ValidationResponse {
            validation_result,
            optimization_suggestions,
        })
    }
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error("Server not found: {server_id}")]
    ServerNotFound { server_id: String },
    
    #[error("Invalid instance type for operation: {instance_type:?}")]
    InvalidInstanceType { instance_type: InstanceType },
    
    #[error("Cache operation failed: {message}")]
    CacheError { message: String },
    
    #[error("Connection pool error: {message}")]
    ConnectionPoolError { message: String },
    
    #[error("Validation failed: {errors:?}")]
    ValidationError { errors: Vec<ValidationError> },
    
    #[error("Conflict resolution required: {conflicts:?}")]
    ConflictResolutionRequired { conflicts: Vec<ConflictInfo> },
    
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },
    
    #[error("Intelligence service error: {message}")]
    IntelligenceError { message: String },
}

// Convert to HTTP responses
impl From<ControllerError> for StatusCode {
    fn from(error: ControllerError) -> Self {
        match error {
            ControllerError::ServerNotFound { .. } => StatusCode::NOT_FOUND,
            ControllerError::InvalidInstanceType { .. } => StatusCode::BAD_REQUEST,
            ControllerError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            ControllerError::ConflictResolutionRequired { .. } => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

This API specification provides a comprehensive, backward-compatible interface that supports the enhanced capabilities database integration while maintaining simplicity and performance. The intelligent caching, conflict resolution, and runtime management features are seamlessly integrated into a unified API surface.