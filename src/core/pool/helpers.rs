//! Pool helper methods for UpstreamConnectionPool
//!
//! Contains helper methods for managing connection instances and utilities within the pool.
//! These methods provide convenient access patterns for getting and manipulating
//! server instances, status queries, and connection state management.

use crate::core::connection::UpstreamConnection;
use crate::core::foundation::types::{
    ConnectionStatus, // status of the connection
    ErrorType,        // type of the error
};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Helper methods for managing connection instances and utilities
impl super::UpstreamConnectionPool {
    // ========================================
    // Instance Access Methods
    // ========================================

    /// Helper method to get a specific instance of a server
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    /// * `instance_id` - ID of the specific instance
    ///
    /// # Returns
    /// * `Ok(&UpstreamConnection)` - Reference to the connection instance
    /// * `Err(...)` - If server or instance not found
    pub fn get_instance(
        &self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<&UpstreamConnection> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        instances.get(instance_id).context(format!(
            "Instance '{instance_id}' not found for server '{server_name}'"
        ))
    }

    /// Helper method to get a mutable reference to a specific instance of a server
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    /// * `instance_id` - ID of the specific instance
    ///
    /// # Returns
    /// * `Ok(&mut UpstreamConnection)` - Mutable reference to the connection instance
    /// * `Err(...)` - If server or instance not found
    pub fn get_instance_mut(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<&mut UpstreamConnection> {
        let instances = self.connections.get_mut(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        instances.get_mut(instance_id).context(format!(
            "Instance '{instance_id}' not found for server '{server_name}'"
        ))
    }

    /// Helper method to get the default instance of a server
    ///
    /// Returns the first available instance for a server. In the future,
    /// this could be enhanced with more sophisticated selection logic.
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok((String, &UpstreamConnection))` - Tuple of (instance_id, connection_ref)
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance(
        &self,
        server_name: &str,
    ) -> Result<(String, &UpstreamConnection)> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No instances found for server '{}'",
                server_name
            ));
        }

        // Get the first instance (for now, we'll just use the first one as default)
        let (instance_id, connection) = instances.iter().next().unwrap();
        Ok((instance_id.clone(), connection))
    }

    /// Helper method to get a mutable reference to the default instance of a server
    ///
    /// Returns the first available instance for a server. In the future,
    /// this could be enhanced with more sophisticated selection logic.
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok((String, &mut UpstreamConnection))` - Tuple of (instance_id, mutable_connection_ref)
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance_mut(
        &mut self,
        server_name: &str,
    ) -> Result<(String, &mut UpstreamConnection)> {
        let instances = self.connections.get_mut(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No instances found for server '{}'",
                server_name
            ));
        }

        // Get the first instance (for now, we'll just use the first one as default)
        let instance_id = instances.keys().next().unwrap().clone();
        let connection = instances.get_mut(&instance_id).unwrap();
        Ok((instance_id, connection))
    }

