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
    eprintln!("Starting Gmail MCP Server (SMTP Version)...");
    
    // Check for credentials availability
    if std::env::var("GMAIL_USER").is_err() || std::env::var("GMAIL_APP_PASSWORD").is_err() {
        eprintln!("Warning: GMAIL_USER or GMAIL_APP_PASSWORD not set. Email sending will fail.");
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

        // Get credentials from env
        let username = std::env::var("GMAIL_USER").map_err(|_| JsonRpcError {
            code: -32000,
            message: "GMAIL_USER env var not set".to_string(),
            data: None,
        })?;
        let password = std::env::var("GMAIL_APP_PASSWORD").map_err(|_| JsonRpcError {
            code: -32000,
            message: "GMAIL_APP_PASSWORD env var not set".to_string(),
            data: None,
        })?;

        let client = GmailClient::new(username, password);

        let result = client.send_email(to, subject, body, attachment_path).await.map_err(|e| JsonRpcError {
            code: -32000,
            message: format!("Failed to send email: {}", e),
            data: None,
        })?;

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Email sent successfully. Result: {}", result)
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
