# InboxAPI CLI

STDIO proxy that bridges the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) over STDIO to the remote [InboxAPI](https://inboxapi.ai) service over Streamable HTTP/SSE. This lets any MCP-compatible client (Claude Desktop, Claude Code, etc.) use InboxAPI's email tools without custom HTTP integration.

## Installation

```bash
npm install -g @inboxapi/cli
```

Prebuilt binaries are included for:

| Platform       | Architecture |
|----------------|--------------|
| macOS          | ARM64, x64   |
| Linux          | x64, ARM64   |
| Windows        | x64          |

## Getting Started

```bash
# Just start the proxy — an account is created automatically on first run
inboxapi proxy
```

On first run with no saved credentials, the CLI auto-creates an account with a generated name (e.g. `brooding-fluffy-owl`) and authenticates. No manual setup needed.

Credentials are stored in your system config directory and automatically injected into tool calls. The CLI checks multiple locations so it can pick up credentials created by AI agents:

- `~/Library/Application Support/inboxapi/credentials.json` (macOS primary)
- `~/.config/inboxapi/credentials.json` (Linux primary / macOS fallback)
- `~/.local/inboxapi/credentials.json` (fallback, used by some AI agents)

## Commands

### `proxy` (default)

Starts the STDIO proxy. Reads JSON-RPC messages from stdin, forwards them to the InboxAPI endpoint, and streams SSE responses to stdout. If no credentials are found, an account is automatically created with a generated name.

```bash
inboxapi proxy
inboxapi proxy --endpoint https://custom-endpoint.example.com/mcp
```

Running `inboxapi` with no subcommand also starts the proxy.

### `login`

Manually creates an account with a chosen name and stores access credentials locally. Not required for basic usage since `proxy` handles account creation automatically.

```bash
inboxapi login
inboxapi login --name myaccount
inboxapi login --endpoint https://custom-endpoint.example.com/mcp
```

### `whoami`

Displays the currently authenticated account and endpoint.

```bash
inboxapi whoami
```

## Usage with MCP Clients

InboxAPI CLI works as an MCP STDIO transport. Point your MCP client at the `inboxapi` binary:

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "inboxapi": {
      "command": "inboxapi"
    }
  }
}
```

**Claude Code:**

```bash
# Add to current project
claude mcp add inboxapi inboxapi

# Add globally (available in all projects)
claude mcp add inboxapi inboxapi -s user
```

**Gemini CLI:**

```bash
# Add to current project
gemini mcp add inboxapi inboxapi

# Add system-wide (available in all directories)
gemini mcp add inboxapi inboxapi --scope user
```

## Development

```bash
cargo build          # Build debug binary
cargo build --release # Build release binary
cargo test           # Run tests
cargo fmt            # Format code
```

## License

MIT
