// Utility functions for UpstreamConnectionPool

use std::collections::HashMap;

use anyhow::Result;

use super::UpstreamConnectionPool;

impl UpstreamConnectionPool {
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
    pub fn get_all_server_instances(
        &self
    ) -> HashMap<String, Vec<(String, crate::core::connection::UpstreamConnection)>> {
        let mut result = HashMap::new();

        for (server_name, instances) in &self.connections {
            let instance_clones: Vec<(String, crate::core::connection::UpstreamConnection)> =
                instances
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
            .map(|cfg| cfg.kind.clone())
    }

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
                    crate::core::types::ConnectionStatus::Initializing => 1,
                    crate::core::types::ConnectionStatus::Ready => 2,
                    crate::core::types::ConnectionStatus::Busy => 3,
                    crate::core::types::ConnectionStatus::Error(_) => 4,
                    crate::core::types::ConnectionStatus::Shutdown => 5,
                };
                status_discriminant.hash(&mut hasher);

                // For error status, also hash the error type (but not the full details)
                if let crate::core::types::ConnectionStatus::Error(details) = &conn.status {
                    let error_type_discriminant = match details.error_type {
                        crate::core::types::ErrorType::Temporary => 1,
                        crate::core::types::ErrorType::Permanent => 2,
                        crate::core::types::ErrorType::Unknown => 3,
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
