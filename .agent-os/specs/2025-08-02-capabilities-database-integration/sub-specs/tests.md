# Tests Specification

This is the tests coverage details for the spec detailed in @.agent-os/specs/2025-08-02-capabilities-database-integration/spec.md

> Created: 2025-08-02
> Version: 1.0.0

## Test Coverage

### Unit Tests

#### Redb Cache Manager Tests

**File**: `tests/unit/cache/redb_cache_manager_test.rs`

```rust
#[cfg(test)]
mod redb_cache_manager_tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_test;
    
    #[tokio::test]
    async fn test_store_and_retrieve_server_data() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = RedbCacheManager::new(
            temp_dir.path().join("test.redb"),
            CacheConfig::default()
        ).await.unwrap();
        
        let server_data = CachedServerData {
            server_id: "test-server".to_string(),
            name: "Test Server".to_string(),
            description: Some("Test description".to_string()),
            // ... other fields
        };
        
        // Test store operation
        cache_manager.store_server(server_data.clone()).await.unwrap();
        
        // Test retrieve operation
        let retrieved = cache_manager.get_server("test-server").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Server");
    }
    
    #[tokio::test]
    async fn test_batch_store_capabilities() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = RedbCacheManager::new(
            temp_dir.path().join("test.redb"),
            CacheConfig::default()
        ).await.unwrap();
        
        let tools = vec![
            CachedToolData {
                server_id: "test-server".to_string(),
                name: "tool1".to_string(),
                description: Some("Tool 1".to_string()),
                // ... other fields
            },
            CachedToolData {
                server_id: "test-server".to_string(),
                name: "tool2".to_string(),
                description: Some("Tool 2".to_string()),
                // ... other fields
            },
        ];
        
        let resources = vec![];
        let prompts = vec![];
        
        // Test batch store
        cache_manager.batch_store_capabilities(
            "test-server",
            tools,
            resources,
            prompts
        ).await.unwrap();
        
        // Verify tools were stored
        let retrieved_tools = cache_manager.get_server_tools("test-server").await.unwrap();
        assert_eq!(retrieved_tools.len(), 2);
        assert_eq!(retrieved_tools[0].name, "tool1");
        assert_eq!(retrieved_tools[1].name, "tool2");
    }
    
    #[tokio::test]
    async fn test_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = Arc::new(RedbCacheManager::new(
            temp_dir.path().join("test.redb"),
            CacheConfig::default()
        ).await.unwrap());
        
        let mut handles = vec![];
        
        // Spawn multiple concurrent operations
        for i in 0..10 {
            let cache_manager = cache_manager.clone();
            let handle = tokio::spawn(async move {
                let server_data = CachedServerData {
                    server_id: format!("server-{}", i),
                    name: format!("Server {}", i),
                    // ... other fields
                };
                cache_manager.store_server(server_data).await.unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all servers were stored
        for i in 0..10 {
            let server = cache_manager.get_server(&format!("server-{}", i)).await.unwrap();
            assert!(server.is_some());
        }
    }
    
    #[tokio::test]
    async fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = RedbCacheManager::new(
            temp_dir.path().join("test.redb"),
            CacheConfig::default()
        ).await.unwrap();
        
        // Store initial data
        let server_data = CachedServerData {
            server_id: "test-server".to_string(),
            fingerprint: "old-fingerprint".to_string(),
            // ... other fields
        };
        cache_manager.store_server(server_data).await.unwrap();
        
        // Test fingerprint-based invalidation
        let is_valid = cache_manager.is_cache_valid(
            "test-server",
            "new-fingerprint"
        ).await.unwrap();
        assert!(!is_valid);
        
        // Test TTL-based invalidation
        let expired_data = CachedServerData {
            server_id: "expired-server".to_string(),
            cached_at: Utc::now() - Duration::hours(2),
            // ... other fields
        };
        cache_manager.store_server(expired_data).await.unwrap();
        
        let is_fresh = cache_manager.is_cache_fresh(
            "expired-server",
            Duration::minutes(30)
        ).await.unwrap();
        assert!(!is_fresh);
    }
}
```

