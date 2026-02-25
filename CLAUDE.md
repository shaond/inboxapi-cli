# InboxAPI CLI - Claude/AI Agent Context

## Project Overview
A Rust-based STDIO proxy for the InboxAPI MCP service. It allows AI tools (like Claude Desktop/Code) that speak JSON-RPC over STDIO to communicate with the remote InboxAPI MCP service hosted over SSE/HTTP.

## Development Commands
- `cargo build`: Build the project
- `cargo run -- login`: Log in to the service
- `cargo run -- proxy`: Start the STDIO proxy (default)

## Architecture
- **Transport**: Bridges STDIO (stdin/stdout) to remote SSE (Server-Sent Events) and POST requests.
- **Authentication**: Automatically injects access tokens from the OS config directory (e.g., `~/.config/inboxapi/credentials.json` on Linux) into tool calls.
- **Onboarding**: Includes a `login` command that handles Hashcash proof-of-work and token exchange.

## Key Files
- `src/main.rs`: Core proxy and login logic.
- `Cargo.toml`: Project dependencies.

## Usage for AI Tools
To use this with Claude Desktop, add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "inboxapi": {
      "command": "path/to/inboxapi-cli",
      "args": ["proxy"]
    }
  }
}
```
