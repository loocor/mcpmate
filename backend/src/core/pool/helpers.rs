//! Pool helper methods for UpstreamConnectionPool
//!
//! Contains helper methods for managing connection instances and utilities within the pool.
//! These methods provide convenient access patterns for getting and manipulating
//! server instances, status queries, and connection state management.

use crate::core::capability::{AffinityKey, ConnectionSelection};
use crate::core::foundation::types::{
    ConnectionStatus, // status of the connection
    ErrorType,        // type of the error
};
use crate::core::pool::{ProductionRouteKey, UpstreamConnection};
use anyhow::{Context, Result};
use std::{collections::HashMap, time::Instant};

/// Helper methods for managing connection instances and utilities
impl super::UpstreamConnectionPool {
    // ========================================
    // Instance Access Methods
    // ========================================

    /// Helper method to get a specific instance of a server
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    /// * `instance_id` - ID of the specific instance
    ///
    /// # Returns
    /// * `Ok(&UpstreamConnection)` - Reference to the connection instance
    /// * `Err(...)` - If server or instance not found
    pub fn get_instance(
        &self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<&UpstreamConnection> {
        // First check shared connections
        if let Some(instances) = self.connections.get(server_id) {
            if let Some(conn) = instances.get(instance_id) {
                return Ok(conn);
            }
        }

        // Then check client-bound connections
        for ((sid, _), instances) in &self.client_bound_connections {
            if sid == server_id {
                if let Some(conn) = instances.get(instance_id) {
                    return Ok(conn);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Instance '{}' not found for server '{}'",
            instance_id,
            server_id
        ))
    }

    /// Helper method to get a mutable reference to a specific instance of a server
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    /// * `instance_id` - ID of the specific instance
    ///
    /// # Returns
    /// * `Ok(&mut UpstreamConnection)` - Mutable reference to the connection instance
    /// * `Err(...)` - If server or instance not found
    pub fn get_instance_mut(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<&mut UpstreamConnection> {
        // First check shared connections
        if let Some(instances) = self.connections.get_mut(server_id) {
            if let Some(conn) = instances.get_mut(instance_id) {
                return Ok(conn);
            }
        }

        // Then check client-bound connections
        for ((sid, _), instances) in &mut self.client_bound_connections {
            if sid == server_id {
                if let Some(conn) = instances.get_mut(instance_id) {
                    return Ok(conn);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Instance '{}' not found for server '{}'",
            instance_id,
            server_id
        ))
    }

    /// Helper method to get the default instance of a server
    ///
    /// Returns the first available instance for a server. In the future,
    /// this could be enhanced with more sophisticated selection logic.
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok((String, &UpstreamConnection))` - Tuple of (instance_id, connection_ref)
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance(
        &self,
        server_id: &str,
    ) -> Result<(String, &UpstreamConnection)> {
        let instances = self
            .connections
            .get(server_id)
            .context(format!("Server '{server_id}' not found in connection pool"))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!("No instances found for server '{}'", server_id));
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
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok((String, &mut UpstreamConnection))` - Tuple of (instance_id, mutable_connection_ref)
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance_mut(
        &mut self,
        server_id: &str,
    ) -> Result<(String, &mut UpstreamConnection)> {
        let instances = self
            .connections
            .get_mut(server_id)
            .context(format!("Server '{server_id}' not found in connection pool"))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!("No instances found for server '{}'", server_id));
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
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok(String)` - The default instance ID
    /// * `Err(...)` - If server not found or has no instances
    pub fn get_default_instance_id(
        &self,
        server_id: &str,
    ) -> Result<String> {
        let (instance_id, _) = self.get_default_instance(server_id)?;
        Ok(instance_id)
    }

    /// Select instance ID for a production route using affinity-aware routing.
    ///
    /// This method implements the production routing logic:
    /// - For `AffinityKey::Default`: select from shared connections
    /// - For `AffinityKey::PerClient(client_id)`: select from client-bound connections
    /// - For `AffinityKey::PerSession(session_id)`: select from client-bound connections (session-treated)
    ///
    /// Falls back to default instance only if no affinity-bound instance exists.
    pub fn select_instance_id(
        &self,
        selection: &ConnectionSelection,
    ) -> Result<String> {
        match &selection.affinity_key {
            AffinityKey::Default => self.select_default_instance_id(&selection.server_id),
            AffinityKey::PerClient(client_id) => self
                .select_affinity_bound_instance_id(&selection.server_id, client_id)
                .or_else(|_| self.select_default_instance_id(&selection.server_id)),
            AffinityKey::PerSession(session_id) => self
                .select_affinity_bound_instance_id(&selection.server_id, session_id)
                .or_else(|_| self.select_default_instance_id(&selection.server_id)),
        }
    }

    /// Select a ready instance ID for a production route using affinity-aware routing.
    ///
    /// Returns only connected and ready instances, respecting affinity constraints.
    pub fn select_ready_instance_id(
        &self,
        selection: &ConnectionSelection,
    ) -> Result<Option<String>> {
        match &selection.affinity_key {
            AffinityKey::Default => self.select_ready_default_instance_id(&selection.server_id),
            AffinityKey::PerClient(client_id) => {
                let affinity_result = self.select_ready_affinity_bound_instance_id(&selection.server_id, client_id)?;
                if affinity_result.is_some() {
                    return Ok(affinity_result);
                }
                self.select_ready_default_instance_id(&selection.server_id)
            }
            AffinityKey::PerSession(session_id) => {
                let affinity_result = self.select_ready_affinity_bound_instance_id(&selection.server_id, session_id)?;
                if affinity_result.is_some() {
                    return Ok(affinity_result);
                }
                self.select_ready_default_instance_id(&selection.server_id)
            }
        }
    }

    /// Resolve instance ID for production routing, using existing route or allocating new.
    ///
    /// This is the primary entry point for connection establishment:
    /// 1. Check if a route already exists for the given (server_id, affinity_key)
    /// 2. If exists, return the mapped instance_id
    /// 3. If not, return None (caller should allocate via allocate_instance_id)
    pub fn resolve_production_route(
        &self,
        selection: &ConnectionSelection,
    ) -> Option<String> {
        let route_key = ProductionRouteKey::new(selection.server_id.clone(), selection.affinity_key.clone());
        self.production_routes.get(&route_key).cloned()
    }

    /// Allocate a new instance ID for a production route.
    ///
    /// This method creates a new instance ID and registers it in the production routes table.
    /// The caller is responsible for creating the actual connection.
    pub fn allocate_production_route(
        &mut self,
        selection: &ConnectionSelection,
    ) -> String {
        let instance_id = super::UpstreamConnection::new(selection.server_id.clone()).id;
        let route_key = ProductionRouteKey::new(selection.server_id.clone(), selection.affinity_key.clone());

        match &selection.affinity_key {
            AffinityKey::Default => {
                let instances = self.connections.entry(selection.server_id.clone()).or_default();
                instances.insert(
                    instance_id.clone(),
                    super::UpstreamConnection::new(selection.server_id.clone()),
                );
            }
            AffinityKey::PerClient(bound_id) | AffinityKey::PerSession(bound_id) => {
                let bound_instances = self
                    .client_bound_connections
                    .entry((selection.server_id.clone(), bound_id.clone()))
                    .or_default();
                bound_instances.insert(
                    instance_id.clone(),
                    super::UpstreamConnection::new(selection.server_id.clone()),
                );
            }
        }

        self.production_routes.insert(route_key, instance_id.clone());
        instance_id
    }

    /// Select instance ID from shared (default) connections.
    fn select_default_instance_id(
        &self,
        server_id: &str,
    ) -> Result<String> {
        self.get_default_instance_id(server_id)
    }

    /// Select instance ID from affinity-bound connections.
    fn select_affinity_bound_instance_id(
        &self,
        server_id: &str,
        bound_id: &str,
    ) -> Result<String> {
        let bound_instances = self
            .client_bound_connections
            .get(&(server_id.to_string(), bound_id.to_string()))
            .context(format!(
                "No affinity-bound instances for server '{}' bound to '{}'",
                server_id, bound_id
            ))?;

        if bound_instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No affinity-bound instances found for server '{}' bound to '{}'",
                server_id,
                bound_id
            ));
        }

        for (instance_id, conn) in bound_instances.iter() {
            if conn.can_connect() {
                return Ok(instance_id.clone());
            }
        }

        let (instance_id, _) = bound_instances.iter().next().unwrap();
        Ok(instance_id.clone())
    }

    /// Select a ready instance from shared (default) connections.
    fn select_ready_default_instance_id(
        &self,
        server_id: &str,
    ) -> Result<Option<String>> {
        let instances = self.connections.get(server_id);
        if let Some(instances) = instances {
            for (instance_id, conn) in instances {
                if conn.is_connected() && conn.service.is_some() {
                    return Ok(Some(instance_id.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Select a ready instance from affinity-bound connections.
    fn select_ready_affinity_bound_instance_id(
        &self,
        server_id: &str,
        bound_id: &str,
    ) -> Result<Option<String>> {
        if let Some(bound_instances) = self
            .client_bound_connections
            .get(&(server_id.to_string(), bound_id.to_string()))
        {
            for (instance_id, conn) in bound_instances {
                if conn.is_connected() && conn.service.is_some() {
                    return Ok(Some(instance_id.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Check if an affinity-bound connection exists for the given server and bound ID.
    pub fn has_affinity_bound_connection(
        &self,
        server_id: &str,
        bound_id: &str,
    ) -> bool {
        self.client_bound_connections
            .get(&(server_id.to_string(), bound_id.to_string()))
            .map(|instances| !instances.is_empty())
            .unwrap_or(false)
    }

    /// Get affinity-bound connection count for a server and bound ID.
    pub fn affinity_bound_connection_count(
        &self,
        server_id: &str,
        bound_id: &str,
    ) -> usize {
        self.client_bound_connections
            .get(&(server_id.to_string(), bound_id.to_string()))
            .map(|instances| instances.len())
            .unwrap_or(0)
    }

    /// Get total production route count.
    pub fn production_route_count(&self) -> usize {
        self.production_routes.len()
    }

    /// Get total client-bound connection count.
    pub fn client_bound_connection_count(&self) -> usize {
        self.client_bound_connections.values().map(|m| m.len()).sum()
    }

    /// Get all instance IDs for a server
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of all instance IDs for the server
    /// * `Err(...)` - If server not found
    pub fn get_all_instance_ids(
        &self,
        server_id: &str,
    ) -> Result<Vec<String>> {
        let instances = self
            .connections
            .get(server_id)
            .context(format!("Server '{server_id}' not found in connection pool"))?;

        Ok(instances.keys().cloned().collect())
    }

    /// Mark the default connection's last activity timestamp.
    ///
    /// Used by runtime flows to keep long-lived instances alive when they
    /// successfully serve API or MCP requests.
    pub fn mark_instance_activity(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) {
        if let Ok(instance) = self.get_instance_mut(server_id, instance_id) {
            instance.last_activity = Instant::now();
        }
    }

    // ========================================
    // Connection Status and State Methods
    // ========================================

    /// Check if a server has any connected instances
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok(bool)` - True if at least one instance is connected
    /// * `Err(...)` - If server not found
    pub fn has_connected_instances(
        &self,
        server_id: &str,
    ) -> Result<bool> {
        let instances = self
            .connections
            .get(server_id)
            .context(format!("Server '{server_id}' not found in connection pool"))?;

        Ok(instances.values().any(|conn| conn.is_connected()))
    }

    /// Get count of connected instances for a server
    ///
    /// # Arguments
    /// * `server_id` - ID of the server
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of connected instances
    /// * `Err(...)` - If server not found
    pub fn count_connected_instances(
        &self,
        server_id: &str,
    ) -> Result<usize> {
        let instances = self
            .connections
            .get(server_id)
            .context(format!("Server '{server_id}' not found in connection pool"))?;

        Ok(instances.values().filter(|conn| conn.is_connected()).count())
    }

    /// Get total number of instances across all servers
    ///
    /// # Returns
    /// * `usize` - Total number of instances in the pool
    pub fn total_instance_count(&self) -> usize {
        self.connections.values().map(|instances| instances.len()).sum()
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
        server_id: &str,
    ) -> Result<String> {
        // Check if the server exists
        if !self.connections.contains_key(server_id) {
            return Err(anyhow::anyhow!("Server '{}' not found in connection pool", server_id));
        }

        // Get the default instance
        let (instance_id, conn) = self.get_default_instance(server_id)?;

        Ok(format!("{} (instance {})", conn.status_string(), instance_id))
    }

    /// Get status of a specific instance of a server
    pub fn get_instance_status(
        &self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<String> {
        let conn = self.get_instance(server_id, instance_id)?;

        Ok(conn.status_string())
    }

    /// Get all server instances with cloned connections
    pub fn get_all_server_instances(&self) -> HashMap<String, Vec<(String, UpstreamConnection)>> {
        let mut result = HashMap::new();

        for (server_id, instances) in &self.connections {
            let instance_clones: Vec<(String, UpstreamConnection)> =
                instances.iter().map(|(id, conn)| (id.clone(), conn.clone())).collect();

            result.insert(server_id.clone(), instance_clones);
        }

        result
    }

    /// Get server type
    pub fn get_server_type(
        &self,
        server_id: &str,
    ) -> Option<String> {
        self.config.mcp_servers.get(server_id).map(|cfg| cfg.kind.to_string())
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
                    ConnectionStatus::Idle => 0,
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

    /// Remove all production routes pointing to a specific instance.
    pub fn remove_production_routes_for_instance(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) {
        self.production_routes
            .retain(|key, &mut ref id| key.server_id != server_id || id != instance_id);
    }

    /// Remove all production routes for a server (regardless of instance).
    pub fn remove_all_production_routes_for_server(
        &mut self,
        server_id: &str,
    ) {
        self.production_routes.retain(|key, _| key.server_id != server_id);
    }

    /// Remove a specific client-bound instance.
    pub fn remove_client_bound_instance(
        &mut self,
        server_id: &str,
        bound_id: &str,
        instance_id: &str,
    ) {
        if let Some(instances) = self
            .client_bound_connections
            .get_mut(&(server_id.to_string(), bound_id.to_string()))
        {
            instances.remove(instance_id);
            if instances.is_empty() {
                self.client_bound_connections
                    .remove(&(server_id.to_string(), bound_id.to_string()));
            }
        }
    }

    /// Remove all client-bound connections for a server (all bound IDs).
    pub fn remove_all_client_bound_connections_for_server(
        &mut self,
        server_id: &str,
    ) {
        self.client_bound_connections.retain(|(sid, _), _| sid != server_id);
    }

    /// Get all bound IDs (client_id or session_id) for a server.
    pub fn get_bound_ids_for_server(
        &self,
        server_id: &str,
    ) -> Vec<String> {
        self.client_bound_connections
            .keys()
            .filter(|(sid, _)| sid == server_id)
            .map(|(_, bound_id)| bound_id.clone())
            .collect()
    }

    /// Clean up all affinity-related entries for a disconnected instance.
    pub fn cleanup_disconnected_instance(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) {
        self.remove_production_routes_for_instance(server_id, instance_id);
        let bound_ids: Vec<String> = self.get_bound_ids_for_server(server_id);
        for bound_id in bound_ids {
            self.remove_client_bound_instance(server_id, &bound_id, instance_id);
        }
    }
}

/// Instance ID helpers (single source of truth)
pub fn format_validation_instance_id(
    server_name: &str,
    session_id: &str,
) -> String {
    format!("validation-{}-{}", server_name, session_id)
}

#[cfg(test)]
mod tests {
    use crate::core::models::Config;
    use crate::core::pool::{ProductionRouteKey, UpstreamConnection, UpstreamConnectionPool};
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

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_removes_production_routes() {
        let mut pool = create_test_pool();
        let server_id = "test-server";
        let instance_id = "instance-1";

        // Set up a production route
        let route_key = ProductionRouteKey::shareable(server_id);
        pool.production_routes
            .insert(route_key.clone(), instance_id.to_string());

        // Verify route exists
        assert_eq!(pool.production_routes.get(&route_key), Some(&instance_id.to_string()));

        // Clean up
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Verify route is removed
        assert!(!pool.production_routes.contains_key(&route_key));
    }

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_removes_client_bound_instances() {
        let mut pool = create_test_pool();
        let server_id = "test-server";
        let client_id = "client-1";
        let instance_id = "instance-1";

        // Set up a client-bound connection
        let bound_key = (server_id.to_string(), client_id.to_string());
        let mut instances = HashMap::new();
        instances.insert(instance_id.to_string(), UpstreamConnection::new(server_id.to_string()));
        pool.client_bound_connections.insert(bound_key.clone(), instances);

        // Set up corresponding production route
        let route_key = ProductionRouteKey::per_client(server_id, client_id);
        pool.production_routes.insert(route_key, instance_id.to_string());

        // Verify client-bound instance exists
        assert!(pool.client_bound_connections.contains_key(&bound_key));
        assert_eq!(pool.client_bound_connections.get(&bound_key).unwrap().len(), 1);

        // Clean up
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Verify client-bound instance is removed (bound key should be removed when empty)
        assert!(!pool.client_bound_connections.contains_key(&bound_key));
    }

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_removes_session_bound_instances() {
        let mut pool = create_test_pool();
        let server_id = "test-server";
        let session_id = "session-1";
        let instance_id = "instance-1";

        // Set up a session-bound connection
        let bound_key = (server_id.to_string(), session_id.to_string());
        let mut instances = HashMap::new();
        instances.insert(instance_id.to_string(), UpstreamConnection::new(server_id.to_string()));
        pool.client_bound_connections.insert(bound_key.clone(), instances);

        // Set up corresponding production route
        let route_key = ProductionRouteKey::per_session(server_id, session_id);
        pool.production_routes.insert(route_key, instance_id.to_string());

        // Verify session-bound instance exists
        assert!(pool.client_bound_connections.contains_key(&bound_key));

        // Clean up
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Verify session-bound instance is removed
        assert!(!pool.client_bound_connections.contains_key(&bound_key));
    }

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_does_not_affect_other_servers() {
        let mut pool = create_test_pool();
        let server_a = "server-a";
        let server_b = "server-b";
        let instance_a = "instance-a";
        let instance_b = "instance-b";

        // Set up routes for both servers
        let route_a = ProductionRouteKey::shareable(server_a);
        let route_b = ProductionRouteKey::shareable(server_b);
        pool.production_routes.insert(route_a.clone(), instance_a.to_string());
        pool.production_routes.insert(route_b.clone(), instance_b.to_string());

        // Clean up server A only
        pool.cleanup_disconnected_instance(server_a, instance_a);

        // Server A route should be removed
        assert!(!pool.production_routes.contains_key(&route_a));
        // Server B route should remain
        assert_eq!(pool.production_routes.get(&route_b), Some(&instance_b.to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_handles_multiple_bound_ids() {
        let mut pool = create_test_pool();
        let server_id = "test-server";
        let client_1 = "client-1";
        let client_2 = "client-2";
        let instance_id = "instance-1";

        // Set up client-bound connections for two clients with the SAME instance
        // (edge case: same instance bound to multiple clients)
        for client_id in [&client_1, &client_2] {
            let bound_key = (server_id.to_string(), client_id.to_string());
            let mut instances = HashMap::new();
            instances.insert(instance_id.to_string(), UpstreamConnection::new(server_id.to_string()));
            pool.client_bound_connections.insert(bound_key, instances);
        }

        // Verify both client-bound entries exist
        assert_eq!(pool.affinity_bound_connection_count(server_id, client_1), 1);
        assert_eq!(pool.affinity_bound_connection_count(server_id, client_2), 1);

        // Clean up the instance
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Both client-bound entries should be removed
        assert_eq!(pool.affinity_bound_connection_count(server_id, client_1), 0);
        assert_eq!(pool.affinity_bound_connection_count(server_id, client_2), 0);
    }

    #[tokio::test]
    async fn test_cleanup_disconnected_instance_idempotent() {
        let mut pool = create_test_pool();
        let server_id = "test-server";
        let instance_id = "instance-1";

        // Call cleanup on non-existent instance (should not panic)
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Call cleanup again (idempotent)
        pool.cleanup_disconnected_instance(server_id, instance_id);

        // Verify pool is still in valid state
        assert_eq!(pool.production_route_count(), 0);
        assert_eq!(pool.client_bound_connection_count(), 0);
    }
}