#### Connection Pool Enhancement Tests

**File**: `tests/unit/pool/enhanced_connection_pool_test.rs`

```rust
#[cfg(test)]
mod enhanced_connection_pool_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_instance_type_isolation() {
        let pool = EnhancedConnectionPool::new(PoolConfig::default()).await;
        
        // Create production instance
        let prod_instance = pool.get_or_create_instance(
            "server-1",
            InstanceType::Production
        ).await.unwrap();
        
        // Create exploration instance
        let exp_instance = pool.get_or_create_instance(
            "server-1",
            InstanceType::Exploration {
                session_id: "session-1".to_string(),
                ttl_minutes: 30,
            }
        ).await.unwrap();
        
        // Verify instances are separate
        assert_ne!(prod_instance.instance_id(), exp_instance.instance_id());
        
        // Verify visibility rules
        let visible_instances = pool.get_visible_instances().await.unwrap();
        assert_eq!(visible_instances.len(), 1); // Only production visible
        assert_eq!(visible_instances[0].instance_id(), prod_instance.instance_id());
    }
    
    #[tokio::test]
    async fn test_ttl_cleanup() {
        let pool = EnhancedConnectionPool::new(PoolConfig::default()).await;
        
        // Create short-lived validation instance
        let validation_instance = pool.get_or_create_instance(
            "server-1",
            InstanceType::Validation {
                session_id: "validation-1".to_string(),
                ttl_minutes: 1, // Very short TTL for testing
            }
        ).await.unwrap();
        
        let instance_id = validation_instance.instance_id().clone();
        
        // Wait for TTL to expire
        tokio::time::sleep(Duration::seconds(65)).await;
        
        // Trigger cleanup
        pool.cleanup_expired_instances().await.unwrap();
        
        // Verify instance was cleaned up
        let instance = pool.get_instance(&instance_id).await;
        assert!(instance.is_none());
    }
    
    #[tokio::test]
    async fn test_concurrent_instance_creation() {
        let pool = Arc::new(EnhancedConnectionPool::new(PoolConfig::default()).await);
        let mut handles = vec![];
        
        // Spawn multiple concurrent instance creation requests
        for i in 0..10 {
            let pool = pool.clone();
            let handle = tokio::spawn(async move {
                pool.get_or_create_instance(
                    &format!("server-{}", i),
                    InstanceType::Production
                ).await.unwrap()
            });
            handles.push(handle);
        }
        
        // Wait for all instances to be created
        let instances: Vec<_> = futures::future::try_join_all(handles).await.unwrap();
        
        // Verify all instances were created successfully
        assert_eq!(instances.len(), 10);
        
        // Verify each instance has unique ID
        let mut instance_ids = std::collections::HashSet::new();
        for instance in instances {
            assert!(instance_ids.insert(instance.instance_id().clone()));
        }
    }
}
```

#### Fingerprinting System Tests

**File**: `tests/unit/fingerprint/fingerprint_manager_test.rs`

