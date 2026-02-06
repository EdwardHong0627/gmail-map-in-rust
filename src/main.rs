mod gmail_client;

use anyhow::Result;
use gmail_client::GmailClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Represents a JSON-RPC 2.0 Request.
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

/// Represents a JSON-RPC 2.0 Response.
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// Represents a JSON-RPC 2.0 Error.
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Gmail client (lazy init or init on cleanup? For now, let's init on start or first call)
    // To allow capabilities listing without auth, we might want to delay auth.
    // However, simplest is to try to init. If it fails, we might just panic or log.
    // Given it's a CLI tool, logging to stderr is fine.
    
    eprintln!("Starting Gmail MCP Server...");
    
    // Check for credentials availability:
    // 1. GOOGLE_CLIENT_SECRET environment variable (raw JSON content)
    // 2. client_secret.json file (in current directory)
    if std::env::var("GOOGLE_CLIENT_SECRET").is_err() && !std::path::Path::new("client_secret.json").exists() {
        eprintln!("Warning: client_secret.json not found and GOOGLE_CLIENT_SECRET not set. Authentication interactions will fail.");
    }

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    // Read lines from stdin (JSON-RPC messages are line-delimited in this implementation)
    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse JSON: {}", e);
                continue;
            }
        };

        handle_request(req).await;
    }

    Ok(())
}

/// Handles a single JSON-RPC request and writes the response to stdout.
async fn handle_request(req: JsonRpcRequest) {
    let id = req.id.clone();
    let response = match req.method.as_str() {
        "initialize" => {
            // MCP Handshake: Return server capabilities
            Ok(json!({
                "protocolVersion": "0.1.0",
                "serverInfo": {
                    "name": "gmail-mcp-server",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }))
        }
        "notifications/initialized" => {
            // Client confirming initialization
            Ok(json!("OK"))
        }
        "tools/list" => {
            // List available tools
            Ok(json!({
                "tools": [
                    {
                        "name": "send_email",
                        "description": "Send an email with an optional attachment via Gmail",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "to": { "type": "string", "description": "Recipient email address" },
                                "subject": { "type": "string", "description": "Email subject" },
                                "body": { "type": "string", "description": "Email body content" },
                                "attachment_path": { "type": "string", "description": "Absolute path to an attachment file (optional)" }
                            },
                            "required": ["to", "subject", "body"]
                        }
                    }
                ]
            }))
        }
        "tools/call" => {
            // Execute a tool
            handle_tool_call(req.params).await
        }
        _ => {
            // Method not found
             Err(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            })
        }
    };

    // If request had an ID, send a response. If it was a notification (no ID), do nothing.
    if let Some(id_val) = id {
        let resp = match response {
            Ok(res) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(id_val),
                result: Some(res),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(id_val),
                result: None,
                error: Some(e),
            },
        };
        
        let out = serde_json::to_string(&resp).unwrap();
        println!("{}", out);
    }
}

/// Dispatches tool calls to specific implementations.
async fn handle_tool_call(params: Option<Value>) -> Result<Value, JsonRpcError> {
    let params = params.ok_or(JsonRpcError {
        code: -32602,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let name = params.get("name").and_then(|n| n.as_str()).ok_or(JsonRpcError{
        code: -32602,
        message: "Missing tool name".to_string(),
        data: None,
    })?;

    if name == "send_email" {
        let args = params.get("arguments").ok_or(JsonRpcError{
            code: -32602,
            message: "Missing arguments".to_string(),
            data: None,
        })?;

        let to = args.get("to").and_then(|s| s.as_str()).ok_or(JsonRpcError{
             code: -32602, message: "Missing 'to'".to_string(), data: None
        })?;
        let subject = args.get("subject").and_then(|s| s.as_str()).unwrap_or("(No Subject)");
        let body = args.get("body").and_then(|s| s.as_str()).unwrap_or("");
        let attachment_path = args.get("attachment_path").and_then(|s| s.as_str());

        // Initialize Gmail client for every call (simple approach).
        // It uses cached tokens ("token_cache.json") so subsequent calls don't require re-auth.
        let client = GmailClient::new("client_secret.json").await.map_err(|e| JsonRpcError {
            code: -32000,
            message: format!("Failed to init Gmail client: {}", e),
            data: None,
        })?;

        let msg_id = client.send_email(to, subject, body, attachment_path).await.map_err(|e| JsonRpcError {
            code: -32000,
            message: format!("Failed to send email: {}", e),
            data: None,
        })?;

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Email sent successfully. Message ID: {}", msg_id)
                }
            ]
        }))
    } else {
        Err(JsonRpcError {
            code: -32601,
            message: format!("Unknown tool: {}", name),
            data: None,
        })
    }
}
