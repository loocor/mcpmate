use rmcp::ClientHandler;
use rmcp::service::{Peer, RoleClient};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InspectorMcpHandshakeMessage {
    pub direction: String,
    pub method: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InspectorSessionHandshakeData {
    pub protocol_version: String,
    pub server_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_title: Option<String>,
    pub messages: Vec<InspectorMcpHandshakeMessage>,
}

pub fn build_session_handshake(peer: &Peer<RoleClient>) -> InspectorSessionHandshakeData {
    let handler = crate::core::transport::client::UpstreamClientHandler::new("inspector".to_string());
    let client_info = handler.get_info();
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": client_info.protocol_version.to_string(),
            "capabilities": serde_json::to_value(&client_info.capabilities).unwrap_or(Value::Null),
            "clientInfo": serde_json::to_value(&client_info.client_info).unwrap_or(Value::Null),
        }
    });

    let (initialize_response, protocol_version, server_name, server_title) = if let Some(info) = peer.peer_info() {
        let result = json!({
            "protocolVersion": info.protocol_version.to_string(),
            "capabilities": serde_json::to_value(&info.capabilities).unwrap_or(Value::Null),
            "serverInfo": serde_json::to_value(&info.server_info).unwrap_or(Value::Null),
        });
        (
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": result,
            }),
            info.protocol_version.to_string(),
            info.server_info.name.clone(),
            info.server_info.title.clone(),
        )
    } else {
        (Value::Null, String::new(), String::new(), None)
    };

    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    InspectorSessionHandshakeData {
        protocol_version,
        server_name,
        server_title,
        messages: vec![
            handshake_message("outbound", "initialize", initialize_request),
            handshake_message("inbound", "initialize", initialize_response),
            handshake_message("outbound", "notifications/initialized", initialized_notification),
        ],
    }
}

fn handshake_message(
    direction: &str,
    method: &str,
    payload: Value,
) -> InspectorMcpHandshakeMessage {
    InspectorMcpHandshakeMessage {
        direction: direction.to_string(),
        method: method.to_string(),
        payload,
    }
}