```rust
#[cfg(test)]
mod fingerprint_manager_tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_code_fingerprint_generation() {
        let temp_dir = TempDir::new().unwrap();
        let server_path = temp_dir.path().join("test-server");
        std::fs::create_dir_all(&server_path).unwrap();
        
        // Create test files
        std::fs::write(server_path.join("server.js"), "console.log('hello');").unwrap();
        std::fs::write(server_path.join("package.json"), r#"{"name": "test"}"#).unwrap();
        
        let fingerprint_manager = FingerprintManager::new();
        let fingerprint = fingerprint_manager
            .generate_fingerprint(&server_path)
            .await
            .unwrap();
        
        // Verify fingerprint structure
        assert!(!fingerprint.code_fingerprint.file_hashes.is_empty());
        assert_eq!(fingerprint.code_fingerprint.total_files, 2);
        assert!(!fingerprint.combined_hash.is_empty());
    }
    
    #[tokio::test]
    async fn test_fingerprint_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let server_path = temp_dir.path().join("test-server");
        std::fs::create_dir_all(&server_path).unwrap();
        
        // Create initial file
        std::fs::write(server_path.join("server.js"), "console.log('hello');").unwrap();
        
        let fingerprint_manager = FingerprintManager::new();
        
        // Generate initial fingerprint
        let initial_fingerprint = fingerprint_manager
            .generate_fingerprint(&server_path)
            .await
            .unwrap();
        
        // Modify file
        std::fs::write(server_path.join("server.js"), "console.log('modified');").unwrap();
        
        // Generate new fingerprint
        let new_fingerprint = fingerprint_manager
            .generate_fingerprint(&server_path)
            .await
            .unwrap();
        
        // Compare fingerprints
        let diff = fingerprint_manager
            .compare_fingerprints(&initial_fingerprint, &new_fingerprint)
            .await;
        
        match diff.change_type {
            ChangeType::CodeChange { files_changed } => {
                assert_eq!(files_changed.len(), 1);
                assert!(files_changed[0].ends_with("server.js"));
            },
            _ => panic!("Expected CodeChange"),
        }
    }
    
    #[tokio::test]
    async fn test_dependency_fingerprint() {
        let temp_dir = TempDir::new().unwrap();
        let server_path = temp_dir.path().join("test-server");
        std::fs::create_dir_all(&server_path).unwrap();
        
        // Create package.json
        let package_json = r#"{
            "name": "test-server",
            "dependencies": {
                "express": "^4.18.0",
                "lodash": "^4.17.21"
            }
        }"#;
        std::fs::write(server_path.join("package.json"), package_json).unwrap();
        
        // Create package-lock.json
        let package_lock = r#"{
            "name": "test-server",
            "lockfileVersion": 2,
            "packages": {
                "node_modules/express": {
                    "version": "4.18.2"
                }
            }
        }"#;
        std::fs::write(server_path.join("package-lock.json"), package_lock).unwrap();
        
        let fingerprint_manager = FingerprintManager::new();
        let fingerprint = fingerprint_manager
            .generate_fingerprint(&server_path)
            .await
            .unwrap();
        
        // Verify dependency fingerprint
        assert!(fingerprint.dependency_fingerprint.package_lock_hash.is_some());
        assert!(!fingerprint.dependency_fingerprint.manifest_hash.is_empty());
        assert!(!fingerprint.dependency_fingerprint.resolved_versions.is_empty());
    }
}
```

#### Conflict Detection Tests

**File**: `tests/unit/conflict/conflict_analyzer_test.rs`

```rust
#[cfg(test)]
mod conflict_analyzer_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_similarity_detection() {
        let analyzer = ConflictAnalyzer::new(ConflictConfig::default());
        
        let tool1 = ToolInfo {
            name: "read_file".to_string(),
            description: Some("Read contents of a file".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        };
        
        let tool2 = ToolInfo {
            name: "file_read".to_string(),
            description: Some("Read file contents".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                }
            }),
        };
        
        let similarity = analyzer.analyze_tool_similarity(&tool1, &tool2).await.unwrap();
        
        // Should detect high similarity despite different names
        assert!(similarity > 0.7);
    }
    
    #[tokio::test]
    async fn test_conflict_recommendation_generation() {
        let analyzer = ConflictAnalyzer::new(ConflictConfig::default());
        
        let server1_tools = vec![
            ToolInfo {
                name: "read_file".to_string(),
                description: Some("Read file contents".to_string()),
                // ... other fields
            }
        ];
        
        let server2_tools = vec![
            ToolInfo {
                name: "file_reader".to_string(),
                description: Some("Read contents from file".to_string()),
                // ... other fields
            }
        ];
        
        let conflict_report = analyzer.analyze_server_conflicts(
            "server-1",
            &server1_tools,
            "server-2",
            &server2_tools
        ).await.unwrap();
        
        assert!(conflict_report.similarity_score > 0.5);
        
        match conflict_report.recommendation {
            SmartRecommendation::UserChoice { options, .. } => {
                assert!(!options.is_empty());
            },
            _ => {}, // Other recommendations are also valid
        }
    }
    
    #[tokio::test]
    async fn test_no_false_positives() {
        let analyzer = ConflictAnalyzer::new(ConflictConfig::default());
        
        let file_tool = ToolInfo {
            name: "read_file".to_string(),
            description: Some("Read file contents".to_string()),
            // ... other fields
        };
        
        let math_tool = ToolInfo {
            name: "calculate".to_string(),
            description: Some("Perform mathematical calculations".to_string()),
            // ... other fields
        };
        
        let similarity = analyzer.analyze_tool_similarity(&file_tool, &math_tool).await.unwrap();
        
        // Should detect low similarity for completely different tools
        assert!(similarity < 0.3);
    }
}
```

