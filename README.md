# Gmail MCP Server

A local Model Context Protocol (MCP) server that enables sending emails with attachments via the Gmail API.

## Features

- **Send Email**: Send emails with optional attachments using the `send_email` tool.
- **Secure Authentication**: Uses Google OAuth2 for authentication. Secrets can be managed via file or environment variable.
- **Protocol**: Implements the MCP JSON-RPC 2.0 protocol over stdio.

## Prerequisites

- **Rust**: Ensure you have Rust installed (`cargo`).
- **Gmail Account**:
    1.  Enable **2-Step Verification** on your Google Account.
    2.  Generate an **[App Password](https://support.google.com/accounts/answer/185833)**.
        - Go to your Google Account > Security.
        - Under "Signing in to Google," select **App passwords**.
        - Generate a new password (e.g., name it "MCP Server").
        - **Copy the 16-character password**.

## Configuration

This server requires two environment variables to be set:

- `GMAIL_USER`: Your Gmail address (e.g., `user@gmail.com`).
- `GMAIL_APP_PASSWORD`: The 16-character App Password you generated (without spaces).

### Example

```bash
export GMAIL_USER="your-email@gmail.com"
export GMAIL_APP_PASSWORD="xxxx xxxx xxxx xxxx"
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

### Manual Testing (Interactive)

The server communicates via **JSON-RPC** over Standard Input/Output. You generally do not run it manually unless testing.

To test it manually:

1.  Run `cargo run`.
2.  **Paste** the following JSON blob into the running terminal and hit Enter:

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

3.  The server should respond with a JSON string indicating success or failure.

## Integration with MCP Clients

### Example: Claude Desktop Configuration

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gmail": {
      "command": "/path/to/your/gmail-mcp-server/target/release/gmail-mcp-server",
      "args": [],
      "env": {
        "GMAIL_USER": "your-email@gmail.com",
        "GMAIL_APP_PASSWORD": "xxxx xxxx xxxx xxxx"
      }
    }
  }
}
```

## Distribution

### Option 1: Source
Share this repository. The user runs `cargo run`.

### Option 2: Binary
1.  Build: `cargo build --release`
2.  Share the binary at `target/release/gmail-mcp-server`.
3.  Recipient sets env vars and runs it.

### Option 3: Docker
1.  **Build**:
    ```bash
    docker build -t gmail-mcp-server .
    ```
2.  **Run**:
    ```bash
    docker run -i \
      -e GMAIL_USER="your@gmail.com" \
      -e GMAIL_APP_PASSWORD="xxxx" \
      gmail-mcp-server
    ```
    *Note: The `-i` flag is crucial as the server reads from stdin.*

## Troubleshooting

- **Authentication Fails**: Ensure `GMAIL_APP_PASSWORD` is correct and does NOT include spaces (though `lettre` might handle them, it's safer to remove/quote them). Ensure 2FA is on.
- **Build Errors**: Check internet connection.
