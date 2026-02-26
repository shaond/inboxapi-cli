# InboxAPI CLI

Give your AI agent its own email address. Send, receive, read, search, and reply to emails — right from Claude, OpenCode, Codex, Gemini, or any MCP-compatible AI client. No email server to run, no SMTP to configure.

**Quick Setup (Claude Code, Gemini CLI, OpenCode, etc.):**

Copy and paste this sentence into your agent:  

```
Install the `@inboxapi/cli` npm package globally and then register the `inboxapi` command as an MCP server named `inboxapi`.
```

### How it works

1. Install the CLI
2. Connect it to your AI client (Claude Desktop, Claude Code, Gemini CLI, OpenCode, etc.)
3. Your AI can now use email — no code or API keys needed

An account with a unique email address is created automatically on first run. Your AI can then:

- **Send emails** to any address
- **Receive emails** at its own inbox
- **Reply to** and **forward** emails
- **Search** emails by keyword
- **Read full threads** of conversation

## Technical details

The CLI acts as a local bridge between your AI client and the [InboxAPI](https://inboxapi.ai) cloud service. It speaks the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) over standard input/output, so any compatible AI client can use it without custom integration.

## Good to know

- **This is your agent's personal email** — InboxAPI gives your AI agent its own email address for personal use. It is not a transactional email service — don't use it for bulk sending, marketing, or application notifications.
- **Weekly send limit** — Each account can send to up to five unique email addresses per week. This resets weekly.
- **Check your spam folder** — Emails from InboxAPI may land in your junk or spam folder. If you're not seeing messages, check there first.
- **No attachments yet** — Attachment support is not available right now, but it's coming soon.
- **No rich text yet** — Emails are sent as plain text only. Rich text (HTML) support is coming soon.

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

Just start the proxy — an account is created automatically on first run

```bash
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

Add to current project:

```bash
claude mcp add inboxapi inboxapi
```

Add globally (available in all projects):

```bash
claude mcp add inboxapi inboxapi -s user
```

**Gemini CLI:**

Add to current project:

```bash
gemini mcp add inboxapi inboxapi
```

Add system-wide (available in all directories):

```bash
gemini mcp add inboxapi inboxapi --scope user
```

**OpenCode:**

Add to current project:

```bash
opencode mcp add inboxapi inboxapi
```

Add globally (available in all projects):

```bash
opencode mcp add inboxapi inboxapi -s user
```

**Codex CLI:**

```bash
codex mcp add inboxapi -- inboxapi
```

## Development

```bash
cargo build           # Build debug binary
cargo build --release # Build release binary
cargo test            # Run tests
cargo fmt             # Format code
```

## License

The source code in this repository is licensed under the [MIT License](LICENSE).

## Disclaimer

The InboxAPI service is provided as-is, with no guarantees or warranties of any kind. We reserve all rights regarding how the service is operated. Service terms, features, and availability may change at any time without notice.