### Integration Tests

#### End-to-End API Tests

**File**: `tests/integration/api/server_management_test.rs`

```rust
#[cfg(test)]
mod server_management_integration_tests {
    use super::*;
    use axum_test::TestServer;
    
    #[tokio::test]
    async fn test_server_creation_with_validation() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let create_request = serde_json::json!({
            "source_type": "manual",
            "server_data": {
                "name": "Test Server",
                "description": "Integration test server",
                "command": "node test-server.js",
                "args": [],
                "env": {},
                "working_directory": "/tmp/test-server"
            },
            "validation_options": {
                "validate_before_save": true,
                "conflict_resolution": "manual",
                "instance_type": "validation"
            }
        });
        
        let response = server
            .post("/api/mcp/servers")
            .json(&create_request)
            .await;
        
        response.assert_status_ok();
        
        let response_body: serde_json::Value = response.json();
        assert!(response_body["server"]["id"].is_string());
        assert_eq!(response_body["server"]["name"], "Test Server");
        assert!(response_body["validation_result"]["is_valid"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_instance_type_parameter_handling() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        // First create a server
        let server_id = create_test_server(&server).await;
        
        // Test production instance (default)
        let response = server
            .get(&format!("/api/mcp/servers/{}/tools", server_id))
            .await;
        response.assert_status_ok();
        let body: serde_json::Value = response.json();
        assert_eq!(body["instance_info"]["type"], "production");
        
        // Test exploration instance
        let response = server
            .get(&format!("/api/mcp/servers/{}/tools?instance_type=exploration&session_id=test-session", server_id))
            .await;
        response.assert_status_ok();
        let body: serde_json::Value = response.json();
        assert_eq!(body["instance_info"]["type"], "exploration");
        assert_eq!(body["instance_info"]["session_id"], "test-session");
        
        // Test validation instance
        let response = server
            .get(&format!("/api/mcp/servers/{}/tools?instance_type=validation&session_id=validation-session", server_id))
            .await;
        response.assert_status_ok();
        let body: serde_json::Value = response.json();
        assert_eq!(body["instance_info"]["type"], "validation");
    }
    
    #[tokio::test]
    async fn test_cache_performance_improvement() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let server_id = create_test_server(&server).await;
        
        // First request (cache miss)
        let start = std::time::Instant::now();
        let response = server
            .get(&format!("/api/mcp/servers/{}/tools", server_id))
            .await;
        let first_request_time = start.elapsed();
        response.assert_status_ok();
        
        // Second request (cache hit)
        let start = std::time::Instant::now();
        let response = server
            .get(&format!("/api/mcp/servers/{}/tools", server_id))
            .await;
        let second_request_time = start.elapsed();
        response.assert_status_ok();
        
        // Cache hit should be significantly faster
        assert!(second_request_time < first_request_time / 2);
        
        let body: serde_json::Value = response.json();
        assert_eq!(body["cache_info"]["is_fresh"], true);
    }
    
    #[tokio::test]
    async fn test_concurrent_requests() {
        let app = create_test_app().await;
        let server = Arc::new(TestServer::new(app).unwrap());
        
        let server_id = create_test_server(&server).await;
        
        let mut handles = vec![];
        
        // Spawn multiple concurrent requests
        for i in 0..20 {
            let server = server.clone();
            let server_id = server_id.clone();
            let handle = tokio::spawn(async move {
                let response = server
                    .get(&format!("/api/mcp/servers/{}/tools?session_id=concurrent-{}", server_id, i))
                    .await;
                response.assert_status_ok();
                response.json::<serde_json::Value>()
            });
            handles.push(handle);
        }
        
        // Wait for all requests to complete
        let results = futures::future::try_join_all(handles).await.unwrap();
        
        // Verify all requests succeeded
        assert_eq!(results.len(), 20);
        for result in results {
            assert!(result["tools"].is_array());
        }
    }
}
```