    /// Helper method to get just the default instance ID of a server
    ///
    /// This is useful when you only need the instance ID for other operations.
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok(String)` - The default instance ID
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance_id(
        &self,
        server_name: &str,
    ) -> Result<String> {
        let (instance_id, _) = self.get_default_instance(server_name)?;
        Ok(instance_id)
    }

    /// Get all instance IDs for a server
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of all instance IDs for the server
    /// * `Err(...)` - If server not found
    pub fn get_all_instance_ids(
        &self,
        server_name: &str,
    ) -> Result<Vec<String>> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        Ok(instances.keys().cloned().collect())
    }

    // ========================================
    // Connection Status and State Methods
    // ========================================

    /// Check if a server has any connected instances
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok(bool)` - True if at least one instance is connected
    /// * `Err(...)` - If server not found
    pub fn has_connected_instances(
        &self,
        server_name: &str,
    ) -> Result<bool> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        Ok(instances.values().any(|conn| conn.is_connected()))
    }

    /// Get count of connected instances for a server
    ///
    /// # Arguments
    /// * `server_name` - Name of the server
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of connected instances
    /// * `Err(...)` - If server not found
    pub fn count_connected_instances(
        &self,
        server_name: &str,
    ) -> Result<usize> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        Ok(instances
            .values()
            .filter(|conn| conn.is_connected())
            .count())
    }

    /// Get total number of instances across all servers
    ///
    /// # Returns
    /// * `usize` - Total number of instances in the pool
    pub fn total_instance_count(&self) -> usize {
        self.connections
            .values()
            .map(|instances| instances.len())
            .sum()
    }

    /// Get total number of connected instances across all servers
    ///
    /// # Returns
    /// * `usize` - Total number of connected instances in the pool
    pub fn total_connected_count(&self) -> usize {
        self.connections
            .values()
            .flat_map(|instances| instances.values())
            .filter(|conn| conn.is_connected())
            .count()
    }

    // ========================================
    // Status Query Methods
    // ========================================

    /// Get status of the default instance of a server
    pub fn get_server_status(
        &self,
        server_name: &str,
    ) -> Result<String> {
        // Check if the server exists
        if !self.connections.contains_key(server_name) {
            return Err(anyhow::anyhow!(
                "Server '{}' not found in connection pool",
                server_name
            ));
        }

        // Get the default instance
        let (instance_id, conn) = self.get_default_instance(server_name)?;

        Ok(format!(
            "{} (instance {})",
            conn.status_string(),
            instance_id
        ))
    }

    /// Get status of a specific instance of a server
    pub fn get_instance_status(
        &self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<String> {
        let conn = self.get_instance(server_name, instance_id)?;

        Ok(conn.status_string())
    }

    /// Get all server instances with cloned connections
    pub fn get_all_server_instances(&self) -> HashMap<String, Vec<(String, UpstreamConnection)>> {
        let mut result = HashMap::new();

        for (server_name, instances) in &self.connections {
            let instance_clones: Vec<(String, UpstreamConnection)> = instances
                .iter()
                .map(|(id, conn)| (id.clone(), conn.clone()))
                .collect();

            result.insert(server_name.clone(), instance_clones);
        }

        result
    }

    /// Get server type
    pub fn get_server_type(
        &self,
        server_name: &str,
    ) -> Option<String> {
        self.config
            .mcp_servers
            .get(server_name)
            .map(|cfg| cfg.kind.to_string())
    }

    // ========================================
    // Connection State Hashing
    // ========================================

    /// Calculate a hash value representing the current state of the connection pool
    /// This can be used to detect changes in the connection pool
    ///
    /// This optimized version:
    /// 1. Avoids unnecessary string allocations
    /// 2. Only hashes essential information needed to detect relevant changes
    /// 3. Uses a more efficient hashing approach for connection status
    pub fn calculate_connection_state_hash(&self) -> u64 {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let mut hasher = DefaultHasher::new();

        // Hash the number of servers (important for detecting server additions/removals)
        self.connections.len().hash(&mut hasher);

        // For each server, hash its name and the state of each instance
        for (server_name, instances) in &self.connections {
            // Hash server name
            server_name.hash(&mut hasher);

            // Hash number of instances (important for detecting instance additions/removals)
            instances.len().hash(&mut hasher);

            // For each instance, hash its state
            for (instance_id, conn) in instances {
                // Hash instance ID
                instance_id.hash(&mut hasher);

                // Hash connection status - use discriminant instead of debug format
                // This is more efficient and still captures the essential state
                let status_discriminant = match &conn.status {
                    ConnectionStatus::Initializing => 1,
                    ConnectionStatus::Ready => 2,
                    ConnectionStatus::Busy => 3,
                    ConnectionStatus::Error(_) => 4,
                    ConnectionStatus::Shutdown => 5,
                    ConnectionStatus::Disabled(_) => 6,
                    ConnectionStatus::Validating => 7,
                };
                status_discriminant.hash(&mut hasher);

                // For error status, also hash the error type (but not the full details)
                if let ConnectionStatus::Error(details) = &conn.status {
                    let error_type_discriminant = match details.error_type {
                        ErrorType::Temporary => 1,
                        ErrorType::Permanent => 2,
                        ErrorType::Unknown => 3,
                    };
                    error_type_discriminant.hash(&mut hasher);

                    // Hash failure count as it affects reconnection behavior
                    details.failure_count.hash(&mut hasher);
                }

                // Hash the number of tools (important for detecting tool changes)
                conn.tools.len().hash(&mut hasher);

                // Hash tool names (important for detecting tool changes)
                // Avoid unnecessary string allocations by using references
                for tool in &conn.tools {
                    tool.name.hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::models::Config;
    use crate::core::pool::UpstreamConnectionPool;
    use std::{collections::HashMap, sync::Arc};

    fn create_test_pool() -> UpstreamConnectionPool {
        let config = Arc::new(Config {
            mcp_servers: HashMap::new(),
            pagination: None,
        });
        UpstreamConnectionPool::new(config, None)
    }

    #[tokio::test]
    async fn test_instance_helpers_with_empty_pool() {
        let pool = create_test_pool();

        // Should return errors for non-existent servers
        assert!(pool.get_instance("nonexistent", "instance1").is_err());
        assert!(pool.get_default_instance("nonexistent").is_err());
        assert!(pool.get_all_instance_ids("nonexistent").is_err());
        assert!(pool.has_connected_instances("nonexistent").is_err());

        // Should return 0 for empty pool
        assert_eq!(pool.total_instance_count(), 0);
        assert_eq!(pool.total_connected_count(), 0);
    }
}
