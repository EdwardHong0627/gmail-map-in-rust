# Gmail MCP Server

A local Model Context Protocol (MCP) server that enables sending emails with attachments via the Gmail API.

## Features

- **Send Email**: Send emails with optional attachments using the `send_email` tool.
- **Secure Authentication**: Uses Google OAuth2 for authentication. Secrets can be managed via file or environment variable.
- **Protocol**: Implements the MCP JSON-RPC 2.0 protocol over stdio.

## Prerequisites

- **Rust**: Ensure you have Rust installed (`cargo`).
- **Google Cloud Credentials**:
    1.  Go to the [Google Cloud Console](https://console.cloud.google.com/).
    2.  Create a project and enable the **Gmail API**.
    3.  Create OAuth Credentials (type **Desktop App**).
    4.  Download the JSON file.

## Configuration

You can provide the Google Client Secret in one of two ways:

1.  **File**: Rename the downloaded JSON file to `client_secret.json` and place it in the root directory of the server (next to `Cargo.toml`).
2.  **Environment Variable**: Set `GOOGLE_CLIENT_SECRET` to the content of the JSON file.

    ```bash
    export GOOGLE_CLIENT_SECRET='{"installed":{"client_id":"...","client_secret":"..."}}'
    ```

## Installation

```bash
cd gmail-mcp-server
cargo build --release
```

## Usage

Run the server via Cargo or the compiled binary:

```bash
cargo run
```

### MC Protocol

The server communicates via JSON-RPC over Standard Input/Output.

#### Request Example (Send Email)

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "send_email",
    "arguments": {
      "to": "recipient@example.com",
      "subject": "Hello from MCP",
      "body": "This is a test email sent via a local Rust MCP server.",
      "attachment_path": "/path/to/file.txt"
    }
  },
  "id": 1
}
```

## Integration with MCP Clients

To use this server with an MCP client (like Claude Desktop), you need to configure it to run the executable.

### Example: Claude Desktop Configuration

Add the following to your `claude_desktop_config.json` (usually found in `~/Library/Application Support/Claude/` on macOS):

```json
{
  "mcpServers": {
    "gmail": {
      "command": "/path/to/your/gmail-mcp-server/target/release/gmail-mcp-server",
      "args": [],
      "env": {
        "GOOGLE_CLIENT_SECRET": "{\"installed\":{...}}"
      }
    }
  }
}
```

*Note: Ensure you build with `cargo build --release` first to get the optimized binary.*

## Distribution

### Option 1: Source (Recommended for Developers)
Share this repository. The user runs:
```bash
cargo run
```

### Option 2: Binary
1.  Build the release binary:
    ```bash
    cargo build --release
    ```
2.  The binary is located at `target/release/gmail-mcp-server`.
3.  Share this single file. Users can run it directly, provided they set the `GOOGLE_CLIENT_SECRET` environment variable.

## Troubleshooting

- **Authentication Fails**: Ensure `client_secret.json` or `GOOGLE_CLIENT_SECRET` is correct. The first time you run, it will open a browser to authenticate.
- **Build Errors**: Check your internet connection (access to `crates.io` is required).