#### Migration Tests

**File**: `tests/integration/migration/json_to_redb_test.rs`

```rust
#[cfg(test)]
mod migration_integration_tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_complete_migration_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let json_cache_dir = temp_dir.path().join("json_cache");
        let redb_path = temp_dir.path().join("cache.redb");
        
        // Create test JSON cache files
        std::fs::create_dir_all(&json_cache_dir).unwrap();
        create_test_json_cache_files(&json_cache_dir).await;
        
        // Initialize Redb cache manager
        let redb_manager = RedbCacheManager::new(&redb_path, CacheConfig::default())
            .await
            .unwrap();
        
        // Create migrator
        let migrator = JsonToRedbMigrator::new(
            json_cache_dir,
            redb_manager,
            MigrationConfig {
                batch_size: 10,
                validate_data: true,
                backup_json: true,
                parallel_processing: true,
                max_concurrent_files: 4,
            }
        );
        
        // Run migration
        let migration_report = migrator.migrate_all().await.unwrap();
        
        // Verify migration results
        assert!(migration_report.success);
        assert_eq!(migration_report.files_processed, 5); // Assuming 5 test files
        assert_eq!(migration_report.records_migrated, 15); // Assuming 15 total records
        assert!(migration_report.size_reduction_percentage > 50.0);
        
        // Verify data integrity
        let validation_result = migration_report.validation_result.unwrap();
        assert!(validation_result.data_integrity_check_passed);
        assert_eq!(validation_result.missing_records, 0);
        assert_eq!(validation_result.corrupted_records, 0);
    }
    
    #[tokio::test]
    async fn test_migration_performance() {
        let temp_dir = TempDir::new().unwrap();
        let json_cache_dir = temp_dir.path().join("json_cache");
        let redb_path = temp_dir.path().join("cache.redb");
        
        // Create large test dataset
        std::fs::create_dir_all(&json_cache_dir).unwrap();
        create_large_test_dataset(&json_cache_dir, 100).await; // 100 servers
        
        let redb_manager = RedbCacheManager::new(&redb_path, CacheConfig::default())
            .await
            .unwrap();
        
        let migrator = JsonToRedbMigrator::new(
            json_cache_dir,
            redb_manager,
            MigrationConfig {
                batch_size: 20,
                validate_data: false, // Skip validation for performance test
                backup_json: false,
                parallel_processing: true,
                max_concurrent_files: 8,
            }
        );
        
        let start_time = std::time::Instant::now();
        let migration_report = migrator.migrate_all().await.unwrap();
        let migration_time = start_time.elapsed();
        
        // Verify performance expectations
        assert!(migration_time < std::time::Duration::from_secs(30)); // Should complete in under 30 seconds
        assert!(migration_report.average_file_processing_time < std::time::Duration::from_millis(500));
        
        // Verify storage efficiency
        let json_size = calculate_directory_size(&json_cache_dir).await;
        let redb_size = std::fs::metadata(&redb_path).unwrap().len();
        let compression_ratio = (redb_size as f64) / (json_size as f64);
        
        assert!(compression_ratio < 0.5); // At least 50% size reduction
    }
    
    async fn create_test_json_cache_files(cache_dir: &Path) {
        // Create sample JSON cache files for testing
        for i in 1..=5 {
            let server_data = serde_json::json!({
                "server_id": format!("server-{}", i),
                "name": format!("Test Server {}", i),
                "description": format!("Test server {} description", i),
                "cached_at": "2025-08-02T10:00:00Z",
                "tools": [
                    {
                        "name": "tool1",
                        "description": "Test tool 1",
                        "input_schema": {"type": "object"}
                    },
                    {
                        "name": "tool2",
                        "description": "Test tool 2",
                        "input_schema": {"type": "object"}
                    }
                ],
                "resources": [],
                "prompts": []
            });
            
            let file_path = cache_dir.join(format!("server-{}.json", i));
            tokio::fs::write(file_path, serde_json::to_string_pretty(&server_data).unwrap())
                .await
                .unwrap();
        }
    }
}
```

### Performance Tests

#### Cache Performance Benchmarks

**File**: `tests/performance/cache_benchmarks.rs`

```rust
#[cfg(test)]
mod cache_performance_tests {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use tempfile::TempDir;
    
    fn benchmark_redb_vs_json_read(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        // Setup test data
        let temp_dir = TempDir::new().unwrap();
        let (redb_manager, json_cache_dir) = rt.block_on(async {
            setup_benchmark_data(&temp_dir).await
        });
        
        let mut group = c.benchmark_group("cache_read_performance");
        
        // Benchmark Redb read
        group.bench_function("redb_read", |b| {
            b.to_async(&rt).iter(|| async {
                let server_data = redb_manager.get_server("benchmark-server").await.unwrap();
                black_box(server_data);
            });
        });
        
        // Benchmark JSON read
        group.bench_function("json_read", |b| {
            b.to_async(&rt).iter(|| async {
                let json_path = json_cache_dir.join("benchmark-server.json");
                let json_content = tokio::fs::read_to_string(json_path).await.unwrap();
                let server_data: serde_json::Value = serde_json::from_str(&json_content).unwrap();
                black_box(server_data);
            });
        });
        
        group.finish();
    }
    
    fn benchmark_concurrent_access(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let temp_dir = TempDir::new().unwrap();
        let redb_manager = rt.block_on(async {
            let redb_path = temp_dir.path().join("concurrent.redb");
            let manager = RedbCacheManager::new(&redb_path, CacheConfig::default()).await.unwrap();
            
            // Populate with test data
            for i in 0..100 {
                let server_data = create_test_server_data(&format!("server-{}", i));
                manager.store_server(server_data).await.unwrap();
            }
            
            Arc::new(manager)
        });
        
        c.bench_function("concurrent_reads", |b| {
            b.to_async(&rt).iter(|| async {
                let manager = redb_manager.clone();
                let mut handles = vec![];
                
                for i in 0..10 {
                    let manager = manager.clone();
                    let handle = tokio::spawn(async move {
                        manager.get_server(&format!("server-{}", i)).await.unwrap()
                    });
                    handles.push(handle);
                }
                
                let results = futures::future::try_join_all(handles).await.unwrap();
                black_box(results);
            });
        });
    }
    
    async fn setup_benchmark_data(temp_dir: &TempDir) -> (Arc<RedbCacheManager>, PathBuf) {
        // Setup Redb
        let redb_path = temp_dir.path().join("benchmark.redb");
        let redb_manager = Arc::new(
            RedbCacheManager::new(&redb_path, CacheConfig::default()).await.unwrap()
        );
        
        // Setup JSON cache
        let json_cache_dir = temp_dir.path().join("json_cache");
        std::fs::create_dir_all(&json_cache_dir).unwrap();
        
        // Create test data
        let server_data = create_large_test_server_data("benchmark-server");
        
        // Store in Redb
        redb_manager.store_server(server_data.clone()).await.unwrap();
        
        // Store in JSON
        let json_path = json_cache_dir.join("benchmark-server.json");
        let json_content = serde_json::to_string_pretty(&server_data).unwrap();
        tokio::fs::write(json_path, json_content).await.unwrap();
        
        (redb_manager, json_cache_dir)
    }
    
    criterion_group!(benches, benchmark_redb_vs_json_read, benchmark_concurrent_access);
    criterion_main!(benches);
}
```

## Mocking Requirements

### Mock MCP Server

**File**: `tests/mocks/mock_mcp_server.rs`

```rust
pub struct MockMcpServer {
    pub server_id: String,
    pub tools: Vec<ToolInfo>,
    pub resources: Vec<ResourceInfo>,
    pub prompts: Vec<PromptInfo>,
    pub response_delay: Duration,
    pub failure_rate: f64,
}

impl MockMcpServer {
    pub fn new(server_id: &str) -> Self {
        Self {
            server_id: server_id.to_string(),
            tools: vec![
                ToolInfo {
                    name: "mock_tool_1".to_string(),
                    description: Some("Mock tool for testing".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": {"type": "string"}
                        }
                    }),
                },
                ToolInfo {
                    name: "mock_tool_2".to_string(),
                    description: Some("Another mock tool".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "data": {"type": "object"}
                        }
                    }),
                },
            ],
            resources: vec![
                ResourceInfo {
                    uri: "mock://resource1".to_string(),
                    name: Some("Mock Resource 1".to_string()),
                    description: Some("Test resource".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    annotations: None,
                },
            ],
            prompts: vec![
                PromptInfo {
                    name: "mock_prompt".to_string(),
                    description: Some("Mock prompt for testing".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "context".to_string(),
                            description: Some("Context for the prompt".to_string()),
                            required: true,
                        },
                    ],
                },
            ],
            response_delay: Duration::from_millis(100),
            failure_rate: 0.0,
        }
    }
    
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }
    
    pub fn with_failure_rate(mut self, rate: f64) -> Self {
        self.failure_rate = rate;
        self
    }
    
    pub async fn get_capabilities(&self) -> Result<ServerCapabilities, MockError> {
        // Simulate network delay
        tokio::time::sleep(self.response_delay).await;
        
        // Simulate random failures
        if rand::random::<f64>() < self.failure_rate {
            return Err(MockError::SimulatedFailure);
        }
        
        Ok(ServerCapabilities {
            tools: self.tools.clone(),
            resources: self.resources.clone(),
            prompts: self.prompts.clone(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MockError {
    #[error("Simulated failure for testing")]
    SimulatedFailure,
}
```

### Mock LLM Client

**File**: `tests/mocks/mock_llm_client.rs`

```rust
pub struct MockLLMClient {
    pub responses: HashMap<String, String>,
    pub response_delay: Duration,
    pub failure_scenarios: Vec<String>,
}

impl MockLLMClient {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        
        // Pre-configured responses for common test scenarios
        responses.insert(
            "file management".to_string(),
            "I recommend using a file management server with tools for reading, writing, and organizing files.".to_string()
        );
        
        responses.insert(
            "git operations".to_string(),
            "For git operations, I suggest a git integration server with commit, push, and branch management tools.".to_string()
        );
        
        Self {
            responses,
            response_delay: Duration::from_millis(500),
            failure_scenarios: vec![],
        }
    }
    
    pub fn with_response(mut self, query: &str, response: &str) -> Self {
        self.responses.insert(query.to_string(), response.to_string());
        self
    }
    
    pub fn with_failure_scenario(mut self, scenario: &str) -> Self {
        self.failure_scenarios.push(scenario.to_string());
        self
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn analyze_user_intent(&self, user_intent: &str) -> Result<IntentAnalysis, LLMError> {
        tokio::time::sleep(self.response_delay).await;
        
        if self.failure_scenarios.iter().any(|s| user_intent.contains(s)) {
            return Err(LLMError::ServiceUnavailable);
        }
        
        // Simple keyword-based mock analysis
        let keywords = if user_intent.to_lowercase().contains("file") {
            vec!["file_management".to_string(), "io_operations".to_string()]
        } else if user_intent.to_lowercase().contains("git") {
            vec!["version_control".to_string(), "git_operations".to_string()]
        } else {
            vec!["general_purpose".to_string()]
        };
        
        Ok(IntentAnalysis {
            primary_intent: keywords[0].clone(),
            secondary_intents: keywords[1..].to_vec(),
            confidence_score: 0.85,
            suggested_categories: vec!["development_tools".to_string()],
        })
    }
    
    async fn generate_server_recommendation(
        &self,
        intent: &IntentAnalysis,
        context: &RecommendationContext,
    ) -> Result<ServerRecommendation, LLMError> {
        tokio::time::sleep(self.response_delay).await;
        
        let response_text = self.responses
            .get(&intent.primary_intent)
            .cloned()
            .unwrap_or_else(|| "I recommend exploring available servers in the marketplace.".to_string());
        
        Ok(ServerRecommendation {
            recommended_servers: vec![
                RecommendedServer {
                    name: "Mock Recommended Server".to_string(),
                    description: response_text,
                    confidence_score: 0.8,
                    reasoning: "Based on mock analysis".to_string(),
                    source: ServerSource::Marketplace,
                }
            ],
            alternative_approaches: vec![],
        })
    }
}
```

### Test Utilities

**File**: `tests/utils/test_helpers.rs`

```rust
pub async fn create_test_app() -> Router {
    let config = AppConfig::test_config();
    let app_state = AppState::new_for_testing(config).await;
    create_app(app_state)
}

pub async fn create_test_server(test_server: &TestServer) -> String {
    let create_request = serde_json::json!({
        "source_type": "manual",
        "server_data": {
            "name": "Test Server",
            "description": "Test server for integration tests",
            "command": "node test-server.js",
            "args": [],
            "env": {},
            "working_directory": "/tmp/test-server"
        },
        "validation_options": {
            "validate_before_save": false,
            "conflict_resolution": "skip"
        }
    });
    
    let response = test_server
        .post("/api/mcp/servers")
        .json(&create_request)
        .await;
    
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    body["server"]["id"].as_str().unwrap().to_string()
}

pub fn create_test_server_data(server_id: &str) -> CachedServerData {
    CachedServerData {
        server_id: server_id.to_string(),
        name: format!("Test Server {}", server_id),
        description: Some("Test server description".to_string()),
        version: Some("1.0.0".to_string()),
        status: ServerStatus::Active,
        connection_info: ConnectionInfo {
            transport: TransportType::Stdio,
            command: Some("node".to_string()),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            working_directory: None,
            timeout: Some(Duration::from_secs(30)),
        },
        cached_at: Utc::now(),
        last_accessed: Utc::now(),
        fingerprint: "test-fingerprint".to_string(),
        instance_type: InstanceType::Production,
    }
}

pub async fn setup_test_database() -> RedbCacheManager {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.redb");
    
    RedbCacheManager::new(&db_path, CacheConfig::test_config())
        .await
        .unwrap()
}

pub fn assert_performance_improvement(
    baseline_time: Duration,
    optimized_time: Duration,
    expected_improvement_factor: f64,
) {
    let improvement_factor = baseline_time.as_secs_f64() / optimized_time.as_secs_f64();
    assert!(
        improvement_factor >= expected_improvement_factor,
        "Expected {}x improvement, got {}x (baseline: {:?}, optimized: {:?})",
        expected_improvement_factor,
        improvement_factor,
        baseline_time,
        optimized_time
    );
}
```

## Test Execution Strategy

### Continuous Integration Pipeline

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: cargo test --lib
      
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run integration tests
        run: cargo test --test '*'
        
  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run performance benchmarks
        run: cargo bench
      - name: Upload benchmark results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/
```

### Local Development Testing

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --lib                    # Unit tests only
cargo test --test integration       # Integration tests only
cargo test --test performance       # Performance tests only

# Run tests with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench

# Run tests with specific features
cargo test --features "debug-mode"
```

### Test Data Management

```rust
// Test data fixtures
pub struct TestDataFixtures {
    pub servers: Vec<CachedServerData>,
    pub tools: HashMap<String, Vec<CachedToolData>>,
    pub conflicts: Vec<ConflictTestCase>,
}

impl TestDataFixtures {
    pub fn load() -> Self {
        // Load test data from fixtures files
        Self {
            servers: load_server_fixtures(),
            tools: load_tool_fixtures(),
            conflicts: load_conflict_fixtures(),
        }
    }
    
    pub fn create_minimal() -> Self {
        // Create minimal test data for fast tests
        Self {
            servers: vec![create_test_server_data("minimal-server")],
            tools: HashMap::new(),
            conflicts: vec![],
        }
    }
}
```

This comprehensive test specification ensures thorough coverage of all aspects of the capabilities database integration refactor, from individual component testing to end-to-end performance validation.